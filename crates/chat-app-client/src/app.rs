use egui::ScrollArea;
use tracing::{debug, error, instrument};
use tracing::{info, warn};
use wykies_client_core::WakeFn;
use wykies_shared::uac::{init_permissions_to_defaults, DisplayName};
use wykies_time::Timestamp;

use crate::lockout::ScreenLockInfo;
use crate::pages::{
    change_password::UiChangePassword, chat::UiChat, egui_settings::UiEguiSettings, uac::UiUAC,
    UiLogin, UiPage,
};
use crate::shortcuts::Shortcuts;
use crate::DisplayablePage;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
// TODO 2: Make chat page to show by default
#[derive(Debug, Default, serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct ChatApp {
    #[serde(skip)]
    login_page: Option<UiLogin>,
    data_shared: DataShared,
    active_pages: Vec<UiPage>,
    shortcuts: Shortcuts,
}

#[derive(Debug, Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct DataShared {
    pub username: String,

    #[serde(skip)]
    /// Allows for forcing password change and updating of data outside of the
    /// client-core
    is_login_completed: bool,
    #[serde(skip)]
    pub display_name: DisplayName,
    #[serde(skip)]
    // TODO 2: Add option for user to change the server they are connecting to (Saving a list of
    //          recent servers)
    pub client: wykies_client_core::Client,
    #[serde(skip)]
    screen_lock_info: ScreenLockInfo,
}

impl DataShared {
    /// Doesn't do anything if the client does not have user info
    #[instrument]
    pub(crate) fn mark_login_complete(&mut self) {
        if let Some(user_info) = self.client.user_info() {
            debug!("Updating username to {}", user_info.username);
            self.username = user_info.username.clone().into();
            self.display_name = user_info.display_name.clone();
            self.is_login_completed = true;
        } else {
            warn!("No user found in client");
        }
    }

    pub fn is_logged_in(&mut self) -> bool {
        if self.client.is_logged_in() {
            self.is_login_completed
        } else {
            self.is_login_completed = false; // Reset completed status (ensure reset after logout)
            false
        }
    }

    pub fn is_screen_locked(&mut self) -> bool {
        self.screen_lock_info.is_locked()
    }

    pub fn unlock(&mut self) {
        self.screen_lock_info.unlock()
    }

    pub fn lock(&mut self) {
        self.screen_lock_info.lock()
    }

    fn has_permissions<T: DisplayablePage>(&self) -> bool {
        let Some(permissions) = self.client.user_info().map(|user| user.permissions.clone()) else {
            error!(
                "Attempt to get user information when it doesn't exist. Isn't the user logged in?"
            );
            debug_assert!(false, "This shouldn't happen we should only be checking user information after login when it exists");
            return false;
        };
        T::has_permissions(&permissions)
    }
}

impl eframe::App for ChatApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        // TODO 4: Save settings per user
        info!("Saving with key: {}", eframe::APP_KEY);
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per
    /// second. Put your widgets into a `SidePanel`, `TopPanel`,
    /// `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.data_shared.screen_lock_info.tick();
        self.top_panel(ctx);
        self.bottom_panel(ctx);
        self.show_pages(ctx);

        // Request repaint after 1 second
        ctx.request_repaint_after(std::time::Duration::from_secs(1));
    }
}

impl ChatApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        init_permissions_to_defaults();
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            info!("Storage found. Loading...");
            match eframe::get_value(storage, eframe::APP_KEY) {
                Some(value) => {
                    info!("Loaded succeeded");
                    value
                }
                None => {
                    warn!("Load failed");
                    Default::default()
                }
            }
        } else {
            info!("No storage found");
            Default::default()
        }
    }

    fn is_logged_in(&mut self) -> bool {
        self.data_shared.is_logged_in()
    }

    fn menu(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
        self.ui_menu_file(ui, ctx);
        self.ui_menu_pages(ui);
    }

    fn ui_menu_pages(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("Pages", |ui| {
            self.ui_menu_page_btn::<UiChat>(ui);
            self.ui_menu_page_btn::<UiUAC>(ui);
            self.ui_menu_page_btn::<UiEguiSettings>(ui);

            ui.separator();
            if ui.button("Open All Pages").clicked() {
                self.open_all_pages();
                ui.close_menu();
            }
            if ui.button("Close All Pages").clicked() {
                self.close_all_pages();
                ui.close_menu();
            }
            if ui.button("Deactivate All Pages").clicked() {
                self.deactivate_all_pages();
                ui.close_menu();
            }
            if ui.button("Sort Pages By Name").clicked() {
                self.sort_pages_by_name();
                ui.close_menu();
            }
            if ui
                .add(
                    egui::Button::new("Organize Pages")
                        .shortcut_text(ui.ctx().format_shortcut(&self.shortcuts.organize_pages)),
                )
                .clicked()
            {
                do_organize_pages(ui);
                ui.close_menu();
            }
        });
    }

    fn top_panel(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                egui::widgets::global_theme_preference_switch(ui);
                if self.is_logged_in() && !self.is_locked() {
                    ui.separator();
                    self.menu(ui, ctx);
                }
            });
        });
    }

    fn bottom_panel(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::BOTTOM), |ui| {
                ui.label(self.current_time());
                if self.is_logged_in() {
                    if ui.button("Logout").clicked() {
                        self.logout();
                    }
                    if !self.is_locked() && ui.button("Lock").clicked() {
                        self.lock();
                    }
                    ui.label(format!("Logged in as {}", self.data_shared.display_name));
                }
                egui::warn_if_debug_build(ui);
            });
        });
    }

    fn show_pages(&mut self, ctx: &egui::Context) {
        if !self.is_logged_in() || self.is_locked() {
            self.login_page
                .get_or_insert(Default::default())
                .show(ctx, &mut self.data_shared);
        } else {
            self.ui_active_pages_panel(ctx);
            self.login_page = None; // Clear out login page once we are logged in
            for page in self.active_pages.iter_mut() {
                page.display_page(ctx, &mut self.data_shared);
            }
        }
    }

    fn current_time(&self) -> String {
        Timestamp::now().display_as_locale_datetime()
    }

    fn logout(&mut self) {
        self.data_shared.client.logout_no_wait();

        // Convert pages to json and back to remove state that should only stay when
        // logged in
        let pages =
            serde_json::to_string(&self.active_pages).expect("failed to parse pages to json");
        self.active_pages =
            serde_json::from_str(&pages).expect("failed to convert back into pages from json");
    }

    fn ui_menu_page_btn<T: DisplayablePage>(&mut self, ui: &mut egui::Ui) {
        if !self.data_shared.has_permissions::<T>() {
            return;
        }
        let base_title = T::title_base();
        if ui.button(base_title).clicked() {
            let mut max_id_found = None;
            for page in self.active_pages.iter_mut() {
                if page.title_base() == base_title {
                    max_id_found = max_id_found.max(Some(page.page_unique_number()))
                }
            }
            let new_num = if let Some(val) = max_id_found {
                val + 1
            } else {
                0
            };
            self.active_pages
                .push(UiPage::new_page_with_unique_number::<T>(new_num));
            ui.close_menu();
        }
    }

    #[cfg_attr(target_arch = "wasm32", allow(unused_variables))]
    fn ui_menu_file(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.menu_button("File", |ui| {
            self.ui_menu_page_btn::<UiChangePassword>(ui);

            // On the web the browser controls the zoom
            #[cfg(not(target_arch = "wasm32"))]
            {
                ui.separator();
                egui::gui_zoom::zoom_menu_buttons(ui);
                ui.weak(format!(
                    "Current zoom: {:.0}%",
                    100.0 * ui.ctx().zoom_factor()
                ))
                .on_hover_text("The UI zoom level, on top of the operating system's default value");
                ui.separator();
            }

            if ui.button("Lock").clicked() {
                self.lock();
                ui.close_menu();
            }

            if ui.button("Logout").clicked() {
                self.logout();
                ui.close_menu();
            }

            #[cfg(not(target_arch = "wasm32"))] // no File->Quit on web pages!
            if ui.button("Quit").clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
    }

    fn ui_active_pages_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("side_panel")
            .resizable(false)
            .default_width(200.0)
            .show(ctx, |ui| {
                self.process_shortcuts(ui);

                ui.vertical_centered(|ui| {
                    ui.heading("Active Pages");
                });

                ui.separator();

                self.ui_pages_list(ui);
            });
    }

    fn ui_pages_list(&mut self, ui: &mut egui::Ui) {
        ScrollArea::vertical().show(ui, |ui| {
            ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                if self.active_pages.is_empty() {
                    ui.label("NO PAGES ARE ACTIVE.\nUse top menu to activate a page");
                }
                let mut to_deactivate = Vec::new();
                for (i, page) in self.active_pages.iter_mut().enumerate() {
                    let mut is_open = page.is_page_open();
                    ui.horizontal(|ui| {
                        let is_open_before = is_open;
                        if ui.button("x").clicked() {
                            to_deactivate.push(i); // Mark page for removal
                        }
                        if ui.toggle_value(&mut is_open, page.title()).middle_clicked() {
                            to_deactivate.push(i); // Mark page for removal
                        };
                        if is_open != is_open_before {
                            if is_open {
                                page.open_page();
                            } else {
                                page.close_page();
                            }
                        }
                    });
                }

                // Deactivate marked pages
                to_deactivate.sort_unstable(); // Should already be sorted but put here because it is assumed in following loop
                while let Some(marked_index) = to_deactivate.pop() {
                    self.active_pages.remove(marked_index);
                }

                ui.separator();

                if ui.button("Open All Pages").clicked() {
                    self.open_all_pages();
                }
                if ui.button("Close All Pages").clicked() {
                    self.close_all_pages();
                }
                if ui.button("Deactivate All Pages").clicked() {
                    self.deactivate_all_pages();
                }
                if ui.button("Sort Pages by Name").clicked() {
                    self.sort_pages_by_name();
                }
                if ui
                    .add(
                        egui::Button::new("Organize Pages").shortcut_text(
                            ui.ctx().format_shortcut(&self.shortcuts.organize_pages),
                        ),
                    )
                    .clicked()
                {
                    do_organize_pages(ui);
                }
            });
        });
    }

    fn deactivate_all_pages(&mut self) {
        self.active_pages.clear();
    }

    fn close_all_pages(&mut self) {
        self.active_pages
            .iter_mut()
            .for_each(|page| page.close_page())
    }

    fn open_all_pages(&mut self) {
        self.active_pages
            .iter_mut()
            .for_each(|page| page.open_page())
    }

    fn sort_pages_by_name(&mut self) {
        self.active_pages.sort_by_key(|x| x.title());
    }

    fn process_shortcuts(&mut self, ui: &mut egui::Ui) {
        if ui.input_mut(|i| i.consume_shortcut(&self.shortcuts.organize_pages)) {
            do_organize_pages(ui);
        }
    }

    fn is_locked(&mut self) -> bool {
        self.data_shared.is_screen_locked()
    }

    fn lock(&mut self) {
        self.data_shared.lock();
    }
}

fn do_organize_pages(ui: &mut egui::Ui) {
    ui.ctx().memory_mut(|mem| mem.reset_areas());
}

#[inline]
pub fn wake_fn(ctx: egui::Context) -> impl WakeFn {
    move || ctx.request_repaint()
}
