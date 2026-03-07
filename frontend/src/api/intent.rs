//! Intent management API

use chrono::Utc;
use daemon::DbHandle;

use crate::api::{KipError, LocationId, RunResult};

/// Create a new intent
pub async fn create_intent(
	db: &DbHandle,
	source: LocationId,
	destinations: Vec<LocationId>,
	config: crate::api::IntentConfig,
) -> Result<LocationId, KipError> {
	let intent_id = format!("intent:{}", ulid::Ulid::new());

	db.db
        .query("CREATE intent CONTENT { source: $source, destinations: $destinations, status: 'idle', kind: 'backup', speed_mode: 'fast', priority: $priority, name: $name }")
        .bind(("source", source))
        .bind(("destinations", destinations))
        .bind(("priority", config.priority as i64))
        .bind(("name", config.name))
        .await
        .map_err(|e| KipError::Database(e.to_string()))?
        .check()
        .map_err(|e| KipError::Database(e.to_string()))?;

	Ok(intent_id)
}

/// Delete an intent
pub async fn delete_intent(db: &DbHandle, intent_id: &str) -> Result<(), KipError> {
	let intent_id = intent_id.to_string();

	db.db
		.query("DELETE FROM transfer_job WHERE intent = $intent")
		.bind(("intent", intent_id.clone()))
		.await
		.map_err(|e| KipError::Database(e.to_string()))?
		.check()
		.map_err(|e| KipError::Database(e.to_string()))?;

	db.db
		.query("DELETE FROM intent WHERE id = $intent")
		.bind(("intent", intent_id))
		.await
		.map_err(|e| KipError::Database(e.to_string()))?
		.check()
		.map_err(|e| KipError::Database(e.to_string()))?;

	Ok(())
}

/// List all intents
pub async fn list_intents(db: &DbHandle) -> Result<Vec<crate::api::IntentSummary>, KipError> {
	let mut response = db
		.db
		.query("SELECT * FROM intent ORDER BY created_at")
		.await
		.map_err(|e| KipError::Database(e.to_string()))?
		.check()
		.map_err(|e| KipError::Database(e.to_string()))?;

	let rows: Vec<serde_json::Value> = response
		.take(0)
		.map_err(|e| KipError::Database(e.to_string()))?;
	let mut intents = Vec::new();

	for row in rows {
		let id = row["id"].as_str().unwrap_or("").to_string();
		if id.is_empty() {
			continue;
		}

		let name = row["name"].as_str().map(|s| s.to_string());
		let status_str = row["status"].as_str().unwrap_or("idle");
		let kind_str = row["kind"].as_str().unwrap_or("backup");

		let status = match status_str {
			"scanning" => crate::api::IntentStatus::Scanning,
			"transferring" => crate::api::IntentStatus::Transferring,
			"complete" => crate::api::IntentStatus::Complete,
			"needs_review" => crate::api::IntentStatus::NeedsReview,
			"error" => crate::api::IntentStatus::Error,
			_ => crate::api::IntentStatus::Idle,
		};

		let kind = match kind_str {
			"sync" => crate::api::IntentKind::Sync,
			"archive" => crate::api::IntentKind::Archive,
			_ => crate::api::IntentKind::Backup,
		};

		let created_at = row["created_at"]
			.as_str()
			.and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
			.map(|dt| dt.with_timezone(&Utc))
			.unwrap_or_else(Utc::now);

		let updated_at = row["updated_at"]
			.as_str()
			.and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
			.map(|dt| dt.with_timezone(&Utc))
			.unwrap_or_else(Utc::now);

		let progress = crate::api::IntentProgress {
			total_files: row["total_files"].as_u64().unwrap_or(0),
			total_bytes: row["total_bytes"].as_u64().unwrap_or(0),
			completed_files: row["completed_files"].as_u64().unwrap_or(0),
			completed_bytes: row["completed_bytes"].as_u64().unwrap_or(0),
		};

		let source = crate::api::LocationSummary {
			id: "unknown".to_string(),
			path: "Unknown".to_string(),
			label: None,
			machine: crate::api::MachineSummary {
				id: "local".to_string(),
				name: "Local".to_string(),
				kind: crate::api::MachineKind::Local,
				online: true,
			},
			available: true,
		};

		intents.push(crate::api::IntentSummary {
			id,
			name,
			source: source.clone(),
			destinations: vec![source],
			status,
			kind,
			created_at,
			updated_at,
			progress,
		});
	}

	Ok(intents)
}

/// Get intent details
pub async fn get_intent(db: &DbHandle, intent_id: &str) -> Result<crate::api::IntentDetail, KipError> {
	let mut response = db
		.db
		.query("SELECT * FROM intent WHERE id = $id")
		.bind(("id", intent_id.to_string()))
		.await
		.map_err(|e| KipError::Database(e.to_string()))?
		.check()
		.map_err(|e| KipError::Database(e.to_string()))?;

	let row: Option<serde_json::Value> = response
		.take(0)
		.map_err(|e| KipError::Database(e.to_string()))?;
	let row = row.ok_or_else(|| KipError::IntentNotFound(intent_id.to_string()))?;

	let summary = parse_intent_summary(&row)?;

	Ok(crate::api::IntentDetail {
		summary,
		config: crate::api::IntentConfig::default(),
		recent_transfers: vec![],
	})
}

/// Run an intent
pub async fn run_intent(
	db: &DbHandle,
	intent_id: &str,
	_progress: Option<crate::api::ProgressCallback>,
) -> Result<RunResult, KipError> {
	use daemon::engine::scheduler;

	let record_id = surrealdb::types::RecordId::new("intent", intent_id);

	let status = get_intent_status(db, &record_id).await?;

	if status == crate::api::IntentStatus::Idle {
		scan_intent(db, intent_id).await?;
	}

	let result = scheduler::run_intent(db, &record_id)
		.await
		.map_err(|e| KipError::Database(e.to_string()))?;

	Ok(RunResult {
		completed: result.completed,
		failed: result.failed,
		needs_review: result.needs_review,
		bytes_transferred: 0,
		duration: std::time::Duration::from_secs(0),
	})
}

/// Cancel an intent
pub async fn cancel_intent(db: &DbHandle, intent_id: &str) -> Result<(), KipError> {
	db.db
		.query("UPDATE intent SET status = 'error', updated_at = time::now() WHERE id = $id")
		.bind(("id", intent_id.to_string()))
		.await
		.map_err(|e| KipError::Database(e.to_string()))?
		.check()
		.map_err(|e| KipError::Database(e.to_string()))?;

	db.db
		.query(
			"UPDATE transfer_job SET status = 'cancelled' WHERE intent = $id AND status IN ['pending', 'transferring']",
		)
		.bind(("id", intent_id.to_string()))
		.await
		.map_err(|e| KipError::Database(e.to_string()))?
		.check()
		.map_err(|e| KipError::Database(e.to_string()))?;

	Ok(())
}

/// Scan an intent's source
pub async fn scan_intent(db: &DbHandle, intent_id: &str) -> Result<crate::api::ScanResult, KipError> {
	use daemon::engine::scanner;

	let record_id = surrealdb::types::RecordId::new("intent", intent_id);

	let result = scanner::scan_intent(db, &record_id)
		.await
		.map_err(|e| KipError::Database(e.to_string()))?;

	Ok(crate::api::ScanResult {
		files_found: result.files_found,
		total_bytes: result.total_bytes,
		jobs_created: result.jobs_created,
		skipped_entries: result.skipped_entries,
	})
}

/// Retry failed transfers
pub async fn retry_failed(db: &DbHandle, intent_id: &str) -> Result<RunResult, KipError> {
	db.db
        .query("UPDATE transfer_job SET status = 'pending', error_kind = NONE WHERE intent = $id AND status IN ['failed', 'needs_review']")
        .bind(("id", intent_id.to_string()))
        .await
        .map_err(|e| KipError::Database(e.to_string()))?
        .check()
        .map_err(|e| KipError::Database(e.to_string()))?;

	run_intent(db, intent_id, None).await
}

async fn get_intent_status(
	db: &DbHandle,
	record_id: &surrealdb::types::RecordId,
) -> Result<crate::api::IntentStatus, KipError> {
	let mut response = db
		.db
		.query("SELECT status FROM intent WHERE id = $id")
		.bind(("id", format!("{:?}", record_id)))
		.await
		.map_err(|e| KipError::Database(e.to_string()))?
		.check()
		.map_err(|e| KipError::Database(e.to_string()))?;

	let row: Option<serde_json::Value> = response
		.take(0)
		.map_err(|e| KipError::Database(e.to_string()))?;
	let status_str = row
		.and_then(|r| r["status"].as_str().map(|s| s.to_string()))
		.unwrap_or_else(|| "idle".to_string());

	Ok(match status_str.as_str() {
		"scanning" => crate::api::IntentStatus::Scanning,
		"transferring" => crate::api::IntentStatus::Transferring,
		"complete" => crate::api::IntentStatus::Complete,
		"needs_review" => crate::api::IntentStatus::NeedsReview,
		"error" => crate::api::IntentStatus::Error,
		_ => crate::api::IntentStatus::Idle,
	})
}

fn parse_intent_summary(row: &serde_json::Value) -> Result<crate::api::IntentSummary, KipError> {
	let id = row["id"].as_str().unwrap_or("").to_string();
	if id.is_empty() {
		return Err(KipError::IntentNotFound("empty id".to_string()));
	}

	let name = row["name"].as_str().map(|s| s.to_string());
	let status_str = row["status"].as_str().unwrap_or("idle");
	let kind_str = row["kind"].as_str().unwrap_or("backup");

	let status = match status_str {
		"scanning" => crate::api::IntentStatus::Scanning,
		"transferring" => crate::api::IntentStatus::Transferring,
		"complete" => crate::api::IntentStatus::Complete,
		"needs_review" => crate::api::IntentStatus::NeedsReview,
		"error" => crate::api::IntentStatus::Error,
		_ => crate::api::IntentStatus::Idle,
	};

	let kind = match kind_str {
		"sync" => crate::api::IntentKind::Sync,
		"archive" => crate::api::IntentKind::Archive,
		_ => crate::api::IntentKind::Backup,
	};

	let created_at = row["created_at"]
		.as_str()
		.and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
		.map(|dt| dt.with_timezone(&Utc))
		.unwrap_or_else(Utc::now);

	let updated_at = row["updated_at"]
		.as_str()
		.and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
		.map(|dt| dt.with_timezone(&Utc))
		.unwrap_or_else(Utc::now);

	let progress = crate::api::IntentProgress {
		total_files: row["total_files"].as_u64().unwrap_or(0),
		total_bytes: row["total_bytes"].as_u64().unwrap_or(0),
		completed_files: row["completed_files"].as_u64().unwrap_or(0),
		completed_bytes: row["completed_bytes"].as_u64().unwrap_or(0),
	};

	let source = crate::api::LocationSummary {
		id: "unknown".to_string(),
		path: "Unknown".to_string(),
		label: None,
		machine: crate::api::MachineSummary {
			id: "local".to_string(),
			name: "Local".to_string(),
			kind: crate::api::MachineKind::Local,
			online: true,
		},
		available: true,
	};

	Ok(crate::api::IntentSummary {
		id,
		name,
		source: source.clone(),
		destinations: vec![source.clone()],
		status,
		kind,
		created_at,
		updated_at,
		progress,
	})
}
