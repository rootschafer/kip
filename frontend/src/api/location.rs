//! Location management API

use std::path::PathBuf;

use crate::{
	api::{KipError, LocationId, LocationSummary, MachineKind, MachineSummary},
	db::DbHandle,
};

/// Add a new location
pub async fn add_location(
	db: &DbHandle,
	path: PathBuf,
	label: Option<String>,
	_machine: Option<String>,
) -> Result<LocationId, KipError> {
	let path = expand_tilde(path)?;

	if !path.exists() {
		return Err(KipError::SourcePathNotExists(path.clone()));
	}

	if let Some(existing) = find_location_by_path(db, &path).await? {
		return Ok(existing);
	}

	let location_id = format!("location:{}", ulid::Ulid::new());

	db.db
		.query("CREATE location CONTENT { path: $path, label: $label, available: true }")
		.bind(("path", path.to_string_lossy().to_string()))
		.bind(("label", label))
		.await
		.map_err(|e| KipError::Database(e.to_string()))?
		.check()
		.map_err(|e| KipError::Database(e.to_string()))?;

	Ok(location_id)
}

/// List all locations - explicitly select only simple fields
pub async fn list_locations(db: &DbHandle) -> Result<Vec<LocationSummary>, KipError> {
	// Select only the fields we need, avoiding record types
	let mut response = db
		.db
		.query("SELECT id, path, label, available FROM location WHERE machine IS NONE")
		.await
		.map_err(|e| KipError::Database(e.to_string()))?
		.check()
		.map_err(|e| KipError::Database(e.to_string()))?;

	let rows: Vec<serde_json::Value> = response
		.take(0)
		.map_err(|e| KipError::Database(e.to_string()))?;
	let mut locations = Vec::new();

	for row in rows {
		let id_obj = &row["id"];
		let id = if let Some(id_str) = id_obj.as_str() {
			id_str.split(':').last().unwrap_or(id_str).to_string()
		} else if let Some(id_obj) = id_obj.as_object() {
			id_obj
				.get("key")
				.and_then(|k| k.as_str())
				.unwrap_or("")
				.to_string()
		} else {
			continue;
		};

		let path = row["path"].as_str().unwrap_or("").to_string();
		let label = row["label"].as_str().map(|s| s.to_string());
		let available = row["available"].as_bool().unwrap_or(false);

		locations.push(LocationSummary {
			id,
			path,
			label,
			machine: MachineSummary {
				id: "local".to_string(),
				name: "Local".to_string(),
				kind: MachineKind::Local,
				online: true,
			},
			available,
		});
	}

	Ok(locations)
}

/// Remove a location
pub async fn remove_location(db: &DbHandle, location_id: &str) -> Result<(), KipError> {
	let mut check_response = db
		.db
		.query("SELECT * FROM intent WHERE source = $id LIMIT 1")
		.bind(("id", format!("location:{}", location_id)))
		.await
		.map_err(|e| KipError::Database(e.to_string()))?
		.check()
		.map_err(|e| KipError::Database(e.to_string()))?;

	let existing: Option<serde_json::Value> = check_response
		.take(0)
		.map_err(|e| KipError::Database(e.to_string()))?;
	if existing.is_some() {
		return Err(KipError::InvalidIntentConfig(
			"Location is referenced by active intents".to_string(),
		));
	}

	db.db
		.query("DELETE $id")
		.bind(("id", format!("location:{}", location_id)))
		.await
		.map_err(|e| KipError::Database(e.to_string()))?
		.check()
		.map_err(|e| KipError::Database(e.to_string()))?;

	Ok(())
}

async fn find_location_by_path(db: &DbHandle, path: &PathBuf) -> Result<Option<LocationId>, KipError> {
	let mut response = db
		.db
		.query("SELECT id, path FROM location WHERE path = $path LIMIT 1")
		.bind(("path", path.to_string_lossy().to_string()))
		.await
		.map_err(|e| KipError::Database(e.to_string()))?
		.check()
		.map_err(|e| KipError::Database(e.to_string()))?;

	let row: Option<serde_json::Value> = response
		.take(0)
		.map_err(|e| KipError::Database(e.to_string()))?;

	if let Some(row) = row {
		let id_obj = &row["id"];
		let id = if let Some(id_str) = id_obj.as_str() {
			id_str.split(':').last().unwrap_or(id_str).to_string()
		} else if let Some(id_obj) = id_obj.as_object() {
			id_obj
				.get("key")
				.and_then(|k| k.as_str())
				.unwrap_or("")
				.to_string()
		} else {
			return Ok(None);
		};
		Ok(Some(id))
	} else {
		Ok(None)
	}
}

fn expand_tilde(path: PathBuf) -> Result<PathBuf, KipError> {
	if path.starts_with("~") {
		let home = dirs::home_dir().ok_or_else(|| {
			KipError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "Home directory not found"))
		})?;
		let mut result = home;
		result.push(
			path.strip_prefix("~")
				.map_err(|_| KipError::InvalidIntentConfig("Invalid path with ~".to_string()))?,
		);
		Ok(result)
	} else {
		Ok(path)
	}
}
