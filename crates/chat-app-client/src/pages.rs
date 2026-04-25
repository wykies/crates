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

use anyhow::bail;
use change_password::UiChangePassword;
use chat::UiChat;
use egui_helpers::RemovableItem;
use egui_pages::{DisplayablePage, PageContainer, show_page};
use egui_settings::UiEguiSettings;
pub use login::UiLogin;
use strum::{EnumIter, IntoEnumIterator};
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
    fn new_page_with_unique_number(&self, page_unique_number: usize) -> Self {
        match self {
            UiPage::Chat(_) => Self::Chat(UiChat::new_page(page_unique_number).and_open_page()),
            UiPage::ChangePassword(_) => {
                Self::ChangePassword(UiChangePassword::new_page(page_unique_number).and_open_page())
            }
            UiPage::EguiSetting(_) => {
                Self::EguiSetting(UiEguiSettings::new_page(page_unique_number).and_open_page())
            }
            UiPage::Uac(_) => Self::Uac(UiUAC::new_page(page_unique_number).and_open_page()),
        }
    }

    fn type_to_instance<T: DisplayablePage<DataShared, Permission, private::Token>>()
    -> anyhow::Result<Self> {
        for page in Self::iter() {
            if page.title_base() == T::title_base() {
                return Ok(match page {
                    UiPage::Chat(_) => Self::Chat(UiChat::default()),
                    UiPage::ChangePassword(_) => Self::ChangePassword(UiChangePassword::default()),
                    UiPage::EguiSetting(_) => Self::EguiSetting(UiEguiSettings::default()),
                    UiPage::Uac(_) => Self::Uac(UiUAC::default()),
                });
            }
        }
        bail!(
            "invalid type passed. `UiPage` does not support: '{}'",
            T::title_base()
        )
    }

    fn ui_menu_page_btn<T: DisplayablePage<DataShared, Permission, private::Token>>(
        ui: &mut egui::Ui,
        data_shared: &DataShared,
        active_pages: &mut Vec<Self>,
    ) -> anyhow::Result<()>
    where
        DataShared: egui_pages::PermissionValidator<Permission>,
    {
        Self::type_to_instance::<T>()?.ui_menu_page_btn_inst(ui, data_shared, active_pages);
        Ok(())
    }

    fn page_permissions(&self) -> &[Permission] {
        do_on_ui_page!(self, page, { page.page_permissions_from_inst() })
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

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
