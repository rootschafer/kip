//! State tracking for backup operations
//!
//! Tracks completion status per destination (not per folder).
//! Each folder can have multiple destinations, each tracked independently.

use std::{collections::HashMap, fs, path::PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::info;

/// Expand tilde in a path (delegates to folder::expand_tilde)
fn expand_tilde_path(path: &PathBuf) -> PathBuf {
	crate::folder::expand_tilde(path)
}

/// Old state file structure (for migration)
#[derive(Debug, Serialize, Deserialize, Default)]
struct OldState {
	/// Last run timestamp
	#[serde(default)]
	last_run: Option<DateTime<Utc>>,

	/// State for each folder (old format)
	#[serde(default)]
	folders: HashMap<String, OldFolderState>,
}

/// Old folder state format
#[derive(Debug, Serialize, Deserialize, Clone)]
struct OldFolderState {
	/// Source path
	source: String,

	/// Destination paths (old format with flat structure)
	dest_flash: String,
	dest_server: String,

	/// Completion status per destination type
	flash_completed: bool,
	server_completed: bool,

	/// Last sync timestamps
	flash_last_sync: Option<DateTime<Utc>>,
	server_last_sync: Option<DateTime<Utc>>,

	/// Bytes transferred
	flash_bytes_transferred: u64,
	server_bytes_transferred: u64,
}

/// State file structure
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct State {
	/// Last run timestamp
	#[serde(default)]
	pub last_run: Option<DateTime<Utc>>,

	/// State for each folder
	#[serde(default)]
	pub folders: HashMap<String, FolderState>,
}

/// State for a single folder
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FolderState {
	/// Source path
	pub source: String,

	/// Destinations and their completion status
	#[serde(default)]
	pub destinations: HashMap<String, DestinationState>,
}

/// State for a single destination
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DestinationState {
	/// Whether backup to this destination is complete
	#[serde(default)]
	pub completed: bool,

	/// Last successful sync timestamp
	#[serde(default)]
	pub last_sync: Option<DateTime<Utc>>,

	/// Bytes transferred
	#[serde(default)]
	pub bytes_transferred: u64,
}

/// State manager
pub struct StateManager {
	state: State,
	state_file: PathBuf,
}

impl StateManager {
	/// Create a new state manager
	pub fn new(state_file: Option<PathBuf>) -> Result<Self> {
		let state_file = if let Some(path) = state_file {
			// Expand tilde if present
			expand_tilde_path(&path)
		} else {
			crate::config::config_dir().join("state.json")
		};

		let state = Self::load_state(&state_file).unwrap_or_default();

		Ok(Self { state, state_file })
	}

	/// Load state from file (with migration support)
	fn load_state(path: &PathBuf) -> Option<State> {
		if !path.exists() {
			return None;
		}

		let content = fs::read_to_string(path).ok()?;

		// Try to load as new format first
		if let Ok(state) = serde_json::from_str(&content) {
			return Some(state);
		}

		// Try to load as old format and migrate
		if let Ok(old_state) = serde_json::from_str::<OldState>(&content) {
			info!("Migrating state file from old format to new format");
			return Some(migrate_old_state(old_state));
		}

		// If both fail, return None to start fresh
		info!("State file format unrecognized, starting with fresh state");
		None
	}

	/// Save state to file
	pub fn save(&self) -> Result<()> {
		info!("Saving state to: {}", self.state_file.display());

		// Ensure directory exists
		if let Some(parent) = self.state_file.parent() {
			fs::create_dir_all(parent)
				.with_context(|| format!("Failed to create state directory: {}", parent.display()))?;
		}

		let content = serde_json::to_string_pretty(&self.state).context("Failed to serialize state")?;

		fs::write(&self.state_file, content)
			.with_context(|| format!("Failed to write state file: {}", self.state_file.display()))?;

		info!("State saved successfully");
		Ok(())
	}

	/// Get state for a folder
	pub fn get_folder_state(&self, folder_id: &str) -> Option<&FolderState> {
		self.state.folders.get(folder_id)
	}

	/// Get or create state for a folder
	pub fn get_or_create_folder_state(&mut self, folder_id: &str) -> &mut FolderState {
		use std::collections::hash_map::Entry;

		match self.state.folders.entry(folder_id.to_string()) {
			Entry::Vacant(entry) => entry.insert(FolderState {
				source: String::new(),
				destinations: HashMap::new(),
			}),
			Entry::Occupied(entry) => entry.into_mut(),
		}
	}

	/// Mark a destination as completed
	pub fn mark_destination_completed(&mut self, folder_id: &str, dest_path: &str, bytes: u64, source: &str) {
		let folder_state = self.get_or_create_folder_state(folder_id);
		folder_state.source = source.to_string();

		let dest_state = folder_state
			.destinations
			.entry(dest_path.to_string())
			.or_insert_with(|| DestinationState {
				completed: false,
				last_sync: None,
				bytes_transferred: 0,
			});

		dest_state.completed = true;
		dest_state.last_sync = Some(Utc::now());
		dest_state.bytes_transferred = bytes;
	}

	/// Update last run timestamp
	pub fn update_last_run(&mut self) {
		self.state.last_run = Some(Utc::now());
	}

	/// Get completion statistics
	pub fn get_stats(&self) -> StateStats {
		let mut total_destinations = 0;
		let mut complete_destinations = 0;

		for folder in self.state.folders.values() {
			for dest in folder.destinations.values() {
				total_destinations += 1;
				if dest.completed {
					complete_destinations += 1;
				}
			}
		}

		StateStats {
			total_folders: self.state.folders.len(),
			total_destinations,
			complete_destinations,
		}
	}
}

/// Migrate old state format to new format
fn migrate_old_state(old_state: OldState) -> State {
	let mut new_state = State {
		last_run: old_state.last_run,
		folders: HashMap::new(),
	};

	for (folder_id, old_folder) in old_state.folders {
		let mut destinations = HashMap::new();

		// Migrate flash destination if it exists
		if !old_folder.dest_flash.is_empty() {
			// Extract just the relative path from the full path
			let flash_path = old_folder
				.dest_flash
				.strip_prefix("/Volumes/SOMETHING/mac_emergency_backup/")
				.unwrap_or(&old_folder.dest_flash)
				.to_string();

			destinations.insert(
				flash_path,
				DestinationState {
					completed: old_folder.flash_completed,
					last_sync: old_folder.flash_last_sync,
					bytes_transferred: old_folder.flash_bytes_transferred,
				},
			);
		}

		// Migrate server destination if it exists
		if !old_folder.dest_server.is_empty() {
			destinations.insert(
				old_folder.dest_server.clone(),
				DestinationState {
					completed: old_folder.server_completed,
					last_sync: old_folder.server_last_sync,
					bytes_transferred: old_folder.server_bytes_transferred,
				},
			);
		}

		new_state
			.folders
			.insert(folder_id, FolderState { source: old_folder.source, destinations });
	}

	new_state
}

/// Backup statistics
#[derive(Debug, Default)]
pub struct StateStats {
	pub total_folders: usize,
	pub total_destinations: usize,
	pub complete_destinations: usize,
}

impl StateStats {
	pub fn percent_complete(&self) -> f64 {
		if self.total_destinations == 0 {
			return 0.0;
		}
		(self.complete_destinations as f64 / self.total_destinations as f64) * 100.0
	}
}

#[cfg(test)]
mod tests {
	use chrono::Utc;
	use tempfile::TempDir;

	use super::*;

	#[test]
	fn test_migrate_old_state_single_destination() {
		let old_state = OldState {
			last_run: Some(Utc::now()),
			folders: {
				let mut folders = HashMap::new();
				folders.insert(
					"test:folder".to_string(),
					OldFolderState {
						source: "/Users/test/source".to_string(),
						dest_flash: "/Volumes/SOMETHING/mac_emergency_backup/flash_dest".to_string(),
						dest_server: "server_dest".to_string(),
						flash_completed: true,
						server_completed: false,
						flash_last_sync: Some(Utc::now()),
						server_last_sync: None,
						flash_bytes_transferred: 1024,
						server_bytes_transferred: 0,
					},
				);
				folders
			},
		};

		let new_state = migrate_old_state(old_state);

		assert_eq!(new_state.folders.len(), 1);
		let folder = new_state.folders.get("test:folder").unwrap();
		assert_eq!(folder.source, "/Users/test/source");

		// Check flash destination
		let flash_dest = folder.destinations.get("flash_dest").unwrap();
		assert!(flash_dest.completed);
		assert_eq!(flash_dest.bytes_transferred, 1024);

		// Check server destination
		let server_dest = folder.destinations.get("server_dest").unwrap();
		assert!(!server_dest.completed);
		assert_eq!(server_dest.bytes_transferred, 0);
	}

	#[test]
	fn test_migrate_old_state_empty_server() {
		let old_state = OldState {
			last_run: None,
			folders: {
				let mut folders = HashMap::new();
				folders.insert(
					"test:folder".to_string(),
					OldFolderState {
						source: "/Users/test/source".to_string(),
						dest_flash: "/Volumes/SOMETHING/mac_emergency_backup/only_flash".to_string(),
						dest_server: "".to_string(),
						flash_completed: true,
						server_completed: false,
						flash_last_sync: None,
						server_last_sync: None,
						flash_bytes_transferred: 2048,
						server_bytes_transferred: 0,
					},
				);
				folders
			},
		};

		let new_state = migrate_old_state(old_state);

		assert_eq!(new_state.folders.len(), 1);
		let folder = new_state.folders.get("test:folder").unwrap();

		// Should only have flash destination
		assert_eq!(folder.destinations.len(), 1);
		assert!(folder.destinations.contains_key("only_flash"));
	}

	#[test]
	fn test_state_manager_save_and_load() {
		let temp_dir = TempDir::new().expect("Failed to create temp dir");
		let state_file = temp_dir.path().join("state.json");

		// Create and save state
		{
			let mut state_mgr = StateManager::new(Some(state_file.clone())).unwrap();
			state_mgr.mark_destination_completed("test:folder", "dest1", 1024, "/Users/test/source");
			state_mgr.save().unwrap();
		}

		// Load state
		{
			let state_mgr = StateManager::new(Some(state_file)).unwrap();
			let stats = state_mgr.get_stats();
			assert_eq!(stats.total_folders, 1);
			assert_eq!(stats.total_destinations, 1);
			assert_eq!(stats.complete_destinations, 1);
		}
	}

	#[test]
	fn test_state_stats_percent_complete() {
		let stats = StateStats {
			total_folders: 10,
			total_destinations: 20,
			complete_destinations: 15,
		};

		assert!((stats.percent_complete() - 75.0).abs() < f64::EPSILON);
	}

	#[test]
	fn test_state_stats_empty() {
		let stats = StateStats::default();
		assert_eq!(stats.percent_complete(), 0.0);
	}

	#[test]
	fn test_expand_tilde_path() {
		let home = dirs::home_dir().expect("Failed to get home dir");
		let path = PathBuf::from("~/test/path");

		let result = expand_tilde_path(&path);

		assert!(result.starts_with(&home));
		assert!(result.ends_with("test/path"));
	}

	#[test]
	fn test_expand_tilde_path_no_tilde() {
		let path = PathBuf::from("/absolute/path");
		let result = expand_tilde_path(&path);
		assert_eq!(result, path);
	}
}
