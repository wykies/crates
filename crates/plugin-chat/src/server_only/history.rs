use std::fmt::Debug;

use anyhow::{bail, Context};
use ringbuffer::{AllocRingBuffer, RingBuffer};
use sqlx::QueryBuilder;
use tokio::{select, sync::mpsc, time::Sleep};
use tracing::{error, info, instrument};
use tracked_cancellations::TrackedCancellationToken;
use wykies_server::db_types::{Db, DbPool};
use wykies_shared::{const_config::CHANNEL_BUFFER_SIZE, debug_panic};
use wykies_time::{Seconds, Timestamp};

use crate::ChatIM;

#[derive(Debug)]
pub struct ChatHistory {
    recent: AllocRingBuffer<ChatIM>,
    db_writer_handle: ChatDbWriterHandle,
}

#[derive(Debug)]
struct ChatDbWriterHandle {
    tx: mpsc::Sender<ChatIM>,
}

// Update Debug impl if adding new fields
struct ChatDbWriter {
    rx: mpsc::Receiver<ChatIM>,
    last_save_time: Timestamp,
    max_time_before_save: Seconds,
    max_ims_before_save: u8,
    pool: DbPool,
    buffer: Vec<ChatIM>,
}

impl ChatHistory {
    #[instrument(skip(pool))]
    pub fn new(
        recent_capacity: usize,
        pool: DbPool,
        cancellation_token: TrackedCancellationToken,
        max_time_before_save: Seconds,
        max_ims_before_save: u8,
    ) -> Self {
        let handle = ChatDbWriterHandle::new(
            pool,
            cancellation_token,
            max_time_before_save,
            max_ims_before_save,
        );
        Self {
            recent: AllocRingBuffer::new(recent_capacity),
            db_writer_handle: handle,
        }
    }

    #[instrument]
    pub async fn push(&mut self, im: ChatIM) -> anyhow::Result<()> {
        self.recent.push(im.clone());
        self.db_writer_handle
            .enqueue_for_saving(im)
            .await
            .context("failed to enqueue IM to be saved")
    }

    #[instrument]
    pub fn get_recent(&self) -> Vec<ChatIM> {
        self.recent.to_vec()
    }
}

impl ChatDbWriterHandle {
    #[instrument]
    fn new(
        pool: DbPool,
        cancellation_token: TrackedCancellationToken,
        max_time_before_save: Seconds,
        max_ims_before_save: u8,
    ) -> Self {
        let (tx, rx) = mpsc::channel(CHANNEL_BUFFER_SIZE);
        let writer = ChatDbWriter {
            rx,
            last_save_time: Timestamp::now(),
            max_time_before_save,
            max_ims_before_save,
            pool,
            buffer: Default::default(),
        };
        tokio::spawn(writer.run(cancellation_token));
        Self { tx }
    }

    #[instrument]
    async fn enqueue_for_saving(&self, im: ChatIM) -> anyhow::Result<()> {
        self.tx
            .send(im)
            .await
            .context("failed to send IM to writer")
    }
}

impl ChatDbWriter {
    #[instrument(err(Debug))]
    async fn run(mut self, cancellation_token: TrackedCancellationToken) -> anyhow::Result<()> {
        // Drop guard ensures that if we exit we shutdown the rest of the server
        let _drop_guard = cancellation_token.clone().drop_guard();
        loop {
            let max_time_before_save = self.time_until_max_time_before_save();
            select! {
                _ = cancellation_token.cancelled() => {
                    self.save("cancellation").await.context("failed to save after receiving cancellation request")?;
                    bail!("Received cancellation request. Shutdown ChatDbWriter");
                }
                im = self.rx.recv() => self.process_im(im).await?,
                _ = max_time_before_save, if !self.buffer.is_empty() => self.save("time").await.context("failed to save at max time reached")?,
            }
        }
    }

    #[instrument]
    /// Return a `Sleep` for the time until we should write to the database
    fn time_until_max_time_before_save(&self) -> Sleep {
        let now = Timestamp::now();
        let target = self.last_save_time + self.max_time_before_save;
        let time_left = if now < target {
            target.abs_diff(self.last_save_time)
        } else {
            // Use min value of 1 second
            info!("Minimum time value used");
            Seconds::new(1)
        };
        info!(?time_left);
        tokio::time::sleep(time_left.into())
    }

    #[instrument(err(Debug))]
    async fn process_im(&mut self, im: Option<ChatIM>) -> anyhow::Result<()> {
        match im {
            Some(im) => {
                if self.buffer.is_empty() {
                    // Was empty no point saving right away
                    self.last_save_time = Timestamp::now();
                }
                self.buffer.push(im);
                if self.buffer.len() >= self.max_ims_before_save as usize {
                    self.save("buffer full")
                        .await
                        .context("failed to save for buffer oversize")?;
                }
                Ok(())
            }
            None => {
                self.save("Closing None Received")
                    .await
                    .context("failed to save while server exiting")?;
                bail!("Saved and exiting ChatDbWriter. Sender dropped, server was likely stopped")
            }
        }
    }

    #[instrument(err(Debug))]
    async fn save(&mut self, save_reason: &str) -> anyhow::Result<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }

        let mut query_builder: QueryBuilder<Db> =
            QueryBuilder::new("INSERT INTO `chat` (`Author`, `Timestamp`, `Content`) ");

        query_builder.push_values(self.buffer.drain(..), |mut b, im| {
            b.push_bind::<String>(im.author.into())
                .push_bind(im.timestamp.as_secs_since_unix_epoch())
                .push_bind::<String>(im.content.into());
        });

        // TODO 5: Optimizations left on the table are to try to have the size sent be
        // more similar so caching would work and reusing the query_builder (see reset)

        // Persistent is set to false because the sizes changes and each would have to
        // be cached separately
        let query = query_builder.build().persistent(false);
        match query
            .execute(&self.pool)
            .await
            .context("failed to save chat IMs to DB")
        {
            Ok(_) => {
                info!("IMs save succeeded")
            }
            Err(err) => {
                error!(?err, "failed to save IMs");
                debug_panic!(err);
            }
        };
        self.last_save_time = Timestamp::now();
        Ok(())
    }
}

impl Debug for ChatDbWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChatDbWriter")
            .field("rx", &self.rx)
            .field("last_save_time", &self.last_save_time)
            .field("max_time_before_save", &self.max_time_before_save)
            .field("max_ims_before_save", &self.max_ims_before_save)
            .field("pool", &self.pool)
            .field("buffer_len", &self.buffer.len())
            .finish()
    }
}
