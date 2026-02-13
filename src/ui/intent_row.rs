use dioxus::prelude::*;
use surrealdb::types::RecordId;

use crate::db::DbHandle;
use crate::engine::{scanner, scheduler};

#[component]
pub fn IntentRow(
    intent_id: RecordId,
    source_path: String,
    dest_path: String,
    status: String,
    total_files: i64,
    completed_files: i64,
    total_bytes: i64,
    completed_bytes: i64,
) -> Element {
    let db = use_context::<DbHandle>();
    let mut running = use_signal(|| false);

    let percent = if total_files > 0 {
        ((completed_files as f64 / total_files as f64) * 100.0) as u32
    } else {
        0
    };

    let badge_class = match status.as_str() {
        "idle" => "badge badge-idle",
        "scanning" => "badge badge-scanning",
        "transferring" => "badge badge-transferring",
        "complete" => "badge badge-complete",
        "needs_review" => "badge badge-needs-review",
        "failed" => "badge badge-failed",
        _ => "badge badge-idle",
    };

    let start = move |_| {
        *running.write() = true;
        let db = db.clone();
        let id = intent_id.clone();

        spawn(async move {
            // Scan then schedule
            if let Err(e) = scanner::scan_intent(&db, &id).await {
                eprintln!("scan error: {e}");
                *running.write() = false;
                return;
            }
            if let Err(e) = scheduler::run_intent(&db, &id).await {
                eprintln!("scheduler error: {e}");
            }
            *running.write() = false;
        });
    };

    let display_name = if source_path.len() > 40 {
        format!("...{}", &source_path[source_path.len() - 37..])
    } else {
        source_path.clone()
    };

    rsx! {
        div { class: "intent-row",
            div { class: "intent-header",
                span { class: "intent-name", "{display_name}" }
                div { style: "display: flex; align-items: center; gap: 8px;",
                    span { class: "{badge_class}", "{status}" }
                    if status == "idle" && !running() {
                        button {
                            class: "btn-start",
                            onclick: start,
                            "Start"
                        }
                    }
                    if running() {
                        span { style: "color: #58a6ff; font-size: 12px;", "Running..." }
                    }
                }
            }
            div { class: "intent-paths",
                "{source_path} -> {dest_path}"
            }
            div { class: "progress-container",
                div { class: "progress-bar",
                    div {
                        class: "progress-fill",
                        style: "width: {percent}%",
                    }
                }
                div { class: "progress-text",
                    "{completed_files} / {total_files} files"
                }
            }
        }
    }
}
