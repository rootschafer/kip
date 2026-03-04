//! Configuration loading and management
//!
//! Flexible config system - scans all .toml files in config directory
//! and merges settings, drives, and app configs from any file.

use std::{
	fs,
	path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::{drive_config::DriveConfig, folder::FolderConfig};

/// Unified configuration - can be in any .toml file
#[derive(Debug, Deserialize, Default)]
pub struct BackupConfig {
	/// Global settings
	#[serde(default)]
	pub settings: Settings,

	/// Drive configurations (can be inline or in separate files)
	#[serde(default, rename = "drives")]
	pub drives: Vec<DriveConfig>,

	/// Server configuration (legacy, for backwards compatibility)
	#[serde(default)]
	pub server: ServerConfig,

	/// App metadata (for inline app configs)
	#[serde(default)]
	pub metadata: Option<Metadata>,

	/// Folder configs (for inline app configs)
	#[serde(default, rename = "folders")]
	pub folder_configs: Vec<FolderConfig>,
}

#[derive(Debug, Deserialize, Default)]
pub struct Settings {
	/// State file location
	pub state_file: Option<String>,

	/// Pipe rsync stdout to console (for verbose server backup output)
	#[serde(default)]
	pub pipe_rsync_stdout: bool,
}

#[derive(Debug, Deserialize, Default)]
pub struct ServerConfig {
	pub host: Option<String>,
	pub user: Option<String>,
	pub identity_file: Option<String>,
	pub proxy_command: Option<String>,
}

/// Application-specific folder configuration
#[derive(Debug, Deserialize)]
pub struct AppConfig {
	/// Metadata about this config
	pub metadata: Metadata,

	/// List of folders to backup
	#[serde(rename = "folders")]
	pub folder_configs: Vec<FolderConfig>,
}

#[derive(Debug, Deserialize)]
pub struct Metadata {
	/// Application/category name
	pub name: String,

	/// Description of what this backs up
	pub description: String,

	/// Default priority for folders in this config (1-1000)
	#[serde(default = "default_priority")]
	pub priority: u16,
}

fn default_priority() -> u16 {
	500
}

/// Load configuration by scanning all .toml files in config directory
pub fn load_main_config() -> Result<BackupConfig> {
	let config_dir = config_dir();
	let mut merged_config = BackupConfig::default();

	// Scan all .toml files in config directory (not subdirectories)
	if config_dir.exists() {
		for entry in fs::read_dir(&config_dir)
			.with_context(|| format!("Failed to read config directory: {}", config_dir.display()))?
		{
			let entry = entry?;
			let path = entry.path();

			// Skip directories and the apps subdirectory
			if !path.is_file() || path.file_name().and_then(|s| s.to_str()) == Some("apps") {
				continue;
			}

			// Only process .toml files
			if path.extension().and_then(|s| s.to_str()) != Some("toml") {
				continue;
			}

			// Skip git_repos.toml (handled separately)
			if path.file_stem().and_then(|s| s.to_str()) == Some("git_repos") {
				continue;
			}

			// Load and merge this config file
			match load_config_file::<BackupConfig>(&path) {
				Ok(config) => {
					// Merge settings (last one wins)
					if config.settings.state_file.is_some() {
						merged_config.settings.state_file = config.settings.state_file;
					}
					merged_config.settings.pipe_rsync_stdout = config.settings.pipe_rsync_stdout;

					// Merge drives
					merged_config.drives.extend(config.drives);

					// Merge server config (last one wins)
					if config.server.host.is_some() {
						merged_config.server = config.server;
					}

					// Merge inline app config if present
					if let Some(metadata) = config.metadata {
						merged_config.metadata = Some(metadata);
						merged_config.folder_configs = config.folder_configs;
					}
				}
				Err(e) => {
					// Non-critical files can fail to parse
					tracing::debug!("Skipping config file {}: {}", path.display(), e);
				}
			}
		}
	}

	// If no drives found, check for drives.toml specifically (backwards compat)
	if merged_config.drives.is_empty() {
		let drives_path = config_dir.join("drives.toml");
		if drives_path.exists() {
			if let Ok(drives_config) = load_config_file::<crate::drive_config::DrivesConfig>(&drives_path) {
				merged_config.drives = drives_config.drives;
			}
		}
	}

	Ok(merged_config)
}

/// Load all application configurations from the apps directory
pub fn load_app_configs() -> Result<Vec<(String, AppConfig)>> {
	let apps_dir = config_dir().join("apps");
	let mut configs = Vec::new();

	if !apps_dir.exists() {
		anyhow::bail!("Apps directory does not exist: {}", apps_dir.display());
	}

	for entry in
		fs::read_dir(&apps_dir).with_context(|| format!("Failed to read apps directory: {}", apps_dir.display()))?
	{
		let entry = entry?;
		let path = entry.path();

		if path.extension().and_then(|s| s.to_str()) == Some("toml") {
			// Skip non-folder configs (git_repos.toml, etc.)
			let config_name = path
				.file_stem()
				.and_then(|s| s.to_str())
				.unwrap_or("unknown")
				.to_string();

			if config_name == "git_repos" {
				continue; // Skip git repos config, handled separately
			}

			let config: AppConfig =
				load_config_file(&path).with_context(|| format!("Failed to load config: {}", path.display()))?;

			configs.push((config_name, config));
		}
	}

	Ok(configs)
}

/// Load a TOML configuration file
fn load_config_file<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
	let content =
		fs::read_to_string(path).with_context(|| format!("Failed to read config file: {}", path.display()))?;

	toml::from_str(&content).with_context(|| format!("Failed to parse TOML: {}", path.display()))
}

/// Get the configuration directory
pub fn config_dir() -> PathBuf {
	// On macOS, prefer ~/.config over ~/Library/Application Support for easier access
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

/// List all configured folders
pub fn list_folders(sort_by_priority: bool, filter: Option<&str>) -> Result<()> {
	let configs = load_app_configs()?;

	let mut all_folders = Vec::new();

	for (config_name, config) in &configs {
		if let Some(f) = filter {
			if !config_name.contains(f) && !config.metadata.name.contains(f) {
				continue;
			}
		}

		for folder in &config.folder_configs {
			all_folders.push((config_name, config, folder));
		}
	}

	if sort_by_priority {
		all_folders.sort_by(|a, b| {
			let priority_a = a.2.priority.unwrap_or(a.1.metadata.priority);
			let priority_b = b.2.priority.unwrap_or(b.1.metadata.priority);
			priority_b.cmp(&priority_a) // Descending order
		});
	}

	println!("{:<25} {:<30} {:<10} {}", "App", "Source", "Priority", "Destinations");
	println!("{}", "-".repeat(110));

	for (_config_name, config, folder) in &all_folders {
		let priority = folder.priority.unwrap_or(config.metadata.priority);
		let dests: Vec<String> = folder
			.destinations
			.iter()
			.map(|d| format!("{} → {}", d.drive, d.path))
			.collect();
		let dest_str = if dests.is_empty() {
			"(none)".to_string()
		} else {
			dests.join(", ")
		};

		println!(
			"{:<25} {:<30} {:<10} {}",
			config.metadata.name,
			folder.source.to_string_lossy(),
			priority,
			dest_str
		);
	}

	Ok(())
}

/// Load drive configurations (convenience wrapper)
pub fn load_drives() -> Result<Vec<DriveConfig>> {
	crate::drive_config::load_drive_configs()
}
