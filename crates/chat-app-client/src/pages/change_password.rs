use egui::{Button, Context};
use reqwest_cross::{Awaiting, DataState};
use secrecy::{ExposeSecret as _, SecretString};
use wykies_shared::{
    const_config::path::PATH_API_CHANGE_PASSWORD, req_args::api::ChangePasswordReqArgs,
    uac::get_required_permissions,
};

use crate::{app::wake_fn, displayable_page_common, ui_helpers::ui_password_edit};

use super::DisplayablePage;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct UiChangePassword {
    is_open: bool,
    page_unique_number: usize,
    #[serde(skip)]
    heading_text: Option<String>,
    #[serde(skip)]
    should_send: bool,
    #[serde(skip)]
    pub data_state: DataState<()>,
    #[serde(skip)]
    current_password: SecretString,
    #[serde(skip)]
    new_password: SecretString,
    #[serde(skip)]
    confirmation_password: SecretString,
}
impl UiChangePassword {
    fn is_ready_to_send(&self) -> bool {
        !self.current_password.expose_secret().is_empty()
            && !self.new_password.expose_secret().is_empty()
            && !self.confirmation_password.expose_secret().is_empty()
    }

    fn send_request(&mut self, ctx: Context, data_shared: &mut crate::DataShared) {
        let rx = data_shared.client.change_password(
            &ChangePasswordReqArgs {
                current_password: self.current_password.clone(),
                new_password: self.new_password.clone(),
                new_password_check: self.confirmation_password.clone(),
            },
            wake_fn(ctx),
        );
        self.data_state = DataState::AwaitingResponse(Awaiting(rx));
    }

    fn show_controls(&mut self, ui: &mut egui::Ui, data_shared: &mut crate::DataShared) {
        let mut should_send = false;
        ui.add_enabled(false, egui::TextEdit::singleline(&mut data_shared.username));

        ui.spacing();
        let mut lost_focus =
            ui_password_edit(ui, &mut self.current_password, "Current Password").lost_focus();
        ui.spacing();
        lost_focus =
            ui_password_edit(ui, &mut self.new_password, "New Password").lost_focus() || lost_focus;
        ui.spacing();
        lost_focus = ui_password_edit(ui, &mut self.confirmation_password, "Confirm New Password")
            .lost_focus()
            || lost_focus;

        let is_ready_to_send = self.is_ready_to_send();
        if lost_focus && is_ready_to_send && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            should_send = true;
        }

        ui.spacing();
        ui.spacing();
        if ui
            .add_enabled(is_ready_to_send, Button::new("Send"))
            .clicked()
        {
            should_send = true;
        }

        if should_send {
            self.send_request(ui.ctx().clone(), data_shared);
        }
    }

    pub(crate) fn new_with_heading<S: Into<String>>(heading_text: S) -> Self {
        Self {
            heading_text: Some(heading_text.into()),
            ..Default::default()
        }
    }
}

impl DisplayablePage for UiChangePassword {
    displayable_page_common!(
        "Change Password",
        get_required_permissions(PATH_API_CHANGE_PASSWORD.path).expect("failed to get permissions")
    );

    fn reset_to_default(&mut self, _: super::private::Token) {
        self.data_state = Default::default();
    }

    fn show(&mut self, ui: &mut eframe::egui::Ui, data_shared: &mut crate::DataShared) {
        self.should_send = false; // Reset at top of the loop
        ui.vertical_centered(|ui| {
            if let Some(heading_text) = self.heading_text.as_ref() {
                ui.heading(heading_text);
            }
            match &mut self.data_state {
                DataState::None => self.show_controls(ui, data_shared),
                DataState::AwaitingResponse(rx) => {
                    if let Some(new_state) = DataState::await_data(rx) {
                        self.data_state = new_state;
                    } else {
                        ui.spinner();
                    }
                }
                DataState::Present(()) => {
                    ui.spacing();
                    ui.label("Password Successfully Changed");
                }
                DataState::Failed(e) => {
                    ui.colored_label(ui.visuals().error_fg_color, format!("Failed {e}"));
                    if ui.button("Try Again").clicked() {
                        self.data_state = DataState::default();
                    }
                }
            }
        });
    }
}

impl Default for UiChangePassword {
    fn default() -> Self {
        Self {
            is_open: Default::default(),
            page_unique_number: Default::default(),
            current_password: SecretString::from(""),
            new_password: SecretString::from(""),
            confirmation_password: SecretString::from(""),
            data_state: Default::default(),
            should_send: false,
            heading_text: Default::default(),
        }
    }
}
