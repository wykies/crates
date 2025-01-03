use super::{
    data_state::{AwaitingType, DataState},
    DisplayablePage,
};
use crate::{
    app::wake_fn,
    displayable_page_common,
    ui_helpers::{get_text_height, readonly_checkbox_no_text, ui_escape_button, ui_password_edit},
};
use edit_user_info::EditUserInfo;
use egui::Button;
use egui_extras::{Column, TableBuilder};
use new_user_info::NewUserInfo;
use pass_reset_user_info::PassResetUserInfo;
use secrecy::ExposeSecret;
use std::ops::ControlFlow;
use wykies_client_core::Client;
use wykies_shared::internal_error;
use wykies_shared::{
    const_config::{error::err_role_name, path::PATH_API_ADMIN_USERS_LIST_AND_ROLES},
    id::DbId,
    uac::{
        get_required_permissions, DisplayName, ListUsersRoles, RoleName, UserMetadata, Username,
    },
};
use wykies_time::Seconds;

mod edit_user_info;
mod new_user_info;
mod pass_reset_user_info;

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct UiUAC {
    is_open: bool,
    page_unique_number: usize,
    #[serde(skip)]
    should_refresh: bool,
    #[serde(skip)]
    data_state: DataState<ListUsersRoles>,
    #[serde(skip)]
    user_op: UserOp,
}

#[derive(Debug, Default)]
enum UserOp {
    #[default]
    None,
    Selected(UserMetadata),
    New(NewUserInfo),
    Edit(EditUserInfo),
    PasswordReset(PassResetUserInfo),
}

#[must_use]
#[derive(Debug, PartialEq, Eq)]
enum OpResult {
    NoAction,
    ResetPage,
}

#[must_use]
enum SaveState {
    Completed,
    Ongoing,
    Failed(String),
}

impl UserOp {
    // Serves as a way to check if there are changes to be lost
    fn has_changes(&self) -> bool {
        match self {
            UserOp::None => false,
            UserOp::Selected(_) => false,
            UserOp::New(_) => true,
            UserOp::Edit(edit_user_info) => edit_user_info.has_changes(),
            UserOp::PasswordReset(_) => true,
        }
    }

    fn set_selected_user(&mut self, user: Option<UserMetadata>) {
        *self = match user {
            Some(user) => Self::Selected(user),
            None => Self::None,
        };
    }

    fn selected_user(&self) -> Option<&UserMetadata> {
        match self {
            UserOp::Selected(user_metadata) => Some(user_metadata),
            UserOp::Edit(edit_user_info) => Some(edit_user_info.original_user()),
            _ => None,
        }
    }
}

impl DisplayablePage for UiUAC {
    displayable_page_common!(
        "User Account Control",
        get_required_permissions(PATH_API_ADMIN_USERS_LIST_AND_ROLES.path)
            .expect("failed to get permissions")
    );

    fn reset_to_default(&mut self, _: super::private::Token) {
        self.should_refresh = Default::default();
        self.data_state = Default::default();
        self.user_op = Default::default();
    }

    fn show(&mut self, ui: &mut eframe::egui::Ui, data_shared: &mut crate::DataShared) {
        if self.should_refresh {
            self.reset_to_default(super::private::Token {});
        }
        if let DataState::Present(data) = &mut self.data_state {
            egui::TopBottomPanel::bottom(format!("user edit panel{}", self.page_unique_number))
                .show_inside(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        // Centering only works on the label and button but the grid is not centered
                        if ui_show_user_op(ui, &data_shared.client, data, &mut self.user_op)
                            == OpResult::ResetPage
                        {
                            self.should_refresh = true;
                        };
                    });
                });

            egui::CentralPanel::default().show_inside(ui, |ui| {
                if self.user_op.has_changes() {
                    // Reduce risk of accident data loss by changing user
                    ui.disable();
                }
                ui.horizontal_wrapped(|ui| {
                    if ui.button("Refresh Page").clicked() {
                        self.should_refresh = true;
                        return;
                    }
                    ui.spacing();
                    if ui.button("Add New User").clicked() {
                        self.user_op = UserOp::New(NewUserInfo::new());
                    }
                });
                ui.separator();
                egui::ScrollArea::horizontal()
                    .show(ui, |ui| ui_show_user_list(ui, data, &mut self.user_op));
            });
        } else {
            let ctx = ui.ctx().clone();
            self.data_state.egui_get(ui, None, || {
                AwaitingType(data_shared.client.list_users_and_roles(wake_fn(ctx)))
            });
        }
    }
}

fn ui_show_user_op(
    ui: &mut egui::Ui,
    client_core: &Client,
    data: &ListUsersRoles,
    user_op: &mut UserOp,
) -> OpResult {
    match user_op {
        UserOp::None => {
            ui.label("[NO USER SELECTED]");
            OpResult::NoAction
        }
        UserOp::Selected(user_metadata) => {
            if ui.button("Edit User").clicked() {
                let mut edit_user_info = EditUserInfo::new(user_metadata.clone());
                edit_user_info.load_user_info(ui, client_core);
                *user_op = UserOp::Edit(edit_user_info);
                return OpResult::NoAction;
            }
            if client_core
                .user_info()
                .expect("unable to get user info")
                .username
                != user_metadata.username
                && ui.button("Reset Password").clicked()
            {
                *user_op = UserOp::PasswordReset(PassResetUserInfo::new(user_metadata));
                return OpResult::NoAction;
            }
            OpResult::NoAction
        }
        UserOp::New(new_user_info) => ui_show_new_user(ui, client_core, data, new_user_info),
        UserOp::Edit(edit_user_info) => ui_show_edit_user(ui, client_core, data, edit_user_info),
        UserOp::PasswordReset(pass_reset_user_info) => {
            ui_show_reset_password(ui, client_core, pass_reset_user_info)
        }
    }
}

fn ui_show_reset_password(
    ui: &mut egui::Ui,
    client_core: &Client,
    pass_reset_user_info: &mut PassResetUserInfo,
) -> OpResult {
    match poll_save_outcome(pass_reset_user_info.save_outcome(), ui) {
        ControlFlow::Continue(()) => {} // Do nothing just continue
        ControlFlow::Break(action) => return action,
    }

    egui::Grid::new("Reset password grid")
        .num_columns(2)
        .show(ui, |ui| {
            ui_user_username_read_only(ui, &pass_reset_user_info.data.username);
            ui.end_row();

            ui.label("New Password");
            ui_password_edit(ui, &mut pass_reset_user_info.data.new_password, "");
            ui.end_row();
        });

    if ui
        .add_enabled(
            !pass_reset_user_info
                .data
                .new_password
                .expose_secret()
                .is_empty(),
            Button::new("Reset Password"),
        )
        .clicked()
    {
        pass_reset_user_info.save(client_core);
    }

    if ui_escape_button(ui, "Cancel") {
        return OpResult::ResetPage;
    }

    OpResult::NoAction
}

fn ui_show_new_user(
    ui: &mut egui::Ui,
    client_core: &Client,
    data: &ListUsersRoles,
    new_user_info: &mut NewUserInfo,
) -> OpResult {
    let mut has_errors = false;
    match poll_save_outcome(new_user_info.save_outcome(), ui) {
        ControlFlow::Continue(()) => {} // Do nothing just continue
        ControlFlow::Break(action) => return action,
    }

    egui::Grid::new("New User Grid")
        .num_columns(2)
        .show(ui, |ui| {
            ui.label("Username");
            ui.text_edit_singleline(&mut new_user_info.username);
            if let Err(e) = Username::try_from(new_user_info.username.clone()) {
                has_errors = true;
                ui.colored_label(ui.visuals().error_fg_color, e.to_string());
            }
            ui.end_row();

            //----------------------------------------------------------------------
            ui.label("Display Name");
            ui.text_edit_singleline(&mut new_user_info.display_name);
            if let Err(e) = DisplayName::try_from(new_user_info.display_name.clone()) {
                has_errors = true;
                ui.colored_label(ui.visuals().error_fg_color, e.to_string());
            }
            ui.end_row();

            //----------------------------------------------------------------------
            ui.label("Password");
            ui_password_edit(ui, &mut new_user_info.password, "User's Password");
            if new_user_info.password.expose_secret().is_empty() {
                has_errors = true;
                ui.colored_label(ui.visuals().error_fg_color, "Required".to_string());
            }
            ui.end_row();

            //----------------------------------------------------------------------
            ui_user_role(ui, None, &mut new_user_info.assigned_role, data);
        });

    if ui
        .add_enabled(!has_errors, Button::new("Save New User"))
        .clicked()
    {
        new_user_info.save(client_core);
    }

    if ui_escape_button(ui, "Cancel") {
        return OpResult::ResetPage;
    }

    OpResult::NoAction
}

fn ui_show_edit_user(
    ui: &mut egui::Ui,
    client_core: &Client,
    data: &ListUsersRoles,
    edit_user_info: &mut EditUserInfo,
) -> OpResult {
    if !edit_user_info.has_load_started() {
        if ui.button("Reload User").clicked() {
            edit_user_info.load_user_info(ui, client_core);
        } else {
            // Not clicked add no other UI
            return OpResult::NoAction;
        }
    }

    match poll_save_outcome(edit_user_info.save_outcome(), ui) {
        ControlFlow::Continue(()) => {} // Do nothing just continue
        ControlFlow::Break(action) => return action,
    }

    let time_left = edit_user_info.time_before_auto_unload_user();

    let Some((org_user, edit_user)) = edit_user_info.org_user_and_edit_user(ui, client_core) else {
        // User still loading
        return OpResult::NoAction;
    };

    egui::Grid::new("Edit User Grid")
        .num_columns(2)
        .show(ui, |ui| {
            ui_user_username_read_only(ui, &edit_user.username);
            ui.end_row();

            ui_user_display_name(
                ui,
                Some(&org_user.display_name),
                &mut edit_user.display_name,
            );
            ui.end_row();

            ui_user_force_pass_change(
                ui,
                Some(&org_user.force_pass_change),
                &mut edit_user.force_pass_change,
            );
            ui.end_row();

            ui_user_role(
                ui,
                Some(&org_user.assigned_role),
                &mut edit_user.assigned_role,
                data,
            );
            ui.end_row();

            ui_user_enabled(ui, Some(&org_user.enabled), &mut edit_user.enabled);
            ui.end_row();

            ui_user_locked_out(
                ui,
                Some(&org_user.locked_out),
                &mut edit_user.locked_out,
                &mut edit_user.failed_attempts,
            );
            ui.end_row();

            ui_user_failed_attempts_read_only(
                ui,
                Some(&org_user.failed_attempts),
                &mut edit_user.failed_attempts,
            );
            ui.end_row();
        });

    if ui
        .add_enabled(edit_user_info.has_changes(), Button::new("Save"))
        .clicked()
    {
        edit_user_info.save(ui, client_core);
    }

    if ui_escape_button(ui, "Cancel") {
        edit_user_info.unload_user_info();
    }

    ui.label(format!(
        "Automatically cancels in {} seconds",
        time_left.unwrap_or(Seconds::new(0))
    ));

    OpResult::NoAction
}

fn poll_save_outcome(outcome: Option<SaveState>, ui: &mut egui::Ui) -> ControlFlow<OpResult> {
    if let Some(save_status) = outcome {
        // Save in progress
        match save_status {
            SaveState::Completed => return ControlFlow::Break(OpResult::ResetPage),
            SaveState::Ongoing => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.spacing();
                    ui.label("Saving...");
                });
            }
            SaveState::Failed(e) => {
                ui.colored_label(ui.visuals().error_fg_color, e);
                if ui.button("Clear Error").clicked() {
                    return ControlFlow::Break(OpResult::ResetPage);
                }
            }
        }
        ControlFlow::Break(OpResult::NoAction)
    } else {
        ControlFlow::Continue(())
    }
}

fn ui_user_failed_attempts_read_only(ui: &mut egui::Ui, org: Option<&u8>, edit: &mut u8) {
    ui.horizontal(|ui| {
        ui.label("Failed Attempts");
        if let Some(org) = org {
            ui_change_indicator(ui, org != edit);
        }
    });
    ui.label(edit.to_string());
}

fn ui_user_locked_out(
    ui: &mut egui::Ui,
    org: Option<&bool>,
    edit: &mut bool,
    edit_failed_attempts: &mut u8,
) {
    ui.horizontal(|ui| {
        ui.label("Locked Out");
        if let Some(org) = org {
            ui_change_indicator(ui, org != edit);
        }
    });
    if *edit {
        if ui.button("Unlock").clicked() {
            *edit = false;
            *edit_failed_attempts = 0;
        }
    } else {
        ui.label("Not Locked");
    }
}

fn ui_user_enabled(ui: &mut egui::Ui, org: Option<&bool>, edit: &mut bool) {
    ui.horizontal(|ui| {
        ui.label("Enabled");
        if let Some(org) = org {
            ui_change_indicator(ui, org != edit);
        }
    });
    ui.checkbox(edit, "");
}

fn ui_user_role(
    ui: &mut egui::Ui,
    org: Option<&Option<DbId>>,
    edit: &mut Option<DbId>,
    data: &ListUsersRoles,
) {
    ui.horizontal(|ui| {
        ui.label("Role");
        if let Some(org) = org {
            ui_change_indicator(ui, org != edit);
        }
    });
    egui::ComboBox::from_label("")
        .selected_text(
            edit.map(|id| {
                data.role_id_to_name(id).unwrap_or_else(|e| {
                    internal_error!(format!("unable to find Role ID {:?}. {e:?}", edit));
                    err_role_name()
                })
            })
            .unwrap_or(RoleName::no_role_set()),
        )
        .show_ui(ui, |ui| {
            ui.selectable_value(edit, None, RoleName::no_role_set());
            for x in data.roles.iter() {
                ui.selectable_value(edit, Some(x.id), &x.name);
            }
        });
}

fn ui_user_force_pass_change(ui: &mut egui::Ui, org: Option<&bool>, edit: &mut bool) {
    ui.horizontal(|ui| {
        ui.label("Force Pass Change");
        if let Some(org) = org {
            ui_change_indicator(ui, org != edit);
        }
    });
    ui.checkbox(edit, "");
}

fn ui_change_indicator(ui: &mut egui::Ui, is_changed: bool) {
    if is_changed {
        ui.label("*");
    } else {
        // Add placeholder space for indicator to avoid resizing
        ui.label("  ");
    }
}

fn ui_user_username_read_only(ui: &mut egui::Ui, username: &Username) {
    ui.label("Username");
    ui.label(username);
}

fn ui_user_display_name(ui: &mut egui::Ui, org: Option<&DisplayName>, edit: &mut DisplayName) {
    ui.horizontal(|ui| {
        ui.label("Display Name");
        if let Some(org) = org {
            ui_change_indicator(ui, org != edit);
        }
    });
    let mut temp: String = edit.clone().into();
    ui.text_edit_singleline(&mut temp);
    if let Ok(x) = temp.try_into() {
        *edit = x;
    }
}

fn ui_show_user_list(ui: &mut egui::Ui, data: &mut ListUsersRoles, user_op: &mut UserOp) {
    let text_height = get_text_height(ui);
    let mut table_builder = TableBuilder::new(ui)
        .striped(true)
        .resizable(true)
        .cell_layout(egui::Layout::left_to_right(egui::Align::LEFT))
        .column(Column::auto())
        .column(Column::auto())
        .column(Column::auto())
        .column(Column::auto())
        .column(Column::auto())
        .column(Column::auto())
        .column(Column::auto())
        .column(Column::auto())
        .column(Column::remainder())
        .min_scrolled_height(0.0);

    table_builder = table_builder.sense(egui::Sense::click());

    let table = table_builder.header(text_height, |mut header| {
        header.col(|ui| {
            ui.strong("Selected");
        });
        header.col(|ui| {
            ui.strong("Username");
        });
        header.col(|ui| {
            ui.strong("Display Name");
        });
        header.col(|ui| {
            ui.strong("Force Pass Change");
        });
        header.col(|ui| {
            ui.strong("Role");
        });
        header.col(|ui| {
            ui.strong("Enabled");
        });
        header.col(|ui| {
            ui.strong("Locked Out");
        });
        header.col(|ui| {
            ui.strong("Failed Attempts");
        });
        header.col(|ui| {
            ui.strong("Pass Change Date");
        });
    });

    table.body(|body| {
        body.rows(text_height, data.users.len(), |mut row| {
            let row_index = row.index();
            let user = &data.users[row_index];
            let is_selected_at_start = if let Some(selected) = user_op.selected_user() {
                let is_selected = selected.same_username(user);
                row.set_selected(is_selected);
                is_selected
            } else {
                false
            };
            let mut is_selected_at_end = is_selected_at_start;
            row.col(|ui| {
                ui.vertical_centered(|ui| {
                    ui.checkbox(&mut is_selected_at_end, "");
                });
            });
            row.col(|ui| {
                ui.label(&user.username);
            });
            row.col(|ui| {
                ui.label(&user.display_name);
            });
            row.col(|ui| {
                ui.vertical_centered(|ui| {
                    readonly_checkbox_no_text(ui, user.force_pass_change);
                });
            });
            row.col(|ui| {
                ui.label(
                    user.assigned_role
                        .map(|id| {
                            data.role_id_to_name(id).unwrap_or_else(|e| {
                                internal_error!(format!(
                                    "unable to find Role ID {:?}. {e:?}",
                                    user.assigned_role
                                ));
                                err_role_name()
                            })
                        })
                        .unwrap_or(RoleName::no_role_set()),
                );
            });
            row.col(|ui| {
                ui.vertical_centered(|ui| {
                    readonly_checkbox_no_text(ui, user.enabled);
                });
            });
            row.col(|ui| {
                ui.vertical_centered(|ui| {
                    readonly_checkbox_no_text(ui, user.locked_out);
                });
            });
            row.col(|ui| {
                ui.label(user.failed_attempts.to_string());
            });
            row.col(|ui| {
                ui.label(user.pass_change_date.format("%F").to_string());
            });

            // Check for click of a row
            if row.response().clicked() {
                is_selected_at_end = !is_selected_at_end;
            }
            match (is_selected_at_start, is_selected_at_end) {
                (true, true) | (false, false) => {} // No change
                (true, false) => user_op.set_selected_user(None),
                (false, true) => user_op.set_selected_user(Some(user.clone())),
            }
        });
    });
}

fn get_save_outcome(save_status: &mut DataState<()>) -> Option<SaveState> {
    match save_status {
        DataState::None => {
            // No action no save ongoing
            None
        }
        DataState::AwaitingResponse(rx) => {
            if let Some(new_state) = DataState::await_data(rx) {
                *save_status = new_state;
            }
            Some(SaveState::Ongoing)
        }
        DataState::Present(_data) => Some(SaveState::Completed),
        DataState::Failed(e) => Some(SaveState::Failed(format!("Save failed. {e}"))),
    }
}
