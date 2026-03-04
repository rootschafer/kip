use std::{path::Path, time::SystemTime};

use surrealdb::types::RecordId;
use thiserror::Error;
use walkdir::WalkDir;

use crate::db::DbHandle;

#[derive(Debug, Error)]
pub enum ScanError {
	#[error("intent not found: {0}")]
	IntentNotFound(String),

	#[error("source location not found: {0}")]
	SourceLocationNotFound(String),

	#[error("destination location not found: {0}")]
	DestLocationNotFound(String),

	#[error("source path does not exist: {0}")]
	SourcePathNotExists(String),

	#[error("source path is not a directory: {0}")]
	SourcePathNotDir(String),

	#[error("filesystem walk error: {0}")]
	WalkError(#[from] walkdir::Error),

	#[error("database error: {0}")]
	DbError(String),
}

#[derive(Debug, Clone)]
pub struct ScanResult {
	pub files_found: u64,
	pub total_bytes: u64,
	pub jobs_created: u64,
	pub skipped_entries: u64,
}

#[derive(Debug)]
struct FileEntry {
	relative_path: String,
	size: u64,
	#[allow(dead_code)]
	modified: SystemTime,
}

/// Loaded intent fields needed for scanning.
struct IntentData {
	source: RecordId,
	destinations: Vec<RecordId>,
}

/// Scan an intent's source, create transfer_jobs for all destinations.
///
/// State transitions: idle → scanning → transferring (or complete if empty).
pub async fn scan_intent(db: &DbHandle, intent_id: &RecordId) -> Result<ScanResult, ScanError> {
	// 1. Load intent fields we need
	let intent = load_intent(db, intent_id).await?;

	// 2. Transition to scanning
	db.db
		.query("UPDATE $id SET status = 'scanning', updated_at = time::now()")
		.bind(("id", intent_id.clone()))
		.await
		.map_err(|e| ScanError::DbError(e.to_string()))?
		.check()
		.map_err(|e| ScanError::DbError(e.to_string()))?;

	// 3. Resolve source path
	let source_path = resolve_location_path(db, &intent.source, true).await?;

	// 4. Walk filesystem (blocking — offload to thread pool)
	let (entries, skipped) = tokio::task::spawn_blocking({
		let source_path = source_path.clone();
		move || walk_source(&source_path)
	})
	.await
	.map_err(|e| ScanError::DbError(format!("task join error: {e}")))??;

	// 5. Resolve destination paths
	let mut destinations = Vec::with_capacity(intent.destinations.len());
	for dest_id in &intent.destinations {
		let dest_path = resolve_location_path(db, dest_id, false).await?;
		destinations.push((dest_id.clone(), dest_path));
	}

	// 6. Create transfer jobs
	let jobs_created = create_transfer_jobs(db, intent_id, &source_path, &entries, &destinations).await?;

	// 7. Update intent totals and transition
	let total_bytes: u64 = entries.iter().map(|e| e.size).sum();
	let total_jobs = entries.len() as u64 * destinations.len() as u64;
	let next_status = if total_jobs == 0 {
		"complete"
	} else {
		"transferring"
	};

	db.db
		.query(
			"UPDATE $id SET
                status = $status,
                total_files = $total_files,
                total_bytes = $total_bytes,
                updated_at = time::now()",
		)
		.bind(("id", intent_id.clone()))
		.bind(("status", next_status.to_string()))
		.bind(("total_files", total_jobs as i64))
		.bind(("total_bytes", total_bytes as i64 * destinations.len() as i64))
		.await
		.map_err(|e| ScanError::DbError(e.to_string()))?
		.check()
		.map_err(|e| ScanError::DbError(e.to_string()))?;

	Ok(ScanResult {
		files_found: entries.len() as u64,
		total_bytes,
		jobs_created,
		skipped_entries: skipped,
	})
}

/// Load the intent fields needed for scanning via raw query + JSON.
async fn load_intent(db: &DbHandle, intent_id: &RecordId) -> Result<IntentData, ScanError> {
	let mut response = db
		.db
		.query("SELECT source, destinations FROM $id")
		.bind(("id", intent_id.clone()))
		.await
		.map_err(|e| ScanError::DbError(e.to_string()))?;

	let row: Option<serde_json::Value> = response
		.take(0)
		.map_err(|e| ScanError::DbError(e.to_string()))?;

	let row = row.ok_or_else(|| ScanError::IntentNotFound(format!("{:?}", intent_id)))?;

	let source: RecordId = serde_json::from_value(row["source"].clone())
		.map_err(|e| ScanError::DbError(format!("failed to parse intent.source: {e}")))?;

	let destinations: Vec<RecordId> = serde_json::from_value(row["destinations"].clone())
		.map_err(|e| ScanError::DbError(format!("failed to parse intent.destinations: {e}")))?;

	Ok(IntentData { source, destinations })
}

/// Resolve a location record ID to its absolute filesystem path.
async fn resolve_location_path(db: &DbHandle, location_id: &RecordId, is_source: bool) -> Result<String, ScanError> {
	let mut response = db
		.db
		.query("SELECT path FROM $id")
		.bind(("id", location_id.clone()))
		.await
		.map_err(|e| ScanError::DbError(e.to_string()))?;

	let path: Option<String> = response
		.take("path")
		.map_err(|e| ScanError::DbError(e.to_string()))?;

	path.ok_or_else(|| {
		let id_str = format!("{:?}", location_id);
		if is_source {
			ScanError::SourceLocationNotFound(id_str)
		} else {
			ScanError::DestLocationNotFound(id_str)
		}
	})
}

fn walk_source(source_path: &str) -> Result<(Vec<FileEntry>, u64), ScanError> {
	let root = Path::new(source_path);

	if !root.exists() {
		return Err(ScanError::SourcePathNotExists(source_path.to_string()));
	}
	if !root.is_dir() {
		return Err(ScanError::SourcePathNotDir(source_path.to_string()));
	}

	let mut entries = Vec::new();
	let mut skipped = 0u64;

	for result in WalkDir::new(root).follow_links(false) {
		let entry = match result {
			Ok(e) => e,
			Err(_) => {
				skipped += 1;
				continue;
			}
		};

		if entry.file_type().is_dir() || entry.file_type().is_symlink() {
			if entry.file_type().is_symlink() {
				skipped += 1;
			}
			continue;
		}

		let metadata = match entry.metadata() {
			Ok(m) => m,
			Err(_) => {
				skipped += 1;
				continue;
			}
		};

		let relative = entry
			.path()
			.strip_prefix(root)
			.expect("walkdir entry must be under root")
			.to_string_lossy()
			.to_string();

		entries.push(FileEntry {
			relative_path: relative,
			size: metadata.len(),
			modified: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
		});
	}

	Ok((entries, skipped))
}

async fn create_transfer_jobs(
	db: &DbHandle,
	intent_id: &RecordId,
	source_base_path: &str,
	entries: &[FileEntry],
	destinations: &[(RecordId, String)],
) -> Result<u64, ScanError> {
	let mut jobs_created = 0u64;
	let source_base = source_base_path.trim_end_matches('/');

	for (dest_id, dest_base_path) in destinations {
		let dest_base = dest_base_path.trim_end_matches('/');

		for entry in entries {
			let source_full = format!("{source_base}/{}", entry.relative_path);
			let dest_full = format!("{dest_base}/{}", entry.relative_path);

			db.db
				.query(
					"CREATE transfer_job CONTENT {
                        intent: $intent_id,
                        source_path: $source_path,
                        dest_path: $dest_path,
                        destination: $dest_id,
                        size: $size,
                        bytes_transferred: 0,
                        status: 'pending',
                        attempts: 0,
                        max_attempts: 3,
                        last_error: NONE,
                        error_kind: NONE,
                        source_hash: NONE,
                        dest_hash: NONE,
                        started_at: NONE,
                        completed_at: NONE,
                        created_at: time::now(),
                    }",
				)
				.bind(("intent_id", intent_id.clone()))
				.bind(("source_path", source_full))
				.bind(("dest_path", dest_full))
				.bind(("dest_id", dest_id.clone()))
				.bind(("size", entry.size as i64))
				.await
				.map_err(|e| ScanError::DbError(e.to_string()))?
				.check()
				.map_err(|e| ScanError::DbError(e.to_string()))?;

			jobs_created += 1;
		}
	}

	Ok(jobs_created)
}

#[cfg(test)]
mod tests {
	use std::fs;

	use super::*;

	fn setup_tree(dir: &Path) {
		fs::create_dir_all(dir.join("subdir/deep")).unwrap();
		fs::write(dir.join("root.txt"), "hello").unwrap();
		fs::write(dir.join("subdir/mid.txt"), "ab").unwrap();
		fs::write(dir.join("subdir/deep/bottom.txt"), "abcdefghij").unwrap();
	}

	#[test]
	fn walks_nested_dirs() {
		let tmp = tempfile::tempdir().unwrap();
		setup_tree(tmp.path());

		let (entries, skipped) = walk_source(tmp.path().to_str().unwrap()).unwrap();

		assert_eq!(skipped, 0);
		assert_eq!(entries.len(), 3);

		let mut paths: Vec<&str> = entries.iter().map(|e| e.relative_path.as_str()).collect();
		paths.sort();
		assert_eq!(paths, vec!["root.txt", "subdir/deep/bottom.txt", "subdir/mid.txt"]);
	}

	#[test]
	fn reports_correct_sizes() {
		let tmp = tempfile::tempdir().unwrap();
		setup_tree(tmp.path());

		let (entries, _) = walk_source(tmp.path().to_str().unwrap()).unwrap();

		let total: u64 = entries.iter().map(|e| e.size).sum();
		// "hello" (5) + "ab" (2) + "abcdefghij" (10)
		assert_eq!(total, 17);
	}

	#[test]
	fn skips_symlinks() {
		let tmp = tempfile::tempdir().unwrap();
		setup_tree(tmp.path());
		std::os::unix::fs::symlink(tmp.path().join("root.txt"), tmp.path().join("link.txt")).unwrap();

		let (entries, skipped) = walk_source(tmp.path().to_str().unwrap()).unwrap();

		assert_eq!(entries.len(), 3); // symlink not counted as a file
		assert_eq!(skipped, 1);
	}

	#[test]
	fn empty_dir_returns_zero() {
		let tmp = tempfile::tempdir().unwrap();

		let (entries, skipped) = walk_source(tmp.path().to_str().unwrap()).unwrap();

		assert_eq!(entries.len(), 0);
		assert_eq!(skipped, 0);
	}

	#[test]
	fn nonexistent_path_errors() {
		let err = walk_source("/tmp/kip_definitely_not_real").unwrap_err();
		assert!(matches!(err, ScanError::SourcePathNotExists(_)));
	}

	#[test]
	fn file_not_dir_errors() {
		let tmp = tempfile::tempdir().unwrap();
		let file = tmp.path().join("afile.txt");
		fs::write(&file, "x").unwrap();

		let err = walk_source(file.to_str().unwrap()).unwrap_err();
		assert!(matches!(err, ScanError::SourcePathNotDir(_)));
	}
}
