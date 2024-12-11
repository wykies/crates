//! Plugin to add chat functionality

#[derive(serde::Deserialize, Clone)]
pub struct ChatSettings {
    pub heartbeat_interval_secs: u8,
}
