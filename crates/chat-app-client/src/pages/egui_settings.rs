use crate::displayable_page_common;

use super::DisplayablePage;

#[derive(Debug, serde::Serialize, serde::Deserialize, Default)]
#[serde(default)]
pub struct UiEguiSettings {
    is_open: bool,
    page_unique_number: usize,
}

impl DisplayablePage for UiEguiSettings {
    displayable_page_common!("UI Settings", &[]);

    fn reset_to_default(&mut self, _: super::private::Token) {}

    fn show(&mut self, ui: &mut eframe::egui::Ui, _data_shared: &mut crate::DataShared) {
        let ctx = ui.ctx().clone();
        ctx.settings_ui(ui);
    }
}
