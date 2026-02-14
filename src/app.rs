use dioxus::prelude::*;

use crate::db::DbHandle;
use crate::ui::file_picker::{FilePickerLayer, PickerManager};
use crate::ui::graph::MappingGraph;
use crate::ui::review_queue::ReviewQueue;

const MAIN_CSS: Asset = asset!("/assets/main.css");

#[derive(Clone)]
pub struct DbError(pub String);

#[component]
pub fn DbErrorApp() -> Element {
    let err = use_context::<DbError>();
    let msg = if err.0.contains("already locked") {
        "Database is being accessed by another Kip instance."
    } else {
        &err.0
    };

    rsx! {
		document::Stylesheet { href: MAIN_CSS }
		div { class: "app",
			div { class: "header",
				h1 { "Kip" }
			}
			div { class: "db-locked-banner", "{msg}" }
		}
	}
}

#[component]
pub fn App() -> Element {
    let db = use_context::<DbHandle>();
    let picker = use_store(|| PickerManager::new());
    let hostname = use_signal(|| String::from("..."));
    let mut refresh_tick = use_signal(|| 0u32);

    let db_for_hostname = db.clone();
    let db_for_watcher = db.clone();

    // Load hostname once
    use_effect(move || {
        let db = db_for_hostname.clone();
        let mut hostname = hostname;
        spawn(async move {
            let mut response = db
                .db
                .query("SELECT name FROM machine:local")
                .await
                .unwrap();
            let result: Option<String> = response.take("name").unwrap_or(None);
            if let Some(name) = result {
                *hostname.write() = name;
            }
        });
    });

    // Start drive watcher (polls /Volumes/ every 5s)
    use_effect(move || {
        let db = db_for_watcher.clone();
        spawn(async move {
            let _watcher = crate::devices::DriveWatcher::start(db);
            std::future::pending::<()>().await;
        });
    });

    // Poll for updates every 2 seconds
    use_effect(move || {
        spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                *refresh_tick.write() += 1;
            }
        });
    });

    let on_refresh = move |_| {
        *refresh_tick.write() += 1;
    };

    rsx! {
		document::Stylesheet { href: MAIN_CSS }
		div { class: "app",
			// div { class: "header",
			//     h1 { "Kip" }
			//     div { class: "header-right",
			//         span { class: "host", "{hostname}" }
			//     }
			// }
			MappingGraph {
				picker,
				refresh_tick: refresh_tick(),
				on_changed: on_refresh,
			}
			FilePickerLayer { picker, on_location_added: on_refresh }
			ReviewQueue { refresh_tick: refresh_tick(), on_resolved: on_refresh }
		}
	}
}
