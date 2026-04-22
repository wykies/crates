use crate::do_organize_pages;
use egui_helpers::{RemovableItem, UiHelpers as _};
use tracing::info;

/// Trait for types that can be treated as pages to display
///
/// It uses Default and serde Traits as super traits to ensure all these types
/// implement these traits.
///
/// The PrivateToken generic should be set to any type that only the pages have
/// access to so that the methods it protects cannot be called from outside of
/// the page itself. It needs to implement Default so that there is a
/// predetermined way to create an instance even if it carries no data
pub trait DisplayablePage<DataShared, Permission: 'static, PrivateToken: Default>:
    Default + serde::Serialize + serde::de::DeserializeOwned
{
    /// Reset the state of the screen
    ///
    /// # Warning
    ///
    /// This by it's very nature can cause loss of data. It only persists the
    /// data that is set to be serialized
    fn reset_to_default(&mut self) {
        let data = ron::to_string(self).expect("failed serialize to ron for reset");
        *self = ron::from_str(&data).expect("failed deserialize ron during reset");
    }

    /// Displays the page
    fn show(&mut self, ui: &mut egui::Ui, data_shared: &mut DataShared);

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

    /// Provides a consistent way to generate IDs that are unique throughout the
    /// application
    ///
    /// Needed to prevent duplicate ID if multiple of the same window are used
    /// and not need to be aware of the global namespace for panels or other
    /// controls that can have conflict. Provides a prefix as it my be used by
    /// called functions and not have direct access to this method.
    ///
    /// # Precondition
    ///
    /// `id_name` is unique for the page on which it is provided or will be
    /// joined with something that is unique on a subpage
    ///
    /// # Assumptions
    ///
    /// - `Self::title` is unique throughout the application
    fn unique_prefix_for_id(&self, id_name: &str) -> String {
        format!("{}{id_name}", self.title())
    }

    fn is_page_open(&self) -> bool;

    fn open_page(&mut self) {
        info!("Open Page {}", self.title());
        self.internal_do_open_page(PrivateToken::default());
    }

    fn close_page(&mut self) {
        info!("Close Page {}", self.title());
        self.internal_do_close_page(PrivateToken::default());
    }

    fn internal_do_open_page(&mut self, _: PrivateToken);

    /// This usually clears any state loaded from the database
    fn internal_do_close_page(&mut self, _: PrivateToken);

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
}

/// Provides a way to validate some set of permissions is met
pub trait PermissionValidator<Permission: 'static> {
    fn has_permissions(&self, required_permissions: &[Permission]) -> bool;
}

pub trait PageContainer<DataShared, Permission: 'static, PrivateToken: Default>:
    RemovableItem
where
    Self: Sized,
{
    fn new_page_with_unique_number<T: DisplayablePage<DataShared, Permission, PrivateToken>>(
        page_unique_number: usize,
    ) -> Self;

    fn display_page(&mut self, ui: &mut egui::Ui, data_shared: &mut DataShared);

    fn title_base(&self) -> &'static str;

    fn page_unique_number(&self) -> usize;

    fn is_page_open(&self) -> bool;

    fn title(&self) -> String;

    fn open_page(&mut self);

    fn close_page(&mut self);

    /// This function is intended to allow the implementer to specify the
    /// relevant types and then call the "internal_do_" version
    fn ui_menu_page_btn<T: DisplayablePage<DataShared, Permission, PrivateToken>>(
        ui: &mut egui::Ui,
        data_shared: &DataShared,
        active_pages: &mut Vec<Self>,
    ) where
        DataShared: PermissionValidator<Permission>;

    fn internal_do_ui_menu_page_btn<T: DisplayablePage<DataShared, Permission, PrivateToken>>(
        ui: &mut egui::Ui,
        data_shared: &DataShared,
        active_pages: &mut Vec<Self>,
    ) where
        DataShared: PermissionValidator<Permission>,
    {
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

    fn ui_pages_management_controls(
        ui: &mut egui::Ui,
        pages: &mut Vec<Self>,
        organize_shortcut: &egui::KeyboardShortcut,
    ) {
        if ui.button("Open All Pages").clicked() {
            Self::open_all_pages(pages);
            ui.close();
        }
        if ui.button("Close All Pages").clicked() {
            Self::close_all_pages(pages);
            ui.close();
        }
        if ui.button("Deactivate All Pages").clicked() {
            Self::deactivate_all_pages(pages);
            ui.close();
        }
        if ui.button("Sort Pages By Name").clicked() {
            Self::sort_pages_by_name(pages);
            ui.close();
        }
        if ui
            .add(
                egui::Button::new("Organize Pages")
                    .shortcut_text(ui.format_shortcut(organize_shortcut)),
            )
            .clicked()
        {
            do_organize_pages(ui);
            ui.close();
        }
    }

    fn open_all_pages(active_pages: &mut Vec<Self>) {
        active_pages.iter_mut().for_each(|page| page.open_page());
    }

    fn close_all_pages(active_pages: &mut Vec<Self>) {
        active_pages.iter_mut().for_each(|page| page.close_page());
    }

    fn deactivate_all_pages(active_pages: &mut Vec<Self>) {
        active_pages.clear();
    }

    fn sort_pages_by_name(active_pages: &mut Vec<Self>) {
        active_pages.sort_by_key(|x| x.title());
    }

    fn ui_active_pages_panel(
        ui: &mut egui::Ui,
        active_pages: &mut Vec<Self>,
        organize_shortcut: &egui::KeyboardShortcut,
    ) {
        egui::Panel::right("right_side_panel")
            .resizable(false)
            .default_size(200.0)
            .show_inside(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("Active Pages");
                });

                ui.separator();

                Self::ui_pages_list(ui, active_pages, organize_shortcut);
            });
    }

    fn ui_display_pages(
        ui: &mut egui::Ui,
        active_pages: &mut Vec<Self>,
        data_shared: &mut DataShared,
    ) {
        for page in active_pages.iter_mut() {
            page.display_page(ui, data_shared);
        }
    }

    fn ui_pages_list(
        ui: &mut egui::Ui,
        active_pages: &mut Vec<Self>,
        organize_shortcut: &egui::KeyboardShortcut,
    ) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                ui.removable_items_list(
                    Some(active_pages),
                    "NO PAGES ARE ACTIVE.\nUse top menu to activate a page",
                );

                ui.separator();

                Self::ui_pages_management_controls(ui, active_pages, organize_shortcut);
            });
        });
    }
}
