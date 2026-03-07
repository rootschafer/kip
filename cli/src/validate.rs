//! Configuration validation - pure inspection, never modifies anything

use std::{
	io::{self, Write},
	path::PathBuf,
};

use anyhow::{Context, Result};
use console::style;
use tracing::{info, warn};

use crate::{
	config,
	folder::{self, Folder},
	git_verify,
};

/// Validation result summary
pub struct ValidationResult {
	pub config_errors: Vec<String>,
	pub folder_errors: Vec<String>,
	pub warnings: Vec<String>,
	pub git_repos_with_changes: Vec<(String, usize)>,
}

/// Validate all configurations - pure inspection, never fails
pub fn validate_all() -> Result<()> {
	let mut result = ValidationResult {
		config_errors: Vec::new(),
		folder_errors: Vec::new(),
		warnings: Vec::new(),
		git_repos_with_changes: Vec::new(),
	};

	println!("{}", style("✅ Validating Configurations").bold());
	println!("{}", "=".repeat(60));
	println!();

	// Load main config
	match config::load_main_config() {
		Ok(_main_config) => {
			info!("Main configuration loaded successfully");
			println!("{} Main configuration loaded", style("✓").green());

			// Load app configs
			match config::load_app_configs() {
				Ok(app_configs) => {
					info!("Loaded {} app configurations", app_configs.len());
					println!("{} Loaded {} app configurations", style("✓").green(), app_configs.len());

					// Resolve all folders
					let mut all_folders = Vec::new();
					let mut resolve_errors = Vec::new();

					for (config_name, config) in &app_configs {
						for folder_config in &config.folder_configs {
							match Folder::from_config(folder_config, config_name, config.metadata.priority) {
								Ok(folder) => {
									all_folders.push(folder);
								}
								Err(e) => {
									resolve_errors
										.push(format!("Failed to resolve folder config in {}: {}", config_name, e));
								}
							}
						}
					}

					if resolve_errors.is_empty() {
						info!("Resolved {} folders", all_folders.len());
						println!("{} Resolved {} folders", style("✓").green(), all_folders.len());
					} else {
						for err in &resolve_errors {
							result.folder_errors.push(err.clone());
							println!("{} {}", style("✗").red(), err);
						}
					}

					info!("Resolved {} folders", all_folders.len());

					// Validate no source overlaps
					match folder::validate_no_source_overlaps(&all_folders) {
						Ok(_) => {
							info!("✓ No source path overlaps detected");
							println!("{} No source path overlaps", style("✓").green());
						}
						Err(e) => {
							result.folder_errors.push(e.to_string());
							println!("{} {}", style("✗").red(), e);
						}
					}

					// Validate all folders have destinations
					match folder::validate_destinations(&all_folders) {
						Ok(_) => {
							info!("✓ All folders have destinations");
							println!("{} All folders have destinations", style("✓").green());
						}
						Err(e) => {
							result.folder_errors.push(e.to_string());
							println!("{} {}", style("✗").red(), e);
						}
					}

					// Check source existence (warnings only)
					let missing_sources = folder::validate_sources_exist(&all_folders);
					if missing_sources.is_empty() {
						info!("✓ All source paths exist");
						println!("{} All source paths exist", style("✓").green());
					} else {
						for warning in &missing_sources {
							result.warnings.push(warning.clone());
							warn!("{}", warning);
							println!("{} {}", style("⚠").yellow(), warning);
						}
					}

					// Validate priority ranges
					let mut priority_warnings = Vec::new();
					for folder in &all_folders {
						if folder.priority < 1 || folder.priority > 1000 {
							priority_warnings.push(format!(
								"Priority {} for {} is outside valid range (1-1000)",
								folder.priority, folder.id
							));
						}
					}
					if priority_warnings.is_empty() {
						println!("{} All priorities in valid range", style("✓").green());
					} else {
						for warning in &priority_warnings {
							result.warnings.push(warning.clone());
							println!("{} {}", style("⚠").yellow(), warning);
						}
					}

					// Validate git repositories with interactive handling
					println!();
					handle_git_validation(&mut result)?;
				}
				Err(e) => {
					result
						.config_errors
						.push(format!("Failed to load app configs: {}", e));
					println!("{} Failed to load app configs: {}", style("✗").red(), e);
				}
			}
		}
		Err(e) => {
			result
				.config_errors
				.push(format!("Failed to load main config: {}", e));
			println!("{} Failed to load main config: {}", style("✗").red(), e);
		}
	}

	// Print summary
	println!();
	println!("{}", "=".repeat(60));
	print_validation_summary(&result);

	Ok(())
}

/// Handle git repository validation with interactive options
fn handle_git_validation(result: &mut ValidationResult) -> Result<()> {
	println!("{}", style("🔍 Git Repository Verification").bold());
	println!("{}", "=".repeat(60));

	// Load git repos config
	let git_config_path = config::config_dir().join("apps").join("git_repos.toml");

	if !git_config_path.exists() {
		println!(
			"{} Git repos config not found: {}",
			style("ℹ").blue(),
			git_config_path.display()
		);
		return Ok(());
	}

	// Parse git repos
	let git_repos_config = std::fs::read_to_string(&git_config_path).context("Failed to read git_repos.toml")?;

	#[derive(serde::Deserialize, Default)]
	struct GitReposConfig {
		#[serde(default, rename = "git")]
		repos: Vec<String>,
	}

	let config: GitReposConfig = toml::from_str(&git_repos_config).context("Failed to parse git_repos.toml")?;

	if config.repos.is_empty() {
		println!("{} No git repositories configured", style("ℹ").blue());
		return Ok(());
	}

	// Verify each repo
	for repo_path_str in &config.repos {
		let repo_path = crate::folder::expand_tilde(&PathBuf::from(repo_path_str));

		match git_verify::verify_git_repo(&repo_path) {
			Ok(verify_result) => {
				if verify_result.is_ready {
					println!("{} {} - Ready", style("✅").green(), style(repo_path.display()).bold());
				} else {
					// Repo has issues - show interactive options
					println!("{} {} - NOT ready", style("❌").red(), style(repo_path.display()).bold());

					for detail in &verify_result.details {
						println!("   {}", style(detail).dim());
					}

					// Only prompt if there are uncommitted changes
					if verify_result.uncommitted_count > 0 {
						result
							.git_repos_with_changes
							.push((repo_path.display().to_string(), verify_result.uncommitted_count));

						println!();
						print!("   [L]azygit  [ENTER] Skip  [S]kip forever: ");
						io::stdout().flush()?;

						let mut input = String::new();
						io::stdin().read_line(&mut input)?;

						match input.trim().to_lowercase().as_str() {
							"l" => {
								// Open lazygit and WAIT for it to close
								println!("   🚀 Opening lazygit for {}...", repo_path.display());
								println!("   (Press 'q' in lazygit to return)");
								println!();

								// Flush stdout before launching lazygit
								io::stdout().flush()?;

								// Spawn lazygit and wait for it to exit
								let mut child = std::process::Command::new("lazygit")
									.current_dir(&repo_path)
									.spawn()
									.context("Failed to launch lazygit")?;

								// Wait for lazygit to close
								let _ = child.wait();

								// Clear screen after lazygit closes
								print!("\x1B[2J\x1B[1;1H");
								io::stdout().flush()?;

								// Re-print the validation header
								println!("{}", style("🔍 Git Repository Verification").bold());
								println!("{}", "=".repeat(60));
							}
							"s" => {
								println!("   {} Will skip {} in future validations", style("✓").green(), repo_path.display());
							}
							_ => {
								// Enter or anything else - just skip
							}
						}
						println!();
					}
				}
			}
			Err(e) => {
				result
					.warnings
					.push(format!("Failed to verify {}: {}", repo_path.display(), e));
				println!("{} {} - Error: {}", style("⚠").yellow(), style(repo_path.display()).bold(), e);
			}
		}
	}

	// Print git summary
	println!("{}", "=".repeat(60));
	let ready_count = config.repos.len() - result.git_repos_with_changes.len();
	println!(
		"Git Summary: {} ready, {} with changes",
		style(ready_count).green().bold(),
		style(result.git_repos_with_changes.len()).yellow().bold()
	);

	Ok(())
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

/// Print validation summary
fn print_validation_summary(result: &ValidationResult) {
	let mut has_errors = false;

	if !result.config_errors.is_empty() {
		has_errors = true;
		println!();
		println!("{}", style("Configuration Errors:").bold().red());
		for err in &result.config_errors {
			println!("  • {}", err);
		}
	}

	if !result.folder_errors.is_empty() {
		has_errors = true;
		println!();
		println!("{}", style("Folder Errors:").bold().red());
		for err in &result.folder_errors {
			println!("  • {}", err);
		}
	}

	if !result.warnings.is_empty() {
		println!();
		println!("{}", style("Warnings:").bold().yellow());
		for warning in &result.warnings {
			println!("  • {}", warning);
		}
	}

	println!();
	if has_errors {
		println!("{} Validation completed with errors", style("⚠").yellow().bold());
	} else if !result.warnings.is_empty() {
		println!("{} Validation completed with warnings", style("⚠").yellow().bold());
	} else {
		println!("{} All validations passed!", style("✅").green().bold());
	}
}

/// Validate a single configuration file
pub fn validate_config_file(path: &std::path::Path) -> Result<()> {
	use std::fs;

	let content = fs::read_to_string(path).with_context(|| format!("Failed to read config: {}", path.display()))?;

	// Try to parse as AppConfig
	let result: Result<crate::config::AppConfig, toml::de::Error> = toml::from_str(&content);

	match result {
		Ok(_) => {
			info!("✓ Config is valid: {}", path.display());
			Ok(())
		}
		Err(e) => {
			anyhow::bail!("Invalid config {}: {}", path.display(), e);
		}
	}
}

/// Check for common configuration mistakes
pub fn lint_configs() -> Result<Vec<String>> {
	let app_configs = config::load_app_configs()?;
	let mut suggestions = Vec::new();

	for (config_name, config) in &app_configs {
		// Check for low priority on important-sounding configs
		if config.metadata.priority < 500
			&& (config.metadata.name.to_lowercase().contains("secret")
				|| config.metadata.name.to_lowercase().contains("identity")
				|| config.metadata.name.to_lowercase().contains("ssh")
				|| config.metadata.name.to_lowercase().contains("gpg")
				|| config.metadata.name.to_lowercase().contains("key")
				|| config.metadata.name.to_lowercase().contains("wallet"))
		{
			suggestions.push(format!(
				"Config '{}' has low priority ({}) but name suggests it's important. Consider priority >= 900",
				config_name, config.metadata.priority
			));
		}

		// Check for zip on already-compressed formats
		for folder in &config.folder_configs {
			for dest in &folder.destinations {
				if dest.zip {
					let source_str = folder.source.to_string_lossy().to_lowercase();
					if source_str.contains(".zip")
						|| source_str.contains(".tar")
						|| source_str.contains(".gz")
						|| source_str.contains(".7z")
					{
						suggestions.push(format!(
							"Folder {} has zip=true for destination {} but source appears to already be compressed",
							folder.source.display(),
							dest.path
						));
					}
				}
			}
		}
	}

	Ok(suggestions)
}
