//! Database initialization

use std::path::{Path, PathBuf};

use surrealdb::{
    engine::local::{Db, SurrealKv},
    Surreal,
};

use crate::db::{DbHandle, SCHEMA_V1};

/// Initialize the database: connect, select ns/db, run migrations, bootstrap machine.
pub async fn init() -> Result<DbHandle, Box<dyn std::error::Error>> {
    let path = db_path();
    init_with_path(&path).await
}

/// Initialize the database at a specific path
pub async fn init_with_path(path: &Path) -> Result<DbHandle, Box<dyn std::error::Error>> {
    let db = Surreal::new::<SurrealKv>(path).await?;
    db.use_ns("kip").use_db("kip").await?;

    run_migrations(&db).await?;
    bootstrap_local_machine(&db).await?;

    Ok(DbHandle { db })
}

/// Initialize an in-memory database for testing
/// Each call creates a completely isolated instance
pub async fn init_memory() -> Result<DbHandle, Box<dyn std::error::Error>> {
    // Use a unique memory namespace for each test
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let memory_id = format!("mem://test_{}", timestamp);
    
    let db = Surreal::new::<SurrealKv>(memory_id.as_str()).await?;
    db.use_ns("kip").use_db("kip").await?;

    run_migrations(&db).await?;
    bootstrap_local_machine(&db).await?;

    Ok(DbHandle { db })
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

/// Run schema migrations. DEFINE statements are idempotent.
async fn run_migrations(db: &Surreal<Db>) -> Result<(), Box<dyn std::error::Error>> {
    db.query(SCHEMA_V1).await?.check()?;
    Ok(())
}

/// Create/update the "this machine" record on every launch.
async fn bootstrap_local_machine(db: &Surreal<Db>) -> Result<(), Box<dyn std::error::Error>> {
    let hostname = get_hostname();
    tracing::info!("Bootstrapping local machine with hostname: {}", hostname);
    let mut resp = db
        .query(
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
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    resp.check()
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    tracing::info!("Successfully bootstrapped local machine");
    Ok(())
}

fn get_hostname() -> String {
    std::process::Command::new("hostname")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string())
}
