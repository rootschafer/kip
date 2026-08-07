//! kip-rclone: Cloud storage backup support via rclone
//!
//! This crate provides a high-level interface to rclone for backing up
//! to cloud storage providers like Google Drive, Nextcloud, S3, etc.
//!
//! # Example
//!
//! ```rust,no_run
//! use kip_rclone::{CloudDestination, Rclone};
//!
//! # async fn example() -> anyhow::Result<()> {
//! // Configure Google Drive destination
//! let gdrive = CloudDestination::google_drive("my_gdrive", "backups/kip");
//!
//! // Create rclone instance
//! let rclone = Rclone::new();
//!
//! // Copy files to cloud (like cp, doesn't delete)
//! let stats = rclone.copy("/local/path", &gdrive, "").await?;
//! println!("Copied {} bytes", stats.bytes_transferred);
//!
//! // Sync directory (like rsync --delete)
//! let stats = rclone.sync("/local/path", &gdrive, "").await?;
//! println!("Synced {} files", stats.files_transferred);
//! # Ok(())
//! # }
//! ```

pub mod destination;
pub mod error;
pub mod rclone;
pub mod stats;

// Re-export main types
pub use destination::{
    CloudDestination, CloudProvider, GoogleDriveConfig, NextcloudConfig, S3Config, WebDAVConfig,
};
pub use error::{CloudError, Result};
pub use rclone::Rclone;
pub use stats::CloudStats;
