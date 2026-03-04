//! Daemon - Background service for Kip
//!
//! This crate contains:
//! - SurrealDB connection and queries
//! - File watching
//! - Sync queue processing
//! - Graph state management
//!
//! Depends on `kip-core` for data models.

pub mod db;
pub mod engine;
pub mod graph_store;

pub use graph_store::{Graph, DragState};
pub use graph_store::{rid_string, load_graph_data, scan_directory, save_node_position, add_remote_machine};
pub use db::DbHandle;
