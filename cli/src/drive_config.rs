//! Drive configuration and management
//!
//! Drives are backup destinations configured in drives.toml.
//! Each drive has a unique name and type-specific configuration.

use anyhow::{Context, Result};
use serde::Deserialize;

/// Wrapper struct for drives.toml
#[derive(Debug, Clone, Deserialize)]
pub struct DrivesConfig {
	pub drives: Vec<DriveConfig>,
}

/// A configured backup drive
#[derive(Debug, Clone, Deserialize)]
pub struct DriveConfig {
	/// Unique name for this drive (referenced in folder configs)
	pub name: String,

	/// Type of drive
	#[serde(rename = "type")]
	pub drive_type: DriveType,

	/// For local drives: mount point path
	#[serde(default)]
	pub mount_point: Option<String>,

	/// For SSH drives: remote host
	#[serde(default)]
	pub host: Option<String>,

	/// For SSH drives: remote user
	#[serde(default)]
	pub user: Option<String>,

	/// For SSH drives: remote path
	#[serde(default)]
	pub path: Option<String>,

	/// For SSH drives: SSH identity file
	#[serde(default)]
	pub identity_file: Option<String>,

	/// For SSH drives: proxy command (e.g., for Cloudflare tunnel)
	#[serde(default)]
	pub proxy_command: Option<String>,

	/// For SSH drives: connection timeout in seconds
	#[serde(default)]
	pub connect_timeout: Option<u32>,

	/// For SSH drives: bandwidth limit in KB/s (0 = unlimited)
	#[serde(default)]
	pub bwlimit: Option<u32>,

	/// For SSH drives: port number
	#[serde(default)]
	pub port: Option<u16>,

	/// Check if drive is mounted before backup (for local drives)
	#[serde(default = "default_check_mounted")]
	pub check_mounted: bool,
}

fn default_check_mounted() -> bool {
	true
}

/// Type of backup drive
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DriveType {
	/// Local drive (USB, external HDD, etc.)
	Local,
	/// Remote server via SSH
	Ssh,
}

impl DriveConfig {
	/// Get the full path for a destination on this drive
	pub fn get_destination_path(&self, dest_path: &str) -> String {
		match self.drive_type {
			DriveType::Local => self
				.mount_point
				.as_ref()
				.map(|m| format!("{}/{}", m, dest_path))
				.unwrap_or_else(|| dest_path.to_string()),
			DriveType::Ssh => {
				let user = self.user.as_deref().unwrap_or("user");
				let host = self.host.as_deref().unwrap_or("localhost");
				let path = self.path.as_deref().unwrap_or("");

				// Don't include port in destination - it's handled by SSH -p flag
				format!("{}@{}:{}/{}", user, host, path, dest_path)
			}
		}
	}

	/// Check if this drive is a local drive
	pub fn is_local(&self) -> bool {
		matches!(self.drive_type, DriveType::Local)
	}

	/// Check if this drive is an SSH drive
	pub fn is_ssh(&self) -> bool {
		matches!(self.drive_type, DriveType::Ssh)
	}
}

/// Load drive configurations
pub fn load_drive_configs() -> Result<Vec<DriveConfig>> {
	use std::fs;

	let config_path = crate::config::config_dir().join("drives.toml");

	if !config_path.exists() {
		anyhow::bail!("Drive configuration file not found: {}", config_path.display());
	}

	let content = fs::read_to_string(&config_path)
		.with_context(|| format!("Failed to read drive config: {}", config_path.display()))?;

	let config: DrivesConfig =
		toml::from_str(&content).with_context(|| format!("Failed to parse drive config: {}", config_path.display()))?;

	let drives = config.drives;

	if drives.is_empty() {
		anyhow::bail!("No drives configured in {}", config_path.display());
	}

	// Validate drive names are unique
	let mut names = std::collections::HashSet::new();
	for drive in &drives {
		if !names.insert(&drive.name) {
			anyhow::bail!("Duplicate drive name: {}", drive.name);
		}
	}

	Ok(drives)
}

/// Get a drive configuration by name
pub fn get_drive_by_name<'a>(drives: &'a [DriveConfig], name: &str) -> Result<&'a DriveConfig> {
	drives
		.iter()
		.find(|d| d.name == name)
		.with_context(|| format!("Drive not found: {}", name))
}
