//! Statistics for completed rsync operations

use std::time::Duration;

/// Statistics from a completed rsync operation
#[derive(Debug, Clone, Default)]
pub struct RsyncStats {
	/// Total bytes transferred
	pub bytes_transferred: u64,
	/// Total files transferred
	pub files_transferred: u64,
	/// Files that were updated (changed)
	pub files_updated: u64,
	/// Files that were deleted
	pub files_deleted: u64,
	/// Transfer duration
	pub duration: Option<Duration>,
	/// Average transfer speed in bytes per second
	pub avg_speed_bps: u64,
	/// Whether the operation was a dry-run
	pub dry_run: bool,
}

impl RsyncStats {
	pub fn new() -> Self {
		Self::default()
	}

	/// Merge stats from another operation (for parallel operations)
	pub fn merge(&mut self, other: &RsyncStats) {
		self.bytes_transferred += other.bytes_transferred;
		self.files_transferred += other.files_transferred;
		self.files_updated += other.files_updated;
		self.files_deleted += other.files_deleted;

		// For duration and speed, we'd need more sophisticated merging
		// For now, just take the max
		if other.duration > self.duration {
			self.duration = other.duration;
		}
		if other.avg_speed_bps > self.avg_speed_bps {
			self.avg_speed_bps = other.avg_speed_bps;
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

		if let Some(duration) = self.duration {
			let secs = duration.as_secs();
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

	/// Format bytes in human-readable form
	pub fn format_bytes(bytes: u64) -> String {
		const KB: u64 = 1024;
		const MB: u64 = KB * 1024;
		const GB: u64 = MB * 1024;

		if bytes >= GB {
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
}

/// Parse rsync --stats output
///
/// Example rsync stats output (macOS/BSD rsync):
/// ```text
/// Number of files: 42
/// Number of files transferred: 9
/// Total file size: 52836488 B
/// Total transferred file size: 52836435 B
/// sent 2496 bytes  received 272 bytes  542745 bytes/sec
/// total size is 52836488  speedup is 19088.32
/// ```
///
/// Example rsync stats output (GNU rsync):
/// ```text
/// Number of files: 100 (reg: 80, dir: 20)
/// Number of regular files transferred: 80
/// Total file size: 12345678 bytes
/// Total transferred file size: 12345678 bytes
/// ```
pub fn parse_stats_output(output: &str) -> RsyncStats {
	let mut stats = RsyncStats::new();

	for line in output.lines() {
		let line = line.trim();

		// Parse "Number of files transferred: 9" (macOS) or "Number of regular files transferred: 80" (GNU)
		if line.starts_with("Number of files transferred:") || line.starts_with("Number of regular files transferred:")
		{
			if let Some(num) = parse_count(line) {
				stats.files_transferred = num;
			}
		}

		// Parse "Total file size: 52836488 B" or "Total file size: 12345678 bytes"
		if line.starts_with("Total file size:") {
			if let Some(num) = parse_count(line) {
				stats.bytes_transferred = num;
			}
		}

		// Parse "Total transferred file size: 52836435 B" (GNU rsync)
		if line.starts_with("Total transferred file size:") {
			if let Some(num) = parse_count(line) {
				stats.bytes_transferred = num;
			}
		}

		// Parse "Number of deleted files: 5"
		if line.starts_with("Number of deleted files:") {
			if let Some(num) = parse_count(line) {
				stats.files_deleted = num;
			}
		}

		// Parse "Number of files: 100 (reg: 80, dir: 20)"
		if line.starts_with("Number of files:") && !line.contains("transferred") {
			// This is total file count, not transferred
		}
	}

	stats
}

/// Pull the number out of an rsync `--stats` line of the form `Label: 1,234 bytes`.
///
/// GNU rsync groups digits with a thousands separator (`10,485,760`) while the
/// rsync shipped with macOS does not. Parsing the raw token with `parse::<u64>`
/// therefore silently yields 0 on Linux, which is how this went unnoticed —
/// every byte count came back zero there. Strip the separators before parsing.
/// The separator is locale-dependent, so drop every non-digit rather than
/// special-casing the comma; these lines never carry a fractional part.
fn parse_count(line: &str) -> Option<u64> {
	let token = line.split(':').nth(1)?.trim().split_whitespace().next()?;

	let digits: String = token.chars().filter(char::is_ascii_digit).collect();
	if digits.is_empty() {
		return None;
	}
	digits.parse::<u64>().ok()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_format_bytes() {
		assert_eq!(RsyncStats::format_bytes(0), "0 B");
		assert_eq!(RsyncStats::format_bytes(1024), "1.00 KB");
		assert_eq!(RsyncStats::format_bytes(1024 * 1024), "1.00 MB");
	}

	#[test]
	fn test_stats_merge() {
		let mut stats1 = RsyncStats {
			bytes_transferred: 1000,
			files_transferred: 10,
			..Default::default()
		};

		let stats2 = RsyncStats {
			bytes_transferred: 500,
			files_transferred: 5,
			..Default::default()
		};

		stats1.merge(&stats2);
		assert_eq!(stats1.bytes_transferred, 1500);
		assert_eq!(stats1.files_transferred, 15);
	}

	#[test]
	fn test_parse_stats() {
		let output = r#"
Number of files: 150 (reg: 123, dir: 27)
Number of regular files transferred: 123
Total file size: 12345678 bytes
Total transferred file size: 12345678 bytes
Number of deleted files: 5
"#;
		let stats = parse_stats_output(output);
		assert_eq!(stats.files_transferred, 123);
		assert_eq!(stats.bytes_transferred, 12345678);
		assert_eq!(stats.files_deleted, 5);
	}

	/// GNU rsync (Linux) groups digits; the macOS build does not. Parsing the
	/// grouped form used to fail and silently report zero bytes transferred.
	#[test]
	fn test_parse_stats_with_thousands_separators() {
		let output = r#"
Number of files: 2 (reg: 1, dir: 1)
Number of created files: 1 (reg: 1)
Number of deleted files: 1,024
Number of regular files transferred: 1,234
Total file size: 10,485,760 bytes
Total transferred file size: 10,485,760 bytes
"#;
		let stats = parse_stats_output(output);
		assert_eq!(stats.files_transferred, 1234);
		assert_eq!(stats.bytes_transferred, 10_485_760);
		assert_eq!(stats.files_deleted, 1024);
	}

	#[test]
	fn test_parse_stats_ignores_unparseable_values() {
		let stats = parse_stats_output("Total file size: unknown bytes\n");
		assert_eq!(stats.bytes_transferred, 0);
	}
}
