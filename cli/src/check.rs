//! Check/analyze backup status

use std::{collections::HashMap, path::PathBuf};

use anyhow::{Context, Result};
use console::style;
use tracing::info;

use crate::{config, folder::Folder, state::StateManager};

/// Check backup status without modifying anything
pub async fn check_backup_status() -> Result<()> {
	// Load configurations
	let app_configs = config::load_app_configs().context("Failed to load app configurations")?;

	let main_config = config::load_main_config().context("Failed to load main configuration")?;

	// Resolve all folders
	let mut all_folders = Vec::new();

	for (config_name, config) in &app_configs {
		for folder_config in &config.folder_configs {
			if let Ok(folder) = Folder::from_config(folder_config, config_name, config.metadata.priority) {
				all_folders.push((config_name.clone(), folder));
			}
		}
	}

	// Sort by priority
	all_folders.sort_by(|a, b| b.1.priority.cmp(&a.1.priority));

	// Load state
	let state = StateManager::new(main_config.settings.state_file.map(|s: String| s.into()))?;

	println!("\n{}", style("🔍 Checking Backup Status").bold());
	println!("{}", "=".repeat(60));

	// Group by config and count completions
	let mut by_config: HashMap<String, (usize, usize)> = HashMap::new();
	let mut total_destinations = 0;
	let mut complete_destinations = 0;

	for (config_name, folder) in &all_folders {
		let folder_state = state.get_folder_state(&folder.id);

		for dest in &folder.destinations {
			total_destinations += 1;
			let is_complete = folder_state
				.and_then(|s| s.destinations.get(&dest.path))
				.map(|d| d.completed)
				.unwrap_or(false);

			if is_complete {
				complete_destinations += 1;
			}

			let entry = by_config.entry(config_name.clone()).or_insert((0, 0));
			entry.1 += 1;
			if is_complete {
				entry.0 += 1;
			}
		}
	}

	// Print status by config
	println!("\n=== Backup Status Check ===\n");
	for (config_name, (complete, total)) in &by_config {
		let pct = (*complete as f64 / *total as f64) * 100.0;
		println!("{}: {} complete ({:.0}%)", config_name, complete, pct);
	}

	let overall_pct = if total_destinations > 0 {
		(complete_destinations as f64 / total_destinations as f64) * 100.0
	} else {
		0.0
	};

	println!(
		"\n=== Overall: {}/{} destinations ({:.0}%) ===\n",
		complete_destinations, total_destinations, overall_pct
	);

	// Show incomplete destinations
	println!("=== Incomplete Destinations ===\n");
	let mut incomplete_count = 0;

	for (_config_name, folder) in &all_folders {
		let folder_state = state.get_folder_state(&folder.id);

		for dest in &folder.destinations {
			let is_complete = folder_state
				.and_then(|s| s.destinations.get(&dest.path))
				.map(|d| d.completed)
				.unwrap_or(false);

			if !is_complete {
				incomplete_count += 1;
				println!(
					"[{}] {} → {} ({})",
					folder.priority,
					folder.source.display(),
					dest.path,
					dest.drive
				);
			}
		}
	}

	if incomplete_count == 0 {
		println!("{} All destinations are complete!", style("✅").bold().green());
	} else {
		println!(
			"\n{} {} incomplete destination(s)",
			style("⚠️").bold().yellow(),
			incomplete_count
		);
	}

	// Run rsync dry-run to check for changes on flash destinations
	println!("\n=== Checking for uncommitted changes (flash destinations) ===\n");

	// Load unified config to get drives
	let drives = match config::load_main_config() {
		Ok(c) => c.drives,
		Err(_) => Vec::new(),
	};

	for (_config_name, folder) in &all_folders {
		for dest in &folder.destinations {
			// Only check local/flash drives
			let is_flash = drives
				.iter()
				.find(|d| d.name == dest.drive)
				.map(|d| d.is_local())
				.unwrap_or(false);

			if !is_flash {
				continue;
			}

			let dest_path = PathBuf::from(&dest.path);

			if !dest_path.exists() {
				println!("⚠ {} → DEST NOT FOUND: {}", folder.source.display(), dest.path);
				continue;
			}

			// Run rsync --dry-run --itemize-changes
			let mut cmd = tokio::process::Command::new("rsync");
			cmd.args(&[
				"-avn", // archive, verbose, dry-run
				"--delete",
			]);

			for exclude in &folder.excludes {
				cmd.arg("--exclude").arg(exclude);
			}

			cmd.arg(format!("{}/", folder.source.display()))
				.arg(&dest_path);

			match cmd.output().await {
				Ok(output) => {
					let stdout = String::from_utf8_lossy(&output.stdout);
					let changes: Vec<&str> = stdout
						.lines()
						.filter(|l| !l.is_empty() && !l.starts_with("sending "))
						.collect();

					if !changes.is_empty() {
						println!(
							"📝 {} has {} changed files for {}:",
							folder.source.display(),
							changes.len(),
							dest.path
						);
						for line in changes.iter().take(5) {
							println!("   {}", line);
						}
						if changes.len() > 5 {
							println!("   ... and {} more", changes.len() - 5);
						}
					}
				}
				Err(e) => {
					info!("Failed to check {}: {}", folder.source.display(), e);
				}
			}
		}
	}

	Ok(())
}

/// Check server backup status via SSH
pub async fn check_server_status() -> Result<()> {
	let main_config = config::load_main_config().context("Failed to load main configuration")?;

	let server_config = &main_config.server;

	println!("\n{}", style("🌐 Checking Server Status").bold());
	println!("{}", "=".repeat(60));
	println!(
		"Server: {}@{}\n",
		server_config.user.as_deref().unwrap_or("ders"),
		server_config.host.as_deref().unwrap_or("ssh.anders.place")
	);

	// Build SSH command
	let ssh_opts = build_ssh_options(server_config);

	let ls_cmd = format!(
		"ssh {} {}@{} 'ls -lh {} 2>/dev/null | head -30'",
		ssh_opts,
		server_config.user.as_deref().unwrap_or("ders"),
		server_config.host.as_deref().unwrap_or("ssh.anders.place"),
		"/mnt/usb2tb/mac_emergency_backup"
	);

	let mut cmd = tokio::process::Command::new("bash");
	cmd.arg("-c").arg(&ls_cmd);

	match cmd.output().await {
		Ok(output) => {
			if output.status.success() {
				println!("Server contents:\n");
				println!("{}", String::from_utf8_lossy(&output.stdout));
			} else {
				println!("⚠️  Failed to connect to server");
				println!("Error: {}", String::from_utf8_lossy(&output.stderr));
			}
		}
		Err(e) => {
			println!("⚠️  Failed to execute SSH command: {}", e);
		}
	}

	Ok(())
}

/// Build SSH options string from server config
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

/// Expand tilde in a path
fn expand_tilde_path(path: &str) -> PathBuf {
	if path.starts_with('~') {
		if let Some(home) = dirs::home_dir() {
			return home.join(path.trim_start_matches('~').trim_start_matches('/'));
		}
	}
	PathBuf::from(path)
}
