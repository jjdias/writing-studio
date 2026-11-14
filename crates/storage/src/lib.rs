//! Local project store.
//!
//! A Writing Studio project is a folder containing `FORMAT` and `project.sqlite`.
//! SQLite WAL mode plus transactional writes are the crash-safety foundation.
//! Backups copy the database into `backups/` without touching the live file
//! until the copy succeeds.

mod recovery;
mod schema;

pub use recovery::{BackupInfo, SnapshotInfo};

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, OptionalExtension};
use writing_studio_core::{
    Error, Id, Result, Timestamp, PROJECT_DB_FILE, PROJECT_FORMAT_FILE, PROJECT_FORMAT_VERSION,
};
use writing_studio_characters::{Cast, Character, CharacterRelationship, Motivation};
use writing_studio_document::Document;
use writing_studio_plotting::{PlotGrid, PlotPoint, Plotline};
use writing_studio_project::{BinderNode, BinderTree, NodeKind, ProjectMeta, ProjectSettings};
use writing_studio_templates::blank_novel;
use writing_studio_worldbuilding::{Culture, Location, LoreEntry, TimelineEvent, World};
use writing_studio_research::{ResearchFile, ResearchItem, ResearchLibrary, stored_attachment_name};

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
        fs::create_dir_all(root.join("attachments"))?;
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
                        deleted_at, created_at, updated_at, restore_parent_id
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
                    deleted_at, created_at, updated_at, restore_parent_id
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
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
                    updated_at=excluded.updated_at,
                    restore_parent_id=excluded.restore_parent_id",
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
                    node.restore_parent_id.map(|id| id.to_string()),
                ],
            )
            .map_err(sqlite)?;
        Ok(())
    }

    pub fn persist_binder(&self, tree: &BinderTree) -> Result<()> {
        for node in &tree.nodes {
            self.upsert_node(node)?;
        }
        Ok(())
    }

    pub fn load_plot(&self) -> Result<PlotGrid> {
        let mut lines = self
            .conn
            .prepare(
                "SELECT id, title, color, sort_index, created_at, updated_at FROM plotlines",
            )
            .map_err(sqlite)?;
        let lines = lines
            .query_map([], |row| Ok(row_to_plotline(row)))
            .map_err(sqlite)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(sqlite)?;
        let mut points = self
            .conn
            .prepare(
                "SELECT id, plotline_id, title, summary, scene_id, sort_index, created_at, updated_at
                 FROM plot_points",
            )
            .map_err(sqlite)?;
        let points = points
            .query_map([], |row| Ok(row_to_plot_point(row)))
            .map_err(sqlite)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(sqlite)?;
        Ok(PlotGrid { lines, points })
    }

    pub fn persist_plot(&self, grid: &PlotGrid) -> Result<()> {
        let tx = self.conn.unchecked_transaction().map_err(sqlite)?;
        tx.execute("DELETE FROM plot_points", []).map_err(sqlite)?;
        tx.execute("DELETE FROM plotlines", []).map_err(sqlite)?;
        for line in &grid.lines {
            tx.execute(
                "INSERT INTO plotlines (id, title, color, sort_index, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    line.id.to_string(),
                    line.title,
                    line.color,
                    line.sort_index,
                    line.created_at.to_rfc3339(),
                    line.updated_at.to_rfc3339(),
                ],
            )
            .map_err(sqlite)?;
        }
        for point in &grid.points {
            tx.execute(
                "INSERT INTO plot_points (
                    id, plotline_id, title, summary, scene_id, sort_index, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    point.id.to_string(),
                    point.plotline_id.to_string(),
                    point.title,
                    point.summary,
                    point.scene_id.map(|id| id.to_string()),
                    point.sort_index,
                    point.created_at.to_rfc3339(),
                    point.updated_at.to_rfc3339(),
                ],
            )
            .map_err(sqlite)?;
        }
        tx.commit().map_err(sqlite)?;
        Ok(())
    }

    pub fn load_cast(&self) -> Result<Cast> {
        let mut characters = self
            .conn
            .prepare(
                "SELECT id, name, role, summary, sort_index, created_at, updated_at FROM characters",
            )
            .map_err(sqlite)?;
        let characters = characters
            .query_map([], |row| Ok(row_to_character(row)))
            .map_err(sqlite)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(sqlite)?;
        let character_scenes = load_scene_link_map(&self.conn, "character")?;
        let mut characters = characters;
        for character in &mut characters {
            character.scene_ids = apply_scene_links(&character_scenes, character.id);
        }
        let mut motivations = self
            .conn
            .prepare(
                "SELECT id, character_id, body, sort_index, created_at, updated_at
                 FROM character_motivations",
            )
            .map_err(sqlite)?;
        let motivations = motivations
            .query_map([], |row| Ok(row_to_motivation(row)))
            .map_err(sqlite)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(sqlite)?;
        let mut relationships = self
            .conn
            .prepare(
                "SELECT id, from_id, to_id, kind, sort_index, created_at, updated_at
                 FROM character_relationships",
            )
            .map_err(sqlite)?;
        let relationships = relationships
            .query_map([], |row| Ok(row_to_relationship(row)))
            .map_err(sqlite)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(sqlite)?;
        Ok(Cast {
            characters,
            motivations,
            relationships,
        })
    }

    pub fn persist_cast(&self, cast: &Cast) -> Result<()> {
        let tx = self.conn.unchecked_transaction().map_err(sqlite)?;
        tx.execute("DELETE FROM character_relationships", [])
            .map_err(sqlite)?;
        tx.execute("DELETE FROM character_motivations", [])
            .map_err(sqlite)?;
        tx.execute("DELETE FROM characters", []).map_err(sqlite)?;
        for character in &cast.characters {
            tx.execute(
                "INSERT INTO characters (id, name, role, summary, sort_index, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    character.id.to_string(),
                    character.name,
                    character.role,
                    character.summary,
                    character.sort_index,
                    character.created_at.to_rfc3339(),
                    character.updated_at.to_rfc3339(),
                ],
            )
            .map_err(sqlite)?;
        }
        for item in &cast.motivations {
            tx.execute(
                "INSERT INTO character_motivations (
                    id, character_id, body, sort_index, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    item.id.to_string(),
                    item.character_id.to_string(),
                    item.body,
                    item.sort_index,
                    item.created_at.to_rfc3339(),
                    item.updated_at.to_rfc3339(),
                ],
            )
            .map_err(sqlite)?;
        }
        for item in &cast.relationships {
            tx.execute(
                "INSERT INTO character_relationships (
                    id, from_id, to_id, kind, sort_index, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    item.id.to_string(),
                    item.from_id.to_string(),
                    item.to_id.to_string(),
                    item.kind,
                    item.sort_index,
                    item.created_at.to_rfc3339(),
                    item.updated_at.to_rfc3339(),
                ],
            )
            .map_err(sqlite)?;
        }
        write_scene_links(
            &tx,
            "character",
            cast.characters
                .iter()
                .map(|character| (character.id, character.scene_ids.clone())),
        )?;
        tx.commit().map_err(sqlite)?;
        Ok(())
    }

    pub fn load_world(&self) -> Result<World> {
        let cultures = query_rows(
            &self.conn,
            "SELECT id, name, summary, sort_index, created_at, updated_at FROM cultures",
            row_to_culture,
        )?;
        let locations = query_rows(
            &self.conn,
            "SELECT id, name, summary, culture_id, sort_index, created_at, updated_at FROM locations",
            row_to_location,
        )?;
        let lore = query_rows(
            &self.conn,
            "SELECT id, title, body, location_id, culture_id, sort_index, created_at, updated_at FROM lore_entries",
            row_to_lore,
        )?;
        let events = query_rows(
            &self.conn,
            "SELECT id, title, when_label, summary, location_id, sort_index, created_at, updated_at FROM timeline_events",
            row_to_event,
        )?;
        let mut locations = locations;
        let mut cultures = cultures;
        let mut lore = lore;
        let mut events = events;
        let location_scenes = load_scene_link_map(&self.conn, "location")?;
        let culture_scenes = load_scene_link_map(&self.conn, "culture")?;
        let lore_scenes = load_scene_link_map(&self.conn, "lore")?;
        let event_scenes = load_scene_link_map(&self.conn, "timeline")?;
        for place in &mut locations {
            place.scene_ids = apply_scene_links(&location_scenes, place.id);
        }
        for culture in &mut cultures {
            culture.scene_ids = apply_scene_links(&culture_scenes, culture.id);
        }
        for entry in &mut lore {
            entry.scene_ids = apply_scene_links(&lore_scenes, entry.id);
        }
        for event in &mut events {
            event.scene_ids = apply_scene_links(&event_scenes, event.id);
        }
        Ok(World {
            locations,
            cultures,
            lore,
            events,
        })
    }

    pub fn persist_world(&self, world: &World) -> Result<()> {
        let tx = self.conn.unchecked_transaction().map_err(sqlite)?;
        tx.execute("DELETE FROM timeline_events", []).map_err(sqlite)?;
        tx.execute("DELETE FROM lore_entries", []).map_err(sqlite)?;
        tx.execute("DELETE FROM locations", []).map_err(sqlite)?;
        tx.execute("DELETE FROM cultures", []).map_err(sqlite)?;
        for culture in &world.cultures {
            tx.execute(
                "INSERT INTO cultures (id, name, summary, sort_index, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    culture.id.to_string(),
                    culture.name,
                    culture.summary,
                    culture.sort_index,
                    culture.created_at.to_rfc3339(),
                    culture.updated_at.to_rfc3339(),
                ],
            )
            .map_err(sqlite)?;
        }
        for place in &world.locations {
            tx.execute(
                "INSERT INTO locations (id, name, summary, culture_id, sort_index, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    place.id.to_string(),
                    place.name,
                    place.summary,
                    place.culture_id.map(|id| id.to_string()),
                    place.sort_index,
                    place.created_at.to_rfc3339(),
                    place.updated_at.to_rfc3339(),
                ],
            )
            .map_err(sqlite)?;
        }
        for entry in &world.lore {
            tx.execute(
                "INSERT INTO lore_entries (
                    id, title, body, location_id, culture_id, sort_index, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    entry.id.to_string(),
                    entry.title,
                    entry.body,
                    entry.location_id.map(|id| id.to_string()),
                    entry.culture_id.map(|id| id.to_string()),
                    entry.sort_index,
                    entry.created_at.to_rfc3339(),
                    entry.updated_at.to_rfc3339(),
                ],
            )
            .map_err(sqlite)?;
        }
        for event in &world.events {
            tx.execute(
                "INSERT INTO timeline_events (
                    id, title, when_label, summary, location_id, sort_index, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    event.id.to_string(),
                    event.title,
                    event.when_label,
                    event.summary,
                    event.location_id.map(|id| id.to_string()),
                    event.sort_index,
                    event.created_at.to_rfc3339(),
                    event.updated_at.to_rfc3339(),
                ],
            )
            .map_err(sqlite)?;
        }
        write_scene_links(
            &tx,
            "location",
            world.locations.iter().map(|place| (place.id, place.scene_ids.clone())),
        )?;
        write_scene_links(
            &tx,
            "culture",
            world.cultures.iter().map(|culture| (culture.id, culture.scene_ids.clone())),
        )?;
        write_scene_links(
            &tx,
            "lore",
            world.lore.iter().map(|entry| (entry.id, entry.scene_ids.clone())),
        )?;
        write_scene_links(
            &tx,
            "timeline",
            world.events.iter().map(|event| (event.id, event.scene_ids.clone())),
        )?;
        tx.commit().map_err(sqlite)?;
        Ok(())
    }

    pub fn load_library(&self) -> Result<ResearchLibrary> {
        let items = query_rows(
            &self.conn,
            "SELECT id, title, notes, sort_index, created_at, updated_at FROM research_items",
            row_to_research_item,
        )?;
        let files = query_rows(
            &self.conn,
            "SELECT id, item_id, file_name, stored_name, created_at FROM research_files",
            row_to_research_file,
        )?;
        let mut items = items;
        let note_scenes = load_scene_link_map(&self.conn, "research")?;
        for item in &mut items {
            item.scene_ids = apply_scene_links(&note_scenes, item.id);
        }
        Ok(ResearchLibrary { items, files })
    }

    pub fn persist_library(&self, library: &ResearchLibrary) -> Result<()> {
        let tx = self.conn.unchecked_transaction().map_err(sqlite)?;
        tx.execute("DELETE FROM research_files", []).map_err(sqlite)?;
        tx.execute("DELETE FROM research_items", []).map_err(sqlite)?;
        for item in &library.items {
            tx.execute(
                "INSERT INTO research_items (id, title, notes, sort_index, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    item.id.to_string(),
                    item.title,
                    item.notes,
                    item.sort_index,
                    item.created_at.to_rfc3339(),
                    item.updated_at.to_rfc3339(),
                ],
            )
            .map_err(sqlite)?;
        }
        for file in &library.files {
            tx.execute(
                "INSERT INTO research_files (id, item_id, file_name, stored_name, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    file.id.to_string(),
                    file.item_id.to_string(),
                    file.file_name,
                    file.stored_name,
                    file.created_at.to_rfc3339(),
                ],
            )
            .map_err(sqlite)?;
        }
        write_scene_links(
            &tx,
            "research",
            library.items.iter().map(|item| (item.id, item.scene_ids.clone())),
        )?;
        tx.commit().map_err(sqlite)?;
        Ok(())
    }

    /// Copy a file into `attachments/` and record it on the note.
    pub fn import_research_file(
        &self,
        library: &ResearchLibrary,
        item_id: Id,
        source: &Path,
    ) -> Result<ResearchLibrary> {
        if !library.items.iter().any(|item| item.id == item_id) {
            return Err(Error::message("that research note does not exist"));
        }
        if !source.is_file() {
            return Err(Error::message("choose a file to attach"));
        }
        let file_name = source
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.is_empty())
            .unwrap_or("file")
            .to_string();
        let file_id = Id::new();
        let stored_name = stored_attachment_name(item_id, file_id, &file_name);
        let destination = self.attachment_path(&stored_name)?;
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(source, &destination)?;
        let mut next = library.clone();
        let added = next.add_file(ResearchFile {
            id: file_id,
            item_id,
            file_name,
            stored_name: stored_name.clone(),
            created_at: Timestamp::now(),
        });
        if !added {
            let _ = fs::remove_file(&destination);
            return Err(Error::message("could not record the attachment"));
        }
        if let Err(err) = self.persist_library(&next) {
            let _ = fs::remove_file(&destination);
            return Err(err);
        }
        Ok(next)
    }

    pub fn forget_research_file(
        &self,
        library: &ResearchLibrary,
        file_id: Id,
    ) -> Result<ResearchLibrary> {
        let mut next = library.clone();
        let Some(stored) = next.remove_file(file_id) else {
            return Err(Error::message("that attachment is not in the library"));
        };
        self.persist_library(&next)?;
        self.remove_stored_file(&stored)?;
        Ok(next)
    }

    pub fn forget_research_item(
        &self,
        library: &ResearchLibrary,
        item_id: Id,
    ) -> Result<ResearchLibrary> {
        let mut next = library.clone();
        let Some(stored) = next.remove_item(item_id) else {
            return Err(Error::message("that research note is not in the library"));
        };
        self.persist_library(&next)?;
        for name in stored {
            self.remove_stored_file(&name)?;
        }
        Ok(next)
    }

    fn attachment_path(&self, stored_name: &str) -> Result<PathBuf> {
        let mut parts = stored_name.split('/');
        let item = parts.next().unwrap_or("");
        let file = parts.next().unwrap_or("");
        if parts.next().is_some()
            || item.is_empty()
            || file.is_empty()
            || item.contains('.')
            || file.contains('/')
            || stored_name.contains('\\')
            || stored_name.contains("..")
        {
            return Err(Error::message("attachment path is not inside attachments/"));
        }
        Ok(self.root.join("attachments").join(item).join(file))
    }

    fn remove_stored_file(&self, stored_name: &str) -> Result<()> {
        let path = self.attachment_path(stored_name)?;
        if path.exists() {
            fs::remove_file(&path)?;
        }
        if let Some(parent) = path.parent() {
            let _ = fs::remove_dir(parent);
        }
        Ok(())
    }

    pub fn delete_node(&self, id: Id) -> Result<()> {
        self.conn
            .execute(
                "DELETE FROM binder_nodes WHERE id = ?1",
                params![id.to_string()],
            )
            .map_err(sqlite)?;
        Ok(())
    }

    pub fn delete_document(&self, id: Id) -> Result<()> {
        self.conn
            .execute(
                "DELETE FROM snapshots WHERE document_id = ?1",
                params![id.to_string()],
            )
            .map_err(sqlite)?;
        self.conn
            .execute(
                "DELETE FROM tracking_baselines WHERE document_id = ?1",
                params![id.to_string()],
            )
            .map_err(sqlite)?;
        self.conn
            .execute(
                "DELETE FROM documents WHERE id = ?1",
                params![id.to_string()],
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

    pub fn load_all_documents(&self) -> Result<Vec<Document>> {
        let mut stmt = self
            .conn
            .prepare("SELECT content_json FROM documents")
            .map_err(sqlite)?;
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(sqlite)?;
        let mut documents = Vec::new();
        for json in rows {
            let json = json.map_err(sqlite)?;
            documents.push(Document::from_json(&json).map_err(|err| Error::message(err.to_string()))?);
        }
        Ok(documents)
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
        restore_parent_id: row
            .get::<_, Option<String>>(13)
            .ok()
            .flatten()
            .as_deref()
            .map(parse_id),
        created_at: parse_ts(&row.get::<_, String>(11).unwrap_or_default()),
        updated_at: parse_ts(&row.get::<_, String>(12).unwrap_or_default()),
    }
}

fn row_to_plotline(row: &rusqlite::Row<'_>) -> Plotline {
    Plotline {
        id: parse_id(&row.get::<_, String>(0).unwrap_or_default()),
        title: row.get(1).unwrap_or_default(),
        color: row.get(2).unwrap_or_default(),
        sort_index: row.get(3).unwrap_or(0),
        created_at: parse_ts(&row.get::<_, String>(4).unwrap_or_default()),
        updated_at: parse_ts(&row.get::<_, String>(5).unwrap_or_default()),
    }
}

fn row_to_plot_point(row: &rusqlite::Row<'_>) -> PlotPoint {
    let scene: Option<String> = row.get(4).unwrap_or(None);
    PlotPoint {
        id: parse_id(&row.get::<_, String>(0).unwrap_or_default()),
        plotline_id: parse_id(&row.get::<_, String>(1).unwrap_or_default()),
        title: row.get(2).unwrap_or_default(),
        summary: row.get(3).unwrap_or_default(),
        scene_id: scene.as_deref().filter(|value| !value.is_empty()).map(parse_id),
        sort_index: row.get(5).unwrap_or(0),
        created_at: parse_ts(&row.get::<_, String>(6).unwrap_or_default()),
        updated_at: parse_ts(&row.get::<_, String>(7).unwrap_or_default()),
    }
}

fn query_rows<T>(
    conn: &Connection,
    sql: &str,
    map: fn(&rusqlite::Row<'_>) -> T,
) -> Result<Vec<T>> {
    let mut stmt = conn.prepare(sql).map_err(sqlite)?;
    let rows = stmt.query_map([], |row| Ok(map(row))).map_err(sqlite)?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(sqlite)
}

fn row_to_culture(row: &rusqlite::Row<'_>) -> Culture {
    Culture {
        id: parse_id(&row.get::<_, String>(0).unwrap_or_default()),
        name: row.get(1).unwrap_or_default(),
        summary: row.get(2).unwrap_or_default(),
        sort_index: row.get(3).unwrap_or(0),
        scene_ids: Vec::new(),
        created_at: parse_ts(&row.get::<_, String>(4).unwrap_or_default()),
        updated_at: parse_ts(&row.get::<_, String>(5).unwrap_or_default()),
    }
}

fn row_to_location(row: &rusqlite::Row<'_>) -> Location {
    Location {
        id: parse_id(&row.get::<_, String>(0).unwrap_or_default()),
        name: row.get(1).unwrap_or_default(),
        summary: row.get(2).unwrap_or_default(),
        culture_id: optional_id(row, 3),
        sort_index: row.get(4).unwrap_or(0),
        scene_ids: Vec::new(),
        created_at: parse_ts(&row.get::<_, String>(5).unwrap_or_default()),
        updated_at: parse_ts(&row.get::<_, String>(6).unwrap_or_default()),
    }
}

fn row_to_lore(row: &rusqlite::Row<'_>) -> LoreEntry {
    LoreEntry {
        id: parse_id(&row.get::<_, String>(0).unwrap_or_default()),
        title: row.get(1).unwrap_or_default(),
        body: row.get(2).unwrap_or_default(),
        location_id: optional_id(row, 3),
        culture_id: optional_id(row, 4),
        sort_index: row.get(5).unwrap_or(0),
        scene_ids: Vec::new(),
        created_at: parse_ts(&row.get::<_, String>(6).unwrap_or_default()),
        updated_at: parse_ts(&row.get::<_, String>(7).unwrap_or_default()),
    }
}

fn row_to_research_item(row: &rusqlite::Row<'_>) -> ResearchItem {
    ResearchItem {
        id: parse_id(&row.get::<_, String>(0).unwrap_or_default()),
        title: row.get(1).unwrap_or_default(),
        notes: row.get(2).unwrap_or_default(),
        sort_index: row.get(3).unwrap_or(0),
        scene_ids: Vec::new(),
        created_at: parse_ts(&row.get::<_, String>(4).unwrap_or_default()),
        updated_at: parse_ts(&row.get::<_, String>(5).unwrap_or_default()),
    }
}

fn row_to_research_file(row: &rusqlite::Row<'_>) -> ResearchFile {
    ResearchFile {
        id: parse_id(&row.get::<_, String>(0).unwrap_or_default()),
        item_id: parse_id(&row.get::<_, String>(1).unwrap_or_default()),
        file_name: row.get(2).unwrap_or_default(),
        stored_name: row.get(3).unwrap_or_default(),
        created_at: parse_ts(&row.get::<_, String>(4).unwrap_or_default()),
    }
}

fn row_to_event(row: &rusqlite::Row<'_>) -> TimelineEvent {
    TimelineEvent {
        id: parse_id(&row.get::<_, String>(0).unwrap_or_default()),
        title: row.get(1).unwrap_or_default(),
        when_label: row.get(2).unwrap_or_default(),
        summary: row.get(3).unwrap_or_default(),
        location_id: optional_id(row, 4),
        sort_index: row.get(5).unwrap_or(0),
        scene_ids: Vec::new(),
        created_at: parse_ts(&row.get::<_, String>(6).unwrap_or_default()),
        updated_at: parse_ts(&row.get::<_, String>(7).unwrap_or_default()),
    }
}

fn optional_id(row: &rusqlite::Row<'_>, index: usize) -> Option<Id> {
    row.get::<_, Option<String>>(index)
        .ok()
        .flatten()
        .filter(|value| !value.is_empty())
        .map(|value| parse_id(&value))
}

fn row_to_character(row: &rusqlite::Row<'_>) -> Character {
    Character {
        id: parse_id(&row.get::<_, String>(0).unwrap_or_default()),
        name: row.get(1).unwrap_or_default(),
        role: row.get(2).unwrap_or_default(),
        summary: row.get(3).unwrap_or_default(),
        sort_index: row.get(4).unwrap_or(0),
        scene_ids: Vec::new(),
        created_at: parse_ts(&row.get::<_, String>(5).unwrap_or_default()),
        updated_at: parse_ts(&row.get::<_, String>(6).unwrap_or_default()),
    }
}

fn load_scene_link_map(conn: &Connection, kind: &str) -> Result<HashMap<Id, Vec<Id>>> {
    let mut stmt = conn
        .prepare(
            "SELECT owner_id, scene_id FROM bible_scene_links
             WHERE owner_kind = ?1 ORDER BY sort_index",
        )
        .map_err(sqlite)?;
    let rows = stmt
        .query_map(params![kind], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(sqlite)?;
    let mut map: HashMap<Id, Vec<Id>> = HashMap::new();
    for row in rows {
        let (owner, scene) = row.map_err(sqlite)?;
        map.entry(parse_id(&owner)).or_default().push(parse_id(&scene));
    }
    Ok(map)
}

fn apply_scene_links(map: &HashMap<Id, Vec<Id>>, id: Id) -> Vec<Id> {
    map.get(&id).cloned().unwrap_or_default()
}

fn write_scene_links(
    tx: &rusqlite::Transaction<'_>,
    kind: &str,
    owners: impl IntoIterator<Item = (Id, Vec<Id>)>,
) -> Result<()> {
    tx.execute(
        "DELETE FROM bible_scene_links WHERE owner_kind = ?1",
        params![kind],
    )
    .map_err(sqlite)?;
    for (owner, scenes) in owners {
        for (index, scene) in scenes.iter().enumerate() {
            tx.execute(
                "INSERT INTO bible_scene_links (owner_kind, owner_id, scene_id, sort_index)
                 VALUES (?1, ?2, ?3, ?4)",
                params![kind, owner.to_string(), scene.to_string(), index as i64],
            )
            .map_err(sqlite)?;
        }
    }
    Ok(())
}

fn row_to_motivation(row: &rusqlite::Row<'_>) -> Motivation {
    Motivation {
        id: parse_id(&row.get::<_, String>(0).unwrap_or_default()),
        character_id: parse_id(&row.get::<_, String>(1).unwrap_or_default()),
        body: row.get(2).unwrap_or_default(),
        sort_index: row.get(3).unwrap_or(0),
        created_at: parse_ts(&row.get::<_, String>(4).unwrap_or_default()),
        updated_at: parse_ts(&row.get::<_, String>(5).unwrap_or_default()),
    }
}

fn row_to_relationship(row: &rusqlite::Row<'_>) -> CharacterRelationship {
    CharacterRelationship {
        id: parse_id(&row.get::<_, String>(0).unwrap_or_default()),
        from_id: parse_id(&row.get::<_, String>(1).unwrap_or_default()),
        to_id: parse_id(&row.get::<_, String>(2).unwrap_or_default()),
        kind: row.get(3).unwrap_or_default(),
        sort_index: row.get(4).unwrap_or(0),
        created_at: parse_ts(&row.get::<_, String>(5).unwrap_or_default()),
        updated_at: parse_ts(&row.get::<_, String>(6).unwrap_or_default()),
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
    fn loading_a_snapshot_leaves_the_live_document_alone() {
        let dir = env::temp_dir().join(format!("ws-snapload-{}", Id::new()));
        let store = ProjectStore::create(&dir, "Compare Novel").unwrap();
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

        let held = store.load_snapshot(snap).unwrap();
        assert_eq!(held.id, doc_id);
        assert_eq!(held.to_plain_text(), "Original scene.");
        assert_eq!(
            store.load_document(doc_id).unwrap().to_plain_text(),
            "Changed scene."
        );
        assert_eq!(store.list_snapshots(doc_id).unwrap().len(), 1);
        assert!(store.load_snapshot(Id::new()).is_err());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn tracking_baseline_is_kept_replaced_and_dropped_with_its_document() {
        let dir = env::temp_dir().join(format!("ws-track-{}", Id::new()));
        let store = ProjectStore::create(&dir, "Tracked Novel").unwrap();
        let doc_id = store
            .load_binder()
            .unwrap()
            .nodes
            .iter()
            .find(|node| node.kind == NodeKind::Scene)
            .and_then(|node| node.document_id)
            .unwrap();
        assert!(store.load_tracking_baseline(doc_id).unwrap().is_none());

        let mut document = store.load_document(doc_id).unwrap();
        document.replace_plain_text("First draft.");
        store.save_tracking_baseline(&document).unwrap();
        let (baseline, started) = store.load_tracking_baseline(doc_id).unwrap().unwrap();
        assert_eq!(baseline.to_plain_text(), "First draft.");

        document.replace_plain_text("Second draft.");
        store.save_tracking_baseline(&document).unwrap();
        let reopened = ProjectStore::open(&dir).unwrap();
        let (baseline, kept) = reopened.load_tracking_baseline(doc_id).unwrap().unwrap();
        assert_eq!(baseline.to_plain_text(), "Second draft.");
        assert_eq!(kept, started);

        reopened.stop_tracking(doc_id).unwrap();
        assert!(reopened.load_tracking_baseline(doc_id).unwrap().is_none());
        reopened.save_tracking_baseline(&document).unwrap();
        reopened.delete_document(doc_id).unwrap();
        assert!(reopened.load_tracking_baseline(doc_id).unwrap().is_none());
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

    #[test]
    fn plot_grid_round_trips_lines_points_and_scene_links() {
        let dir = env::temp_dir().join(format!("ws-plot-{}", Id::new()));
        let store = ProjectStore::create(&dir, "Plot Novel").unwrap();
        assert!(store.load_plot().unwrap().lines.is_empty());

        let tree = store.load_binder().unwrap();
        let scene = tree
            .nodes
            .iter()
            .find(|node| node.kind == NodeKind::Scene)
            .unwrap()
            .id;
        let mut grid = PlotGrid::new();
        let line = grid.add_line();
        grid.set_line_title(line, "Main plot".into());
        let point = grid.add_point(line).unwrap();
        grid.set_point_title(point, "Inciting incident".into());
        grid.set_point_summary(point, "She opens the door.".into());
        grid.set_point_scene(point, Some(scene));
        store.persist_plot(&grid).unwrap();

        let loaded = ProjectStore::open(&dir).unwrap().load_plot().unwrap();
        assert_eq!(loaded.lines.len(), 1);
        assert_eq!(loaded.lines[0].title, "Main plot");
        assert_eq!(loaded.points.len(), 1);
        assert_eq!(loaded.points[0].title, "Inciting incident");
        assert_eq!(loaded.points[0].summary, "She opens the door.");
        assert_eq!(loaded.points[0].scene_id, Some(scene));
        assert_eq!(loaded.points[0].plotline_id, loaded.lines[0].id);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn cast_round_trips_profiles_motivations_and_relationships() {
        let dir = env::temp_dir().join(format!("ws-cast-{}", Id::new()));
        let store = ProjectStore::create(&dir, "Cast Novel").unwrap();
        assert!(store.load_cast().unwrap().characters.is_empty());

        let mut cast = Cast::new();
        let hero = cast.add_character();
        let rival = cast.add_character();
        cast.set_name(hero, "Ada".into());
        cast.set_role(hero, "Protagonist".into());
        cast.set_summary(hero, "Keeps the door locked.".into());
        let motive = cast.add_motivation(hero).unwrap();
        cast.set_motivation(motive, "Protect the house".into());
        let link = cast.add_relationship(hero, rival).unwrap();
        cast.set_relationship_kind(link, "rival".into());
        let scene = Id::new();
        let later = Id::new();
        assert!(cast.link_scene(hero, scene));
        assert!(cast.link_scene(hero, later));
        store.persist_cast(&cast).unwrap();

        let loaded = ProjectStore::open(&dir).unwrap().load_cast().unwrap();
        assert_eq!(loaded.characters.len(), 2);
        let ada = loaded
            .characters
            .iter()
            .find(|character| character.name == "Ada")
            .unwrap();
        assert_eq!(ada.role, "Protagonist");
        assert_eq!(ada.summary, "Keeps the door locked.");
        assert_eq!(loaded.motivations.len(), 1);
        assert_eq!(loaded.motivations[0].body, "Protect the house");
        assert_eq!(loaded.motivations[0].character_id, ada.id);
        assert_eq!(loaded.relationships.len(), 1);
        assert_eq!(loaded.relationships[0].kind, "rival");
        assert_eq!(loaded.relationships[0].from_id, ada.id);
        assert_eq!(ada.scene_ids, vec![scene, later]);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn world_round_trips_places_cultures_lore_and_events() {
        let dir = env::temp_dir().join(format!("ws-world-{}", Id::new()));
        let store = ProjectStore::create(&dir, "World Novel").unwrap();
        assert!(store.load_world().unwrap().locations.is_empty());

        let mut world = World::new();
        let culture = world.add_culture();
        world.set_culture_name(culture, "River folk".into());
        world.set_culture_summary(culture, "They mark floods on stone.".into());
        let place = world.add_location();
        world.set_location_name(place, "Mill town".into());
        world.set_location_summary(place, "A town on the bend.".into());
        world.set_location_culture(place, Some(culture));
        let note = world.add_lore();
        world.set_lore_title(note, "The flood mark".into());
        world.set_lore_body(note, "A line cut into the mill wall.".into());
        world.set_lore_location(note, Some(place));
        world.set_lore_culture(note, Some(culture));
        let event = world.add_event();
        world.set_event_title(event, "The high water".into());
        world.set_event_when(event, "Year 12".into());
        world.set_event_summary(event, "The river took the lower street.".into());
        world.set_event_location(event, Some(place));
        store.persist_world(&world).unwrap();

        let loaded = ProjectStore::open(&dir).unwrap().load_world().unwrap();
        assert_eq!(loaded.cultures.len(), 1);
        assert_eq!(loaded.cultures[0].name, "River folk");
        assert_eq!(loaded.locations.len(), 1);
        assert_eq!(loaded.locations[0].name, "Mill town");
        assert_eq!(loaded.locations[0].culture_id, Some(loaded.cultures[0].id));
        assert_eq!(loaded.lore.len(), 1);
        assert_eq!(loaded.lore[0].title, "The flood mark");
        assert_eq!(loaded.lore[0].location_id, Some(loaded.locations[0].id));
        assert_eq!(loaded.events.len(), 1);
        assert_eq!(loaded.events[0].when_label, "Year 12");
        assert_eq!(loaded.events[0].location_id, Some(loaded.locations[0].id));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn research_library_copies_a_file_into_attachments_and_removes_it() {
        let dir = env::temp_dir().join(format!("ws-research-{}", Id::new()));
        let source = env::temp_dir().join(format!("ws-clip-{}.txt", Id::new()));
        fs::write(&source, b"coastal charts").unwrap();
        let store = ProjectStore::create(&dir, "Research Novel").unwrap();
        let mut library = ResearchLibrary::new();
        let item = library.add_item();
        library.set_title(item, "Maps".into());
        library.set_notes(item, "Coastal charts.".into());
        store.persist_library(&library).unwrap();

        let library = store
            .import_research_file(&library, item, &source)
            .unwrap();
        let copied = store
            .root
            .join("attachments")
            .join(&library.files[0].stored_name);
        assert_eq!(fs::read(&copied).unwrap(), b"coastal charts");

        let loaded = ProjectStore::open(&dir).unwrap().load_library().unwrap();
        assert_eq!(loaded.items[0].title, "Maps");
        assert_eq!(loaded.items[0].notes, "Coastal charts.");
        assert_eq!(loaded.files.len(), 1);
        assert_eq!(loaded.files[0].file_name, source.file_name().unwrap().to_str().unwrap());

        let library = store.forget_research_file(&loaded, loaded.files[0].id).unwrap();
        assert!(library.files.is_empty());
        assert!(!copied.exists());
        let library = store.forget_research_item(&library, item).unwrap();
        assert!(library.items.is_empty());
        let _ = fs::remove_file(&source);
        let _ = fs::remove_dir_all(dir);
    }
}
