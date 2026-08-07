//! rclone command wrapper

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Instant;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tracing::debug;

use crate::destination::CloudDestination;
use crate::error::{CloudError, Result};
use crate::stats::CloudStats;

/// rclone wrapper for cloud operations
pub struct Rclone {
    /// Path to rclone binary (default: search PATH)
    rclone_path: Option<PathBuf>,
    /// Additional rclone flags
    extra_flags: Vec<String>,
    /// Whether to use verbose output
    verbose: bool,
    /// Whether this is a dry-run
    dry_run: bool,
}

impl Rclone {
    /// Create a new rclone instance
    pub fn new() -> Self {
        Self {
            rclone_path: None,
            extra_flags: Vec::new(),
            verbose: false,
            dry_run: false,
        }
    }

    /// Set custom rclone path
    pub fn with_rclone_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.rclone_path = Some(path.into());
        self
    }

    /// Add extra rclone flags
    pub fn with_flag(mut self, flag: impl Into<String>) -> Self {
        self.extra_flags.push(flag.into());
        self
    }

    /// Enable verbose output
    pub fn with_verbose(mut self) -> Self {
        self.verbose = true;
        self
    }

    /// Enable dry-run mode
    pub fn with_dry_run(mut self) -> Self {
        self.dry_run = true;
        self
    }

    /// Check if rclone is installed and available
    pub async fn check_installed() -> Result<bool> {
        let output = Command::new("rclone")
            .arg("--version")
            .output()
            .await;

        match output {
            Ok(output) if output.status.success() => Ok(true),
            Ok(output) => {
                debug!("rclone version check failed: {}", String::from_utf8_lossy(&output.stderr));
                Ok(false)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(CloudError::RcloneCommandFailed(e.to_string())),
        }
    }

    /// Get rclone version string
    pub async fn version() -> Result<String> {
        let output = Command::new("rclone")
            .arg("--version")
            .output()
            .await
            .map_err(|e| CloudError::RcloneCommandFailed(e.to_string()))?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            Ok(stdout.lines().next().unwrap_or("unknown").to_string())
        } else {
            Err(CloudError::RcloneCommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ))
        }
    }

    /// List configured remotes
    pub async fn list_remotes() -> Result<Vec<String>> {
        let output = Command::new("rclone")
            .arg("listremotes")
            .output()
            .await
            .map_err(|e| CloudError::RcloneCommandFailed(e.to_string()))?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let remotes = stdout
                .lines()
                .filter_map(|line| {
                    // Remove trailing colon from remote names
                    line.strip_suffix(':').map(|s| s.to_string())
                })
                .collect();
            Ok(remotes)
        } else {
            Err(CloudError::RcloneCommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ))
        }
    }

    /// Copy files from source to destination (like cp, doesn't delete)
    pub async fn copy<P: AsRef<Path>>(
        &self,
        source: P,
        dest: &CloudDestination,
        dest_subpath: &str,
    ) -> Result<CloudStats> {
        let full_dest = format!("{}/{}", dest.full_path(), dest_subpath);
        self.run_rclone_command("copy", source.as_ref(), &full_dest)
            .await
    }

    /// Sync files from source to destination (like rsync, deletes extra files at dest)
    pub async fn sync<P: AsRef<Path>>(
        &self,
        source: P,
        dest: &CloudDestination,
        dest_subpath: &str,
    ) -> Result<CloudStats> {
        let full_dest = format!("{}/{}", dest.full_path(), dest_subpath);
        self.run_rclone_command("sync", source.as_ref(), &full_dest)
            .await
    }

    /// Move files from source to destination (copies then deletes source)
    pub async fn move_files<P: AsRef<Path>>(
        &self,
        source: P,
        dest: &CloudDestination,
        dest_subpath: &str,
    ) -> Result<CloudStats> {
        let full_dest = format!("{}/{}", dest.full_path(), dest_subpath);
        self.run_rclone_command("move", source.as_ref(), &full_dest)
            .await
    }

    /// Check if source and destination match (checksum comparison)
    pub async fn check<P: AsRef<Path>>(
        &self,
        source: P,
        dest: &CloudDestination,
        dest_subpath: &str,
    ) -> Result<CloudStats> {
        let full_dest = format!("{}/{}", dest.full_path(), dest_subpath);
        self.run_rclone_command("check", source.as_ref(), &full_dest)
            .await
    }

    /// List files in cloud destination
    pub async fn list(&self, dest: &CloudDestination, path: &str) -> Result<Vec<String>> {
        let full_path = if path.is_empty() {
            dest.full_path()
        } else {
            format!("{}/{}", dest.full_path(), path)
        };

        let output = self
            .build_command("lsf", &[&full_path])
            .output()
            .await
            .map_err(|e| CloudError::RcloneCommandFailed(e.to_string()))?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            Ok(stdout.lines().map(|s| s.to_string()).collect())
        } else {
            Err(CloudError::RcloneCommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ))
        }
    }

    /// Get disk space info for a remote
    pub async fn disk_usage(&self, dest: &CloudDestination) -> Result<DiskUsage> {
        let full_path = dest.full_path();
        let output = self
            .build_command("about", &[&full_path])
            .output()
            .await
            .map_err(|e| CloudError::RcloneCommandFailed(e.to_string()))?;

        if output.status.success() {
            // Parse rclone about output
            // Format varies by provider, so we'll do basic parsing
            let stdout = String::from_utf8_lossy(&output.stdout);
            parse_about_output(&stdout)
        } else {
            Err(CloudError::RcloneCommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ))
        }
    }

    /// Build rclone command
    fn build_command(&self, command: &str, args: &[&str]) -> Command {
        let mut cmd = if let Some(ref path) = self.rclone_path {
            Command::new(path)
        } else {
            Command::new("rclone")
        };
        cmd.arg(command);

        // Add extra flags
        for flag in &self.extra_flags {
            cmd.arg(flag);
        }

        // Add verbose if enabled
        if self.verbose {
            cmd.arg("-v");
        }

        // Add dry-run if enabled
        if self.dry_run {
            cmd.arg("--dry-run");
        }

        // Add standard flags for reliable operation
        cmd.arg("--transfers=4"); // Parallel transfers
        cmd.arg("--checkers=8"); // Parallel checkers

        // Add path arguments
        for arg in args {
            cmd.arg(arg);
        }

        cmd
    }

    /// Run rclone command and parse stats
    async fn run_rclone_command(&self, command: &str, source: &Path, dest: &str) -> Result<CloudStats> {
        let start = Instant::now();

        let source_str = source.to_str().ok_or_else(|| {
            CloudError::InvalidPath("Source path contains invalid UTF-8".to_string())
        })?;

        let mut cmd = self.build_command(command, &[source_str, dest]);

        // Add stats output
        cmd.arg("--stats=1s");
        cmd.arg("--stats-log-level=NOTICE");

        debug!("Running rclone: {:?}", cmd);

        let mut child = cmd
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    CloudError::RcloneNotFound
                } else {
                    CloudError::RcloneCommandFailed(e.to_string())
                }
            })?;

        let mut stats = CloudStats::new();
        let mut stderr_output = String::new();

        // Read stderr for progress and stats
        if let Some(mut stderr) = child.stderr.take() {
            let mut reader = BufReader::new(&mut stderr);
            let mut line = String::new();

            while reader.read_line(&mut line).await? > 0 {
                stderr_output.push_str(&line);

                // Parse rclone stats from stderr
                // Format: "Transferred: 100.000 MiB / 100.000 MiB, 100%, 1.000 MiB/s, ETA 0s"
                if line.contains("Transferred:") {
                    if let Some(parsed) = parse_rclone_stats(&line) {
                        stats.bytes_transferred = parsed.bytes_transferred;
                        stats.files_transferred = parsed.files_transferred;
                        stats.avg_speed_bps = parsed.avg_speed_bps;
                    }
                }

                line.clear();
            }
        }

        // Wait for completion
        let status = child
            .wait()
            .await
            .map_err(|e| CloudError::RcloneCommandFailed(e.to_string()))?;

        stats.duration_secs = start.elapsed().as_secs_f64();
        stats.dry_run = self.dry_run;

        if !status.success() {
            // Check for specific error types
            let error_msg = stderr_output.to_lowercase();
            if error_msg.contains("authentication failed") || error_msg.contains("oauth") {
                return Err(CloudError::AuthenticationFailed(
                    dest.to_string(),
                    stderr_output,
                ));
            }

            if error_msg.contains("rate limit") || error_msg.contains("too many requests") {
                return Err(CloudError::RateLimitExceeded(dest.to_string()));
            }

            return Err(CloudError::RcloneCommandFailed(stderr_output));
        }

        Ok(stats)
    }
}

impl Default for Rclone {
    fn default() -> Self {
        Self::new()
    }
}

/// Disk usage information from rclone about
#[derive(Debug, Clone, Default)]
pub struct DiskUsage {
    pub total_bytes: Option<u64>,
    pub used_bytes: Option<u64>,
    pub free_bytes: Option<u64>,
    pub trashed_bytes: Option<u64>,
    pub other_bytes: Option<u64>,
}

impl DiskUsage {
    pub fn usage_percent(&self) -> Option<f64> {
        match (self.used_bytes, self.total_bytes) {
            (Some(used), Some(total)) if total > 0 => Some((used as f64 / total as f64) * 100.0),
            _ => None,
        }
    }
}

/// Parse rclone about output
fn parse_about_output(output: &str) -> Result<DiskUsage> {
    let mut usage = DiskUsage::default();

    for line in output.lines() {
        let line = line.trim();

        // Parse lines like "Total: 15.000 GiB"
        if line.starts_with("Total:") {
            if let Some(value) = parse_size_value(line.strip_prefix("Total:").unwrap_or("")) {
                usage.total_bytes = Some(value);
            }
        }

        // Parse lines like "Used: 5.000 GiB"
        if line.starts_with("Used:") {
            if let Some(value) = parse_size_value(line.strip_prefix("Used:").unwrap_or("")) {
                usage.used_bytes = Some(value);
            }
        }

        // Parse lines like "Free: 10.000 GiB"
        if line.starts_with("Free:") {
            if let Some(value) = parse_size_value(line.strip_prefix("Free:").unwrap_or("")) {
                usage.free_bytes = Some(value);
            }
        }
    }

    Ok(usage)
}

/// Parse size value like "15.000 GiB" to bytes
fn parse_size_value(value: &str) -> Option<u64> {
    let value = value.trim();
    let parts: Vec<&str> = value.split_whitespace().collect();

    if parts.len() >= 2 {
        let num: f64 = parts[0].parse().ok()?;
        let unit = parts[1].to_lowercase();

        let multiplier = match unit.as_str() {
            "b" => 1u64,
            "kib" | "kb" => 1024u64,
            "mib" | "mb" => 1024u64 * 1024,
            "gib" | "gb" => 1024u64 * 1024 * 1024,
            "tib" | "tb" => 1024u64 * 1024 * 1024 * 1024,
            _ => return None,
        };

        Some((num * multiplier as f64) as u64)
    } else {
        None
    }
}

/// Parse rclone stats output
fn parse_rclone_stats(line: &str) -> Option<CloudStats> {
    // Example: "Transferred: 100.000 MiB / 100.000 MiB, 100%, 1.000 MiB/s, ETA 0s"
    let mut stats = CloudStats::new();

    // Extract bytes transferred
    if let Some(transferred_start) = line.find("Transferred:") {
        let transferred_part = &line[transferred_start + 12..];
        if let Some(mib_pos) = transferred_part.find("MiB") {
            let num_str = transferred_part[..mib_pos].trim();
            if let Ok(num) = num_str.parse::<f64>() {
                stats.bytes_transferred = (num * 1024.0 * 1024.0) as u64;
            }
        } else if let Some(gib_pos) = transferred_part.find("GiB") {
            let num_str = transferred_part[..gib_pos].trim();
            if let Ok(num) = num_str.parse::<f64>() {
                stats.bytes_transferred = (num * 1024.0 * 1024.0 * 1024.0) as u64;
            }
        } else if let Some(kib_pos) = transferred_part.find("KiB") {
            let num_str = transferred_part[..kib_pos].trim();
            if let Ok(num) = num_str.parse::<f64>() {
                stats.bytes_transferred = (num * 1024.0) as u64;
            }
        }
    }

    // Extract speed
    if let Some(speed_start) = line.find("MiB/s") {
        let speed_part = &line[..speed_start];
        if let Some(comma_pos) = speed_part.rfind(',') {
            let num_str = speed_part[comma_pos + 1..].trim();
            if let Ok(num) = num_str.parse::<f64>() {
                stats.avg_speed_bps = (num * 1024.0 * 1024.0) as u64;
            }
        }
    }

    Some(stats)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_size_value() {
        assert_eq!(parse_size_value("15.000 GiB"), Some(15 * 1024 * 1024 * 1024));
        assert_eq!(parse_size_value("100.000 MiB"), Some(100 * 1024 * 1024));
        assert_eq!(parse_size_value("1.000 KiB"), Some(1024));
    }

    #[test]
    fn test_parse_rclone_stats() {
        let line = "Transferred: 100.000 MiB / 100.000 MiB, 100%, 1.000 MiB/s, ETA 0s";
        let stats = parse_rclone_stats(line).unwrap();
        assert_eq!(stats.bytes_transferred, 100 * 1024 * 1024);
        assert_eq!(stats.avg_speed_bps, 1024 * 1024);
    }
}
