use crate::pages::{
    UiLogin, UiPage, change_password::UiChangePassword, chat::UiChat,
    egui_settings::UiEguiSettings, uac::UiUAC,
};
use crate::shortcuts::Shortcuts;
pub use data_shared::DataShared;
use egui_pages::{PageContainer as _, do_organize_pages};
use tracing::{info, warn};
use wykies_shared::uac::init_permissions_to_defaults;
use wykies_time::Timestamp;

const VERSION_STR: &str = concat!("ver: ", env!("CARGO_PKG_VERSION"));

mod data_shared;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct ChatApp {
    #[serde(skip)]
    login_page: Option<UiLogin>,
    data_shared: DataShared,
    active_pages: Vec<UiPage>,
    shortcuts: Shortcuts,
}

impl eframe::App for ChatApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        // TODO 4: Save settings per user
        info!("Saving with key: {}", eframe::APP_KEY);
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per
    /// second.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.data_shared.screen_lock_info_tick();
        self.top_panel(ui);
        self.bottom_panel(ui);
        self.show_pages(ui);

        // Request repaint after 1 second
        ui.request_repaint_after(std::time::Duration::from_secs(1));
    }
}

impl ChatApp {
    pub const VISUALS_KEY: &str = "visuals";

    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        init_permissions_to_defaults();
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.
        if let Some(visuals) = cc
            .egui_ctx
            .data_mut(|r| r.get_persisted::<egui::Visuals>(egui::Id::new(ChatApp::VISUALS_KEY)))
        {
            info!("Found saved Visuals. Loading...");
            cc.egui_ctx.set_visuals(visuals);
        } else {
            info!("Unable to load Visuals, no saved version found");
        }

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            info!("Storage found. Loading App State...");
            match eframe::get_value(storage, eframe::APP_KEY) {
                Some(value) => {
                    info!("App state loading succeeded");
                    value
                }
                None => {
                    warn!(
                        "App state loading failed, no value saved or loading failed (see message a debug level from egui if failed)"
                    );
                    Default::default()
                }
            }
        } else {
            info!("Unable to load app state, no storage found");
            Default::default()
        }
    }

    fn is_logged_in(&mut self) -> bool {
        self.data_shared.is_logged_in()
    }

    fn menu(&mut self, ui: &mut egui::Ui) {
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
        self.ui_menu_file(ui);
        self.ui_menu_pages(ui);
    }

    fn ui_menu_pages(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("Pages", |ui| {
            UiPage::ui_menu_page_btn::<UiChat>(ui, &self.data_shared, &mut self.active_pages)
                .expect("type is correct and defined at compile time");
            UiPage::ui_menu_page_btn::<UiUAC>(ui, &self.data_shared, &mut self.active_pages)
                .expect("type is correct and defined at compile time");
            UiPage::ui_menu_page_btn::<UiEguiSettings>(
                ui,
                &self.data_shared,
                &mut self.active_pages,
            )
            .expect("type is correct and defined at compile time");

            ui.separator();
            UiPage::ui_pages_management_controls(
                ui,
                &mut self.active_pages,
                &self.shortcuts.organize_pages,
            );
        });
    }

    fn top_panel(&mut self, ui: &mut egui::Ui) {
        // Single instance of global panel thus unique
        egui::Panel::top("top_panel").show_inside(ui, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                egui::widgets::global_theme_preference_switch(ui);
                if self.is_logged_in() && !self.is_locked() {
                    ui.separator();
                    self.menu(ui);
                }
                ui.label(VERSION_STR);
            });
        });
    }

    fn bottom_panel(&mut self, ui: &mut egui::Ui) {
        // Single instance of global panel thus unique
        egui::Panel::bottom("bottom_panel").show_inside(ui, |ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::BOTTOM), |ui| {
                ui.label(self.current_time());
                if self.is_logged_in() {
                    if ui.button("Logout").clicked() {
                        self.logout();
                    }
                    if !self.is_locked() && ui.button("Lock").clicked() {
                        self.lock();
                    }
                    if let Some(user_info) = self.data_shared.client.user_info() {
                        ui.label(format!("Logged in as {}", user_info.username));
                    }
                }
                egui::warn_if_debug_build(ui);
            });
        });
    }

    fn show_pages(&mut self, ui: &mut egui::Ui) {
        if !self.is_logged_in() || self.is_locked() {
            self.login_page
                .get_or_insert(Default::default())
                .show(ui, &mut self.data_shared);
        } else {
            self.login_page = None; // Clear out login page once we are logged in
            UiPage::ui_active_pages_panel(
                ui,
                &mut self.active_pages,
                &self.shortcuts.organize_pages,
            );
            UiPage::ui_display_pages(ui, &mut self.active_pages, &mut self.data_shared);
            self.process_shortcuts(ui);
        }
    }

    fn current_time(&self) -> String {
        Timestamp::now().display_as_utc_datetime_long()
    }

    fn logout(&mut self) {
        self.data_shared.client.logout_no_wait();

        // Convert pages to ron and back to remove state that should only stay when
        // logged in
        let pages = ron::to_string(&self.active_pages).expect("failed to parse pages to ron");
        self.active_pages =
            ron::from_str(&pages).expect("failed to convert back into pages from ron");
    }

    #[cfg_attr(target_arch = "wasm32", allow(unused_variables))]
    fn ui_menu_file(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("File", |ui| {
            UiPage::ui_menu_page_btn::<UiChangePassword>(
                ui,
                &self.data_shared,
                &mut self.active_pages,
            )
            .expect("type is correct and defined at compile time");

            // On the web the browser controls the zoom
            #[cfg(not(target_arch = "wasm32"))]
            {
                ui.separator();
                egui::gui_zoom::zoom_menu_buttons(ui);
                ui.weak(format!("Current zoom: {:.0}%", 100.0 * ui.zoom_factor()))
                    .on_hover_text(
                        "The UI zoom level, on top of the operating system's default value",
                    );
                ui.separator();
            }

            if ui.button("Lock").clicked() {
                self.lock();
                ui.close();
            }

            if ui.button("Logout").clicked() {
                self.logout();
                ui.close();
            }

            #[cfg(not(target_arch = "wasm32"))] // no File->Quit on web pages!
            if ui.button("Quit").clicked() {
                ui.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
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

impl Default for ChatApp {
    fn default() -> Self {
        // Preload `active_pages` with a chat page
        Self {
            login_page: Default::default(),
            data_shared: Default::default(),
            active_pages: vec![
                UiPage::type_to_instance::<UiChat>()
                    .expect("invalid page type")
                    .new_page_with_unique_number(0),
            ],
            shortcuts: Default::default(),
        }
    }
}
