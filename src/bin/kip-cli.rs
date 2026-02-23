//! Kip CLI - Command-line interface for file transfer orchestration

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use kip::{api, db};

#[derive(Parser)]
#[command(name = "kip")]
#[command(about = "File transfer orchestrator")]
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
	/// Show system status
	Status,

	/// Manage intents
	Intent {
		#[command(subcommand)]
		command: IntentCommands,
	},

	/// Manage locations
	Location {
		#[command(subcommand)]
		command: LocationCommands,
	},

	/// Manage review queue
	Review {
		#[command(subcommand)]
		command: ReviewCommands,
	},

	/// Config management
	Config {
		#[command(subcommand)]
		command: ConfigCommands,
	},

	/// Run all idle intents
	Run,
}

#[derive(Subcommand)]
enum IntentCommands {
	/// List all intents
	List {
		/// Filter by status
		#[arg(long)]
		status: Option<String>,
	},

	/// Create a new intent
	Create {
		/// Source path
		source: String,

		/// Destination paths (can specify multiple)
		#[arg(required = true)]
		destinations: Vec<String>,

		/// Human-readable name
		#[arg(long)]
		name: Option<String>,

		/// Priority 0-1000 (default: 500)
		#[arg(long, default_value_t = 500)]
		priority: u16,

		/// Glob pattern to include (repeatable)
		#[arg(long)]
		include: Vec<String>,

		/// Glob pattern to exclude (repeatable)
		#[arg(long)]
		exclude: Vec<String>,
	},

	/// Show intent details
	Show {
		/// Intent ID
		id: String,
	},

	/// Delete an intent
	Delete {
		/// Intent ID
		id: String,

		/// Force delete even if transferring
		#[arg(long)]
		force: bool,
	},

	/// Run transfer for an intent
	Run {
		/// Intent ID
		id: String,

		/// Show per-file progress
		#[arg(long)]
		verbose: bool,
	},

	/// Cancel a running intent
	Cancel {
		/// Intent ID
		id: String,
	},
}

#[derive(Subcommand)]
enum LocationCommands {
	/// List all locations
	List,

	/// Add a new location
	Add {
		/// Path to add
		path: String,

		/// Human-readable label
		#[arg(long)]
		label: Option<String>,

		/// Machine name (default: local)
		#[arg(long)]
		machine: Option<String>,
	},

	/// Remove a location
	Remove {
		/// Location ID
		id: String,
	},
}

#[derive(Subcommand)]
enum ReviewCommands {
	/// List items needing review
	List {
		/// Filter by intent ID
		#[arg(long)]
		intent: Option<String>,
	},

	/// Resolve a review item
	Resolve {
		/// Review ID
		id: String,

		/// Resolution: retry, skip, overwrite, delete-source, delete-dest, abort
		#[arg(long)]
		option: String,
	},

	/// Resolve all review items for an intent
	ResolveAll {
		/// Intent ID
		intent: String,

		/// Resolution to apply to all
		#[arg(long)]
		option: String,
	},
}

#[derive(Subcommand)]
enum ConfigCommands {
	/// Import backup-tool configuration
	Import {
		/// Config directory (default: ~/.config/backup-tool)
		#[arg(long)]
		config_dir: Option<String>,

		/// Show what would be imported without making changes
		#[arg(long)]
		dry_run: bool,
	},
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let cli = Cli::parse();

	// Initialize logging
	let filter = if cli.verbose { "debug" } else { "info" };
	tracing_subscriber::fmt()
		.with_max_level(match filter {
			"debug" => tracing::Level::DEBUG,
			"info" => tracing::Level::INFO,
			_ => tracing::Level::WARN,
		})
		.init();

	// Initialize database
	let db = db::init().await?;

	match cli.command {
		Commands::Status => cmd_status(&db).await,
		Commands::Intent { command } => cmd_intent(&db, command).await,
		Commands::Location { command } => cmd_location(&db, command).await,
		Commands::Review { command } => cmd_review(&db, command).await,
		Commands::Config { command } => cmd_config(&db, command).await,
		Commands::Run => cmd_run(&db).await,
	}
}

async fn cmd_status(db: &db::DbHandle) -> Result<(), Box<dyn std::error::Error>> {
	let status = api::status(db).await?;

	println!("\n{}", "Kip Status".to_string() + &format_time());
	println!("{}", "=".repeat(50));

	println!("\n{}", style_section("Intents"));
	println!(
		"  Total: {}    Idle: {}    Transferring: {}    Complete: {}    Needs Review: {}",
		status.intents.total,
		status.intents.idle,
		status.intents.transferring,
		status.intents.complete,
		status.intents.needs_review
	);

	println!("\n{}", style_section("Transfers"));
	println!(
		"  Pending: {}    Transferring: {}    Complete: {}    Failed: {}    Needs Review: {}",
		status.transfers.pending,
		status.transfers.transferring,
		status.transfers.complete,
		status.transfers.failed,
		status.transfers.needs_review
	);

	println!("\n{}", style_section("Review Queue"));
	println!("  Total: {}", status.review_queue.total);

	println!("\n{}", style_section("Drives"));
	println!("  Connected:");
	if status.drives.connected.is_empty() {
		println!("    (none)");
	} else {
		for drive in &status.drives.connected {
			let capacity = drive
				.capacity_bytes
				.map(|b| format_bytes(b))
				.unwrap_or_else(|| "Unknown".to_string());
			println!(
				"    ✅ {}    {}    {} available",
				drive.name,
				drive.mount_point.as_deref().unwrap_or("N/A"),
				capacity
			);
		}
	}

	if !status.drives.disconnected.is_empty() {
		println!("  Disconnected:");
		for drive in &status.drives.disconnected {
			println!("    ❌ {}", drive.name);
		}
	}

	println!();
	Ok(())
}

async fn cmd_intent(db: &db::DbHandle, command: IntentCommands) -> Result<(), Box<dyn std::error::Error>> {
	match command {
		IntentCommands::List { status } => {
			let intents = api::list_intents(db).await?;

			if intents.is_empty() {
				println!("No intents found.");
				return Ok(());
			}

			println!(
				"{:<35} {:<30} {:<25} {:<15} {}",
				"ID", "Source", "Destinations", "Status", "Progress"
			);
			println!("{}", "-".repeat(120));

			for intent in intents {
				if let Some(ref filter) = status {
					if format!("{:?}", intent.status).to_lowercase() != filter.to_lowercase() {
						continue;
					}
				}

				let dests: Vec<String> = intent.destinations.iter().map(|d| d.path.clone()).collect();
				let dest_str = if dests.is_empty() {
					"(none)".to_string()
				} else {
					dests.join(", ")
				};

				let progress = if intent.progress.total_bytes == 0 {
					"-".to_string()
				} else {
					format!(
						"{:.0}% ({})",
						intent.progress.percent_complete(),
						format_bytes(intent.progress.completed_bytes)
					)
				};

				println!(
					"{:<35} {:<30} {:<25} {:<15} {}",
					truncate(&intent.id, 35),
					truncate(&intent.source.path, 30),
					truncate(&dest_str, 25),
					format!("{:?}", intent.status),
					progress
				);
			}
		}

		IntentCommands::Create {
			source,
			destinations,
			name,
			priority,
			include,
			exclude,
		} => {
			let config = api::IntentConfig {
				name,
				priority,
				include_patterns: include,
				exclude_patterns: exclude,
				..Default::default()
			};

			// Add locations if they don't exist
			let source_id = api::add_location(db, source.into(), None, None).await?;
			let mut dest_ids = Vec::new();
			for dest in destinations {
				let dest_id = api::add_location(db, dest.into(), None, None).await?;
				dest_ids.push(dest_id);
			}

			let intent_id = api::create_intent(db, source_id, dest_ids, config).await?;

			println!("Created intent: {}", intent_id);
			println!("  Run with: kip intent run {}", intent_id);
		}

		IntentCommands::Show { id } => {
			let detail = api::get_intent(db, &id).await?;

			println!("\nIntent: {}", detail.summary.id);
			if let Some(name) = &detail.summary.name {
				println!("  Name: {}", name);
			}
			println!("  Status: {:?}", detail.summary.status);
			println!("  Source: {}", detail.summary.source.path);
			println!("  Destinations:");
			for dest in &detail.summary.destinations {
				println!("    - {}", dest.path);
			}
			println!(
				"  Progress: {} / {} ({:.0}%)",
				format_bytes(detail.summary.progress.completed_bytes),
				format_bytes(detail.summary.progress.total_bytes),
				detail.summary.progress.percent_complete()
			);
		}

		IntentCommands::Delete { id, force } => {
			if !force {
				// Check if intent is transferring
				let detail = api::get_intent(db, &id).await;
				if let Ok(d) = detail {
					if d.summary.status == api::IntentStatus::Transferring {
						eprintln!("Intent is currently transferring. Use --force to delete anyway.");
						return Ok(());
					}
				}
			}

			api::delete_intent(db, &id).await?;
			println!("Deleted intent: {}", id);
		}

		IntentCommands::Run { id, verbose: _ } => {
			println!("Running intent: {}", id);

			let start = std::time::Instant::now();
			let result = api::run_intent(db, &id, None).await?;
			let duration = start.elapsed();

			println!(
				"\n✅ Complete: {} files, {} in {:?}",
				result.completed,
				format_bytes(result.bytes_transferred),
				duration
			);

			if result.needs_review > 0 {
				println!("⚠️  {} items need review", result.needs_review);
				println!("   Run: kip review list");
			}

			if result.failed > 0 {
				println!("❌ {} items failed", result.failed);
			}
		}

		IntentCommands::Cancel { id } => {
			api::cancel_intent(db, &id).await?;
			println!("Cancelled intent: {}", id);
		}
	}

	Ok(())
}

async fn cmd_location(db: &db::DbHandle, command: LocationCommands) -> Result<(), Box<dyn std::error::Error>> {
	match command {
		LocationCommands::List => {
			let locations = api::list_locations(db).await?;

			if locations.is_empty() {
				println!("No locations found.");
				return Ok(());
			}

			println!("{:<25} {:<40} {:<15} {}", "ID", "Path", "Machine", "Available");
			println!("{}", "-".repeat(90));

			for loc in locations {
				let available = if loc.available { "✅" } else { "❌" };
				println!(
					"{:<25} {:<40} {:<15} {}",
					truncate(&loc.id, 25),
					truncate(&loc.path, 40),
					loc.machine.name,
					available
				);
			}
		}

		LocationCommands::Add { path, label, machine } => {
			let id = api::add_location(db, path.into(), label, machine).await?;
			println!("Added location: {}", id);
		}

		LocationCommands::Remove { id } => {
			api::remove_location(db, &id).await?;
			println!("Removed location: {}", id);
		}
	}

	Ok(())
}

async fn cmd_review(db: &db::DbHandle, command: ReviewCommands) -> Result<(), Box<dyn std::error::Error>> {
	match command {
		ReviewCommands::List { intent } => {
			let items = api::list_review_items(db).await?;

			if items.is_empty() {
				println!("No review items.");
				return Ok(());
			}

			println!("{:<15} {:<30} {:<20} {}", "ID", "Error", "Source", "Destination");
			println!("{}", "-".repeat(90));

			for item in items {
				if let Some(ref filter) = intent {
					if !item.intent.contains(filter) {
						continue;
					}
				}

				println!(
					"{:<15} {:<30} {:<20} {}",
					truncate(&item.id, 15),
					format!("{:?}", item.error_kind),
					truncate(&item.source_path, 20),
					truncate(&item.dest_path, 20)
				);
			}
		}

		ReviewCommands::Resolve { id, option } => {
			let resolution = parse_resolution(&option)?;
			api::resolve_review(db, &id, resolution).await?;
			println!("Resolved review item: {}", id);
		}

		ReviewCommands::ResolveAll { intent, option } => {
			let resolution = parse_resolution(&option)?;
			let count = api::resolve_all_review(db, &intent, resolution).await?;
			println!("Resolved {} items for intent: {}", count, intent);
		}
	}

	Ok(())
}

async fn cmd_config(db: &db::DbHandle, command: ConfigCommands) -> Result<(), Box<dyn std::error::Error>> {
	match command {
		ConfigCommands::Import { config_dir, dry_run } => {
			let config_dir = config_dir.map(PathBuf::from);

			if dry_run {
				// Just show what would be imported
				println!("Import preview:");
				println!(
					"  Config directory: {}",
					config_dir
						.as_ref()
						.unwrap_or(&dirs::config_dir().unwrap().join("backup-tool"))
						.display()
				);
				println!("\n  Would scan for:");
				println!("    - drives.toml");
				println!("    - apps/*.toml");
				println!("\n  Run without --dry-run to import.");
			} else {
				let result = api::import_backup_tool_config(db, config_dir).await?;

				println!("Import complete:");
				println!("  Locations created: {}", result.locations_created);
				println!("  Intents created: {}", result.intents_created);

				if !result.errors.is_empty() {
					println!("\n  Errors:");
					for err in &result.errors {
						println!("    - {}: {}", err.file.display(), err.reason);
					}
				}

				println!("\nRun 'kip intent list' to see imported intents.");
			}
		}
	}

	Ok(())
}

async fn cmd_run(db: &db::DbHandle) -> Result<(), Box<dyn std::error::Error>> {
	let intents = api::list_intents(db).await?;
	let idle_intents: Vec<_> = intents
		.into_iter()
		.filter(|i| i.status == api::IntentStatus::Idle)
		.collect();

	if idle_intents.is_empty() {
		println!("No idle intents to run.");
		return Ok(());
	}

	println!("Running {} idle intents...\n", idle_intents.len());

	for intent in idle_intents {
		println!("[{}]", intent.id);
		match api::run_intent(db, &intent.id, None).await {
			Ok(result) => {
				println!(
					"  ✅ Complete: {} files, {}",
					result.completed,
					format_bytes(result.bytes_transferred)
				);
				if result.needs_review > 0 {
					println!("  ⚠️  {} items need review", result.needs_review);
				}
			}
			Err(e) => {
				println!("  ❌ Error: {}", e);
			}
		}
		println!();
	}

	println!("All intents complete.");
	Ok(())
}

// Helper functions

fn parse_resolution(option: &str) -> Result<api::Resolution, Box<dyn std::error::Error>> {
	Ok(match option.to_lowercase().as_str() {
		"retry" => api::Resolution::Retry,
		"skip" => api::Resolution::Skip,
		"overwrite" => api::Resolution::Overwrite,
		"delete-source" => api::Resolution::DeleteSource,
		"delete-dest" => api::Resolution::DeleteDest,
		"abort" | "abort-intent" => api::Resolution::AbortIntent,
		_ => {
			return Err(format!(
				"Unknown resolution: {}. Use: retry, skip, overwrite, delete-source, delete-dest, or abort",
				option
			)
			.into())
		}
	})
}

fn format_bytes(bytes: u64) -> String {
	const KB: u64 = 1024;
	const MB: u64 = KB * 1024;
	const GB: u64 = MB * 1024;

	if bytes >= GB {
		format!("{:.1} GB", bytes as f64 / GB as f64)
	} else if bytes >= MB {
		format!("{:.1} MB", bytes as f64 / MB as f64)
	} else if bytes >= KB {
		format!("{:.1} KB", bytes as f64 / KB as f64)
	} else {
		format!("{} B", bytes)
	}
}

fn truncate(s: &str, max_len: usize) -> String {
	if s.len() <= max_len {
		s.to_string()
	} else {
		format!("{}...", &s[..max_len - 3])
	}
}

fn format_time() -> String {
	use chrono::{format::strftime, Local};
	Local::now().format(" — %Y-%m-%d %H:%M:%S").to_string()
}

fn style_section(s: &str) -> String {
	format!("{}:", s)
}
