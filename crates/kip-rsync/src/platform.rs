//! Platform detection for rsync compatibility

/// Detected platform type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
	/// macOS with BSD rsync
	Macos,
	/// Linux with GNU rsync
	Linux,
	/// Other Unix-like system
	Unix,
	/// Windows (Cygwin/WSL)
	Windows,
}

impl Platform {
	/// Detect the current platform
	pub fn detect() -> Self {
		if cfg!(target_os = "macos") {
			Self::Macos
		} else if cfg!(target_os = "linux") {
			Self::Linux
		} else if cfg!(target_os = "windows") {
			Self::Windows
		} else if cfg!(unix) {
			Self::Unix
		} else {
			Self::Unix // Fallback
		}
	}

	/// Get the appropriate rsync progress flag for this platform
	pub fn progress_flag(&self) -> &'static str {
		match self {
			// macOS BSD rsync doesn't support --info=progress2
			Self::Macos => "--progress",
			// GNU rsync supports the more detailed --info=progress2
			Self::Linux => "--info=progress2",
			// Default to --progress for compatibility
			Self::Unix => "--progress",
			Self::Windows => "--progress",
		}
	}

	/// Get additional rsync flags needed for this platform
	pub fn extra_flags(&self) -> &'static [&'static str] {
		match self {
			// macOS may need additional flags for full compatibility
			Self::Macos => &["--no-specials", "--no-devices"],
			Self::Linux => &["--no-specials", "--no-devices"],
			Self::Unix => &["--no-specials"],
			Self::Windows => &[],
		}
	}

	/// Check if this platform supports --info=progress2
	pub fn supports_info_progress(&self) -> bool {
		match self {
			Self::Linux => true,
			_ => false,
		}
	}
}

/// Get the current platform
pub fn current_platform() -> Platform {
	Platform::detect()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_platform_detect() {
		let platform = Platform::detect();
		// Just verify it doesn't panic
		assert!(matches!(
			platform,
			Platform::Macos | Platform::Linux | Platform::Unix | Platform::Windows
		));
	}

	#[test]
	fn test_progress_flag() {
		let platform = Platform::detect();
		let flag = platform.progress_flag();
		assert!(!flag.is_empty());
	}
}
