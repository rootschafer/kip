use std::{
	fs,
	io::{self, Read, Write},
	path::Path,
};

use surrealdb::types::RecordId;
use thiserror::Error;

use crate::db::DbHandle;

const CHUNK_SIZE: usize = 256 * 1024; // 256KB
const PROGRESS_INTERVAL: usize = 4; // update DB every 4 chunks (~1MB)

#[derive(Debug, Error)]
pub enum CopyError {
	#[error("job not found: {0}")]
	JobNotFound(String),

	#[error("source file not found: {0}")]
	SourceNotFound(String),

	#[error("permission denied: {0}")]
	PermissionDenied(String),

	#[error("disk full: {0}")]
	DiskFull(String),

	#[error("I/O error: {0}")]
	IoError(String),

	#[error("hash mismatch: source={source_hash}, dest={dest_hash}")]
	HashMismatch {
		source_hash: String,
		dest_hash: String,
	},

	#[error("database error: {0}")]
	DbError(String),
}

impl CopyError {
	/// Whether this error is retryable (transient I/O) vs needs immediate review.
	pub fn is_retryable(&self) -> bool {
		matches!(self, CopyError::IoError(_))
	}
}

#[derive(Debug, Clone)]
pub struct CopyResult {
	pub bytes_copied: u64,
	pub source_hash: String,
	pub dest_hash: String,
	pub verified: bool,
}

/// Data we need from a transfer_job record.
struct JobData {
	intent: serde_json::Value,
	source_path: String,
	dest_path: String,
	attempts: i64,
	max_attempts: i64,
}

/// Execute a single transfer job: copy file, hash, verify.
///
/// Handles DB status transitions and error classification.
pub async fn copy_job(db: &DbHandle, job_id: &RecordId) -> Result<CopyResult, CopyError> {
	// 1. Load job data
	let job = load_job(db, job_id).await?;

	// 2. Transition to transferring
	db.db
		.query("UPDATE $id SET status = 'transferring', started_at = time::now()")
		.bind(("id", job_id.clone()))
		.await
		.map_err(|e| CopyError::DbError(e.to_string()))?
		.check()
		.map_err(|e| CopyError::DbError(e.to_string()))?;

	// 3. Run the copy pipeline (blocking I/O on dedicated thread)
	let source = job.source_path.clone();
	let dest = job.dest_path.clone();
	let db_clone = db.clone();
	let job_id_clone = job_id.clone();

	let result = tokio::task::spawn_blocking(move || copy_and_hash(&source, &dest, &db_clone, &job_id_clone))
		.await
		.map_err(|e| CopyError::IoError(format!("task join error: {e}")))?;

	match result {
		Ok(copy_result) => {
			// 4. Mark complete
			db.db
				.query(
					"UPDATE $id SET
                        status = 'complete',
                        source_hash = $source_hash,
                        dest_hash = $dest_hash,
                        bytes_transferred = $bytes,
                        completed_at = time::now()",
				)
				.bind(("id", job_id.clone()))
				.bind(("source_hash", copy_result.source_hash.clone()))
				.bind(("dest_hash", copy_result.dest_hash.clone()))
				.bind(("bytes", copy_result.bytes_copied as i64))
				.await
				.map_err(|e| CopyError::DbError(e.to_string()))?
				.check()
				.map_err(|e| CopyError::DbError(e.to_string()))?;

			Ok(copy_result)
		}
		Err(err) => {
			// 5. Handle error: retryable vs needs_review
			let new_attempts = job.attempts + 1;
			let (new_status, error_kind) = if err.is_retryable() && new_attempts < job.max_attempts {
				("pending", classify_error(&err))
			} else {
				("needs_review", classify_error(&err))
			};

			db.db
				.query(
					"UPDATE $id SET
                        status = $status,
                        attempts = $attempts,
                        last_error = $error,
                        error_kind = $error_kind",
				)
				.bind(("id", job_id.clone()))
				.bind(("status", new_status.to_string()))
				.bind(("attempts", new_attempts))
				.bind(("error", err.to_string()))
				.bind(("error_kind", error_kind.to_string()))
				.await
				.map_err(|e| CopyError::DbError(e.to_string()))?
				.check()
				.map_err(|e| CopyError::DbError(e.to_string()))?;

			// Create review item for non-retryable failures
			if new_status == "needs_review" {
				let options = resolution_options(error_kind);
				let _ = db
					.db
					.query(
						"CREATE review_item CONTENT {
                            job: $job_id,
                            intent: $intent_id,
                            error_kind: $error_kind,
                            error_message: $error_msg,
                            source_path: $source_path,
                            dest_path: $dest_path,
                            options: $options,
                            created_at: time::now(),
                        }",
					)
					.bind(("job_id", job_id.clone()))
					.bind(("intent_id", job.intent.clone()))
					.bind(("error_kind", error_kind.to_string()))
					.bind(("error_msg", err.to_string()))
					.bind(("source_path", job.source_path.clone()))
					.bind(("dest_path", job.dest_path.clone()))
					.bind(("options", options))
					.await;
			}

			Err(err)
		}
	}
}

fn resolution_options(error_kind: &str) -> Vec<String> {
	match error_kind {
		"source_missing" => vec!["skip".into(), "rescan".into()],
		"permission_denied" => vec!["retry".into(), "skip".into()],
		"disk_full" => vec!["retry".into(), "skip".into()],
		"hash_mismatch" => vec!["retry".into(), "skip".into(), "accept".into()],
		"io_error" => vec!["retry".into(), "skip".into()],
		_ => vec!["skip".into()],
	}
}

fn classify_error(err: &CopyError) -> &'static str {
	match err {
		CopyError::SourceNotFound(_) => "source_missing",
		CopyError::PermissionDenied(_) => "permission_denied",
		CopyError::DiskFull(_) => "disk_full",
		CopyError::HashMismatch { .. } => "hash_mismatch",
		CopyError::IoError(_) => "io_error",
		CopyError::JobNotFound(_) | CopyError::DbError(_) => "internal",
	}
}

async fn load_job(db: &DbHandle, job_id: &RecordId) -> Result<JobData, CopyError> {
	let mut response = db
		.db
		.query("SELECT intent, source_path, dest_path, attempts, max_attempts FROM $id")
		.bind(("id", job_id.clone()))
		.await
		.map_err(|e| CopyError::DbError(e.to_string()))?;

	let row: Option<serde_json::Value> = response
		.take(0)
		.map_err(|e| CopyError::DbError(e.to_string()))?;

	let row = row.ok_or_else(|| CopyError::JobNotFound(format!("{:?}", job_id)))?;

	Ok(JobData {
		intent: row["intent"].clone(),
		source_path: row["source_path"].as_str().unwrap_or_default().to_string(),
		dest_path: row["dest_path"].as_str().unwrap_or_default().to_string(),
		attempts: row["attempts"].as_i64().unwrap_or(0),
		max_attempts: row["max_attempts"].as_i64().unwrap_or(3),
	})
}

/// Core copy pipeline: read source → hash → write dest → verify.
/// This is synchronous and should run on spawn_blocking.
fn copy_and_hash(
	source_path: &str,
	dest_path: &str,
	db: &DbHandle,
	job_id: &RecordId,
) -> Result<CopyResult, CopyError> {
	// Create destination parent directories
	if let Some(parent) = Path::new(dest_path).parent() {
		fs::create_dir_all(parent).map_err(|e| map_io_error(e, dest_path))?;
	}

	// Open source
	let mut source = fs::File::open(source_path).map_err(|e| map_io_error(e, source_path))?;

	// Open dest (create/truncate)
	let mut dest = fs::File::create(dest_path).map_err(|e| map_io_error(e, dest_path))?;

	// Single-pass: read → hash → write
	let mut hasher = blake3::Hasher::new();
	let mut buf = vec![0u8; CHUNK_SIZE];
	let mut bytes_copied: u64 = 0;
	let mut chunks_since_progress = 0usize;

	loop {
		let n = source
			.read(&mut buf)
			.map_err(|e| map_io_error(e, source_path))?;
		if n == 0 {
			break;
		}

		hasher.update(&buf[..n]);
		dest.write_all(&buf[..n])
			.map_err(|e| map_io_error(e, dest_path))?;

		bytes_copied += n as u64;
		chunks_since_progress += 1;

		if chunks_since_progress >= PROGRESS_INTERVAL {
			chunks_since_progress = 0;
			update_progress(db, job_id, bytes_copied);
		}
	}

	dest.flush().map_err(|e| map_io_error(e, dest_path))?;
	drop(dest);

	let source_hash = hasher.finalize().to_hex().to_string();

	// Verify: re-read dest, compute hash
	let dest_hash = hash_file(dest_path)?;

	let verified = source_hash == dest_hash;
	if !verified {
		return Err(CopyError::HashMismatch { source_hash, dest_hash });
	}

	Ok(CopyResult { bytes_copied, source_hash, dest_hash, verified })
}

/// Hash a file using blake3 in 256KB chunks.
pub fn hash_file(path: &str) -> Result<String, CopyError> {
	let mut file = fs::File::open(path).map_err(|e| map_io_error(e, path))?;
	let mut hasher = blake3::Hasher::new();
	let mut buf = vec![0u8; CHUNK_SIZE];

	loop {
		let n = file.read(&mut buf).map_err(|e| map_io_error(e, path))?;
		if n == 0 {
			break;
		}
		hasher.update(&buf[..n]);
	}

	Ok(hasher.finalize().to_hex().to_string())
}

fn map_io_error(err: io::Error, path: &str) -> CopyError {
	match err.kind() {
		io::ErrorKind::NotFound => CopyError::SourceNotFound(path.to_string()),
		io::ErrorKind::PermissionDenied => CopyError::PermissionDenied(path.to_string()),
		io::ErrorKind::StorageFull => CopyError::DiskFull(path.to_string()),
		_ => CopyError::IoError(format!("{path}: {err}")),
	}
}

/// Fire-and-forget progress update. Errors are silently ignored
/// (progress is best-effort, not critical).
fn update_progress(db: &DbHandle, job_id: &RecordId, bytes: u64) {
	let db = db.clone();
	let job_id = job_id.clone();
	tokio::task::block_in_place(move || {
		tokio::runtime::Handle::current().block_on(async {
			let _ = db
				.db
				.query("UPDATE $id SET bytes_transferred = $bytes")
				.bind(("id", job_id))
				.bind(("bytes", bytes as i64))
				.await;
		});
	});
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn copy_and_verify_small_file() {
		let tmp = tempfile::tempdir().unwrap();
		let src = tmp.path().join("source.txt");
		let dst = tmp.path().join("dest.txt");
		fs::write(&src, "hello world").unwrap();

		// Copy manually using the same pipeline as copy_and_hash
		let mut source = fs::File::open(&src).unwrap();
		let mut dest = fs::File::create(&dst).unwrap();
		let mut hasher = blake3::Hasher::new();
		let mut buf = vec![0u8; CHUNK_SIZE];
		loop {
			let n = source.read(&mut buf).unwrap();
			if n == 0 {
				break;
			}
			hasher.update(&buf[..n]);
			dest.write_all(&buf[..n]).unwrap();
		}
		drop(dest);

		let source_hash = hasher.finalize().to_hex().to_string();
		let dest_hash = hash_file(dst.to_str().unwrap()).unwrap();

		assert_eq!(source_hash, dest_hash);
		assert_eq!(fs::read_to_string(&dst).unwrap(), "hello world");
	}

	#[test]
	fn hash_empty_file() {
		let tmp = tempfile::tempdir().unwrap();
		let f = tmp.path().join("empty.txt");
		fs::write(&f, "").unwrap();

		let hash = hash_file(f.to_str().unwrap()).unwrap();
		assert_eq!(hash.len(), 64); // blake3 hex
	}

	#[test]
	fn hash_multichunk_file() {
		let tmp = tempfile::tempdir().unwrap();
		let f = tmp.path().join("big.bin");
		let data = vec![42u8; CHUNK_SIZE * 3 + 1000];
		fs::write(&f, &data).unwrap();

		let hash = hash_file(f.to_str().unwrap()).unwrap();

		// Must match blake3 computed in one shot
		let expected = blake3::hash(&data).to_hex().to_string();
		assert_eq!(hash, expected);
	}

	#[test]
	fn hash_file_not_found() {
		let err = hash_file("/tmp/kip_definitely_not_real.txt").unwrap_err();
		assert!(matches!(err, CopyError::SourceNotFound(_)));
	}

	#[test]
	fn error_classification() {
		assert!(CopyError::IoError("tmp".into()).is_retryable());
		assert!(!CopyError::SourceNotFound("x".into()).is_retryable());
		assert!(!CopyError::PermissionDenied("x".into()).is_retryable());
		assert!(!CopyError::DiskFull("x".into()).is_retryable());
		assert!(!CopyError::HashMismatch { source_hash: "a".into(), dest_hash: "b".into() }.is_retryable());
	}
}
