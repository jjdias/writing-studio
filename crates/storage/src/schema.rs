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

        CREATE TABLE IF NOT EXISTS characters (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            role TEXT NOT NULL DEFAULT '',
            summary TEXT NOT NULL DEFAULT '',
            sort_index INTEGER NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS character_motivations (
            id TEXT PRIMARY KEY,
            character_id TEXT NOT NULL,
            body TEXT NOT NULL DEFAULT '',
            sort_index INTEGER NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS character_relationships (
            id TEXT PRIMARY KEY,
            from_id TEXT NOT NULL,
            to_id TEXT NOT NULL,
            kind TEXT NOT NULL DEFAULT '',
            sort_index INTEGER NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS cultures (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            summary TEXT NOT NULL DEFAULT '',
            sort_index INTEGER NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS locations (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            summary TEXT NOT NULL DEFAULT '',
            culture_id TEXT,
            sort_index INTEGER NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS lore_entries (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            body TEXT NOT NULL DEFAULT '',
            location_id TEXT,
            culture_id TEXT,
            sort_index INTEGER NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS timeline_events (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            when_label TEXT NOT NULL DEFAULT '',
            summary TEXT NOT NULL DEFAULT '',
            location_id TEXT,
            sort_index INTEGER NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS research_items (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            notes TEXT NOT NULL DEFAULT '',
            sort_index INTEGER NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS research_files (
            id TEXT PRIMARY KEY,
            item_id TEXT NOT NULL,
            file_name TEXT NOT NULL,
            stored_name TEXT NOT NULL,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS bible_scene_links (
            owner_kind TEXT NOT NULL,
            owner_id TEXT NOT NULL,
            scene_id TEXT NOT NULL,
            sort_index INTEGER NOT NULL,
            PRIMARY KEY (owner_kind, owner_id, scene_id)
        );
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
    // Whether an item is left out of compile, 1 or 0.
    let _ = conn.execute(
        "ALTER TABLE binder_nodes ADD COLUMN left_out INTEGER NOT NULL DEFAULT 0",
        [],
    );
    conn.execute_batch("CREATE TABLE IF NOT EXISTS binder_lists (name TEXT PRIMARY KEY, json TEXT NOT NULL);")?;
    // Which entry of the bible each entry is inside (Phase 11).
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS bible_parents (owner_kind TEXT NOT NULL, id TEXT PRIMARY KEY, parent_id TEXT NOT NULL);",
    )?;
    Ok(())
}
