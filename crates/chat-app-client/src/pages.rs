use crate::DataShared;
pub mod change_password;
pub mod chat;
pub mod egui_settings;
pub mod login;
pub mod uac;

mod private {
    #[derive(Default)]
    /// Used to make some trait methods private
    pub struct Token;
}
/// Used to allow outside types to refer to the trait without needing the
/// private token
pub trait DisplayablePageExternal: DisplayablePage<DataShared, Permission, private::Token> {}
impl<U: DisplayablePage<DataShared, Permission, private::Token>> DisplayablePageExternal for U {}

use change_password::UiChangePassword;
use chat::UiChat;
use egui_helpers::RemovableItem;
use egui_pages::{DisplayablePage, show_page};
use egui_settings::UiEguiSettings;
pub use login::UiLogin;
use strum::{EnumIter, IntoEnumIterator};
use tracing::error;
use uac::UiUAC;
use wykies_shared::uac::Permission;

#[derive(Debug, serde::Serialize, serde::Deserialize, EnumIter)]
pub enum UiPage {
    ChangePassword(UiChangePassword),
    Chat(UiChat),
    EguiSetting(UiEguiSettings),
    UAC(UiUAC),
}

impl RemovableItem for UiPage {
    fn widget_text(&self) -> impl Into<egui::WidgetText> {
        self.title()
    }

    fn is_enabled(&self) -> bool {
        self.is_page_open()
    }

    fn set_enabled(&mut self, value: bool) {
        if value {
            self.open_page();
        } else {
            self.close_page();
        }
    }
}

macro_rules! do_on_ui_page {
    ($on:ident, $page:ident, $body:tt) => {
        match $on {
            UiPage::ChangePassword($page) => $body,
            UiPage::Chat($page) => $body,
            UiPage::EguiSetting($page) => $body,
            UiPage::UAC($page) => $body,
        }
    };
}

impl UiPage {
    #[tracing::instrument(ret)]
    pub fn new_page_with_unique_number<T: DisplayablePageExternal>(
        page_unique_number: usize,
    ) -> UiPage {
        for page in Self::iter() {
            if page.title_base() == T::title_base() {
                return match page {
                    UiPage::Chat(_) => {
                        Self::Chat(UiChat::new_page(page_unique_number).and_open_page())
                    }
                    UiPage::ChangePassword(_) => Self::ChangePassword(
                        UiChangePassword::new_page(page_unique_number).and_open_page(),
                    ),
                    UiPage::EguiSetting(_) => Self::EguiSetting(
                        UiEguiSettings::new_page(page_unique_number).and_open_page(),
                    ),
                    UiPage::UAC(_) => {
                        Self::UAC(UiUAC::new_page(page_unique_number).and_open_page())
                    }
                };
            }
        }
        let msg = format!(
            "execution should never get here. All pages should be able to be found but {:?} not found",
            T::title_base()
        );
        error!("{msg}");
        unreachable!("{msg}");
    }

    pub fn display_page(&mut self, ui: &mut egui::Ui, data_shared: &mut DataShared) {
        do_on_ui_page!(self, page, { show_page(page, ui, data_shared) })
    }

    pub fn title_base(&self) -> &'static str {
        do_on_ui_page!(self, page, { page.title_base_from_instance() })
    }

    pub fn page_unique_number(&self) -> usize {
        do_on_ui_page!(self, page, { page.page_unique_number() })
    }

    pub fn is_page_open(&self) -> bool {
        do_on_ui_page!(self, page, { page.is_page_open() })
    }

    pub fn title(&self) -> String {
        do_on_ui_page!(self, page, { page.title() })
    }

    pub fn open_page(&mut self) {
        do_on_ui_page!(self, page, { page.open_page() })
    }

    pub fn close_page(&mut self) {
        do_on_ui_page!(self, page, { page.close_page() })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn all_page_base_names_are_unique() {
        let mut set: HashSet<&str> = Default::default();
        for page in UiPage::iter() {
            let title_base = page.title_base();
            let is_unique = set.insert(title_base);
            assert!(
                is_unique,
                "Duplicate page title base name found: {title_base}"
            );
        }
    }
}
