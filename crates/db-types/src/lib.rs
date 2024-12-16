// #[cfg_attr(all(), path = "db_types_mysql.rs")]
// pub mod db_types;

#[cfg(not(feature = "mysql"))]
compile_error!("No Database Type selected. Please use create features to select a DB to use");

#[cfg_attr(all(feature = "mysql"), path = "db_types_mysql.rs")]
mod types;

pub use types::*;
