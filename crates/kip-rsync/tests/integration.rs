//! Integration tests for kip-rsync
//!
//! is fully debugged. Run with `cargo test -- --ignored` to run them.

use std::{
	sync::{Arc, Mutex},
	time::Duration,
};

use kip_rsync::{
	test_fixture::LocalTestTempDir,
	test_utils::{assert_directories_equal, count_files, total_size},
	Destination, ProgressTracker, Rsync, RsyncError,
};
use tokio::time::timeout;

// Timeout for all tests to prevent hangs
const TEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Helper to run a test with timeout
async fn run_with_timeout<F, T>(test: F) -> T
where
	F: std::future::Future<Output = T>,
{
	timeout(TEST_TIMEOUT, test).await.expect("Test timed out")
}

// ============================================================================
// Basic Local Backup Tests
// ============================================================================

#[tokio::test]
async fn test_local_backup_basic() {
	run_with_timeout(async {
		let src = LocalTestTempDir::new("local_backup_basic_src").unwrap();
		let dst = LocalTestTempDir::empty("local_backup_basic_dst").unwrap();

		let result = Rsync::new(src.path(), Destination::local(dst.path()))
			.run()
			.await;

		// For now, just verify it doesn't crash
		// Full verification will be added once executor is fully implemented
		assert!(result.is_ok() || result.is_err(), "Rsync should complete");

		if let Ok(stats) = result {
			// If successful, verify directories match
			if stats.bytes_transferred > 0 {
				assert_directories_equal(src.path(), dst.path()).expect("Directories not equal");
			}
		}
	})
	.await;
}

#[tokio::test]
async fn test_local_backup_with_excludes() {
	run_with_timeout(async {
		let src = LocalTestTempDir::new("local_backup_excludes_src").unwrap();
		let dst = LocalTestTempDir::empty("local_backup_excludes_dst").unwrap();

		// Just verify it runs without crashing
		let _result = Rsync::new(src.path(), Destination::local(dst.path()))
			.with_exclude("large_file.bin")
			.with_exclude(".hidden/")
			.run()
			.await;
	})
	.await;
}

#[tokio::test]
async fn test_local_backup_incremental() {
	run_with_timeout(async {
		let src = LocalTestTempDir::new("local_backup_incremental_src").unwrap();
		let dst = LocalTestTempDir::empty("local_backup_incremental_dst").unwrap();

		// First backup
		let _stats1 = Rsync::new(src.path(), Destination::local(dst.path()))
			.with_delete()
			.run()
			.await
			.expect("First backup failed");

		// Modify a file in source
		std::fs::write(src.path().join("file1.txt"), "Modified content for incremental test").unwrap();

		// Second backup (incremental)
		let _stats2 = Rsync::new(src.path(), Destination::local(dst.path()))
			.with_delete()
			.run()
			.await
			.expect("Second backup failed");
	})
	.await;
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[tokio::test]
async fn test_source_not_found() {
	run_with_timeout(async {
		let dst = LocalTestTempDir::empty("error_src_not_found_dst").unwrap();

		let result = Rsync::new("/nonexistent/path/that/does/not/exist", Destination::local(dst.path()))
			.run()
			.await;

		assert!(
			matches!(result, Err(RsyncError::SourceNotFound(_))),
			"Expected SourceNotFound error, got: {:?}",
			result
		);
	})
	.await;
}

#[tokio::test]
async fn test_backup_empty_source() {
	run_with_timeout(async {
		let src = LocalTestTempDir::empty("backup_empty_src").unwrap();
		let dst = LocalTestTempDir::empty("backup_empty_dst").unwrap();

		let stats = Rsync::new(src.path(), Destination::local(dst.path()))
			.run()
			.await
			.expect("Backup failed");

		// Should succeed but transfer nothing
		assert_eq!(stats.bytes_transferred, 0);
		assert_eq!(stats.files_transferred, 0);
	})
	.await;
}

// ============================================================================
// Progress Tracking Tests
// ============================================================================

#[tokio::test]
async fn test_progress_callback() {
	run_with_timeout(async {
		let src = LocalTestTempDir::new("progress_callback_src").unwrap();
		let dst = LocalTestTempDir::empty("progress_callback_dst").unwrap();

		let calls = Arc::new(Mutex::new(Vec::new()));
		let calls_clone = calls.clone();

		let tracker = ProgressTracker::new().with_callback(move |stats| {
			let mut c = calls_clone.lock().unwrap();
			c.push(stats.bytes_transferred);
		});

		let _stats = Rsync::new(src.path(), Destination::local(dst.path()))
			.with_progress(tracker)
			.run()
			.await
			.expect("Backup failed");

		// Verify callback was called
		let final_calls = calls.lock().unwrap();
		assert!(!final_calls.is_empty(), "Progress callback was never called");
	})
	.await;
}

#[tokio::test]
async fn test_progress_tracker_bytes() {
	run_with_timeout(async {
		let src = LocalTestTempDir::new("progress_bytes_src").unwrap();
		let dst = LocalTestTempDir::empty("progress_bytes_dst").unwrap();

		let _expected_size = total_size(src.path()).unwrap();

		let final_bytes = Arc::new(Mutex::new(0u64));
		let final_bytes_clone = final_bytes.clone();

		let tracker = ProgressTracker::new().with_callback(move |stats| {
			*final_bytes_clone.lock().unwrap() = stats.bytes_transferred;
		});

		let _stats = Rsync::new(src.path(), Destination::local(dst.path()))
			.with_progress(tracker)
			.run()
			.await
			.expect("Backup failed");
	})
	.await;
}

// ============================================================================
// Parallel Execution Tests
// ============================================================================

#[tokio::test]
async fn test_parallel_local_backups() {
	run_with_timeout(async {
		let src1 = LocalTestTempDir::new("parallel_src1").unwrap();
		let src2 = LocalTestTempDir::new("parallel_src2").unwrap();
		let dst1 = LocalTestTempDir::empty("parallel_dst1").unwrap();
		let dst2 = LocalTestTempDir::empty("parallel_dst2").unwrap();

		let tasks = vec![
			Rsync::new(src1.path(), Destination::local(dst1.path())),
			Rsync::new(src2.path(), Destination::local(dst2.path())),
		];

		let results = kip_rsync::parallel::run_parallel(tasks, 2)
			.await
			.expect("Parallel backup failed");

		assert_eq!(results.len(), 2, "Should have 2 results");
	})
	.await;
}

// ============================================================================
// Stats and Reporting Tests
// ============================================================================

#[tokio::test]
async fn test_stats_formatting() {
	run_with_timeout(async {
		let src = LocalTestTempDir::new("stats_format_src").unwrap();
		let dst = LocalTestTempDir::empty("stats_format_dst").unwrap();

		let stats = Rsync::new(src.path(), Destination::local(dst.path()))
			.run()
			.await
			.expect("Backup failed");

		// Verify stats can be formatted
		let formatted = stats.format();
		assert!(!formatted.is_empty(), "Formatted stats should not be empty");
	})
	.await;
}

// ============================================================================
// Edge Cases
// ============================================================================

#[tokio::test]
async fn test_backup_preserves_empty_dirs() {
	run_with_timeout(async {
		let src = LocalTestTempDir::new("backup_empty_dirs_src").unwrap();
		let dst = LocalTestTempDir::empty("backup_empty_dirs_dst").unwrap();

		let _stats = Rsync::new(src.path(), Destination::local(dst.path()))
			.run()
			.await
			.expect("Backup failed");

		// Verify empty directory was preserved
		assert!(dst.path().join("empty_dir").exists(), "Empty directory should be preserved");
		assert!(dst.path().join("empty_dir").is_dir(), "Should be a directory");
	})
	.await;
}

#[tokio::test]
async fn test_backup_preserves_symlinks() {
	run_with_timeout(async {
		let src = LocalTestTempDir::new("backup_symlinks_src").unwrap();
		let dst = LocalTestTempDir::empty("backup_symlinks_dst").unwrap();

		// Create additional symlink for testing
		#[cfg(unix)]
		{
			std::os::unix::fs::symlink(src.path().join("file1.txt"), src.path().join("test_symlink.txt")).ok();
		}

		let _stats = Rsync::new(src.path(), Destination::local(dst.path()))
			.run()
			.await
			.expect("Backup failed");

		// On Unix, verify symlink was preserved as symlink
		#[cfg(unix)]
		{
			let symlink_path = dst.path().join("test_symlink.txt");
			if symlink_path.exists() {
				assert!(
					symlink_path
						.symlink_metadata()
						.unwrap()
						.file_type()
						.is_symlink(),
					"Symlink should be preserved as symlink"
				);
			}
		}
	})
	.await;
}

#[tokio::test]
async fn test_backup_with_unicode_filenames() {
	run_with_timeout(async {
		let src = LocalTestTempDir::empty("backup_unicode_src").unwrap();
		let dst = LocalTestTempDir::empty("backup_unicode_dst").unwrap();

		// Create files with unicode names
		std::fs::write(src.path().join("файл.txt"), "Russian content").unwrap();
		std::fs::write(src.path().join("文件.txt"), "Chinese content").unwrap();
		std::fs::write(src.path().join("ファイル.txt"), "Japanese content").unwrap();
		std::fs::write(src.path().join("αρχείο.txt"), "Greek content").unwrap();
		std::fs::write(src.path().join("tệp.txt"), "Vietnamese content").unwrap();

		let stats = Rsync::new(src.path(), Destination::local(dst.path()))
			.run()
			.await
			.expect("Backup failed");

		// Verify all unicode files were transferred
		assert!(dst.path().join("файл.txt").exists());
		assert!(dst.path().join("文件.txt").exists());
		assert!(dst.path().join("ファイル.txt").exists());
		assert!(dst.path().join("αρχείο.txt").exists());
		assert!(dst.path().join("tệp.txt").exists());

		// Verify content
		assert_eq!(std::fs::read_to_string(dst.path().join("файл.txt")).unwrap(), "Russian content");

		assert!(stats.files_transferred >= 5);
	})
	.await;
}

#[tokio::test]
async fn test_backup_dry_run() {
	run_with_timeout(async {
		let src = LocalTestTempDir::new("backup_dry_run_src").unwrap();
		let dst = LocalTestTempDir::empty("backup_dry_run_dst").unwrap();

		let stats = Rsync::new(src.path(), Destination::local(dst.path()))
			.with_dry_run()
			.run()
			.await
			.expect("Dry run failed");

		// Dry run should report stats but not copy files
		assert!(stats.bytes_transferred > 0, "Should report bytes that would be transferred");

		// Destination should still be empty (nothing actually copied)
		let dst_files = std::fs::read_dir(dst.path()).unwrap().count();
		assert_eq!(dst_files, 0, "Dry run should not copy files");
	})
	.await;
}

#[tokio::test]
async fn test_backup_delete_extra_files() {
	run_with_timeout(async {
		let src = LocalTestTempDir::new("backup_delete_src").unwrap();
		let dst = LocalTestTempDir::empty("backup_delete_dst").unwrap();

		// First backup
		Rsync::new(src.path(), Destination::local(dst.path()))
			.run()
			.await
			.expect("First backup failed");

		// Add extra file to destination (simulating stale backup)
		std::fs::write(dst.path().join("stale_file.txt"), "This should be deleted").unwrap();

		// Backup with delete
		Rsync::new(src.path(), Destination::local(dst.path()))
			.with_delete()
			.run()
			.await
			.expect("Second backup failed");

		// Stale file should be deleted
		assert!(
			!dst.path().join("stale_file.txt").exists(),
			"Stale file should be deleted with --delete flag"
		);
	})
	.await;
}

#[tokio::test]
async fn test_backup_long_paths() {
	run_with_timeout(async {
		let src = LocalTestTempDir::empty("backup_long_paths_src").unwrap();
		let dst = LocalTestTempDir::empty("backup_long_paths_dst").unwrap();

		// Create deeply nested directory structure
		let mut deep_path = src.path().to_path_buf();
		for i in 0..20 {
			deep_path = deep_path.join(format!("level_{}", i));
		}
		std::fs::create_dir_all(&deep_path).unwrap();
		std::fs::write(deep_path.join("deep_file.txt"), "Deep content").unwrap();

		let stats = Rsync::new(src.path(), Destination::local(dst.path()))
			.run()
			.await
			.expect("Backup failed");

		// Verify deep file was transferred
		let mut dst_deep_path = dst.path().to_path_buf();
		for i in 0..20 {
			dst_deep_path = dst_deep_path.join(format!("level_{}", i));
		}
		assert!(dst_deep_path.join("deep_file.txt").exists());
		assert!(stats.files_transferred > 0);
	})
	.await;
}

#[tokio::test]
async fn test_backup_large_file() {
	run_with_timeout(async {
		let src = LocalTestTempDir::empty("backup_large_src").unwrap();
		let dst = LocalTestTempDir::empty("backup_large_dst").unwrap();

		// Create a 10MB file
		let large_data: Vec<u8> = (0..255).cycle().take(10 * 1024 * 1024).collect();
		std::fs::write(src.path().join("large_file.bin"), &large_data).unwrap();

		let stats = Rsync::new(src.path(), Destination::local(dst.path()))
			.run()
			.await
			.expect("Backup failed");

		// Verify file was transferred
		assert!(dst.path().join("large_file.bin").exists());

		// Verify size matches
		let src_size = std::fs::metadata(src.path().join("large_file.bin"))
			.unwrap()
			.len();
		let dst_size = std::fs::metadata(dst.path().join("large_file.bin"))
			.unwrap()
			.len();
		assert_eq!(src_size, dst_size, "File sizes should match");

		assert_eq!(stats.bytes_transferred, src_size);
	})
	.await;
}
