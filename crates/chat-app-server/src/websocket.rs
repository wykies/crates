use ws_auth::WsServiceId;

pub struct WsIds;

impl WsIds {
    pub const CHAT: WsServiceId = WsServiceId::new(1);
}
