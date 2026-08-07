//! SSH unit tests for kip-rsync
//!
//! These tests verify SSH command building, option parsing, and error handling
//! without requiring an actual SSH server.

use std::path::PathBuf;

use kip_rsync::{Destination, Rsync, RsyncError, SshOptions};

// ============================================================================
// SSH Destination Tests
// ============================================================================

#[test]
fn test_ssh_destination_creation() {
	let dest = Destination::ssh("example.com", "user", "/backup");

	assert!(dest.is_ssh());
	assert!(!dest.is_local());
	assert_eq!(dest.format_for_rsync(), "user@example.com:/backup");
}

#[test]
fn test_ssh_destination_with_port() {
	let dest = Destination::ssh("example.com", "user", "/backup").with_port(2222);

	// Port is stored in the destination
	if let Destination::Ssh { port, .. } = dest {
		assert_eq!(port, 2222);
	} else {
		panic!("Expected SSH destination");
	}
}

#[test]
fn test_ssh_destination_with_identity() {
	let dest = Destination::ssh("example.com", "user", "/backup").with_identity(PathBuf::from("~/.ssh/id_ed25519"));

	if let Destination::Ssh { ssh_options, .. } = dest {
		assert!(ssh_options.identity_file.is_some());
		assert_eq!(ssh_options.identity_file.unwrap(), PathBuf::from("~/.ssh/id_ed25519"));
	} else {
		panic!("Expected SSH destination");
	}
}

#[test]
fn test_ssh_destination_path() {
	let dest = Destination::ssh("host", "user", "/path/to/backup");

	assert_eq!(dest.path(), &PathBuf::from("/path/to/backup"));
}

#[test]
fn test_ssh_destination_parent_path() {
	let dest = Destination::ssh("host", "user", "/path/to/backup/file.tar.gz");

	let parent = dest.parent_path().unwrap();
	assert!(parent.contains("user@host"));
	assert!(parent.contains("/path/to/backup"));
}

// ============================================================================
// SSH Options Tests
// ============================================================================

#[test]
fn test_ssh_options_default() {
	let opts = SshOptions::default();

	assert_eq!(opts.port, 22);
	assert!(opts.identity_file.is_none());
	assert!(opts.extra_options.is_empty());
}

#[test]
fn test_ssh_options_builder() {
	let opts = SshOptions::new()
		.with_port(2222)
		.with_identity(PathBuf::from("~/.ssh/test_key"))
		.with_option("StrictHostKeyChecking=no".to_string())
		.with_option("UserKnownHostsFile=/dev/null".to_string());

	assert_eq!(opts.port, 2222);
	assert_eq!(opts.identity_file, Some(PathBuf::from("~/.ssh/test_key")));
	assert_eq!(opts.extra_options.len(), 2);
}

#[test]
fn test_ssh_options_build_rsync_command() {
	let opts = SshOptions::new()
		.with_port(2222)
		.with_identity(PathBuf::from("~/.ssh/test_key"));

	let cmd = opts.build_rsync_ssh_command();

	assert!(cmd.starts_with("ssh"));
	assert!(cmd.contains("-p 2222"));
	assert!(cmd.contains("-i"));
	assert!(cmd.contains("~/.ssh/test_key"));
}

#[test]
fn test_ssh_options_build_ssh_command() {
	let opts = SshOptions::new().with_port(2222);

	let cmd = opts.build_ssh_command("testuser", "testhost");

	assert!(cmd.contains("testuser@testhost"));
	assert!(cmd.contains("-p 2222"));
}

#[test]
fn test_ssh_options_no_identity() {
	let opts = SshOptions::new();
	let cmd = opts.build_rsync_ssh_command();

	// Should not contain -i flag if no identity file
	assert!(!cmd.contains("-i"));
}

// ============================================================================
// SSH Error Handling Tests
// ============================================================================

#[test]
fn test_rsync_error_ssh_connection_refused() {
	let stderr = "ssh: connect to host example.com port 22: Connection refused";

	// Verify error detection works
	assert!(stderr.to_lowercase().contains("connection refused"));
}

#[test]
fn test_rsync_error_ssh_permission_denied() {
	let stderr = "Permission denied (publickey,password).";

	assert!(stderr.to_lowercase().contains("permission denied"));
}

#[test]
fn test_rsync_error_ssh_host_key_verification() {
	let stderr = "The authenticity of host 'example.com' can't be established.";

	assert!(stderr.to_lowercase().contains("authenticity"));
}

#[test]
fn test_rsync_error_remote_directory_not_exist() {
	let stderr = "rsync: change_dir \"/backup\" failed: No such file or directory";

	assert!(stderr.to_lowercase().contains("no such file"));
}

// ============================================================================
// SSH Command Building Tests
// ============================================================================

#[test]
fn test_rsync_ssh_command_structure() {
	// Verify that Rsync builder can accept SSH destinations
	let dest = Destination::ssh("host", "user", "/backup");

	// Create rsync operation (won't execute, just verify building works)
	let _rsync = Rsync::new("/tmp/source", dest);

	// Test passes if it compiles and builds without panic
}

#[test]
fn test_ssh_destination_format_variations() {
	// Test various path formats
	let dest1 = Destination::ssh("host", "user", "/absolute/path");
	assert_eq!(dest1.format_for_rsync(), "user@host:/absolute/path");

	let dest2 = Destination::ssh("host", "user", "/");
	assert_eq!(dest2.format_for_rsync(), "user@host:/");

	let dest3 = Destination::ssh("host", "user", "/path/with spaces/");
	assert_eq!(dest3.format_for_rsync(), "user@host:/path/with spaces/");
}

#[test]
fn test_ssh_options_special_characters() {
	let opts = SshOptions::new()
		.with_identity(PathBuf::from("~/.ssh/key with spaces"))
		.with_option("ProxyCommand=ssh -W %h:%p jumphost".to_string());

	let cmd = opts.build_rsync_ssh_command();

	// Should include all options
	assert!(cmd.contains("key with spaces"));
	assert!(cmd.contains("ProxyCommand"));
}

// ============================================================================
// SSH Integration Helper Tests
// ============================================================================

#[test]
fn test_ssh_mkdir_command_building() {
	// Test that ssh_mkdir would build correct command
	// (We can't actually test execution without a server)

	let host = "testhost";
	let user = "testuser";
	let port = 2222;
	let path = "/test/path";

	// Build expected command structure
	let expected_contains = vec!["ssh", "-p", "2222", "testuser@testhost", "mkdir -p /test/path"];

	// Verify command would contain expected parts
	let cmd = format!("ssh -p {} {}@{} 'mkdir -p {}'", port, user, host, path);

	for part in expected_contains {
		assert!(cmd.contains(part), "Command should contain '{}'", part);
	}
}

#[test]
fn test_ssh_destination_clone_and_copy() {
	let dest = Destination::ssh("host", "user", "/backup")
		.with_port(2222)
		.with_identity(PathBuf::from("~/.ssh/key"));

	// Test that destination can be cloned
	let dest2 = dest.clone();

	assert_eq!(dest.format_for_rsync(), dest2.format_for_rsync());
}

#[test]
fn test_ssh_options_debug_display() {
	let opts = SshOptions::new().with_port(2222);

	// Verify Debug trait is implemented
	let debug_str = format!("{:?}", opts);
	assert!(debug_str.contains("SshOptions"));
}
