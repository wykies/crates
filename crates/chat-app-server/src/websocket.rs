use ws_auth::WsId;

pub struct WsIds;

impl WsIds {
    pub const CHAT: WsId = WsId::new(1);
}
