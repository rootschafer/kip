#![cfg_attr(not(feature = "desktop"), allow(dead_code))]

#[cfg(feature = "desktop")]
mod app;
#[cfg(feature = "desktop")]
mod db;
#[cfg(feature = "desktop")]
mod devices;
#[cfg(feature = "desktop")]
mod engine;
#[cfg(feature = "desktop")]
mod models;
#[cfg(feature = "desktop")]
mod ui;
#[cfg(feature = "desktop")]
mod util;

#[cfg(feature = "desktop")]
use dioxus::prelude::*;

#[cfg(feature = "desktop")]
fn main() {
	// Enable INFO level logging to console
	tracing_subscriber::fmt()
		.with_max_level(tracing::Level::INFO)
		.init();

	info!("Starting Kip");

	// Initialize database before Dioxus launch.
	// We keep the runtime alive — SurrealDB uses it for background tasks.
	let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
	let db_result = rt.block_on(async { db::init().await });

	// Leak the runtime so SurrealDB's internal channels stay open.
	// Dioxus creates its own runtime for UI async work.
	Box::leak(Box::new(rt));

	match db_result {
		Ok(db) => {
			LaunchBuilder::new().with_context(db).launch(app::App);
		}
		Err(e) => {
			let err = e.to_string();
			LaunchBuilder::new()
				.with_context(app::DbError(err))
				.launch(app::DbErrorApp);
		}
	}
}

#[cfg(not(feature = "desktop"))]
fn main() {
    // Web or other platform entry point
    tracing::info!("Kip starting...");
}
