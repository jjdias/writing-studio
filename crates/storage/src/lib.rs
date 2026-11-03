//! Local project store.
//!
//! A Writing Studio project is a folder containing `FORMAT` and `project.sqlite`.
//! SQLite WAL mode plus transactional writes are the crash-safety foundation.
//! Backups copy the database into `backups/` without touching the live file
//! until the copy succeeds.

mod recovery;
mod schema;

pub use recovery::{BackupInfo, SnapshotInfo};

use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, OptionalExtension};
use writing_studio_core::{
    Error, Id, Result, Timestamp, PROJECT_DB_FILE, PROJECT_FORMAT_FILE, PROJECT_FORMAT_VERSION,
};
use writing_studio_document::Document;
use writing_studio_project::{BinderNode, BinderTree, NodeKind, ProjectMeta, ProjectSettings};
use writing_studio_templates::blank_novel;

pub struct ProjectStore {
    pub root: PathBuf,
    pub(crate) conn: Connection,
}

impl ProjectStore {
    pub fn create(root: impl AsRef<Path>, title: &str) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root)?;
        fs::create_dir_all(root.join("backups"))?;
        fs::create_dir_all(root.join("attachments"))?;
        fs::write(
            root.join(PROJECT_FORMAT_FILE),
            format!("writing-studio-project {PROJECT_FORMAT_VERSION}\n"),
        )?;

        let db_path = root.join(PROJECT_DB_FILE);
        if db_path.exists() {
            return Err(Error::message("a project already exists in this folder"));
        }

        let conn = open_connection(&db_path)?;
        schema::install(&conn).map_err(sqlite)?;
        let store = Self { root, conn };
        let seeded = blank_novel(title);
        store.persist_meta(&seeded.meta)?;
        for node in &seeded.nodes {
            store.upsert_node(node)?;
        }
        for document in &seeded.documents {
            store.save_document(document)?;
        }
        Ok(store)
    }

    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        let db_path = root.join(PROJECT_DB_FILE);
        if !db_path.exists() {
            return Err(Error::ProjectNotFound(root));
        }
        read_format_version(&root)?;
        let conn = open_connection(&db_path)?;
        schema::install(&conn).map_err(sqlite)?;
        Ok(Self { root, conn })
    }

    pub fn load_meta(&self) -> Result<ProjectMeta> {
        self.conn
            .query_row(
                "SELECT id, format_version, title, created_at, updated_at, settings_json FROM project_meta LIMIT 1",
                [],
                |row| {
                    let settings_json: String = row.get(5)?;
                    let settings: ProjectSettings =
                        serde_json::from_str(&settings_json).unwrap_or_default();
                    Ok(ProjectMeta {
                        id: parse_id(&row.get::<_, String>(0)?),
                        format_version: row.get(1)?,
                        title: row.get(2)?,
                        created_at: parse_ts(&row.get::<_, String>(3)?),
                        updated_at: parse_ts(&row.get::<_, String>(4)?),
                        settings,
                    })
                },
            )
            .map_err(sqlite)
    }

    pub fn persist_meta(&self, meta: &ProjectMeta) -> Result<()> {
        let settings = serde_json::to_string(&meta.settings).map_err(|err| Error::message(err.to_string()))?;
        self.conn
            .execute(
                "INSERT INTO project_meta (id, format_version, title, created_at, updated_at, settings_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(id) DO UPDATE SET
                    title=excluded.title,
                    updated_at=excluded.updated_at,
                    settings_json=excluded.settings_json",
                params![
                    meta.id.to_string(),
                    meta.format_version as i64,
                    meta.title,
                    meta.created_at.to_rfc3339(),
                    meta.updated_at.to_rfc3339(),
                    settings
                ],
            )
            .map_err(sqlite)?;
        Ok(())
    }

    pub fn load_binder(&self) -> Result<BinderTree> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, parent_id, kind, title, sort_index, document_id, synopsis, label, status, color,
                        deleted_at, created_at, updated_at
                 FROM binder_nodes",
            )
            .map_err(sqlite)?;
        let nodes = stmt
            .query_map([], |row| Ok(row_to_node(row)))
            .map_err(sqlite)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(sqlite)?;
        Ok(BinderTree::new(nodes))
    }

    pub fn upsert_node(&self, node: &BinderNode) -> Result<()> {
        self.conn
            .execute(
                "INSERT INTO binder_nodes (
                    id, parent_id, kind, title, sort_index, document_id, synopsis, label, status, color,
                    deleted_at, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
                 ON CONFLICT(id) DO UPDATE SET
                    parent_id=excluded.parent_id,
                    title=excluded.title,
                    sort_index=excluded.sort_index,
                    document_id=excluded.document_id,
                    synopsis=excluded.synopsis,
                    label=excluded.label,
                    status=excluded.status,
                    color=excluded.color,
                    deleted_at=excluded.deleted_at,
                    updated_at=excluded.updated_at",
                params![
                    node.id.to_string(),
                    node.parent_id.map(|id| id.to_string()),
                    node.kind.as_str(),
                    node.title,
                    node.sort_index,
                    node.document_id.map(|id| id.to_string()),
                    node.synopsis,
                    node.label,
                    node.status,
                    node.color,
                    node.deleted_at.map(|ts| ts.to_rfc3339()),
                    node.created_at.to_rfc3339(),
                    node.updated_at.to_rfc3339(),
                ],
            )
            .map_err(sqlite)?;
        Ok(())
    }

    pub fn load_document(&self, id: Id) -> Result<Document> {
        let json: Option<String> = self
            .conn
            .query_row(
                "SELECT content_json FROM documents WHERE id = ?1",
                params![id.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(sqlite)?;
        match json {
            Some(json) => Document::from_json(&json).map_err(|err| Error::message(err.to_string())),
            None => Err(Error::DocumentNotFound(id.to_string())),
        }
    }

    pub fn load_or_create_document(&self, id: Id) -> Result<Document> {
        match self.load_document(id) {
            Ok(doc) => Ok(doc),
            Err(Error::DocumentNotFound(_)) => {
                let doc = Document::new(id);
                self.save_document(&doc)?;
                Ok(doc)
            }
            Err(err) => Err(err),
        }
    }

    pub fn save_document(&self, document: &Document) -> Result<()> {
        let json = document
            .to_json()
            .map_err(|err| Error::message(err.to_string()))?;
        let tx = self.conn.unchecked_transaction().map_err(sqlite)?;
        tx.execute(
            "INSERT INTO documents (id, content_json, word_count, char_count, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET
                content_json=excluded.content_json,
                word_count=excluded.word_count,
                char_count=excluded.char_count,
                updated_at=excluded.updated_at",
            params![
                document.id.to_string(),
                json,
                document.word_count() as i64,
                document.char_count() as i64,
                document.updated_at.to_rfc3339(),
                document.updated_at.to_rfc3339(),
            ],
        )
        .map_err(sqlite)?;
        tx.commit().map_err(sqlite)?;
        Ok(())
    }

    pub fn snapshot_document(&self, document: &Document, label: &str) -> Result<Id> {
        let id = Id::new();
        let json = document
            .to_json()
            .map_err(|err| Error::message(err.to_string()))?;
        self.conn
            .execute(
                "INSERT INTO snapshots (id, document_id, content_json, created_at, label)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    id.to_string(),
                    document.id.to_string(),
                    json,
                    Timestamp::now().to_rfc3339(),
                    label
                ],
            )
            .map_err(sqlite)?;
        Ok(id)
    }

    pub fn backup(&self) -> Result<PathBuf> {
        let stamp = Timestamp::now()
            .to_rfc3339()
            .replace(':', "-")
            .replace('+', "_");
        let dest = self.root.join("backups").join(format!("project-{stamp}.sqlite"));
        let mut dest_conn = Connection::open(&dest).map_err(sqlite)?;
        let backup = rusqlite::backup::Backup::new(&self.conn, &mut dest_conn).map_err(sqlite)?;
        backup
            .run_to_completion(5, std::time::Duration::from_millis(25), None)
            .map_err(sqlite)?;
        Ok(dest)
    }

    pub fn document_word_count(&self, id: Id) -> u32 {
        self.conn
            .query_row(
                "SELECT word_count FROM documents WHERE id = ?1",
                params![id.to_string()],
                |row| row.get::<_, i64>(0),
            )
            .ok()
            .unwrap_or(0) as u32
    }
}

pub(crate) fn open_connection(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path).map_err(sqlite)?;
    conn.pragma_update(None, "journal_mode", "WAL").map_err(sqlite)?;
    conn.pragma_update(None, "synchronous", "NORMAL").map_err(sqlite)?;
    conn.pragma_update(None, "foreign_keys", "ON").map_err(sqlite)?;
    Ok(conn)
}

fn read_format_version(root: &Path) -> Result<u32> {
    let path = root.join(PROJECT_FORMAT_FILE);
    if !path.exists() {
        return Ok(PROJECT_FORMAT_VERSION);
    }
    let text = fs::read_to_string(path)?;
    let version = text
        .split_whitespace()
        .last()
        .and_then(|part| part.parse().ok())
        .unwrap_or(PROJECT_FORMAT_VERSION);
    if version > PROJECT_FORMAT_VERSION {
        return Err(Error::UnsupportedFormat {
            found: version,
            expected: PROJECT_FORMAT_VERSION,
        });
    }
    Ok(version)
}

fn row_to_node(row: &rusqlite::Row<'_>) -> BinderNode {
    let parent: Option<String> = row.get(1).unwrap_or(None);
    let document: Option<String> = row.get(5).unwrap_or(None);
    let deleted: Option<String> = row.get(10).unwrap_or(None);
    let kind_raw: String = row.get(2).unwrap_or_else(|_| "note".into());
    BinderNode {
        id: parse_id(&row.get::<_, String>(0).unwrap_or_default()),
        parent_id: parent.as_deref().map(parse_id),
        kind: NodeKind::parse(&kind_raw).unwrap_or(NodeKind::Note),
        title: row.get(3).unwrap_or_default(),
        sort_index: row.get(4).unwrap_or(0),
        document_id: document.as_deref().map(parse_id),
        synopsis: row.get(6).unwrap_or_default(),
        label: row.get(7).unwrap_or_default(),
        status: row.get(8).unwrap_or_default(),
        color: row.get(9).unwrap_or_default(),
        deleted_at: deleted.as_deref().and_then(|value| Timestamp::parse(value).ok()),
        created_at: parse_ts(&row.get::<_, String>(11).unwrap_or_default()),
        updated_at: parse_ts(&row.get::<_, String>(12).unwrap_or_default()),
    }
}

fn parse_id(value: &str) -> Id {
    value.parse().unwrap_or_else(|_| Id::nil())
}

fn parse_ts(value: &str) -> Timestamp {
    Timestamp::parse(value).unwrap_or_else(|_| Timestamp::now())
}

pub(crate) fn sqlite(err: rusqlite::Error) -> Error {
    Error::message(err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn create_open_and_round_trip_scene() {
        let dir = env::temp_dir().join(format!("ws-test-{}", Id::new()));
        let store = ProjectStore::create(&dir, "Test Novel").unwrap();
        let tree = store.load_binder().unwrap();
        let scene = tree
            .nodes
            .iter()
            .find(|node| node.kind == NodeKind::Scene)
            .unwrap();
        let doc_id = scene.document_id.unwrap();
        let mut document = store.load_document(doc_id).unwrap();
        document.replace_plain_text("She opened the door.");
        store.save_document(&document).unwrap();

        let reopened = ProjectStore::open(&dir).unwrap();
        let loaded = reopened.load_document(doc_id).unwrap();
        assert_eq!(loaded.to_plain_text(), "She opened the door.");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn snapshot_restore_replaces_document_and_keeps_pre_restore_copy() {
        let dir = env::temp_dir().join(format!("ws-snap-{}", Id::new()));
        let store = ProjectStore::create(&dir, "Snap Novel").unwrap();
        let tree = store.load_binder().unwrap();
        let doc_id = tree
            .nodes
            .iter()
            .find(|node| node.kind == NodeKind::Scene)
            .and_then(|node| node.document_id)
            .unwrap();

        let mut document = store.load_document(doc_id).unwrap();
        document.replace_plain_text("Original scene.");
        store.save_document(&document).unwrap();
        let snap = store.snapshot_document(&document, "keep").unwrap();

        document.replace_plain_text("Changed scene.");
        store.save_document(&document).unwrap();

        let restored = store.restore_snapshot(snap).unwrap();
        assert_eq!(restored.to_plain_text(), "Original scene.");
        assert_eq!(
            store.load_document(doc_id).unwrap().to_plain_text(),
            "Original scene."
        );
        let snaps = store.list_snapshots(doc_id).unwrap();
        assert!(snaps.iter().any(|item| item.label == "pre-restore"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn backup_restore_reloads_earlier_project_state() {
        let dir = env::temp_dir().join(format!("ws-bak-{}", Id::new()));
        let mut store = ProjectStore::create(&dir, "Backup Novel").unwrap();
        let tree = store.load_binder().unwrap();
        let doc_id = tree
            .nodes
            .iter()
            .find(|node| node.kind == NodeKind::Scene)
            .and_then(|node| node.document_id)
            .unwrap();

        let mut document = store.load_document(doc_id).unwrap();
        document.replace_plain_text("Version one.");
        store.save_document(&document).unwrap();
        let backup = store.backup().unwrap();

        document.replace_plain_text("Version two.");
        store.save_document(&document).unwrap();
        assert_eq!(
            store.load_document(doc_id).unwrap().to_plain_text(),
            "Version two."
        );

        store.restore_backup(&backup).unwrap();
        assert_eq!(
            store.load_document(doc_id).unwrap().to_plain_text(),
            "Version one."
        );
        assert!(!store.list_backups().unwrap().is_empty());
        let _ = fs::remove_dir_all(dir);
    }
}
