use dioxus::prelude::*;
use surrealdb::types::RecordId;

use crate::db::DbHandle;
use crate::ui::intent_row::IntentRow;

#[derive(Debug, Clone, PartialEq)]
struct IntentView {
    id: RecordId,
    source_path: String,
    dest_path: String,
    status: String,
    total_files: i64,
    completed_files: i64,
    total_bytes: i64,
    completed_bytes: i64,
}

#[component]
pub fn IntentList(refresh_tick: u32) -> Element {
    let db = use_context::<DbHandle>();

    let intents = use_resource(move || {
        let db = db.clone();
        let _tick = refresh_tick; // dependency so we re-fetch on tick change
        async move { fetch_intents(&db).await }
    });

    rsx! {
        div { class: "section-title mt-24", "Intents" }
        match &*intents.read() {
            Some(Ok(list)) if list.is_empty() => {
                rsx! { div { class: "empty", "No intents yet. Create one above." } }
            }
            Some(Ok(list)) => {
                rsx! {
                    for intent in list.iter() {
                        IntentRow {
                            key: "{intent.id:?}",
                            intent_id: intent.id.clone(),
                            source_path: intent.source_path.clone(),
                            dest_path: intent.dest_path.clone(),
                            status: intent.status.clone(),
                            total_files: intent.total_files,
                            completed_files: intent.completed_files,
                            total_bytes: intent.total_bytes,
                            completed_bytes: intent.completed_bytes,
                        }
                    }
                }
            }
            Some(Err(e)) => {
                rsx! { div { class: "empty", "Error loading intents: {e}" } }
            }
            None => {
                rsx! { div { class: "empty", "Loading..." } }
            }
        }
    }
}

async fn fetch_intents(db: &DbHandle) -> Result<Vec<IntentView>, String> {
    let mut resp = db
        .db
        .query(
            "SELECT
                id, status, total_files, completed_files, total_bytes, completed_bytes,
                created_at,
                source.path AS source_path,
                destinations[0].path AS dest_path
            FROM intent ORDER BY created_at DESC",
        )
        .await
        .map_err(|e| e.to_string())?;

    let rows: Vec<serde_json::Value> = resp.take(0).map_err(|e| e.to_string())?;

    let mut intents = Vec::with_capacity(rows.len());
    for row in rows {
        let id: RecordId = match serde_json::from_value(row["id"].clone()) {
            Ok(id) => id,
            Err(_) => continue,
        };

        intents.push(IntentView {
            id,
            source_path: row["source_path"]
                .as_str()
                .unwrap_or("?")
                .to_string(),
            dest_path: row["dest_path"]
                .as_str()
                .unwrap_or("?")
                .to_string(),
            status: row["status"]
                .as_str()
                .unwrap_or("unknown")
                .to_string(),
            total_files: row["total_files"].as_i64().unwrap_or(0),
            completed_files: row["completed_files"].as_i64().unwrap_or(0),
            total_bytes: row["total_bytes"].as_i64().unwrap_or(0),
            completed_bytes: row["completed_bytes"].as_i64().unwrap_or(0),
        });
    }

    Ok(intents)
}
