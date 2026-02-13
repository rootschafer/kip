use dioxus::prelude::*;
use surrealdb::types::RecordId;

use crate::db::DbHandle;

#[derive(Debug, Clone, PartialEq)]
struct ReviewView {
    id: RecordId,
    job: serde_json::Value,
    error_kind: String,
    error_message: String,
    source_path: String,
    dest_path: String,
    options: Vec<String>,
    source_size: Option<i64>,
    dest_size: Option<i64>,
}

#[component]
pub fn ReviewQueue(refresh_tick: u32, on_resolved: EventHandler) -> Element {
    let db = use_context::<DbHandle>();

    let items = use_resource(move || {
        let db = db.clone();
        let _tick = refresh_tick;
        async move { fetch_review_items(&db).await }
    });

    rsx! {
        match &*items.read() {
            Some(Ok(list)) if list.is_empty() => {
                rsx! {}
            }
            Some(Ok(list)) => {
                rsx! {
                    div { class: "section-title mt-24",
                        "Review Queue ({list.len()})"
                    }
                    for item in list.iter() {
                        ReviewCard {
                            key: "{item.id:?}",
                            item: item.clone(),
                            on_resolved: on_resolved,
                        }
                    }
                }
            }
            Some(Err(e)) => {
                rsx! {
                    div { class: "section-title mt-24", "Review Queue" }
                    div { class: "empty", "Error loading review items: {e}" }
                }
            }
            None => {
                rsx! {}
            }
        }
    }
}

#[component]
fn ReviewCard(item: ReviewView, on_resolved: EventHandler) -> Element {
    let db = use_context::<DbHandle>();
    let mut resolving = use_signal(|| false);

    let kind_class = match item.error_kind.as_str() {
        "source_missing" => "review-kind review-kind-missing",
        "permission_denied" => "review-kind review-kind-permission",
        "disk_full" => "review-kind review-kind-disk",
        "hash_mismatch" => "review-kind review-kind-hash",
        _ => "review-kind review-kind-io",
    };

    let kind_label = match item.error_kind.as_str() {
        "source_missing" => "Source Missing",
        "permission_denied" => "Permission Denied",
        "disk_full" => "Disk Full",
        "hash_mismatch" => "Hash Mismatch",
        "io_error" => "I/O Error",
        _ => &item.error_kind,
    };

    let size_info = match (item.source_size, item.dest_size) {
        (Some(s), Some(d)) => format!("{} → {}", format_bytes(s), format_bytes(d)),
        (Some(s), None) => format!("{}", format_bytes(s)),
        _ => String::new(),
    };

    rsx! {
        div { class: "review-card",
            div { class: "review-header",
                span { class: "{kind_class}", "{kind_label}" }
            }
            div { class: "review-message", "{item.error_message}" }
            div { class: "review-paths",
                "{item.source_path} → {item.dest_path}"
            }
            if !size_info.is_empty() {
                div { class: "review-meta", "{size_info}" }
            }
            div { class: "review-actions",
                for option in item.options.iter() {
                    {
                        let opt = option.clone();
                        let item_id = item.id.clone();
                        let job = item.job.clone();
                        let db = db.clone();
                        let on_resolved = on_resolved;

                        let btn_class = match opt.as_str() {
                            "retry" | "rescan" => "btn-resolve btn-resolve-retry",
                            "accept" => "btn-resolve btn-resolve-accept",
                            _ => "btn-resolve btn-resolve-skip",
                        };

                        rsx! {
                            button {
                                class: "{btn_class}",
                                disabled: resolving(),
                                onclick: move |_| {
                                    *resolving.write() = true;
                                    let db = db.clone();
                                    let item_id = item_id.clone();
                                    let job = job.clone();
                                    let opt = opt.clone();
                                    let on_resolved = on_resolved;
                                    spawn(async move {
                                        let _ = resolve_item(&db, &item_id, &job, &opt).await;
                                        on_resolved.call(());
                                    });
                                },
                                "{opt}"
                            }
                        }
                    }
                }
            }
        }
    }
}

fn format_bytes(bytes: i64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.1} GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.0} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

async fn fetch_review_items(db: &DbHandle) -> Result<Vec<ReviewView>, String> {
    let mut resp = db
        .db
        .query(
            "SELECT id, job, error_kind, error_message, source_path, dest_path,
                    options, source_size, dest_size, created_at
             FROM review_item
             WHERE resolution IS NONE
             ORDER BY created_at DESC",
        )
        .await
        .map_err(|e| e.to_string())?;

    let rows: Vec<serde_json::Value> = resp.take(0).map_err(|e| e.to_string())?;

    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        let id: RecordId = match serde_json::from_value(row["id"].clone()) {
            Ok(id) => id,
            Err(_) => continue,
        };

        let options: Vec<String> = row["options"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        items.push(ReviewView {
            id,
            job: row["job"].clone(),
            error_kind: row["error_kind"].as_str().unwrap_or("unknown").to_string(),
            error_message: row["error_message"].as_str().unwrap_or("").to_string(),
            source_path: row["source_path"].as_str().unwrap_or("?").to_string(),
            dest_path: row["dest_path"].as_str().unwrap_or("?").to_string(),
            options,
            source_size: row["source_size"].as_i64(),
            dest_size: row["dest_size"].as_i64(),
        });
    }

    Ok(items)
}

async fn resolve_item(
    db: &DbHandle,
    item_id: &RecordId,
    job: &serde_json::Value,
    resolution: &str,
) -> Result<(), String> {
    // Mark the review item as resolved
    db.db
        .query("UPDATE $id SET resolution = $res, resolved_at = time::now()")
        .bind(("id", item_id.clone()))
        .bind(("res", resolution.to_string()))
        .await
        .map_err(|e| e.to_string())?
        .check()
        .map_err(|e| e.to_string())?;

    // Act on the resolution
    match resolution {
        "retry" | "rescan" => {
            // Reset job to pending so scheduler can retry
            db.db
                .query("UPDATE $job SET status = 'pending', attempts = 0")
                .bind(("job", job.clone()))
                .await
                .map_err(|e| e.to_string())?
                .check()
                .map_err(|e| e.to_string())?;
        }
        "accept" => {
            // Mark job as complete (user accepts the result)
            db.db
                .query("UPDATE $job SET status = 'complete', completed_at = time::now()")
                .bind(("job", job.clone()))
                .await
                .map_err(|e| e.to_string())?
                .check()
                .map_err(|e| e.to_string())?;
        }
        "skip" => {
            // Mark job as skipped
            db.db
                .query("UPDATE $job SET status = 'skipped'")
                .bind(("job", job.clone()))
                .await
                .map_err(|e| e.to_string())?
                .check()
                .map_err(|e| e.to_string())?;
        }
        _ => {}
    }

    Ok(())
}
