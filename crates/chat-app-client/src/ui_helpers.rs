use secrecy::{ExposeSecret as _, SecretString};

pub fn get_text_height(ui: &mut egui::Ui) -> f32 {
    egui::TextStyle::Body
        .resolve(ui.style())
        .size
        .max(ui.spacing().interact_size.y)
}

pub fn ui_password_edit(
    ui: &mut egui::Ui,
    password: &mut SecretString,
    hint_text: &str,
) -> egui::Response {
    let mut temp = password.expose_secret().to_owned();
    let result = ui.add(
        egui::TextEdit::singleline(&mut temp)
            .password(true)
            .hint_text(hint_text),
    );
    *password = SecretString::from(temp);
    result
}

pub fn readonly_checkbox_no_text(ui: &mut egui::Ui, mut value: bool) {
    ui.add_enabled(false, egui::Checkbox::without_text(&mut value));
}

/// Convenience function to create escape buttons
pub fn ui_escape_button(ui: &mut egui::Ui, caption: impl Into<egui::WidgetText>) -> bool {
    crate::shortcuts::shortcut_button(
        ui,
        caption,
        "",
        &egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::Escape),
    )
}
