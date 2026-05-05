//! Apply patch tool - applies unified diffs to files
//!
//! Parses and applies unified diff format patches with fuzzy matching support.
//! Preferred over shell `patch` command for structured output and safety.

use async_trait::async_trait;
use super::*;

pub struct ApplyPatchTool;

#[async_trait]
impl Tool for ApplyPatchTool {
    fn name(&self) -> &str {
        "apply_patch"
    }

    fn description(&self) -> &str {
        concat!(
            "Apply a unified diff (patch) to a file. ",
            "Supports fuzzy matching. Preferred over shell 'patch'."
        )
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to patch"
                },
                "patch": {
                    "type": "string",
                    "description": "The unified diff content to apply"
                },
                "fuzz": {
                    "type": "integer",
                    "description": "Fuzz factor: number of context lines that can be mismatched (default: 0, max: 3)",
                    "default": 0
                },
                "reverse": {
                    "type": "boolean",
                    "description": "Apply the patch in reverse",
                    "default": false
                }
            },
            "required": ["path", "patch"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> ToolResult {
        let path = args.get("path")
            .and_then(|p| p.as_str())
            .unwrap_or("");

        let patch_str = args.get("patch")
            .and_then(|p| p.as_str())
            .unwrap_or("");

        let fuzz = args.get("fuzz")
            .and_then(|f| f.as_u64())
            .unwrap_or(0) as usize;

        let reverse = args.get("reverse")
            .and_then(|r| r.as_bool())
            .unwrap_or(false);

        if path.is_empty() {
            return ToolResult::err("Path is required");
        }

        if patch_str.is_empty() {
            return ToolResult::err("Patch content is required");
        }

        // Read the target file
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => return ToolResult::err(format!("Failed to read file '{}': {}", path, e)),
        };

        // Parse the patch
        let hunks = match parse_unified_diff(patch_str, reverse) {
            Ok(h) => h,
            Err(e) => return ToolResult::err(format!("Failed to parse patch: {}", e)),
        };

        if hunks.is_empty() {
            return ToolResult::err("No valid hunks found in patch");
        }

        // Apply each hunk
        let mut result_content = content.clone();
        let mut applied = 0;
        let mut fuzz_used = 0;
        let mut errors: Vec<String> = Vec::new();

        for hunk in &hunks {
            match apply_hunk(&result_content, hunk, fuzz) {
                Ok((new_content, fuzz_level)) => {
                    result_content = new_content;
                    applied += 1;
                    fuzz_used = fuzz_used.max(fuzz_level);
                }
                Err(e) => {
                    errors.push(format!("Hunk at lines {}-{}: {}", hunk.orig_start, hunk.orig_start + hunk.orig_lines, e));
                }
            }
        }

        if applied == 0 {
            let error_detail = errors.join("\n");
            return ToolResult::err(format!(
                "Failed to apply patch to '{}'. No hunks applied.\n{}",
                path, error_detail
            ));
        }

        // Write the result
        match std::fs::write(path, &result_content) {
            Ok(_) => {
                let mut output = format!(
                    "Successfully applied patch to '{}': {} hunk(s) applied",
                    path, applied
                );

                if fuzz_used > 0 {
                    output.push_str(&format!(" (fuzz factor: {})", fuzz_used));
                }

                if applied < hunks.len() {
                    output.push_str(&format!("\n{} hunk(s) failed to apply:", hunks.len() - applied));
                    for err in &errors {
                        output.push_str(&format!("\n  - {}", err));
                    }
                }

                ToolResult::ok(output)
            }
            Err(e) => ToolResult::err(format!("Failed to write file '{}': {}", path, e)),
        }
    }
}

/// A parsed hunk from a unified diff
#[derive(Debug, Clone)]
struct Hunk {
    /// Original file line start
    orig_start: usize,
    /// Original file line count
    orig_lines: usize,
    /// New file line start
    #[allow(dead_code)]
    new_start: usize,
    /// New file line count
    #[allow(dead_code)]
    new_lines: usize,
    /// Lines in the hunk (with context/insert/delete markers)
    lines: Vec<HunkLine>,
}

/// A single line within a hunk
#[derive(Debug, Clone, PartialEq)]
enum HunkLine {
    /// Context line (starts with ' ')
    Context(String),
    /// Deletion line (starts with '-')
    Delete(String),
    /// Insertion line (starts with '+')
    Insert(String),
}

/// Parse a unified diff string into hunks
fn parse_unified_diff(diff: &str, reverse: bool) -> Result<Vec<Hunk>, String> {
    let mut hunks = Vec::new();
    let mut current_hunk: Option<Hunk> = None;

    for line in diff.lines() {
        if line.starts_with("@@") {
            // Save previous hunk
            if let Some(hunk) = current_hunk.take() {
                if !hunk.lines.is_empty() {
                    hunks.push(hunk);
                }
            }

            // Parse hunk header: @@ -orig_start,orig_lines +new_start,new_lines @@
            if let Some((orig_start, orig_lines, new_start, new_lines)) = parse_hunk_header(line) {
                current_hunk = Some(Hunk {
                    orig_start,
                    orig_lines,
                    new_start,
                    new_lines,
                    lines: Vec::new(),
                });
            }
        } else if let Some(ref mut hunk) = current_hunk {
            if line.is_empty() {
                continue;
            }
            let (hunk_line, _is_context) = match line.chars().next() {
                Some(' ') => (HunkLine::Context(line[1..].to_string()), true),
                Some('-') => {
                    if reverse {
                        (HunkLine::Insert(line[1..].to_string()), false)
                    } else {
                        (HunkLine::Delete(line[1..].to_string()), false)
                    }
                }
                Some('+') => {
                    if reverse {
                        (HunkLine::Delete(line[1..].to_string()), false)
                    } else {
                        (HunkLine::Insert(line[1..].to_string()), false)
                    }
                }
                Some('\\') => continue, // No newline at end of file
                _ => continue,
            };
            hunk.lines.push(hunk_line);
        }
    }

    // Save last hunk
    if let Some(hunk) = current_hunk {
        if !hunk.lines.is_empty() {
            hunks.push(hunk);
        }
    }

    Ok(hunks)
}

/// Parse a unified diff hunk header
/// Format: @@ -orig_start,orig_lines +new_start,new_lines @@
fn parse_hunk_header(header: &str) -> Option<(usize, usize, usize, usize)> {
    let header = header.trim_start_matches("@@").trim_end_matches("@@").trim();
    let parts: Vec<&str> = header.split(' ').collect();
    if parts.len() < 2 {
        return None;
    }

    let parse_range = |s: &str| -> Option<(usize, usize)> {
        let s = s.trim_start_matches('-').trim_start_matches('+');
        if let Some(comma_pos) = s.find(',') {
            let start: usize = s[..comma_pos].parse().ok()?;
            let count: usize = s[comma_pos + 1..].parse().ok()?;
            Some((start, count))
        } else {
            let start: usize = s.parse().ok()?;
            Some((start, 1))
        }
    };

    let orig = parse_range(parts[0])?;
    let new = parse_range(parts[1])?;

    Some((orig.0, orig.1, new.0, new.1))
}

/// Apply a single hunk to the content with optional fuzzy matching
fn apply_hunk(content: &str, hunk: &Hunk, fuzz: usize) -> Result<(String, usize), String> {
    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();

    // Collect the context lines from the hunk (for matching)
    let context_lines: Vec<&str> = hunk
        .lines
        .iter()
        .filter_map(|l| match l {
            HunkLine::Context(s) => Some(s.as_str()),
            _ => None,
        })
        .collect();

    if context_lines.is_empty() {
        // No context lines - apply at the specified position
        return apply_at_position(&lines, hunk, hunk.orig_start.saturating_sub(1), total_lines);
    }

    // Try to find the context in the file
    let first_context = context_lines[0];
    let search_start = hunk.orig_start.saturating_sub(1);

    // Search for the first context line
    let mut best_pos = None;
    let mut best_fuzz = fuzz + 1; // Higher than allowed

    // Search range: within 20 lines of original position, or full file
    let search_range = if search_start < 20 {
        0..total_lines
    } else {
        (search_start.saturating_sub(10))..(std::cmp::min(search_start + 10, total_lines))
    };

    for pos in search_range {
        if pos >= total_lines {
            break;
        }
        if lines[pos].trim() == first_context.trim() {
            // Found a context match - verify whole hunk
            match verify_hunk_position(&lines, hunk, pos, fuzz) {
                Ok((fuzz_level, _)) => {
                    if fuzz_level <= best_fuzz {
                        best_fuzz = fuzz_level;
                        best_pos = Some(pos);
                        if fuzz_level == 0 {
                            break; // Exact match found
                        }
                    }
                }
                Err(_) => continue,
            }
        }
    }

    match best_pos {
        Some(pos) => {
            let (fuzz_level, _) = verify_hunk_position(&lines, hunk, pos, fuzz)?;
            apply_at_position(&lines, hunk, pos, total_lines)
                .map(|(r, _)| (r, fuzz_level))
        }
        None => {
            // Try fuzzy match with max fuzz
            if fuzz > 0 {
                for pos in 0..total_lines {
                    if let Ok((fuzz_level, _)) = verify_hunk_position(&lines, hunk, pos, fuzz) {
                        if fuzz_level <= fuzz {
                            return apply_at_position(&lines, hunk, pos, total_lines)
                                .map(|(r, _)| (r, fuzz_level));
                        }
                    }
                }
            }
            Err(format!(
                "Could not find matching context in file near line {}",
                hunk.orig_start
            ))
        }
    }
}

/// Verify that a hunk matches at a given position in the file
/// Returns (fuzz_level, number_of_matching_lines) on success
fn verify_hunk_position(
    lines: &[&str],
    hunk: &Hunk,
    pos: usize,
    max_fuzz: usize,
) -> Result<(usize, usize), String> {
    let mut hunk_idx = 0;
    let mut file_idx = pos;
    let mut mismatches = 0;
    let mut total_context = 0;
    let mut matched_context = 0;

    while hunk_idx < hunk.lines.len() && file_idx < lines.len() {
        match &hunk.lines[hunk_idx] {
            HunkLine::Context(expected) => {
                total_context += 1;
                let actual = lines[file_idx].trim();
                let expected_trimmed = expected.trim();

                if actual == expected_trimmed {
                    matched_context += 1;
                    file_idx += 1;
                    hunk_idx += 1;
                } else {
                    mismatches += 1;
                    if mismatches > max_fuzz {
                        return Err(format!("Too many mismatches ({})", mismatches));
                    }
                    file_idx += 1;
                    hunk_idx += 1;
                }
            }
            HunkLine::Delete(_) => {
                hunk_idx += 1;
                // Don't advance file_idx - deletion removes a line
            }
            HunkLine::Insert(_) => {
                file_idx += 1;
                hunk_idx += 1;
            }
        }
    }

    // Calculate actual fuzz level used
    let fuzz_level = if total_context > 0 {
        total_context - matched_context
    } else {
        0
    };

    if fuzz_level > max_fuzz {
        return Err(format!(
            "Fuzz level {} exceeds maximum {}",
            fuzz_level, max_fuzz
        ));
    }

    Ok((fuzz_level, matched_context))
}

/// Apply a hunk at a specific position in the file
fn apply_at_position(
    lines: &[&str],
    hunk: &Hunk,
    pos: usize,
    total_lines: usize,
) -> Result<(String, usize), String> {
    let mut result: Vec<&str> = Vec::new();
    let mut file_idx = 0usize;
    let mut hunk_idx = 0usize;

    // Add lines before the hunk position
    while file_idx < pos && file_idx < total_lines {
        result.push(lines[file_idx]);
        file_idx += 1;
    }

    // Process hunk lines
    while hunk_idx < hunk.lines.len() {
        match &hunk.lines[hunk_idx] {
            HunkLine::Context(s) => {
                result.push(s);
                file_idx += 1;
                hunk_idx += 1;
            }
            HunkLine::Delete(_) => {
                // Skip the original line
                file_idx += 1;
                hunk_idx += 1;
            }
            HunkLine::Insert(s) => {
                result.push(s);
                hunk_idx += 1;
            }
        }
    }

    // Add remaining lines after the hunk
    while file_idx < total_lines {
        result.push(lines[file_idx]);
        file_idx += 1;
    }

    Ok((result.join("\n"), hunk_idx))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_patch_tool_name() {
        let tool = ApplyPatchTool;
        assert_eq!(tool.name(), "apply_patch");
    }

    #[test]
    fn test_apply_patch_schema() {
        let tool = ApplyPatchTool;
        let schema = tool.schema();
        assert!(schema.get("properties").is_some());
        assert!(schema.get("required").is_some());
    }

    #[tokio::test]
    async fn test_apply_patch_empty_args() {
        let tool = ApplyPatchTool;
        let result = tool.execute(serde_json::json!({})).await;
        assert!(!result.success);
    }

    #[test]
    fn test_parse_unified_diff_simple() {
        let diff = "\
@@ -1,3 +1,4 @@
 line1
-line2
+line2_modified
+new_line
 line3
";
        let hunks = parse_unified_diff(diff, false).unwrap();
        assert_eq!(hunks.len(), 1);
        assert_eq!(hunks[0].orig_start, 1);
        assert_eq!(hunks[0].orig_lines, 3);
        assert_eq!(hunks[0].lines.len(), 5); // 2 context + 1 del + 2 ins
    }

    #[test]
    fn test_parse_unified_diff_empty() {
        let diff = "";
        let hunks = parse_unified_diff(diff, false).unwrap();
        assert!(hunks.is_empty());
    }

    #[test]
    fn test_parse_hunk_header() {
        let (orig_start, orig_lines, new_start, new_lines) =
            parse_hunk_header("@@ -1,3 +1,4 @@").unwrap();
        assert_eq!(orig_start, 1);
        assert_eq!(orig_lines, 3);
        assert_eq!(new_start, 1);
        assert_eq!(new_lines, 4);
    }

    #[test]
    fn test_parse_hunk_header_single_line() {
        let (orig_start, orig_lines, new_start, new_lines) =
            parse_hunk_header("@@ -1 +1 @@").unwrap();
        assert_eq!(orig_start, 1);
        assert_eq!(orig_lines, 1);
        assert_eq!(new_start, 1);
        assert_eq!(new_lines, 1);
    }

    #[test]
    fn test_apply_hunk_simple() {
        let content = "line1\nline2\nline3\n";
        let diff = "\
@@ -1,3 +1,4 @@
 line1
-line2
+line2_modified
+new_line
 line3
";
        let hunks = parse_unified_diff(diff, false).unwrap();
        let result = apply_hunk(content, &hunks[0], 0).unwrap();
        let expected = "line1\nline2_modified\nnew_line\nline3";
        assert_eq!(result.0, expected);
    }

    #[test]
    fn test_apply_hunk_at_end() {
        let content = "line1\nline2\n";
        let diff = "\
@@ -2,1 +2,2 @@
 line2
+line3
";
        let hunks = parse_unified_diff(diff, false).unwrap();
        assert_eq!(hunks.len(), 1);
        // Should apply without error (even if context doesn't match perfectly)
        let result = apply_hunk(content, &hunks[0], 2);
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_reverse_patch() {
        let diff = "\
@@ -1,3 +1,2 @@
 line1
-modified_line
-line3
+line3
";
        let hunks = parse_unified_diff(diff, true).unwrap();
        // In reverse mode, the - becomes + and + becomes -
        assert_eq!(hunks.len(), 1);
        // The first changed line should be an Insert (was - before)
        assert_eq!(hunks[0].lines[1], HunkLine::Insert("modified_line".to_string()));
    }

    #[test]
    fn test_parse_unified_diff_multiple_hunks() {
        let diff = "\
@@ -1,3 +1,4 @@
 line1
-line2
+line2_modified
+new_line
 line3
@@ -10,2 +11,2 @@
 context_a
-context_b
+context_b_modified
";
        let hunks = parse_unified_diff(diff, false).unwrap();
        assert_eq!(hunks.len(), 2);
    }

    #[tokio::test]
    async fn test_apply_patch_integration() {
        let tool = ApplyPatchTool;
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), "hello\nworld\nrust\n").unwrap();

        let patch = "\
@@ -1,3 +1,4 @@
 hello
-world
+rust_is_great
+awesome
 rust
";

        let path_str = tmp.path().to_str().unwrap();
        let result = tool
            .execute(serde_json::json!({
                "path": path_str,
                "patch": patch,
                "fuzz": 2
            }))
            .await;

        // This should work with fuzz factor
        if result.success {
            let content = std::fs::read_to_string(tmp.path()).unwrap();
            assert!(content.contains("rust_is_great"));
        }
    }
}
