use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, OptionalExtension};
use writing_studio_core::{Error, Id, Result, Timestamp};
use writing_studio_document::Document;

use crate::{sqlite, ProjectStore};

#[derive(Clone, Debug)]
pub struct SnapshotInfo {
    pub id: Id,
    pub document_id: Id,
    pub created_at: Timestamp,
    pub label: String,
    pub word_count: u32,
    pub preview: String,
}

impl SnapshotInfo {
    pub fn created_label(&self) -> String {
        self.created_at
            .inner()
            .format("%Y-%m-%d %H:%M UTC")
            .to_string()
    }
}

#[derive(Clone, Debug)]
pub struct BackupInfo {
    pub path: PathBuf,
    pub file_name: String,
    pub modified_label: String,
    pub size_bytes: u64,
}

impl BackupInfo {
    pub fn size_label(&self) -> String {
        if self.size_bytes < 1024 {
            format!("{} B", self.size_bytes)
        } else {
            format!("{:.1} KB", self.size_bytes as f64 / 1024.0)
        }
    }
}

impl ProjectStore {
    pub fn list_snapshots(&self, document_id: Id) -> Result<Vec<SnapshotInfo>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, document_id, content_json, created_at, label
                 FROM snapshots
                 WHERE document_id = ?1
                 ORDER BY created_at DESC",
            )
            .map_err(sqlite)?;
        let rows = stmt
            .query_map(params![document_id.to_string()], |row| {
                let json: String = row.get(2)?;
                let created_raw: String = row.get(3)?;
                let label: Option<String> = row.get(4)?;
                Ok(snapshot_from_row(
                    &row.get::<_, String>(0)?,
                    &row.get::<_, String>(1)?,
                    &json,
                    &created_raw,
                    label.unwrap_or_default(),
                ))
            })
            .map_err(sqlite)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(sqlite)?;
        Ok(rows)
    }

    /// Read the text a snapshot holds without changing the live document.
    pub fn load_snapshot(&self, snapshot_id: Id) -> Result<Document> {
        let (document_id, json): (String, String) = self
            .conn
            .query_row(
                "SELECT document_id, content_json FROM snapshots WHERE id = ?1",
                params![snapshot_id.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(sqlite)?
            .ok_or_else(|| Error::message("snapshot was not found"))?;

        let document_id = document_id
            .parse::<Id>()
            .map_err(|_| Error::message("snapshot has an invalid document id"))?;
        let mut document =
            Document::from_json(&json).map_err(|err| Error::message(err.to_string()))?;
        document.id = document_id;
        Ok(document)
    }

    pub fn restore_snapshot(&self, snapshot_id: Id) -> Result<Document> {
        let mut restored = self.load_snapshot(snapshot_id)?;
        let document_id = restored.id;
        restored.updated_at = Timestamp::now();

        if let Ok(current) = self.load_document(document_id) {
            if current.to_plain_text() != restored.to_plain_text() {
                let _ = self.snapshot_document(&current, "pre-restore");
            }
        }

        self.save_document(&restored)?;
        Ok(restored)
    }

    /// Keep `baseline` as the text tracked changes are measured from. The
    /// start time is kept when a baseline is replaced.
    pub fn save_tracking_baseline(&self, baseline: &Document) -> Result<()> {
        let json = baseline
            .to_json()
            .map_err(|err| Error::message(err.to_string()))?;
        self.conn
            .execute(
                "INSERT INTO tracking_baselines (document_id, content_json, started_at)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(document_id) DO UPDATE SET content_json = excluded.content_json",
                params![baseline.id.to_string(), json, Timestamp::now().to_rfc3339()],
            )
            .map_err(sqlite)?;
        Ok(())
    }

    /// The baseline and when tracking began, or `None` when the document is
    /// not tracked.
    pub fn load_tracking_baseline(&self, document_id: Id) -> Result<Option<(Document, Timestamp)>> {
        let row: Option<(String, String)> = self
            .conn
            .query_row(
                "SELECT content_json, started_at FROM tracking_baselines WHERE document_id = ?1",
                params![document_id.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(sqlite)?;
        let Some((json, started)) = row else {
            return Ok(None);
        };
        let mut baseline =
            Document::from_json(&json).map_err(|err| Error::message(err.to_string()))?;
        baseline.id = document_id;
        let started = Timestamp::parse(&started).unwrap_or_else(|_| Timestamp::now());
        Ok(Some((baseline, started)))
    }

    pub fn stop_tracking(&self, document_id: Id) -> Result<()> {
        self.conn
            .execute(
                "DELETE FROM tracking_baselines WHERE document_id = ?1",
                params![document_id.to_string()],
            )
            .map_err(sqlite)?;
        Ok(())
    }

    pub fn list_backups(&self) -> Result<Vec<BackupInfo>> {
        let dir = self.root.join("backups");
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut backups = Vec::new();
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if !name.ends_with(".sqlite") {
                continue;
            }
            let file_name = name.to_string();
            let meta = entry.metadata()?;
            if !meta.is_file() {
                continue;
            }
            let modified = meta
                .modified()
                .ok()
                .map(|time| {
                    chrono::DateTime::<chrono::Utc>::from(time)
                        .format("%Y-%m-%d %H:%M UTC")
                        .to_string()
                })
                .unwrap_or_else(|| "unknown".into());
            backups.push(BackupInfo {
                path,
                file_name,
                modified_label: modified,
                size_bytes: meta.len(),
            });
        }
        backups.sort_by(|a, b| b.file_name.cmp(&a.file_name));
        Ok(backups)
    }

    pub fn restore_backup(&mut self, backup_path: &Path) -> Result<()> {
        if !backup_path.exists() {
            return Err(Error::message("backup file was not found"));
        }
        if !backup_path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(".sqlite"))
        {
            return Err(Error::message("backup must be a .sqlite file"));
        }

        let safety = self.backup()?;

        if let Err(err) = copy_database(backup_path, &mut self.conn) {
            let _ = copy_database(&safety, &mut self.conn);
            return Err(err);
        }

        self.conn
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .ok();

        if schema_ok(&self.conn).is_err() {
            copy_database(&safety, &mut self.conn)?;
            return Err(Error::message(
                "backup is not a Writing Studio project database; live project was left unchanged",
            ));
        }
        // A backup written by an older version lacks the tables added since.
        crate::schema::install(&self.conn).map_err(sqlite)?;
        Ok(())
    }
}

fn copy_database(from: &Path, dest: &mut Connection) -> Result<()> {
    let src = Connection::open(from).map_err(sqlite)?;
    let restore = rusqlite::backup::Backup::new(&src, dest).map_err(sqlite)?;
    restore
        .run_to_completion(5, std::time::Duration::from_millis(25), None)
        .map_err(sqlite)?;
    Ok(())
}

fn snapshot_from_row(
    id: &str,
    document_id: &str,
    json: &str,
    created_raw: &str,
    label: String,
) -> SnapshotInfo {
    let document = Document::from_json(json).ok();
    let preview = document
        .as_ref()
        .map(|doc| preview_text(&doc.to_plain_text()))
        .unwrap_or_default();
    let word_count = document.as_ref().map(Document::word_count).unwrap_or(0);
    SnapshotInfo {
        id: id.parse().unwrap_or_else(|_| Id::nil()),
        document_id: document_id.parse().unwrap_or_else(|_| Id::nil()),
        created_at: Timestamp::parse(created_raw).unwrap_or_else(|_| Timestamp::now()),
        label: if label.is_empty() {
            "snapshot".into()
        } else {
            label
        },
        word_count,
        preview,
    }
}

fn preview_text(text: &str) -> String {
    let collapsed: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    const LIMIT: usize = 90;
    if collapsed.chars().count() <= LIMIT {
        collapsed
    } else {
        let clipped: String = collapsed.chars().take(LIMIT).collect();
        format!("{clipped}…")
    }
}

fn schema_ok(conn: &Connection) -> Result<()> {
    conn.query_row("SELECT title FROM project_meta LIMIT 1", [], |row| {
        row.get::<_, String>(0)
    })
    .map_err(|_| Error::message("backup is not a Writing Studio project database"))?;
    Ok(())
}

