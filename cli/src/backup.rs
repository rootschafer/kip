//! Backup operations
//!
//! Provides backup functionality with support for:
//! - Multiple destinations per folder (configured in drives.toml)
//! - Local and SSH destinations
//! - Progress tracking and verbose output
//! - State persistence for resume capability

use std::{
	collections::HashSet,
	path::PathBuf,
	process::Stdio,
	sync::atomic::Ordering,
	time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use tracing::{error, info, warn};

use crate::{
	config,
	disk_space,
	drive_config::{get_drive_by_name, DriveConfig, DriveType},
	error::BackupError,
	folder::Folder,
	progress::{format_bytes, BackupProgress},
	safety,
	state::StateManager,
	zip,
};

/// Run initial SSH authentication check for a drive
/// This triggers Cloudflare Access browser auth if needed
fn run_ssh_auth_check(drive: &DriveConfig, server_config: &crate::config::ServerConfig) -> Result<bool> {
	let ssh_cmd = build_ssh_command_for_rsync(drive, server_config);

	// Build the full SSH connection string with user@host
	let user = drive.user.as_deref().unwrap_or("user");
	let host = drive.host.as_deref().unwrap_or("localhost");
	let target = format!("{}@{}", user, host);

	// Run a simple SSH command that will trigger auth but exit immediately
	// Using "true" as the remote command - it does nothing and exits successfully
	let auth_cmd = format!("{} {} 'true'", ssh_cmd, target);

	info!("Running SSH auth check for drive: {} to {}", drive.name, target);

	let output = std::process::Command::new("bash")
		.arg("-c")
		.arg(&auth_cmd)
		.stderr(std::process::Stdio::inherit())
		.stdout(std::process::Stdio::inherit())
		.output()
		.context("Failed to run SSH auth check")?;

	if output.status.success() {
		info!("SSH auth check successful for drive: {}", drive.name);
		Ok(true)
	} else {
		warn!("SSH auth check failed for drive: {}", drive.name);
		Ok(false)
	}
}

/// Authenticate with all remote drives before backup
/// Returns set of authenticated drive names
fn authenticate_drives(drives: &[DriveConfig], server_config: &crate::config::ServerConfig) -> Result<HashSet<String>> {
	let mut authenticated = HashSet::new();

	for drive in drives {
		if drive.is_local() {
			authenticated.insert(drive.name.clone());
			continue;
		}

		println!("   🔐 Authenticating with {}...", drive.name);
		if run_ssh_auth_check(drive, server_config)? {
			println!("   {} Connected to {}", style("✅").green(), drive.name);
			authenticated.insert(drive.name.clone());
		} else {
			println!("   {} Failed to connect to {}", style("❌").red(), drive.name);
			// Don't fail - just mark as not authenticated, individual backups will skip
		}
	}

	Ok(authenticated)
}

/// Run backup operation
pub async fn run_backup(filter: Option<String>, limit: Option<usize>) -> Result<()> {
	run_backup_with_progress(filter, limit, None, false).await
}

/// Run backup operation with progress tracking and optional verbose output
pub async fn run_backup_with_progress(
	filter: Option<String>,
	limit: Option<usize>,
	progress: Option<BackupProgress>,
	verbose: bool,
) -> Result<()> {
	// Load unified configuration (scans all .toml files)
	let main_config = config::load_main_config().map_err(|e| BackupError::ConfigLoad {
		config_type: "configurations".to_string(),
		path: String::new(),
		reason: e.to_string(),
	})?;

	let drives = &main_config.drives;
	let server_config = &main_config.server;
	let pipe_rsync_stdout = main_config.settings.pipe_rsync_stdout;

	// Load app configs from apps/ subdirectory
	let app_configs = config::load_app_configs().map_err(|e| BackupError::ConfigLoad {
		config_type: "app configurations".to_string(),
		path: String::new(),
		reason: e.to_string(),
	})?;

	// Resolve all folders from configs
	let mut all_folders = Vec::new();

	for (config_name, config) in &app_configs {
		if let Some(ref f) = filter {
			if !config_name.contains(f) && !config.metadata.name.contains(f) {
				continue;
			}
		}

		for folder_config in &config.folder_configs {
			match Folder::from_config(folder_config, config_name, config.metadata.priority) {
				Ok(folder) => {
					all_folders.push(folder);
				}
				Err(e) => {
					warn!("Failed to resolve folder config: {}", e);
				}
			}
		}
	}

	// Sort by priority (descending)
	all_folders.sort_by(|a, b| b.priority.cmp(&a.priority));

	// Initialize state manager (JSON — legacy)
	let mut state = StateManager::new(main_config.settings.state_file.map(|s| s.into()))?;

	// Initialize SurrealDB (shared with Kip) — optional, failures are non-fatal
	let surreal_db = match crate::db::init().await {
		Ok(handle) => {
			info!("SurrealDB connected — syncing drives");
			if let Err(e) = crate::db::sync_drives_to_db(&handle.db, drives).await {
				warn!("Failed to sync drives to SurrealDB: {}", e);
			}
			Some(handle)
		}
		Err(e) => {
			warn!("SurrealDB not available (continuing with state.json only): {}", e);
			None
		}
	};

	// Filter to incomplete folders only
	let mut pending_folders: Vec<&Folder> = all_folders
		.iter()
		.filter(|f| {
			let folder_state = state.get_folder_state(&f.id);
			// Check if any destination is incomplete
			f.destinations.iter().any(|dest| {
				folder_state
					.and_then(|s| s.destinations.get(&dest.path))
					.map(|d| !d.completed)
					.unwrap_or(true)
			})
		})
		.collect();

	// Apply limit if specified
	if let Some(l) = limit {
		pending_folders.truncate(l);
	}

	if pending_folders.is_empty() {
		info!("All folders are already backed up!");
		println!("\n{} All folders are already backed up!", style("✅").bold().green());
		return Ok(());
	}

	// Check for running rsync processes to prevent duplicates
	if has_running_rsync_for_folders(&pending_folders) {
		return Err(BackupError::BackupInProgress.into());
	}

	// Initialize status file for progress monitoring
	let mut backup_status = crate::status::BackupStatus {
		is_running: true,
		pid: Some(std::process::id()),
		started_at: Some(SystemTime::now().duration_since(UNIX_EPOCH).ok().unwrap_or_default().as_secs()),
		total_folders: Some(pending_folders.len() as u64),
		completed_folders: 0,
		current_folder: None,
		bytes_transferred: 0,
		total_bytes: None,
		last_updated: Some(SystemTime::now().duration_since(UNIX_EPOCH).ok().unwrap_or_default().as_secs()),
		errors: Vec::new(),
	};
	backup_status.save().ok(); // Ignore errors, status is non-critical

	info!("Found {} folders to backup", pending_folders.len());

	// Authenticate with all remote drives first
	println!("\n🔐 Authenticating with remote drives...");
	let authenticated_drives = authenticate_drives(&drives, server_config)?;
	println!();

	// Create progress tracker if not provided
	let progress = progress.unwrap_or_else(|| BackupProgress::new(pending_folders.len()));
	let total_folders = pending_folders.len();

	// Backup each folder
	for (idx, folder) in pending_folders.iter().enumerate() {
		// Check for cancellation
		if progress.is_cancelled() {
			println!("\n{}", style("⚠️  Backup cancelled by user").bold().yellow());
			break;
		}

		// Update progress
		let current_name = format!("{}: {}", folder.config_name, folder.source.display());
		progress.set_current_folder(current_name.clone());
		progress.current_folder.store(idx as u64, Ordering::Relaxed);
		
		// Update status file
		backup_status.current_folder = Some(current_name);
		backup_status.last_updated = Some(SystemTime::now().duration_since(UNIX_EPOCH).ok().unwrap_or_default().as_secs());
		backup_status.save().ok();

		info!("Backing up: {} (priority: {})", folder.source.display(), folder.priority);

		println!(
			"\n{} [{}/{}] Backing up: {}",
			style("📁").bold(),
			idx + 1,
			total_folders,
			folder.source.display()
		);

		// Backup to each destination
		for dest in &folder.destinations {
			// Resolve drive name to configuration
			let drive = match get_drive_by_name(&drives, &dest.drive) {
				Ok(d) => d,
				Err(e) => {
					error!("Failed to resolve drive '{}': {}", dest.drive, e);
					continue;
				}
			};

			// Check if we're authenticated with this drive
			if !authenticated_drives.contains(&drive.name) {
				error!("Not authenticated with drive '{}': {}", dest.drive, drive.name);
				println!("   {} Skipping {} - not authenticated", style("⚠️").yellow(), dest.drive);
				continue;
			}

			// Build full destination path
			let full_dest = drive.get_destination_path(&dest.path);

			// Check available disk space for local destinations
			if drive.drive_type == DriveType::Local {
				let source_size = disk_space::get_directory_size(&folder.source)
					.unwrap_or(0);
				
				if source_size > 0 {
					match disk_space::ensure_available_space(full_dest.as_ref(), source_size) {
						Ok(()) => {
							// Enough space - proceed
						}
						Err(e) => {
							warn!("Insufficient space for {}: {}", dest.path, e);
							println!("   {} Insufficient disk space", style("⚠️").yellow());
							println!("      Required: {}", disk_space::format_bytes(source_size));
							println!("      Available: {}", e.to_string()
								.split("Available: ").nth(1)
								.unwrap_or("unknown"));
							println!("   {} Skipping {}", style("⚠️").yellow(), dest.drive);
							continue;
						}
					}
				}
			}

			let bytes = match drive.drive_type {
				DriveType::Local => {
					if dest.zip {
						match backup_to_flash_zipped(folder, &full_dest, verbose).await {
							Ok(bytes) => bytes,
							Err(e) => {
								error!("Flash backup to {} skipped: {}", dest.path, e);
								println!("   {} Flash backup skipped: {}", style("⚠️").yellow(), e);
								continue; // Skip this destination, continue with others
							}
						}
					} else {
						match backup_to_flash(folder, &full_dest, verbose).await {
							Ok(bytes) => bytes,
							Err(e) => {
								error!("Flash backup to {} skipped: {}", dest.path, e);
								println!("   {} Flash backup skipped: {}", style("⚠️").yellow(), e);
								continue; // Skip this destination, continue with others
							}
						}
					}
				}
				DriveType::Ssh => {
					if dest.zip {
						match backup_to_server_zipped(
							folder, &full_dest, &drive, server_config, verbose, pipe_rsync_stdout,
						)
						.await
						{
							Ok(bytes) => bytes,
							Err(e) => {
								error!("Server backup to {} skipped: {}", dest.path, e);
								println!("   {} Server backup skipped: {}", style("⚠️").yellow(), e);
								continue; // Skip this destination, continue with others
							}
						}
					} else {
						match backup_to_server_direct(
							folder, &full_dest, &drive, server_config, verbose, pipe_rsync_stdout,
						)
						.await
						{
							Ok(bytes) => bytes,
							Err(e) => {
								error!("Server backup to {} skipped: {}", dest.path, e);
								println!("   {} Server backup skipped: {}", style("⚠️").yellow(), e);
								continue; // Skip this destination, continue with others
							}
						}
					}
				}
			};

			info!("Backup complete to {} ({}): {} bytes", dest.drive, dest.path, bytes);
			state.mark_destination_completed(&folder.id, &dest.path, bytes, &folder.source.to_string_lossy());
			state.save()?;

			// Also record in SurrealDB (non-fatal, silent on errors)
			if let Some(ref db_handle) = surreal_db {
				if let Err(e) = crate::db::record_backup_completion(
					&db_handle.db,
					&folder.source.to_string_lossy(),
					&full_dest,
					&dest.drive,
					bytes,
					drive.is_local(),
				)
				.await
				{
					tracing::debug!("SurrealDB backup record skipped: {}", e);
				}
			}
		}

		// Update status after each folder completes
		backup_status.completed_folders = (idx + 1) as u64;
		backup_status.current_folder = None;
		backup_status.last_updated = Some(SystemTime::now().duration_since(UNIX_EPOCH).ok().unwrap_or_default().as_secs());
		backup_status.save().ok();
	}

	state.update_last_run();
	state.save()?;

	// Clear status file when backup completes
	backup_status.is_running = false;
	backup_status.save().ok();

	info!("Backup run complete!");

	Ok(())
}

/// Run backup dry-run - preview changes without modifying anything
pub async fn dry_run_backup(filter: Option<String>, limit: Option<usize>) -> Result<()> {
	// Load unified configuration (scans all .toml files)
	let main_config = config::load_main_config().context("Failed to load configurations")?;

	let drives = &main_config.drives;

	// Load app configs from apps/ subdirectory
	let app_configs = config::load_app_configs().context("Failed to load app configurations")?;

	// Resolve all folders from configs
	let mut all_folders = Vec::new();

	for (config_name, config) in &app_configs {
		if let Some(ref f) = filter {
			if !config_name.contains(f) && !config.metadata.name.contains(f) {
				continue;
			}
		}

		for folder_config in &config.folder_configs {
			match Folder::from_config(folder_config, config_name, config.metadata.priority) {
				Ok(folder) => {
					all_folders.push(folder);
				}
				Err(e) => {
					warn!("Failed to resolve folder config: {}", e);
				}
			}
		}
	}

	// Sort by priority (descending)
	all_folders.sort_by(|a, b| b.priority.cmp(&a.priority));

	// Apply limit if specified
	if let Some(l) = limit {
		all_folders.truncate(l);
	}

	println!("\n{}", style("🔍 BACKUP DRY-RUN PREVIEW").bold());
	println!("{}", "=".repeat(60));
	println!("Configured drives:");
	for drive in drives {
		println!(
			"  - {} ({})",
			drive.name,
			match drive.drive_type {
				DriveType::Local => "local",
				DriveType::Ssh => "ssh",
			}
		);
	}
	println!("{}", "=".repeat(60));

	// Deduplicate by source
	let mut seen_sources = std::collections::HashSet::new();
	let mut unique_folders: Vec<&Folder> = Vec::new();
	for folder in &all_folders {
		if seen_sources.insert(folder.source.clone()) {
			unique_folders.push(folder);
		}
	}

	for folder in &unique_folders {
		println!(
			"\n{} [Priority {}] {}",
			style("📁").bold(),
			folder.priority,
			folder.source.display()
		);

		// Show all destinations
		for dest in &folder.destinations {
			println!("   → {}: {}", dest.drive, dest.path);

			// Validate source
			if !folder.source.exists() {
				println!("   {} Source does not exist", style("⚠️").bold().yellow());
				continue;
			}

			// Only run dry-run for local/flash destinations
			let is_local = drives
				.iter()
				.find(|d: &&DriveConfig| d.name == dest.drive)
				.map(|d| d.is_local())
				.unwrap_or(false);

			if is_local {
				let dest_path = PathBuf::from(&dest.path);

				// Run dry-run
				match safety::rsync_dry_run(&folder.source, &dest_path, &folder.excludes) {
					Ok(result) => {
						println!("      {}", result.summary());
						if result.files_to_delete > 0 {
							println!(
								"      {} Would delete {} files from destination",
								style("⚠️").bold().yellow(),
								result.files_to_delete
							);
						}
						if !result.is_safe() {
							println!(
								"      {} Operation appears UNSAFE - review recommended",
								style("❌").bold().red()
							);
						} else {
							println!("      {} Operation appears safe", style("✅").bold().green());
						}
					}
					Err(e) => {
						println!("      {} Dry-run failed: {}", style("❌").bold().red(), e);
					}
				}
			}
		}
	}

	println!("\n{}", "=".repeat(60));
	println!("This was a DRY-RUN. No files were modified.");
	println!("Run 'backup-tool run backup' to execute the backup.");

	Ok(())
}

/// Backup a folder to flash drive using rsync
async fn backup_to_flash(folder: &Folder, dest_path: &str, verbose: bool) -> Result<u64> {
	let dest = PathBuf::from(dest_path);

	// SAFETY: Validate destination is safe
	safety::validate_backup_destination(&dest).map_err(|e| BackupError::DestinationValidation {
		path: dest.display().to_string(),
		reason: format!("{} for {}", e, dest.display()),
	})?;

	// SAFETY: Validate source exists and is not empty
	safety::validate_backup_source(&folder.source)
		.map_err(|_e| BackupError::SourceNotFound { path: folder.source.display().to_string() })?;

	if !safety::check_source_not_empty(&folder.source)? {
		warn!(
			"Source directory is empty, skipping to prevent accidental deletion: {}",
			folder.source.display()
		);
		return Ok(0);
	}

	// SAFETY: Run dry-run first
	info!("Running rsync dry-run to preview changes...");
	let dry_run =
		safety::rsync_dry_run(&folder.source, &dest, &folder.excludes).context("Failed to run rsync dry-run")?;

	info!("Dry-run result: {}", dry_run.summary());

	if !dry_run.is_safe() {
		return Err(BackupError::UnsafeOperation { summary: dry_run.summary() }.into());
	}

	// Ensure destination directory exists
	// For zip files (.tar.gz), create parent directory, not the file itself
	let dest_dir = if dest.extension().map(|e| e == "gz").unwrap_or(false) {
		dest.parent().unwrap_or(&dest).to_path_buf()
	} else {
		dest.clone()
	};

	std::fs::create_dir_all(&dest_dir).map_err(|e| BackupError::CreateDestinationDir {
		path: dest_dir.display().to_string(),
		reason: e.to_string(),
	})?;

	// Build rsync command
	let mut cmd = tokio::process::Command::new("rsync");
	cmd.args(&[
		"-a",            // archive mode
		"--no-specials", // Skip special files (sockets, devices)
	]);

	// Add verbose flag if requested
	if verbose {
		cmd.arg("-v");
	}

	// Exclude socket files (can't be copied, cause mkstempsock errors)
	cmd.arg("--exclude").arg("*.sock");
	cmd.arg("--exclude").arg("agent*");
	cmd.arg("--exclude").arg("S.*"); // SSH agent sockets

	// Add excludes
	for exclude in &folder.excludes {
		cmd.arg("--exclude").arg(exclude);
	}

	cmd.arg(format!("{}/", folder.source.display()))
		.arg(format!("{}/", dest.display()));

	info!("Running rsync to {}", dest.display());

	if verbose {
		// Stream output directly to console
		let output = cmd
			.stderr(Stdio::inherit())
			.stdout(Stdio::inherit())
			.output()
			.await
			.map_err(|e| BackupError::RsyncFailed {
				source_path: folder.source.display().to_string(),
				dest_path: dest.display().to_string(),
				error: e.to_string(),
			})?;

		if !output.status.success() {
			let stderr = String::from_utf8_lossy(&output.stderr).to_string();
			
			// Check for out-of-space errors
			if stderr.contains("No space left on device") || stderr.contains("ENOSPC") {
				return Err(BackupError::InsufficientSpace {
					dest_path: dest.display().to_string(),
					required: "unknown".to_string(),
					available: "0 B".to_string(),
				}.into());
			}
			
			return Err(BackupError::RsyncFailed {
				source_path: folder.source.display().to_string(),
				dest_path: dest.display().to_string(),
				error: stderr,
			}
			.into());
		}

		// Get size after completion
		let bytes = get_directory_size(&folder.source);
		Ok(bytes)
	} else {
		// Use progress bar for non-verbose mode
		run_rsync_with_progress(cmd, &folder.source, &folder.source, &dest).await
	}
}

/// Run rsync with a progress indicator
///
/// Uses --progress to get real-time progress from rsync's stderr,
/// displayed via a spinner. Works with both macOS BSD rsync and GNU rsync.
async fn run_rsync_with_progress(
	mut cmd: tokio::process::Command,
	source: &PathBuf,
	src_path: &PathBuf,
	dest_path: &PathBuf,
) -> Result<u64> {
	use tokio::io::{AsyncBufReadExt, BufReader};
	use tokio::select;

	// Use --progress instead of --info=progress2 for macOS compatibility
	cmd.arg("--progress");

	// Spawn the process
	let mut child = cmd
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.spawn()
		.map_err(|e| BackupError::RsyncFailed {
			source_path: src_path.display().to_string(),
			dest_path: dest_path.display().to_string(),
			error: e.to_string(),
		})?;

	let source_name = source
		.file_name()
		.and_then(|n| n.to_str())
		.unwrap_or("files");

	// Create spinner
	let pb = ProgressBar::new_spinner();
	pb.set_style(
		ProgressStyle::default_spinner()
			.template("{spinner:.green} [{elapsed}] {msg}")?
			.tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
	);
	pb.set_message(format!("Syncing {}", source_name));
	pb.enable_steady_tick(std::time::Duration::from_millis(100));

	// Take stdout for reading progress
	let stdout = child.stdout.take();
	let mut last_progress = String::new();
	
	// Use select! to wait for either stdout lines OR child completion
	// This prevents hanging when rsync completes without producing progress output
	if let Some(stdout) = stdout {
		let mut reader = BufReader::new(stdout).lines();
		
		loop {
			select! {
				line_result = reader.next_line() => {
					match line_result {
						Ok(Some(line)) => {
							let trimmed = line.trim();
							// --progress lines look like: "filename (100%)" or byte counts
							// Just track that we're making progress
							if !trimmed.is_empty() {
								last_progress = trimmed.to_string();
								pb.set_message(format!("{}: {}", source_name, trimmed));
							}
						}
						Ok(None) | Err(_) => {
							// EOF or error - stop reading
							break;
						}
					}
				}
				status_result = child.wait() => {
					// Child completed - stop waiting for stdout
					let status = status_result.map_err(|e| BackupError::RsyncFailed {
						source_path: src_path.display().to_string(),
						dest_path: dest_path.display().to_string(),
						error: e.to_string(),
					})?;

					pb.finish_and_clear();

					if !status.success() {
						// Try to get stderr for better error message
						let error_msg = if let Some(mut stderr) = child.stderr {
							use tokio::io::AsyncReadExt;
							let mut buf = String::new();
							stderr.read_to_string(&mut buf).await.ok();
							
							// Check for out-of-space errors
							if buf.contains("No space left on device") || buf.contains("ENOSPC") {
								return Err(BackupError::InsufficientSpace {
									dest_path: dest_path.display().to_string(),
									required: "unknown".to_string(),
									available: "0 B".to_string(),
								}.into());
							}
							
							format!("rsync exited with {}: {}", status, buf)
						} else {
							format!("rsync exited with {}", status)
						};
						
						return Err(BackupError::RsyncFailed {
							source_path: src_path.display().to_string(),
							dest_path: dest_path.display().to_string(),
							error: error_msg,
						}.into());
					}
					
					// Parse bytes from last progress line or get directory size
					let bytes = if !last_progress.is_empty() {
						parse_rsync_progress_line(&last_progress)
					} else {
						get_directory_size(source)
					};
					println!("   {} Complete: {}", style("✅").green(), format_bytes(bytes));
					return Ok(bytes);
				}
			}
		}
	}
	
	// Fallback if no stdout was captured
	let status = child.wait().await.map_err(|e| BackupError::RsyncFailed {
		source_path: src_path.display().to_string(),
		dest_path: dest_path.display().to_string(),
		error: e.to_string(),
	})?;
	
	pb.finish_and_clear();
	
	if !status.success() {
		return Err(BackupError::RsyncFailed {
			source_path: src_path.display().to_string(),
			dest_path: dest_path.display().to_string(),
			error: format!("rsync exited with {}", status),
		}.into());
	}
	
	let bytes = get_directory_size(source);
	println!("   {} Complete: {}", style("✅").green(), format_bytes(bytes));
	Ok(bytes)
}

/// Parse a --info=progress2 line to extract bytes transferred
/// Format: "1,234,567 100%  1.23MB/s  0:00:01"
fn parse_rsync_progress_line(line: &str) -> u64 {
	line.split_whitespace()
		.next()
		.map(|s| s.replace(',', ""))
		.and_then(|s| s.parse::<u64>().ok())
		.unwrap_or(0)
}

/// Backup a folder to flash drive with zip (tar.gz)
async fn backup_to_flash_zipped(folder: &Folder, dest_path: &str, verbose: bool) -> Result<u64> {
	let dest = PathBuf::from(dest_path);

	// Create parent directory if needed
	if let Some(parent) = dest.parent() {
		std::fs::create_dir_all(parent).map_err(|e| BackupError::CreateDestinationDir {
			path: parent.display().to_string(),
			reason: e.to_string(),
		})?;
	}

	info!("Creating tar.gz: {}", dest.display());

	// Create a spinner for zip creation
	let pb = ProgressBar::new_spinner();
	pb.set_style(
		ProgressStyle::default_spinner()
			.template("{spinner:.green} Creating archive: {msg}")?
			.tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
	);
	let source_name = folder
		.source
		.file_name()
		.and_then(|n| n.to_str())
		.unwrap_or("files");
	pb.set_message(format!("{}", source_name));
	pb.enable_steady_tick(std::time::Duration::from_millis(80));

	// Create tar.gz archive
	let zip_result = zip::create_tar_gz(&folder.source, &dest, &folder.excludes);

	pb.finish_and_clear();

	zip_result.map_err(|e| BackupError::ArchiveCreationFailed {
		source_path: folder.source.display().to_string(),
		dest_path: dest.display().to_string(),
		error: e.to_string(),
	})?;

	let zip_size = std::fs::metadata(&dest)
		.map_err(|e| BackupError::CreateDestinationDir {
			path: dest.display().to_string(),
			reason: format!("Failed to get metadata: {}", e),
		})?
		.len();

	info!("Zip created: {} bytes", zip_size);

	if verbose {
		println!("   {} Archive created: {}", style("✅").green(), format_bytes(zip_size));
	} else {
		println!("   {} Archive created: {}", style("✅").green(), format_bytes(zip_size));
	}

	Ok(zip_size)
}

/// Get approximate size of a directory (for progress bar estimation)
pub fn get_directory_size(path: &PathBuf) -> u64 {
	let mut total = 0u64;

	if let Ok(entries) = std::fs::read_dir(path) {
		for entry in entries.flatten() {
			if let Ok(metadata) = entry.metadata() {
				if metadata.is_file() {
					total += metadata.len();
				} else if metadata.is_dir() {
					total += get_directory_size(&entry.path());
				}
			}
		}
	}

	total
}

/// Check if rsync is already running for any of the given folders
fn has_running_rsync_for_folders(folders: &[&Folder]) -> bool {
	use std::process::Command;

	let output = match Command::new("ps").args(&["auxwww"]).output() {
		Ok(o) => o,
		Err(_) => return false,
	};

	let ps_output = String::from_utf8_lossy(&output.stdout);

	for folder in folders {
		let source_str = folder.source.to_string_lossy();
		for line in ps_output.lines() {
			if (line.contains(" rsync ") || line.starts_with("rsync "))
				&& !line.contains("--server")
				&& !line.contains("grep")
			{
				if line.contains(&*source_str) {
					info!("rsync already running for {}", folder.source.display());
					return true;
				}
			}
		}
	}

	false
}

/// Monitor active backup operations - text-based, works in any terminal
pub fn monitor_backups() -> Result<()> {
	use std::process::Command;

	println!("\n{}", style("🔍 Active Backup Operations").bold());
	println!("{}", "=".repeat(70));

	let output = Command::new("ps")
		.args(&["auxwww"])
		.output()
		.context("Failed to run ps command")?;

	let ps_output = String::from_utf8_lossy(&output.stdout);

	let mut rsync_count = 0;

	for line in ps_output.lines() {
		if (line.contains(" rsync ") || line.starts_with("rsync "))
			&& !line.contains("grep")
			&& !line.contains("ps aux")
		{
			rsync_count += 1;

			let parts: Vec<&str> = line.split_whitespace().collect();
			if parts.len() > 10 {
				let pid = parts.get(1).unwrap_or(&"?");
				let cpu = parts.get(2).unwrap_or(&"?");

				let is_server = line.contains("--server");
				let server_label = if is_server { " (server helper)" } else { "" };

				let full_cmd = parts[10..].join(" ");
				let cmd_parts: Vec<&str> = full_cmd.split_whitespace().collect();
				let source = cmd_parts
					.get(cmd_parts.len().saturating_sub(2))
					.unwrap_or(&"?");
				let dest = cmd_parts.last().unwrap_or(&"?");

				let process_type = if is_server {
					style("🖥️").bold()
				} else {
					style("📦").bold()
				};

				println!(
					"{} PID {} | CPU {}% | {} → {}{}",
					process_type,
					style(pid).cyan(),
					style(cpu).yellow(),
					style(source).blue(),
					style(dest).green(),
					style(server_label).dim()
				);
			} else {
				println!("📦 {}", style(line).dim());
			}
		}
	}

	if rsync_count == 0 {
		println!("\n{} No active backup operations", style("✅").bold().green());
		println!("   Run 'nix-ders backup run backup' to start backing up");
	} else {
		println!("\n{} {} active rsync process(es)", style("🔄").bold().yellow(), rsync_count);
	}

	// Show disk space
	println!("\n{}", style("💾 Flash Drive Status").bold());
	println!("{}", "=".repeat(70));

	if let Ok(output) = Command::new("df")
		.args(&["-h", "/Volumes/SOMETHING"])
		.output()
	{
		let df_str = String::from_utf8_lossy(&output.stdout);
		for line in df_str.lines().skip(1) {
			let parts: Vec<&str> = line.split_whitespace().collect();
			if parts.len() >= 5 {
				println!(
					"   Used: {} | Available: {} | {}",
					style(parts[2]).cyan(),
					style(parts[3]).green(),
					style(parts[4]).yellow()
				);
			}
		}
	}

	println!();

	Ok(())
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

/// Backup a folder to server with zip (tar.gz)
async fn backup_to_server_zipped(
	folder: &Folder,
	dest_path: &str,
	drive: &DriveConfig,
	server_config: &crate::config::ServerConfig,
	verbose: bool,
	pipe_rsync_stdout: bool,
) -> Result<u64> {
	// Create temp directory for zip file
	let temp_dir = std::env::temp_dir().join("backup-tool");
	std::fs::create_dir_all(&temp_dir)?;

	// Create zip filename from source folder name
	let zip_name = folder
		.source
		.file_name()
		.and_then(|s| s.to_str())
		.unwrap_or("backup")
		.replace(' ', "_");
	let zip_name = format!("{}.tar.gz", zip_name);
	let zip_path = temp_dir.join(&zip_name);

	info!("Creating tar.gz: {}", zip_path.display());

	// Create tar.gz archive
	zip::create_tar_gz(&folder.source, &zip_path, &folder.excludes).map_err(|e| {
		BackupError::ArchiveCreationFailed {
			source_path: folder.source.display().to_string(),
			dest_path: zip_path.display().to_string(),
			error: e.to_string(),
		}
	})?;

	let zip_size = std::fs::metadata(&zip_path)?.len();
	info!("Zip created: {} bytes", zip_size);

	// Transfer zip to server destination
	info!("Transferring zip to server: {}", dest_path);
	let ssh_cmd = build_ssh_command_for_rsync(drive, server_config);
	let ssh_options = build_ssh_options(drive, server_config);
	
	// Build user@host target for SSH commands
	let user = drive.user.as_deref().unwrap_or("user");
	let host = drive.host.as_deref().unwrap_or("localhost");
	let target = format!("{}@{}", user, host);

	// Create parent directory on server first
	if let Some(parent_dir) = std::path::Path::new(dest_path).parent() {
		// Extract just the path part (after user@host:)
		let parent_dir_str = parent_dir.to_string_lossy();
		let remote_path = parent_dir_str.splitn(2, ':').last().unwrap_or(&parent_dir_str);
		
		let mkdir_cmd = format!("ssh{} {} 'mkdir -p {}'", ssh_options, target, remote_path);

		info!("Creating parent directory on server: {}", remote_path);
		info!("SSH mkdir command: {}", mkdir_cmd);
		let mkdir_output = std::process::Command::new("bash")
			.arg("-c")
			.arg(&mkdir_cmd)
			.output()
			.context("Failed to create parent directory on server")?;

		if !mkdir_output.status.success() {
			let stderr = String::from_utf8_lossy(&mkdir_output.stderr);
			let stdout = String::from_utf8_lossy(&mkdir_output.stdout);
			warn!("Failed to create parent directory: stderr={}, stdout={}", stderr, stdout);
			
			// Check for permission denied specifically
			if stderr.contains("Permission denied") {
				error!("Permission denied creating directory on server");
				println!("   {} Permission denied on server", style("❌").red());
				println!("      Cannot create: {}", parent_dir_str);
				println!();
				println!("      {} To fix this, run on the server:", style("🔧").bold());
				println!("      sudo mkdir -p {}", dest_path);
				println!("      sudo chown -R {} $(dirname {})", user, dest_path);
				println!();
				println!("      Or change the backup path in config.toml to a directory you own");
			} else {
				warn!("Failed to create parent directory: {}", stderr);
			}
			// Continue anyway - rsync might still work if directory already exists
		}
	}

	// Build rsync command — use rsync's --timeout (idle timeout) instead of wrapping with `timeout`
	let idle_timeout = drive.connect_timeout.unwrap_or(300);
	
	// dest_path is already in format user@host:/remote/path from get_destination_path()
	// Use it directly with rsync -e for SSH transfer
	let mut cmd = tokio::process::Command::new("rsync");
	cmd.args(&["-avP", "--no-specials"])
		.arg(format!("--timeout={}", idle_timeout))
		.arg("--exclude=*.sock")
		.arg("--exclude=agent*")
		.arg("--exclude=S.*")
		.arg("-e")
		.arg(&ssh_cmd)
		.arg(&zip_path)
		.arg(dest_path);

	// Load and set service tokens for cloudflared
	if let Some(creds) = load_cloudflare_credentials() {
		if let (Some(token_id), Some(token_secret)) = (creds.service_token_id, creds.service_token_secret) {
			cmd.env("CLOUDFLARED_SERVICE_TOKEN_ID", token_id)
				.env("CLOUDFLARED_SERVICE_TOKEN_SECRET", token_secret);
		}
	}

	// Pipe output if verbose OR if pipe_rsync_stdout is enabled
	if verbose || pipe_rsync_stdout {
		cmd.stderr(Stdio::inherit()).stdout(Stdio::inherit());
	} else {
		cmd.stderr(Stdio::piped()).stdout(Stdio::piped());
	}

	let output = cmd
		.output()
		.await
		.context("Failed to execute rsync to server")?;

	if !output.status.success() {
		std::fs::remove_file(&zip_path)?;

		let stderr = String::from_utf8_lossy(&output.stderr).to_string();
		let stdout = String::from_utf8_lossy(&output.stdout).to_string();

		// Check for permission denied
		if stderr.contains("Permission denied") || stderr.contains("permission denied") {
			// Extract just the path part (after the colon)
			let remote_path = dest_path.splitn(2, ':').last().unwrap_or(dest_path);
			
			return Err(BackupError::SshTransferFailed { 
				dest: dest_path.to_string(), 
				error: format!("{}\n\n{} Permission denied on server\n   Run these commands on the server to fix:\n   sudo mkdir -p {}\n   sudo chown -R {} $(dirname {})", 
					stderr.trim(), 
					style("🔧").bold(),
					remote_path, user, remote_path)
			}.into());
		}
		
		// Check for directory not found (parent directory doesn't exist)
		if stderr.contains("No such file or directory") || stderr.contains("does not exist") {
			// Extract just the path part (after the colon)
			let remote_path = dest_path.splitn(2, ':').last().unwrap_or(dest_path);
			
			return Err(BackupError::SshTransferFailed { 
				dest: dest_path.to_string(), 
				error: format!("{}\n\n{} Remote directory does not exist\n   The parent directory on the server could not be created.\n   Run these commands on the server to fix:\n   sudo mkdir -p {}\n   sudo chown -R {} $(dirname {})", 
					stderr.trim(), 
					style("🔧").bold(),
					remote_path, user, remote_path)
			}.into());
		}

		// Build informative error message
		let error_msg = if stderr.trim().is_empty() {
			if stdout.trim().is_empty() {
				"Command timed out or failed silently (check cloudflared authentication)".to_string()
			} else {
				format!("rsync output: {}", stdout.trim())
			}
		} else {
			stderr.trim().to_string()
		};

		return Err(BackupError::SshTransferFailed { dest: dest_path.to_string(), error: error_msg }.into());
	}

	// Clean up temp zip
	std::fs::remove_file(&zip_path)?;

	Ok(zip_size)
}

/// Backup a folder directly to server via rsync over SSH
async fn backup_to_server_direct(
	folder: &Folder,
	dest_path: &str,
	drive: &DriveConfig,
	server_config: &crate::config::ServerConfig,
	verbose: bool,
	pipe_rsync_stdout: bool,
) -> Result<u64> {
	// Build SSH command and options
	let ssh_cmd = build_ssh_command_for_rsync(drive, server_config);
	let ssh_options = build_ssh_options(drive, server_config);
	
	// Build user@host target for SSH commands
	let user = drive.user.as_deref().unwrap_or("user");
	let host = drive.host.as_deref().unwrap_or("localhost");
	let target = format!("{}@{}", user, host);

	// Create parent directory on server first
	if let Some(parent_dir) = std::path::Path::new(dest_path).parent() {
		// Extract just the path part (after user@host:)
		let parent_dir_str = parent_dir.to_string_lossy();
		let remote_path = parent_dir_str.splitn(2, ':').last().unwrap_or(&parent_dir_str);
		
		let mkdir_cmd = format!("ssh{} {} 'mkdir -p {}'", ssh_options, target, remote_path);
		info!("Creating parent directory on server: {}", remote_path);
		
		let mkdir_output = std::process::Command::new("bash")
			.arg("-c")
			.arg(&mkdir_cmd)
			.output()
			.ok();
		
		if let Some(output) = mkdir_output {
			if !output.status.success() {
				let stderr = String::from_utf8_lossy(&output.stderr);
				warn!("Failed to create parent directory: {}", stderr);
			}
		}
	}

	info!("Running rsync to server: {}", dest_path);

	// Build rsync command — use rsync's --timeout (idle timeout) instead of wrapping with `timeout`
	let idle_timeout = drive.connect_timeout.unwrap_or(300);
	let mut cmd = tokio::process::Command::new("rsync");
	cmd.args(&["-avP", "--progress", "--no-specials"])
		.arg(format!("--timeout={}", idle_timeout))
		.arg("--exclude=*.sock")
		.arg("--exclude=agent*")
		.arg("--exclude=S.*");

	// Add folder-specific excludes
	for exclude in &folder.excludes {
		cmd.arg(format!("--exclude={}", exclude));
	}

	cmd.arg("-e")
		.arg(&ssh_cmd)
		.arg(format!("{}/", folder.source.display()))
		.arg(dest_path);

	// Load and set service tokens for cloudflared
	if let Some(creds) = load_cloudflare_credentials() {
		if let (Some(token_id), Some(token_secret)) = (creds.service_token_id, creds.service_token_secret) {
			cmd.env("CLOUDFLARED_SERVICE_TOKEN_ID", token_id)
				.env("CLOUDFLARED_SERVICE_TOKEN_SECRET", token_secret);
		}
	}

	// Pipe output if verbose OR if pipe_rsync_stdout is enabled
	if verbose || pipe_rsync_stdout {
		cmd.stderr(Stdio::inherit()).stdout(Stdio::inherit());
	} else {
		cmd.stderr(Stdio::piped()).stdout(Stdio::piped());
	}

	let output = cmd
		.output()
		.await
		.context("Failed to execute rsync to server")?;

	if !output.status.success() {
		let stderr = String::from_utf8_lossy(&output.stderr).to_string();
		let stdout = String::from_utf8_lossy(&output.stdout).to_string();

		// Build informative error message
		let error_msg = if stderr.trim().is_empty() {
			if stdout.trim().is_empty() {
				"Command timed out or failed silently (check cloudflared authentication)".to_string()
			} else {
				format!("rsync output: {}", stdout.trim())
			}
		} else {
			stderr.trim().to_string()
		};

		return Err(BackupError::SshTransferFailed { dest: dest_path.to_string(), error: error_msg }.into());
	}

	// Parse bytes transferred
	let bytes = parse_rsync_output(&output.stdout);
	Ok(bytes)
}

/// Cloudflare service token configuration
#[derive(Debug, Clone, serde::Deserialize, Default)]
struct CloudflareCredentials {
	cloudflare: Option<CloudflareConfig>,
}

#[derive(Debug, Clone, serde::Deserialize, Default)]
struct CloudflareConfig {
	ssh: Option<SshCredentials>,
}

#[derive(Debug, Clone, serde::Deserialize, Default)]
struct SshCredentials {
	service_token_id: Option<String>,
	service_token_secret: Option<String>,
}

/// Load Cloudflare service tokens from encrypted config
fn load_cloudflare_credentials() -> Option<SshCredentials> {
	use std::fs;

	let cred_path = dirs::home_dir().map(|h| h.join(".config/backup-tool/secrets/cloudflare_creds.yaml"))?;

	if !cred_path.exists() {
		return None;
	}

	// Read encrypted file
	let content = fs::read_to_string(&cred_path).ok()?;

	// Try to decrypt with sops
	let decrypted = run_sops_decrypt(&content)?;

	// Parse YAML
	let creds: CloudflareCredentials = serde_yaml::from_str(&decrypted).ok()?;
	creds.cloudflare?.ssh
}

/// Run sops decrypt on content
fn run_sops_decrypt(content: &str) -> Option<String> {
	use std::process::{Command, Stdio};

	let mut child = Command::new("sops")
		.arg("--decrypt")
		.stdin(Stdio::piped())
		.stdout(Stdio::piped())
		.stderr(Stdio::null())
		.spawn()
		.ok()?;

	if let Some(mut stdin) = child.stdin.take() {
		use std::io::Write;
		stdin.write_all(content.as_bytes()).ok()?;
	}

	let output = child.wait_with_output().ok()?;
	if output.status.success() {
		Some(String::from_utf8_lossy(&output.stdout).to_string())
	} else {
		None
	}
}

/// Build SSH command for rsync -e flag
/// Returns full command like: "ssh -i key -o options"
fn build_ssh_command_for_rsync(drive: &DriveConfig, server_config: &crate::config::ServerConfig) -> String {
	let mut ssh_cmd = String::from("ssh");

	// Identity file - prefer drive config, fall back to server_config
	let identity_file = drive
		.identity_file
		.as_ref()
		.or(server_config.identity_file.as_ref());

	if let Some(ref key) = identity_file {
		let expanded = expand_tilde_path(key);
		ssh_cmd.push_str(&format!(" -i {}", expanded));
	}

	// Port - prefer drive config
	if let Some(port) = drive.port {
		ssh_cmd.push_str(&format!(" -p {}", port));
	}

	// Note: We don't use ProxyCommand here because Cloudflare Access needs
	// to trigger browser authentication. The initial SSH auth check handles this.
	// After authentication, SSH connections work normally.
	ssh_cmd.push_str(" -o IdentitiesOnly=yes");
	ssh_cmd.push_str(" -o BatchMode=no"); // Allow interactive auth

	ssh_cmd
}

/// Build SSH options (without the "ssh" prefix) for direct SSH commands
fn build_ssh_options(drive: &DriveConfig, server_config: &crate::config::ServerConfig) -> String {
	let mut options = String::new();

	// Identity file - prefer drive config, fall back to server_config
	let identity_file = drive
		.identity_file
		.as_ref()
		.or(server_config.identity_file.as_ref());

	if let Some(ref key) = identity_file {
		let expanded = expand_tilde_path(key);
		options.push_str(&format!(" -i {}", expanded));
	}

	// Port - prefer drive config
	if let Some(port) = drive.port {
		options.push_str(&format!(" -p {}", port));
	}

	options.push_str(" -o IdentitiesOnly=yes");
	options.push_str(" -o BatchMode=no");

	options
}

/// Expand tilde in a path string (delegates to folder::expand_tilde)
fn expand_tilde_path(path: &str) -> String {
	crate::folder::expand_tilde(&std::path::PathBuf::from(path))
		.to_string_lossy()
		.to_string()
}

#[cfg(test)]
mod tests {
	use std::fs;

	use tempfile::TempDir;

	use super::*;

	#[test]
	fn test_get_directory_size() {
		let temp_dir = TempDir::new().expect("Failed to create temp dir");
		let temp_path = temp_dir.path().to_path_buf();

		// Create test files
		fs::write(temp_path.join("file1.txt"), "content1").unwrap();
		fs::write(temp_path.join("file2.txt"), "content2").unwrap();

		let size = get_directory_size(&temp_path);

		// Should be at least the size of the two files
		assert!(size >= 16); // "content1" + "content2" = 16 bytes
	}

	#[test]
	fn test_get_directory_size_nested() {
		let temp_dir = TempDir::new().expect("Failed to create temp dir");
		let temp_path = temp_dir.path().to_path_buf();

		// Create nested structure
		let subdir = temp_path.join("subdir");
		fs::create_dir_all(&subdir).unwrap();
		fs::write(temp_path.join("file1.txt"), "content1").unwrap();
		fs::write(subdir.join("file2.txt"), "content2").unwrap();

		let size = get_directory_size(&temp_path);
		assert!(size >= 16);
	}

	#[test]
	fn test_get_directory_size_empty() {
		let temp_dir = TempDir::new().expect("Failed to create temp dir");
		let temp_path = temp_dir.path().to_path_buf();

		let size = get_directory_size(&temp_path);
		assert_eq!(size, 0);
	}

	#[test]
	fn test_parse_rsync_output() {
		// Test typical rsync output
		let output = b"1234567 100.00%    0.00MB/s    0:00:00 (xfr#1, to-chk=0/1)";
		let bytes = parse_rsync_output(output);
		assert!(bytes > 0);

		// Test empty output
		let empty_output = b"";
		let bytes = parse_rsync_output(empty_output);
		assert_eq!(bytes, 0);
	}

	#[test]
	fn test_expand_tilde_path() {
		// Test path with tilde
		let home = dirs::home_dir().expect("Failed to get home dir");
		let home_str = home.to_string_lossy();

		let result = expand_tilde_path("~/test/path");

		// Check that result contains the home directory
		assert!(
			result.contains(&*home_str),
			"Result '{}' should contain home '{}'",
			result,
			home_str
		);
		assert!(result.ends_with("test/path"), "Result '{}' should end with 'test/path'", result);

		// Test path without tilde
		let result = expand_tilde_path("/absolute/path");
		assert_eq!(result, "/absolute/path");
	}

	#[test]
	fn test_backup_error_display() {
		let err = BackupError::SourceNotFound { path: "/nonexistent".to_string() };
		assert!(err.to_string().contains("/nonexistent"));

		let err = BackupError::DriveNotFound {
			drive_name: "flash".to_string(),
			available_drives: "flash, server".to_string(),
		};
		assert!(err.to_string().contains("flash"));

		let err = BackupError::RsyncFailed {
			source_path: "/source".to_string(),
			dest_path: "/dest".to_string(),
			error: "connection refused".to_string(),
		};
		assert!(err.to_string().contains("connection refused"));
	}
}
