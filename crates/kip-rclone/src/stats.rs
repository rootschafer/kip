//! Statistics for cloud operations

use serde::{Deserialize, Serialize};

/// Statistics from a cloud operation
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CloudStats {
	/// Total bytes transferred
	pub bytes_transferred: u64,
	/// Total files transferred
	pub files_transferred: u64,
	/// Files that were updated (changed)
	pub files_updated: u64,
	/// Files that were deleted (sync only)
	pub files_deleted: u64,
	/// Transfer duration in seconds
	pub duration_secs: f64,
	/// Average transfer speed in bytes per second
	pub avg_speed_bps: u64,
	/// Number of errors encountered
	pub errors: u64,
	/// Whether this was a dry-run
	pub dry_run: bool,
}

impl CloudStats {
	pub fn new() -> Self {
		Self::default()
	}

	/// Format bytes in human-readable form
	pub fn format_bytes(bytes: u64) -> String {
		const KB: u64 = 1024;
		const MB: u64 = KB * 1024;
		const GB: u64 = MB * 1024;
		const TB: u64 = GB * 1024;

		if bytes >= TB {
			format!("{:.2} TB", bytes as f64 / TB as f64)
		} else if bytes >= GB {
			format!("{:.2} GB", bytes as f64 / GB as f64)
		} else if bytes >= MB {
			format!("{:.2} MB", bytes as f64 / MB as f64)
		} else if bytes >= KB {
			format!("{:.2} KB", bytes as f64 / KB as f64)
		} else {
			format!("{} B", bytes)
		}
	}

	/// Format speed in human-readable form
	pub fn format_speed(bps: u64) -> String {
		const KB: u64 = 1024;
		const MB: u64 = KB * 1024;

		if bps >= MB {
			format!("{:.2} MB/s", bps as f64 / MB as f64)
		} else if bps >= KB {
			format!("{:.2} KB/s", bps as f64 / KB as f64)
		} else {
			format!("{} B/s", bps)
		}
	}

	/// Format stats for display
	pub fn format(&self) -> String {
		let mut parts = vec![];

		if self.bytes_transferred > 0 {
			parts.push(format!("{} transferred", Self::format_bytes(self.bytes_transferred)));
		}

		if self.files_transferred > 0 {
			parts.push(format!("{} files", self.files_transferred));
		}

		if self.files_updated > 0 {
			parts.push(format!("{} updated", self.files_updated));
		}

		if self.files_deleted > 0 {
			parts.push(format!("{} deleted", self.files_deleted));
		}

		if self.duration_secs > 0.0 {
			let secs = self.duration_secs as u64;
			if secs >= 60 {
				parts.push(format!("in {}m {}s", secs / 60, secs % 60));
			} else {
				parts.push(format!("in {}s", secs));
			}
		}

		if self.avg_speed_bps > 0 {
			parts.push(format!("at {}", Self::format_speed(self.avg_speed_bps)));
		}

		parts.join(", ")
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_format_bytes() {
		assert_eq!(CloudStats::format_bytes(0), "0 B");
		assert_eq!(CloudStats::format_bytes(1024), "1.00 KB");
		assert_eq!(CloudStats::format_bytes(1024 * 1024), "1.00 MB");
		assert_eq!(CloudStats::format_bytes(1024 * 1024 * 1024), "1.00 GB");
	}

	#[test]
	fn test_format_speed() {
		assert_eq!(CloudStats::format_speed(1024), "1.00 KB/s");
		assert_eq!(CloudStats::format_speed(1024 * 1024), "1.00 MB/s");
	}

	#[test]
	fn test_stats_format() {
		let stats = CloudStats {
			bytes_transferred: 1024 * 1024 * 100,
			files_transferred: 50,
			files_updated: 10,
			files_deleted: 5,
			duration_secs: 125.5,
			avg_speed_bps: 1024 * 1024,
			errors: 0,
			dry_run: false,
		};

		let formatted = stats.format();
		assert!(formatted.contains("100.00 MB"));
		assert!(formatted.contains("50 files"));
		assert!(formatted.contains("2m 5s"));
	}
}
