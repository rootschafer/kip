//! Error types for cloud operations

use thiserror::Error;

/// Cloud storage operation errors
#[derive(Error, Debug)]
pub enum CloudError {
	#[error("rclone not found in PATH. Please install rclone from https://rclone.org")]
	RcloneNotFound,

	#[error("rclone command failed: {0}")]
	RcloneCommandFailed(String),

	#[error("rclone configuration not found. Run 'rclone config' to set up remotes")]
	RcloneConfigNotFound,

	#[error("remote '{0}' not configured. Run 'rclone config' to set it up")]
	RemoteNotFound(String),

	#[error("authentication failed for {0}: {1}")]
	AuthenticationFailed(String, String),

	#[error("insufficient storage on {provider}: required {required}, available {available}")]
	InsufficientStorage {
		provider: String,
		required: String,
		available: String,
	},

	#[error("rate limit exceeded for {0}. Try again later")]
	RateLimitExceeded(String),

	#[error("file not found: {0}")]
	FileNotFound(String),

	#[error("invalid path: {0}")]
	InvalidPath(String),

	#[error("IO error: {0}")]
	Io(#[from] std::io::Error),

	#[error("JSON error: {0}")]
	Json(#[from] serde_json::Error),
}

/// Result type alias for cloud operations
pub type Result<T> = std::result::Result<T, CloudError>;
