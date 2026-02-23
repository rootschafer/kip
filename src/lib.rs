//! Kip - File transfer orchestrator

#[cfg(feature = "desktop")]
pub mod api;
#[cfg(feature = "desktop")]
pub mod db;
#[cfg(feature = "desktop")]
pub mod engine;
#[cfg(feature = "desktop")]
pub mod devices;
#[cfg(feature = "desktop")]
pub mod models;
#[cfg(feature = "desktop")]
pub mod util;

#[cfg(feature = "desktop")]
pub mod ui;

#[cfg(feature = "desktop")]
pub mod app;

// Re-export API for easy access
#[cfg(feature = "desktop")]
pub use api::*;
