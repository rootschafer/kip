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

	/// For Cloud drives: rclone remote name (e.g., "gdrive", "nextcloud")
	#[serde(default)]
	pub rclone_remote: Option<String>,

	/// For Cloud drives: path within the rclone remote
	#[serde(default)]
	pub rclone_path: Option<String>,

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
	/// Cloud storage via rclone (Google Drive, Nextcloud, S3, etc.)
	Cloud,
}

impl DriveConfig {
	/// Get the full path for a destination on this drive.
	///
	/// Every field consulted here identifies *where data gets written*, so a
	/// missing one is a hard error rather than a guess. Silently falling back to
	/// something like `user@localhost:/` would send a backup to the wrong place
	/// and report success.
	pub fn get_destination_path(&self, dest_path: &str) -> Result<String> {
		Ok(match self.drive_type {
			DriveType::Local => {
				let mount = self.require("mount_point", self.mount_point.as_deref(), "the drive's mount point")?;
				format!("{}/{}", mount.trim_end_matches('/'), dest_path)
			}
			DriveType::Ssh => {
				let user = self.require("user", self.user.as_deref(), "the SSH login name")?;
				let host = self.require("host", self.host.as_deref(), "the server's hostname")?;
				let path = self.require("path", self.path.as_deref(), "the backup root on the server")?;

				// Don't include port in destination - it's handled by SSH -p flag
				format!("{}@{}:{}/{}", user, host, path.trim_end_matches('/'), dest_path)
			}
			DriveType::Cloud => {
				let remote = self.require(
					"rclone_remote",
					self.rclone_remote.as_deref(),
					"an rclone remote name (see `rclone listremotes`)",
				)?;
				let rpath = self.rclone_path.as_deref().unwrap_or("").trim_matches('/');
				if rpath.is_empty() {
					format!("{}:{}", remote, dest_path)
				} else {
					format!("{}:{}/{}", remote, rpath, dest_path)
				}
			}
		})
	}

	/// The login name for connecting to this SSH drive.
	pub fn ssh_user(&self) -> Result<&str> {
		self.require("user", self.user.as_deref(), "the SSH login name")
	}

	/// The `user@host` string for connecting to this SSH drive.
	pub fn ssh_target(&self) -> Result<String> {
		let user = self.ssh_user()?;
		let host = self.require("host", self.host.as_deref(), "the server's hostname")?;
		Ok(format!("{}@{}", user, host))
	}

	/// The rclone remote name backing this cloud drive.
	pub fn require_rclone_remote(&self) -> Result<&str> {
		self.require(
			"rclone_remote",
			self.rclone_remote.as_deref(),
			"an rclone remote name (see `rclone listremotes`)",
		)
	}

	/// Human-readable description of this drive's backup root, for banners.
	pub fn describe_root(&self) -> String {
		match self.get_destination_path("") {
			Ok(root) => root.trim_end_matches('/').to_string(),
			Err(_) => format!("<{} drive '{}' is not fully configured>", self.type_name(), self.name),
		}
	}

	/// The drive type as it appears in `drives.toml`.
	pub fn type_name(&self) -> &'static str {
		match self.drive_type {
			DriveType::Local => "local",
			DriveType::Ssh => "ssh",
			DriveType::Cloud => "cloud",
		}
	}

	/// Return `value`, or an error naming the drive and the missing key.
	fn require<'a>(&self, field: &str, value: Option<&'a str>, expects: &str) -> Result<&'a str> {
		value.filter(|v| !v.trim().is_empty()).ok_or_else(|| {
			crate::error::BackupError::MissingConfigField {
				field: field.to_string(),
				config: format!("drives.toml [[drives]] name = \"{}\"", self.name),
				context: format!("resolving the destination path for {} drive '{}'", self.type_name(), self.name),
				hint: format!("Set `{}` on that drive to {}.", field, expects),
			}
			.into()
		})
	}

	/// Check if this drive is a local drive
	pub fn is_local(&self) -> bool {
		matches!(self.drive_type, DriveType::Local)
	}

	/// Check if this drive is an SSH drive
	pub fn is_ssh(&self) -> bool {
		matches!(self.drive_type, DriveType::Ssh)
	}

	/// Check if this drive is a cloud drive
	pub fn is_cloud(&self) -> bool {
		matches!(self.drive_type, DriveType::Cloud)
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
