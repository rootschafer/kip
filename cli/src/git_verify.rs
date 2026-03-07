//! Git repository verification
//!
//! Verifies that git repositories are in a backup-ready state:
//! - No uncommitted changes
//! - All commits pushed to remote (optional)

use std::{path::PathBuf, process::Command};

use anyhow::{Context, Result};
use console::style;
use tracing::{debug, info, warn};

/// Git repository verification result
#[derive(Debug, Clone)]
pub struct GitVerificationResult {
	/// Repository path
	pub path: PathBuf,
	/// Whether verification passed
	pub is_ready: bool,
	/// Number of uncommitted changes
	pub uncommitted_count: usize,
	/// Number of commits ahead of remote
	pub commits_ahead: usize,
	/// Number of commits behind remote
	pub commits_behind: usize,
	/// Whether repo has a remote
	pub has_remote: bool,
	/// Details about issues
	pub details: Vec<String>,
}

/// Verify a git repository
pub fn verify_git_repo(path: &PathBuf) -> Result<GitVerificationResult> {
	debug!("Verifying git repo at {}", path.display());

	let mut result = GitVerificationResult {
		path: path.clone(),
		is_ready: true,
		uncommitted_count: 0,
		commits_ahead: 0,
		commits_behind: 0,
		has_remote: false,
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
		.current_dir(path)
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
	let has_remote_output = Command::new("git")
		.args(&["remote"])
		.current_dir(path)
		.output()
		.map(|o| !o.stdout.is_empty())
		.unwrap_or(false);

	result.has_remote = has_remote_output;

	if has_remote_output {
		let rev_list_output = Command::new("git")
			.args(&["rev-list", "--left-right", "--count", "HEAD...@{u}"])
			.current_dir(path)
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
		info!("✓ Git repo at {} is ready for backup", path.display());
	} else {
		warn!("✗ Git repo at {} is NOT ready for backup", path.display());
		for detail in &result.details {
			debug!("  {}", detail);
		}
	}

	Ok(result)
}

/// Check if a path is a git repository
pub fn is_git_repo(path: &PathBuf) -> bool {
	path.join(".git").exists()
}

/// Print verification results
pub fn print_verification_results(results: &[GitVerificationResult]) {
	println!("\n{}", style("🔍 Git Repository Verification").bold());
	println!("{}", "=".repeat(60));

	let mut ready_count = 0;
	let mut not_ready_count = 0;

	for result in results {
		if result.is_ready {
			ready_count += 1;
			println!(
				"{} {} - Ready for backup",
				style("✅").green(),
				style(result.path.display()).bold()
			);
		} else {
			not_ready_count += 1;
			println!("{} {} - NOT ready", style("❌").red(), style(result.path.display()).bold());
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

/// Result of git repo handling
#[derive(Debug, Clone, PartialEq)]
pub enum GitRepoAction {
	/// Backup this repo (git-ignored files only)
	Backup,
	/// Skip this repo for this run
	Skip,
	/// Skip all remaining repos for this run
	SkipAll,
}

/// Interactive prompt for git repos with issues
/// Returns the action to take
pub fn handle_git_repo_with_issues(result: &GitVerificationResult) -> GitRepoAction {
	use std::io::{self, Write};

	println!(
		"\n{} Git repo at {} has issues:",
		style("⚠️").yellow(),
		style(result.path.display()).bold()
	);
	for detail in &result.details {
		println!("   {}", style(detail).dim());
	}

	println!();
	print!("   [L]azygit  [S]kip this  [A]ll skip  [C]ontinue: ");
	io::stdout().flush().unwrap();

	let mut input = String::new();
	io::stdin().read_line(&mut input).unwrap();

	match input.trim().to_lowercase().as_str() {
		"l" => {
			// Open lazygit and wait for it to close
			println!("   🚀 Opening lazygit...");
			println!("   (Press 'q' in lazygit to return)");
			println!();

			io::stdout().flush().unwrap();

			// Spawn lazygit and wait for it to exit
			let mut child = std::process::Command::new("lazygit")
				.current_dir(&result.path)
				.spawn()
				.expect("Failed to launch lazygit");

			// Wait for lazygit to close
			let _ = child.wait();

			// Clear screen after lazygit closes
			print!("\x1B[2J\x1B[1;1H");
			io::stdout().flush().unwrap();

			// Re-verify the repo
			println!("   Re-verifying repository...");
			let new_result = verify_git_repo(&result.path).unwrap();
			if new_result.is_ready {
				println!("   {} Repository is now ready!", style("✅").green());
				GitRepoAction::Backup
			} else {
				println!("   {} Repository still has issues", style("⚠️").yellow());
				handle_git_repo_with_issues(&new_result)
			}
		}
		"s" => {
			println!("   {} Skipping this repository for this run", style("✓").green());
			GitRepoAction::Skip
		}
		"a" => {
			println!("   {} Skipping all remaining repositories for this run", style("✓").green());
			GitRepoAction::SkipAll
		}
		_ => {
			// Enter or anything else - continue anyway
			println!("   {} Backing up git-ignored files only", style("✓").green());
			GitRepoAction::Backup
		}
	}
}
