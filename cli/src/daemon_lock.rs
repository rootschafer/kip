//! Daemon lock management
//!
//! Ensures only one backup daemon runs at a time using a PID file.

use std::{
	fs::{self, File},
	io::{Read, Write},
	path::PathBuf,
	process,
};

use anyhow::{Context, Result};

const LOCK_FILE_NAME: &str = "kip-backup.lock";

/// Get the lock file path
fn get_lock_file_path() -> PathBuf {
	dirs::config_dir()
		.unwrap_or_else(|| PathBuf::from("."))
		.join("backup-tool")
		.join(LOCK_FILE_NAME)
}

/// Try to acquire the daemon lock
/// Returns Ok(true) if lock acquired, Ok(false) if another daemon is running
pub fn try_acquire_lock() -> Result<bool> {
	let lock_path = get_lock_file_path();

	// Ensure parent directory exists
	if let Some(parent) = lock_path.parent() {
		fs::create_dir_all(parent)?;
	}

	// Try to create lock file exclusively
	match File::options()
		.write(true)
		.create_new(true)
		.open(&lock_path)
	{
		Ok(mut file) => {
			// Write our PID to the lock file
			let pid = process::id();
			writeln!(file, "{}", pid).context("Failed to write PID to lock file")?;
			Ok(true)
		}
		Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
			// Lock file exists - check if the process is still running
			if let Ok(mut file) = File::open(&lock_path) {
				let mut contents = String::new();
				if file.read_to_string(&mut contents).is_ok() {
					if let Ok(existing_pid) = contents.trim().parse::<u32>() {
						// Check if process exists
						if !is_process_running(existing_pid) {
							// Stale lock file - remove it and try again
							fs::remove_file(&lock_path).ok();
							return try_acquire_lock();
						}
					}
				}
			}
			// Another daemon is running
			Ok(false)
		}
		Err(e) => Err(e).context("Failed to create lock file"),
	}
}

/// Release the daemon lock
pub fn release_lock() -> Result<()> {
	let lock_path = get_lock_file_path();
	fs::remove_file(&lock_path).ok();
	Ok(())
}

/// Check if a process with given PID is running
fn is_process_running(pid: u32) -> bool {
	#[cfg(unix)]
	{
		use libc;
		unsafe { libc::kill(pid as i32, 0) == 0 }
	}
	#[cfg(windows)]
	{
		// Windows implementation would use OpenProcess
		false
	}
}

/// Get the PID of the running daemon (if any)
pub fn get_running_daemon_pid() -> Option<u32> {
	let lock_path = get_lock_file_path();
	if !lock_path.exists() {
		return None;
	}

	if let Ok(mut file) = File::open(&lock_path) {
		let mut contents = String::new();
		if file.read_to_string(&mut contents).is_ok() {
			if let Ok(pid) = contents.trim().parse::<u32>() {
				if is_process_running(pid) {
					return Some(pid);
				}
			}
		}
	}

	None
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_lock_file_path() {
		let path = get_lock_file_path();
		assert!(path.ends_with(LOCK_FILE_NAME));
	}
}
