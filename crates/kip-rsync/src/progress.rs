//! Progress tracking for rsync operations

use std::sync::{
	atomic::{AtomicU64, AtomicBool, Ordering},
	Arc,
};

/// Statistics about an ongoing rsync operation
#[derive(Debug, Clone, Default)]
pub struct ProgressStats {
	/// Bytes transferred so far
	pub bytes_transferred: u64,
	/// Total bytes to transfer (if known)
	pub total_bytes: Option<u64>,
	/// Files transferred so far
	pub files_transferred: u64,
	/// Current file being transferred
	pub current_file: Option<String>,
	/// Percent complete (0.0 - 100.0)
	pub percent: f64,
	/// Transfer speed in bytes per second
	pub speed_bps: u64,
}

impl ProgressStats {
	pub fn new() -> Self {
		Self::default()
	}

	/// Calculate percentage
	pub fn calculate_percent(&mut self) {
		if let Some(total) = self.total_bytes {
			if total > 0 {
				self.percent = (self.bytes_transferred as f64 / total as f64) * 100.0;
			}
		}
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

	/// Format transfer speed
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

/// Callback type for progress updates
pub type ProgressCallback = Arc<dyn Fn(ProgressStats) + Send + Sync>;

/// Shared state for tracking progress across async operations
#[derive(Clone, Default)]
pub struct ProgressTracker {
	stats: Arc<ProgressStatsInner>,
	callback: Option<ProgressCallback>,
}

#[derive(Default)]
struct ProgressStatsInner {
	bytes_transferred: AtomicU64,
	total_bytes: AtomicU64,
	files_transferred: AtomicU64,
	current_file: parking_lot::Mutex<Option<String>>,
	cancelled: AtomicBool,
}

impl ProgressTracker {
	pub fn new() -> Self {
		Self::default()
	}

	/// Set a progress callback
	pub fn with_callback(mut self, callback: impl Fn(ProgressStats) + Send + Sync + 'static) -> Self {
		self.callback = Some(Arc::new(callback));
		self
	}

	/// Set total bytes (if known)
	pub fn set_total_bytes(&self, total: u64) {
		self.stats.total_bytes.store(total, Ordering::Relaxed);
	}

	/// Update bytes transferred
	pub fn update_bytes(&self, bytes: u64) {
		self.stats.bytes_transferred.store(bytes, Ordering::Relaxed);
		self.notify();
	}

	/// Increment bytes transferred
	pub fn add_bytes(&self, bytes: u64) {
		self.stats.bytes_transferred.fetch_add(bytes, Ordering::Relaxed);
		self.notify();
	}

	/// Set current file being transferred
	pub fn set_current_file(&self, file: impl Into<String>) {
		*self.stats.current_file.lock() = Some(file.into());
		self.notify();
	}

	/// Increment files transferred
	pub fn add_file(&self) {
		self.stats.files_transferred.fetch_add(1, Ordering::Relaxed);
		self.notify();
	}

	/// Request cancellation
	pub fn cancel(&self) {
		self.stats.cancelled.store(true, Ordering::Relaxed);
	}

	/// Check if cancelled
	pub fn is_cancelled(&self) -> bool {
		self.stats.cancelled.load(Ordering::Relaxed)
	}

	/// Get current stats
	pub fn get_stats(&self) -> ProgressStats {
		ProgressStats {
			bytes_transferred: self.stats.bytes_transferred.load(Ordering::Relaxed),
			total_bytes: {
				let total = self.stats.total_bytes.load(Ordering::Relaxed);
				if total > 0 { Some(total) } else { None }
			},
			files_transferred: self.stats.files_transferred.load(Ordering::Relaxed),
			current_file: self.stats.current_file.lock().clone(),
			percent: 0.0, // Will be calculated
			speed_bps: 0, // Would need timing info
		}
	}

	/// Notify callback of progress update
	fn notify(&self) {
		if let Some(ref callback) = self.callback {
			let mut stats = self.get_stats();
			stats.calculate_percent();
			callback(stats);
		}
	}
}

/// Parse rsync --progress output line
/// Format: "filename\n\t1234567 100%  123.45kB/s    0:00:10"
pub fn parse_progress_line(line: &str) -> Option<ProgressStats> {
	let line = line.trim();
	
	// Skip empty lines
	if line.is_empty() {
		return None;
	}

	// Try to parse byte count from start of line
	let parts: Vec<&str> = line.split_whitespace().collect();
	if parts.is_empty() {
		return None;
	}

	// First part might be filename or byte count
	let bytes = parts[0].replace(',', "").parse::<u64>().ok()?;
	
	// Look for percentage
	let percent = parts.iter()
		.find(|s| s.contains('%'))
		.and_then(|s| s.trim_end_matches('%').parse::<f64>().ok())
		.unwrap_or(0.0);

	Some(ProgressStats {
		bytes_transferred: bytes,
		percent,
		..Default::default()
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_format_bytes() {
		assert_eq!(ProgressStats::format_bytes(0), "0 B");
		assert_eq!(ProgressStats::format_bytes(1024), "1.00 KB");
		assert_eq!(ProgressStats::format_bytes(1024 * 1024), "1.00 MB");
		assert_eq!(ProgressStats::format_bytes(1024 * 1024 * 1024), "1.00 GB");
	}

	#[test]
	fn test_progress_parsing() {
		let line = "1234567 100%  123.45kB/s    0:00:10";
		let stats = parse_progress_line(line).unwrap();
		assert_eq!(stats.bytes_transferred, 1234567);
		assert!((stats.percent - 100.0).abs() < 0.01);
	}

	#[test]
	fn test_progress_tracker() {
		let tracker = ProgressTracker::new();
		tracker.set_total_bytes(1000);
		tracker.update_bytes(500);

		let mut stats = tracker.get_stats();
		assert_eq!(stats.bytes_transferred, 500);
		assert_eq!(stats.total_bytes, Some(1000));
		stats.calculate_percent();
		assert!((stats.percent - 50.0).abs() < 0.01);
	}
}
