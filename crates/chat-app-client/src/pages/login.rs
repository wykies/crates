use super::{change_password::UiChangePassword, data_state::DataState, DisplayablePage};
use crate::{app::wake_fn, ui_helpers::ui_password_edit, DataShared};
use futures::channel::oneshot;
use secrecy::{ExposeSecret, SecretString};
use std::fmt::Debug;
use tracing::{error, info};
use wykies_client_core::LoginOutcome;
use wykies_shared::{internal_error, req_args::LoginReqArgs};

#[derive(Debug)]
pub struct UiLogin {
    password: SecretString,
    login_attempt_status: LoginAttemptStatus,
    password_change_page: Option<UiChangePassword>,
}

// TODO 5: See if we should replace this type with the one in data_shared
type AwaitingType = oneshot::Receiver<anyhow::Result<LoginOutcome>>;

#[derive(Default)]
enum LoginAttemptStatus {
    #[default]
    NotAttempted,
    AwaitingResponse(AwaitingType),
    Failed(String),
    Success,
    SuccessForcePassChange,
    ResendWithBranch,
}

impl Debug for LoginAttemptStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotAttempted => write!(f, "NotAttempted"),
            Self::AwaitingResponse(_) => write!(f, "AwaitingResponse"),
            Self::Failed(e) => f.debug_tuple("Failed").field(e).finish(),
            Self::Success => write!(f, "Success"),
            Self::SuccessForcePassChange => write!(f, "SuccessForcePassChange"),
            Self::ResendWithBranch => write!(f, "ResendWithBranch"),
        }
    }
}

impl LoginAttemptStatus {
    fn is_allowed_to_login(&self) -> bool {
        match self {
            LoginAttemptStatus::NotAttempted
            | LoginAttemptStatus::Failed(_)
            | LoginAttemptStatus::ResendWithBranch => true,
            LoginAttemptStatus::AwaitingResponse(_)
            | LoginAttemptStatus::Success
            | LoginAttemptStatus::SuccessForcePassChange => false,
        }
    }
}

impl UiLogin {
    fn is_password_set(&self) -> bool {
        !self.password.expose_secret().is_empty()
    }

    fn login_prompt(&mut self, ui: &mut egui::Ui, data_shared: &mut DataShared) {
        // Being logged in implies that we are locked out because we shouldn't be on
        // this screen unless we are locked out. (Except for the brief period of time we
        // are doing the post login cleanup)
        let is_effectively_locked_out = data_shared.is_logged_in();
        let username_widget =
            egui::TextEdit::singleline(&mut data_shared.username).hint_text("Username");
        let mut lost_focus = ui
            .add_enabled(!is_effectively_locked_out, username_widget)
            .on_disabled_hover_text("User locked out - Only they can login")
            .lost_focus();

        lost_focus =
            ui_password_edit(ui, &mut self.password, "Password").lost_focus() || lost_focus;

        if lost_focus
            && is_allowed_to_login(self, &data_shared.username)
            && ui.input(|i| i.key_pressed(egui::Key::Enter))
        {
            self.send_login_attempt(ui, data_shared)
        }
    }

    fn check_login_attempt_status(&mut self, ui: &mut egui::Ui, data_shared: &mut DataShared) {
        match &mut self.login_attempt_status {
            LoginAttemptStatus::NotAttempted => {
                // No special UI needed
            }
            LoginAttemptStatus::SuccessForcePassChange => {
                // Handled at the start of the update loop
            }
            LoginAttemptStatus::Success => {
                if data_shared.is_logged_in() {
                    debug_assert!(
                        data_shared.is_screen_locked(),
                        "expected to only get if the screen was locked"
                    );
                    data_shared.unlock();
                } else {
                    data_shared.mark_login_complete();
                }
                ui.ctx().request_repaint(); // Repaint with new value
            }
            LoginAttemptStatus::ResendWithBranch => {
                ui.label("Please select the branch to set");
                // TODO 4: Add ui to choose branch (branch_to_set 1) Two
                //          reminders to mark locations
            }
            LoginAttemptStatus::AwaitingResponse(rx) => match rx.try_recv() {
                Ok(recv_opt) => match recv_opt {
                    Some(outcome_result) => match outcome_result {
                        Ok(outcome) => {
                            info!("login outcome from client-core: {outcome:?}");
                            self.login_attempt_status = match outcome {
                                LoginOutcome::Success => LoginAttemptStatus::Success,
                                LoginOutcome::ForcePasswordChange => {
                                    LoginAttemptStatus::SuccessForcePassChange
                                }
                                LoginOutcome::RetryWithBranchSet => {
                                    LoginAttemptStatus::ResendWithBranch
                                }
                            };
                            info!(
                                "login_attempt_status changed to: {:?}",
                                self.login_attempt_status
                            );
                            // Repaint with new value
                            ui.ctx().request_repaint();
                        }
                        Err(e) => {
                            info!("error returned from core-client: {e:?}");
                            self.login_attempt_status = LoginAttemptStatus::Failed(e.to_string())
                        }
                    },
                    None => {
                        ui.spinner();
                    }
                },
                Err(e) => {
                    error!("Error receiving on channel. Canceled: {e:?}");
                    self.login_attempt_status = LoginAttemptStatus::Failed(internal_error!(e));
                }
            },
            LoginAttemptStatus::Failed(e) => {
                let err_msg = format!("Login attempt failed: {e}");
                ui.separator();
                ui.colored_label(ui.visuals().error_fg_color, err_msg);
                if ui.button("Clear error status").clicked() {
                    self.login_attempt_status = LoginAttemptStatus::NotAttempted;
                }
                ui.separator();
            }
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, data_shared: &mut DataShared) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if matches!(
                self.login_attempt_status,
                LoginAttemptStatus::SuccessForcePassChange,
            ) {
                let password_page = self.password_change_page.get_or_insert_with(|| {
                    UiChangePassword::new_with_heading(
                        "You are required to change your password to logon",
                    )
                });
                password_page.show(ui, data_shared);
                if matches!(password_page.data_state, DataState::Present(())) {
                    self.login_attempt_status = LoginAttemptStatus::Success;
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
            self.send_login_attempt(ui, data_shared);
        }
    }

    fn send_login_attempt(&mut self, ui: &mut egui::Ui, data_shared: &mut DataShared) {
        let args = LoginReqArgs::new_with_branch(
            data_shared.username.clone(),
            self.password.clone(),
            1.into(), // Branches are not needed by the demo
        );

        let rx = data_shared.client.login(args, wake_fn(ui.ctx().clone()));
        self.login_attempt_status = LoginAttemptStatus::AwaitingResponse(rx);
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
    !username.is_empty()
        && data.is_password_set()
        && data.login_attempt_status.is_allowed_to_login()
}
