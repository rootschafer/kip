//! Utility functions for Kip core

use std::path::Path;

/// Calculate BLAKE3 hash of file contents
pub fn hash_file(path: &Path) -> Result<String, std::io::Error> {
    let contents = std::fs::read(path)?;
    let hash = blake3::hash(&contents);
    Ok(hash.to_hex().to_string())
}

/// Get file type icon emoji
pub fn file_type_icon(path: &Path) -> &'static str {
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    
    match ext.as_str() {
        "rs" | "js" | "ts" | "jsx" | "tsx" | "py" | "cpp" | "c" | "h" | "hpp" | "go" | "rb" | "sh" => "📝",
        "txt" | "md" | "markdown" | "pdf" | "doc" | "docx" | "rtf" => "📄",
        "json" | "yaml" | "yml" | "toml" | "ini" | "env" | "config" => "⚙️",
        "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" | "ico" => "🖼️",
        "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" => "🎵",
        "mp4" | "mov" | "avi" | "mkv" | "webm" => "🎬",
        "zip" | "tar" | "gz" | "7z" | "rar" | "bz2" => "📦",
        "exe" | "dll" | "so" | "dylib" | "bin" | "app" => "⚡",
        _ => "📎",
    }
}

/// Check if path is a hidden file/directory
pub fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.starts_with('.'))
        .unwrap_or(false)
}
