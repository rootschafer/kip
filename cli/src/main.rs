//! Kip CLI
//!
//! CLI interface for Kip backup and sync operations.

use anyhow::Result;
use clap::{Parser, Subcommand};
use cli::{backup, check, config, daemon_lock, restore, status};
use kip_rclone::{CloudDestination, CloudStats, Rclone};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser)]
#[command(name = "kip")]
#[command(about = "Kip - File synchronization and backup CLI")]
#[command(long_about = None)]
struct Cli {
	#[command(subcommand)]
	command: Commands,

	/// Enable verbose output
	#[arg(short, long, default_value_t = false)]
	verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
	/// Run backup operation
	Backup {
		/// Only backup folders matching this pattern
		#[arg(short, long)]
		filter: Option<String>,

		/// Maximum number of folders to backup
		#[arg(short, long)]
		limit: Option<usize>,

		/// Show detailed progress output
		#[arg(short, long)]
		verbose: bool,
	},

	/// Restore files from backup
	Restore {
		/// Restore from 'flash' or 'server'
		#[arg(short, long, default_value = "flash")]
		from: String,

		/// Only restore folders matching this pattern
		#[arg(short, long)]
		filter: Option<String>,

		/// Show detailed progress output
		#[arg(short, long)]
		verbose: bool,
	},

	/// Check backup status
	Check,

	/// List configured folders and their status
	List {
		/// Sort by priority (descending)
		#[arg(short, long, default_value_t = false)]
		sort_priority: bool,

		/// Filter by app config name
		#[arg(short, long)]
		filter: Option<String>,
	},

	/// Import backup-tool configuration into Kip database
	ImportConfig {
		/// Configuration directory (default: ~/.config/kip)
		#[arg(short, long)]
		config_dir: Option<String>,
	},

	/// Validate backup destinations
	Validate,

	/// Clear all intents from database (for re-import)
	ClearIntents,

	/// Clear all data and re-import from backup-tool config
	ResetAndImport,

	/// Show real-time backup progress
	Status {
		/// Clear status file (after backup completes)
		#[arg(long)]
		clear: bool,
	},

	/// Copy files to cloud storage
	CloudCopy {
		/// Source path to copy
		source: String,

		/// Cloud destination (format: remote:path, e.g., "gdrive:backups")
		dest: String,

		/// Show verbose output
		#[arg(short, long)]
		verbose: bool,
	},

	/// Sync files to cloud storage (like rsync --delete)
	CloudSync {
		/// Source path to sync
		source: String,

		/// Cloud destination (format: remote:path, e.g., "gdrive:backups")
		dest: String,

		/// Show verbose output
		#[arg(short, long)]
		verbose: bool,

		/// Dry-run (show what would be transferred)
		#[arg(long)]
		dry_run: bool,
	},

	/// List configured rclone remotes
	CloudRemotes,

	/// Check cloud storage usage
	CloudUsage {
		/// Remote name (e.g., "gdrive")
		remote: String,
	},
}

#[tokio::main]
async fn main() -> Result<()> {
	let cli = Cli::parse();

	// Setup logging
	let log_level = if cli.verbose { "debug" } else { "info" };
	tracing_subscriber::registry()
		.with(tracing_subscriber::fmt::layer())
		.with(tracing_subscriber::EnvFilter::new(log_level))
		.init();

	info!("Kip CLI starting...");

	match cli.command {
		Commands::Backup { filter, limit, verbose } => {
			// Check if another backup is already running
			if let Some(pid) = daemon_lock::get_running_daemon_pid() {
				eprintln!("❌ Another backup is already running (PID: {})", pid);
				eprintln!("   Wait for it to complete or kill it to start a new backup");
				std::process::exit(1);
			}

			// Set up panic hook to release lock on crash
			std::panic::set_hook(Box::new(|_| {
				daemon_lock::release_lock().ok();
			}));

			// Acquire daemon lock
			match daemon_lock::try_acquire_lock()? {
				true => {
					println!("🚀 Starting backup...\n");
					let result = backup::run_backup_with_progress(filter, limit, None, verbose).await;
					// Release lock when done (success or error)
					daemon_lock::release_lock().ok();
					result?;
					println!("\n✅ Backup complete!");
				}
				false => {
					eprintln!("❌ Another backup is already running");
					eprintln!("   Wait for it to complete before starting a new backup");
					std::process::exit(1);
				}
			}
		}

		Commands::Restore { from, filter, .. } => {
			println!("🔄 Starting restore from {}...\n", from);
			restore::run_restore_with_progress(&from, filter, None, false, None).await?;
			println!("\n✅ Restore complete!");
		}

		Commands::Check => {
			println!("🔍 Checking backup status...\n");
			check::check_backup_status().await?;
		}

		Commands::List { sort_priority, filter } => {
			list_configs(sort_priority, filter.as_deref())?;
		}

		Commands::ImportConfig { config_dir } => {
			println!("📥 Importing backup-tool configuration...\n");
			import_config(config_dir).await?;
			println!("\n✅ Configuration imported successfully!");
		}

		Commands::Validate => {
			println!("🔍 Validating backup destinations...\n");
			cli::validate::validate_all()?;
			println!("\n✅ Validation complete!");
		}

		Commands::ClearIntents => {
			println!("🗑️  Clearing all intents from database...\n");
			clear_intents().await?;
			println!("\n✅ All intents cleared!");
		}

		Commands::ResetAndImport => {
			println!("🔄 Resetting database and re-importing configuration...\n");
			reset_and_import().await?;
			println!("\n✅ Reset and import complete!");
		}

		Commands::Status { clear } => {
			if clear {
				status::BackupStatus::clear()?;
				println!("✅ Status cleared");
			} else {
				status::show_status()?;
			}
		}

		Commands::CloudCopy { source, dest, verbose } => {
			println!("☁️  Copying to cloud storage...\n");

			// Parse destination
			let cloud_dest = parse_cloud_destination(&dest)?;

			// Build rclone command
			let mut rclone = Rclone::new();
			if verbose {
				rclone = rclone.with_verbose();
			}

			let stats = rclone.copy(&source, &cloud_dest, "").await?;

			println!("✅ Cloud copy complete!");
			println!("   {}", stats.format());
		}

		Commands::CloudSync { source, dest, verbose, dry_run } => {
			if dry_run {
				println!("🔍 Cloud sync (dry-run)...\n");
			} else {
				println!("☁️  Syncing to cloud storage...\n");
			}

			// Parse destination
			let cloud_dest = parse_cloud_destination(&dest)?;

			// Build rclone command
			let mut rclone = Rclone::new();
			if verbose {
				rclone = rclone.with_verbose();
			}
			if dry_run {
				rclone = rclone.with_dry_run();
			}

			let stats = rclone.sync(&source, &cloud_dest, "").await?;

			if dry_run {
				println!("✅ Dry-run complete (no files transferred)");
			} else {
				println!("✅ Cloud sync complete!");
			}
			println!("   {}", stats.format());
		}

		Commands::CloudRemotes => {
			println!("📋 Configured rclone remotes:\n");

			// Check if rclone is installed
			if !Rclone::check_installed().await.unwrap_or(false) {
				println!("   ⚠️  rclone not found in PATH");
				println!("   Install rclone from https://rclone.org/install/");
				println!();
				println!("   macOS:     brew install rclone");
				println!("   Linux:     curl https://rclone.org/install.sh | sudo bash");
				println!("   Windows:   choco install rclone");
				return Ok(());
			}

			let remotes = Rclone::list_remotes().await?;

			if remotes.is_empty() {
				println!("   ℹ️  No remotes configured");
				println!("   Run 'rclone config' to set up cloud storage");
			} else {
				for remote in remotes {
					println!("   ✅ {}", remote);
				}
			}
		}

		Commands::CloudUsage { remote } => {
			println!("📊 Checking cloud storage usage...\n");

			let dest = CloudDestination::generic(&remote, "");
			let usage = Rclone::new().disk_usage(&dest).await?;

			println!("   Remote: {}", remote);
			if let Some(total) = usage.total_bytes {
				println!("   Total:  {}", CloudStats::format_bytes(total));
			}
			if let Some(used) = usage.used_bytes {
				println!("   Used:   {}", CloudStats::format_bytes(used));
				if let Some(total) = usage.total_bytes {
					let percent = (used as f64 / total as f64 * 100.0) as u32;
					println!("   Usage:  {}%", percent);
				}
			}
			if let Some(free) = usage.free_bytes {
				println!("   Free:   {}", CloudStats::format_bytes(free));
			}
		}
	}

	Ok(())
}

/// Parse cloud destination from "remote:path" format
fn parse_cloud_destination(dest: &str) -> Result<CloudDestination> {
	if let Some(colon_pos) = dest.find(':') {
		let remote = &dest[..colon_pos];
		let path = &dest[colon_pos + 1..];
		Ok(CloudDestination::generic(remote, path))
	} else {
		// No colon, treat as just remote name with empty path
		Ok(CloudDestination::generic(dest, ""))
	}
}

/// List configured folders
fn list_configs(sort_priority: bool, filter: Option<&str>) -> Result<()> {
	let app_configs = config::load_app_configs()?;
	let main_config = config::load_main_config()?;

	// Collect all folders
	let mut all_folders = Vec::new();
	for (config_name, config) in &app_configs {
		for folder_config in &config.folder_configs {
			if let Some(filter_str) = filter {
				let source_str = folder_config.source.to_string_lossy();
				if !config_name.contains(filter_str) && !source_str.contains(filter_str) {
					continue;
				}
			}
			all_folders.push((config_name, folder_config));
		}
	}

	// Sort by priority if requested
	if sort_priority {
		all_folders.sort_by(|a, b| b.1.priority.cmp(&a.1.priority).then_with(|| a.0.cmp(b.0)));
	}

	println!("\n📁 Configured Folders\n");
	println!("{:<30} {:<50} {:>8}", "Config", "Source", "Priority");
	println!("{}", "-".repeat(92));

	for (config_name, folder) in &all_folders {
		let priority = folder.priority.unwrap_or(0);
		let source_display = folder.source.display();
		println!("{:<30} {:<50} {:>8}", config_name, source_display, priority);
	}

	println!("\nTotal: {} folders configured", all_folders.len());

	// Show drive info
	let drives = main_config.drives;
	if !drives.is_empty() {
		println!("\n💾 Configured Drives\n");
		println!("{:<20} {:<40} {:<10}", "Name", "Mount Point", "Type");
		println!("{}", "-".repeat(72));
		for drive in drives {
			let drive_type = if drive.is_local() { "Local" } else { "Remote" };
			let mount = drive.mount_point.as_deref().unwrap_or("Not mounted");
			println!("{:<20} {:<40} {:<10}", drive.name, mount, drive_type);
		}
	}

	Ok(())
}

/// Import backup-tool configuration into database
async fn import_config(config_dir: Option<String>) -> Result<()> {
	use cli::{db, folder};

	let _config_dir = config_dir.map(std::path::PathBuf::from);

	// Initialize database
	let db = db::init().await?;
	println!("✅ Connected to database");

	// Load backup-tool configuration
	let main_config = config::load_main_config()?;
	let app_configs = config::load_app_configs()?;

	println!("📄 Loaded {} app configurations", app_configs.len());
	println!("💾 Found {} drives", main_config.drives.len());

	// Create drive locations in database
	for drive in &main_config.drives {
		if let Some(mount) = &drive.mount_point {
			let mount_path = std::path::PathBuf::from(mount);
			if mount_path.exists() {
				match db::add_location(&db, &mount_path, Some(&drive.name), None).await {
					Ok(_) => println!("   ✅ Added drive location: {}", drive.name),
					Err(e) => println!("   ⚠️  Drive {} already exists: {}", drive.name, e),
				}
			}
		}
	}

	// Create folder locations and intents
	for (config_name, app_config) in &app_configs {
		println!("\n📦 Processing config: {}", config_name);

		for folder_config in &app_config.folder_configs {
			let source_path = folder::expand_tilde(folder_config.source.as_path());

			// Create source location
			match db::add_location(&db, &source_path, None, None).await {
				Ok(source_id) => {
					println!("   ✅ Added source: {}", folder_config.source.display());

					// Create intents for each destination
					for dest in &folder_config.destinations {
						let dest_path = folder::expand_tilde_path(&dest.path);

						// Try to find or create destination location
						let dest_id = match db::add_location(&db, &dest_path, None, None).await {
							Ok(id) => id,
							Err(_) => continue, // Skip if location already exists
						};

						// Create intent
						match db::create_intent(&db, &source_id, &[dest_id], folder_config.priority.unwrap_or(500))
							.await
						{
							Ok(_) => println!("      ✅ Created sync to {}", dest.path),
							Err(e) => println!("      ⚠️  Intent exists: {}", e),
						}
					}
				}
				Err(e) => println!("   ⚠️  Source {} already exists: {}", folder_config.source.display(), e),
			}
		}
	}

	Ok(())
}

/// Clear all intents from the database
async fn clear_intents() -> Result<()> {
	use cli::db;

	let db = db::init().await?;

	// Delete all intents
	db.db.query("DELETE FROM intent").await?.check()?;

	println!("   ✅ Deleted all intent records");

	// Keep locations - they're still valid
	println!("   ℹ️  Location records preserved");

	Ok(())
}

/// Clear all data and re-import from backup-tool config
async fn reset_and_import() -> Result<()> {
	use cli::db;

	// Clear database - use DROP to fully reset tables
	{
		let db = db::init().await?;
		println!("   🗑️  Clearing database...");
		// Drop and recreate tables to avoid lock issues
		db.db
			.query("REMOVE TABLE IF EXISTS intent; REMOVE TABLE IF EXISTS location;")
			.await?
			.check()?;
		println!("   ✅ Database cleared");
		// Explicitly close connection
		drop(db);
	}

	// Wait for SurrealDB to release locks
	tokio::time::sleep(std::time::Duration::from_secs(1)).await;

	// Now import configuration with fresh connection
	println!("\n📥 Importing configuration...\n");
	import_config(None).await?;

	Ok(())
}
