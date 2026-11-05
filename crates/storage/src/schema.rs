use rusqlite::Connection;

pub fn install(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS project_meta (
            id TEXT PRIMARY KEY,
            format_version INTEGER NOT NULL,
            title TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            settings_json TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS binder_nodes (
            id TEXT PRIMARY KEY,
            parent_id TEXT,
            kind TEXT NOT NULL,
            title TEXT NOT NULL,
            sort_index INTEGER NOT NULL,
            document_id TEXT,
            synopsis TEXT NOT NULL DEFAULT '',
            label TEXT NOT NULL DEFAULT '',
            status TEXT NOT NULL DEFAULT 'draft',
            color TEXT NOT NULL DEFAULT '',
            deleted_at TEXT,
            restore_parent_id TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS documents (
            id TEXT PRIMARY KEY,
            content_json TEXT NOT NULL,
            word_count INTEGER NOT NULL DEFAULT 0,
            char_count INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS snapshots (
            id TEXT PRIMARY KEY,
            document_id TEXT NOT NULL,
            content_json TEXT NOT NULL,
            created_at TEXT NOT NULL,
            label TEXT
        );
        "#,
    )?;
    let _ = conn.execute(
        "ALTER TABLE binder_nodes ADD COLUMN restore_parent_id TEXT",
        [],
    );
    Ok(())
}
