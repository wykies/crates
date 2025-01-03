use std::fmt::Debug;

use crate::{app::wake_fn, displayable_page_common};

use super::{
    data_state::{AwaitingType, DataState},
    DisplayablePage,
};
use frontend::FrontEnd;
use wykies_client_core::WebSocketConnection;
use wykies_shared::{const_config::path::PATH_WS_TOKEN_CHAT, uac::get_required_permissions};

mod frontend;

#[derive(Default, serde::Serialize, serde::Deserialize, Debug)]
#[serde(default)]
pub struct UiChat {
    is_open: bool,
    page_unique_number: usize,
    #[serde(skip)]
    frontend: Option<FrontEnd>,
    #[serde(skip)]
    data_state: DataState<WebSocketConnection>,
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
        if let DataState::Present(connection) = &mut self.data_state {
            self.frontend
                .get_or_insert_with(frontend_init)
                .show(ui, connection)
        } else {
            let ctx = ui.ctx().clone();
            self.data_state.egui_get(ui, Some("Reconnect"), || {
                AwaitingType(
                    data_shared
                        .client
                        .ws_connect(PATH_WS_TOKEN_CHAT, wake_fn(ctx)),
                )
            });
        }
    }
}
