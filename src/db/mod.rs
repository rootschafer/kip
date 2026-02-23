//! Database module - Unified database access for Kip

mod handle;
mod init;
mod schema;

pub use handle::DbHandle;
pub use schema::SCHEMA_V1;
pub use init::{init, init_memory, init_with_path};
