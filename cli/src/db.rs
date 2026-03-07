//! SurrealDB integration for backup-tool
//!
//! Connects to the same SurrealDB instance as Kip (~/Library/Application Support/Kip/kip.db).
//! This allows backup-tool and Kip to share state: drives, locations, intents, and transfer records.
//!
//! The CLI writes transfer completions here alongside the legacy state.json,
//! enabling a gradual migration path.

use std::path::PathBuf;

use anyhow::{Context, Result};
use surrealdb::{
	engine::local::{Db, SurrealKv},
	Surreal,
};
use tracing::info;

/// Wrapper around the SurrealDB handle.
/// Compatible with Kip's DbHandle.
#[derive(Clone)]
pub struct DbHandle {
	pub db: Surreal<Db>,
}

/// Resolve the database path — same location Kip uses.
/// ~/Library/Application Support/Kip/kip.db
fn db_path() -> PathBuf {
	let base = dirs::data_dir()
		.or_else(|| dirs::home_dir().map(|h| h.join("Library").join("Application Support")))
		.unwrap_or_else(|| PathBuf::from("."));
	let kip_dir = base.join("Kip");
	std::fs::create_dir_all(&kip_dir).ok();
	kip_dir.join("kip.db")
}

/// Initialize the database connection.
/// Uses the same namespace and database as Kip for shared state.
pub async fn init() -> Result<DbHandle> {
	let path = db_path();
	info!("Connecting to SurrealDB at: {}", path.display());

	let db = Surreal::new::<SurrealKv>(path)
		.await
		.with_context(|| "Failed to open SurrealDB")?;

	db.use_ns("kip").use_db("kip").await?;

	// Run migrations (idempotent DEFINE statements)
	db.query(SCHEMA).await?.check()?;

	// Bootstrap local machine record
	bootstrap_local_machine(&db).await?;

	info!("SurrealDB initialized successfully");
	Ok(DbHandle { db })
}

/// Add a location record to the database
pub async fn add_location(
	db: &DbHandle,
	path: &std::path::Path,
	label: Option<&str>,
	machine: Option<&str>,
) -> Result<String> {
	let path_str = path.to_string_lossy().to_string();
	let label_str = label
		.unwrap_or_else(|| path.file_name().and_then(|s| s.to_str()).unwrap_or(""))
		.to_string();

	// Check if location already exists
	let mut response = db
		.db
		.query("SELECT id FROM location WHERE path = $path LIMIT 1")
		.bind(("path", path_str.clone()))
		.await?
		.check()?;

	let existing: Option<Vec<serde_json::Value>> = response.take(0)?;
	if let Some(rows) = existing {
		if let Some(row) = rows.first() {
			if let Some(id) = row["id"].as_str() {
				return Ok(id.to_string());
			}
		}
	}

	// Create new location
	let location_id = format!("location:{}", slug(&path_str));
	let machine_ref = match machine {
		Some(m) => format!("machine:{}", slug(m)),
		None => "machine:local".to_string(),
	};

	db.db
		.query(&format!(
			"CREATE {} CONTENT {{
			machine: {},
			path: $path,
			label: $label,
			created_at: time::now(),
			available: true,
		}}",
			location_id, machine_ref
		))
		.bind(("path", path_str))
		.bind(("label", label_str))
		.await?
		.check()?;

	Ok(location_id)
}

/// Create an intent (sync relationship) between source and destinations
pub async fn create_intent(db: &DbHandle, source_id: &str, dest_ids: &[String], priority: u16) -> Result<String> {
	let intent_id = format!("intent:{}", slug(&format!("{}_{}", source_id, dest_ids.join("_"))));

	// Build destinations array using type::thing() for proper record references
	let dest_array: String = dest_ids
		.iter()
		.map(|id| {
			// Extract table and ID from the record ID
			if let Some((table, rec_id)) = id.split_once(':') {
				format!("type::thing('{}', '{}')", table, rec_id)
			} else {
				format!("type::thing('location', '{}')", id)
			}
		})
		.collect::<Vec<_>>()
		.join(", ");

	// Create source reference using type::thing()
	let source_ref = if let Some((table, rec_id)) = source_id.split_once(':') {
		format!("type::thing('{}', '{}')", table, rec_id)
	} else {
		format!("type::thing('location', '{}')", source_id)
	};

	db.db
		.query(&format!(
			"CREATE {} SET
			source = {},
			destinations = [{}],
			status = 'idle',
			kind = 'one_shot',
			speed_mode = 'normal',
			priority = $priority,
			created_at = time::now(),
			updated_at = time::now(),
			total_files = 0,
			total_bytes = 0,
			completed_files = 0,
			completed_bytes = 0
		",
			intent_id, source_ref, dest_array
		))
		.bind(("priority", priority as i64))
		.await?
		.check()?;

	Ok(intent_id)
}

/// Record a completed backup transfer in SurrealDB.
/// This creates/updates records that Kip can read.
pub async fn record_backup_completion(
	db: &Surreal<Db>,
	source_path: &str,
	dest_path: &str,
	drive_name: &str,
	bytes_transferred: u64,
	is_local: bool,
) -> Result<()> {
	// Upsert the drive record
	let drive_name_owned = drive_name.to_string();
	db.query(
		"UPSERT drive SET
			name = $name,
			uuid = $name,
			connected = true,
			last_seen = time::now()
		WHERE name = $name",
	)
	.bind(("name", drive_name_owned.clone()))
	.await?
	.check()?;

	// Upsert source location (on local machine)
	let source_id = format!("location:backup_src_{}", slug(source_path));
	let source_label = source_path
		.rsplit('/')
		.next()
		.unwrap_or(source_path)
		.to_string();
	db.query(&format!(
		"UPSERT {} CONTENT {{
			machine: machine:local,
			path: $path,
			label: $label,
			created_at: time::now(),
			available: true,
		}}",
		source_id
	))
	.bind(("path", source_path.to_string()))
	.bind(("label", source_label))
	.await?
	.check()?;

	// Upsert destination location (on the drive)
	let dest_id = format!("location:backup_dst_{}_{}", slug(drive_name), slug(dest_path));
	let dest_label = dest_path
		.rsplit('/')
		.next()
		.unwrap_or(dest_path)
		.to_string();
	db.query(&format!(
		"UPSERT {} CONTENT {{
			path: $path,
			label: $label,
			created_at: time::now(),
			available: $available,
		}}",
		dest_id
	))
	.bind(("path", dest_path.to_string()))
	.bind(("label", dest_label))
	.bind(("available", is_local))
	.await?
	.check()?;

	// Record the transfer as a completed intent
	let intent_id = format!("intent:backup_{}_{}", slug(source_path), slug(drive_name));
	let intent_name = format!("Backup {} → {}", source_path, drive_name);
	db.query(&format!(
		"UPSERT {} CONTENT {{
			name: $name,
			source: {},
			destinations: [{}],
			status: 'complete',
			kind: 'one_shot',
			speed_mode: 'normal',
			priority: 0,
			created_at: time::now(),
			updated_at: time::now(),
			total_bytes: $bytes,
			completed_bytes: $bytes,
			total_files: 0,
			completed_files: 0,
		}}",
		intent_id, source_id, dest_id
	))
	.bind(("name", intent_name))
	.bind(("bytes", bytes_transferred as i64))
	.await?
	.check()?;

	info!(
		"Recorded backup completion in SurrealDB: {} → {} ({} bytes)",
		source_path, dest_path, bytes_transferred
	);

	Ok(())
}

/// Import drive configs from TOML into SurrealDB drive records
pub async fn sync_drives_to_db(db: &Surreal<Db>, drives: &[crate::drive_config::DriveConfig]) -> Result<()> {
	for drive in drives {
		let mount = drive.mount_point.as_deref().unwrap_or("").to_string();
		let connected = if drive.is_local() {
			std::path::Path::new(&mount).exists()
		} else {
			false // SSH drives checked separately
		};

		db.query(
			"UPSERT drive SET
				name = $name,
				uuid = $name,
				mount_point = $mount,
				connected = $connected,
				last_seen = time::now()
			WHERE name = $name",
		)
		.bind(("name", drive.name.clone()))
		.bind(("mount", mount))
		.bind(("connected", connected))
		.await?
		.check()?;
	}

	info!("Synced {} drives to SurrealDB", drives.len());
	Ok(())
}

/// Create/update the local machine record (same as Kip does)
async fn bootstrap_local_machine(db: &Surreal<Db>) -> Result<()> {
	let hostname = std::process::Command::new("hostname")
		.output()
		.map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
		.unwrap_or_else(|_| "unknown".to_string());

	db.query(
		"UPSERT machine:local CONTENT {
			name: $name,
			kind: 'local',
			hostname: $hostname,
			is_current: true,
			last_seen: time::now(),
			online: true,
		}",
	)
	.bind(("name", hostname.clone()))
	.bind(("hostname", hostname))
	.await?
	.check()?;

	Ok(())
}

/// Create a URL-safe slug from a path for use as SurrealDB record IDs
fn slug(path: &str) -> String {
	path.chars()
		.map(|c| match c {
			'/' | '\\' | ' ' | '.' | '~' | ':' | '@' => '_',
			c if c.is_alphanumeric() || c == '_' || c == '-' => c,
			_ => '_',
		})
		.collect::<String>()
		.trim_matches('_')
		.to_string()
}

/// Schema definitions — subset of Kip's schema.
/// Uses DEFINE ... OVERWRITE so it's idempotent and compatible.
const SCHEMA: &str = "
	DEFINE TABLE OVERWRITE machine SCHEMAFULL;
	DEFINE FIELD OVERWRITE name ON machine TYPE string;
	DEFINE FIELD OVERWRITE kind ON machine TYPE string;
	DEFINE FIELD OVERWRITE hostname ON machine TYPE option<string>;
	DEFINE FIELD OVERWRITE is_current ON machine TYPE bool;
	DEFINE FIELD OVERWRITE ssh_user ON machine TYPE option<string>;
	DEFINE FIELD OVERWRITE ssh_key_path ON machine TYPE option<string>;
	DEFINE FIELD OVERWRITE ssh_proxy ON machine TYPE option<string>;
	DEFINE FIELD OVERWRITE last_seen ON machine TYPE datetime;
	DEFINE FIELD OVERWRITE online ON machine TYPE bool DEFAULT false;

	DEFINE TABLE OVERWRITE drive SCHEMAFULL;
	DEFINE FIELD OVERWRITE name ON drive TYPE string;
	DEFINE FIELD OVERWRITE uuid ON drive TYPE string;
	DEFINE FIELD OVERWRITE filesystem ON drive TYPE option<string>;
	DEFINE FIELD OVERWRITE capacity_bytes ON drive TYPE option<int>;
	DEFINE FIELD OVERWRITE mount_point ON drive TYPE option<string>;
	DEFINE FIELD OVERWRITE connected ON drive TYPE bool DEFAULT false;
	DEFINE FIELD OVERWRITE last_seen ON drive TYPE datetime;
	DEFINE FIELD OVERWRITE limitations ON drive TYPE option<object>;
	DEFINE FIELD OVERWRITE limitations.max_file_size ON drive TYPE option<int>;
	DEFINE INDEX OVERWRITE idx_drive_uuid ON drive FIELDS uuid UNIQUE;

	DEFINE TABLE OVERWRITE location SCHEMAFULL;
	DEFINE FIELD OVERWRITE machine ON location TYPE option<record<machine>>;
	DEFINE FIELD OVERWRITE drive ON location TYPE option<record<drive>>;
	DEFINE FIELD OVERWRITE path ON location TYPE string;
	DEFINE FIELD OVERWRITE label ON location TYPE option<string>;
	DEFINE FIELD OVERWRITE created_at ON location TYPE datetime;
	DEFINE FIELD OVERWRITE available ON location TYPE bool DEFAULT false;
	DEFINE FIELD OVERWRITE graph_x ON location TYPE option<float>;
	DEFINE FIELD OVERWRITE graph_y ON location TYPE option<float>;

	DEFINE TABLE OVERWRITE intent SCHEMAFULL;
	DEFINE FIELD OVERWRITE name ON intent TYPE option<string>;
	DEFINE FIELD OVERWRITE source ON intent TYPE record<location>;
	DEFINE FIELD OVERWRITE destinations ON intent TYPE array<record<location>>;
	DEFINE FIELD OVERWRITE status ON intent TYPE string;
	DEFINE FIELD OVERWRITE kind ON intent TYPE string;
	DEFINE FIELD OVERWRITE speed_mode ON intent TYPE string;
	DEFINE FIELD OVERWRITE priority ON intent TYPE int DEFAULT 0;
	DEFINE FIELD OVERWRITE created_at ON intent TYPE datetime;
	DEFINE FIELD OVERWRITE updated_at ON intent TYPE datetime;
	DEFINE FIELD OVERWRITE total_files ON intent TYPE int DEFAULT 0;
	DEFINE FIELD OVERWRITE total_bytes ON intent TYPE int DEFAULT 0;
	DEFINE FIELD OVERWRITE completed_files ON intent TYPE int DEFAULT 0;
	DEFINE FIELD OVERWRITE completed_bytes ON intent TYPE int DEFAULT 0;
	DEFINE FIELD OVERWRITE include_patterns ON intent TYPE option<array<string>>;
	DEFINE FIELD OVERWRITE exclude_patterns ON intent TYPE option<array<string>>;
	DEFINE FIELD OVERWRITE bidirectional ON intent TYPE bool DEFAULT false;
	DEFINE FIELD OVERWRITE initial_sync_complete ON intent TYPE bool DEFAULT false;
";
