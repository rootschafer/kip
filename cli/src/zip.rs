//! Compression utilities for server backups

use std::{
	fs::File,
	path::{Path, PathBuf},
	process::Command,
};

use anyhow::{Context, Result};
use flate2::{write::GzEncoder, Compression};
use tar::Builder;

/// Create a tar.gz archive of a directory or file
pub fn create_tar_gz(source: &Path, dest: &Path, excludes: &[String]) -> Result<()> {
	let output = File::create(dest).with_context(|| format!("Failed to create archive: {}", dest.display()))?;

	let encoder = GzEncoder::new(output, Compression::default());
	let mut tar = Builder::new(encoder);

	let metadata = std::fs::metadata(source).with_context(|| format!("Failed to read source: {}", source.display()))?;

	if metadata.is_file() {
		// Source is a single file
		let file_name = source
			.file_name()
			.and_then(|s| s.to_str())
			.unwrap_or("file");
		let mut file = File::open(source)?;
		tar.append_file(file_name, &mut file)?;
	} else if metadata.is_dir() {
		// Source is a directory
		let source_name = source
			.file_name()
			.and_then(|s| s.to_str())
			.unwrap_or("source");

		// Check if this is a git repo and we should only backup ignored files
		let git_dir = source.join(".git");
		if git_dir.exists() {
			// This is a git repo - backup only ignored files
			add_git_ignored_to_tar(&mut tar, source, source_name, excludes)?;
		} else {
			// Regular directory - backup everything
			add_directory_to_tar(&mut tar, source, source_name, excludes)?;
		}
	} else {
		anyhow::bail!("Source is neither a file nor directory: {}", source.display());
	}

	tar.finish()
		.with_context(|| format!("Failed to finish archive: {}", dest.display()))?;

	Ok(())
}

/// Add only git-ignored files to a tar archive
fn add_git_ignored_to_tar<W: std::io::Write>(
	tar: &mut Builder<W>,
	path: &Path,
	base_name: &str,
	excludes: &[String],
) -> Result<()> {
	// Get list of ignored files from git
	let git_ls_output = Command::new("git")
		.args(&["ls-files", "--others", "--ignored", "--exclude-standard", "-z"])
		.current_dir(path)
		.output()
		.context("Failed to run git ls-files")?;

	if !git_ls_output.status.success() {
		// Git command failed, fall back to regular backup
		return add_directory_to_tar(tar, path, base_name, excludes);
	}

	let output_str = String::from_utf8_lossy(&git_ls_output.stdout);

	// Parse null-separated file list
	for file_path in output_str.split('\0').filter(|s| !s.is_empty()) {
		let full_path = path.join(file_path);

		// Skip if path doesn't exist
		if !full_path.exists() {
			continue;
		}

		// Check additional excludes
		let file_name = full_path.file_name().and_then(|s| s.to_str()).unwrap_or("");

		if should_exclude(file_name, excludes) {
			continue;
		}

		// Build relative path for archive
		let relative_path = if base_name.is_empty() {
			PathBuf::from(file_path)
		} else {
			PathBuf::from(base_name).join(file_path)
		};

		let metadata = std::fs::metadata(&full_path)?;

		// Skip special files (sockets, devices, etc.)
		let file_type = metadata.file_type();
		if !file_type.is_file() && !file_type.is_dir() {
			continue;
		}

		if metadata.is_file() {
			let mut file = File::open(&full_path)?;
			tar.append_file(&relative_path, &mut file)?;
		} else if metadata.is_dir() {
			// For directories, we need to ensure parent dirs exist in archive
			// but we don't add the directory itself (git ls-files gives us files)
		}
	}

	Ok(())
}

/// Recursively add a directory to a tar archive
fn add_directory_to_tar<W: std::io::Write>(
	tar: &mut Builder<W>,
	path: &Path,
	base_name: &str,
	excludes: &[String],
) -> Result<()> {
	let entries = std::fs::read_dir(path).with_context(|| format!("Failed to read directory: {}", path.display()))?;

	for entry in entries {
		let entry = entry?;
		let entry_path = entry.path();

		// Get the name for exclusion checking
		let name = entry_path
			.file_name()
			.and_then(|s| s.to_str())
			.unwrap_or("");

		// Check exclusions
		if should_exclude(name, excludes) {
			continue;
		}

		// Build the relative path for the archive
		let relative_path = if base_name.is_empty() {
			PathBuf::from(name)
		} else {
			PathBuf::from(base_name).join(name)
		};

		let metadata = entry.metadata()?;

		// Skip special files (sockets, devices, etc.)
		let file_type = metadata.file_type();
		if !file_type.is_file() && !file_type.is_dir() {
			continue; // Skip sockets, symlinks, devices, etc.
		}

		if metadata.is_file() {
			let mut file = File::open(&entry_path)?;
			tar.append_file(&relative_path, &mut file)?;
		} else if metadata.is_dir() {
			add_directory_to_tar(tar, &entry_path, &relative_path.to_string_lossy(), excludes)?;
		}
	}

	Ok(())
}

/// Check if a file/folder name matches any exclude pattern
fn should_exclude(name: &str, excludes: &[String]) -> bool {
	for pattern in excludes {
		if matches_pattern(pattern, name) {
			return true;
		}
	}
	false
}

/// Simple glob pattern matching (supports * wildcard)
fn matches_pattern(pattern: &str, name: &str) -> bool {
	if pattern == name {
		return true;
	}

	// Handle simple * wildcard patterns
	if pattern.contains('*') {
		let parts: Vec<&str> = pattern.split('*').collect();
		if parts.len() == 2 {
			let prefix = parts[0];
			let suffix = parts[1];

			if name.starts_with(prefix) && name.ends_with(suffix) {
				return true;
			}
		}
	}

	false
}

/// Extract a tar.gz archive
pub fn extract_tar_gz(archive: &Path, dest: &Path) -> Result<()> {
	let file = File::open(archive).with_context(|| format!("Failed to open archive: {}", archive.display()))?;

	let decoder = flate2::read::GzDecoder::new(file);
	let mut tar = tar::Archive::new(decoder);

	tar.unpack(dest)
		.with_context(|| format!("Failed to extract archive: {}", archive.display()))?;

	Ok(())
}

/// Get the size of a tar.gz archive
pub fn get_archive_size(archive: &Path) -> Result<u64> {
	let metadata =
		std::fs::metadata(archive).with_context(|| format!("Failed to get metadata: {}", archive.display()))?;

	Ok(metadata.len())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_pattern_matching() {
		assert!(matches_pattern("*.log", "test.log"));
		assert!(matches_pattern("*.log", "debug.log"));
		assert!(!matches_pattern("*.log", "test.txt"));

		assert!(matches_pattern("cache/*", "cache/test"));
		assert!(!matches_pattern("cache/*", "other/test"));
	}
}
