#![expect(
    clippy::print_stdout,
    clippy::unwrap_used,
    clippy::missing_assert_message,
    reason = "fine for tests"
)]
mod branch;
mod change_password;
mod chat;
mod health_check;
mod host_branch;
mod login;
mod permissions;
mod roles;
mod users;
mod web_sockets;

mod helpers;
