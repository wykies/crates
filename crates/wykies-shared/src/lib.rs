//! Code shared between the client and the server

#![warn(unused_crate_dependencies)]

#[cfg(not(target_arch = "wasm32"))]
pub mod telemetry;
