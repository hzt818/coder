//! Schema migration management for SQLite database

/// Run all pending migrations in order.
///
/// Uses a `_migrations` table to track applied migrations.
/// Each migration is a simple SQL string applied sequentially.
pub async fn run_migrations(conn: &libsql::Connection) -> anyhow::Result<()> {
    create_migrations_table(conn).await?;

    let applied = get_applied_migrations(conn).await?;

    for (id, name, sql) in MIGRATIONS.iter() {
        let id_str = id.to_string();
        if !applied.contains(&id_str) {
            tracing::info!("Applying migration {}: {}", id, name);
            conn.execute(sql, libsql::params![]).await?;
            record_migration(conn, *id, name).await?;
        }
    }

    Ok(())
}

/// Create the migrations tracking table if it does not exist.
async fn create_migrations_table(conn: &libsql::Connection) -> anyhow::Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS _migrations (
            id          TEXT PRIMARY KEY,
            name        TEXT NOT NULL,
            applied_at  TEXT NOT NULL DEFAULT (datetime('now'))
        )",
        libsql::params![],
    )
    .await?;
    Ok(())
}

/// Get the set of already-applied migration IDs.
async fn get_applied_migrations(
    conn: &libsql::Connection,
) -> anyhow::Result<std::collections::HashSet<String>> {
    let mut rows = conn
        .query("SELECT id FROM _migrations ORDER BY id", libsql::params![])
        .await?;

    let mut applied = std::collections::HashSet::new();
    while let Some(row) = rows.next().await? {
        if let Ok(id) = row.get::<String>(0) {
            applied.insert(id);
        }
    }
    Ok(applied)
}

/// Record a migration as applied.
async fn record_migration(conn: &libsql::Connection, id: u64, name: &str) -> anyhow::Result<()> {
    conn.execute(
        "INSERT INTO _migrations (id, name) VALUES (?1, ?2)",
        libsql::params![id.to_string(), name],
    )
    .await?;
    Ok(())
}

/// Ordered list of migrations: (id, name, SQL)
const MIGRATIONS: &[(u64, &str, &str)] = &[
    (
        1,
        "create_sessions",
        "CREATE TABLE IF NOT EXISTS sessions (
            id          TEXT PRIMARY KEY,
            title       TEXT NOT NULL DEFAULT '',
            created_at  TEXT NOT NULL,
            updated_at  TEXT NOT NULL,
            messages    TEXT NOT NULL DEFAULT '[]',
            metadata    TEXT NOT NULL DEFAULT '{}'
        )",
    ),
    (
        2,
        "create_memories",
        "CREATE TABLE IF NOT EXISTS memories (
            id           TEXT PRIMARY KEY,
            session_id   TEXT NOT NULL,
            content      TEXT NOT NULL,
            insight_type TEXT NOT NULL DEFAULT 'general',
            tags         TEXT NOT NULL DEFAULT '[]',
            created_at   TEXT NOT NULL,
            updated_at   TEXT NOT NULL
        )",
    ),
    (
        3,
        "create_config",
        "CREATE TABLE IF NOT EXISTS config (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )",
    ),
    (
        4,
        "create_events",
        "CREATE TABLE IF NOT EXISTS events (
            id         TEXT PRIMARY KEY,
            event_type TEXT NOT NULL,
            data       TEXT NOT NULL DEFAULT '{}',
            timestamp  TEXT NOT NULL,
            session_id TEXT
        )",
    ),
    (
        5,
        "create_memory_tags_index",
        "CREATE INDEX IF NOT EXISTS idx_memories_tags ON memories(tags)",
    ),
    (
        6,
        "create_events_type_index",
        "CREATE INDEX IF NOT EXISTS idx_events_type ON events(event_type)",
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migrations_ordered() {
        for window in MIGRATIONS.windows(2) {
            assert!(
                window[0].0 < window[1].0,
                "Migrations must be ordered by ID"
            );
        }
    }

    #[test]
    fn test_migrations_non_empty() {
        assert!(!MIGRATIONS.is_empty());
        assert_eq!(MIGRATIONS[0].0, 1);
    }
}
