//! Integration tests for kip-rclone
//!
//! These tests require rclone to be installed and configured.
//! Run with: cargo test -- --ignored

use kip_rclone::{CloudDestination, Rclone};
use tempfile::TempDir;

const TEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);

async fn run_with_timeout<F, T>(test: F) -> T
where
	F: std::future::Future<Output = T>,
{
	tokio::time::timeout(TEST_TIMEOUT, test)
		.await
		.expect("Test timed out")
}

/// Create a test file structure
fn create_test_files(dir: &TempDir) -> std::io::Result<()> {
	std::fs::create_dir_all(dir.path().join("subdir"))?;
	std::fs::write(dir.path().join("file1.txt"), "Test file 1 content")?;
	std::fs::write(dir.path().join("subdir/file2.txt"), "Test file 2 content")?;
	Ok(())
}

#[tokio::test]
#[ignore = "Requires rclone installed and 'test_remote' configured"]
async fn test_rclone_check_installed() {
	let installed = Rclone::check_installed().await.unwrap();
	assert!(installed, "rclone should be installed for this test");
}

#[tokio::test]
#[ignore = "Requires rclone installed"]
async fn test_rclone_version() {
	let version = Rclone::version().await.unwrap();
	assert!(version.contains("rclone"), "Version should contain 'rclone'");
	eprintln!("rclone version: {}", version);
}

#[tokio::test]
#[ignore = "Requires rclone installed and configured"]
async fn test_rclone_list_remotes() {
	let remotes = Rclone::list_remotes().await.unwrap();
	// At least verify it doesn't error
	eprintln!("Configured remotes: {:?}", remotes);
}

#[tokio::test]
#[ignore = "Requires rclone installed and 'test_remote' configured"]
async fn test_rclone_copy() {
	run_with_timeout(async {
		let src = TempDir::new().unwrap();
		create_test_files(&src).unwrap();

		let dest = CloudDestination::generic("test_remote", "kip_test_copy");

		let stats = Rclone::new()
			.copy(src.path(), &dest, "")
			.await
			.expect("Copy failed");

		assert!(stats.bytes_transferred > 0, "Should transfer data");
		assert!(stats.files_transferred > 0, "Should transfer files");

		eprintln!("Copy completed: {}", stats.format());
	})
	.await;
}

#[tokio::test]
#[ignore = "Requires rclone installed and 'test_remote' configured"]
async fn test_rclone_sync() {
	run_with_timeout(async {
		let src = TempDir::new().unwrap();
		create_test_files(&src).unwrap();

		let dest = CloudDestination::generic("test_remote", "kip_test_sync");

		let stats = Rclone::new()
			.sync(src.path(), &dest, "")
			.await
			.expect("Sync failed");

		assert!(stats.bytes_transferred > 0, "Should transfer data");

		eprintln!("Sync completed: {}", stats.format());
	})
	.await;
}

#[tokio::test]
#[ignore = "Requires rclone installed and 'test_remote' configured"]
async fn test_rclone_list() {
	run_with_timeout(async {
		let dest = CloudDestination::generic("test_remote", "kip_test_list");

		let files = Rclone::new().list(&dest, "").await.expect("List failed");

		// Just verify it doesn't error
		eprintln!("Files in remote: {:?}", files);
	})
	.await;
}

#[tokio::test]
#[ignore = "Requires rclone installed and 'test_remote' configured"]
async fn test_rclone_disk_usage() {
	run_with_timeout(async {
		let dest = CloudDestination::generic("test_remote", "");

		let usage = Rclone::new()
			.disk_usage(&dest)
			.await
			.expect("Disk usage check failed");

		eprintln!("Disk usage: {:?}", usage);

		if let (Some(total), Some(used)) = (usage.total_bytes, usage.used_bytes) {
			assert!(used <= total, "Used should not exceed total");
		}
	})
	.await;
}

#[tokio::test]
#[ignore = "Requires rclone installed and 'test_remote' configured"]
async fn test_rclone_dry_run() {
	run_with_timeout(async {
		let src = TempDir::new().unwrap();
		create_test_files(&src).unwrap();

		let dest = CloudDestination::generic("test_remote", "kip_test_dryrun");

		let stats = Rclone::new()
			.with_dry_run()
			.copy(src.path(), &dest, "")
			.await
			.expect("Dry run failed");

		assert!(stats.dry_run, "Should be marked as dry run");

		eprintln!("Dry run completed: {}", stats.format());
	})
	.await;
}

#[tokio::test]
#[ignore = "Requires rclone installed and Google Drive remote configured"]
async fn test_google_drive_destination() {
	run_with_timeout(async {
		let src = TempDir::new().unwrap();
		create_test_files(&src).unwrap();

		let gdrive = CloudDestination::google_drive("gdrive", "kip_test");

		let stats = Rclone::new()
			.copy(src.path(), &gdrive, "")
			.await
			.expect("Google Drive copy failed");

		assert!(stats.bytes_transferred > 0);
		eprintln!("Google Drive backup: {}", stats.format());
	})
	.await;
}

#[tokio::test]
#[ignore = "Requires rclone installed and Nextcloud remote configured"]
async fn test_nextcloud_destination() {
	run_with_timeout(async {
		let src = TempDir::new().unwrap();
		create_test_files(&src).unwrap();

		let nextcloud = CloudDestination::nextcloud("nextcloud", "kip_test");

		let stats = Rclone::new()
			.copy(src.path(), &nextcloud, "")
			.await
			.expect("Nextcloud copy failed");

		assert!(stats.bytes_transferred > 0);
		eprintln!("Nextcloud backup: {}", stats.format());
	})
	.await;
}

#[tokio::test]
#[ignore = "Requires rclone installed and S3 remote configured"]
async fn test_s3_destination() {
	run_with_timeout(async {
		let src = TempDir::new().unwrap();
		create_test_files(&src).unwrap();

		let s3 = CloudDestination::s3("s3", "kip-test-bucket");

		let stats = Rclone::new()
			.copy(src.path(), &s3, "")
			.await
			.expect("S3 copy failed");

		assert!(stats.bytes_transferred > 0);
		eprintln!("S3 backup: {}", stats.format());
	})
	.await;
}
