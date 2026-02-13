use std::path::PathBuf;
use surrealdb::engine::local::{Db, SurrealKv};
use surrealdb::Surreal;

/// Wrapper around the SurrealDB handle.
/// Clone is cheap (Arc internally).
#[derive(Clone)]
pub struct DbHandle {
    pub db: Surreal<Db>,
}

impl PartialEq for DbHandle {
    fn eq(&self, _other: &Self) -> bool {
        true // Single global instance
    }
}

/// Resolve the database file path.
/// ~/Library/Application Support/kip/kip.db
fn db_path() -> PathBuf {
    let home = std::env::var("HOME").expect("HOME not set");
    let path = PathBuf::from(home)
        .join("Library")
        .join("Application Support")
        .join("Kip");
    std::fs::create_dir_all(&path).expect("Failed to create Kip data directory");
    path.join("kip.db")
}

/// Initialize the database: connect, select ns/db, run migrations, bootstrap machine.
pub async fn init() -> Result<DbHandle, Box<dyn std::error::Error>> {
    let path = db_path();
    let db = Surreal::new::<SurrealKv>(path).await?;
    db.use_ns("kip").use_db("kip").await?;

    run_migrations(&db).await?;
    bootstrap_local_machine(&db).await?;

    Ok(DbHandle { db })
}

/// Run schema migrations. DEFINE statements are idempotent.
async fn run_migrations(db: &Surreal<Db>) -> Result<(), Box<dyn std::error::Error>> {
    db.query(SCHEMA_V1).await?.check()?;
    Ok(())
}

/// Create/update the "this machine" record on every launch.
async fn bootstrap_local_machine(db: &Surreal<Db>) -> Result<(), Box<dyn std::error::Error>> {
    let hostname = get_hostname();
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

fn get_hostname() -> String {
    std::process::Command::new("hostname")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string())
}

const SCHEMA_V1: &str = "
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

    DEFINE TABLE OVERWRITE transfer_job SCHEMAFULL;
    DEFINE FIELD OVERWRITE intent ON transfer_job TYPE record<intent>;
    DEFINE FIELD OVERWRITE source_path ON transfer_job TYPE string;
    DEFINE FIELD OVERWRITE dest_path ON transfer_job TYPE string;
    DEFINE FIELD OVERWRITE destination ON transfer_job TYPE record<location>;
    DEFINE FIELD OVERWRITE size ON transfer_job TYPE int;
    DEFINE FIELD OVERWRITE bytes_transferred ON transfer_job TYPE int DEFAULT 0;
    DEFINE FIELD OVERWRITE status ON transfer_job TYPE string;
    DEFINE FIELD OVERWRITE attempts ON transfer_job TYPE int DEFAULT 0;
    DEFINE FIELD OVERWRITE max_attempts ON transfer_job TYPE int DEFAULT 3;
    DEFINE FIELD OVERWRITE last_error ON transfer_job TYPE option<string>;
    DEFINE FIELD OVERWRITE error_kind ON transfer_job TYPE option<string>;
    DEFINE FIELD OVERWRITE source_hash ON transfer_job TYPE option<string>;
    DEFINE FIELD OVERWRITE dest_hash ON transfer_job TYPE option<string>;
    DEFINE FIELD OVERWRITE started_at ON transfer_job TYPE option<datetime>;
    DEFINE FIELD OVERWRITE completed_at ON transfer_job TYPE option<datetime>;
    DEFINE FIELD OVERWRITE created_at ON transfer_job TYPE datetime;

    DEFINE TABLE OVERWRITE file_record SCHEMAFULL;
    DEFINE FIELD OVERWRITE hash ON file_record TYPE string;
    DEFINE FIELD OVERWRITE size ON file_record TYPE int;
    DEFINE FIELD OVERWRITE first_seen ON file_record TYPE datetime;
    DEFINE INDEX OVERWRITE idx_hash ON file_record FIELDS hash;
    DEFINE INDEX OVERWRITE idx_size ON file_record FIELDS size;

    DEFINE TABLE OVERWRITE exists_at SCHEMAFULL;
    DEFINE FIELD OVERWRITE path ON exists_at TYPE string;
    DEFINE FIELD OVERWRITE modified_at ON exists_at TYPE datetime;
    DEFINE FIELD OVERWRITE verified_at ON exists_at TYPE datetime;
    DEFINE FIELD OVERWRITE stale ON exists_at TYPE bool DEFAULT false;

    DEFINE TABLE OVERWRITE review_item SCHEMAFULL;
    DEFINE FIELD OVERWRITE job ON review_item TYPE record<transfer_job>;
    DEFINE FIELD OVERWRITE intent ON review_item TYPE record<intent>;
    DEFINE FIELD OVERWRITE error_kind ON review_item TYPE string;
    DEFINE FIELD OVERWRITE error_message ON review_item TYPE string;
    DEFINE FIELD OVERWRITE source_path ON review_item TYPE string;
    DEFINE FIELD OVERWRITE dest_path ON review_item TYPE string;
    DEFINE FIELD OVERWRITE options ON review_item TYPE array<string>;
    DEFINE FIELD OVERWRITE resolution ON review_item TYPE option<string>;
    DEFINE FIELD OVERWRITE created_at ON review_item TYPE datetime;
    DEFINE FIELD OVERWRITE resolved_at ON review_item TYPE option<datetime>;
    DEFINE FIELD OVERWRITE source_size ON review_item TYPE option<int>;
    DEFINE FIELD OVERWRITE source_hash ON review_item TYPE option<string>;
    DEFINE FIELD OVERWRITE source_modified ON review_item TYPE option<datetime>;
    DEFINE FIELD OVERWRITE dest_size ON review_item TYPE option<int>;
    DEFINE FIELD OVERWRITE dest_hash ON review_item TYPE option<string>;
    DEFINE FIELD OVERWRITE dest_modified ON review_item TYPE option<datetime>;
";
