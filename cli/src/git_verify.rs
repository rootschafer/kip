//! Git repository verification
//!
//! Verifies that git repositories are in a backup-ready state:
//! - No uncommitted changes
//! - All commits pushed to remote

use std::{path::PathBuf, process::Command};

use anyhow::{Context, Result};
use serde::Deserialize;
use tracing::{debug, info, warn};

/// Git repository configuration
#[derive(Debug, Clone, Deserialize)]
pub struct GitRepo {
	/// Path to the repository
	pub path: String,

	/// Name for this repo (for logging)
	pub name: String,

	/// Priority (1-1000)
	#[serde(default = "default_priority")]
	pub priority: u16,

	/// Paths to ignore (gitignore-style patterns)
	#[serde(default)]
	pub ignored: Vec<String>,
}

fn default_priority() -> u16 {
	900
}

/// Git repository verification result
#[derive(Debug, Clone)]
pub struct GitVerificationResult {
	/// Repository path
	pub path: PathBuf,
	/// Repository name
	pub name: String,
	/// Whether verification passed
	pub is_ready: bool,
	/// Number of uncommitted changes
	pub uncommitted_count: usize,
	/// Number of commits ahead of remote
	pub commits_ahead: usize,
	/// Number of commits behind remote
	pub commits_behind: usize,
	/// Details about issues
	pub details: Vec<String>,
}

/// Verify a single git repository
pub fn verify_git_repo(repo: &GitRepo) -> Result<GitVerificationResult> {
	let path = expand_tilde(&repo.path);

	debug!("Verifying git repo: {} at {}", repo.name, path.display());

	let mut result = GitVerificationResult {
		path: path.clone(),
		name: repo.name.clone(),
		is_ready: true,
		uncommitted_count: 0,
		commits_ahead: 0,
		commits_behind: 0,
		details: Vec::new(),
	};

	// Check if path exists and is a git repo
	if !path.exists() {
		result.is_ready = false;
		result
			.details
			.push(format!("Path does not exist: {}", path.display()));
		return Ok(result);
	}

	let git_dir = path.join(".git");
	if !git_dir.exists() {
		result.is_ready = false;
		result
			.details
			.push(format!("Not a git repository: {}", path.display()));
		return Ok(result);
	}

	// Check for uncommitted changes
	let status_output = Command::new("git")
		.args(&["status", "--porcelain"])
		.current_dir(&path)
		.output()
		.context("Failed to run git status")?;

	let status_stdout = String::from_utf8_lossy(&status_output.stdout);
	let uncommitted: Vec<&str> = status_stdout
		.lines()
		.filter(|line| !line.is_empty())
		.collect();

	result.uncommitted_count = uncommitted.len();

	if result.uncommitted_count > 0 {
		result.is_ready = false;
		result
			.details
			.push(format!("{} uncommitted change(s):", result.uncommitted_count));
		for line in uncommitted.iter().take(5) {
			result.details.push(format!("  - {}", line));
		}
		if result.uncommitted_count > 5 {
			result
				.details
				.push(format!("  ... and {} more", result.uncommitted_count - 5));
		}
	}

	// Check for unsynced commits
	// Only check remote sync if we have a remote
	let has_remote = Command::new("git")
		.args(&["remote"])
		.current_dir(&path)
		.output()
		.map(|o| !o.stdout.is_empty())
		.unwrap_or(false);

	if has_remote {
		let rev_list_output = Command::new("git")
			.args(&["rev-list", "--left-right", "--count", "HEAD...@{u}"])
			.current_dir(&path)
			.output();

		if let Ok(output) = rev_list_output {
			let counts = String::from_utf8_lossy(&output.stdout);
			let parts: Vec<&str> = counts.trim().split_whitespace().collect();

			if parts.len() == 2 {
				result.commits_ahead = parts[0].parse().unwrap_or(0);
				result.commits_behind = parts[1].parse().unwrap_or(0);

				if result.commits_ahead > 0 {
					result.is_ready = false;
					result
						.details
						.push(format!("{} commit(s) ahead of remote (not pushed)", result.commits_ahead));
				}

				if result.commits_behind > 0 {
					result
						.details
						.push(format!("{} commit(s) behind remote (pull to sync)", result.commits_behind));
					// Being behind doesn't make it not ready for backup
				}
			}
		}
	}

	if result.is_ready {
		info!("✓ Git repo '{}' is ready for backup", repo.name);
	} else {
		warn!("✗ Git repo '{}' is NOT ready for backup", repo.name);
		for detail in &result.details {
			debug!("  {}", detail);
		}
	}

	Ok(result)
}

/// Verify all git repositories
pub fn verify_git_repos(repos: &[GitRepo]) -> Result<Vec<GitVerificationResult>> {
	let mut results = Vec::new();

	for repo in repos {
		let result = verify_git_repo(repo)?;
		results.push(result);
	}

	Ok(results)
}

/// Expand tilde in path
fn expand_tilde(path: &str) -> PathBuf {
	if path.starts_with('~') {
		if let Some(home) = dirs::home_dir() {
			return home.join(path.trim_start_matches('~').trim_start_matches('/'));
		}
	}
	PathBuf::from(path)
}

/// Print verification results
pub fn print_verification_results(results: &[GitVerificationResult]) {
	use console::style;

	println!("\n{}", style("🔍 Git Repository Verification").bold());
	println!("{}", "=".repeat(60));

	let mut ready_count = 0;
	let mut not_ready_count = 0;

	for result in results {
		if result.is_ready {
			ready_count += 1;
			println!("{} {} - Ready for backup", style("✅").green(), style(&result.name).bold());
		} else {
			not_ready_count += 1;
			println!("{} {} - NOT ready", style("❌").red(), style(&result.name).bold());
			for detail in &result.details {
				println!("   {}", style(detail).dim());
			}
		}
	}

	println!("{}", "=".repeat(60));
	println!(
		"Summary: {} ready, {} not ready",
		style(ready_count).green().bold(),
		style(not_ready_count).yellow().bold()
	);
}
