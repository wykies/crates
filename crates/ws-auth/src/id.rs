/// Distinguishes different types of Websocket services supported
#[derive(Debug, PartialEq, Eq, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct WsServiceId(u8);

impl WsServiceId {
    #[cfg(test)]
    pub const TEST1: Self = Self::new(1);

    pub const fn new(value: u8) -> Self {
        Self(value)
    }
}
