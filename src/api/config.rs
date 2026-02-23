//! Config import/export API

use std::path::PathBuf;

use serde::Deserialize;

use crate::{
	api::{ConfigFormat, ConfigImportError, ImportResult, KipError},
	db::DbHandle,
};

/// Import backup-tool configuration
pub async fn import_backup_tool_config(db: &DbHandle, config_dir: Option<PathBuf>) -> Result<ImportResult, KipError> {
	let config_dir = config_dir.unwrap_or_else(default_backup_tool_config_dir);
	let mut result = ImportResult {
		locations_created: 0,
		intents_created: 0,
		errors: vec![],
	};

	// Check if config directory exists
	if !config_dir.exists() {
		return Err(KipError::ConfigImport(format!(
			"Config directory does not exist: {}",
			config_dir.display()
		)));
	}

	// Load drives.toml
	let drives_path = config_dir.join("drives.toml");
	let drives = if drives_path.exists() {
		match load_drives_config(&drives_path) {
			Ok(d) => d,
			Err(e) => {
				result
					.errors
					.push(ConfigImportError { file: drives_path.clone(), reason: e.to_string() });
				vec![]
			}
		}
	} else {
		vec![]
	};

	// Create drive locations
	for drive in &drives {
		if let Some(mount) = &drive.mount_point {
			let mount_path = PathBuf::from(mount);
			if mount_path.exists() {
				match crate::api::location::add_location(db, mount_path, Some(drive.name.clone()), None).await {
					Ok(_) => {
						result.locations_created += 1;
					}
					Err(e) => {
						result.errors.push(ConfigImportError {
							file: drives_path.clone(),
							reason: format!("Failed to add location for {}: {}", drive.name, e),
						});
					}
				}
			}
		}
	}

	// Load apps/*.toml
	let apps_dir = config_dir.join("apps");
	if apps_dir.exists() {
		for entry in std::fs::read_dir(&apps_dir)
			.map_err(|e| KipError::ConfigImport(format!("Failed to read apps directory: {}", e)))?
		{
			let entry = match entry {
				Ok(e) => e,
				Err(e) => {
					result.errors.push(ConfigImportError {
						file: apps_dir.clone(),
						reason: format!("Failed to read entry: {}", e),
					});
					continue;
				}
			};

			let path = entry.path();

			if path.extension().and_then(|s| s.to_str()) != Some("toml") {
				continue;
			}

			let app_config = match load_app_config(&path) {
				Ok(c) => c,
				Err(e) => {
					result
						.errors
						.push(ConfigImportError { file: path.clone(), reason: e.to_string() });
					continue;
				}
			};

			// Create source locations and intents
			for folder in &app_config.folder_configs {
				let source_path = expand_tilde(&folder.source);

				// Create source location
				let source_id = match crate::api::location::add_location(db, source_path.clone(), None, None).await {
					Ok(id) => {
						result.locations_created += 1;
						id
					}
					Err(e) => {
						result.errors.push(ConfigImportError {
							file: path.clone(),
							reason: format!("Failed to add source location: {}", e),
						});
						continue;
					}
				};

				// Create destination locations
				let mut dest_ids = vec![];
				for dest in &folder.destinations {
					// Find the drive
					let drive = drives.iter().find(|d| d.name == dest.drive);
					let dest_path = drive.and_then(|d| {
						d.mount_point
							.as_ref()
							.map(|m| PathBuf::from(m).join(&dest.path))
					});

					if let Some(p) = dest_path {
						match crate::api::location::add_location(db, p, None, None).await {
							Ok(id) => {
								dest_ids.push(id);
								result.locations_created += 1;
							}
							Err(e) => {
								result.errors.push(ConfigImportError {
									file: path.clone(),
									reason: format!("Failed to add dest location: {}", e),
								});
							}
						}
					}
				}

				// Create intent if we have destinations
				if !dest_ids.is_empty() {
					let config = crate::api::IntentConfig {
						name: Some(app_config.metadata.name.clone()),
						priority: app_config.metadata.priority,
						..Default::default()
					};

					match crate::api::intent::create_intent(db, source_id, dest_ids, config).await {
						Ok(_) => {
							result.intents_created += 1;
						}
						Err(e) => {
							result.errors.push(ConfigImportError {
								file: path.clone(),
								reason: format!("Failed to create intent: {}", e),
							});
						}
					}
				}
			}
		}
	}

	Ok(result)
}

/// Export current configuration
pub async fn export_config(_db: &DbHandle, _format: ConfigFormat, _output_dir: PathBuf) -> Result<(), KipError> {
	// TODO: Implement export
	Err(KipError::ConfigImport("Export not yet implemented".to_string()))
}

// ============================================================================
// Config file structures (matching backup-tool format)
// ============================================================================

#[derive(Debug, Deserialize)]
struct BackupConfig {
	#[serde(default)]
	drives: Vec<DriveConfig>,
	#[serde(default, rename = "folders")]
	folder_configs: Vec<FolderConfig>,
}

#[derive(Debug, Deserialize)]
struct AppConfig {
	metadata: Metadata,
	#[serde(default, rename = "folders")]
	folder_configs: Vec<FolderConfig>,
}

#[derive(Debug, Deserialize, Clone)]
struct DriveConfig {
	name: String,
	mount_point: Option<String>,
	#[serde(default)]
	max_file_size: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct Metadata {
	name: String,
	description: String,
	#[serde(default = "default_priority")]
	priority: u16,
}

fn default_priority() -> u16 {
	500
}

#[derive(Debug, Deserialize)]
struct FolderConfig {
	source: PathBuf,
	#[serde(default)]
	destinations: Vec<DestinationConfig>,
	#[serde(default)]
	priority: Option<u16>,
}

#[derive(Debug, Deserialize, Clone)]
struct DestinationConfig {
	drive: String,
	path: String,
}

// ============================================================================
// Helper functions
// ============================================================================

fn load_drives_config(path: &PathBuf) -> Result<Vec<DriveConfig>, KipError> {
	let content = std::fs::read_to_string(path)
		.map_err(|e| KipError::ConfigImport(format!("Failed to read drives config: {}", e)))?;

	let config: toml::Value = toml::from_str(&content)
		.map_err(|e| KipError::ConfigImport(format!("Failed to parse drives config: {}", e)))?;

	// Handle both [[drives]] and drives = [...] formats
	let drives = if let Some(drives_val) = config.get("drives") {
		if let Some(array) = drives_val.as_array() {
			array
				.iter()
				.filter_map(|v| {
					let table = v.as_table()?;
					let name = table.get("name")?.as_str()?.to_string();
					let mount_point = table
						.get("mount_point")
						.and_then(|v| v.as_str())
						.map(|s| s.to_string());
					let max_file_size = table
						.get("max_file_size")
						.and_then(|v| v.as_integer())
						.map(|i| i as u64);
					Some(DriveConfig { name, mount_point, max_file_size })
				})
				.collect()
		} else {
			vec![]
		}
	} else {
		vec![]
	};

	Ok(drives)
}

fn load_app_config(path: &PathBuf) -> Result<AppConfig, KipError> {
	let content = std::fs::read_to_string(path)
		.map_err(|e| KipError::ConfigImport(format!("Failed to read app config: {}", e)))?;

	let config: AppConfig =
		toml::from_str(&content).map_err(|e| KipError::ConfigImport(format!("Failed to parse app config: {}", e)))?;

	Ok(config)
}

fn default_backup_tool_config_dir() -> PathBuf {
	#[cfg(target_os = "macos")]
	{
		if let Some(home) = dirs::home_dir() {
			let config = home.join(".config").join("backup-tool");
			if config.exists() {
				return config;
			}
		}
	}

	dirs::config_dir()
		.unwrap_or_else(|| PathBuf::from("."))
		.join("backup-tool")
}

fn expand_tilde(path: &PathBuf) -> PathBuf {
	if path.starts_with("~") {
		let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
		let mut result = home;
		result.push(path.strip_prefix("~").unwrap_or(path));
		result
	} else {
		path.clone()
	}
}
