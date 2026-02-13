use std::path::Path;

use serde::Deserialize;
use tokio::task::JoinHandle;

use crate::db::DbHandle;

const POLL_INTERVAL_SECS: u64 = 5;

pub struct DriveWatcher {
    handle: JoinHandle<()>,
}

impl DriveWatcher {
    pub fn start(db: DbHandle) -> Self {
        let handle = tokio::spawn(async move {
            loop {
                if let Err(e) = poll_volumes(&db).await {
                    eprintln!("drive poll error: {e}");
                }
                tokio::time::sleep(std::time::Duration::from_secs(POLL_INTERVAL_SECS)).await;
            }
        });
        DriveWatcher { handle }
    }

    #[allow(dead_code)]
    pub fn stop(self) {
        self.handle.abort();
    }
}

#[derive(Debug, Clone)]
struct VolumeInfo {
    uuid: String,
    name: String,
    mount_point: String,
    filesystem: String,
    capacity_bytes: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct DiskutilInfo {
    #[serde(default, rename = "VolumeUUID")]
    volume_uuid: Option<String>,
    #[serde(default)]
    volume_name: Option<String>,
    #[serde(default)]
    mount_point: Option<String>,
    #[serde(default)]
    filesystem_type: Option<String>,
    #[serde(default)]
    total_size: Option<i64>,
    #[serde(default)]
    internal: Option<bool>,
}

async fn poll_volumes(db: &DbHandle) -> Result<(), String> {
    let volumes = discover_mounted_volumes().await;
    let seen_uuids: Vec<String> = volumes.iter().map(|v| v.uuid.clone()).collect();

    for vol in &volumes {
        sync_drive_to_db(db, vol).await?;
    }

    mark_disconnected_drives(db, &seen_uuids).await?;

    Ok(())
}

async fn discover_mounted_volumes() -> Vec<VolumeInfo> {
    let volumes_dir = Path::new("/Volumes");
    let entries = match std::fs::read_dir(volumes_dir) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("failed to read /Volumes: {e}");
            return Vec::new();
        }
    };

    let mut volumes = Vec::new();

    for entry in entries.flatten() {
        // Skip symlinks (boot volume "Macintosh HD" is a symlink to /)
        if let Ok(meta) = entry.metadata() {
            if meta.file_type().is_symlink() {
                continue;
            }
        }
        // Also check via symlink_metadata for reliability
        if let Ok(meta) = std::fs::symlink_metadata(entry.path()) {
            if meta.file_type().is_symlink() {
                continue;
            }
        }

        let path = entry.path();
        let path_str = path.to_string_lossy().to_string();

        match get_diskutil_info(&path_str).await {
            Some(info) => {
                // Skip internal drives
                if info.internal.unwrap_or(false) {
                    continue;
                }
                // Skip volumes without UUID
                let uuid = match info.volume_uuid {
                    Some(ref u) if !u.is_empty() => u.clone(),
                    _ => continue,
                };

                volumes.push(VolumeInfo {
                    uuid,
                    name: info.volume_name.unwrap_or_else(|| "Untitled".into()),
                    mount_point: info.mount_point.unwrap_or(path_str),
                    filesystem: info.filesystem_type.unwrap_or_default(),
                    capacity_bytes: info.total_size.unwrap_or(0),
                });
            }
            None => continue,
        }
    }

    volumes
}

async fn get_diskutil_info(volume_path: &str) -> Option<DiskutilInfo> {
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        tokio::process::Command::new("diskutil")
            .args(["info", "-plist", volume_path])
            .output(),
    )
    .await
    .ok()?
    .ok()?;

    if !output.status.success() {
        return None;
    }

    plist::from_bytes(&output.stdout).ok()
}

async fn sync_drive_to_db(db: &DbHandle, vol: &VolumeInfo) -> Result<(), String> {
    let limitations = detect_limitations(&vol.filesystem);

    db.db
        .query(
            "UPSERT type::record('drive', $uuid) CONTENT {
                name: $name,
                uuid: $uuid,
                filesystem: $filesystem,
                capacity_bytes: $capacity,
                mount_point: $mount_point,
                connected: true,
                last_seen: time::now(),
                limitations: $limitations,
            }",
        )
        .bind(("uuid", vol.uuid.clone()))
        .bind(("name", vol.name.clone()))
        .bind(("filesystem", vol.filesystem.clone()))
        .bind(("capacity", vol.capacity_bytes))
        .bind(("mount_point", vol.mount_point.clone()))
        .bind(("limitations", limitations))
        .await
        .map_err(|e| e.to_string())?
        .check()
        .map_err(|e| e.to_string())?;

    Ok(())
}

async fn mark_disconnected_drives(db: &DbHandle, seen_uuids: &[String]) -> Result<(), String> {
    db.db
        .query(
            "UPDATE drive SET connected = false, mount_point = NONE
             WHERE connected = true AND uuid NOT IN $seen_uuids",
        )
        .bind(("seen_uuids", seen_uuids.to_vec()))
        .await
        .map_err(|e| e.to_string())?
        .check()
        .map_err(|e| e.to_string())?;

    Ok(())
}

fn detect_limitations(filesystem: &str) -> Option<serde_json::Value> {
    match filesystem.to_lowercase().as_str() {
        "msdos" | "fat32" | "fat16" => Some(serde_json::json!({
            "max_file_size": 4_294_967_295_i64
        })),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_limitations_fat32() {
        let lim = detect_limitations("msdos").unwrap();
        assert_eq!(lim["max_file_size"], 4_294_967_295_i64);
    }

    #[test]
    fn test_detect_limitations_apfs() {
        assert!(detect_limitations("apfs").is_none());
    }

    #[test]
    fn test_detect_limitations_exfat() {
        assert!(detect_limitations("exfat").is_none());
    }

    #[test]
    fn test_parse_diskutil_plist() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>VolumeUUID</key>
    <string>C97B5B92-3557-307E-847C-FB0DCB4A8C2F</string>
    <key>VolumeName</key>
    <string>SOMETHING</string>
    <key>MountPoint</key>
    <string>/Volumes/SOMETHING</string>
    <key>FilesystemType</key>
    <string>msdos</string>
    <key>TotalSize</key>
    <integer>122768752640</integer>
    <key>Internal</key>
    <false/>
</dict>
</plist>"#;

        let info: DiskutilInfo = plist::from_bytes(xml).unwrap();
        assert_eq!(info.volume_uuid.as_deref(), Some("C97B5B92-3557-307E-847C-FB0DCB4A8C2F"));
        assert_eq!(info.volume_name.as_deref(), Some("SOMETHING"));
        assert_eq!(info.mount_point.as_deref(), Some("/Volumes/SOMETHING"));
        assert_eq!(info.filesystem_type.as_deref(), Some("msdos"));
        assert_eq!(info.total_size, Some(122768752640));
        assert_eq!(info.internal, Some(false));
    }
}
