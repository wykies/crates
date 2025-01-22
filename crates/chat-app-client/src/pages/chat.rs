use std::fmt::Debug;

use crate::{app::wake_fn, displayable_page_common};

use super::DisplayablePage;
use frontend::FrontEnd;
use reqwest_cross::DataState;
use wykies_shared::{
    const_config::path::PATH_WS_TOKEN_CHAT, uac::get_required_permissions, websockets::WSConnTxRx,
};

mod frontend;

#[derive(Default, serde::Serialize, serde::Deserialize, Debug)]
#[serde(default)]
pub struct UiChat {
    is_open: bool,
    page_unique_number: usize,
    #[serde(skip)]
    frontend: Option<FrontEnd>,
    #[serde(skip)]
    data_state: DataState<WSConnTxRx>,
}

impl DisplayablePage for UiChat {
    displayable_page_common!(
        "Chat",
        get_required_permissions(PATH_WS_TOKEN_CHAT.path).expect("failed to get permissions")
    );

    fn reset_to_default(&mut self, _: super::private::Token) {
        self.frontend = Default::default();
        self.data_state = DataState::default();
    }

    fn show(&mut self, ui: &mut eframe::egui::Ui, data_shared: &mut crate::DataShared) {
        let title = self.title(); // Needed to allocate it to not capture self
        let frontend_init = || {
            FrontEnd::new(
                data_shared.username.clone().try_into().expect(
                    "at this point the user should be logged in so the username should be valid",
                ),
                title,
            )
        };
        if self.data_state.is_none() {
            let ctx = ui.ctx().clone();
            self.data_state.egui_start_request(ui, || {
                data_shared
                    .client
                    .ws_connect(PATH_WS_TOKEN_CHAT, wake_fn(ctx))
            });
        }
        if let Some(connection) = self.data_state.egui_poll_mut(ui, Some("Reconnect")) {
            self.frontend
                .get_or_insert_with(frontend_init)
                .show(ui, connection)
        }
    }
}
