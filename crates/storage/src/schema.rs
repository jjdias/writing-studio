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

        CREATE TABLE IF NOT EXISTS tracking_baselines (
            document_id TEXT PRIMARY KEY,
            content_json TEXT NOT NULL,
            started_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS scene_beats (
            node_id TEXT PRIMARY KEY,
            goal TEXT NOT NULL DEFAULT '',
            conflict TEXT NOT NULL DEFAULT '',
            outcome TEXT NOT NULL DEFAULT ''
        );

        CREATE TABLE IF NOT EXISTS plotlines (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            color TEXT NOT NULL DEFAULT '',
            sort_index INTEGER NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS plot_points (
            id TEXT PRIMARY KEY,
            plotline_id TEXT NOT NULL,
            title TEXT NOT NULL,
            summary TEXT NOT NULL DEFAULT '',
            scene_id TEXT,
            sort_index INTEGER NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_plot_points_line ON plot_points(plotline_id);

        "#,
    )?;
    let _ = conn.execute(
        "ALTER TABLE binder_nodes ADD COLUMN restore_parent_id TEXT",
        [],
    );
    // Keywords, as a JSON list, and the binder's keyword colors and saved
    // filters. An older cache gets the column here; the error when it is
    // already there is the expected one.
    let _ = conn.execute(
        "ALTER TABLE binder_nodes ADD COLUMN keywords TEXT NOT NULL DEFAULT '[]'",
        [],
    );
    // The writer's notes on an item (Phase 13).
    let _ = conn.execute(
        "ALTER TABLE binder_nodes ADD COLUMN notes TEXT NOT NULL DEFAULT ''",
        [],
    );
    // Whether a folder of notes is compiled as footnotes, 1 or 0 (Phase 14).
    let _ = conn.execute(
        "ALTER TABLE binder_nodes ADD COLUMN footnotes INTEGER NOT NULL DEFAULT 0",
        [],
    );
    // A note's links to binder items, as a JSON list of ids (Phase 14).
    let _ = conn.execute(
        "ALTER TABLE binder_nodes ADD COLUMN links TEXT NOT NULL DEFAULT '[]'",
        [],
    );
    // Whether an item is left out of compile, 1 or 0.
    let _ = conn.execute(
        "ALTER TABLE binder_nodes ADD COLUMN left_out INTEGER NOT NULL DEFAULT 0",
        [],
    );
    conn.execute_batch("CREATE TABLE IF NOT EXISTS binder_lists (name TEXT PRIMARY KEY, json TEXT NOT NULL);")?;
    Ok(())
}
