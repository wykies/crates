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
    Client, LoginOutcome, UiCallBack, DUMMY_ARGUMENT,
};

#[cfg(feature = "expose_internal")]
pub use client::websocket::expose_internal as ws_expose_internal;
