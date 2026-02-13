mod app;
mod db;
mod devices;
mod engine;
mod models;
mod ui;
mod util;

use dioxus::prelude::*;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};


fn main() {
    let file_appender = tracing_appender::rolling::never(".", "kip.log");
    tracing_subscriber::registry()
      .with(tracing_subscriber::fmt::layer().with_writer(file_appender))
      .init();

    // Initialize database before Dioxus launch.
    // We keep the runtime alive — SurrealDB uses it for background tasks.
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    let db_result = rt.block_on(async { db::init().await });



    // Leak the runtime so SurrealDB's internal channels stay open.
    // Dioxus creates its own runtime for UI async work.
    Box::leak(Box::new(rt));

    match db_result {
        Ok(db) => {
            LaunchBuilder::new()
                .with_context(db)
                .launch(app::App);
        }
        Err(e) => {
            let err = e.to_string();
            LaunchBuilder::new()
                .with_context(app::DbError(err))
                .launch(app::DbErrorApp);
        }
    }
}
