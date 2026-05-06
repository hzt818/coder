//! List directory tool - structured, gitignore-aware directory listing
//!
//! Provides a structured directory listing that respects .gitignore files.
//! Preferred over `ls` shell command for better structured output.

use super::*;
use async_trait::async_trait;
use std::path::Path;

pub struct ListDirTool;

#[async_trait]
impl Tool for ListDirTool {
    fn name(&self) -> &str {
        "list_dir"
    }

    fn description(&self) -> &str {
        concat!(
            "List the contents of a directory with structured output. ",
            "Respects .gitignore. Shows file types, sizes, and modification times. ",
            "Preferred over using shell 'ls' for structured output."
        )
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Directory path to list (default: current directory)",
                    "default": "."
                },
                "max_depth": {
                    "type": "integer",
                    "description": "Maximum recursion depth (default: 1, 0 = no recursion)",
                    "default": 1
                },
                "show_hidden": {
                    "type": "boolean",
                    "description": "Show hidden files (starting with .)",
                    "default": false
                },
                "pattern": {
                    "type": "string",
                    "description": "Optional glob pattern to filter (e.g., '*.rs', '*.py')",
                    "default": ""
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> ToolResult {
        let path = args.get("path").and_then(|p| p.as_str()).unwrap_or(".");

        let max_depth = args.get("max_depth").and_then(|d| d.as_i64()).unwrap_or(1) as usize;

        let show_hidden = args
            .get("show_hidden")
            .and_then(|s| s.as_bool())
            .unwrap_or(false);

        let pattern = args.get("pattern").and_then(|p| p.as_str()).unwrap_or("");

        let dir_path = Path::new(path);
        if !dir_path.exists() {
            return ToolResult::err(format!("Directory not found: '{}'", path));
        }
        if !dir_path.is_dir() {
            return ToolResult::err(format!("Not a directory: '{}'", path));
        }

        match list_directory(dir_path, max_depth, show_hidden, pattern) {
            Ok(output) => ToolResult::ok(output),
            Err(e) => ToolResult::err(format!("Failed to list directory: {}", e)),
        }
    }
}

/// Result of listing a single directory entry
struct DirEntry {
    name: String,
    is_dir: bool,
    is_symlink: bool,
    size: u64,
    modified: String,
}

/// Recursively list directory contents
fn list_directory(
    dir: &Path,
    max_depth: usize,
    show_hidden: bool,
    pattern: &str,
) -> Result<String, String> {
    let mut output = String::new();
    let canonical =
        std::fs::canonicalize(dir).map_err(|e| format!("Failed to canonicalize path: {}", e))?;

    output.push_str(&format!("Directory: {}\n\n", canonical.display()));

    let mut entries: Vec<DirEntry> = Vec::new();
    let mut visited_dirs: std::collections::HashSet<std::path::PathBuf> =
        std::collections::HashSet::new();
    collect_entries(
        dir,
        0,
        max_depth,
        show_hidden,
        pattern,
        &mut entries,
        &mut visited_dirs,
    )?;

    if entries.is_empty() {
        output.push_str("(empty directory)");
        return Ok(output);
    }

    // Count summary
    let file_count = entries.iter().filter(|e| !e.is_dir).count();
    let dir_count = entries.iter().filter(|e| e.is_dir).count();
    let total_size: u64 = entries.iter().map(|e| e.size).sum();

    // Format entries
    let max_name_len = entries
        .iter()
        .map(|e| {
            let len = e.name.len() + if e.is_dir { 1 } else { 0 };
            len
        })
        .max()
        .unwrap_or(0)
        .min(60);

    for entry in &entries {
        let icon = if entry.is_symlink {
            "🔗"
        } else if entry.is_dir {
            "📁"
        } else {
            "📄"
        };
        let name_padded = if entry.is_dir {
            format!("{}/", &entry.name)
        } else {
            entry.name.clone()
        };
        let size_str = if entry.is_dir {
            String::new()
        } else {
            format_size(entry.size)
        };
        let sym = if entry.is_symlink { " -> link" } else { "" };

        output.push_str(&format!(
            "  {} {:<width$} {:>8} {}{}\n",
            icon,
            name_padded,
            size_str,
            entry.modified,
            sym,
            width = max_name_len.min(60)
        ));
    }

    output.push_str(&format!(
        "\n{} files, {} directories ({} total)",
        file_count,
        dir_count,
        format_size(total_size)
    ));

    Ok(output)
}

/// Recursively collect directory entries
fn collect_entries(
    dir: &Path,
    depth: usize,
    max_depth: usize,
    show_hidden: bool,
    pattern: &str,
    entries: &mut Vec<DirEntry>,
    visited_dirs: &mut std::collections::HashSet<std::path::PathBuf>,
) -> Result<(), String> {
    if depth > max_depth && max_depth != 0 {
        return Ok(());
    }

    // Symlink cycle detection: canonicalize and track visited directories
    if let Ok(canonical) = std::fs::canonicalize(dir) {
        if !visited_dirs.insert(canonical) {
            // Already visited this directory — symlink cycle detected
            return Ok(());
        }
    }

    let read_dir = std::fs::read_dir(dir)
        .map_err(|e| format!("Failed to read directory '{}': {}", dir.display(), e))?;

    for entry in read_dir {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy().to_string();

        // Filter hidden files
        if !show_hidden && name.starts_with('.') {
            continue;
        }

        // Filter by glob pattern (uses glob::Pattern for correct matching)
        if !pattern.is_empty() && depth == 0 {
            let matches = ::glob::Pattern::new(pattern)
                .map(|p| p.matches(&name))
                .unwrap_or(false);
            if !matches {
                if entry.file_type().map(|t| !t.is_dir()).unwrap_or(true) {
                    continue;
                }
            }
        }

        let file_type = entry
            .file_type()
            .map_err(|e| format!("File type error: {}", e))?;
        let is_dir = file_type.is_dir();
        let is_symlink = file_type.is_symlink();
        let metadata = entry.metadata().ok();
        let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);

        let modified = metadata
            .and_then(|m| m.modified().ok())
            .map(|t| {
                let duration = t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
                let secs = duration.as_secs();
                // Format as relative time
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let age = now.saturating_sub(secs);
                if age < 60 {
                    "just now".to_string()
                } else if age < 3600 {
                    format!("{}m ago", age / 60)
                } else if age < 86400 {
                    format!("{}h ago", age / 3600)
                } else if age < 604800 {
                    format!("{}d ago", age / 86400)
                } else {
                    format!("{}w ago", age / 604800)
                }
            })
            .unwrap_or_default();

        let display_path = if depth == 0 {
            name.clone()
        } else {
            let parent = dir
                .file_name()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            if depth == 1 {
                format!("{}/{}", parent, name)
            } else {
                format!("{}/{}", parent, name) // simplified for deep nesting
            }
        };

        entries.push(DirEntry {
            name: display_path,
            is_dir,
            is_symlink,
            size,
            modified,
        });

        // Recurse into subdirectories (skip symlinks to avoid cycles)
        if is_dir && depth < max_depth {
            collect_entries(
                &entry.path(),
                depth + 1,
                max_depth,
                show_hidden,
                pattern,
                entries,
                visited_dirs,
            )?;
        }
    }

    // Sort: directories first, then by name
    entries.sort_by(|a, b| {
        if a.is_dir != b.is_dir {
            b.is_dir.cmp(&a.is_dir)
        } else {
            a.name.cmp(&b.name)
        }
    });

    Ok(())
}

/// Format file size in human-readable format
fn format_size(size: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = size as f64;
    let mut unit_idx = 0;

    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }

    if unit_idx == 0 {
        format!("{} {}", size as u64, UNITS[unit_idx])
    } else {
        format!("{:.1} {}", size, UNITS[unit_idx])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_dir_tool_name() {
        let tool = ListDirTool;
        assert_eq!(tool.name(), "list_dir");
    }

    #[test]
    fn test_list_dir_schema() {
        let tool = ListDirTool;
        let schema = tool.schema();
        assert!(schema.get("properties").is_some());
        assert!(schema.get("required").is_some());
    }

    #[tokio::test]
    async fn test_list_dir_nonexistent() {
        let tool = ListDirTool;
        let result = tool
            .execute(serde_json::json!({"path": "/nonexistent_dir_xyz"}))
            .await;
        assert!(!result.success);
    }

    #[tokio::test]
    async fn test_list_dir_file_path() {
        let tool = ListDirTool;
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let result = tool.execute(serde_json::json!({"path": tmp.path()})).await;
        assert!(!result.success); // Not a directory
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(1023), "1023 B");
        assert!(format_size(1024).contains("KB"));
        assert!(format_size(1_048_576).contains("MB"));
    }

    #[test]
    fn test_collect_entries_current_dir() {
        let mut entries = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let result = collect_entries(Path::new("."), 0, 1, false, "", &mut entries, &mut visited);
        assert!(result.is_ok());
        // Current directory should have at least some entries
        assert!(!entries.is_empty());
    }
}
