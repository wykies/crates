mod macros;
mod traits;

pub use traits::{DisplayablePage, PageContainer, PermissionValidator};

pub fn show_page<Permission, DataShared, Page, PrivateToken: Default>(
    page: &mut Page,
    ui: &mut egui::Ui,
    data_shared: &mut DataShared,
) where
    Permission: 'static,
    Page: DisplayablePage<DataShared, Permission, PrivateToken>,
{
    let mut is_open = page.is_page_open();
    if !is_open {
        return;
    }
    let mut window = egui::Window::new(page.title()).vscroll(true).hscroll(true);
    window = page.adjust_window_settings(window);
    window
        .open(&mut is_open)
        .show(ui, |ui| page.show(ui, data_shared));
    if !is_open {
        page.close_page();
    }
}
