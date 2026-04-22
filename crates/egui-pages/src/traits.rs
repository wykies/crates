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
