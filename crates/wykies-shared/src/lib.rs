//! Code shared between the client and the server

#![warn(unused_crate_dependencies)]

#[cfg(not(target_arch = "wasm32"))]
use tokio_tungstenite as _; // Needed for our CA to work for WSS

pub mod branch;
pub mod const_config;

#[cfg(feature = "server_only")]
mod error_wrappers;
pub mod errors;
pub mod host_branch;
mod macros_debugging;
mod macros_enums;
mod macros_wrappers;
pub mod random;
pub mod req_args;
pub mod token;
pub mod uac;
pub mod websockets;

#[cfg(feature = "server_only")]
pub use db_types;

pub use macros_wrappers::AlwaysCase;

pub use random::{random_string, random_string_def_len};

#[cfg(not(target_arch = "wasm32"))]
pub mod telemetry;

#[cfg(feature = "server_only")]
pub use error_wrappers::{e400, e500};
