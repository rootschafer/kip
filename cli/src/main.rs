//! Kip CLI
//!
//! CLI interface for Kip backup and sync operations.

use anyhow::Result;
use clap::{Parser, Subcommand};
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
		/// Source path to backup
		#[arg(short, long)]
		source: Option<String>,
	},
	
	/// Restore from backup
	Restore {
		/// Backup ID to restore from
		#[arg(short, long)]
		id: Option<String>,
	},
	
	/// Check backup status
	Check,
	
	/// List available backups
	List,
}

#[tokio::main]
async fn main() -> Result<()> {
	let cli = Cli::parse();
	
	// Setup logging
	if cli.verbose {
		tracing_subscriber::registry()
			.with(tracing_subscriber::fmt::layer())
			.with(tracing_subscriber::EnvFilter::new("debug"))
			.init();
	} else {
		tracing_subscriber::registry()
			.with(tracing_subscriber::fmt::layer())
			.with(tracing_subscriber::EnvFilter::new("info"))
			.init();
	}
	
	info!("Kip CLI starting...");
	
	match cli.command {
		Commands::Backup { source } => {
			println!("Backup command received");
			if let Some(src) = source {
				println!("Source: {}", src);
			}
			// TODO: Implement backup logic using daemon crate
		}
		Commands::Restore { id } => {
			println!("Restore command received");
			if let Some(backup_id) = id {
				println!("Backup ID: {}", backup_id);
			}
			// TODO: Implement restore logic using daemon crate
		}
		Commands::Check => {
			println!("Check command received");
			// TODO: Implement check logic using daemon crate
		}
		Commands::List => {
			println!("List command received");
			// TODO: Implement list logic using daemon crate
		}
	}
	
	Ok(())
}
