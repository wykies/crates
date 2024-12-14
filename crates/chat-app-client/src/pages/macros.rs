#[macro_export]
macro_rules! displayable_page_common {
    ($page_name: expr, $permissions: expr) => {
        fn title_base() -> &'static str {
            $page_name
        }

        fn page_unique_number(&self) -> usize {
            self.page_unique_number
        }

        fn is_page_open(&self) -> bool {
            self.is_open
        }

        fn new_page(page_unique_number: usize) -> Self {
            Self {
                page_unique_number,
                ..Default::default()
            }
        }
        fn internal_do_open_page(&mut self, _: super::private::Token) {
            self.is_open = true;
        }

        fn internal_do_close_page(&mut self, _: super::private::Token) {
            self.is_open = false;
            self.reset_to_default(super::private::Token {});
        }

        fn page_permissions() -> &'static [wykies_shared::uac::Permission] {
            $permissions
        }
    };
}
