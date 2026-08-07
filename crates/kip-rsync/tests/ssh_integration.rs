//! SSH integration tests for kip-rsync
//!
//! These tests require Docker to run an SSH server container.
//! Run with: cargo test -- --ignored
//!
//! ## STATUS: Infrastructure complete, needs SSH container debugging
//!
//! The test infrastructure is fully implemented:
//! - ✅ Docker container lifecycle management
//! - ✅ Random port assignment
//! - ✅ SSH key generation
//! - ✅ Progress tracking
//! - ✅ Cleanup on test completion
//!
//! What needs work:
//! - ⚠️ SSH container authentication setup
//! - ⚠️ Container image selection (tried 3 different images)
//!
//! The SSH UNIT tests (`ssh_unit.rs`) fully test:
//! - SSH command building
//! - Option parsing
//! - Error handling
//! - Destination formatting
//!
//! To run these tests, you need:
//! 1. Docker installed and running
//! 2. A working SSH container image
//! 3. sshpass installed (for password auth testing)
//!
//! The tests:
//! 1. Start an SSH container with a test user
//! 2. Generate SSH keys for authentication
//! 3. Run rsync over SSH to the container
//! 4. Verify the backup succeeded
//! 5. Clean up the container and keys

use kip_rsync::{
    Destination, ProgressTracker, Rsync, RsyncStats,
    test_fixture::LocalTestTempDir,
    test_utils::assert_directories_equal,
};
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::time::timeout;

const TEST_TIMEOUT: Duration = Duration::from_secs(120);
const SSH_CONTAINER_IMAGE: &str = "ghcr.io/linuxserver/openssh-server:latest";

/// Test fixture for SSH integration tests
struct SshTestContainer {
    container_id: String,
    container_name: String,
    port: u16,
    key_path: String,
    username: String,
    password: String,
}

impl SshTestContainer {
    /// Start a new SSH container for testing
    async fn start() -> Result<Self, String> {
        // Use unique container name with timestamp
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let container_name = format!("kip_rsync_ssh_test_{}_{}", std::process::id(), timestamp);
        
        eprintln!("Starting SSH container {} on random port", container_name);
        
        // Generate SSH key FIRST (before starting container)
        let key_path = format!("/tmp/kip_rsync_test_key_{}_{}", std::process::id(), timestamp);
        
        eprintln!("Generating SSH key at {}", key_path);
        let keygen = Command::new("ssh-keygen")
            .args(&[
                "-t",
                "ed25519",
                "-f",
                &key_path,
                "-N",
                "",
                "-q",
            ])
            .output()
            .map_err(|e| format!("Failed to generate SSH key: {}", e))?;
        
        if !keygen.status.success() {
            return Err(format!(
                "ssh-keygen failed: {}",
                String::from_utf8_lossy(&keygen.stderr)
            ));
        }
        
        // Set proper permissions on the key (SSH requires 600)
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("Failed to set key permissions: {}", e))?;
        
        let pub_key = std::fs::read_to_string(format!("{}.pub", key_path))
            .map_err(|e| format!("Failed to read public key: {}", e))?;
        
        eprintln!("SSH key generated, starting container...");
        
        // Start container with random port
        let output = Command::new("docker")
            .args(&[
                "run",
                "-d",
                "--name",
                &container_name,
                "-p",
                "2222",  // Random host port -> container port 2222 (linuxserver openssh default)
                "-e",
                "PUID=1000",
                "-e",
                "PGID=1000",
                "-e",
                "TZ=UTC",
                "-e",
                "PASSWORD_ACCESS=true",  // Explicitly enable password auth
                "-e",
                "USER_PASSWORD=testpassword123",
                "-e",
                "USER_NAME=testuser",
                "-e",
                "LOG_STDOUT=true",  // Enable logging for debugging
                SSH_CONTAINER_IMAGE,
            ])
            .output()
            .map_err(|e| format!("Failed to start Docker container: {}", e))?;
        
        if !output.status.success() {
            // Cleanup key on error
            let _ = std::fs::remove_file(&key_path);
            let _ = std::fs::remove_file(format!("{}.pub", key_path));
            return Err(format!(
                "Docker run failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        
        // Install rsync in the container
        eprintln!("Installing rsync in container...");
        let rsync_install = Command::new("docker")
            .args(&["exec", &container_name, "apk", "add", "--quiet", "rsync"])
            .output();
        
        match rsync_install {
            Ok(output) if output.status.success() => eprintln!("✅ rsync installed"),
            Ok(output) => eprintln!("⚠️ rsync install warning: {}", String::from_utf8_lossy(&output.stderr)),
            Err(e) => eprintln!("⚠️ rsync install failed: {}", e),
        }
        
        let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
        
        // Get the assigned port
        let port_output = Command::new("docker")
            .args(&["port", &container_name, "2222"])
            .output()
            .map_err(|e| format!("Failed to get container port: {}", e))?;
        
        let port_str = String::from_utf8_lossy(&port_output.stdout);
        let port: u16 = port_str
            .lines()
            .next()
            .and_then(|line| line.split(':').last())
            .and_then(|p| p.trim().parse().ok())
            .unwrap_or(22222);
        
        eprintln!("Container {} assigned port {}", container_name, port);
        
        // Verify container is running
        let ps_output = Command::new("docker")
            .args(&["ps", "--filter", &format!("name={}", container_name), "--format", "{{.Status}}"])
            .output()
            .ok();
        if let Some(ps) = ps_output {
            eprintln!("Container status: {}", String::from_utf8_lossy(&ps.stdout).trim());
        }
        
        // Wait for container to be ready
        eprintln!("Waiting for SSH container to be ready...");
        
        // First wait for SSH daemon to start listening
        for i in 0..60 {
            tokio::time::sleep(Duration::from_millis(500)).await;
            
            // Check if port is listening
            use std::net::TcpStream;
            if TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok() {
                eprintln!("✅ Port {} is listening after {}ms", port, i * 500);
                break;
            }
            
            if i == 59 {
                // Cleanup on error
                let _ = Command::new("docker").args(&["rm", "-f", &container_name]).output();
                let _ = std::fs::remove_file(&key_path);
                let _ = std::fs::remove_file(format!("{}.pub", key_path));
                return Err(format!("Container port {} never started listening", port));
            }
        }
        
        // Copy public key to container and create backup directory
        eprintln!("Copying SSH key to container...");
        let key_setup = Command::new("docker")
            .args(&[
                "exec",
                &container_name,
                "sh",
                "-c",
                &format!("mkdir -p /config/.ssh && echo '{}' > /config/.ssh/authorized_keys && chmod 600 /config/.ssh/authorized_keys && chown testuser:testuser /config/.ssh/authorized_keys && mkdir -p /config/backup && chown testuser:testuser /config/backup && ls -la /config/.ssh/ /config/backup/", pub_key.trim()),
            ])
            .output();
        
        match key_setup {
            Ok(output) => {
                if output.status.success() {
                    eprintln!("✅ SSH key setup successful");
                } else {
                    eprintln!("❌ SSH key setup failed: {}", String::from_utf8_lossy(&output.stderr));
                }
            }
            Err(e) => eprintln!("❌ SSH key setup command failed: {}", e),
        }
        
        // Additional wait for SSH to pick up the key and rsync to be installed
        eprintln!("Waiting for SSH key to be recognized and rsync to be installed...");
        tokio::time::sleep(Duration::from_secs(10)).await;
        
        // Test SSH connection with key auth
        eprintln!("Testing SSH connection with key...");
        for i in 0..60 {
            tokio::time::sleep(Duration::from_millis(500)).await;
            
            let ssh_test = Command::new("ssh")
                .args(&[
                    "-i", &key_path,  // Use our generated key
                    "-o", "StrictHostKeyChecking=no",
                    "-o", "UserKnownHostsFile=/dev/null",
                    "-o", "BatchMode=yes",
                    "-o", "ConnectTimeout=2",
                    "-o", "ConnectionAttempts=1",
                    "-o", "IdentitiesOnly=yes",  // Only use the specified key
                    "-p", &port.to_string(),
                    "testuser@127.0.0.1",
                    "echo SSH_SUCCESS",
                ])
                .output();
            
            match &ssh_test {
                Ok(out) => {
                    if out.status.success() {
                        let output = String::from_utf8_lossy(&out.stdout);
                        eprintln!("✅ SSH connection successful: {}", output.trim());
                        break;
                    } else {
                        if i < 3 || i == 29 {
                            eprintln!("❌ SSH attempt {} failed (exit {:?}): {}", i, out.status.code(), String::from_utf8_lossy(&out.stderr).lines().take(3).collect::<Vec<_>>().join(" "));
                        }
                    }
                }
                Err(e) => {
                    if i % 10 == 9 || i < 3 {
                        eprintln!("❌ SSH attempt {} failed to execute: {}", i, e);
                    }
                }
            }
            
            if i == 59 {
                // Final debug info
                eprintln!("\n=== Final debug info ===");
                let logs = Command::new("docker").args(&["logs", &container_name]).output();
                if let Ok(logs) = logs {
                    eprintln!("Container logs:\n{}", String::from_utf8_lossy(&logs.stdout).lines().take(15).collect::<Vec<_>>().join("\n"));
                }
                return Err("SSH container failed to authenticate after key setup (timeout after 30 seconds)".to_string());
            }
        }
        
        eprintln!("SSH container ready: {} on port {}", container_id, port);
        
        Ok(Self {
            container_id,
            container_name,
            port,
            key_path,
            username: "testuser".to_string(),
            password: "testpassword123".to_string(),
        })
    }
    
    /// Get the SSH port
    fn port(&self) -> u16 {
        self.port
    }
    
    /// Get the SSH key path
    fn key_path(&self) -> &str {
        &self.key_path
    }
    
    /// Get the username
    fn username(&self) -> &str {
        &self.username
    }
    
    /// Stop and remove the container
    fn stop(&self) {
        eprintln!("Stopping SSH container {}", self.container_name);
        
        let _ = Command::new("docker")
            .args(&["stop", &self.container_name])
            .stderr(Stdio::null())
            .output();
        
        let _ = Command::new("docker")
            .args(&["rm", "-f", &self.container_name])
            .stderr(Stdio::null())
            .output();
        
        // Clean up SSH keys
        let _ = std::fs::remove_file(&self.key_path);
        let _ = std::fs::remove_file(format!("{}.pub", self.key_path));
    }
}

impl Drop for SshTestContainer {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Find an available port for the SSH server
async fn find_available_port() -> u16 {
    use std::net::TcpListener;
    
    // Try ports in a range to avoid conflicts
    for port in 22000..23000 {
        if TcpListener::bind(("127.0.0.1", port)).is_ok() {
            return port;
        }
    }
    
    // Fallback to random port
    0 // Let Docker choose a random port
}

// ============================================================================
// SSH Integration Tests
// ============================================================================

#[tokio::test]
#[ignore = "Requires Docker"]
async fn test_ssh_backup_basic() {
    run_with_timeout(async {
        // Start SSH container
        let container = SshTestContainer::start()
            .await
            .expect("Failed to start SSH container");
        
        // Create test source
        let src = LocalTestTempDir::new("ssh_backup_src")
            .expect("Failed to create source temp dir");
        
        // Create destination path in container
        let dest_path = format!("/config/backup");
        
        // Run rsync over SSH
        let dest = Destination::ssh("127.0.0.1", container.username(), dest_path)
            .with_port(container.port())
            .with_identity(std::path::PathBuf::from(container.key_path()));
        
        let stats = Rsync::new(src.path(), dest)
            .run()
            .await
            .expect("SSH backup failed");
        
        // Verify backup succeeded
        assert!(stats.bytes_transferred > 0, "No bytes transferred");
        assert!(stats.files_transferred > 0, "No files transferred");
        
        eprintln!("SSH backup completed: {} bytes, {} files", 
            stats.bytes_transferred, stats.files_transferred);
    })
    .await;
}

#[tokio::test]
#[ignore = "Requires Docker"]
async fn test_ssh_backup_with_progress() {
    run_with_timeout(async {
        let container = SshTestContainer::start()
            .await
            .expect("Failed to start SSH container");
        
        let src = LocalTestTempDir::new("ssh_progress_src")
            .expect("Failed to create source");
        
        let progress_calls = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let progress_calls_clone = progress_calls.clone();
        
        let tracker = ProgressTracker::new()
            .with_callback(move |stats| {
                let mut calls = progress_calls_clone.lock().unwrap();
                calls.push(stats.bytes_transferred);
            });
        
        let dest_path = format!("/config/backup_progress");
        let dest = Destination::ssh("127.0.0.1", container.username(), dest_path)
            .with_port(container.port())
            .with_identity(std::path::PathBuf::from(container.key_path()));
        
        let _stats = Rsync::new(src.path(), dest)
            .with_progress(tracker)
            .run()
            .await
            .expect("SSH backup with progress failed");
        
        // Verify progress was reported
        let calls = progress_calls.lock().unwrap();
        assert!(!calls.is_empty(), "Progress callback was never called");
        
        eprintln!("Progress reported {} times", calls.len());
    })
    .await;
}

#[tokio::test]
#[ignore = "Requires Docker"]
async fn test_ssh_backup_incremental() {
    run_with_timeout(async {
        let container = SshTestContainer::start()
            .await
            .expect("Failed to start SSH container");
        
        let src = LocalTestTempDir::new("ssh_incremental_src")
            .expect("Failed to create source");
        
        let dest_path = format!("/config/backup_incremental");
        let dest = Destination::ssh("127.0.0.1", container.username(), dest_path)
            .with_port(container.port())
            .with_identity(std::path::PathBuf::from(container.key_path()));
        
        // First backup
        let stats1 = Rsync::new(src.path(), dest.clone())
            .run()
            .await
            .expect("First SSH backup failed");
        
        // Modify source
        std::fs::write(
            src.path().join("modified_file.txt"),
            "Modified content for incremental test"
        ).expect("Failed to modify file");
        
        // Second backup (should be incremental)
        let stats2 = Rsync::new(src.path(), dest)
            .run()
            .await
            .expect("Second SSH backup failed");
        
        // Second backup should transfer less data
        assert!(
            stats2.bytes_transferred < stats1.bytes_transferred,
            "Incremental backup should transfer less data"
        );
        
        eprintln!("First backup: {} bytes, Second backup: {} bytes",
            stats1.bytes_transferred, stats2.bytes_transferred);
    })
    .await;
}

#[tokio::test]
#[ignore = "Requires Docker"]
async fn test_ssh_backup_large_file() {
    run_with_timeout(async {
        let container = SshTestContainer::start()
            .await
            .expect("Failed to start SSH container");
        
        let src = LocalTestTempDir::empty("ssh_large_src")
            .expect("Failed to create source");
        
        // Create a 5MB file
        let large_data: Vec<u8> = (0..255).cycle().take(5 * 1024 * 1024).collect();
        std::fs::write(src.path().join("large_file.bin"), &large_data)
            .expect("Failed to create large file");
        
        let dest_path = format!("/config/backup_large");
        let dest = Destination::ssh("127.0.0.1", container.username(), dest_path)
            .with_port(container.port())
            .with_identity(std::path::PathBuf::from(container.key_path()));
        
        let stats = Rsync::new(src.path(), dest)
            .run()
            .await
            .expect("SSH large file backup failed");
        
        // Verify file was transferred
        assert_eq!(stats.bytes_transferred, large_data.len() as u64);
        
        eprintln!("Large file backup: {} bytes", stats.bytes_transferred);
    })
    .await;
}

/// Helper to run tests with timeout
async fn run_with_timeout<F, T>(test: F) -> T
where
    F: std::future::Future<Output = T>,
{
    timeout(TEST_TIMEOUT, test)
        .await
        .expect("SSH test timed out")
}
