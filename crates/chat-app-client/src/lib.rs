#![warn(unused_crate_dependencies)]

#[cfg(target_arch = "wasm32")]
mod suppress_wasm_warnings {
    use getrandom as _; // Needed because we need to enable a feature on this crate

    // Only used in binary and triggers unused warning
    use wasm_bindgen_futures as _;
    use web_sys as _;
}

mod app;
pub mod background_worker;
pub mod cli;
mod lockout;
mod pages;
mod shortcuts;
pub mod tracing;
mod ui_helpers;

pub use app::{ChatApp, DataShared};
pub use pages::DisplayablePage;
pub use pages::UiPage;

/// Function is here to ensure lib also uses the log create to prevent the warning that it is not used
#[cfg(target_arch = "wasm32")]
pub fn wasm_log_level() -> log::LevelFilter {
    log::LevelFilter::Debug
}
