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

use change_password::UiChangePassword;
use chat::UiChat;
use egui_helpers::RemovableItem;
use egui_pages::{DisplayablePage, PageContainer, PermissionValidator as _, show_page};
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
    Uac(UiUAC),
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
            UiPage::Uac($page) => $body,
        }
    };
}

impl PageContainer<DataShared, Permission, private::Token> for UiPage {
    #[tracing::instrument(ret)]
    fn new_page_with_unique_number<T: DisplayablePage<DataShared, Permission, private::Token>>(
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
                    UiPage::Uac(_) => {
                        Self::Uac(UiUAC::new_page(page_unique_number).and_open_page())
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

    fn display_page(&mut self, ui: &mut egui::Ui, data_shared: &mut DataShared) {
        do_on_ui_page!(self, page, { show_page(page, ui, data_shared) })
    }

    fn title_base(&self) -> &'static str {
        do_on_ui_page!(self, page, { page.title_base_from_instance() })
    }

    fn page_unique_number(&self) -> usize {
        do_on_ui_page!(self, page, { page.page_unique_number() })
    }

    fn is_page_open(&self) -> bool {
        do_on_ui_page!(self, page, { page.is_page_open() })
    }

    fn title(&self) -> String {
        do_on_ui_page!(self, page, { page.title() })
    }

    fn open_page(&mut self) {
        do_on_ui_page!(self, page, { page.open_page() })
    }

    fn close_page(&mut self) {
        do_on_ui_page!(self, page, { page.close_page() })
    }

    fn ui_menu_page_btn<T: DisplayablePage<DataShared, Permission, private::Token>>(
        ui: &mut egui::Ui,
        data_shared: &DataShared,
        active_pages: &mut Vec<Self>,
    ) {
        if !data_shared.has_permissions(T::page_permissions()) {
            return;
        }
        let base_title = T::title_base();
        if ui.button(base_title).clicked() {
            let mut max_id_found = None;
            for page in active_pages.iter_mut() {
                if page.title_base() == base_title {
                    max_id_found = max_id_found.max(Some(page.page_unique_number()))
                }
            }
            let new_num = if let Some(val) = max_id_found {
                val + 1
            } else {
                0
            };
            active_pages.push(Self::new_page_with_unique_number::<T>(new_num));
            ui.close();
        }
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
