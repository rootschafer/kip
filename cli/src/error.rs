//! Backup tool error types with helpful, actionable messages

use thiserror::Error;

/// Backup-specific errors with clear, actionable messages
#[derive(Error, Debug)]
pub enum BackupError {
	#[error("Failed to load {config_type}\n  Path: {path}\n  Reason: {reason}")]
	ConfigLoad {
		config_type: String,
		path: String,
		reason: String,
	},

	#[error("Drive '{drive_name}' not found in drives.toml\n  Available drives: {available_drives}")]
	DriveNotFound {
		drive_name: String,
		available_drives: String,
	},

	#[error("Source path does not exist\n  Path: {path}\n  Hint: Check if the path is correct and accessible")]
	SourceNotFound { path: String },

	#[error("Destination validation failed\n  Path: {path}\n  Reason: {reason}")]
	DestinationValidation { path: String, reason: String },

	#[error(
        "Failed to create destination directory\n  Path: {path}\n  Reason: {reason}\n  Hint: Check if parent directory exists and is writable"
    )]
	CreateDestinationDir { path: String, reason: String },

	#[error("Rsync failed\n  Source: {source_path}\n  Destination: {dest_path}\n  Error: {error}")]
	RsyncFailed {
		source_path: String,
		dest_path: String,
		error: String,
	},

	#[error(
        "Insufficient disk space\n  Destination: {dest_path}\n  Required: {required}\n  Available: {available}\n  Hint: Free up space or choose a different destination"
    )]
	InsufficientSpace {
		dest_path: String,
		required: String,
		available: String,
	},

	#[error(
		"Unsafe backup operation detected\n  Summary: {summary}\n  Hint: Review the dry-run output before proceeding"
	)]
	UnsafeOperation { summary: String },

	#[error(
		"Backup already in progress for some folders\n  Hint: Run 'nix-ders backup monitor' to see active backups"
	)]
	BackupInProgress,

	#[error("SSH transfer failed\n  Destination: {dest}\n  Error: {error}")]
	SshTransferFailed { dest: String, error: String },

	#[error("Archive creation failed\n  Source: {source_path}\n  Destination: {dest_path}\n  Error: {error}")]
	ArchiveCreationFailed {
		source_path: String,
		dest_path: String,
		error: String,
	},

	#[error("Git repository not ready for backup\n  Path: {path}\n  Issues: {issues}")]
	GitNotReady { path: String, issues: String },

	#[error(
        "Source path overlaps with another backup\n  Path 1: {path1}\n  Path 2: {path2}\n  Hint: Each source path must be independent"
    )]
	SourceOverlap { path1: String, path2: String },

	#[error(
        "Folder has no destinations configured\n  Source: {source_path}\n  Config: {config}\n  Hint: Add at least one destination to the folder config"
    )]
	NoDestinations { source_path: String, config: String },
}

/// Git verification errors
#[derive(Error, Debug)]
pub enum GitError {
	#[error("Failed to run git command\n  Command: {command}\n  Error: {error}")]
	GitCommandFailed { command: String, error: String },

	#[error("Path is not a git repository\n  Path: {path}")]
	NotGitRepo { path: String },

	#[error("Git repository has uncommitted changes\n  Path: {path}\n  Changes: {count} file(s) modified")]
	UncommittedChanges { path: String, count: usize },

	#[error("Git repository has unpushed commits\n  Path: {path}\n  Commits ahead: {ahead}")]
	UnpushedCommits { path: String, ahead: usize },
}
