//! Progress tracking for backup operations

use std::{
	fmt::Write,
	sync::{
		atomic::{AtomicBool, AtomicU64, Ordering},
		Arc,
	},
};

use indicatif::{MultiProgress, ProgressBar, ProgressState, ProgressStyle};

/// Shared state for tracking backup progress
#[derive(Clone, Debug)]
pub struct BackupProgress {
	/// Total folders to backup
	pub total_folders: u64,
	/// Current folder index
	pub current_folder: Arc<AtomicU64>,
	/// Current folder name
	pub current_name: Arc<parking_lot::Mutex<String>>,
	/// Bytes transferred in current operation
	pub bytes_transferred: Arc<AtomicU64>,
	/// Total bytes to transfer (estimated)
	pub total_bytes: Arc<AtomicU64>,
	/// Whether cancellation was requested
	pub cancel_requested: Arc<AtomicBool>,
	/// Multi-progress bar for displaying multiple operations
	pub multi_progress: Option<MultiProgress>,
}

impl BackupProgress {
	pub fn new(total_folders: usize) -> Self {
		Self {
			total_folders: total_folders as u64,
			current_folder: Arc::new(AtomicU64::new(0)),
			current_name: Arc::new(parking_lot::Mutex::new(String::new())),
			bytes_transferred: Arc::new(AtomicU64::new(0)),
			total_bytes: Arc::new(AtomicU64::new(0)),
			cancel_requested: Arc::new(AtomicBool::new(false)),
			multi_progress: None,
		}
	}

	/// Create a new progress tracker with external cancel flag
	pub fn with_cancel_flag(total_folders: usize, cancel_flag: Arc<AtomicBool>) -> Self {
		Self {
			total_folders: total_folders as u64,
			current_folder: Arc::new(AtomicU64::new(0)),
			current_name: Arc::new(parking_lot::Mutex::new(String::new())),
			bytes_transferred: Arc::new(AtomicU64::new(0)),
			total_bytes: Arc::new(AtomicU64::new(0)),
			cancel_requested: cancel_flag,
			multi_progress: None,
		}
	}

	/// Check if cancellation was requested
	pub fn is_cancelled(&self) -> bool {
		self.cancel_requested.load(Ordering::Relaxed)
	}

	/// Request cancellation
	pub fn request_cancel(&self) {
		self.cancel_requested.store(true, Ordering::Relaxed);
	}

	/// Update current folder being processed
	pub fn set_current_folder(&self, name: String) {
		let mut current = self.current_name.lock();
		*current = name;
	}

	/// Increment folder counter
	pub fn advance_folder(&self) {
		self.current_folder.fetch_add(1, Ordering::Relaxed);
	}

	/// Update bytes transferred
	pub fn set_bytes(&self, transferred: u64, total: u64) {
		self.bytes_transferred.store(transferred, Ordering::Relaxed);
		self.total_bytes.store(total, Ordering::Relaxed);
	}
}

/// Create a progress bar for rsync operations
pub fn create_rsync_progress_bar() -> ProgressBar {
	let pb = ProgressBar::new(100);
	pb.set_style(
		ProgressStyle::with_template(
			"{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({percent}%)",
		)
		.unwrap()
		.with_key("percent", |state: &ProgressState, w: &mut dyn Write| {
			write!(w, "{:.1}%", state.fraction() * 100.0).unwrap()
		})
		.progress_chars("█▓▒░ "),
	);
	pb
}

/// Create a multi-progress display for backup operations
pub fn create_multi_progress() -> MultiProgress {
	MultiProgress::new()
}

/// Format bytes in human-readable format
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
