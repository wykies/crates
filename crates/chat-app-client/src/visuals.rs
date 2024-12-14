use egui::Color32;
use tracing::warn;

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CustomVisuals {
    /// Used to adjust background for dark_mode and restore on next startup
    last_dark_mode: bool,

    /// Used to restore light/dark mode user was using as egui always seems to
    /// start in dark (and apply custom colors on startup)
    #[serde(skip)]
    is_initialized: bool,
}

impl CustomVisuals {
    pub(crate) fn update_style(&mut self, ctx: &egui::Context) {
        let dark_mode = ctx.style().visuals.dark_mode;
        if !self.is_initialized {
            self.is_initialized = true;
            match (self.last_dark_mode, dark_mode) {
                (true, true) => self.bg_adjust(ctx),
                (true, false) => {
                    // Not expecting this case to happen based on testing
                    warn!("Not expecting the program to start in light mode");
                    // This will change it back to dark mode as the user was expecting
                    self.bg_adjust(ctx);
                }
                (false, true) => self.set_light_mode(ctx),
                (false, false) => (), // Nothing to do, they already match
            }
        } else if self.last_dark_mode != dark_mode {
            self.last_dark_mode = dark_mode;
            if dark_mode {
                self.bg_adjust(ctx);
            }
        }
    }

    fn bg_adjust(&self, ctx: &egui::Context) {
        // TODO 4: Remove customizations
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = Color32::from_gray(64);
        visuals.window_fill = Color32::from_gray(64);
        ctx.set_visuals(visuals);
    }

    fn set_light_mode(&self, ctx: &egui::Context) {
        ctx.set_visuals(egui::Visuals::light());
    }
}
