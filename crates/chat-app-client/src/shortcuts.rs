use egui::{Key, KeyboardShortcut, Modifiers};

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct Shortcuts {
    pub organize_pages: KeyboardShortcut,
}

impl Default for Shortcuts {
    fn default() -> Self {
        Self {
            organize_pages: KeyboardShortcut::new(Modifiers::CTRL | Modifiers::SHIFT, Key::R),
        }
    }
}
