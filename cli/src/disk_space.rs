//! Disk space utilities

use std::path::Path;

use anyhow::{Context, Result};
#[cfg(any(target_os = "macos", target_os = "linux"))]
use libc;

/// Get available disk space in bytes for a given path
pub fn get_available_space(path: &Path) -> Result<u64> {
	#[cfg(target_os = "macos")]
	{
		use std::{ffi::CString, os::unix::ffi::OsStrExt};

		let path_c = CString::new(path.as_os_str().as_bytes()).context("Failed to convert path to C string")?;

		let mut stat = unsafe { std::mem::zeroed() };
		let result = unsafe { libc::statfs(path_c.as_ptr(), &mut stat) };

		if result != 0 {
			// Try parent directory
			if let Some(parent) = path.parent() {
				return get_available_space(parent);
			}
			return Err(anyhow::anyhow!("Failed to stat filesystem"));
		}

		// f_bavail = free blocks available to non-super user
		// f_bsize = fundamental filesystem block size
		let available = unsafe { stat.f_bavail as u64 * stat.f_bsize as u64 };

		Ok(available)
	}

	#[cfg(target_os = "linux")]
	{
		use std::{ffi::CString, os::unix::ffi::OsStrExt};

		let path_c = CString::new(path.as_os_str().as_bytes()).context("Failed to convert path to C string")?;

		let mut stat = unsafe { std::mem::zeroed() };
		let result = unsafe { libc::statvfs(path_c.as_ptr(), &mut stat) };

		if result != 0 {
			// Try parent directory
			if let Some(parent) = path.parent() {
				return get_available_space(parent);
			}
			return Err(anyhow::anyhow!("Failed to stat filesystem"));
		}

		// f_bavail = free blocks available to non-super user
		// f_frsize = fundamental filesystem block size
		let available = unsafe { stat.f_bavail as u64 * stat.f_frsize as u64 };

		Ok(available)
	}

	#[cfg(not(any(target_os = "macos", target_os = "linux")))]
	{
		// Unsupported platform - return 0 to disable space checking
		Ok(0)
	}
}

/// Get the size of a directory in bytes (recursive)
pub fn get_directory_size(path: &Path) -> Result<u64> {
	let mut total_size = 0u64;

	if !path.exists() {
		return Ok(0);
	}

	for entry in walkdir::WalkDir::new(path) {
		let entry = entry?;
		if entry.file_type().is_file() {
			if let Ok(metadata) = entry.metadata() {
				total_size += metadata.len();
			}
		}
	}

	Ok(total_size)
}

/// Format bytes as human-readable string
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

/// Check if there's enough space for a backup
/// Returns Ok(true) if enough space, Ok(false) if not, Err if can't determine
pub fn check_available_space(dest_path: &Path, required_bytes: u64) -> Result<bool> {
	let available = get_available_space(dest_path)?;

	if available == 0 {
		// Can't determine available space (Windows or unsupported)
		// Assume there's enough space
		return Ok(true);
	}

	Ok(available >= required_bytes)
}

/// Check available space and return detailed error if insufficient
pub fn ensure_available_space(dest_path: &Path, required_bytes: u64) -> Result<()> {
	let available = get_available_space(dest_path)?;

	if available == 0 {
		// Can't determine - assume OK
		return Ok(());
	}

	if available < required_bytes {
		return Err(crate::error::BackupError::InsufficientSpace {
			dest_path: dest_path.display().to_string(),
			required: format_bytes(required_bytes),
			available: format_bytes(available),
		}
		.into());
	}

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_format_bytes() {
		assert_eq!(format_bytes(0), "0 B");
		assert_eq!(format_bytes(1024), "1.00 KB");
		assert_eq!(format_bytes(1024 * 1024), "1.00 MB");
		assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GB");
		assert_eq!(format_bytes(1024 * 1024 * 1024 * 1024), "1.00 TB");
	}

	#[test]
	fn test_get_directory_size() {
		// Test with current directory (should exist)
		let size = get_directory_size(Path::new(".")).unwrap();
		assert!(size > 0);
	}
}
