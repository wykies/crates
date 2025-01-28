use wykies_time::Seconds;

#[derive(serde::Deserialize, Clone, Debug)]
pub struct WebSocketSettings {
    pub token_lifetime_secs: Seconds,
    pub heartbeat_times_missed_allowance: u8,
    pub heartbeat_additional_buffer_time_secs: Seconds,
}
