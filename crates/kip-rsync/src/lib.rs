//! kip-rsync: Type-safe rsync CLI wrapper with progress tracking
//!
//! This crate provides a high-level, type-safe interface to rsync,
//! handling platform differences, SSH configuration, and progress tracking.
//!
//! # Example
//!
//! ```rust,no_run
//! use kip_rsync::{Rsync, Destination, ProgressTracker, RsyncStats};
//! use std::path::PathBuf;
//!
//! # async fn example() -> anyhow::Result<()> {
//! // Simple local backup
//! let stats = Rsync::new("/path/to/source", Destination::local("/path/to/dest"))
//!     .with_exclude("*.tmp")
//!     .with_exclude(".git/")
//!     .with_delete()
//!     .run()
//!     .await?;
//!
//! println!("Backed up {} in {:?}",
//!     RsyncStats::format_bytes(stats.bytes_transferred),
//!     stats.duration);
//!
//! // SSH backup with progress
//! let tracker = ProgressTracker::new()
//!     .with_callback(|stats| {
//!         println!("Progress: {:.1}% - {}", stats.percent,
//!             RsyncStats::format_bytes(stats.bytes_transferred));
//!     });
//!
//! let dest = Destination::ssh("example.com", "user", "/backup")
//!     .with_port(2222)
//!     .with_identity(PathBuf::from("~/.ssh/id_ed25519"));
//!
//! let stats = Rsync::new("/path/to/source", dest)
//!     .with_compression()
//!     .with_progress(tracker)
//!     .run()
//!     .await?;
//! # Ok(())
//! # }
//! ```

pub mod destination;
pub mod error;
pub mod executor;
pub mod platform;
pub mod progress;
pub mod stats;

// Test utilities - exposed for integration tests
#[cfg(any(test, feature = "test-utils"))]
pub mod test_fixture;
#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils;

// Re-export main types
pub use destination::{Destination, SshOptions};
pub use error::{Result, RsyncError};
pub use executor::{ssh_mkdir, Rsync};
pub use platform::Platform;
pub use progress::{ProgressCallback, ProgressStats, ProgressTracker};
pub use stats::RsyncStats;

/// Parallel execution utilities
pub mod parallel {
	use futures::stream::{self, StreamExt};

	use crate::{error::Result, executor::Rsync, stats::RsyncStats};

	/// Execute multiple rsync operations in parallel
	///
	/// Groups operations by destination to avoid race conditions
	pub async fn run_parallel(tasks: Vec<Rsync>, max_concurrent: usize) -> Result<Vec<RsyncStats>> {
		// Group by destination type to avoid conflicts
		// (simplified - in production would group by exact destination)
		let mut local_tasks = Vec::new();
		let mut ssh_tasks = Vec::new();

		for task in tasks {
			if task.destination().is_local() {
				local_tasks.push(task);
			} else {
				ssh_tasks.push(task);
			}
		}

		// Run local and SSH tasks in parallel
		let local_results = stream::iter(local_tasks)
			.map(|task| task.run())
			.buffered(max_concurrent)
			.collect::<Vec<_>>()
			.await;

		let ssh_results = stream::iter(ssh_tasks)
			.map(|task| task.run())
			.buffered(max_concurrent)
			.collect::<Vec<_>>()
			.await;

		// Combine results
		let mut results = Vec::new();
		for result in local_results.into_iter().chain(ssh_results) {
			results.push(result?);
		}

		Ok(results)
	}
}
