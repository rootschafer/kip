//! Drive operations (flash drive and server)
//! 
//! Handles SSH connections with passphrase support via ssh-agent.

use anyhow::{Context, Result};
use console::style;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use tracing::info;

/// Backup destination types
#[derive(Debug, Clone)]
pub enum Drive {
    FlashDrive {
        mount_point: PathBuf,
    },
    Server {
        host: String,
        user: String,
        path: PathBuf,
        identity_file: PathBuf,
        proxy_command: Option<String>,
    },
}

impl Default for Drive {
    fn default() -> Self {
        Drive::FlashDrive {
            mount_point: PathBuf::new(),
        }
    }
}

impl Drive {
    /// Get the root backup path for this drive
    pub fn root_path(&self) -> String {
        match self {
            Drive::FlashDrive { mount_point } => mount_point.to_string_lossy().to_string(),
            Drive::Server { path, .. } => path.to_string_lossy().to_string(),
        }
    }

    /// Check if this drive is accessible
    pub async fn is_accessible(&self) -> bool {
        match self {
            Drive::FlashDrive { mount_point } => {
                mount_point.exists() && mount_point.is_dir()
            }
            Drive::Server {
                host,
                user,
                identity_file,
                proxy_command,
                ..
            } => {
                // Test SSH connectivity with passphrase support
                Self::test_ssh_connection(host, user, identity_file, proxy_command.as_deref())
                    .await
                    .is_ok()
            }
        }
    }

    /// Check if SSH key is loaded in ssh-agent
    fn is_key_in_agent(_identity_file: &PathBuf) -> Result<bool> {
        let output = Command::new("ssh-add")
            .arg("-l")
            .output()
            .context("Failed to run ssh-add -l")?;

        if !output.status.success() {
            // ssh-agent not running or no keys
            return Ok(false);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        
        // ssh-add -l outputs: "256 SHA256:xxx comment (ED25519)"
        // We can't check by file path, so just check if any keys are loaded
        // The actual key will be verified by SSH itself
        Ok(!stdout.is_empty())
    }

    /// Add key to ssh-agent (will prompt for passphrase)
    fn add_key_to_agent(identity_file: &PathBuf) -> Result<()> {
        println!("{} SSH key not loaded in agent", style("🔑").bold());
        println!("   Key: {}", identity_file.display());
        println!();
        println!("{} Attempting to add key to ssh-agent...", style("📡").bold());
        println!("   You will be prompted for the passphrase.");
        println!();

        // Try to add the key - this will prompt for passphrase
        // ssh-add will use SSH_ASKPASS if available, or terminal prompt
        let mut cmd = Command::new("ssh-add");
        cmd.arg(identity_file);
        
        // Set SSH_ASKPASS_REQUIRE to force use of askpass if available
        cmd.env("SSH_ASKPASS_REQUIRE", "auto");
        
        let output = cmd.output()
            .context("Failed to run ssh-add")?;

        if output.status.success() {
            println!("{} Key added to ssh-agent successfully", style("✅").bold().green());
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            
            if stderr.contains("incorrect passphrase") || stderr.contains("bad passphrase") {
                anyhow::bail!("Incorrect passphrase for SSH key");
            } else if stderr.contains("No such file") {
                anyhow::bail!("SSH key file not found: {}", identity_file.display());
            } else {
                anyhow::bail!(
                    "Failed to add SSH key:\nstdout: {}\nstderr: {}",
                    stdout, stderr
                );
            }
        }
    }

    /// Test SSH connection to server with passphrase support
    pub async fn test_ssh_connection(
        host: &str,
        user: &str,
        identity_file: &PathBuf,
        proxy_command: Option<&str>,
    ) -> Result<()> {
        // First, check if key exists
        if !identity_file.exists() {
            anyhow::bail!("SSH identity file not found: {}", identity_file.display());
        }

        // Check if key is in ssh-agent, add if not
        let key_in_agent = Self::is_key_in_agent(identity_file)?;
        if !key_in_agent {
            Self::add_key_to_agent(identity_file)?;
        }

        // Build ssh command using direct process spawning instead of shell
        // This avoids quote escaping issues with ProxyCommand
        let mut cmd = tokio::process::Command::new("ssh");

        // Identity file - always specify it with IdentitiesOnly to ensure correct key is used
        cmd.arg("-i").arg(identity_file);
        cmd.arg("-o").arg("IdentitiesOnly=yes");

        // Cloudflare tunnel connections need more time for initial handshake
        // Use 60 second timeout for tunnel connections, 10 for direct
        let connect_timeout = if proxy_command.is_some() { 60 } else { 10 };
        cmd.arg("-o").arg(format!("ConnectTimeout={}", connect_timeout));
        cmd.arg("-o").arg("ServerAliveInterval=30");
        cmd.arg("-o").arg("ServerAliveCountMax=3");
        
        // Proxy command for Cloudflare tunnel
        if let Some(proxy) = proxy_command {
            cmd.arg("-o").arg(format!("ProxyCommand={}", proxy));
        }
        
        // Test command - just echo
        cmd.arg(format!("{}@{}", user, host))
            .arg("echo connected");

        info!("Testing SSH connection with: ssh {}@{}", user, host);

        let output = cmd
            // Don't clear environment - we need SSH_AUTH_SOCK for ssh-agent
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .output()
            .await
            .context("Failed to execute SSH test")?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);

            // Log full output for debugging
            info!("SSH test failed - stdout: {}, stderr: {}", stdout, stderr);

            // Provide helpful error messages based on the output
            if stderr.contains("Permission denied") || stderr.contains("publickey") {
                anyhow::bail!(
                    "SSH authentication failed.\n\
                     stderr: {}\n\
                     Try running: ssh-add {}", stderr, identity_file.display()
                );
            } else if stderr.contains("Connection timed out") || stderr.contains("timeout") {
                anyhow::bail!(
                    "SSH connection timed out. Check network connectivity to {}.", host
                );
            } else if stderr.contains("No route to host") {
                anyhow::bail!("Cannot reach host {}. Check network connection.", host);
            } else if stderr.contains("cloudflared") {
                anyhow::bail!(
                    "Cloudflare tunnel error. Make sure cloudflared is installed and logged in.\n\
                     stderr: {}", stderr
                );
            } else {
                anyhow::bail!(
                    "SSH connection failed:\nstdout: {}\nstderr: {}",
                    stdout, stderr
                );
            }
        }
    }

    /// Build SSH command prefix for server operations (for interactive use)
    pub fn build_ssh_command(
        host: &str,
        user: &str,
        identity_file: &PathBuf,
        proxy_command: Option<&str>,
    ) -> String {
        let mut cmd = format!("ssh -i {}", identity_file.display());

        // Don't use BatchMode for interactive commands
        if let Some(proxy) = proxy_command {
            cmd.push_str(&format!(" -o ProxyCommand={}", proxy));
        }

        cmd.push_str(&format!(" {}@{}", user, host));
        cmd
    }

    /// Build rsync destination for server (with ssh-agent support)
    pub fn build_rsync_dest(
        host: &str,
        user: &str,
        path: &PathBuf,
        relative_path: &str,
        identity_file: &PathBuf,
        proxy_command: Option<&str>,
    ) -> String {
        // Build SSH options for rsync -e flag
        let mut ssh_opts = String::from("ssh");

        ssh_opts.push_str(&format!(" -i {}", identity_file.display()));
        
        // Don't use BatchMode - rely on ssh-agent
        if let Some(proxy) = proxy_command {
            ssh_opts.push_str(&format!(" -o ProxyCommand={}", proxy));
        }

        // Build full rsync destination
        let dest = format!("{}@{}:{}/{}", user, host, path.display(), relative_path);

        format!(" -e '{}' {}", ssh_opts, dest)
    }

    /// Build rsync command for this drive
    pub fn rsync_dest(&self, relative_path: &str) -> String {
        match self {
            Drive::FlashDrive { mount_point } => {
                mount_point.join(relative_path).to_string_lossy().to_string()
            }
            Drive::Server {
                host,
                user,
                path,
                identity_file,
                proxy_command,
            } => {
                Self::build_rsync_dest(
                    host,
                    user,
                    path,
                    relative_path,
                    identity_file,
                    proxy_command.as_deref(),
                )
            }
        }
    }
}

/// Check if flash drive is mounted
pub fn is_flash_drive_mounted(mount_point: &PathBuf) -> bool {
    mount_point.exists() && mount_point.is_dir()
}

/// Get available space on flash drive using statfs
pub fn flash_drive_free_space(mount_point: &PathBuf) -> Result<u64> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let path_c = CString::new(mount_point.as_os_str().as_bytes())
        .context("Failed to convert path to C string")?;

    let mut stat = unsafe { std::mem::zeroed::<libc::statfs>() };

    let result = unsafe { libc::statfs(path_c.as_ptr(), &mut stat) };

    if result == 0 {
        Ok(stat.f_bavail as u64 * stat.f_bsize as u64)
    } else {
        anyhow::bail!("Failed to get filesystem stats")
    }
}

/// Get total size of flash drive
pub fn flash_drive_total_size(mount_point: &PathBuf) -> Result<u64> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let path_c = CString::new(mount_point.as_os_str().as_bytes())
        .context("Failed to convert path to C string")?;

    let mut stat = unsafe { std::mem::zeroed::<libc::statfs>() };

    let result = unsafe { libc::statfs(path_c.as_ptr(), &mut stat) };

    if result == 0 {
        Ok(stat.f_blocks as u64 * stat.f_bsize as u64)
    } else {
        anyhow::bail!("Failed to get filesystem stats")
    }
}

/// Get used space on flash drive
pub fn flash_drive_used_space(mount_point: &PathBuf) -> Result<u64> {
    let total = flash_drive_total_size(mount_point)?;
    let free = flash_drive_free_space(mount_point)?;
    Ok(total.saturating_sub(free))
}

/// Ensure SSH key is loaded in ssh-agent (helper for backup operations)
pub fn ensure_ssh_key_loaded(identity_file: &PathBuf) -> Result<()> {
    if !Drive::is_key_in_agent(identity_file)? {
        Drive::add_key_to_agent(identity_file)?;
    }
    Ok(())
}
