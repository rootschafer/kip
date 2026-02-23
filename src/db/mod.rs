//! Database module - Unified database access for Kip

mod handle;
mod schema;
mod init;

pub use handle::DbHandle;
pub use schema::SCHEMA_V1;
pub use init::{init, init_with_path, init_memory};
