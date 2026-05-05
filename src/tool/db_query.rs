//! Database query tool - SQL query execution via libsql
//!
//! Supports SQL queries against local SQLite databases.

use async_trait::async_trait;
use super::*;

pub struct DbQueryTool;

#[async_trait]
impl Tool for DbQueryTool {
    fn name(&self) -> &str {
        "db_query"
    }

    fn description(&self) -> &str {
        "Execute SQL queries against SQLite databases."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "db_path": {
                    "type": "string",
                    "description": "Path to SQLite database file",
                    "default": "./data.db"
                },
                "query": {
                    "type": "string",
                    "description": "SQL query to execute"
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> ToolResult {
        let query = args.get("query")
            .and_then(|q| q.as_str())
            .unwrap_or("");

        if query.is_empty() {
            return ToolResult::err("SQL query is required");
        }

        let db_path = args.get("db_path")
            .and_then(|c| c.as_str())
            .unwrap_or("./data.db");

        match execute_query(db_path, query).await {
            Ok(result) => ToolResult::ok(result),
            Err(e) => ToolResult::err(format!("Query failed: {}", e)),
        }
    }

    fn requires_permission(&self) -> bool {
        true
    }
}

async fn execute_query(db_path: &str, query: &str) -> Result<String, String> {
    // Open the database
    let db = libsql::Builder::new_local(db_path)
        .build()
        .await
        .map_err(|e| format!("Failed to open database '{}': {}", db_path, e))?;

    let conn = db.connect()
        .map_err(|e| format!("Failed to connect: {}", e))?;

    let trimmed = query.trim().to_uppercase();
    let is_query = trimmed.starts_with("SELECT")
        || trimmed.starts_with("PRAGMA")
        || trimmed.starts_with("EXPLAIN");

    if is_query {
        let mut rows = conn.query(query, libsql::params![])
            .await
            .map_err(|e| format!("Query failed: {}", e))?;

        let mut result = format!("Query: {}\n\n", query);
        let mut row_count = 0u64;

        while let Some(row) = rows.next().await
            .map_err(|e| format!("Row read error: {}", e))?
        {
            let mut values = Vec::new();
            for i in 0..100 {
                match row.get::<String>(i) {
                    Ok(v) => values.push(v),
                    Err(_) => break,
                }
            }
            result.push_str(&format!("  {}\n", values.join(" | ")));
            row_count += 1;
        }

        result.push_str(&format!("\n({} row(s) returned)", row_count));
        Ok(result)
    } else {
        conn.execute(query, libsql::params![])
            .await
            .map_err(|e| format!("Query failed: {}", e))?;

        Ok(format!("Query executed successfully: {}", query))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db_query_tool_name() {
        let tool = DbQueryTool;
        assert_eq!(tool.name(), "db_query");
    }

    #[tokio::test]
    async fn test_db_query_empty() {
        let tool = DbQueryTool;
        let result = tool.execute(serde_json::json!({"query": ""})).await;
        assert!(!result.success);
    }
}
