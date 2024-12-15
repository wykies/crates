//! # Notable design decisions (notes)
//! Using this area to document important design decisions made for ease of
//! reference and discoverability
//! - For variable length fields in the database we restrict in the code to
//!   number of bytes not characters because our common case will be to use
//!   ascii anyway and checking number of bytes is cheaper and will work all
//!   version of MySQL
//! - Permissions restrictions on endpoints see
//!   [`wykies_shared::uac::get_required_permissions`]
//! - Force password change is not enforced by the server yet
//! - Chained calls to endpoints is not preferred but may be suitable if the
//!   chaining doesn't happen in the common case like in login and having to set
//!   the branch. When this happens please ensure a constant in the same place
//!   with the permissions is used so these cases can be tracked.
//! - WebSocket connections are not able to be authenticated using cookies so we
//!   are using a two step process. A request is sent via an authenticated
//!   connection for a token. The token and the requesting IP are saved then a
//!   request to connect to the websocket is sent and the IP is validated and
//!   the first message required to be sent by the client is the token that was
//!   sent to them before. To simplify using this the path for the token and the
//!   connection must have the same suffix so that one method in the client can
//!   do both calls.
//! - Suggested sequence of steps to create an endpoint:
//!     - Go to `server/src/routes.rs` and decide where it belongs, create a
//!       stub in the appropriate module and add the use statement
//!     - Go to `server/src/startup.rs`
//!         - Add the use statement at the top
//!         - Add the route in the server configuration
//!     - Go to `shared/src/uac/permissions.rs` and add the permissions entry
//!       (requires also creating the constant for the path)
//!     - Add tests (Should fail as it's not implemented)
//!     - Implement endpoint (Test should pass now)
//!     - Add client interface

#![warn(unused_crate_dependencies)]

mod warning_suppress {
    use sqlx as _; // Needed to enable TLS
}

#[cfg(test)] // Included to prevent unused crate warning
mod warning_suppress_test {
    use chrono as _;
    use ewebsock as _;
    use insta as _;
    use secrecy as _;
    use serde_json as _;
    use sqlx as _;
    use uuid as _;
    use wykies_client_core as _;
    use wykies_server_test_helper as _;
    use wykies_time as _;
}

pub mod startup;
mod websocket;

// TODO 3: Ensure we have a way to access the logs... Maybe we have to switch back to stdout but see what options the hosting provider supports
// TODO 3: Enable HTTPS https://actix.rs/docs/server/#tls--https https://github.com/actix/examples/tree/master/https-tls/rustls
// TODO 4: Some performance was left on the table by using `text` for the
//         websockets instead of `binary`
// TODO 4: Purge history
