use anyhow::Context;
use connected_users::ConnectedUsers;
use egui::{
    scroll_area::ScrollBarVisibility, Align, KeyboardShortcut, Layout, Modifiers, ScrollArea,
};
use ewebsock::{WsEvent, WsMessage};
use plugin_chat::{
    consts::{
        CHAT_HISTORY_REQUEST_SIZE, CHAT_MIN_TIME_BETWEEN_HISTORY_REQUESTS, CHAT_SYSTEM_USERNAME,
    },
    ChatIM, ChatImText, ChatMsg, ReqHistoryBody,
};
use tracing::{error, info};
use wykies_client_core::WebSocketConnection;
use wykies_shared::{internal_error, uac::Username};
use wykies_time::Timestamp;

mod connected_users;

/// Scrolling to the bottom once doesn't seem to always get you there especially
/// if messages are still coming in
const SCROLLS_TO_GET_TO_BOTTOM: u8 = 2;

#[derive(Debug)]
pub struct FrontEnd {
    username: Username,
    system_username: Username,
    page_unique_name: String,
    ims: Vec<ChatIM>,
    text_to_send: String,
    error_status: Option<ChatUiError>,
    scroll_to_bottom: Option<u8>,
    connected_users: ConnectedUsers,
    last_history_request: Timestamp,
}

#[derive(Debug)]
struct ChatUiError {
    msg: String,
    is_transient: bool,
}

impl FrontEnd {
    pub fn new(username: Username, page_unique_name: String) -> Self {
        Self {
            username,
            system_username: Username::try_from(CHAT_SYSTEM_USERNAME)
                .expect("username is from a constant should either always work or always fail"),
            page_unique_name,
            ims: Default::default(),
            text_to_send: Default::default(),
            error_status: Default::default(),
            scroll_to_bottom: Default::default(),
            connected_users: Default::default(),
            last_history_request: Timestamp::now(),
        }
    }

    pub fn show(&mut self, ui: &mut eframe::egui::Ui, connection: &mut WebSocketConnection) {
        if self.error_status.is_none() {
            self.check_for_server_msgs(connection);
        }
        let half_height = ui.available_height() / 2.;
        egui::TopBottomPanel::bottom(self.generate_id("bottom panel"))
            .resizable(true)
            .max_height(half_height)
            .show_inside(ui, |ui| {
                if self.error_status.is_none() {
                    self.ui_send_area(ui, connection)
                } else {
                    self.ui_error_msg(ui)
                }
            });

        egui::SidePanel::right(self.generate_id("connected users"))
            .min_width(20.)
            .show_inside(ui, |ui| self.ui_connected_users(ui));

        egui::CentralPanel::default().show_inside(ui, |ui| self.ui_messages(ui, connection));
    }

    fn check_for_server_msgs(&mut self, connection: &mut WebSocketConnection) {
        while let Some(event) = connection.rx.try_recv() {
            info!(?event, "Event received");
            match event {
                WsEvent::Opened => {
                    // Expected to have been received by the client core
                    self.set_error_transient(internal_error!("unexpected opened event received"));
                    return;
                }
                WsEvent::Message(ws_msg) => match ws_msg {
                    WsMessage::Binary(_) => {
                        self.set_error_transient(internal_error!(
                            "unexpected binary message received"
                        ));
                        return;
                    }
                    WsMessage::Text(text) => match serde_json::from_str(&text) {
                        Ok(chat_msg) => {
                            if self.process_chat_msg(chat_msg).is_err() {
                                break;
                            }
                        }
                        Err(err) => {
                            error!(?err);
                            self.set_error_unrecoverable(internal_error!(
                                "Received a malformed message from the server"
                            ));
                            return;
                        }
                    },
                    WsMessage::Unknown(unknown_msg_content) => {
                        error!(
                            ?unknown_msg_content,
                            "unknown message received over websocket"
                        );
                        self.set_error_transient(internal_error!(
                            "unexpected `unknown` message received"
                        ));
                        return;
                    }
                    WsMessage::Ping(_) | WsMessage::Pong(_) => {
                        //  Do nothing seems to be handled by the library
                    }
                },
                WsEvent::Error(err) => {
                    error!(?err, "error received in websocket stream");
                    self.set_error_unrecoverable(internal_error!(
                        "error received in websocket stream"
                    ));
                    return;
                }
                WsEvent::Closed => {
                    self.set_error_unrecoverable("Connection Closed By Server - Reopen window");
                    return;
                }
            }
        }
    }

    fn process_chat_msg(&mut self, chat_msg: ChatMsg) -> Result<(), ()> {
        match chat_msg {
            ChatMsg::UserJoined(user) => {
                let sys_msg = self.system_msg(format!("{user} connected"))?;
                self.ims.push(sys_msg);
                self.connected_users.user_joined(user);
            }
            ChatMsg::UserLeft(user) => {
                let sys_msg = self.system_msg(format!("{user} disconnected"))?;
                self.ims.push(sys_msg);
                if let Err(err) = self
                    .connected_users
                    .user_left(user)
                    .context("removing user failed")
                {
                    error!(?err);
                    self.set_error_unrecoverable("error occurred trying to disconnect user")
                }
            }
            ChatMsg::IM(im) => {
                self.ims.push(im);
            }
            ChatMsg::InitialState(state) => {
                let connected_users = state.connected_users;
                let history_ims = state.history.ims;
                self.connected_users.merge_initial_users(connected_users);
                self.prepend_ims_history(history_ims);
            }
            ChatMsg::ReqHistory(req_history_body) => {
                error!("Received a request for history: {req_history_body:?}");
                self.set_error_transient(internal_error!(
                    "unexpected request for history received from the server"
                ));
                return Err(());
            }
            ChatMsg::RespHistory(body) => {
                self.prepend_ims_history(body.ims);
            }
        }
        Ok(())
    }

    fn ui_send_area(&mut self, ui: &mut egui::Ui, connection: &mut WebSocketConnection) {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::BOTTOM), |ui| {
            let bytes_left = ChatImText::MAX_LENGTH as i32 - self.text_to_send.len() as i32;
            if bytes_left <= ChatImText::MAX_LENGTH as i32 / 10 {
                let color = match bytes_left {
                    0 => ui.visuals().warn_fg_color,
                    1.. => ui.visuals().strong_text_color(),
                    ..0 => ui.visuals().error_fg_color,
                };
                ui.colored_label(color, format!("{bytes_left: >3} left",))
                    .on_hover_text("Number of bytes left");
            } else {
                // To prevent the edit from losing focus when the number of controls changes
                ui.label("");
            }

            ui.horizontal_centered(|ui| {
                if ui.button("Send").clicked() {
                    self.send_msg(connection);
                }
            });

            let key_combination_for_new_line = KeyboardShortcut {
                modifiers: Modifiers::SHIFT,
                logical_key: egui::Key::Enter,
            };
            let edit_response = ui.add_sized(
                ui.available_size(),
                egui::TextEdit::multiline(&mut self.text_to_send)
                    .return_key(Some(key_combination_for_new_line))
                    .hint_text("Message to send")
                    .char_limit(ChatImText::MAX_LENGTH),
            );
            if edit_response.has_focus()
                && ui.input_mut(|i| {
                    !i.consume_shortcut(&key_combination_for_new_line)
                        && i.key_pressed(egui::Key::Enter)
                })
            {
                self.send_msg(connection);
                edit_response.request_focus();
            }
        });
    }

    fn send_msg(&mut self, connection: &mut WebSocketConnection) {
        if self.text_to_send.is_empty() {
            return;
        }
        if self.text_to_send.len() > ChatImText::MAX_LENGTH {
            self.set_error_transient(format!(
                "Max bytes allowed is {} but {} currently used.
NB: Number of bytes is not equal the number of characters, eg. emojis use multiple bytes each",
                ChatImText::MAX_LENGTH,
                self.text_to_send.len()
            ));
            return;
        }

        let content = match ChatImText::try_from(std::mem::take(&mut self.text_to_send)) {
            Ok(x) => x,
            Err(err) => {
                // We lose the users input in this case but we have a guard at the top that
                // should cause this to never happen
                error!(?err);
                self.set_error_transient(err.to_string());
                return;
            }
        };
        let chat_msg = ChatMsg::IM(ChatIM {
            author: self.username.clone(),
            timestamp: Timestamp::now(),
            content,
        });
        connection.tx.send(WsMessage::Text(
            serde_json::to_string(&chat_msg).expect("failed to serialize chat msg for IM"),
        ));
        self.request_scroll_to_bottom();
    }

    fn ui_messages(&mut self, ui: &mut egui::Ui, connection: &mut WebSocketConnection) {
        ScrollArea::vertical()
            .auto_shrink(false)
            .stick_to_bottom(true)
            .scroll_bar_visibility(ScrollBarVisibility::AlwaysVisible)
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    let now = Timestamp::now();
                    let min_time_stamp_for_request =
                        self.last_history_request + CHAT_MIN_TIME_BETWEEN_HISTORY_REQUESTS;
                    if min_time_stamp_for_request < now {
                        if ui.button("Load more history").clicked() {
                            self.request_more_history(connection);
                        }
                    } else {
                        let time_left = now.abs_diff(min_time_stamp_for_request);
                        ui.add_enabled(
                            false,
                            egui::Button::new(format!("Load more history ({time_left})")),
                        );
                    }
                });
                for im in self.ims.iter() {
                    let mut frame = egui::Frame::default().inner_margin(4.0).begin(ui);
                    {
                        let ui = &mut frame.content_ui;
                        ui.with_layout(
                            Layout::top_down(Align::LEFT).with_cross_justify(true),
                            |ui| {
                                let color = match &im.author {
                                    x if x == &self.username => ui.visuals().strong_text_color(),
                                    x if x == &self.system_username => {
                                        ui.visuals().weak_text_color()
                                    }
                                    _ => ui.visuals().text_color(),
                                };
                                ui.colored_label(color, format!("{im}"))
                                    .on_hover_text(im.timestamp.display_as_locale_datetime());
                            },
                        );
                    }
                    let response = frame.allocate_space(ui);
                    if response.hovered() {
                        frame.frame.fill = ui.visuals().faint_bg_color;
                    }
                    frame.paint(ui);
                }
                if let Some(left) = self.scroll_to_bottom.as_mut() {
                    if *left == 0 {
                        self.scroll_to_bottom = None;
                        return;
                    }
                    ui.scroll_to_cursor(Some(Align::BOTTOM));
                    *left -= 1;
                }
            });
    }

    fn ui_connected_users(&mut self, ui: &mut egui::Ui) {
        ui.heading("Connected Users");
        for (user, qty) in self.connected_users.iter() {
            ui.label(format!("{user} ({qty})"));
        }
    }

    /// Prerequisite: Error must be set
    fn ui_error_msg(&mut self, ui: &mut egui::Ui) {
        let Some(ChatUiError { msg, is_transient }) = self.error_status.as_ref() else {
            self.set_error_transient(internal_error!("failed to find original error to show"));
            return;
        };
        ui.colored_label(ui.visuals().error_fg_color, msg);
        if *is_transient {
            ui.vertical_centered(|ui| {
                if ui.button("Clear Error Status").clicked() {
                    self.error_status = None;
                }
            });
        } else {
            ui.colored_label(
                ui.visuals().strong_text_color(),
                "Unable to clear this error please try reopening the window",
            );
        }
    }

    /// Needed to prevent duplicate ID in different windows
    fn generate_id(&self, id_name: &str) -> String {
        format!("{id_name}{}", self.page_unique_name)
    }

    fn request_scroll_to_bottom(&mut self) {
        self.scroll_to_bottom = Some(SCROLLS_TO_GET_TO_BOTTOM);
    }

    fn set_error_unrecoverable(&mut self, msg: impl Into<String>) {
        self.error_status = Some(ChatUiError {
            msg: msg.into(),
            is_transient: false,
        });
    }

    fn set_error_transient(&mut self, msg: impl Into<String>) {
        self.error_status = Some(ChatUiError {
            msg: msg.into(),
            is_transient: true,
        });
    }

    fn prepend_ims_history(&mut self, mut history_ims: Vec<ChatIM>) {
        // Sort lists to ensure ordered by timestamp
        self.ims.sort_by_key(|x| x.timestamp);
        history_ims.sort_by_key(|x| x.timestamp);

        // Put these at the start of our history by swapping with current history and
        // then adding in the current history
        std::mem::swap(&mut history_ims, &mut self.ims);

        // Remove any duplicates from the end of end of what was returned.
        let mut ims_from_before_history = history_ims; // Swapped above
        let possibly_duplicated_timestamp = ims_from_before_history.first().map(|x| x.timestamp);
        if let Some(possibly_duplicated_timestamp) = possibly_duplicated_timestamp {
            // Get range of possibly duplicated values
            let mut last_index_with_same_timestamp = 0;
            for (i, im) in ims_from_before_history.iter().enumerate() {
                if im.timestamp == possibly_duplicated_timestamp {
                    last_index_with_same_timestamp = i;
                } else {
                    break;
                }
            }
            let range_to_consider = &ims_from_before_history[..=last_index_with_same_timestamp];

            // Only keep non-duplicated values
            self.ims.retain(|x| {
                x.timestamp != possibly_duplicated_timestamp || !range_to_consider.contains(x)
            });
        }

        self.ims.append(&mut ims_from_before_history);
    }

    fn system_msg(&mut self, content: String) -> Result<ChatIM, ()> {
        let content = match content
            .try_into()
            .context("failed to generate system message")
        {
            Ok(x) => x,
            Err(err) => {
                error!(?err, "failed to generate system message");
                self.set_error_unrecoverable("Error generating system message");
                return Err(());
            }
        };
        Ok(ChatIM {
            author: self.system_username.clone(),
            timestamp: Timestamp::now(),
            content,
        })
    }

    fn request_more_history(&mut self, connection: &mut WebSocketConnection) {
        self.last_history_request = Timestamp::now();
        let qty = CHAT_HISTORY_REQUEST_SIZE;
        let latest_timestamp = self
            .ims
            .first()
            .map(|chat_im| chat_im.timestamp)
            .unwrap_or_else(Timestamp::now);
        let chat_msg = ChatMsg::ReqHistory(ReqHistoryBody::new(qty, latest_timestamp));
        connection.tx.send(WsMessage::Text(
            serde_json::to_string(&chat_msg)
                .expect("failed to serialize chat msg for history request"),
        ));
    }
}
