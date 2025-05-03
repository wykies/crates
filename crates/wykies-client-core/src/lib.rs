//! Stores functionality that should be shared between different clients
//! NB: The assumption is made that the async runtime has already been started
//! before any functions from this library are called

#![warn(unused_crate_dependencies)]

#[cfg(test)] // Included to prevent unused crate warning
mod warning_suppress {
    use wasm_bindgen_test as _;
    use wykies_server_test_helper as _;
}

mod client;
mod error_helpers;

pub use client::{Client, LoginOutcome, DUMMY_ARGUMENT};
pub use error_helpers::ErrorStore;
