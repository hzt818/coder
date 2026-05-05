//! LSP tool — code intelligence via Language Server Protocol.
//!
//! Uses the LSP client infrastructure in `crate::lsp` to provide
//! go_to_definition, find_references, hover, document_symbols,
//! workspace_symbols, and call_hierarchy. Falls back to helpful
//! grep hints when the LSP feature is not enabled.

use async_trait::async_trait;
use super::*;

// When the `lsp` feature is enabled, the full LSP client is available.
// When disabled, we fall back to grep hints.
#[cfg(feature = "lsp")]
use std::sync::OnceLock;

/// Global LSP handler: initialized once, reused across tool invocations.
#[cfg(feature = "lsp")]
fn global_lsp_handler() -> Option<&'static crate::lsp::handler::LspHandler> {
    static HANDLER: OnceLock<crate::lsp::handler::LspHandler> = OnceLock::new();
    let handler = HANDLER.get_or_init(|| {
        let root_uri = std::env::current_dir()
            .ok()
            .map(|p| format!("file:///{}", p.display()));
        crate::lsp::handler::LspHandler::new(root_uri)
    });
    Some(handler)
}

/// Map a file extension to a language ID and LSP server command.
fn language_for_extension(path: &str) -> Option<(&'static str, &'static str, Vec<&'static str>)> {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    match ext {
        "rs" => Some(("rust", "rust-analyzer", vec![])),
        "ts" | "tsx" => Some(("typescript", "typescript-language-server", vec!["--stdio"])),
        "js" | "jsx" => Some(("javascript", "typescript-language-server", vec!["--stdio"])),
        "py" => Some(("python", "pyright-langserver", vec!["--stdio"])),
        "go" => Some(("go", "gopls", vec![])),
        "json" => Some(("json", "vscode-json-languageserver", vec!["--stdio"])),
        "css" | "scss" => Some(("css", "vscode-css-language-server", vec!["--stdio"])),
        "html" => Some(("html", "vscode-html-language-server", vec!["--stdio"])),
        _ => None,
    }
}

/// Format an LSP location (URI + range) into a human-readable string.
fn format_location(uri: &str, range: &serde_json::Value) -> String {
    let start = range.get("start").or_else(|| range.as_object().map(|_| range));
    if let Some(start) = start {
        let line = start.get("line").and_then(|l| l.as_u64()).unwrap_or(0);
        let col = start.get("character").and_then(|c| c.as_u64()).unwrap_or(0);
        let path = uri.strip_prefix("file://").unwrap_or(uri);
        format!("{}:{}:{}", path, line + 1, col + 1)
    } else {
        uri.to_string()
    }
}

pub struct LspTool;

#[async_trait]
impl Tool for LspTool {
    fn name(&self) -> &str { "lsp" }
    fn description(&self) -> &str {
        "Code intelligence via LSP: go_to_definition, find_references, hover, document_symbols, workspace_symbols, call_hierarchy."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object", "properties": {
                "operation": {
                    "type": "string", "enum": ["go_to_definition", "find_references", "hover", "document_symbols", "workspace_symbols", "call_hierarchy"],
                    "description": "LSP operation"
                },
                "file_path": { "type": "string", "description": "Path to the source file" },
                "line": { "type": "integer", "description": "Line number (1-based)" },
                "column": { "type": "integer", "description": "Column number (1-based)" },
                "query": { "type": "string", "description": "Search query (for workspace_symbols)" }
            }, "required": ["operation"]
        })
    }
    async fn execute(&self, args: serde_json::Value) -> ToolResult {
        let op = args.get("operation").and_then(|o| o.as_str()).unwrap_or("");
        if op.is_empty() { return ToolResult::err("operation is required"); }

        // Real LSP execution when the feature is enabled
        #[cfg(feature = "lsp")]
        { return self.execute_real(op, &args).await; }

        // Fallback
        #[cfg(not(feature = "lsp"))]
        {
            ToolResult::ok(match op {
                "go_to_definition" | "find_references" | "hover" => {
                    let file = args.get("file_path").and_then(|f| f.as_str()).unwrap_or("");
                    let line = args.get("line").and_then(|l| l.as_i64()).unwrap_or(0);
                    let col = args.get("column").and_then(|c| c.as_i64()).unwrap_or(0);
                    format!("LSP '{}' for {}:{}:{} requires the 'lsp' feature.\nBuild with: cargo build --features lsp\nFor now, try: grep -rn 'symbol_name' src/", op, file, line, col)
                }
                _ => format!("LSP '{}' requires the 'lsp' feature.\nBuild with: cargo build --features lsp", op),
            })
        }
    }
    fn requires_permission(&self) -> bool { false }
}

#[cfg(feature = "lsp")]
impl LspTool {
    async fn execute_real(&self, op: &str, args: &serde_json::Value) -> ToolResult {
        let handler = match global_lsp_handler() {
            Some(h) => h,
            None => return ToolResult::err("Failed to initialize LSP handler"),
        };

        match op {
            "go_to_definition" | "find_references" | "hover" => {
                let file = args.get("file_path").and_then(|f| f.as_str()).unwrap_or("");
                let line = args.get("line").and_then(|l| l.as_i64()).unwrap_or(0) as u64;
                let column = args.get("column").and_then(|c| c.as_i64()).unwrap_or(0) as u64;
                if file.is_empty() { return ToolResult::err("file_path is required"); }
                let (lang, _, _) = match language_for_extension(file) {
                    Some(l) => l, None => return ToolResult::err(format!("Unsupported file type: {}", file)),
                };
                let uri = format!("file:///{}", file.replace('\\', "/").trim_start_matches("file:///"));

                let configs = crate::lsp::handler::default_lsp_configs();
                let (cmd, args_list) = match configs.get(lang) {
                    Some(c) => *c,
                    None => return ToolResult::err(format!("No LSP server configured for '{}'", lang)),
                };
                if let Err(e) = handler.start_server(lang, cmd, args_list.iter().map(|s| s.to_string()).collect()).await {
                    return ToolResult::err(format!("Failed to start LSP server for '{}': {}", lang, e));
                }

                let result = match op {
                    "go_to_definition" => handler.goto_definition(lang, &uri, line, column).await,
                    "find_references" => match handler.get_client(lang).await {
                        Some(client) => client.send_request("textDocument/references", serde_json::json!({
                            "textDocument": { "uri": uri },
                            "position": { "line": line, "character": column }
                        })).await,
                        None => Err(anyhow::anyhow!("No LSP client for '{}'", lang)),
                    },
                    "hover" => handler.hover(lang, &uri, line, column).await,
                    _ => unreachable!(),
                };

                match result {
                    Ok(value) => {
                        let output = match op {
                            "go_to_definition" => {
                                let locations = if value.is_array() { value.as_array().cloned().unwrap_or_default() } else { vec![value.clone()] };
                                if locations.is_empty() { "No definition found.".to_string() }
                                else {
                                    let mut out = format!("{} location(s):\n", locations.len());
                                    for (i, loc) in locations.iter().enumerate() {
                                        let u = loc.get("uri").and_then(|u| u.as_str()).unwrap_or("");
                                        let r = loc.get("range").unwrap_or(loc);
                                        out.push_str(&format!("  {}. {}\n", i + 1, format_location(u, r)));
                                    }
                                    out
                                }
                            }
                            "find_references" => {
                                let refs = value.as_array().map(|a| a.clone()).unwrap_or_default();
                                if refs.is_empty() { "No references found.".to_string() }
                                else {
                                    let mut out = format!("{} reference(s):\n", refs.len());
                                    for (i, r) in refs.iter().enumerate() {
                                        let u = r.get("uri").and_then(|u| u.as_str()).unwrap_or("");
                                        let range = r.get("range").unwrap_or(r);
                                        out.push_str(&format!("  {}. {}\n", i + 1, format_location(u, range)));
                                    }
                                    out
                                }
                            }
                            "hover" => {
                                let contents = value.get("contents").or_else(|| value.as_object().map(|_| value));
                                match contents {
                                    Some(c) => {
                                        let text = c.get("value").and_then(|v| v.as_str()).or_else(|| c.as_str()).unwrap_or("(no hover content)");
                                        format!("── Hover ──\n{}", text)
                                    }
                                    None => "(no hover information)".to_string(),
                                }
                            }
                            _ => format!("{:?}", value),
                        };
                        ToolResult::ok(output)
                    }
                    Err(e) => ToolResult::ok(format!("LSP {} failed: {}\n\nTry: grep -rn", op, e))
                }
            }
            "document_symbols" => {
                let file = args.get("file_path").and_then(|f| f.as_str()).unwrap_or("");
                if file.is_empty() { return ToolResult::err("file_path is required"); }
                let uri = format!("file:///{}", file.replace('\\', "/").trim_start_matches("file:///"));
                let (lang, _, _) = match language_for_extension(file) {
                    Some(l) => l, None => return ToolResult::err(format!("Unsupported file type: {}", file)),
                };
                match handler.get_client(lang).await {
                    Some(client) => {
                        match client.send_request("textDocument/documentSymbol", serde_json::json!({"textDocument": { "uri": uri }})).await {
                            Ok(value) => {
                                let symbols = value.as_array().map(|a| a.clone()).unwrap_or_default();
                                if symbols.is_empty() { ToolResult::ok("No symbols found.") }
                                else {
                                    let mut out = format!("── Symbols ({}) ──\n", symbols.len());
                                    for sym in &symbols {
                                        let name = sym.get("name").and_then(|n| n.as_str()).unwrap_or("?");
                                        out.push_str(&format!("  • {}\n", name));
                                    }
                                    ToolResult::ok(out)
                                }
                            }
                            Err(e) => ToolResult::ok(format!("LSP documentSymbols failed: {}\nTry: grep -rn 'fn \\|pub ' {}", e, file)),
                        }
                    }
                    None => ToolResult::ok(format!("No LSP server for '{}'. Try: grep -rn 'fn \\|pub ' {}", lang, file)),
                }
            }
            "workspace_symbols" => {
                let query = args.get("query").and_then(|q| q.as_str()).unwrap_or("");
                if query.is_empty() { return ToolResult::err("query is required"); }
                let langs = handler.connected_languages().await;
                if langs.is_empty() {
                    return ToolResult::ok(format!("No running LSP servers. Try: grep -rn '{}' src/", query));
                }
                let mut out = format!("── Workspace Symbols: '{}' ──\n", query);
                for lang in &langs {
                    if let Some(client) = handler.get_client(lang).await {
                        if let Ok(result) = client.send_request("workspace/symbol", serde_json::json!({"query": query})).await {
                            let symbols = result.as_array().map(|a| a.clone()).unwrap_or_default();
                            for sym in &symbols {
                                let name = sym.get("name").and_then(|n| n.as_str()).unwrap_or("?");
                                let container = sym.get("containerName").and_then(|c| c.as_str()).unwrap_or("");
                                if !container.is_empty() { out.push_str(&format!("  {}.{}", container, name)); }
                                else { out.push_str(&format!("  {}", name)); }
                                out.push('\n');
                            }
                        }
                    }
                }
                if out.lines().count() <= 2 { out.push_str("  (no results)\n"); }
                ToolResult::ok(out)
            }
            "call_hierarchy" => {
                let file = args.get("file_path").and_then(|f| f.as_str()).unwrap_or("");
                let line = args.get("line").and_then(|l| l.as_i64()).unwrap_or(0) as u64;
                let col = args.get("column").and_then(|c| c.as_i64()).unwrap_or(0) as u64;
                if file.is_empty() { return ToolResult::err("file_path is required"); }
                let uri = format!("file:///{}", file.replace('\\', "/").trim_start_matches("file:///"));
                let (lang, _, _) = match language_for_extension(file) {
                    Some(l) => l, None => return ToolResult::err(format!("Unsupported file type: {}", file)),
                };
                match handler.get_client(lang).await {
                    Some(client) => {
                        match client.send_request("textDocument/prepareCallHierarchy", serde_json::json!({
                            "textDocument": { "uri": uri },
                            "position": { "line": line, "character": col }
                        })).await {
                            Ok(items) => {
                                let items = items.as_array().map(|a| a.clone()).unwrap_or_else(|| vec![items.clone()]);
                                let mut out = format!("── Call Hierarchy at {}:{}:{} ──\n", file, line + 1, col + 1);
                                for item in &items {
                                    let name = item.get("name").and_then(|n| n.as_str()).unwrap_or("?");
                                    out.push_str(&format!("  {}\n", name));
                                }
                                if items.is_empty() { out.push_str("  (no call hierarchy)\n"); }
                                ToolResult::ok(out)
                            }
                            Err(e) => ToolResult::ok(format!("LSP call_hierarchy failed: {}\nTry: grep -rn", e)),
                        }
                    }
                    None => ToolResult::ok(format!("No LSP server for '{}'. Try: grep -rn", lang)),
                }
            }
            _ => ToolResult::err(format!("Unknown LSP operation: '{}'", op)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_name() { assert_eq!(LspTool.name(), "lsp"); }
    #[tokio::test] async fn test_empty_op() { assert!(!LspTool.execute(serde_json::json!({})).await.success); }
    #[tokio::test] async fn test_unknown_op() {
        assert!(!LspTool.execute(serde_json::json!({"operation": "bogus"})).await.success);
    }
    #[test] fn test_language_mapping() {
        assert!(language_for_extension("main.rs").is_some());
        assert!(language_for_extension("main.py").is_some());
        assert!(language_for_extension("unknown.xyz").is_none());
    }
}
