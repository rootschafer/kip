//! Rsync execution engine

use std::path::PathBuf;
use std::process::Stdio;
use std::time::Instant;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tracing::{debug, warn};

use crate::destination::Destination;
use crate::error::{Result, RsyncError};
use crate::platform::Platform;
use crate::progress::{ProgressTracker, parse_progress_line};
use crate::stats::RsyncStats;

/// Rsync operation builder
pub struct Rsync {
	source: PathBuf,
	destination: Destination,
	excludes: Vec<String>,
	delete: bool,
	compress: bool,
	dry_run: bool,
	verbose: bool,
	progress: Option<ProgressTracker>,
}

impl Rsync {
	/// Get the destination for this rsync operation
	pub fn destination(&self) -> &Destination {
		&self.destination
	}
	/// Create a new rsync operation
	pub fn new<P: Into<PathBuf>>(source: P, destination: Destination) -> Self {
		Self {
			source: source.into(),
			destination,
			excludes: vec![],
			delete: false,
			compress: false,
			dry_run: false,
			verbose: false,
			progress: None,
		}
	}

	/// Add an exclude pattern
	pub fn with_exclude(mut self, pattern: impl Into<String>) -> Self {
		self.excludes.push(pattern.into());
		self
	}

	/// Add multiple exclude patterns
	pub fn with_excludes(mut self, patterns: Vec<String>) -> Self {
		self.excludes.extend(patterns);
		self
	}

	/// Enable deletion of extra files at destination
	pub fn with_delete(mut self) -> Self {
		self.delete = true;
		self
	}

	/// Enable compression (useful for SSH transfers)
	pub fn with_compression(mut self) -> Self {
		self.compress = true;
		self
	}

	/// Enable dry-run mode
	pub fn with_dry_run(mut self) -> Self {
		self.dry_run = true;
		self
	}

	/// Enable verbose output
	pub fn with_verbose(mut self) -> Self {
		self.verbose = true;
		self
	}

	/// Set progress tracking callback
	pub fn with_progress(mut self, tracker: ProgressTracker) -> Self {
		self.progress = Some(tracker);
		self
	}

	/// Execute the rsync operation
	pub async fn run(self) -> Result<RsyncStats> {
		let start = Instant::now();

		// Verify source exists
		if !self.source.exists() {
			return Err(RsyncError::SourceNotFound(
				self.source.display().to_string(),
			));
		}

		// Build command
		let mut cmd = Command::new("rsync");
		self.build_command(&mut cmd);

		debug!("Running rsync: {:?}", cmd);

		// Execute
		let stats = if self.verbose {
			self.run_verbose(cmd).await?
		} else {
			self.run_with_progress(cmd).await?
		};

		// Set duration
		let mut stats = stats;
		stats.duration = Some(start.elapsed());
		if let Some(duration) = stats.duration {
			let secs = duration.as_secs();
			if secs > 0 {
				stats.avg_speed_bps = stats.bytes_transferred / secs;
			}
		}

		Ok(stats)
	}

	/// Build the rsync command
	fn build_command(&self, cmd: &mut Command) {
		let platform = Platform::detect();

		// Base flags
		cmd.args(&["-a", "--no-specials", "--no-devices"]);

		// Platform-specific flags
		for flag in platform.extra_flags() {
			cmd.arg(flag);
		}

		// Progress flag
		if !self.verbose {
			cmd.arg(platform.progress_flag());
		}

		// Delete flag
		if self.delete {
			cmd.arg("--delete");
		}

		// Compression
		if self.compress && self.destination.is_ssh() {
			cmd.arg("--compress");
		}

		// Dry run
		if self.dry_run {
			cmd.arg("--dry-run");
		}

		// Excludes
		for exclude in &self.excludes {
			cmd.arg("--exclude").arg(exclude);
		}

		// SSH configuration
		if let Some(ssh_opts) = self.destination.ssh_options() {
			// Build complete SSH command string for rsync -e flag
			let mut ssh_cmd = String::from("ssh");
			
			if let Some(ref key) = ssh_opts.identity_file {
				ssh_cmd.push_str(&format!(" -i '{}'", key.display()));
			}
			
			// Get port from destination
			let port = match &self.destination {
				crate::destination::Destination::Ssh { port, .. } => *port,
				_ => 22,
			};
			if port != 22 {
				ssh_cmd.push_str(&format!(" -p {}", port));
			}
			
			// Add default options for automated/rsync usage
			ssh_cmd.push_str(" -o StrictHostKeyChecking=no");
			ssh_cmd.push_str(" -o UserKnownHostsFile=/dev/null");
			ssh_cmd.push_str(" -o BatchMode=yes");
			ssh_cmd.push_str(" -o IdentitiesOnly=yes");
			
			// Add any extra options
			for option in &ssh_opts.extra_options {
				ssh_cmd.push_str(&format!(" -o '{}'", option));
			}
			
			cmd.arg("-e").arg(&ssh_cmd);
		}

		// Source and destination
		let source_path = format!("{}/", self.source.display());
		cmd.arg(&source_path);
		cmd.arg(&self.destination.format_for_rsync());
	}

	/// Run with verbose output (stream to console)
	async fn run_verbose(&self, mut cmd: Command) -> Result<RsyncStats> {
		let output = cmd
			.stderr(Stdio::inherit())
			.stdout(Stdio::inherit())
			.output()
			.await
			.map_err(|e| RsyncError::CommandFailed(e.to_string()))?;

		if !output.status.success() {
			let stderr = String::from_utf8_lossy(&output.stderr);
			return Err(self.handle_error(&stderr));
		}

		// Parse stats if available
		let stdout = String::from_utf8_lossy(&output.stdout);
		Ok(crate::stats::parse_stats_output(&stdout))
	}

	/// Run with progress tracking
	async fn run_with_progress(&self, mut cmd: Command) -> Result<RsyncStats> {
		// Add --stats flag for parsing output
		cmd.arg("--stats");
		
		// For SSH destinations, also capture stderr to see SSH errors
		let capture_stderr = self.destination.is_ssh();
		
		let mut child = cmd
			.stderr(Stdio::piped())
			.stdout(Stdio::piped())
			.spawn()
			.map_err(|e| {
				if e.kind() == std::io::ErrorKind::NotFound {
					RsyncError::RsyncNotFound
				} else {
					RsyncError::CommandFailed(e.to_string())
				}
			})?;
		
		let mut stats = RsyncStats::new();
		let mut all_output = String::new();
		let mut ssh_error = String::new();
		
		// Read stderr for progress and SSH errors
		if let Some(mut stderr) = child.stderr.take() {
			let mut reader = BufReader::new(&mut stderr);
			let mut line = Vec::new();
			
			while reader.read_until(b'\n', &mut line).await? > 0 {
				// Try to parse as UTF-8, skip if invalid
				if let Ok(line_str) = String::from_utf8(line.clone()) {
					// Capture SSH-related errors
					if capture_stderr && (line_str.contains("ssh:") || line_str.contains("Permission denied") || line_str.contains("Connection refused")) {
						ssh_error.push_str(&line_str);
					}
					
					// Parse progress line (lines with %)
					if line_str.contains('%') {
						if let Some(progress) = parse_progress_line(&line_str) {
							if let Some(ref tracker) = self.progress {
								tracker.update_bytes(progress.bytes_transferred);
							}
						}
					}
					all_output.push_str(&line_str);
				}
				line.clear();
			}
		}

		// Also read stdout for stats
		if let Some(mut stdout) = child.stdout.take() {
			let mut reader = BufReader::new(&mut stdout);
			let mut line = Vec::new();
			while reader.read_until(b'\n', &mut line).await? > 0 {
				if let Ok(line_str) = String::from_utf8(line.clone()) {
					all_output.push_str(&line_str);
				}
				line.clear();
			}
		}

		// Wait for child to complete
		let status = child.wait().await
			.map_err(|e| RsyncError::CommandFailed(e.to_string()))?;
		
		if !status.success() {
			// Include SSH error if available
			let error_msg = if !ssh_error.is_empty() {
				format!("rsync exited with {}: {}", status, ssh_error.trim())
			} else {
				format!("rsync exited with {}", status)
			};
			return Err(RsyncError::CommandFailed(error_msg));
		}

		// Parse stats from collected output
		stats = crate::stats::parse_stats_output(&all_output);

		// Update progress tracker with final stats
		if let Some(ref tracker) = self.progress {
			tracker.update_bytes(stats.bytes_transferred);
		}

		Ok(stats)
	}

	/// Handle rsync errors
	fn handle_error(&self, stderr: &str) -> RsyncError {
		let stderr_lower = stderr.to_lowercase();

		// Check for specific error types
		if stderr_lower.contains("no space left on device") || stderr_lower.contains("enospc") {
			RsyncError::InsufficientSpace {
				destination: self.destination.format_for_rsync(),
				required: "unknown".to_string(),
				available: "0 B".to_string(),
			}
		} else if stderr_lower.contains("permission denied") {
			RsyncError::CommandFailed(format!("Permission denied: {}", stderr))
		} else if stderr_lower.contains("connection refused") || stderr_lower.contains("could not resolve hostname") {
			if let Destination::Ssh { host, .. } = &self.destination {
				RsyncError::SshFailed {
					host: host.clone(),
					error: stderr.to_string(),
				}
			} else {
				RsyncError::CommandFailed(stderr.to_string())
			}
		} else {
			RsyncError::CommandFailed(stderr.to_string())
		}
	}
}

/// Create parent directory on remote SSH server
pub async fn ssh_mkdir(
	host: &str,
	user: &str,
	port: u16,
	path: &str,
	identity_file: Option<&PathBuf>,
) -> Result<()> {
	use std::process::Stdio;

	let mut cmd = Command::new("ssh");
	cmd.arg("-p").arg(port.to_string());
	cmd.arg("-o").arg("BatchMode=yes");

	if let Some(key) = identity_file {
		cmd.arg("-i").arg(key);
	}

	cmd.arg("-o").arg("IdentitiesOnly=yes");
	cmd.arg(format!("{}@{}", user, host));
	cmd.arg(format!("mkdir -p {}", path));

	debug!("Running SSH mkdir: {:?}", cmd);

	let output = cmd
		.stderr(Stdio::piped())
		.stdout(Stdio::piped())
		.output()
		.await
		.map_err(|e| RsyncError::CommandFailed(e.to_string()))?;

	if !output.status.success() {
		let stderr = String::from_utf8_lossy(&output.stderr);
		warn!("SSH mkdir failed: {}", stderr);
		// Don't fail - rsync might still work
	}

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_rsync_builder() {
		let rsync = Rsync::new("/tmp/source", Destination::local("/tmp/dest"))
			.with_exclude("*.tmp")
			.with_delete()
			.with_compression();

		// Just verify it builds without panicking
		assert!(rsync.source.exists() == false); // Won't exist in test
	}
}
