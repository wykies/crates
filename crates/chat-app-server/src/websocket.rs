use ws_auth::WsServiceId;

pub struct WsServiceIds;

impl WsServiceIds {
    pub const CHAT: WsServiceId = WsServiceId::new(1);
}
