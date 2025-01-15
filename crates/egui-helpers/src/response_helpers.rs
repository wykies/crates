use egui::Response;

/// Adds convenance functions to [`egui::Response`]
pub trait ResponseHelpers {
    /// Returns true if the Response lost focus and the enter key was pressed
    fn enter_pressed(&mut self, ui: &egui::Ui) -> bool;
}

impl ResponseHelpers for Response {
    fn enter_pressed(&mut self, ui: &egui::Ui) -> bool {
        self.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))
    }
}
