//! Folder configuration and validation
//!
//! Architecture: Each folder has ONE source and MULTIPLE destinations.
//! Destinations reference drives by name (configured in drives.toml).
//! This enforces that no source overlaps with another.

use std::path::PathBuf;

use anyhow::Result;
use serde::Deserialize;

/// A single backup destination
#[derive(Debug, Clone, Deserialize)]
pub struct Destination {
	/// Name of the drive (must match a drive in drives.toml)
	pub drive: String,

	/// Path relative to the drive root
	pub path: String,

	/// Whether to zip before transferring (for remote destinations)
	#[serde(default)]
	pub zip: bool,
}

/// A folder to be backed up
#[derive(Debug, Clone, Deserialize)]
pub struct FolderConfig {
	/// Source path on the local filesystem
	pub source: PathBuf,

	/// Priority for ordering (1-1000, higher = first). Defaults to config metadata priority.
	#[serde(default)]
	pub priority: Option<u16>,

	/// Rsync-style exclude patterns
	#[serde(default)]
	pub excludes: Vec<String>,

	/// List of destinations for this folder (at least one required)
	pub destinations: Vec<Destination>,
}

/// A resolved folder with all defaults filled in
#[derive(Debug, Clone)]
pub struct Folder {
	/// Source path on the local filesystem
	pub source: PathBuf,

	/// Priority for ordering (1-1000, higher = first)
	pub priority: u16,

	/// Rsync-style exclude patterns
	pub excludes: Vec<String>,

	/// List of destinations for this folder
	pub destinations: Vec<Destination>,

	/// Name of the config file this came from
	pub config_name: String,

	/// Unique ID for state tracking (based on source)
	pub id: String,
}

impl Folder {
	/// Create a resolved Folder from a FolderConfig
	pub fn from_config(config: &FolderConfig, app_name: &str, app_priority: u16) -> Result<Folder> {
		// Require at least one destination
		if config.destinations.is_empty() {
			anyhow::bail!(
				"Folder '{}' has no destinations specified. Must specify at least one destination.",
				config.source.display()
			);
		}

		let source_file_name = config
			.source
			.file_name()
			.and_then(|n| n.to_str())
			.unwrap_or("unknown");
		let id = format!("{}:{}", app_name, source_file_name);

		// Start with default excludes, then add config-specific ones
		let mut excludes = Self::default_excludes();
		excludes.extend(config.excludes.clone());

		Ok(Folder {
			source: expand_tilde(&config.source),
			priority: config.priority.unwrap_or(app_priority),
			excludes,
			destinations: config.destinations.clone(),
			config_name: app_name.to_string(),
			id,
		})
	}

	/// Default exclude patterns applied to all backups
	fn default_excludes() -> Vec<String> {
		vec![
			"target".to_string(), // Cargo/Rust build output
			"target/".to_string(),
			"node_modules".to_string(), // Node.js dependencies
			"node_modules/".to_string(),
			".git".to_string(), // Git metadata (already backed up separately)
			".git/".to_string(),
			"__pycache__".to_string(), // Python cache
			"__pycache__/".to_string(),
			"*.pyc".to_string(), // Python bytecode
			"*.pyo".to_string(),
			".DS_Store".to_string(), // macOS metadata
			"Thumbs.db".to_string(), // Windows thumbnails
		]
	}

	/// Check if source exists
	pub fn source_exists(&self) -> bool {
		self.source.exists()
	}

	/// Get exclude arguments for rsync
	pub fn rsync_excludes(&self) -> Vec<String> {
		self.excludes
			.iter()
			.flat_map(|e| vec!["--exclude".to_string(), e.clone()])
			.collect()
	}
}

/// Expand tilde in paths
pub fn expand_tilde(path: &PathBuf) -> PathBuf {
	let path_str = path.to_string_lossy();
	if path_str.starts_with('~') {
		if let Some(home) = dirs::home_dir() {
			return home.join(path_str.trim_start_matches('~').trim_start_matches('/'));
		}
	}
	path.clone()
}

/// Validate that no source paths overlap (one contains another)
pub fn validate_no_source_overlaps(folders: &[Folder]) -> Result<()> {
	for (i, f1) in folders.iter().enumerate() {
		for f2 in folders.iter().skip(i + 1) {
			// Check if f1 source is inside f2 source
			if f1.source.starts_with(&f2.source) {
				anyhow::bail!(
					"Source path overlap detected:\n  '{}' is inside '{}'\n\n\
                     Each source path must be independent (no folder can contain another).",
					f1.source.display(),
					f2.source.display()
				);
			}
			// Check if f2 source is inside f1 source
			if f2.source.starts_with(&f1.source) {
				anyhow::bail!(
					"Source path overlap detected:\n  '{}' is inside '{}'\n\n\
                     Each source path must be independent (no folder can contain another).",
					f2.source.display(),
					f1.source.display()
				);
			}
		}
	}
	Ok(())
}

/// Validate that all sources exist (warning only, doesn't fail)
pub fn validate_sources_exist(folders: &[Folder]) -> Vec<String> {
	folders
		.iter()
		.filter(|f| !f.source_exists())
		.map(|f| format!("Source path does not exist: {} (config: {})", f.source.display(), f.config_name))
		.collect()
}

/// Validate that all folders have at least one destination
pub fn validate_destinations(folders: &[Folder]) -> Result<()> {
	for folder in folders {
		if folder.destinations.is_empty() {
			anyhow::bail!(
				"Folder '{}' has no destinations. Each folder must have at least one destination.",
				folder.source.display()
			);
		}
	}
	Ok(())
}

/// Validate that all drive references exist
pub fn validate_drive_references(folders: &[Folder], drive_names: &[&str]) -> Result<()> {
	for folder in folders {
		for dest in &folder.destinations {
			if !drive_names.contains(&dest.drive.as_str()) {
				anyhow::bail!(
					"Folder '{}' references unknown drive '{}'. Available drives: {}",
					folder.source.display(),
					dest.drive,
					drive_names.join(", ")
				);
			}
		}
	}
	Ok(())
}
