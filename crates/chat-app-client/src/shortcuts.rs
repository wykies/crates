use egui::{Key, KeyboardShortcut, Modifiers};

/// Returns true if the button is clicked or the shortcut is pressed
///
/// Note: This makes it the case that the code for both the button and the
/// shortcut press will do the same thing and you cannot use the shortcut to
/// bypass the button when it is not showing
pub fn shortcut_button(
    ui: &mut egui::Ui,
    caption: impl Into<egui::WidgetText>,
    hint_msg: &str,
    shortcut: &KeyboardShortcut,
) -> bool {
    ui.button(caption)
        .on_hover_text(shortcut_hint_text(ui, hint_msg, shortcut))
        .clicked()
        || ui.input_mut(|i| i.consume_shortcut(shortcut))
}

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

fn shortcut_hint_text(ui: &mut egui::Ui, hint_msg: &str, shortcut: &KeyboardShortcut) -> String {
    let space = if hint_msg.is_empty() { "" } else { " " };
    format!("{hint_msg}{space}({})", ui.ctx().format_shortcut(shortcut))
}
