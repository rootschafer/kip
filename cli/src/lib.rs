//! Backup Tool Core Library
//!
//! A Rust library for managing emergency Mac backups with:
//! - TOML-based folder configurations
//! - Priority-based backup ordering
//! - State tracking (JSON file)
//! - rsync-based transfers to flash drive and SSH server
//! - Safety checks (dry-run, destination validation, empty source detection)
//! - Git repository verification
//! - Daemon lock management
//! - Disk space monitoring

pub mod backup;
pub mod check;
pub mod config;
pub mod daemon_lock;
pub mod db;
pub mod disk_space;
pub mod drive_config;
pub mod error;
pub mod folder;
pub mod git_verify;
pub mod progress;
pub mod restore;
pub mod safety;
pub mod state;
pub mod status;
pub mod validate;
pub mod zip;

// Re-export main types for convenience
pub use config::{AppConfig, BackupConfig, Metadata, ServerConfig, Settings};
pub use folder::{Destination, Folder, FolderConfig};
pub use drive_config::{get_drive_by_name, DriveConfig, DriveType};
pub use state::{DestinationState, FolderState, State, StateManager, StateStats};
pub use backup::{dry_run_backup, get_directory_size, monitor_backups, run_backup, run_backup_with_progress};
pub use restore::{run_restore, run_restore_with_progress};
pub use check::{check_backup_status, check_server_status};
pub use validate::validate_all;
pub use safety::{rsync_dry_run, validate_backup_destination, validate_backup_source, DryRunResult};
pub use progress::{format_bytes, BackupProgress};
pub use git_verify::{handle_git_repo_with_issues, is_git_repo, print_verification_results, verify_git_repo, GitRepoAction, GitVerificationResult};
pub use error::{BackupError, GitError};
