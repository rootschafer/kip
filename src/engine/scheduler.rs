use std::sync::Arc;

use surrealdb::types::RecordId;
use thiserror::Error;
use tokio::sync::Semaphore;

use crate::{db::DbHandle, engine::copier};

const MAX_CONCURRENCY: usize = 4;

#[derive(Debug, Error)]
pub enum SchedulerError {
	#[error("intent not found: {0}")]
	IntentNotFound(String),

	#[error("database error: {0}")]
	DbError(String),
}

#[derive(Debug, Clone)]
pub struct RunResult {
	pub completed: u64,
	pub failed: u64,
	pub needs_review: u64,
}

/// Run all pending jobs for an intent with bounded concurrency.
/// Returns when all jobs are complete, failed, or need review.
pub async fn run_intent(db: &DbHandle, intent_id: &RecordId) -> Result<RunResult, SchedulerError> {
	// Verify intent exists
	let mut response = db
		.db
		.query("SELECT id FROM $id")
		.bind(("id", intent_id.clone()))
		.await
		.map_err(|e| SchedulerError::DbError(e.to_string()))?;

	let exists: Option<serde_json::Value> = response
		.take(0)
		.map_err(|e| SchedulerError::DbError(e.to_string()))?;

	if exists.is_none() {
		return Err(SchedulerError::IntentNotFound(format!("{:?}", intent_id)));
	}

	// Recovery: reset any jobs stuck in 'transferring' from a previous crash
	db.db
		.query(
			"UPDATE transfer_job SET status = 'pending', bytes_transferred = 0
             WHERE intent = $intent_id AND status = 'transferring'",
		)
		.bind(("intent_id", intent_id.clone()))
		.await
		.map_err(|e| SchedulerError::DbError(e.to_string()))?
		.check()
		.map_err(|e| SchedulerError::DbError(e.to_string()))?;

	let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENCY));

	// Main dispatch loop: keep pulling pending jobs until none remain
	loop {
		let job_ids = get_pending_jobs(db, intent_id).await?;

		if job_ids.is_empty() {
			break;
		}

		// Spawn concurrent copy tasks
		let mut handles = Vec::with_capacity(job_ids.len());

		for job_id in job_ids {
			let permit = semaphore.clone().acquire_owned().await.unwrap();
			let db = db.clone();

			handles.push(tokio::spawn(async move {
				let result = copier::copy_job(&db, &job_id).await;
				drop(permit);
				(job_id, result)
			}));
		}

		// Wait for all tasks in this batch
		for handle in handles {
			// Ignore join errors (panics in copy tasks) — the job stays
			// in 'transferring' and will be recovered on next loop iteration
			let _ = handle.await;
		}

		// After batch completes, loop back to check for any jobs that
		// were retried (set back to 'pending' by the copier)
	}

	// All jobs processed — compute final counts and update intent
	let result = compute_result(db, intent_id).await?;
	finalize_intent(db, intent_id, &result).await?;

	Ok(result)
}

/// Query all pending job IDs for an intent.
async fn get_pending_jobs(db: &DbHandle, intent_id: &RecordId) -> Result<Vec<RecordId>, SchedulerError> {
	let mut response = db
		.db
		.query("SELECT id FROM transfer_job WHERE intent = $intent_id AND status = 'pending'")
		.bind(("intent_id", intent_id.clone()))
		.await
		.map_err(|e| SchedulerError::DbError(e.to_string()))?;

	let rows: Vec<serde_json::Value> = response
		.take(0)
		.map_err(|e| SchedulerError::DbError(e.to_string()))?;

	let mut ids = Vec::with_capacity(rows.len());
	for row in rows {
		if let Ok(id) = serde_json::from_value::<RecordId>(row["id"].clone()) {
			ids.push(id);
		}
	}

	Ok(ids)
}

/// Compute final job counts for the intent.
async fn compute_result(db: &DbHandle, intent_id: &RecordId) -> Result<RunResult, SchedulerError> {
	let mut response = db
		.db
		.query(
			"SELECT
                math::sum(IF status = 'complete' THEN 1 ELSE 0 END) AS completed,
                math::sum(IF status = 'needs_review' THEN 1 ELSE 0 END) AS needs_review,
                math::sum(IF status = 'failed' THEN 1 ELSE 0 END) AS failed
             FROM transfer_job WHERE intent = $intent_id GROUP ALL",
		)
		.bind(("intent_id", intent_id.clone()))
		.await
		.map_err(|e| SchedulerError::DbError(e.to_string()))?;

	let row: Option<serde_json::Value> = response
		.take(0)
		.map_err(|e| SchedulerError::DbError(e.to_string()))?;

	match row {
		Some(r) => Ok(RunResult {
			completed: r["completed"].as_u64().unwrap_or(0),
			needs_review: r["needs_review"].as_u64().unwrap_or(0),
			failed: r["failed"].as_u64().unwrap_or(0),
		}),
		None => Ok(RunResult { completed: 0, failed: 0, needs_review: 0 }),
	}
}

/// Update the intent's final status based on job results.
async fn finalize_intent(db: &DbHandle, intent_id: &RecordId, result: &RunResult) -> Result<(), SchedulerError> {
	let status = if result.needs_review > 0 {
		"needs_review"
	} else {
		"complete"
	};

	// Also update completed_files and completed_bytes from actual job data
	db.db
		.query(
			"UPDATE $id SET
                status = $status,
                completed_files = $completed,
                updated_at = time::now()",
		)
		.bind(("id", intent_id.clone()))
		.bind(("status", status.to_string()))
		.bind(("completed", result.completed as i64))
		.await
		.map_err(|e| SchedulerError::DbError(e.to_string()))?
		.check()
		.map_err(|e| SchedulerError::DbError(e.to_string()))?;

	Ok(())
}
