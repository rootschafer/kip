//! Type-safe destination handling for rsync operations

use std::path::PathBuf;

/// SSH connection options
#[derive(Debug, Clone)]
pub struct SshOptions {
	/// SSH identity file path
	pub identity_file: Option<PathBuf>,
	/// SSH port
	pub port: u16,
	/// Additional SSH options
	pub extra_options: Vec<String>,
}

impl Default for SshOptions {
	fn default() -> Self {
		Self {
			identity_file: None,
			port: 22,
			extra_options: vec![],
		}
	}
}

impl SshOptions {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn with_identity(mut self, path: PathBuf) -> Self {
		self.identity_file = Some(path);
		self
	}

	pub fn with_port(mut self, port: u16) -> Self {
		self.port = port;
		self
	}

	pub fn with_option(mut self, option: String) -> Self {
		self.extra_options.push(option);
		self
	}

	/// Build SSH command arguments for rsync -e flag
	pub fn build_rsync_ssh_command(&self) -> String {
		let mut cmd = String::from("ssh");

		// Add identity file if specified
		if let Some(ref key) = self.identity_file {
			cmd.push_str(&format!(" -i {}", key.display()));
		}

		// Add port if not default
		if self.port != 22 {
			cmd.push_str(&format!(" -p {}", self.port));
		}

		// Add default options for automated/rsync usage
		cmd.push_str(" -o StrictHostKeyChecking=no");
		cmd.push_str(" -o UserKnownHostsFile=/dev/null");
		cmd.push_str(" -o BatchMode=yes");
		cmd.push_str(" -o IdentitiesOnly=yes");

		// Add any extra options
		for option in &self.extra_options {
			cmd.push_str(&format!(" -o {}", option));
		}

		cmd
	}

	/// Build SSH command for direct execution (mkdir, etc.)
	pub fn build_ssh_command(&self, user: &str, host: &str) -> String {
		let mut cmd = self.build_rsync_ssh_command();
		cmd.push_str(&format!(" {}@{}", user, host));
		cmd
	}
}

/// Backup destination - type-safe representation
#[derive(Debug, Clone)]
pub enum Destination {
	/// Local filesystem path
	Local(PathBuf),
	/// Remote SSH destination
	Ssh {
		host: String,
		user: String,
		port: u16,
		path: PathBuf,
		ssh_options: SshOptions,
	},
}

impl Destination {
	/// Create a local destination
	pub fn local<P: Into<PathBuf>>(path: P) -> Self {
		Self::Local(path.into())
	}

	/// Create an SSH destination
	pub fn ssh<P: Into<PathBuf>>(
		host: impl Into<String>,
		user: impl Into<String>,
		path: P,
	) -> Self {
		Self::Ssh {
			host: host.into(),
			user: user.into(),
			port: 22,
			path: path.into(),
			ssh_options: SshOptions::default(),
		}
	}

	/// Set SSH port for SSH destinations
	pub fn with_port(mut self, port: u16) -> Self {
		if let Self::Ssh { port: p, .. } = &mut self {
			*p = port;
		}
		self
	}

	/// Set SSH identity file for SSH destinations
	pub fn with_identity(mut self, path: PathBuf) -> Self {
		if let Self::Ssh { ssh_options, .. } = &mut self {
			ssh_options.identity_file = Some(path);
		}
		self
	}

	/// Check if this is a local destination
	pub fn is_local(&self) -> bool {
		matches!(self, Self::Local(_))
	}

	/// Check if this is an SSH destination
	pub fn is_ssh(&self) -> bool {
		matches!(self, Self::Ssh { .. })
	}

	/// Get the path component
	pub fn path(&self) -> &PathBuf {
		match self {
			Self::Local(path) => path,
			Self::Ssh { path, .. } => path,
		}
	}

	/// Get SSH options if this is an SSH destination
	pub fn ssh_options(&self) -> Option<&SshOptions> {
		match self {
			Self::Ssh { ssh_options, .. } => Some(ssh_options),
			_ => None,
		}
	}

	/// Format destination for rsync command
	/// For local: just the path
	/// For SSH: user@host:path
	pub fn format_for_rsync(&self) -> String {
		match self {
			Self::Local(path) => path.display().to_string(),
			Self::Ssh { user, host, path, .. } => {
				format!("{}@{}:{}", user, host, path.display())
			}
		}
	}

	/// Get the parent directory path for mkdir operations
	pub fn parent_path(&self) -> Option<String> {
		match self {
			Self::Local(path) => path.parent().map(|p| p.display().to_string()),
			Self::Ssh { user, host, path, .. } => {
				path.parent().map(|p| format!("{}@{}:{}", user, host, p.display()))
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_local_destination() {
		let dest = Destination::local("/tmp/backup");
		assert!(dest.is_local());
		assert!(!dest.is_ssh());
		assert_eq!(dest.format_for_rsync(), "/tmp/backup");
	}

	#[test]
	fn test_ssh_destination() {
		let dest = Destination::ssh("example.com", "user", "/backup");
		assert!(!dest.is_local());
		assert!(dest.is_ssh());
		assert_eq!(dest.format_for_rsync(), "user@example.com:/backup");
	}

	#[test]
	fn test_ssh_with_port() {
		let dest = Destination::ssh("example.com", "user", "/backup").with_port(2222);
		assert_eq!(dest.format_for_rsync(), "user@example.com:/backup");
		if let Destination::Ssh { port, .. } = dest {
			assert_eq!(port, 2222);
		}
	}

	#[test]
	fn test_ssh_options_command() {
		let opts = SshOptions::new()
			.with_port(2222)
			.with_option("IdentitiesOnly=yes".to_string());

		let cmd = opts.build_rsync_ssh_command();
		assert!(cmd.contains("-p 2222"));
		assert!(cmd.contains("-o IdentitiesOnly=yes"));
	}
}
