//! Test fixture for local rsync tests

use std::path::{Path, PathBuf};
use std::fs;
use tempfile::TempDir;

use crate::test_utils::create_test_filesystem;

/// A test fixture that creates a temporary directory with a test filesystem
/// 
/// Automatically cleaned up on drop unless `cleanup` is set to false.
/// 
/// # Example
/// 
/// ```rust,no_run
/// #[tokio::test]
/// async fn test_backup() {
///     let src = LocalTestTempDir::new("test_backup_src").unwrap();
///     let dst = LocalTestTempDir::new("test_backup_dst").unwrap();
///     
///     // Run backup from src to dst
///     // ...
///     
///     // Directories automatically cleaned up
/// }
/// ```
pub struct LocalTestTempDir {
    /// The temporary directory
    temp_dir: Option<TempDir>,
    /// Whether to cleanup on drop
    cleanup: bool,
    /// Name/description of this temp dir (for debugging)
    name: String,
}

impl LocalTestTempDir {
    /// Create a new test temp directory with the test filesystem
    /// 
    /// The directory will have a unique name based on the test name and timestamp.
    pub fn new(name: impl Into<String>) -> std::io::Result<Self> {
        let name = name.into();
        let temp_dir = TempDir::with_prefix(&format!("kip_rsync_{}_", name))?;
        
        // Create test filesystem
        create_test_filesystem(temp_dir.path())?;
        
        Ok(Self {
            temp_dir: Some(temp_dir),
            cleanup: true,
            name,
        })
    }
    
    /// Create a new empty test temp directory (no test filesystem)
    pub fn empty(name: impl Into<String>) -> std::io::Result<Self> {
        let name = name.into();
        let temp_dir = TempDir::with_prefix(&format!("kip_rsync_{}_", name))?;
        
        Ok(Self {
            temp_dir: Some(temp_dir),
            cleanup: true,
            name,
        })
    }
    
    /// Get the path to the temp directory
    pub fn path(&self) -> &Path {
        self.temp_dir.as_ref().unwrap().path()
    }
    
    /// Get the path as a PathBuf
    pub fn path_buf(&self) -> PathBuf {
        self.temp_dir.as_ref().unwrap().path().to_path_buf()
    }
    
    /// Set whether to cleanup on drop
    /// 
    /// If false, the directory will be preserved after the test for inspection.
    pub fn with_cleanup(mut self, cleanup: bool) -> Self {
        self.cleanup = cleanup;
        self
    }
    
    /// Disable cleanup (preserve directory after test)
    pub fn no_cleanup(mut self) -> Self {
        self.cleanup = false;
        self
    }
    
    /// Check if cleanup is enabled
    pub fn will_cleanup(&self) -> bool {
        self.cleanup
    }
    
    /// Get the name/description
    pub fn name(&self) -> &str {
        &self.name
    }
    
    /// Manually cleanup (even if cleanup is disabled)
    pub fn cleanup_now(mut self) {
        self.cleanup = true;
        drop(self);
    }
}

impl Drop for LocalTestTempDir {
    fn drop(&mut self) {
        if self.cleanup {
            // TempDir automatically cleans up when dropped
            // We just need to not prevent it
            if let Ok(path) = std::env::var("KIP_RSYNC_KEEP_TEMP") {
                if !path.is_empty() {
                    // User wants to keep all temp dirs
                    eprintln!(
                        "Preserving test dir {} at {:?} (KIP_RSYNC_KEEP_TEMP is set)",
                        self.name,
                        self.path()
                    );
                    // Prevent cleanup by taking ownership and keeping the temp dir
                    if let Some(temp_dir) = self.temp_dir.take() {
                        let _ = temp_dir.keep();
                    }
                    return;
                }
            }
            
            eprintln!("Cleaning up test dir {} at {:?}", self.name, self.path());
        } else {
            eprintln!(
                "Preserving test dir {} at {:?} (cleanup disabled)",
                self.name,
                self.path()
            );
            // Prevent cleanup by taking ownership and keeping the temp dir
            if let Some(temp_dir) = self.temp_dir.take() {
                let _ = temp_dir.keep();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_local_test_temp_dir() {
        let temp = LocalTestTempDir::new("test").unwrap();
        
        // Verify test filesystem was created
        assert!(temp.path().join("file1.txt").exists());
        assert!(temp.path().join("subdir1").exists());
        assert!(temp.path().join("large_file.bin").exists());
        
        // Verify path methods
        assert_eq!(temp.path(), temp.path_buf().as_path());
        
        // Verify name
        assert_eq!(temp.name(), "test");
        
        // Verify cleanup is enabled by default
        assert!(temp.will_cleanup());
        
        // Directory will be cleaned up on drop
    }
    
    #[test]
    fn test_local_test_temp_dir_no_cleanup() {
        let temp = LocalTestTempDir::new("test_no_cleanup")
            .unwrap()
            .no_cleanup();
        
        assert!(!temp.will_cleanup());
        
        // Directory will NOT be cleaned up on drop
        // (we can't easily verify this in a test, but the Drop impl handles it)
    }
    
    #[test]
    fn test_local_test_temp_dir_empty() {
        let temp = LocalTestTempDir::empty("test_empty").unwrap();
        
        // Verify no test filesystem
        assert!(!temp.path().join("file1.txt").exists());
        
        // Directory should be empty
        assert!(fs::read_dir(temp.path()).unwrap().next().is_none());
    }
    
    #[test]
    fn test_local_test_temp_dir_with_cleanup() {
        let temp = LocalTestTempDir::new("test_cleanup")
            .unwrap()
            .with_cleanup(false);
        
        assert!(!temp.will_cleanup());
        
        let temp = temp.with_cleanup(true);
        assert!(temp.will_cleanup());
    }
}
