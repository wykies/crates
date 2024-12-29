use tracing::info;

use crate::{displayable_page_common, ChatApp};

use super::DisplayablePage;

#[derive(Debug, serde::Serialize, serde::Deserialize, Default)]
#[serde(default)]
pub struct UiEguiSettings {
    is_open: bool,
    page_unique_number: usize,
    #[serde(skip)]
    prev_ui_options: Option<egui::Options>,
}
impl UiEguiSettings {
    fn save_current_ui_options(&mut self, ctx: egui::Context) {
        let current_ui_options = ctx.options(|o| o.clone());
        self.prev_ui_options = Some(current_ui_options);
        let visuals = ctx.style().visuals.clone();
        ctx.data_mut(|w| w.insert_persisted(egui::Id::new(ChatApp::VISUALS_KEY), visuals));
        info!("Saved UI Visuals");
    }
}

impl DisplayablePage for UiEguiSettings {
    displayable_page_common!("UI Settings", &[]);

    fn reset_to_default(&mut self, _: super::private::Token) {}

    fn show(&mut self, ui: &mut eframe::egui::Ui, _data_shared: &mut crate::DataShared) {
        let ctx = ui.ctx().clone();
        ctx.settings_ui(ui);
        match self.prev_ui_options.as_ref() {
            Some(prev) => {
                if ctx.options(|o| o != prev) {
                    self.save_current_ui_options(ctx)
                }
            }
            None => self.save_current_ui_options(ctx),
        }
    }
}
