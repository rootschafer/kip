//! Backup status monitoring
//!
//! Reads progress state from a file and displays real-time backup status.

use std::{
	fs,
	path::PathBuf,
	time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use console::style;
use serde::{Deserialize, Serialize};
#[cfg(unix)]
use libc;

/// Progress state written by backup process
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BackupStatus {
	/// Whether a backup is currently running
	pub is_running: bool,
	/// Process ID of running backup
	pub pid: Option<u32>,
	/// Start time (Unix timestamp)
	pub started_at: Option<u64>,
	/// Total folders to backup
	pub total_folders: Option<u64>,
	/// Folders completed
	pub completed_folders: u64,
	/// Current folder being processed
	pub current_folder: Option<String>,
	/// Bytes transferred
	pub bytes_transferred: u64,
	/// Estimated total bytes
	pub total_bytes: Option<u64>,
	/// Last update time
	pub last_updated: Option<u64>,
	/// Errors encountered
	pub errors: Vec<String>,
}

impl BackupStatus {
	pub fn status_file_path() -> PathBuf {
		dirs::config_dir()
			.unwrap_or_else(|| PathBuf::from("."))
			.join("backup-tool")
			.join("backup-status.json")
	}

	/// Load current status from file
	pub fn load() -> Result<Self> {
		let path = Self::status_file_path();
		if !path.exists() {
			return Ok(Self::default());
		}

		let content = fs::read_to_string(&path).context("Failed to read status file")?;
		serde_json::from_str(&content).context("Failed to parse status file")
	}

	/// Save status to file
	pub fn save(&self) -> Result<()> {
		let path = Self::status_file_path();

		// Ensure parent directory exists
		if let Some(parent) = path.parent() {
			fs::create_dir_all(parent)?;
		}

		let content = serde_json::to_string_pretty(self)?;
		fs::write(&path, content)?;
		Ok(())
	}

	/// Clear status (backup completed)
	pub fn clear() -> Result<()> {
		let path = Self::status_file_path();
		if path.exists() {
			fs::remove_file(&path)?;
		}
		Ok(())
	}

	/// Check if the backup process is still running
	pub fn is_process_running(&self) -> bool {
		if let Some(pid) = self.pid {
			#[cfg(unix)]
			{
				// Check if process exists
				unsafe { libc::kill(pid as i32, 0) == 0 }
			}
			#[cfg(not(unix))]
			{
				false
			}
		} else {
			false
		}
	}

	/// Calculate ETA based on current progress
	pub fn eta(&self) -> Option<Duration> {
		if let (Some(started), Some(total), _) = (self.started_at, self.total_folders, &self.current_folder) {
			let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();

			let elapsed = now - started;
			let completed = self.completed_folders;

			if completed > 0 && elapsed > 0 {
				let rate = completed as f64 / elapsed as f64; // folders per second
				let remaining = total.saturating_sub(completed);
				let eta_secs = (remaining as f64 / rate) as u64;
				Some(Duration::from_secs(eta_secs))
			} else {
				None
			}
		} else {
			None
		}
	}
}

/// Display current backup status
pub fn show_status() -> Result<()> {
	let status = BackupStatus::load()?;

	if !status.is_running {
		println!("{} No backup currently running", style("ℹ️").blue());
		return Ok(());
	}

	// Check if process is still alive
	if !status.is_process_running() {
		println!("{} Backup process appears to have stopped", style("⚠️").yellow());
		println!("   Last known PID: {}", status.pid.unwrap_or(0));
		if let Some(last_updated) = status.last_updated {
			let ago = SystemTime::now()
				.duration_since(UNIX_EPOCH)
				.ok()
				.map(|t| t.as_secs() - last_updated)
				.unwrap_or(0);
			println!("   Last update: {} seconds ago", ago);
		}
		println!();
		println!(
			"   {} Stale status file detected. Run `kip status --clear` to reset",
			style("💡").blue()
		);
		return Ok(());
	}

	// Display status
	println!("\n{}", style("🔄 Backup in Progress").bold().cyan());
	println!("{}", "=".repeat(60));

	if let Some(pid) = status.pid {
		println!("{} Process ID: {}", style("PID:").bold(), pid);
	}

	if let Some(started) = status.started_at {
		let now = SystemTime::now()
			.duration_since(UNIX_EPOCH)
			.ok()
			.unwrap_or_default()
			.as_secs();
		let elapsed = now - started;
		let mins = elapsed / 60;
		let secs = elapsed % 60;
		println!("{} Running for: {}m {}s", style("Time:").bold(), mins, secs);
	}

	if let (Some(total), Some(_current)) = (status.total_folders, &status.current_folder) {
		let completed = status.completed_folders;
		let percent = if total > 0 {
			(completed as f64 / total as f64 * 100.0) as u32
		} else {
			0
		};
		println!("{} {} / {} folders ({}%)", style("Progress:").bold(), completed, total, percent);

		// Progress bar
		let bar_width = 40;
		let filled = (percent as f64 / 100.0 * bar_width as f64) as usize;
		let empty = bar_width - filled;
		print!("   [");
		for _ in 0..filled {
			print!("{}", style("█").green());
		}
		for _ in 0..empty {
			print!("{}", style("░").dim());
		}
		println!("]");
	}

	if let Some(ref current) = status.current_folder {
		println!("{} {}", style("Current:").bold(), current);
	}

	if status.bytes_transferred > 0 {
		println!(
			"{} {}",
			style("Transferred:").bold(),
			crate::progress::format_bytes(status.bytes_transferred)
		);
	}

	if let Some(eta) = status.eta() {
		let mins = eta.as_secs() / 60;
		let secs = eta.as_secs() % 60;
		println!("{} {}m {}s remaining", style("ETA:").bold(), mins, secs);
	}

	if !status.errors.is_empty() {
		println!();
		println!("{} {} errors encountered:", style("⚠️").yellow(), status.errors.len());
		for error in &status.errors {
			println!("   • {}", style(error).dim());
		}
	}

	println!();

	Ok(())
}
