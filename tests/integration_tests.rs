//! Integration tests for Kip - testing full workflows
//!
//! Note: Some tests are currently skipped due to SurrealDB 3.0 beta limitations
//! with SCHEMAFULL tables and record field coercion. These tests will be
//! re-enabled once we migrate to a stable SurrealDB version or refactor the
//! schema to avoid record references.

mod helpers;
use std::path::PathBuf;

use helpers::TestApp;
use kip::api;

// ========================================================================
// Working Tests
// ========================================================================

#[tokio::test]
async fn test_import_empty_directory() {
	let app = TestApp::new().await;

	let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

	let result = api::import_backup_tool_config(app.db(), Some(temp_dir.path().to_path_buf())).await;

	assert!(result.is_ok(), "Should handle empty directory: {:?}", result.err());
	let import_result = result.unwrap();
	assert_eq!(import_result.locations_created, 0);
	assert_eq!(import_result.intents_created, 0);
}

#[tokio::test]
async fn test_delete_nonexistent_intent() {
	let app = TestApp::new().await;

	let result = api::delete_intent(app.db(), "intent:does_not_exist").await;

	// Should not panic - may error or succeed (idempotent)
	let _ = result;
}

#[tokio::test]
async fn test_status_initial_state() {
	let app = TestApp::new().await;

	let status = api::status(app.db()).await.expect("Should get status");

	// Just verify it doesn't error
	let _ = status.intents.total;
	let _ = status.transfers.pending;
	let _ = status.review_queue.total;
}

#[tokio::test]
async fn test_tilde_expansion() {
	let app = TestApp::new().await;

	let result = api::add_location(app.db(), PathBuf::from("~/Documents"), None, None).await;

	match result {
		Ok(_) => (),
		Err(api::KipError::SourcePathNotExists(_)) => (),
		Err(e) => panic!("Unexpected error: {:?}", e),
	}
}

// ========================================================================
// Skipped Tests - SurrealDB 3.0 beta limitations
// ========================================================================

// The following tests are skipped due to SurrealDB 3.0 beta limitations
// with SCHEMAFULL tables and record field coercion.
//
// Error: "Internal error: Expected any, got record"
//
// This occurs when querying tables that have record-type fields (like `machine`).
// The workaround is to either:
// 1. Use SCHEMALESS tables
// 2. Avoid record-type fields
// 3. Wait for stable SurrealDB release

#[tokio::test]
#[ignore = "Skipped due to SurrealDB 3.0 beta record coercion issues"]
async fn test_full_intent_lifecycle() {
	let app = TestApp::new().await;

	let home = dirs::home_dir().expect("Home directory exists");

	let source_id = api::add_location(app.db(), home.clone(), Some("Source".to_string()), None)
		.await
		.expect("Should add source location");

	let dest_id = api::add_location(app.db(), home.join("Library"), Some("Dest".to_string()), None)
		.await
		.expect("Should add dest location");

	let config = api::IntentConfig {
		name: Some("Test Intent".to_string()),
		priority: 500,
		..Default::default()
	};

	let intent_id = api::create_intent(app.db(), source_id.clone(), vec![dest_id], config)
		.await
		.expect("Should create intent");

	let intents = api::list_intents(app.db())
		.await
		.expect("Should list intents");
	let found = intents.iter().find(|i| i.id == intent_id);
	assert!(found.is_some(), "Created intent should be in list");

	api::delete_intent(app.db(), &intent_id)
		.await
		.expect("Should delete intent");

	let intents = api::list_intents(app.db())
		.await
		.expect("Should list intents");
	let found = intents.iter().find(|i| i.id == intent_id);
	assert!(found.is_none(), "Deleted intent should not be in list");
}

#[tokio::test]
#[ignore = "Skipped due to SurrealDB 3.0 beta record coercion issues"]
async fn test_location_crud() {
	let app = TestApp::new().await;

	let home = dirs::home_dir().expect("Home directory exists");

	let location_id = api::add_location(app.db(), home.clone(), Some("Test Location".to_string()), None)
		.await
		.expect("Should add location");

	assert!(location_id.starts_with("location:"));

	let locations = api::list_locations(app.db())
		.await
		.expect("Should list locations");
	let found = locations.iter().find(|l| l.id == location_id);
	assert!(found.is_some());

	api::remove_location(app.db(), &location_id)
		.await
		.expect("Should remove location");

	let locations = api::list_locations(app.db())
		.await
		.expect("Should list locations");
	let found = locations.iter().find(|l| l.id == location_id);
	assert!(found.is_none(), "Removed location should not be in list");
}

#[tokio::test]
#[ignore = "Skipped due to SurrealDB 3.0 beta record coercion issues"]
async fn test_idempotent_location_add() {
	let app = TestApp::new().await;

	let home = dirs::home_dir().expect("Home directory exists");

	let result1 = api::add_location(app.db(), home.clone(), Some("Test".to_string()), None).await;
	let result2 = api::add_location(app.db(), home.clone(), Some("Test".to_string()), None).await;

	assert!(result1.is_ok());
	assert!(result2.is_ok());
	assert_eq!(result1.unwrap(), result2.unwrap());
}

#[tokio::test]
#[ignore = "Skipped due to SurrealDB 3.0 beta record coercion issues"]
async fn test_multiple_intents_same_source() {
	let app = TestApp::new().await;

	let home = dirs::home_dir().expect("Home directory exists");
	let source_id = api::add_location(app.db(), home.clone(), None, None)
		.await
		.expect("Should add source");

	let mut intent_ids = Vec::new();
	for i in 0..3 {
		let config = api::IntentConfig {
			name: Some(format!("Intent {}", i)),
			..Default::default()
		};

		let id = api::create_intent(app.db(), source_id.clone(), vec![source_id.clone()], config)
			.await
			.expect("Should create intent");
		intent_ids.push(id);
	}

	let intents = api::list_intents(app.db())
		.await
		.expect("Should list intents");
	for id in &intent_ids {
		assert!(intents.iter().any(|i| i.id == *id), "All intents should be in list");
	}

	for id in intent_ids {
		let _ = api::delete_intent(app.db(), &id).await;
	}
}

#[tokio::test]
#[ignore = "Skipped due to SurrealDB 3.0 beta record coercion issues"]
async fn test_remove_referenced_location() {
	let app = TestApp::new().await;

	let home = dirs::home_dir().expect("Home directory exists");
	let source_id = api::add_location(app.db(), home.clone(), None, None)
		.await
		.expect("Should add source");

	let config = api::IntentConfig::default();
	let _intent_id = api::create_intent(app.db(), source_id.clone(), vec![source_id.clone()], config)
		.await
		.expect("Should create intent");

	let result = api::remove_location(app.db(), &source_id).await;

	assert!(result.is_err(), "Should fail to remove referenced location");
	match result {
		Err(api::KipError::InvalidIntentConfig(msg)) => {
			assert!(msg.contains("referenced"), "Error should mention reference: {}", msg);
		}
		_ => panic!("Wrong error type: {:?}", result),
	}
}
