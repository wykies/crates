//! Stores functionality that should be shared between different clients
//! NB: The assumption is made that the async runtime has already been started
//! before any functions from this library are called

#![warn(unused_crate_dependencies)]

#[cfg(test)] // Included to prevent unused crate warning
mod warning_suppress {
    use wasm_bindgen_test as _;
}

mod client;

pub use client::{
    websocket::{WakeFn, WebSocketConnection},
    Client, LoginOutcome, UiCallBack,
};

#[cfg(feature = "expose_test")]
pub use client::websocket::expose_test as ws_expose_test;
