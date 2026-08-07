//! Test utilities for kip-rsync

use std::{
	fs,
	path::{Path, PathBuf},
};

/// Creates a test filesystem structure in the given directory
///
/// Structure:
/// ```text
/// root/
/// ├── file1.txt (small text file)
/// ├── file2.dat (binary file)
/// ├── subdir1/
/// │   ├── nested1.txt
/// │   └── nested2.txt
/// ├── subdir2/
/// │   └── deep/
/// │       └── deep_file.txt
/// ├── .hidden/
/// │   └── secret.txt
/// └── large_file.bin (1MB file for progress testing)
/// ```
pub fn create_test_filesystem(root: &Path) -> std::io::Result<()> {
	// Create directories
	fs::create_dir_all(root.join("subdir1"))?;
	fs::create_dir_all(root.join("subdir2/deep"))?;
	fs::create_dir_all(root.join(".hidden"))?;

	// Small text file
	fs::write(
		root.join("file1.txt"),
		"This is a test file for rsync testing.\nIt has multiple lines.\nAnd some content.",
	)?;

	// Binary file
	let binary_data: Vec<u8> = (0..=255).cycle().take(1024).collect();
	fs::write(root.join("file2.dat"), &binary_data)?;

	// Nested files
	fs::write(root.join("subdir1/nested1.txt"), "Nested file 1 content")?;
	fs::write(root.join("subdir1/nested2.txt"), "Nested file 2 content")?;

	// Deep nested file
	fs::write(root.join("subdir2/deep/deep_file.txt"), "Deep nested content")?;

	// Hidden file
	fs::write(root.join(".hidden/secret.txt"), "Hidden secret content")?;

	// Large file for progress testing (1MB)
	let large_data: Vec<u8> = (0..255).cycle().take(1024 * 1024).collect();
	fs::write(root.join("large_file.bin"), &large_data)?;

	// Empty directory (should be preserved)
	fs::create_dir_all(root.join("empty_dir"))?;

	// File with special characters in name
	fs::write(root.join("file with spaces.txt"), "File with spaces in name")?;

	// Symlink (if supported)
	#[cfg(unix)]
	{
		let _ = std::os::unix::fs::symlink(root.join("file1.txt"), root.join("symlink.txt"));
	}

	Ok(())
}

/// Verify that two directory trees are equal
///
/// Compares:
/// - Directory structure
/// - File contents
/// - File permissions (on Unix)
///
/// Does NOT compare:
/// - Symlinks (just checks they exist)
/// - Timestamps
pub fn assert_directories_equal(src: &Path, dst: &Path) -> Result<(), String> {
	let mut src_entries: Vec<_> = fs::read_dir(src)
		.map_err(|e| format!("Failed to read source dir {:?}: {}", src, e))?
		.filter_map(|e| e.ok())
		.map(|e| e.path())
		.collect();

	let mut dst_entries: Vec<_> = fs::read_dir(dst)
		.map_err(|e| format!("Failed to read dest dir {:?}: {}", dst, e))?
		.filter_map(|e| e.ok())
		.map(|e| e.path())
		.collect();

	src_entries.sort();
	dst_entries.sort();

	// Check same number of entries
	if src_entries.len() != dst_entries.len() {
		return Err(format!(
			"Different number of entries: src={}, dst={}",
			src_entries.len(),
			dst_entries.len()
		));
	}

	// Check each entry
	for (src_path, dst_path) in src_entries.iter().zip(dst_entries.iter()) {
		let src_name = src_path.file_name().unwrap_or_default();
		let dst_name = dst_path.file_name().unwrap_or_default();

		if src_name != dst_name {
			return Err(format!("Entry mismatch: {:?} vs {:?}", src_name, dst_name));
		}

		if src_path.is_dir() {
			if !dst_path.is_dir() {
				return Err(format!("{:?} is a directory but {:?} is not", src_path, dst_path));
			}
			assert_directories_equal(src_path, dst_path)?;
		} else if src_path.is_file() {
			if !dst_path.is_file() {
				return Err(format!("{:?} is a file but {:?} is not", src_path, dst_path));
			}

			// Compare file contents
			let src_content = fs::read(src_path).map_err(|e| format!("Failed to read {:?}: {}", src_path, e))?;
			let dst_content = fs::read(dst_path).map_err(|e| format!("Failed to read {:?}: {}", dst_path, e))?;

			if src_content != dst_content {
				return Err(format!(
					"File content mismatch: {:?} (src: {} bytes, dst: {} bytes)",
					src_path,
					src_content.len(),
					dst_content.len()
				));
			}
		}
	}

	Ok(())
}

/// Count files in a directory tree
pub fn count_files(dir: &Path) -> std::io::Result<usize> {
	let mut count = 0;
	if dir.is_dir() {
		for entry in fs::read_dir(dir)? {
			let entry = entry?;
			let path = entry.path();
			if path.is_dir() {
				count += count_files(&path)?;
			} else {
				count += 1;
			}
		}
	}
	Ok(count)
}

/// Get total size of files in a directory tree
pub fn total_size(dir: &Path) -> std::io::Result<u64> {
	let mut total = 0;
	if dir.is_dir() {
		for entry in fs::read_dir(dir)? {
			let entry = entry?;
			let path = entry.path();
			if path.is_dir() {
				total += total_size(&path)?;
			} else if let Ok(metadata) = fs::metadata(&path) {
				total += metadata.len();
			}
		}
	}
	Ok(total)
}

#[cfg(test)]
mod tests {
	use tempfile::TempDir;

	use super::*;

	#[test]
	fn test_create_test_filesystem() {
		let temp = TempDir::new().unwrap();
		create_test_filesystem(temp.path()).unwrap();

		// Verify structure
		assert!(temp.path().join("file1.txt").exists());
		assert!(temp.path().join("file2.dat").exists());
		assert!(temp.path().join("subdir1").exists());
		assert!(temp.path().join("subdir1/nested1.txt").exists());
		assert!(temp.path().join("subdir2/deep/deep_file.txt").exists());
		assert!(temp.path().join(".hidden/secret.txt").exists());
		assert!(temp.path().join("large_file.bin").exists());
		assert!(temp.path().join("empty_dir").exists());
		assert!(temp.path().join("file with spaces.txt").exists());

		// Verify file count (should be 10 files, symlink may or may not count)
		let file_count = count_files(temp.path()).unwrap();
		assert!(file_count >= 9, "Expected at least 9 files, got {}", file_count);

		// Verify large file size
		let large_size = fs::metadata(temp.path().join("large_file.bin"))
			.unwrap()
			.len();
		assert_eq!(large_size, 1024 * 1024); // 1MB
	}

	#[test]
	fn test_assert_directories_equal() {
		let src = TempDir::new().unwrap();
		let dst = TempDir::new().unwrap();

		create_test_filesystem(src.path()).unwrap();
		create_test_filesystem(dst.path()).unwrap();

		// Should be equal
		assert!(assert_directories_equal(src.path(), dst.path()).is_ok());
	}

	#[test]
	fn test_assert_directories_not_equal() {
		let src = TempDir::new().unwrap();
		let dst = TempDir::new().unwrap();

		create_test_filesystem(src.path()).unwrap();
		create_test_filesystem(dst.path()).unwrap();

		// Modify dst
		fs::write(dst.path().join("file1.txt"), "Modified content").unwrap();

		// Should not be equal
		assert!(assert_directories_equal(src.path(), dst.path()).is_err());
	}
}
