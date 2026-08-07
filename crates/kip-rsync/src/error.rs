//! Error types for kip-rsync

use thiserror::Error;

/// Rsync operation errors
#[derive(Error, Debug)]
pub enum RsyncError {
	#[error("rsync command failed: {0}")]
	CommandFailed(String),

	#[error("rsync not found in PATH. Please install rsync.")]
	RsyncNotFound,

	#[error("source path does not exist: {0}")]
	SourceNotFound(String),

	#[error("destination path is invalid: {0}")]
	DestinationInvalid(String),

	#[error("insufficient disk space at {destination}: required {required}, available {available}")]
	InsufficientSpace {
		destination: String,
		required: String,
		available: String,
	},

	#[error("SSH connection failed to {host}: {error}")]
	SshFailed { host: String, error: String },

	#[error("operation cancelled")]
	Cancelled,

	#[error("progress parsing failed: {0}")]
	ProgressParseFailed(String),

	#[error("IO error: {0}")]
	Io(#[from] std::io::Error),
}

/// Result type alias for kip-rsync operations
pub type Result<T> = std::result::Result<T, RsyncError>;
