//! Safety checks and validation for backup operations

use std::{
	path::{Path, PathBuf},
	process::Command,
};

use anyhow::{Context, Result};
use tracing::warn;

/// Critical paths that should NEVER be backup destinations
const FORBIDDEN_DESTINATIONS: &[&str] = &[
	"/", "/System", "/Applications", "/Library", "/usr", "/bin", "/sbin", "/etc", "/var", "/tmp", "/private", "/Users",
	"/home",
];

/// Validate that a destination path is safe for backups
pub fn validate_backup_destination(dest: &Path) -> Result<()> {
	let dest_str = dest.to_string_lossy();

	// Check against forbidden destinations
	for forbidden in FORBIDDEN_DESTINATIONS {
		if dest_str.as_ref() == *forbidden || dest_str.starts_with(&format!("{}/", forbidden)) {
			// Allow /Volumes and /mnt paths
			if !dest_str.starts_with("/Volumes") && !dest_str.starts_with("/mnt") {
				anyhow::bail!(
					"CRITICAL: Destination '{}' is a protected system path. \
                     Backup destinations must be on external drives (/Volumes/*) or network mounts (/mnt/*).",
					dest.display()
				);
			}
		}
	}

	// Verify destination exists or parent exists
	if !dest.exists() {
		if let Some(parent) = dest.parent() {
			if !parent.exists() {
				anyhow::bail!("Destination parent directory does not exist: {}", parent.display());
			}
		}
	}

	Ok(())
}

/// Check if source directory is empty (would cause --delete to wipe destination)
pub fn check_source_not_empty(source: &Path) -> Result<bool> {
	if !source.exists() {
		return Ok(false);
	}

	let mut entries =
		std::fs::read_dir(source).with_context(|| format!("Failed to read source directory: {}", source.display()))?;

	Ok(entries.next().is_some())
}

/// Validate backup source exists
pub fn validate_backup_source(source: &Path) -> Result<()> {
	if !source.exists() {
		anyhow::bail!("Source path does not exist: {}", source.display());
	}

	if !source.is_dir() {
		anyhow::bail!("Source path is not a directory: {}", source.display());
	}

	Ok(())
}

/// Run rsync in dry-run mode to preview changes
pub fn rsync_dry_run(source: &Path, dest: &Path, excludes: &[String]) -> Result<DryRunResult> {
	let mut cmd = Command::new("rsync");
	cmd.args(&[
		"-avn", // archive, verbose, dry-run
		"--delete", "--no-specials", "--no-devices",
	]);

	for exclude in excludes {
		cmd.arg("--exclude").arg(exclude);
	}

	cmd.arg(format!("{}/", source.display()))
		.arg(format!("{}/", dest.display()));

	let output = cmd.output().context("Failed to execute rsync dry-run")?;

	let stdout = String::from_utf8_lossy(&output.stdout);
	let stderr = String::from_utf8_lossy(&output.stderr);

	// Parse the output to count changes
	let mut files_to_transfer = 0;
	let mut files_to_delete = 0;
	let mut bytes_to_transfer = 0u64;

	for line in stdout.lines() {
		// Skip header lines
		if line.starts_with("sending ") || line.starts_with("receiving ") || line.is_empty() {
			continue;
		}

		// New rsync format: "Transfer starting: 23134 files"
		if line.starts_with("Transfer starting:") {
			if let Some(files_str) = line.split_whitespace().nth(2) {
				if let Ok(files) = files_str.parse::<usize>() {
					files_to_transfer = files;
				}
			}
			// Can't parse bytes from this format, but we know files need to transfer
			bytes_to_transfer = 1; // Set to non-zero to indicate work needed
			continue;
		}

		// Old rsync format: Lines starting with > are files that would be transferred
		if line.starts_with("> ") {
			files_to_transfer += 1;
			// Try to parse size from line like "> filename.txt  1234 bytes"
			if let Some(size_str) = line.split_whitespace().last() {
				if let Ok(size) = size_str.parse::<u64>() {
					bytes_to_transfer += size;
				}
			}
		}

		// Lines with .d.. are directories
		// Lines starting with < are deletions
		if line.starts_with("< ") || (line.contains(".d..") && line.contains("deleting")) {
			files_to_delete += 1;
		}

		// Check for explicit deleting markers
		if line.contains("deleting ") {
			files_to_delete += 1;
		}
	}

	Ok(DryRunResult {
		files_to_transfer,
		files_to_delete,
		bytes_to_transfer,
		stdout: stdout.to_string(),
		stderr: stderr.to_string(),
		success: output.status.success(),
	})
}

/// Result of a dry-run operation
#[derive(Debug, Clone)]
pub struct DryRunResult {
	pub files_to_transfer: usize,
	pub files_to_delete: usize,
	pub bytes_to_transfer: u64,
	pub stdout: String,
	pub stderr: String,
	pub success: bool,
}

impl DryRunResult {
	/// Check if this dry-run result is safe to proceed
	pub fn is_safe(&self) -> bool {
		// Not safe if rsync failed
		if !self.success {
			return false;
		}

		// Not safe if we're deleting more than 50% of what we're transferring
		// (could indicate wrong source/dest)
		if self.files_to_delete > 0 && self.files_to_transfer > 0 {
			let ratio = self.files_to_delete as f64 / self.files_to_transfer as f64;
			if ratio > 0.5 {
				return false;
			}
		}

		true
	}

	/// Get a summary of changes
	pub fn summary(&self) -> String {
		format!(
			"Transfer: {} files ({:.2} MB), Delete: {} files",
			self.files_to_transfer,
			self.bytes_to_transfer as f64 / (1024.0 * 1024.0),
			self.files_to_delete
		)
	}
}

/// Validate the backup drive is actually an external drive
pub fn validate_backup_drive(mount_point: &Path) -> Result<()> {
	if !mount_point.exists() {
		anyhow::bail!("Backup drive not mounted: {}", mount_point.display());
	}

	// On macOS, external drives are in /Volumes
	if !mount_point.starts_with("/Volumes") && !mount_point.starts_with("/mnt") {
		warn!("Backup drive is not on an external mount point: {}", mount_point.display());
		warn!("Expected /Volumes/* or /mnt/* for external drives");
	}

	// Check if it's actually a mount point (not just a directory)
	#[cfg(target_os = "macos")]
	{
		use std::process::Command;
		let output = Command::new("mount").arg(mount_point).output();

		if let Ok(out) = output {
			if !out.status.success() {
				anyhow::bail!("Path is not a mounted filesystem: {}", mount_point.display());
			}
		}
	}

	Ok(())
}

/// Extension trait for Path to check if it's a child of another
pub trait PathExt {
	fn is_child_of(&self, parent: &Path) -> bool;
}

impl PathExt for Path {
	fn is_child_of(&self, parent: &Path) -> bool {
		self.starts_with(parent) && self != parent
	}
}

impl PathExt for PathBuf {
	fn is_child_of(&self, parent: &Path) -> bool {
		self.as_path().is_child_of(parent)
	}
}
