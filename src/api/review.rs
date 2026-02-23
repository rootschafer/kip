//! Review queue API

use crate::api::{KipError, Resolution, ReviewId, ReviewItem};
use crate::db::DbHandle;
use surrealdb::types::RecordId;

/// List all review items
pub async fn list_review_items(db: &DbHandle) -> Result<Vec<ReviewItem>, KipError> {
    let mut response = db
        .db
        .query("SELECT * FROM review_item WHERE resolved_at = NONE ORDER BY created_at")
        .await.map_err(|e| KipError::Database(e.to_string()))?
        .check().map_err(|e| KipError::Database(e.to_string()))?;

    let rows: Vec<serde_json::Value> = response.take(0)?;
    let mut items = Vec::new();

    for row in rows {
        if let Some(item) = parse_review_item(&row) {
            items.push(item);
        }
    }

    Ok(items)
}

/// Resolve a review item
pub async fn resolve_review(
    db: &DbHandle,
    review_id: &str,
    resolution: Resolution,
) -> Result<(), KipError> {
    let record_id = RecordId::new("review_item", review_id);

    // Load review item to get the transfer job
    let mut response = db
        .db
        .query("SELECT job, intent FROM review_item WHERE id = $id")
        .bind(("id", record_id.clone()))
        .await.map_err(|e| KipError::Database(e.to_string()))?
        .check().map_err(|e| KipError::Database(e.to_string()))?;

    let row: Option<serde_json::Value> = response.take(0)?;
    let (job_id, intent_id) = match row {
        Some(r) => {
            let job = r["job"].as_str().unwrap_or("").to_string();
            let intent = r["intent"].as_str().unwrap_or("").to_string();
            (job, intent)
        }
        None => return Err(KipError::ReviewNotFound(review_id.to_string())),
    };

    // Apply resolution
    match resolution {
        Resolution::Retry => {
            // Reset job to pending
            let job_record = RecordId::new("transfer_job", job_id);
            db.db
                .query("UPDATE $job SET status = 'pending', error_kind = NONE")
                .bind(("job", job_record))
                .await.map_err(|e| KipError::Database(e.to_string()))?
                .check().map_err(|e| KipError::Database(e.to_string()))?;
        }
        Resolution::Skip => {
            // Mark job as skipped
            let job_record = RecordId::new("transfer_job", job_id);
            db.db
                .query("UPDATE $job SET status = 'skipped'")
                .bind(("job", job_record))
                .await.map_err(|e| KipError::Database(e.to_string()))?
                .check().map_err(|e| KipError::Database(e.to_string()))?;
        }
        Resolution::AbortIntent => {
            // Cancel entire intent
            crate::api::intent::cancel_intent(db, &intent_id).await?;
        }
        // Other resolutions would need more complex handling
        _ => {
            return Err(KipError::InvalidIntentConfig(format!(
                "Resolution {:?} not fully implemented",
                resolution
            )));
        }
    }

    // Mark review item as resolved
    db.db
        .query("UPDATE $id SET resolution = $resolution, resolved_at = time::now()")
        .bind(("id", record_id))
        .bind(("resolution", format!("{:?}", resolution)))
        .await.map_err(|e| KipError::Database(e.to_string()))?
        .check().map_err(|e| KipError::Database(e.to_string()))?;

    Ok(())
}

/// Resolve all review items for an intent
pub async fn resolve_all_review(
    db: &DbHandle,
    intent_id: &str,
    resolution: Resolution,
) -> Result<u64, KipError> {
    // Get all review items for this intent
    let mut response = db
        .db
        .query("SELECT id FROM review_item WHERE intent = $intent AND resolved_at = NONE")
        .bind(("intent", RecordId::new("intent", intent_id)))
        .await.map_err(|e| KipError::Database(e.to_string()))?
        .check().map_err(|e| KipError::Database(e.to_string()))?;

    let rows: Vec<serde_json::Value> = response.take(0)?;
    let count = rows.len() as u64;

    // Resolve each one
    for row in rows {
        if let Some(id_val) = row["id"].as_str() {
            let review_id = id_val.to_string();
            // Extract just the ID part after the table name
            let review_id = review_id.split(':').last().unwrap_or(&review_id);
            resolve_review(db, review_id, resolution.clone()).await?;
        }
    }

    Ok(count)
}

fn parse_review_item(row: &serde_json::Value) -> Option<ReviewItem> {
    let id = row["id"].as_str().unwrap_or("").to_string();
    if id.is_empty() {
        return None;
    }

    let intent = row["intent"].as_str().unwrap_or("").to_string();
    let transfer = row["job"].as_str().unwrap_or("").to_string();
    let error_kind_str = row["error_kind"].as_str().unwrap_or("IoError");
    let error_message = row["error_message"].as_str().unwrap_or("").to_string();
    let source_path = row["source_path"].as_str().unwrap_or("").to_string();
    let dest_path = row["dest_path"].as_str().unwrap_or("").to_string();

    // Parse error kind
    let error_kind = match error_kind_str {
        "SourceNotFound" => crate::api::TransferError::SourceNotFound,
        "PermissionDenied" => crate::api::TransferError::PermissionDenied,
        "DiskFull" => crate::api::TransferError::DiskFull,
        "HashMismatch" => crate::api::TransferError::HashMismatch,
        _ => crate::api::TransferError::IoError(error_kind_str.to_string()),
    };

    // Parse timestamps
    let created_at = row["created_at"]
        .as_str()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(chrono::Utc::now);

    Some(ReviewItem {
        id,
        intent,
        transfer,
        error_kind,
        error_message,
        source_path,
        dest_path,
        source_info: None,
        dest_info: None,
        created_at,
    })
}
