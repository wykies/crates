//! Code shared between the client and the server

#![warn(unused_crate_dependencies)]

pub mod branch;
pub mod const_config;
pub mod errors;
pub mod host_branch;
pub mod id;
mod macros;
pub mod random;
pub mod req_args;
pub mod token;
pub mod uac;

pub use random::{random_string, random_string_def_len};

#[cfg(not(target_arch = "wasm32"))]
pub mod telemetry;
