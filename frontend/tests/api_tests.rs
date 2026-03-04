//! Unit tests for the Kip API layer

mod helpers;
use std::path::PathBuf;

use helpers::TestApp;
use frontend::api;

// ========================================================================
// Location API Tests
// ========================================================================

#[tokio::test]
async fn test_add_location_valid_path() {
	let app = TestApp::new().await;

	let home = dirs::home_dir().expect("Home directory exists");
	let result = api::add_location(app.db(), home.clone(), Some("Test Home".to_string()), None).await;

	assert!(result.is_ok(), "Should add valid location: {:?}", result.err());
}

#[tokio::test]
async fn test_add_location_nonexistent_path() {
	let app = TestApp::new().await;

	let fake_path = PathBuf::from("/this/path/does/not/exist");
	let result = api::add_location(app.db(), fake_path, None, None).await;

	assert!(result.is_err(), "Should fail for nonexistent path");
	match result {
		Err(api::KipError::SourcePathNotExists(_)) => (),
		_ => panic!("Wrong error type: {:?}", result),
	}
}

#[tokio::test]
async fn test_list_locations() {
	let app = TestApp::new().await;

	let locations = api::list_locations(app.db()).await;

	assert!(locations.is_ok(), "list_locations should not error: {:?}", locations.err());
}

// ========================================================================
// Intent API Tests
// ========================================================================

// NOTE: These tests are currently ignored due to SurrealDB 3.0 type coercion issues.
// SurrealDB interprets ULID strings as record IDs when binding parameters.
// This is a known limitation that will be addressed by either:
// 1. Using a different ID format that doesn't look like a record ID
// 2. Waiting for SurrealDB to fix the type coercion behavior
// 3. Using raw SQL string interpolation instead of bind parameters

#[tokio::test]
#[ignore = "SurrealDB 3.0 interprets ULID strings as record IDs"]
async fn test_create_intent_basic() {
	let app = TestApp::new().await;

	let home = dirs::home_dir().expect("Home directory exists");
	let source_id = api::add_location(app.db(), home.clone(), None, None)
		.await
		.expect("Add source");

	let config = api::IntentConfig::default();
	let result = api::create_intent(app.db(), source_id.clone(), vec![source_id], config).await;

	assert!(result.is_ok(), "Should create intent: {:?}", result.err());
	let intent_id = result.unwrap();
	assert!(
		intent_id.starts_with("intent:"),
		"Intent ID should have correct format, got: {}",
		intent_id
	);
}

#[tokio::test]
#[ignore = "Depends on test_create_intent_basic"]
async fn test_delete_intent() {
	let app = TestApp::new().await;

	let home = dirs::home_dir().expect("Home directory exists");
	let source_id = api::add_location(app.db(), home.clone(), None, None)
		.await
		.expect("Add source");

	let config = api::IntentConfig::default();
	let intent_result = api::create_intent(app.db(), source_id.clone(), vec![source_id.clone()], config).await;
	assert!(intent_result.is_ok(), "Should create intent: {:?}", intent_result.err());

	let intent_id = intent_result.unwrap();
	let delete_result = api::delete_intent(app.db(), &intent_id).await;

	assert!(delete_result.is_ok(), "Should delete intent: {:?}", delete_result.err());
}

#[tokio::test]
async fn test_list_intents() {
	let app = TestApp::new().await;

	let intents = api::list_intents(app.db()).await;

	assert!(intents.is_ok(), "list_intents should not error: {:?}", intents.err());
}

// ========================================================================
// Query API Tests
// ========================================================================

#[tokio::test]
async fn test_status() {
	let app = TestApp::new().await;

	let status = api::status(app.db()).await;

	assert!(status.is_ok(), "Status should work: {:?}", status.err());
	let s = status.unwrap();

	let _ = s.intents.total;
	let _ = s.transfers.pending;
	let _ = s.review_queue.total;
}

// ========================================================================
// Config Import Tests
// ========================================================================

#[tokio::test]
async fn test_import_nonexistent_config() {
	let app = TestApp::new().await;

	let fake_dir = PathBuf::from("/this/does/not/exist");
	let result = api::import_backup_tool_config(app.db(), Some(fake_dir)).await;

	assert!(result.is_err(), "Should fail for nonexistent directory");
	match result {
		Err(api::KipError::ConfigImport(msg)) => {
			assert!(msg.contains("does not exist"), "Error should mention 'does not exist': {}", msg);
		}
		_ => panic!("Wrong error type: {:?}", result),
	}
}

// ========================================================================
// Error Type Tests (no database needed)
// ========================================================================

#[test]
fn test_transfer_error_is_retryable() {
	assert!(api::TransferError::IoError("test".to_string()).is_retryable());
	assert!(api::TransferError::Interrupted.is_retryable());
	assert!(!api::TransferError::SourceNotFound.is_retryable());
	assert!(!api::TransferError::PermissionDenied.is_retryable());
	assert!(!api::TransferError::HashMismatch.is_retryable());
	assert!(!api::TransferError::DiskFull.is_retryable());
}

#[test]
fn test_transfer_error_needs_review() {
	assert!(api::TransferError::SourceNotFound.needs_review());
	assert!(api::TransferError::PermissionDenied.needs_review());
	assert!(api::TransferError::DiskFull.needs_review());
	assert!(api::TransferError::HashMismatch.needs_review());
	assert!(!api::TransferError::IoError("test".to_string()).needs_review());
	assert!(!api::TransferError::Interrupted.needs_review());
}

#[test]
fn test_error_display() {
	let err = api::TransferError::SourceNotFound;
	let msg = format!("{}", err);
	assert!(msg.contains("Source not found"), "Error message should be descriptive: {}", msg);

	let err = api::TransferError::PermissionDenied;
	let msg = format!("{}", err);
	assert!(
		msg.contains("Permission denied"),
		"Error message should be descriptive: {}",
		msg
	);
}

#[test]
fn test_kip_error_display() {
	let err = api::KipError::IntentNotFound("test_id".to_string());
	let msg = format!("{}", err);
	assert!(msg.contains("test_id"), "Error should include ID: {}", msg);

	let err = api::KipError::SourcePathNotExists(std::path::PathBuf::from("/test"));
	let msg = format!("{}", err);
	assert!(
		msg.contains("Source path does not exist"),
		"Error should be descriptive: {}",
		msg
	);
}
