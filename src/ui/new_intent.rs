use dioxus::prelude::*;

use crate::db::DbHandle;

#[component]
pub fn NewIntent(on_created: EventHandler) -> Element {
    let db = use_context::<DbHandle>();
    let mut source = use_signal(|| String::new());
    let mut dest = use_signal(|| String::new());
    let mut creating = use_signal(|| false);
    let mut error_msg = use_signal(|| Option::<String>::None);

    let create = move |_| {
        let src = source().trim().to_string();
        let dst = dest().trim().to_string();

        if src.is_empty() || dst.is_empty() {
            *error_msg.write() = Some("Both paths are required".into());
            return;
        }

        *creating.write() = true;
        *error_msg.write() = None;

        let db = db.clone();
        let on_created = on_created;

        spawn(async move {
            let result = create_intent(&db, &src, &dst).await;
            *creating.write() = false;

            match result {
                Ok(_) => {
                    *source.write() = String::new();
                    *dest.write() = String::new();
                    on_created.call(());
                }
                Err(e) => {
                    *error_msg.write() = Some(e);
                }
            }
        });
    };

    rsx! {
        div { class: "section-title", "New Intent" }
        div { class: "card",
            div { class: "form-row",
                label { "Source" }
                input {
                    value: "{source}",
                    placeholder: "/path/to/source",
                    oninput: move |e| *source.write() = e.value(),
                }
            }
            div { class: "form-row",
                label { "Dest" }
                input {
                    value: "{dest}",
                    placeholder: "/path/to/destination",
                    oninput: move |e| *dest.write() = e.value(),
                }
            }
            if let Some(err) = error_msg() {
                div {
                    style: "color: #f85149; font-size: 12px; margin-bottom: 8px;",
                    "{err}"
                }
            }
            div { class: "form-actions",
                button {
                    class: "btn-primary",
                    disabled: creating(),
                    onclick: create,
                    if creating() { "Creating..." } else { "Create Intent" }
                }
            }
        }
    }
}

async fn create_intent(db: &DbHandle, source: &str, dest: &str) -> Result<(), String> {
    // Create locations and intent in a single query so record IDs
    // stay as native records (no serde_json::Value coercion issues).
    db.db
        .query(
            "LET $src = (CREATE location CONTENT {
                path: $source_path,
                machine: machine:local,
                available: true,
                created_at: time::now(),
            });
            LET $dst = (CREATE location CONTENT {
                path: $dest_path,
                machine: machine:local,
                available: true,
                created_at: time::now(),
            });
            CREATE intent CONTENT {
                source: $src[0].id,
                destinations: [$dst[0].id],
                status: 'idle',
                kind: 'one_shot',
                speed_mode: 'normal',
                priority: 0,
                total_files: 0,
                total_bytes: 0,
                completed_files: 0,
                completed_bytes: 0,
                created_at: time::now(),
                updated_at: time::now(),
            };",
        )
        .bind(("source_path", source.to_string()))
        .bind(("dest_path", dest.to_string()))
        .await
        .map_err(|e| e.to_string())?
        .check()
        .map_err(|e| e.to_string())?;

    Ok(())
}
