//! Query API - Read-only operations

use daemon::DbHandle;

use crate::api::{DriveStatus, IntentCounts, KipError, ReviewCounts, StatusSummary, TransferCounts};

/// Get overall system status
pub async fn status(db: &DbHandle) -> Result<StatusSummary, KipError> {
	let (intents, transfers, review, drives) = tokio::try_join!(
		query_intent_counts(db),
		query_transfer_counts(db),
		query_review_counts(db),
		query_drive_status(db),
	)?;

	Ok(StatusSummary { intents, transfers, review_queue: review, drives })
}

/// Get transfer history
pub async fn transfer_history(
	db: &DbHandle,
	_intent_id: Option<&str>,
	_limit: Option<u64>,
) -> Result<Vec<crate::api::TransferSummary>, KipError> {
	// TODO: Implement properly
	Ok(vec![])
}

async fn query_intent_counts(db: &DbHandle) -> Result<IntentCounts, KipError> {
	let mut response = db
		.db
		.query("SELECT count() AS total FROM intent GROUP ALL")
		.await
		.map_err(|e| KipError::Database(e.to_string()))?
		.check()
		.map_err(|e| KipError::Database(e.to_string()))?;

	let rows: Vec<serde_json::Value> = response
		.take(0)
		.map_err(|e| KipError::Database(e.to_string()))?;

	let total = rows.iter().map(|r| r["total"].as_u64().unwrap_or(0)).sum();

	// Query by status separately
	let mut idle = 0u64;
	let mut transferring = 0u64;
	let mut complete = 0u64;
	let mut needs_review = 0u64;

	for status in &["idle", "transferring", "complete", "needs_review"] {
		let mut resp = db
			.db
			.query("SELECT count() AS count FROM intent WHERE status = $status GROUP ALL")
			.bind(("status", status.to_string()))
			.await
			.map_err(|e| KipError::Database(e.to_string()))?
			.check()
			.map_err(|e| KipError::Database(e.to_string()))?;

		let rows: Vec<serde_json::Value> = resp
			.take(0)
			.map_err(|e| KipError::Database(e.to_string()))?;
		let count = rows.iter().map(|r| r["count"].as_u64().unwrap_or(0)).sum();

		match *status {
			"idle" => idle = count,
			"transferring" => transferring = count,
			"complete" => complete = count,
			"needs_review" => needs_review = count,
			_ => {}
		}
	}

	Ok(IntentCounts { total, idle, transferring, complete, needs_review })
}

async fn query_transfer_counts(db: &DbHandle) -> Result<TransferCounts, KipError> {
	let mut pending = 0u64;
	let mut transferring = 0u64;
	let mut complete = 0u64;
	let mut failed = 0u64;
	let mut needs_review = 0u64;

	for status in &["pending", "transferring", "complete", "failed", "needs_review"] {
		let mut resp = db
			.db
			.query("SELECT count() AS count FROM transfer_job WHERE status = $status GROUP ALL")
			.bind(("status", status.to_string()))
			.await
			.map_err(|e| KipError::Database(e.to_string()))?
			.check()
			.map_err(|e| KipError::Database(e.to_string()))?;

		let rows: Vec<serde_json::Value> = resp
			.take(0)
			.map_err(|e| KipError::Database(e.to_string()))?;
		let count = rows.iter().map(|r| r["count"].as_u64().unwrap_or(0)).sum();

		match *status {
			"pending" => pending = count,
			"transferring" => transferring = count,
			"complete" => complete = count,
			"failed" => failed = count,
			"needs_review" => needs_review = count,
			_ => {}
		}
	}

	Ok(TransferCounts {
		pending,
		transferring,
		complete,
		failed,
		needs_review,
	})
}

async fn query_review_counts(db: &DbHandle) -> Result<ReviewCounts, KipError> {
	let mut response = db
		.db
		.query("SELECT count() AS total FROM review_item WHERE resolved_at = NONE GROUP ALL")
		.await
		.map_err(|e| KipError::Database(e.to_string()))?
		.check()
		.map_err(|e| KipError::Database(e.to_string()))?;

	let rows: Vec<serde_json::Value> = response
		.take(0)
		.map_err(|e| KipError::Database(e.to_string()))?;
	let total = rows.iter().map(|r| r["total"].as_u64().unwrap_or(0)).sum();

	Ok(ReviewCounts { total, by_error: std::collections::HashMap::new() })
}

async fn query_drive_status(db: &DbHandle) -> Result<DriveStatus, KipError> {
	let mut response = db
		.db
		.query("SELECT name, mount_point, capacity_bytes, connected FROM drive ORDER BY connected DESC, name")
		.await
		.map_err(|e| KipError::Database(e.to_string()))?
		.check()
		.map_err(|e| KipError::Database(e.to_string()))?;

	let rows: Vec<serde_json::Value> = response
		.take(0)
		.map_err(|e| KipError::Database(e.to_string()))?;
	let mut connected = Vec::new();
	let mut disconnected = Vec::new();

	for row in rows {
		let name = row["name"].as_str().unwrap_or("Unknown").to_string();
		let mount_point = row["mount_point"].as_str().map(|s| s.to_string());
		let capacity_bytes = row["capacity_bytes"].as_u64();
		let available = row["connected"].as_bool().unwrap_or(false);

		let summary = crate::api::DriveSummary {
			id: name.clone(),
			name,
			mount_point,
			capacity_bytes,
			available,
		};

		if available {
			connected.push(summary);
		} else {
			disconnected.push(summary);
		}
	}

	Ok(DriveStatus { connected, disconnected })
}
