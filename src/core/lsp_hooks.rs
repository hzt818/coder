//! LSP post-edit diagnostic hooks
//!
//! After every file edit (edit_file, apply_patch, write_file), this module
//! triggers LSP diagnostics collection and injects results into the
//! agent context for the next reasoning turn.

use crate::lsp::client::LspClient;
use std::path::Path;
use std::sync::Arc;

/// A single diagnostic result from LSP
#[derive(Debug, Clone)]
pub struct DiagnosticResult {
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub source: String,
}

/// Severity of a diagnostic
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagnosticSeverity {
    Hint,
    Information,
    Warning,
    Error,
}

impl DiagnosticSeverity {
    pub fn label(&self) -> &'static str {
        match self {
            DiagnosticSeverity::Error => "ERROR",
            DiagnosticSeverity::Warning => "WARN",
            DiagnosticSeverity::Information => "INFO",
            DiagnosticSeverity::Hint => "HINT",
        }
    }

    pub fn emoji(&self) -> &'static str {
        match self {
            DiagnosticSeverity::Error => "🔴",
            DiagnosticSeverity::Warning => "🟡",
            DiagnosticSeverity::Information => "🔵",
            DiagnosticSeverity::Hint => "🟢",
        }
    }
}

/// Run LSP diagnostics for edited files after a tool execution
pub async fn run_post_edit_lsp<'a>(
    edited_files: &[&'a Path],
    lsp_clients: &[Arc<LspClient>],
) -> Vec<DiagnosticResult> {
    let mut all_diagnostics = Vec::new();

    for file in edited_files {
        let extension = file.extension().and_then(|e| e.to_str()).unwrap_or("");

        // Find the matching LSP client for this file type
        for client in lsp_clients {
            if client.supports_extension(extension) {
                let diagnostics = match client.request_diagnostics(file).await {
                    Ok(diags) => diags,
                    Err(e) => {
                        tracing::debug!("LSP diagnostic error for {}: {}", file.display(), e);
                        continue;
                    }
                };

                for diag in diagnostics {
                    let severity = match diag.severity {
                        1 => DiagnosticSeverity::Error,
                        2 => DiagnosticSeverity::Warning,
                        3..=4 => DiagnosticSeverity::Information,
                        _ => DiagnosticSeverity::Hint,
                    };

                    let line = diag.line();
                    let column = diag.column();
                    all_diagnostics.push(DiagnosticResult {
                        severity,
                        message: diag.message,
                        file: file.to_string_lossy().to_string(),
                        line,
                        column,
                        source: client.server_name().to_string(),
                    });
                }
            }
        }
    }

    all_diagnostics
}

/// Format diagnostics for inclusion in agent context
pub fn format_diagnostics_for_context(diagnostics: &[DiagnosticResult]) -> Option<String> {
    if diagnostics.is_empty() {
        return None;
    }

    // Group by severity
    let errors: Vec<&DiagnosticResult> = diagnostics
        .iter()
        .filter(|d| d.severity == DiagnosticSeverity::Error)
        .collect();
    let warnings: Vec<&DiagnosticResult> = diagnostics
        .iter()
        .filter(|d| d.severity == DiagnosticSeverity::Warning)
        .collect();
    let infos: Vec<&DiagnosticResult> = diagnostics
        .iter()
        .filter(|d| d.severity == DiagnosticSeverity::Information)
        .collect();

    let mut result = String::new();
    result.push_str("── LSP Diagnostics ──\n");

    if !errors.is_empty() {
        result.push_str(&format!("\n🔴 Errors ({}):\n", errors.len()));
        for diag in &errors {
            result.push_str(&format!(
                "  {}:{}:{} - {}\n",
                diag.file, diag.line, diag.column, diag.message
            ));
        }
    }

    if !warnings.is_empty() {
        result.push_str(&format!("\n🟡 Warnings ({}):\n", warnings.len()));
        for diag in &warnings {
            result.push_str(&format!(
                "  {}:{}:{} - {}\n",
                diag.file, diag.line, diag.column, diag.message
            ));
        }
    }

    if !infos.is_empty() {
        result.push_str(&format!("\n🔵 Info ({}):\n", infos.len()));
        for diag in &infos {
            result.push_str(&format!(
                "  {}:{}:{} - {}\n",
                diag.file, diag.line, diag.column, diag.message
            ));
        }
    }

    result.push_str("\n── End Diagnostics ──");

    Some(result)
}

/// Quick check if diagnostics are available by verifying file extension support
pub fn has_diagnostics_available(edited_files: &[&Path], lsp_clients: &[Arc<LspClient>]) -> bool {
    if lsp_clients.is_empty() || edited_files.is_empty() {
        return false;
    }

    for file in edited_files {
        let extension = file.extension().and_then(|e| e.to_str()).unwrap_or("");
        for client in lsp_clients {
            if client.supports_extension(extension) {
                return true;
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostic_severity_labels() {
        assert_eq!(DiagnosticSeverity::Error.label(), "ERROR");
        assert_eq!(DiagnosticSeverity::Warning.label(), "WARN");
        assert_eq!(DiagnosticSeverity::Information.label(), "INFO");
    }

    #[test]
    fn test_diagnostic_severity_ordering() {
        assert!(DiagnosticSeverity::Error > DiagnosticSeverity::Warning);
        assert!(DiagnosticSeverity::Warning > DiagnosticSeverity::Information);
        assert!(DiagnosticSeverity::Information > DiagnosticSeverity::Hint);
    }

    #[test]
    fn test_format_diagnostics_none() {
        let result = format_diagnostics_for_context(&[]);
        assert!(result.is_none());
    }

    #[test]
    fn test_format_diagnostics_with_errors() {
        let diagnostics = vec![
            DiagnosticResult {
                severity: DiagnosticSeverity::Error,
                message: "undefined variable".to_string(),
                file: "src/main.rs".to_string(),
                line: 10,
                column: 5,
                source: "rust-analyzer".to_string(),
            },
            DiagnosticResult {
                severity: DiagnosticSeverity::Warning,
                message: "unused variable".to_string(),
                file: "src/main.rs".to_string(),
                line: 15,
                column: 9,
                source: "rust-analyzer".to_string(),
            },
        ];

        let result = format_diagnostics_for_context(&diagnostics);
        assert!(result.is_some());
        let text = result.unwrap();
        assert!(text.contains("LSP Diagnostics"));
        assert!(text.contains("undefined variable"));
        assert!(text.contains("unused variable"));
        assert!(text.contains("🔴"));
        assert!(text.contains("🟡"));
    }

    #[test]
    fn test_format_diagnostics_infos_only() {
        let diagnostics = vec![DiagnosticResult {
            severity: DiagnosticSeverity::Information,
            message: "consider using 'map' here".to_string(),
            file: "src/lib.rs".to_string(),
            line: 42,
            column: 10,
            source: "clippy".to_string(),
        }];

        let result = format_diagnostics_for_context(&diagnostics);
        assert!(result.is_some());
        let text = result.unwrap();
        assert!(text.contains("🔵"));
        assert!(!text.contains("🔴"));
    }

    #[test]
    fn test_diagnostic_result_display() {
        let d = DiagnosticResult {
            severity: DiagnosticSeverity::Error,
            message: "test error".to_string(),
            file: "test.rs".to_string(),
            line: 1,
            column: 1,
            source: "test".to_string(),
        };
        assert_eq!(d.severity.label(), "ERROR");
        assert_eq!(d.file, "test.rs");
    }
}
