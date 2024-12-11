use std::{fmt::Display, time::Duration};

use crate::WebSocketSettings;
use tracing::instrument;
use wykies_time::Seconds;

#[derive(Debug, Clone, Copy)]
pub struct HeartbeatConfig {
    interval_time: Seconds,
    client_timeout: Seconds,
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

    pub fn interval(&self) -> tokio::time::Interval {
        tokio::time::interval(self.interval_time.into())
    }

    pub fn client_timeout(&self) -> Duration {
        self.client_timeout.into()
    }

    pub fn client_timeout_display(&self) -> impl Display {
        format!("{} sec", self.client_timeout)
    }
}
