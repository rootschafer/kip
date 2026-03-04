//! Kip API - Public interface for both GUI and CLI
//!
//! This module provides the public API surface for Kip. Both the GUI (Dioxus)
//! and CLI (clap) consume this API — neither touches the database directly.

pub mod config;
pub mod intent;
pub mod location;
pub mod query;
pub mod review;
pub mod transfer;
pub mod types;

pub use types::*;
pub use intent::*;
pub use transfer::*;
pub use location::*;
pub use review::*;
pub use query::*;
pub use config::*;

// Re-export common types
pub use crate::db::DbHandle;
