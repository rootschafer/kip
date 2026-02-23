//! TestApp - In-memory database fixture for integration tests
//!
//! Each test gets its own isolated in-memory SurrealDB instance.
//! This avoids database lock issues and provides true test isolation.

use kip::db::{self, DbHandle};
use std::path::PathBuf;
use tempfile::TempDir;

/// Test application fixture with isolated in-memory database
pub struct TestApp {
    pub db: DbHandle,
    _temp_dir: TempDir,  // Held to keep temp dir alive
}

impl TestApp {
    /// Create a new test app with a fresh in-memory database
    pub async fn new() -> Self {
        // Use memory storage for tests - no file locking issues
        let db = db::init_memory()
            .await
            .expect("Failed to initialize in-memory test database");
        
        // Create a temp dir that will be cleaned up when TestApp is dropped
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        
        Self {
            db,
            _temp_dir: temp_dir,
        }
    }
    
    /// Create a test app with a file-based database (for persistence tests)
    pub async fn with_file_db() -> Self {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("test.db");
        
        let db = db::init_with_path(&db_path)
            .await
            .expect("Failed to initialize file-based test database");
        
        Self {
            db,
            _temp_dir: temp_dir,
        }
    }
    
    /// Get a reference to the database handle
    pub fn db(&self) -> &DbHandle {
        &self.db
    }
    
    /// Create a test location at the given path
    pub async fn create_test_location(&self, path: PathBuf, label: Option<String>) -> String {
        kip::api::add_location(&self.db, path, label, None)
            .await
            .expect("Failed to create test location")
    }
    
    /// Create a test intent between source and destinations
    pub async fn create_test_intent(
        &self,
        source_id: String,
        dest_ids: Vec<String>,
        name: Option<String>,
    ) -> String {
        let config = kip::api::IntentConfig {
            name,
            priority: 500,
            ..Default::default()
        };
        
        kip::api::create_intent(&self.db, source_id, dest_ids, config)
            .await
            .expect("Failed to create test intent")
    }
}

impl Default for TestApp {
    fn default() -> Self {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(Self::new())
    }
}

impl Drop for TestApp {
    fn drop(&mut self) {
        // Database is automatically cleaned up when dropped
        // Temp dir is also cleaned up via _temp_dir field
    }
}

/// Helper function to get a test home directory
pub fn test_home_dir() -> PathBuf {
    std::env::temp_dir().join("kip-test-home")
}

/// Helper function to create a test directory structure
pub fn setup_test_dir(name: &str) -> PathBuf {
    let base = std::env::temp_dir().join("kip-test").join(name);
    std::fs::create_dir_all(&base).expect("Failed to create test directory");
    base
}

/// Helper function to clean up test directories
pub fn cleanup_test_dir(name: &str) {
    let _ = std::fs::remove_dir_all(std::env::temp_dir().join("kip-test").join(name));
}
