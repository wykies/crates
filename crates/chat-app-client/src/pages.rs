use crate::DataShared;

pub mod change_password;
pub mod chat;
pub mod egui_settings;
pub mod login;
mod macros;
pub mod uac;

mod private {
    /// Used to make some trait methods private
    pub struct Token;
}

use change_password::UiChangePassword;
use chat::UiChat;
use egui_settings::UiEguiSettings;
pub use login::UiLogin;
use strum::{EnumIter, IntoEnumIterator};
use tracing::{error, info};
use uac::UiUAC;
use wykies_shared::uac::{Permission, Permissions};

#[derive(Debug, serde::Serialize, serde::Deserialize, EnumIter)]
pub enum UiPage {
    ChangePassword(UiChangePassword),
    Chat(UiChat),
    EguiSetting(UiEguiSettings),
    UAC(UiUAC),
}

/// Trait for types that can be treated as pages to display
///
/// It uses Default and serde Traits as super traits to ensure all these types
/// implement these traits
pub trait DisplayablePage: Default + serde::Serialize + serde::de::DeserializeOwned {
    /// Reset the state of the screen
    fn reset_to_default(&mut self, _: private::Token);

    /// Displays the page
    fn show(&mut self, ui: &mut eframe::egui::Ui, data_shared: &mut DataShared);

    /// Base of the page's title (numbers get appended to duplicates)
    ///
    /// ASSUMPTION: THIS IS UNIQUE PER TYPE
    fn title_base() -> &'static str;

    /// Convenance function for working with instances inside of the enum
    fn title_base_from_instance(&self) -> &'static str {
        Self::title_base()
    }

    /// Page number to make title unique
    ///
    /// Assumed that the caller will ensure this number is unique across pages
    /// with the same base title
    fn page_unique_number(&self) -> usize;

    /// Creates a page with the unique number passed
    fn new_page(page_unique_number: usize) -> Self;

    /// Pages display title (includes page number if not first)
    fn title(&self) -> String {
        if self.page_unique_number() == 0 {
            Self::title_base().to_string()
        } else {
            format!("{} ({})", Self::title_base(), self.page_unique_number())
        }
    }

    fn is_page_open(&self) -> bool;

    fn open_page(&mut self) {
        info!("Open Page {}", self.title());
        self.internal_do_open_page(private::Token {});
    }

    fn close_page(&mut self) {
        info!("Close Page {}", self.title());
        self.internal_do_close_page(private::Token {});
    }

    fn internal_do_open_page(&mut self, _: private::Token);

    /// This usually clears any state loaded from the database
    fn internal_do_close_page(&mut self, _: private::Token);

    /// Convenance method for chaining
    #[must_use]
    fn and_open_page(mut self) -> Self {
        self.open_page();
        self
    }

    /// Provides an opportunity for the page to change settings on the window
    /// before display
    fn adjust_window_settings<'open>(&self, window: egui::Window<'open>) -> egui::Window<'open> {
        // Provide identity default impl
        window
    }

    /// Provides the permissions required for a page
    fn page_permissions() -> &'static [Permission];

    fn has_permissions(user_permissions: &Permissions) -> bool {
        user_permissions.includes(Self::page_permissions())
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
    pub fn new_page_with_unique_number<T: DisplayablePage>(page_unique_number: usize) -> UiPage {
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
        let msg = format!("execution should never get here. All pages should be able to be found but {:?} not found", T::title_base());
        error!("{msg}");
        unreachable!("{msg}");
    }

    pub fn display_page(&mut self, ctx: &egui::Context, data_shared: &mut DataShared) {
        do_on_ui_page!(self, page, { show_page(page, ctx, data_shared) })
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

fn show_page<P: DisplayablePage>(page: &mut P, ctx: &egui::Context, data_shared: &mut DataShared) {
    let mut is_open = page.is_page_open();
    if !is_open {
        return;
    }
    let mut window = egui::Window::new(page.title()).vscroll(true).hscroll(true);
    window = page.adjust_window_settings(window);
    window
        .open(&mut is_open)
        .show(ctx, |ui| page.show(ui, data_shared));
    if !is_open {
        page.close_page();
    }
}
