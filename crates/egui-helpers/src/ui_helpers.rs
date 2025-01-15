use egui::{Checkbox, KeyboardShortcut, Response, RichText, WidgetText};
use secrecy::{ExposeSecret as _, SecretString};

/// Adds convenance functions to [`egui::Ui`]
pub trait UiHelpers {
    fn label_truncate(&mut self, text: impl Into<WidgetText>) -> Response;
    fn error_label(&mut self, text: impl Into<RichText>) -> Response;
    fn text_height(&mut self) -> f32;
    fn password_edit(&mut self, password: &mut SecretString, hint_text: &str) -> Response;
    fn readonly_checkbox_no_text(&mut self, value: bool) -> Response;
    fn escape_button(&mut self, text: impl Into<WidgetText>) -> bool;
    fn was_enter_pressed(&self) -> bool;
    fn shortcut_hint_text(&mut self, hint_msg: &str, shortcut: &KeyboardShortcut) -> String;
    fn shortcut_button(
        &mut self,
        text: impl Into<WidgetText>,
        hint_msg: &str,
        shortcut: &KeyboardShortcut,
    ) -> bool;
    fn removable_items_list<T: RemovableItem>(&mut self, backing: &mut Vec<T>, empty_msg: &str);
}

/// Provides the behaviour required for the removable item list
pub trait RemovableItem {
    fn widget_text(&self) -> impl Into<WidgetText>;
    fn is_enabled(&self) -> bool {
        // Not able to be enabled by default
        false
    }

    fn set_enabled(&mut self, value: bool) {
        // Does nothing by default
        _ = value;
    }
}

impl UiHelpers for egui::Ui {
    fn label_truncate(&mut self, text: impl Into<WidgetText>) -> Response {
        self.add(egui::Label::new(text).truncate())
    }

    fn error_label(&mut self, text: impl Into<RichText>) -> Response {
        self.colored_label(self.visuals().error_fg_color, text)
    }

    fn text_height(&mut self) -> f32 {
        egui::TextStyle::Body
            .resolve(self.style())
            .size
            .max(self.spacing().interact_size.y)
    }

    fn password_edit(&mut self, password: &mut SecretString, hint_text: &str) -> Response {
        let mut temp = password.expose_secret().to_owned();
        let result = self.add(
            egui::TextEdit::singleline(&mut temp)
                .password(true)
                .hint_text(hint_text),
        );
        *password = SecretString::from(temp);
        result
    }

    fn readonly_checkbox_no_text(&mut self, mut value: bool) -> Response {
        self.add_enabled(false, Checkbox::without_text(&mut value))
    }

    /// Shows a button that is bound to the escape shortcut hotkey
    fn escape_button(&mut self, text: impl Into<WidgetText>) -> bool {
        self.shortcut_button(
            text,
            "",
            &KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::Escape),
        )
    }

    /// Returns true if the enter key was pressed this frame
    fn was_enter_pressed(&self) -> bool {
        self.input(|i| i.key_pressed(egui::Key::Enter))
    }

    /// Returns true if the button is clicked or the shortcut is pressed
    ///
    /// Note: This makes it the case that the code for both the button and the
    /// shortcut press will do the same thing and you cannot use the shortcut to
    /// bypass the button when it is not showing
    fn shortcut_button(
        &mut self,
        text: impl Into<WidgetText>,
        hint_msg: &str,
        shortcut: &KeyboardShortcut,
    ) -> bool {
        self.button(text)
            .on_hover_text(self.shortcut_hint_text(hint_msg, shortcut))
            .clicked()
            || self.input_mut(|i| i.consume_shortcut(shortcut))
    }

    /// Returns a string representation of a shortcut with a hint if supplied
    fn shortcut_hint_text(&mut self, hint_msg: &str, shortcut: &KeyboardShortcut) -> String {
        let space = if hint_msg.is_empty() { "" } else { " " };
        format!(
            "{hint_msg}{space}({})",
            self.ctx().format_shortcut(shortcut)
        )
    }

    /// Adds labels with x's the left that if clicked remove the item from the backing vector
    fn removable_items_list<T: RemovableItem>(&mut self, backing: &mut Vec<T>, empty_msg: &str) {
        if backing.is_empty() {
            self.label(empty_msg);
        }
        let mut to_deactivate = Vec::new();
        for (i, item) in backing.iter_mut().enumerate() {
            let mut is_enabled = item.is_enabled();
            self.horizontal(|ui| {
                let is_enabled_before = is_enabled;
                if ui.button("x").clicked() {
                    to_deactivate.push(i); // Mark page for removal
                }
                if ui
                    .toggle_value(&mut is_enabled, item.widget_text())
                    .middle_clicked()
                {
                    to_deactivate.push(i); // Mark page for removal
                };
                if is_enabled != is_enabled_before {
                    item.set_enabled(is_enabled);
                }
            });
        }

        // Deactivate marked pages
        to_deactivate.sort_unstable(); // Should already be sorted but put here because it is assumed in following loop
        while let Some(marked_index) = to_deactivate.pop() {
            backing.remove(marked_index);
        }
    }
}
