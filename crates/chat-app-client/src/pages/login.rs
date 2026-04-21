use super::{DisplayablePage, change_password::UiChangePassword};
use crate::DataShared;
use egui_helpers::{ResponseHelpers, UiHelpers as _};
use reqwest_cross::{Awaiting, DataState};
use secrecy::{ExposeSecret, SecretString};
use std::fmt::Debug;
use tracing::info;
use wykies_client_core::LoginOutcome;
use wykies_shared::req_args::LoginReqArgs;

#[derive(Debug)]
pub struct UiLogin {
    password: SecretString,
    login_attempt_status: DataState<LoginOutcome>,
    password_change_page: Option<UiChangePassword>,
}

impl UiLogin {
    fn is_password_set(&self) -> bool {
        !self.password.expose_secret().is_empty()
    }

    fn is_login_state_allowed_to_login(&self) -> bool {
        match self.login_attempt_status.as_ref() {
            DataState::None
            | DataState::Failed(_)
            | DataState::Present(LoginOutcome::RetryWithBranchSet) => true,
            DataState::AwaitingResponse(_)
            | DataState::Present(LoginOutcome::ForcePasswordChange)
            | DataState::Present(LoginOutcome::Success) => false,
        }
    }

    fn login_prompt(&mut self, ui: &mut egui::Ui, data_shared: &mut DataShared) {
        // Being logged in implies that we are locked out because we shouldn't be on
        // this screen unless we are locked out. (Except for the brief period of time we
        // are doing the post login cleanup)
        let is_effectively_locked_out = data_shared.is_logged_in();
        let username_widget =
            egui::TextEdit::singleline(&mut data_shared.username).hint_text("Username");
        let mut was_enter_pressed = ui
            .add_enabled(!is_effectively_locked_out, username_widget)
            .on_disabled_hover_text("User locked out - Only they can login")
            .enter_pressed(ui);

        was_enter_pressed = ui
            .password_edit(&mut self.password, "Password")
            .enter_pressed(ui)
            || was_enter_pressed;

        if was_enter_pressed && is_allowed_to_login(self, &data_shared.username) {
            self.send_login_attempt(data_shared)
        }
    }

    fn check_login_attempt_status(&mut self, ui: &mut egui::Ui, data_shared: &mut DataShared) {
        match &mut self.login_attempt_status {
            DataState::None => {
                // No special UI needed
            }
            DataState::Present(LoginOutcome::ForcePasswordChange) => {
                // Handled at the start of the update loop
            }
            DataState::Present(LoginOutcome::Success) => {
                if data_shared.is_logged_in() {
                    debug_assert!(
                        data_shared.is_screen_locked(),
                        "expected to only get if the screen was locked"
                    );
                    data_shared.unlock();
                } else {
                    data_shared.mark_login_complete();
                }
                ui.request_repaint(); // Repaint with new value
            }
            DataState::Present(LoginOutcome::RetryWithBranchSet) => {
                ui.label("Please select the branch to set");
                // TODO 4: Add ui to choose branch (branch_to_set 1) Two
                //          reminders to mark locations
            }
            DataState::AwaitingResponse(rx) => {
                if let Some(new_state) = DataState::await_data(rx) {
                    info!(
                        ?new_state,
                        "Response received for login attempt. New state is: {new_state:?}"
                    );
                    self.login_attempt_status = new_state;
                    ui.request_repaint();
                } else {
                    ui.spinner();
                }
            }
            DataState::Failed(e) => {
                ui.separator();
                let err_msg = format!("Login attempt failed: {e}");
                ui.error_label(err_msg);
                if ui.button("Clear error status").clicked() {
                    self.login_attempt_status = DataState::None;
                }
                ui.separator();
            }
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, data_shared: &mut DataShared) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            if matches!(
                self.login_attempt_status,
                DataState::Present(LoginOutcome::ForcePasswordChange),
            ) {
                let password_page = self.password_change_page.get_or_insert_with(|| {
                    UiChangePassword::new_with_heading(
                        "You are required to change your password to logon",
                    )
                });
                password_page.show(ui, data_shared);
                if password_page.data_state.is_present() {
                    self.login_attempt_status = DataState::Present(LoginOutcome::Success);
                }
            } else {
                ui.vertical_centered(|ui| {
                    ui.heading("Login");

                    self.login_prompt(ui, data_shared);

                    self.check_login_attempt_status(ui, data_shared);

                    self.login_button(ui, data_shared);
                });
            }
        });
    }

    fn login_button(&mut self, ui: &mut egui::Ui, data_shared: &mut DataShared) {
        if ui
            .add_enabled(
                is_allowed_to_login(self, &data_shared.username),
                egui::Button::new("Login"),
            )
            .clicked()
        {
            self.send_login_attempt(data_shared);
        }
    }

    fn send_login_attempt(&mut self, data_shared: &mut DataShared) {
        let args = LoginReqArgs::new_with_branch(
            data_shared.username.clone(),
            self.password.clone(),
            1.into(), // Branches are not needed by the demo
        );

        let rx = data_shared.client.login(args);
        self.login_attempt_status = DataState::AwaitingResponse(Awaiting(rx));
    }
}

impl Default for UiLogin {
    fn default() -> Self {
        Self {
            password: SecretString::from(""),
            login_attempt_status: Default::default(),
            password_change_page: Default::default(),
        }
    }
}

fn is_allowed_to_login(data: &UiLogin, username: &str) -> bool {
    !username.is_empty() && data.is_password_set() && data.is_login_state_allowed_to_login()
}
