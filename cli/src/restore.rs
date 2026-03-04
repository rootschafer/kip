//! Restore operations - recover files from backup
//!
//! Restores from a specific destination (flash or server) back to the original source location.

use std::{path::PathBuf, process::Stdio, sync::atomic::Ordering};

use anyhow::{Context, Result};
use console::style;
use tracing::{error, info, warn};

use crate::{
	config,
	drive_config::{get_drive_by_name, DriveConfig, DriveType},
	folder::Folder,
	progress::{format_bytes, BackupProgress},
	state::StateManager,
};

/// Restore from backup
pub async fn run_restore(from: &str, filter: Option<String>, limit: Option<usize>, dry_run: bool) -> Result<()> {
	run_restore_with_progress(from, filter, limit, dry_run, None).await
}

/// Restore with progress tracking
pub async fn run_restore_with_progress(
	from: &str,
	filter: Option<String>,
	limit: Option<usize>,
	dry_run: bool,
	progress: Option<BackupProgress>,
) -> Result<()> {
	// Load unified configuration
	let main_config = config::load_main_config().context("Failed to load configurations")?;

	let drives = &main_config.drives;

	// Load app configs from apps/ subdirectory
	let app_configs = config::load_app_configs().context("Failed to load app configurations")?;

	// Resolve all folders
	let mut all_folders = Vec::new();

	for (config_name, config) in &app_configs {
		if let Some(ref f) = filter {
			if !config_name.contains(f) && !config.metadata.name.contains(f) {
				continue;
			}
		}

		for folder_config in &config.folder_configs {
			if let Ok(folder) = Folder::from_config(folder_config, config_name, config.metadata.priority) {
				// Filter by drive name
				let has_matching_dest = folder.destinations.iter().any(|d| d.drive == from);

				if has_matching_dest {
					all_folders.push(folder);
				}
			}
		}
	}

	// Sort by priority (descending) - restore critical stuff first
	all_folders.sort_by(|a, b| b.priority.cmp(&a.priority));

	// Initialize state manager
	let state = StateManager::new(main_config.settings.state_file.map(|s| s.into()))?;

	// Filter to folders that have been backed up
	let mut pending_folders: Vec<&Folder> = all_folders
		.iter()
		.filter(|f| {
			let folder_state = state.get_folder_state(&f.id);
			// Check if any destination for this drive is complete
			f.destinations.iter().filter(|d| d.drive == from).any(|d| {
				folder_state
					.and_then(|s| s.destinations.get(&d.path))
					.map(|ds| ds.completed)
					.unwrap_or(false)
			})
		})
		.collect();

	// Apply limit if specified
	if let Some(l) = limit {
		pending_folders.truncate(l);
	}

	if pending_folders.is_empty() {
		warn!("No folders found to restore from {}", from);
		println!("\n{}", style("⚠️  No folders found to restore").bold().yellow());
		println!("   Source: {}", from);
		if let Some(f) = &filter {
			println!("   Filter: {}", f);
		}
		println!("\nRun '{}' to see available backups.", style("nix-ders backup check").bold());
		return Ok(());
	}

	// Create progress tracker if not provided
	let progress = progress.unwrap_or_else(|| BackupProgress::new(pending_folders.len()));
	let total_folders = pending_folders.len();

	if dry_run {
		println!("\n{}", style("🔍 RESTORE DRY-RUN PREVIEW").bold());
		println!("{}", "=".repeat(60));
		println!(
			"Source: {} ({})",
			from,
			if from == "flash" {
				"/Volumes/SOMETHING/mac_emergency_backup"
			} else {
				"ssh.anders.place:/mnt/usb2tb/mac_emergency_backup"
			}
		);
		println!("{}", "=".repeat(60));
	} else {
		println!("\n{}", style("🔄 STARTING RESTORE").bold());
		println!("{}", "=".repeat(60));
		println!(
			"Source: {} ({})",
			from,
			if from == "flash" {
				"/Volumes/SOMETHING/mac_emergency_backup"
			} else {
				"ssh.anders.place:/mnt/usb2tb/mac_emergency_backup"
			}
		);
		println!("Folders to restore: {}", total_folders);
		println!("{}", "=".repeat(60));
	}

	// Restore each folder
	for (idx, folder) in pending_folders.iter().enumerate() {
		// Check for cancellation
		if progress.is_cancelled() {
			println!("\n{}", style("⚠️  Restore cancelled by user").bold().yellow());
			break;
		}

		// Update progress
		progress.set_current_folder(format!("{}: {}", folder.config_name, folder.source.display()));
		progress.current_folder.store(idx as u64, Ordering::Relaxed);

		// Find the matching destination for this drive
		let dest = folder.destinations.iter().find(|d| d.drive == from);

		if let Some(dest) = dest {
			// Resolve drive configuration
			let drive = match get_drive_by_name(&drives, &dest.drive) {
				Ok(d) => d,
				Err(e) => {
					error!("Failed to resolve drive '{}': {}", dest.drive, e);
					continue;
				}
			};

			if dry_run {
				println!(
					"\n{} [{}/{}] Would restore: {}",
					style("📁").bold(),
					idx + 1,
					total_folders,
					folder.source.display()
				);

				println!("   From: {} ({})", dest.path, dest.drive);
				println!("   To: {}", folder.source.display());

				if dest.zip {
					println!("   {} Would extract tarball", style("📦").bold());
				}

				continue;
			}

			println!(
				"\n{} [{}/{}] Restoring: {}",
				style("🔄").bold(),
				idx + 1,
				total_folders,
				folder.source.display()
			);

			match drive.drive_type {
				DriveType::Local => {
					let full_path = drive.get_destination_path(&dest.path);
					match restore_from_flash(folder, &full_path).await {
						Ok(bytes) => {
							info!("Restore complete: {} bytes", bytes);
							println!(
								"   {} Restored {} from {}",
								style("✅").bold().green(),
								format_bytes(bytes),
								dest.drive
							);
						}
						Err(e) => {
							error!("Restore failed: {}", e);
							println!("   {} Failed: {}", style("❌").bold().red(), e);
						}
					}
				}
				DriveType::Ssh => {
					let full_path = drive.get_destination_path(&dest.path);
					if dest.zip {
						match restore_from_server_zipped(folder, &full_path, &drive, &main_config.server).await {
							Ok(bytes) => {
								info!("Restore complete (zipped): {} bytes", bytes);
								println!(
									"   {} Restored {} from {} (extracted)",
									style("✅").bold().green(),
									format_bytes(bytes),
									dest.drive
								);
							}
							Err(e) => {
								error!("Restore failed: {}", e);
								println!("   {} Failed: {}", style("❌").bold().red(), e);
							}
						}
					} else {
						match restore_from_server_direct(folder, &full_path, &drive, &main_config.server).await {
							Ok(bytes) => {
								info!("Restore complete: {} bytes", bytes);
								println!(
									"   {} Restored {} from {}",
									style("✅").bold().green(),
									format_bytes(bytes),
									dest.drive
								);
							}
							Err(e) => {
								error!("Restore failed: {}", e);
								println!("   {} Failed: {}", style("❌").bold().red(), e);
							}
						}
					}
				}
			}
		}
	}

	if !dry_run {
		println!("\n{}", style("=".repeat(60)).bold());
		println!("{}", style("✅ Restore complete!").bold().green());
	}

	Ok(())
}

/// Restore a folder from flash drive
async fn restore_from_flash(folder: &Folder, source_path: &str) -> Result<u64> {
	let source = PathBuf::from(source_path);
	let dest = folder.source.clone();

	// Validate source exists
	if !source.exists() {
		anyhow::bail!("Backup source does not exist: {}", source.display());
	}

	// Ensure destination directory exists
	if let Some(parent) = dest.parent() {
		std::fs::create_dir_all(parent)
			.with_context(|| format!("Failed to create destination directory: {}", parent.display()))?;
	}

	// Build rsync command
	let mut cmd = tokio::process::Command::new("rsync");
	cmd.args(&["-av", "--progress", "--no-specials", "--no-devices"]);

	// Add excludes (same as backup)
	for exclude in &folder.excludes {
		cmd.arg("--exclude").arg(exclude);
	}

	cmd.arg(format!("{}/", source.display()))
		.arg(format!("{}/", dest.display()));

	info!("Running rsync restore: {:?}", cmd);

	let output = cmd
		.stderr(Stdio::piped())
		.stdout(Stdio::piped())
		.output()
		.await
		.context("Failed to execute rsync")?;

	if !output.status.success() {
		anyhow::bail!("rsync failed: {}", String::from_utf8_lossy(&output.stderr));
	}

	// Parse bytes transferred
	let bytes = parse_rsync_output(&output.stdout);

	Ok(bytes)
}

/// Restore a zipped folder from server
async fn restore_from_server_zipped(
	folder: &Folder,
	source_path: &str,
	drive: &DriveConfig,
	server_config: &crate::config::ServerConfig,
) -> Result<u64> {
	use std::process::Command as StdCommand;

	// Parse server path to get remote location
	let (_server_addr, _remote_path) = parse_server_path(source_path)?;

	let tarball_name = folder
		.source
		.file_name()
		.and_then(|s| s.to_str())
		.unwrap_or("backup");
	let temp_dir = std::env::temp_dir().join("backup-tool-restore");
	let temp_tarball = temp_dir.join(format!("{}.tar.gz", tarball_name));

	// Create temp directory
	std::fs::create_dir_all(&temp_dir)?;

	println!("   📥 Downloading from server...");

	// Download tarball from server using drive config
	let ssh_cmd = build_ssh_command(drive, server_config);
	let scp_cmd = format!("scp -e '{}' {} {}", ssh_cmd, source_path, temp_tarball.display());

	let scp_output = tokio::process::Command::new("bash")
		.arg("-c")
		.arg(&scp_cmd)
		.output()
		.await
		.context("Failed to download tarball from server")?;

	if !scp_output.status.success() {
		std::fs::remove_file(&temp_tarball).ok();
		anyhow::bail!("SCP failed: {}", String::from_utf8_lossy(&scp_output.stderr));
	}

	let tarball_size = std::fs::metadata(&temp_tarball)?.len();
	println!("   📦 Downloaded {} ({})", tarball_name, format_bytes(tarball_size));

	// Ensure destination directory exists
	let dest = folder.source.clone();
	if let Some(parent) = dest.parent() {
		std::fs::create_dir_all(parent)?;
	}

	println!("   📤 Extracting to {}...", dest.display());

	// Extract tarball
	let extract_output = StdCommand::new("tar")
		.args(&[
			"-xzf",
			temp_tarball.to_str().unwrap(),
			"-C",
			dest.parent()
				.unwrap_or(&PathBuf::from("."))
				.to_str()
				.unwrap(),
		])
		.output()
		.context("Failed to extract tarball")?;

	if !extract_output.status.success() {
		std::fs::remove_file(&temp_tarball).ok();
		anyhow::bail!("Tar extraction failed: {}", String::from_utf8_lossy(&extract_output.stderr));
	}

	// Clean up temp tarball
	std::fs::remove_file(&temp_tarball)?;

	Ok(tarball_size)
}

/// Restore directly from server (no tarball)
async fn restore_from_server_direct(
	folder: &Folder,
	source_path: &str,
	drive: &DriveConfig,
	server_config: &crate::config::ServerConfig,
) -> Result<u64> {
	// Build SSH command
	let ssh_cmd = build_ssh_command(drive, server_config);

	// Build rsync command
	let rsync_cmd = format!(
		"rsync -avP --progress --delete -e '{}' {} {}/",
		ssh_cmd,
		source_path,
		folder.source.display()
	);

	info!("Running rsync restore from server: {}", rsync_cmd);

	let output = tokio::process::Command::new("bash")
		.arg("-c")
		.arg(&rsync_cmd)
		.stderr(Stdio::piped())
		.stdout(Stdio::piped())
		.output()
		.await
		.context("Failed to execute rsync from server")?;

	if !output.status.success() {
		anyhow::bail!("rsync failed: {}", String::from_utf8_lossy(&output.stderr));
	}

	let bytes = parse_rsync_output(&output.stdout);

	Ok(bytes)
}

/// Parse server path (user@host:path)
fn parse_server_path(path: &str) -> Result<(String, String)> {
	if let Some(at_pos) = path.find('@') {
		let user_host = &path[..at_pos];
		let rest = &path[at_pos + 1..];

		if let Some(colon_pos) = rest.find(':') {
			let host = &rest[..colon_pos];
			let remote_path = &rest[colon_pos + 1..];
			Ok((format!("{}@{}", user_host, host), remote_path.to_string()))
		} else {
			anyhow::bail!("Invalid server path format: {}", path)
		}
	} else {
		anyhow::bail!("Invalid server path format (expected user@host:path): {}", path)
	}
}

/// Build SSH options string from server config
#[allow(dead_code)]
fn build_ssh_options(server_config: &crate::config::ServerConfig) -> String {
	let mut opts = String::new();

	// Identity file
	if let Some(ref key) = server_config.identity_file {
		let expanded = expand_tilde_path(key);
		opts.push_str(&format!("-i {} ", expanded.display()));
	}

	opts.push_str("-o IdentitiesOnly=yes ");
	opts.push_str("-o BatchMode=yes ");

	// Proxy command for Cloudflare tunnel
	if let Some(ref proxy) = server_config.proxy_command {
		opts.push_str(&format!("-o ProxyCommand={} ", proxy));
	}

	opts
}

/// Build SSH command for rsync -e flag
fn build_ssh_command(drive: &DriveConfig, server_config: &crate::config::ServerConfig) -> String {
	let mut ssh_cmd = String::from("ssh");

	// Identity file - prefer drive config, fall back to server_config
	let identity_file = drive
		.identity_file
		.as_ref()
		.or(server_config.identity_file.as_ref());

	if let Some(ref key) = identity_file {
		let expanded = expand_tilde_path(key);
		ssh_cmd.push_str(&format!(" -i {}", expanded.display()));
	}

	// Port - prefer drive config
	if let Some(port) = drive.port {
		ssh_cmd.push_str(&format!(" -p {}", port));
	}

	// Note: We don't use ProxyCommand here because Cloudflare Access needs
	// to trigger browser authentication.
	ssh_cmd.push_str(" -o IdentitiesOnly=yes");
	ssh_cmd.push_str(" -o BatchMode=no"); // Allow interactive auth

	ssh_cmd
}

/// Expand tilde in a path
fn expand_tilde_path(path: &str) -> PathBuf {
	if path.starts_with('~') {
		if let Some(home) = dirs::home_dir() {
			return home.join(path.trim_start_matches('~').trim_start_matches('/'));
		}
	}
	PathBuf::from(path)
}

/// Parse rsync output to extract bytes transferred
fn parse_rsync_output(output: &[u8]) -> u64 {
	let output_str = String::from_utf8_lossy(output);
	let mut total_bytes: u64 = 0;

	for line in output_str.lines() {
		if let Some(bytes_str) = line.split_whitespace().next() {
			if let Ok(bytes) = bytes_str.parse::<u64>() {
				if bytes > 0 && bytes < u64::MAX / 2 {
					total_bytes = total_bytes.max(bytes);
				}
			}
		}
	}

	total_bytes
}
