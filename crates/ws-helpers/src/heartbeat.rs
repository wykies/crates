use crate::WebSocketSettings;
use actix_ws::{CloseCode, CloseReason, Session};
use anyhow::Context;
use std::{fmt::Display, time::Duration};
use tokio::time::{Instant, Interval};
use tracing::{instrument, warn};
use wykies_shared::log_err_as_error;
use wykies_time::Seconds;

#[derive(Debug, Clone, Copy)]
pub struct HeartbeatConfig {
    interval_time: Seconds,
    client_timeout: Seconds,
}

#[derive(Debug)]
pub struct HeartbeatMonitor {
    config: HeartbeatConfig,
    last: Instant,
    interval: Interval,
}

impl HeartbeatConfig {
    #[instrument(ret)]
    pub fn new(interval_time: Seconds, ws_config: &WebSocketSettings) -> Self {
        let times_missed_allowance = ws_config.heartbeat_times_missed_allowance.into();
        let additional_buffer_time = ws_config.heartbeat_additional_buffer_time_secs;
        let client_timeout = interval_time * times_missed_allowance + additional_buffer_time;

        Self {
            interval_time,
            client_timeout,
        }
    }

    pub fn start_new_monitor(&self) -> HeartbeatMonitor {
        let interval = tokio::time::interval(self.interval_time.into());
        let last = Instant::now();
        HeartbeatMonitor {
            config: *self,
            last,
            interval,
        }
    }

    pub fn client_timeout(&self) -> Duration {
        self.client_timeout.into()
    }

    pub fn client_timeout_display(&self) -> impl Display {
        format!("{} sec", self.client_timeout)
    }
}

impl HeartbeatMonitor {
    pub async fn tick(&mut self) -> Instant {
        self.interval.tick().await
    }

    #[instrument(level = "debug", skip(ws_session))]
    /// if no heartbeat ping/pong received recently, close the connection
    pub async fn process_tick(&mut self, ws_session: &mut Session) -> Option<CloseReason> {
        if Instant::now().duration_since(self.last) > self.config.client_timeout() {
            warn!(
                "client has not sent heartbeat in over {}; disconnecting",
                self.config.client_timeout_display()
            );
            Some(CloseReason {
                code: CloseCode::Policy,
                description: Some("Failed to respond to ping".into()),
            })
        } else {
            // send heartbeat ping
            let r = ws_session.ping(b"").await.context("failed to send ping");
            log_err_as_error!(r);
            None
        }
    }

    pub fn response_received(&mut self) {
        self.last = Instant::now();
    }
}
