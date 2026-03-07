//! Core - Pure business logic for Kip
//!
//! This crate contains:
//! - Data models (GraphNode, GraphEdge, FileRecord, etc.)
//! - File type detection
//! - Path utilities
//! - Hash calculation
//!
//! No dependencies on SurrealDB, Dioxus, or CLI.

pub mod graph_types;
pub mod models;
pub mod util;

pub use graph_types::*;
