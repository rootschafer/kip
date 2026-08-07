//! Cloud destination configuration

use serde::{Deserialize, Serialize};

/// Cloud storage provider type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CloudProvider {
    GoogleDrive,
    Nextcloud,
    Dropbox,
    OneDrive,
    S3,
    SFTP,
    WebDAV,
    Other(String),
}

/// Google Drive configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleDriveConfig {
    /// rclone remote name (configured in rclone.conf)
    pub remote_name: String,
    /// Folder path within the remote
    pub folder_path: String,
    /// Optional Service Account credentials file path
    pub service_account_file: Option<String>,
}

impl GoogleDriveConfig {
    pub fn new(remote_name: impl Into<String>, folder_path: impl Into<String>) -> Self {
        Self {
            remote_name: remote_name.into(),
            folder_path: folder_path.into(),
            service_account_file: None,
        }
    }

    pub fn with_service_account(mut self, path: impl Into<String>) -> Self {
        self.service_account_file = Some(path.into());
        self
    }

    /// Get the full remote path (remote_name:folder_path)
    pub fn full_path(&self) -> String {
        format!("{}:{}", self.remote_name, self.folder_path)
    }
}

/// Nextcloud/WebDAV configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NextcloudConfig {
    /// rclone remote name
    pub remote_name: String,
    /// Folder path within the remote
    pub folder_path: String,
    /// Nextcloud server URL (for setup info)
    pub server_url: Option<String>,
}

impl NextcloudConfig {
    pub fn new(remote_name: impl Into<String>, folder_path: impl Into<String>) -> Self {
        Self {
            remote_name: remote_name.into(),
            folder_path: folder_path.into(),
            server_url: None,
        }
    }

    pub fn with_server_url(mut self, url: impl Into<String>) -> Self {
        self.server_url = Some(url.into());
        self
    }

    pub fn full_path(&self) -> String {
        format!("{}:{}", self.remote_name, self.folder_path)
    }
}

/// Amazon S3 configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3Config {
    /// rclone remote name
    pub remote_name: String,
    /// Bucket name
    pub bucket: String,
    /// Optional path within bucket
    pub path: Option<String>,
    /// AWS region
    pub region: Option<String>,
}

impl S3Config {
    pub fn new(remote_name: impl Into<String>, bucket: impl Into<String>) -> Self {
        Self {
            remote_name: remote_name.into(),
            bucket: bucket.into(),
            path: None,
            region: None,
        }
    }

    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    pub fn with_region(mut self, region: impl Into<String>) -> Self {
        self.region = Some(region.into());
        self
    }

    pub fn full_path(&self) -> String {
        match &self.path {
            Some(path) => format!("{}:{}/{}", self.remote_name, self.bucket, path),
            None => format!("{}:{}", self.remote_name, self.bucket),
        }
    }
}

/// Generic WebDAV configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebDAVConfig {
    /// rclone remote name
    pub remote_name: String,
    /// Folder path within the remote
    pub folder_path: String,
    /// WebDAV server URL
    pub url: String,
}

impl WebDAVConfig {
    pub fn new(
        remote_name: impl Into<String>,
        folder_path: impl Into<String>,
        url: impl Into<String>,
    ) -> Self {
        Self {
            remote_name: remote_name.into(),
            folder_path: folder_path.into(),
            url: url.into(),
        }
    }

    pub fn full_path(&self) -> String {
        format!("{}:{}", self.remote_name, self.folder_path)
    }
}

/// Cloud destination - type-safe representation
#[derive(Debug, Clone)]
pub enum CloudDestination {
    GoogleDrive(GoogleDriveConfig),
    Nextcloud(NextcloudConfig),
    S3(S3Config),
    WebDAV(WebDAVConfig),
    /// Generic remote (just use remote_name:path)
    Generic {
        remote_name: String,
        path: String,
    },
}

impl CloudDestination {
    /// Get the provider type
    pub fn provider(&self) -> CloudProvider {
        match self {
            Self::GoogleDrive(_) => CloudProvider::GoogleDrive,
            Self::Nextcloud(_) => CloudProvider::Nextcloud,
            Self::S3(_) => CloudProvider::S3,
            Self::WebDAV(_) => CloudProvider::WebDAV,
            Self::Generic { .. } => CloudProvider::Other("generic".to_string()),
        }
    }

    /// Get the full remote path for rclone
    pub fn full_path(&self) -> String {
        match self {
            Self::GoogleDrive(config) => config.full_path(),
            Self::Nextcloud(config) => config.full_path(),
            Self::S3(config) => config.full_path(),
            Self::WebDAV(config) => config.full_path(),
            Self::Generic {
                remote_name, path, ..
            } => format!("{}:{}", remote_name, path),
        }
    }

    /// Get the remote name
    pub fn remote_name(&self) -> &str {
        match self {
            Self::GoogleDrive(config) => &config.remote_name,
            Self::Nextcloud(config) => &config.remote_name,
            Self::S3(config) => &config.remote_name,
            Self::WebDAV(config) => &config.remote_name,
            Self::Generic { remote_name, .. } => remote_name,
        }
    }

    /// Create a Google Drive destination
    pub fn google_drive(remote_name: impl Into<String>, path: impl Into<String>) -> Self {
        Self::GoogleDrive(GoogleDriveConfig::new(remote_name, path))
    }

    /// Create a Nextcloud destination
    pub fn nextcloud(remote_name: impl Into<String>, path: impl Into<String>) -> Self {
        Self::Nextcloud(NextcloudConfig::new(remote_name, path))
    }

    /// Create an S3 destination
    pub fn s3(remote_name: impl Into<String>, bucket: impl Into<String>) -> Self {
        Self::S3(S3Config::new(remote_name, bucket))
    }

    /// Create a WebDAV destination
    pub fn webdav(
        remote_name: impl Into<String>,
        path: impl Into<String>,
        url: impl Into<String>,
    ) -> Self {
        Self::WebDAV(WebDAVConfig::new(remote_name, path, url))
    }

    /// Create a generic destination
    pub fn generic(remote_name: impl Into<String>, path: impl Into<String>) -> Self {
        Self::Generic {
            remote_name: remote_name.into(),
            path: path.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_google_drive_config() {
        let config = GoogleDriveConfig::new("mydrive", "backups");
        assert_eq!(config.full_path(), "mydrive:backups");
    }

    #[test]
    fn test_nextcloud_config() {
        let config = NextcloudConfig::new("nextcloud", "backup/kip")
            .with_server_url("https://cloud.example.com");
        assert_eq!(config.full_path(), "nextcloud:backup/kip");
        assert_eq!(config.server_url, Some("https://cloud.example.com".to_string()));
    }

    #[test]
    fn test_s3_config() {
        let config = S3Config::new("aws", "my-bucket").with_path("backups");
        assert_eq!(config.full_path(), "aws:my-bucket/backups");
    }

    #[test]
    fn test_cloud_destination() {
        let dest = CloudDestination::google_drive("gdrive", "backups");
        assert_eq!(dest.full_path(), "gdrive:backups");
        assert_eq!(dest.remote_name(), "gdrive");
        assert!(matches!(dest.provider(), CloudProvider::GoogleDrive));
    }
}
