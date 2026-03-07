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

pub use graph_store::{
	add_remote_machine, load_graph_data, rid_string, save_node_position, scan_directory, DragState, Graph,
};
pub use db::DbHandle;
