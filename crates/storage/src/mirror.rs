//! Opening and saving a project whose files are the project.
//!
//! In format 2 the HTML and JSON files in the project folder are what is
//! kept, shared, and trusted. The SQLite file under `.local/` is a cache on
//! this machine: it is filled from the files when a project opens, and it
//! answers what the program asks while it runs. It also holds what belongs
//! to this machine only: snapshots and track-changes baselines.
//!
//! Every save writes the file first and the cache second, so the cache never
//! holds writing that is not in a file.

use std::cell::{Cell, RefCell};
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use rusqlite::{params, OptionalExtension};
use writing_studio_characters::Cast;
use writing_studio_core::{
    Error, Id, Result, PROJECT_DB_FILE, PROJECT_FORMAT_FILE, PROJECT_FORMAT_VERSION,
};
use writing_studio_diagnostics::SceneBeats;
use writing_studio_document::Document;
use writing_studio_plotting::PlotGrid;
use writing_studio_project::{BinderNode, BinderTree, ProjectMeta};
use writing_studio_research::ResearchLibrary;
use writing_studio_templates::blank_novel;
use writing_studio_worldbuilding::World;

use crate::files::{
    bible_files, Extras, binder_json, inside, place, plot_json, read_project_files, scene_text, settings_json,
    verify_project_files, write_atomic, write_format_2_marker, write_project_files, ProjectContent,
    LOCAL_SETTINGS,
};
use crate::{open_connection, row_to_node, schema, sqlite, ProjectStore};

const LOCAL_DIR: &str = ".local";
const CACHE_FILE: &str = "cache.sqlite";

/// What the store remembers about the files, so that a save writes only the
/// files it changes.
#[derive(Default)]
pub(crate) struct Mirror {
    /// The file of each item that has one.
    paths: RefCell<HashMap<Id, String>>,
    /// What the files hold that the program does not use, read when the
    /// project opened. Written back with each item, so that a save keeps it.
    extras: RefCell<HashMap<Id, Extras>>,
    /// A fingerprint of what each scene file's head was last written from.
    heads: RefCell<HashMap<Id, u64>>,
    /// A fingerprint of the text each small file was last written with.
    written: RefCell<HashMap<String, u64>>,
    /// Files are left alone: while the cache is being filled from them, and
    /// for a project opened only to be read.
    paused: Cell<bool>,
}

fn fingerprint(parts: &[&str]) -> u64 {
    let mut hasher = DefaultHasher::new();
    parts.hash(&mut hasher);
    hasher.finish()
}

fn head_fingerprint(node: &BinderNode, beats: &SceneBeats) -> u64 {
    fingerprint(&[
        &node.title,
        &node.synopsis,
        &node.status,
        &node.label,
        &node.color,
        &beats.goal,
        &beats.conflict,
        &beats.outcome,
    ])
}

fn format_version(root: &Path) -> Result<u32> {
    let path = root.join(PROJECT_FORMAT_FILE);
    // The first projects had no marker.
    let version = match fs::read_to_string(path) {
        Ok(text) => text.split_whitespace().last().and_then(|part| part.parse().ok()).unwrap_or(1),
        Err(_) => 1,
    };
    if version > PROJECT_FORMAT_VERSION {
        return Err(Error::UnsupportedFormat {
            found: version,
            expected: PROJECT_FORMAT_VERSION,
        });
    }
    Ok(version)
}

fn cache_path(root: &Path) -> PathBuf {
    root.join(LOCAL_DIR).join(CACHE_FILE)
}

/// Move a format 1 database to where format 2 keeps its cache. A conversion
/// that was interrupted after the folder was marked as format 2 leaves the
/// database behind; opening the project finishes the move.
fn adopt_old_database(root: &Path) -> Result<()> {
    let (old, cache) = (root.join(PROJECT_DB_FILE), cache_path(root));
    if cache.exists() || !old.exists() {
        return Ok(());
    }
    fs::create_dir_all(root.join(LOCAL_DIR))?;
    fs::rename(&old, &cache)?;
    for sidecar in ["-wal", "-shm"] {
        let _ = fs::remove_file(root.join(format!("{PROJECT_DB_FILE}{sidecar}")));
    }
    Ok(())
}

impl ProjectStore {
    fn with_cache(root: PathBuf, database: &Path, paused: bool) -> Result<Self> {
        let conn = open_connection(database)?;
        schema::install(&conn).map_err(sqlite)?;
        let store = Self {
            root,
            conn,
            mirror: Mirror::default(),
        };
        store.mirror.paused.set(paused);
        Ok(store)
    }

    /// Where this machine keeps its backups of the project.
    pub(crate) fn backups_dir(&self) -> PathBuf {
        if self.root.join(PROJECT_DB_FILE).exists() {
            // A format 1 project that was opened only to be read.
            return self.root.join("backups");
        }
        self.root.join(LOCAL_DIR).join("backups")
    }

    pub fn create(root: impl AsRef<Path>, title: &str) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root)?;
        if root.join("project.json").exists() || root.join(PROJECT_DB_FILE).exists() || cache_path(&root).exists() {
            return Err(Error::message("a project already exists in this folder"));
        }
        fs::create_dir_all(root.join(LOCAL_DIR))?;
        fs::create_dir_all(root.join("attachments"))?;

        let cache = cache_path(&root);
        let store = Self::with_cache(root, &cache, true)?;
        let seeded = blank_novel(title);
        store.persist_meta_cached(&seeded.meta)?;
        for node in &seeded.nodes {
            store.upsert_node_cached(node)?;
        }
        for document in &seeded.documents {
            store.save_document_cached(document)?;
        }
        store.write_files_from_cache()?;
        write_format_2_marker(&store.root)?;
        store.mirror.paused.set(false);
        Ok(store)
    }

    /// Whether opening this folder would convert it from format 1.
    pub fn needs_conversion(root: impl AsRef<Path>) -> bool {
        let root = root.as_ref();
        root.join(PROJECT_DB_FILE).exists() && matches!(format_version(root), Ok(1))
    }

    /// Open a project to work in it. A format 1 project is converted to
    /// format 2 first; see [`convert_format_1`](Self::convert_format_1).
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        if format_version(&root)? < 2 {
            Self::convert_format_1(&root)?;
        }
        if !root.join("project.json").exists() {
            return Err(Error::ProjectNotFound(root));
        }
        adopt_old_database(&root)?;
        fs::create_dir_all(root.join(LOCAL_DIR))?;
        fs::create_dir_all(root.join("attachments"))?;
        let cache = cache_path(&root);
        let store = Self::with_cache(root, &cache, true)?;
        store.rekey_documents()?;
        let content = read_project_files(&store.root)?;
        store.fill_cache(&content)?;
        store.mirror.remember(&content, content.files.clone());
        store.mirror.paused.set(false);
        Ok(store)
    }

    /// Open a project without changing it: a format 1 project is not
    /// converted, and no project file is ever written. For tools that only
    /// read.
    pub fn open_as_is(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        if format_version(&root)? >= 2 {
            let store = Self::open(root)?;
            store.mirror.paused.set(true);
            return Ok(store);
        }
        let database = root.join(PROJECT_DB_FILE);
        if !database.exists() {
            return Err(Error::ProjectNotFound(root));
        }
        Self::with_cache(root, &database, true)
    }

    /// Turn a format 1 project into format 2, in place.
    ///
    /// A backup of the database is written first. The files are written and
    /// then checked against the database. Only then is the folder marked as
    /// format 2 and the database moved to be this machine's cache, which
    /// keeps the project's snapshots. If anything fails before the mark, the
    /// project is still a format 1 project with every byte it had.
    fn convert_format_1(root: &Path) -> Result<()> {
        let database = root.join(PROJECT_DB_FILE);
        if !database.exists() {
            return Err(Error::ProjectNotFound(root.to_path_buf()));
        }
        {
            let store = Self::with_cache(root.to_path_buf(), &database, true)?;
            store.backup()?;
            store.rekey_documents()?;
            let content = ProjectContent::from_store(&store)?;
            write_project_files(root, &content)?;
            verify_project_files(root, &content).map_err(|err| {
                Error::message(format!(
                    "this project could not be converted to the new format, and was left as it was: {err}"
                ))
            })?;
            store
                .conn
                .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
                .map_err(sqlite)?;
        }
        write_format_2_marker(root)?;
        adopt_old_database(root)?;
        // Earlier backups go with the database they are copies of.
        let (old, new) = (root.join("backups"), root.join(LOCAL_DIR).join("backups"));
        if let Ok(entries) = fs::read_dir(&old) {
            fs::create_dir_all(&new)?;
            for entry in entries.flatten() {
                let _ = fs::rename(entry.path(), new.join(entry.file_name()));
            }
            let _ = fs::remove_dir(&old);
        }
        Ok(())
    }

    /// Give each item's text the item's own id, in the cache. Format 1 gave
    /// them separate ids. Snapshots and track-changes baselines follow.
    fn rekey_documents(&self) -> Result<()> {
        let pairs: Vec<(String, String)> = {
            let mut stmt = self
                .conn
                .prepare("SELECT id, document_id FROM binder_nodes WHERE document_id IS NOT NULL AND document_id != id")
                .map_err(sqlite)?;
            let rows = stmt
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
                .map_err(sqlite)?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(sqlite)?;
            rows
        };
        if pairs.is_empty() {
            return Ok(());
        }
        let tx = self.conn.unchecked_transaction().map_err(sqlite)?;
        for (node, document) in &pairs {
            for sql in [
                "UPDATE documents SET id = ?1 WHERE id = ?2",
                "UPDATE snapshots SET document_id = ?1 WHERE document_id = ?2",
                "UPDATE tracking_baselines SET document_id = ?1 WHERE document_id = ?2",
                "UPDATE binder_nodes SET document_id = ?1 WHERE document_id = ?2",
            ] {
                tx.execute(sql, params![node, document]).map_err(sqlite)?;
            }
        }
        tx.commit().map_err(sqlite)
    }

    /// Make the cache hold what the files hold. Snapshots and baselines are
    /// not in the files and are left as they are.
    fn fill_cache(&self, content: &ProjectContent) -> Result<()> {
        self.conn
            .execute("DELETE FROM project_meta WHERE id != ?1", params![content.meta.id.to_string()])
            .map_err(sqlite)?;
        self.persist_meta_cached(&content.meta)?;
        self.conn.execute("DELETE FROM binder_nodes", []).map_err(sqlite)?;
        self.persist_binder_cached(&content.binder)?;

        let kept: HashSet<String> = content.documents.iter().map(|doc| doc.id.to_string()).collect();
        let cached: Vec<String> = {
            let mut stmt = self.conn.prepare("SELECT id FROM documents").map_err(sqlite)?;
            let rows = stmt
                .query_map([], |row| row.get(0))
                .map_err(sqlite)?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(sqlite)?;
            rows
        };
        for id in cached.iter().filter(|id| !kept.contains(*id)) {
            self.conn
                .execute("DELETE FROM documents WHERE id = ?1", params![id])
                .map_err(sqlite)?;
        }
        for document in &content.documents {
            self.save_document_cached(document)?;
        }
        self.persist_scene_beats_cached(&content.beats)?;
        self.persist_plot_cached(&content.plot)?;
        self.persist_cast_cached(&content.cast)?;
        self.persist_world_cached(&content.world)?;
        self.persist_library_cached(&content.library)
    }

    /// Write every file from what the cache holds. Used when a project is
    /// made, and after a backup is restored into the cache.
    pub(crate) fn write_files_from_cache(&self) -> Result<()> {
        if self.root.join(PROJECT_DB_FILE).exists() {
            // A format 1 project opened only to be read has no files to write.
            return Ok(());
        }
        self.rekey_documents()?;
        let mut content = ProjectContent::from_store(self)?;
        content.files = self.mirror.paths.borrow().clone();
        content.extras = self.mirror.extras.borrow().clone();
        let files = write_project_files(&self.root, &content)?;
        self.mirror.remember(&content, files);
        Ok(())
    }

    fn mirroring(&self) -> bool {
        !self.mirror.paused.get()
    }

    /// Write a file inside the project folder.
    fn write_file(&self, path: &str, text: &str) -> Result<()> {
        write_atomic(&inside(&self.root, "the project", path)?, text.as_bytes())
    }

    /// Write one of the small files, unless it already holds this text.
    fn write_small(&self, path: &str, text: &str) -> Result<()> {
        let mark = fingerprint(&[text]);
        if self.mirror.written.borrow().get(path) == Some(&mark) {
            return Ok(());
        }
        let on_disk = fs::read_to_string(inside(&self.root, "the project", path)?).ok();
        if on_disk.as_deref() != Some(text) {
            self.write_file(path, text)?;
        }
        self.mirror.written.borrow_mut().insert(path.to_string(), mark);
        Ok(())
    }

    fn remove_file(&self, path: &str) -> Result<()> {
        match fs::remove_file(inside(&self.root, "the project", path)?) {
            Err(err) if err.kind() != std::io::ErrorKind::NotFound => Err(err.into()),
            _ => Ok(()),
        }
    }

    /// Write the scene file of every item whose title, synopsis, status,
    /// label, color, or beats changed, or that has no file yet. An item whose
    /// head is unchanged is not read or written.
    fn write_changed_scenes(&self, tree: &BinderTree, beats: &HashMap<Id, SceneBeats>) -> Result<()> {
        let none = SceneBeats::default();
        for node in &tree.nodes {
            let Some(document_id) = node.document_id else {
                continue;
            };
            let scene_beats = beats.get(&node.id).unwrap_or(&none);
            let mark = head_fingerprint(node, scene_beats);
            let has_file = self.mirror.paths.borrow().contains_key(&node.id);
            if has_file && self.mirror.heads.borrow().get(&node.id) == Some(&mark) {
                continue;
            }
            let blocks = self
                .load_document(document_id)
                .map(|document| document.blocks)
                .unwrap_or_default();
            let path = self.mirror.path_for(node.id, "manuscript", &node.title);
            let text = scene_text(node, scene_beats, &blocks, self.mirror.extras.borrow().get(&node.id));
            self.write_file(&path, &text)?;
            self.mirror.heads.borrow_mut().insert(node.id, mark);
        }
        Ok(())
    }

    fn write_binder(&self, tree: &BinderTree) -> Result<()> {
        let text = binder_json(tree, &self.mirror.paths.borrow(), &self.mirror.extras.borrow());
        self.write_small("binder.json", &text)
    }

    /// Write the bible's files. `folders` are the ones this save owns: an
    /// entry there that is no longer in the bible was deleted by the writer,
    /// and its file goes too.
    fn write_bible(&self, cast: &Cast, world: &World, library: &ResearchLibrary, folders: &[&str]) -> Result<()> {
        let known = self.mirror.paths.borrow().clone();
        let mut used: HashSet<String> = known.values().cloned().collect();
        let (entries, bible) = bible_files(cast, world, library, &known, &mut used, &self.mirror.extras.borrow());
        let mut present = HashSet::new();
        for (id, path, text) in &entries {
            self.write_small(path, text)?;
            self.mirror.paths.borrow_mut().insert(*id, path.clone());
            present.insert(*id);
        }
        self.write_small("bible.json", &bible)?;
        let gone: Vec<(Id, String)> = known
            .into_iter()
            .filter(|(id, path)| !present.contains(id) && folders.iter().any(|folder| path.starts_with(folder)))
            .collect();
        for (id, path) in gone {
            self.remove_file(&path)?;
            self.mirror.paths.borrow_mut().remove(&id);
            self.mirror.written.borrow_mut().remove(&path);
        }
        Ok(())
    }

    /// The binder item whose text has this id.
    fn item_of_document(&self, document_id: Id) -> Result<Option<BinderNode>> {
        self.conn
            .query_row(
                "SELECT id, parent_id, kind, title, sort_index, document_id, synopsis, label, status, color,
                        deleted_at, created_at, updated_at, restore_parent_id
                 FROM binder_nodes WHERE document_id = ?1",
                params![document_id.to_string()],
                |row| Ok(row_to_node(row)),
            )
            .optional()
            .map_err(sqlite)
    }

    fn beats_of(&self, node_id: Id) -> Result<SceneBeats> {
        let found = self
            .conn
            .query_row(
                "SELECT goal, conflict, outcome FROM scene_beats WHERE node_id = ?1",
                params![node_id.to_string()],
                |row| {
                    Ok(SceneBeats {
                        goal: row.get(0)?,
                        conflict: row.get(1)?,
                        outcome: row.get(2)?,
                    })
                },
            )
            .optional()
            .map_err(sqlite)?;
        Ok(found.unwrap_or_default())
    }

    pub fn save_document(&self, document: &Document) -> Result<()> {
        if self.mirroring() {
            if let Some(node) = self.item_of_document(document.id)? {
                let beats = self.beats_of(node.id)?;
                let is_new = !self.mirror.paths.borrow().contains_key(&node.id);
                let path = self.mirror.path_for(node.id, "manuscript", &node.title);
                let text = scene_text(&node, &beats, &document.blocks, self.mirror.extras.borrow().get(&node.id));
                self.write_file(&path, &text)?;
                self.mirror
                    .heads
                    .borrow_mut()
                    .insert(node.id, head_fingerprint(&node, &beats));
                if is_new {
                    self.write_binder(&self.load_binder()?)?;
                }
            }
        }
        self.save_document_cached(document)
    }

    /// Permanently delete a text. Its file goes with it; this is what
    /// emptying the trash asks for.
    pub fn delete_document(&self, id: Id) -> Result<()> {
        if self.mirroring() {
            let path = self.mirror.paths.borrow_mut().remove(&id);
            if let Some(path) = path.filter(|path| path.starts_with("manuscript/")) {
                self.remove_file(&path)?;
            }
            self.mirror.heads.borrow_mut().remove(&id);
        }
        self.delete_document_cached(id)
    }

    pub fn persist_binder(&self, tree: &BinderTree) -> Result<()> {
        if self.mirroring() {
            self.write_changed_scenes(tree, &self.load_scene_beats()?)?;
            self.write_binder(tree)?;
        }
        self.persist_binder_cached(tree)
    }

    /// Save one binder item. The binder file lists every item, so it is
    /// written from the cache once the item is in it.
    pub fn upsert_node(&self, node: &BinderNode) -> Result<()> {
        self.upsert_node_cached(node)?;
        if self.mirroring() {
            let tree = self.load_binder()?;
            self.write_changed_scenes(&tree, &self.load_scene_beats()?)?;
            self.write_binder(&tree)?;
        }
        Ok(())
    }

    pub fn persist_scene_beats(&self, beats: &HashMap<Id, SceneBeats>) -> Result<()> {
        if self.mirroring() {
            self.write_changed_scenes(&self.load_binder()?, beats)?;
        }
        self.persist_scene_beats_cached(beats)
    }

    pub fn persist_meta(&self, meta: &ProjectMeta) -> Result<()> {
        if self.mirroring() {
            let (project, local) = settings_json(meta)?;
            self.write_small("project.json", &project)?;
            self.write_small(LOCAL_SETTINGS, &local)?;
        }
        self.persist_meta_cached(meta)
    }

    pub fn persist_plot(&self, grid: &PlotGrid) -> Result<()> {
        if self.mirroring() {
            self.write_small("plot.json", &plot_json(grid, &self.mirror.extras.borrow())?)?;
        }
        self.persist_plot_cached(grid)
    }

    pub fn persist_cast(&self, cast: &Cast) -> Result<()> {
        if self.mirroring() {
            self.write_bible(cast, &self.load_world()?, &self.load_library()?, &["characters/"])?;
        }
        self.persist_cast_cached(cast)
    }

    pub fn persist_world(&self, world: &World) -> Result<()> {
        if self.mirroring() {
            let folders = ["places/", "cultures/", "lore/", "timeline/"];
            self.write_bible(&self.load_cast()?, world, &self.load_library()?, &folders)?;
        }
        self.persist_world_cached(world)
    }

    pub fn persist_library(&self, library: &ResearchLibrary) -> Result<()> {
        if self.mirroring() {
            self.write_bible(&self.load_cast()?, &self.load_world()?, library, &["research/"])?;
        }
        self.persist_library_cached(library)
    }
}

impl Mirror {
    /// Take in what was just read from, or written to, the files.
    fn remember(&self, content: &ProjectContent, files: HashMap<Id, String>) {
        let none = SceneBeats::default();
        *self.heads.borrow_mut() = content
            .binder
            .nodes
            .iter()
            .filter(|node| node.document_id.is_some())
            .map(|node| (node.id, head_fingerprint(node, content.beats.get(&node.id).unwrap_or(&none))))
            .collect();
        *self.paths.borrow_mut() = files;
        *self.extras.borrow_mut() = content.extras.clone();
        self.written.borrow_mut().clear();
    }

    /// The file of an item, giving it one if it has none.
    fn path_for(&self, id: Id, folder: &str, title: &str) -> String {
        let mut paths = self.paths.borrow_mut();
        if let Some(path) = paths.get(&id) {
            return path.clone();
        }
        let mut used: HashSet<String> = paths.values().cloned().collect();
        let path = place(&paths, &mut used, folder, title, id);
        paths.insert(id, path.clone());
        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use writing_studio_project::NodeKind;

    fn temp(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("ws-mirror-{tag}-{}", Id::new()))
    }

    fn scene_of(store: &ProjectStore) -> BinderNode {
        store
            .load_binder()
            .unwrap()
            .nodes
            .into_iter()
            .find(|node| node.kind == NodeKind::Scene)
            .unwrap()
    }

    fn file_of(store: &ProjectStore, id: Id) -> PathBuf {
        store.root.join(&store.mirror.paths.borrow()[&id])
    }

    /// A project as the previous version of the program left it: one SQLite
    /// file, and text with ids of its own.
    fn format_1_project(tag: &str) -> (PathBuf, Id, Id) {
        let dir = temp(tag);
        fs::create_dir_all(dir.join("backups")).unwrap();
        fs::write(dir.join(PROJECT_FORMAT_FILE), "writing-studio-project 1\n").unwrap();
        fs::write(dir.join("backups").join("project-old.sqlite"), b"an earlier backup").unwrap();
        let store = ProjectStore::with_cache(dir.clone(), &dir.join(PROJECT_DB_FILE), true).unwrap();
        let seeded = blank_novel("Old Novel");
        store.persist_meta_cached(&seeded.meta).unwrap();
        let mut scene = None;
        for mut node in seeded.nodes {
            if node.document_id.is_some() {
                node.document_id = Some(Id::new());
            }
            if node.kind == NodeKind::Scene {
                scene = Some((node.id, node.document_id.unwrap()));
            }
            store.upsert_node_cached(&node).unwrap();
        }
        let (scene_id, document_id) = scene.unwrap();
        let document = Document::from_plain_text(document_id, "Written in the old format.");
        store.save_document_cached(&document).unwrap();
        store.snapshot_document(&document, "kept").unwrap();
        (dir, scene_id, document_id)
    }

    #[test]
    fn a_new_project_is_files_with_a_cache_beside_them() {
        let dir = temp("new");
        let store = ProjectStore::create(&dir, "New Novel").unwrap();
        assert_eq!(
            fs::read_to_string(dir.join(PROJECT_FORMAT_FILE)).unwrap(),
            "writing-studio-project 2\n"
        );
        for name in ["project.json", "binder.json", "bible.json", "plot.json", "README.md", ".gitignore"] {
            assert!(dir.join(name).exists(), "{name}");
        }
        assert!(dir.join(".local").join("cache.sqlite").exists());
        assert!(!dir.join(PROJECT_DB_FILE).exists());
        let scene = scene_of(&store);
        assert_eq!(scene.document_id, Some(scene.id));
        assert!(fs::read_to_string(file_of(&store, scene.id)).unwrap().contains("Start writing this scene."));
        assert!(ProjectStore::create(&dir, "Again").is_err());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn saving_writes_the_scene_file_and_only_the_files_that_change() {
        let dir = temp("save");
        let store = ProjectStore::create(&dir, "Saved Novel").unwrap();
        let scene = scene_of(&store);
        let path = file_of(&store, scene.id);
        let binder_before = fs::read_to_string(dir.join("binder.json")).unwrap();

        let mut document = store.load_document(scene.id).unwrap();
        document.replace_plain_text("She opened the door.");
        store.save_document(&document).unwrap();
        assert!(fs::read_to_string(&path).unwrap().contains("<p>She opened the door.</p>"));

        // A new title is written into the scene's file. The file keeps its
        // name, and the binder file, which holds no titles of scenes, is
        // left alone.
        let mut tree = store.load_binder().unwrap();
        tree.nodes.iter_mut().find(|node| node.id == scene.id).unwrap().title = "The Door".into();
        store.persist_binder(&tree).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("<title>The Door</title>") && text.contains("She opened the door."));
        assert_eq!(fs::read_to_string(dir.join("binder.json")).unwrap(), binder_before);

        // Beats go into the same file.
        let mut beats = HashMap::new();
        beats.insert(
            scene.id,
            SceneBeats {
                goal: "Get in".into(),
                ..SceneBeats::default()
            },
        );
        store.persist_scene_beats(&beats).unwrap();
        assert!(fs::read_to_string(&path).unwrap().contains("<meta name=\"goal\" content=\"Get in\">"));

        // A new scene gets a file and a line in the binder.
        let chapter = scene.parent_id;
        let added = BinderNode::new(NodeKind::Scene, "After", chapter, 5);
        store.upsert_node(&added).unwrap();
        store.save_document(&Document::from_plain_text(added.id, "Later.")).unwrap();
        assert!(fs::read_to_string(file_of(&store, added.id)).unwrap().contains("Later."));
        assert!(fs::read_to_string(dir.join("binder.json")).unwrap().contains(&added.id.to_string()));

        // Deleting it for good removes the file.
        let gone = file_of(&store, added.id);
        store.delete_document(added.id).unwrap();
        assert!(!gone.exists());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn opening_reads_the_files_even_when_the_cache_is_gone_or_stale() {
        let dir = temp("reopen");
        let store = ProjectStore::create(&dir, "Reopened").unwrap();
        let scene = scene_of(&store);
        let path = file_of(&store, scene.id);
        let mut document = store.load_document(scene.id).unwrap();
        document.replace_plain_text("First.");
        store.save_document(&document).unwrap();
        store.snapshot_document(&document, "kept").unwrap();
        drop(store);

        // The file is changed outside the program; the cache still says "First."
        let text = fs::read_to_string(&path).unwrap();
        fs::write(&path, text.replace("First.", "Changed in a text editor.")).unwrap();
        let store = ProjectStore::open(&dir).unwrap();
        assert_eq!(
            store.load_document(scene.id).unwrap().to_plain_text(),
            "Changed in a text editor."
        );
        assert_eq!(store.document_word_count(scene.id), 5);
        // What belongs to this machine is still there.
        assert_eq!(store.list_snapshots(scene.id).unwrap().len(), 1);
        drop(store);

        // Without `.local`, as on another machine, the project is all there.
        fs::remove_dir_all(dir.join(".local")).unwrap();
        let store = ProjectStore::open(&dir).unwrap();
        assert_eq!(
            store.load_document(scene.id).unwrap().to_plain_text(),
            "Changed in a text editor."
        );
        assert!(store.list_snapshots(scene.id).unwrap().is_empty());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_format_1_project_is_converted_and_keeps_its_snapshots_and_backups() {
        let (dir, scene_id, old_document_id) = format_1_project("convert");
        let store = ProjectStore::open(&dir).unwrap();

        assert_eq!(
            fs::read_to_string(dir.join(PROJECT_FORMAT_FILE)).unwrap(),
            "writing-studio-project 2\n"
        );
        assert!(!dir.join(PROJECT_DB_FILE).exists());
        assert!(!dir.join("backups").exists());
        // The text now has its item's id, and the snapshot followed it.
        let scene = scene_of(&store);
        assert_eq!((scene.id, scene.document_id), (scene_id, Some(scene_id)));
        assert_eq!(
            store.load_document(scene_id).unwrap().to_plain_text(),
            "Written in the old format."
        );
        assert!(store.load_document(old_document_id).is_err());
        assert_eq!(store.list_snapshots(scene_id).unwrap().len(), 1);
        // The earlier backup, and the one written before converting.
        let backups = store.list_backups().unwrap();
        assert_eq!(backups.len(), 2);
        assert!(fs::read_to_string(file_of(&store, scene_id))
            .unwrap()
            .contains("Written in the old format."));
        drop(store);
        // Opening again does not convert again.
        let store = ProjectStore::open(&dir).unwrap();
        assert_eq!(store.list_backups().unwrap().len(), 2);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_conversion_cut_short_after_the_mark_is_finished_on_the_next_open() {
        let (dir, scene_id, _) = format_1_project("cut-short");
        // Do everything a conversion does up to the mark, and stop there:
        // the database is still where format 1 keeps it.
        {
            let store = ProjectStore::with_cache(dir.clone(), &dir.join(PROJECT_DB_FILE), true).unwrap();
            store.rekey_documents().unwrap();
            let content = ProjectContent::from_store(&store).unwrap();
            write_project_files(&dir, &content).unwrap();
            store.conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);").unwrap();
        }
        write_format_2_marker(&dir).unwrap();
        assert!(dir.join(PROJECT_DB_FILE).exists());

        let store = ProjectStore::open(&dir).unwrap();
        assert!(!dir.join(PROJECT_DB_FILE).exists());
        assert_eq!(store.list_snapshots(scene_id).unwrap().len(), 1);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn opening_as_is_changes_nothing_in_a_format_1_project() {
        let (dir, scene_id, document_id) = format_1_project("as-is");
        let store = ProjectStore::open_as_is(&dir).unwrap();
        let scene = scene_of(&store);
        assert_eq!((scene.id, scene.document_id), (scene_id, Some(document_id)));
        let mut document = store.load_document(document_id).unwrap();
        document.replace_plain_text("Saved, but to the database only.");
        store.save_document(&document).unwrap();
        drop(store);
        assert_eq!(
            fs::read_to_string(dir.join(PROJECT_FORMAT_FILE)).unwrap(),
            "writing-studio-project 1\n"
        );
        assert!(dir.join(PROJECT_DB_FILE).exists());
        assert!(!dir.join("project.json").exists() && !dir.join("manuscript").exists());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_deleted_character_loses_its_file_and_other_folders_are_left_alone() {
        let dir = temp("bible");
        let store = ProjectStore::create(&dir, "Cast").unwrap();
        let mut cast = Cast::new();
        let mara = cast.add_character();
        cast.set_name(mara, "Mara Vance".into());
        store.persist_cast(&cast).unwrap();
        let path = file_of(&store, mara);
        assert!(fs::read_to_string(&path).unwrap().contains("<title>Mara Vance</title>"));
        let scene_file = file_of(&store, scene_of(&store).id);

        store.persist_cast(&Cast::new()).unwrap();
        assert!(!path.exists());
        assert!(scene_file.exists());
        assert!(!fs::read_to_string(dir.join("bible.json")).unwrap().contains(&mara.to_string()));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn entries_the_program_does_not_know_survive_every_kind_of_save() {
        let dir = temp("extras");
        let store = ProjectStore::create(&dir, "Kept").unwrap();
        let scene = scene_of(&store);
        let mut cast = Cast::new();
        let mara = cast.add_character();
        cast.set_name(mara, "Mara".into());
        store.persist_cast(&cast).unwrap();
        let mut grid = PlotGrid::new();
        let line = grid.add_line();
        store.persist_plot(&grid).unwrap();
        drop(store);

        // A person, or a later version of the program, adds to the files.
        let edit = |path: &Path, from: &str, to: &str| {
            let text = fs::read_to_string(path).unwrap();
            assert!(text.contains(from), "{from:?} is not in {}", path.display());
            fs::write(path, text.replacen(from, to, 1)).unwrap();
        };
        let reopened = ProjectStore::open(&dir).unwrap();
        let scene_file = file_of(&reopened, scene.id);
        let mara_file = file_of(&reopened, mara);
        drop(reopened);
        edit(&scene_file, "<link rel=", "<meta name=\"pov\" content=\"Mara\">\n<link rel=");
        edit(&mara_file, "<link rel=", "<meta name=\"eyes\" content=\"grey\">\n<link rel=");
        let id = format!("\"id\":\"{}\"", scene.id);
        edit(&dir.join("binder.json"), &id, &format!("{id},\"tag\":\"draft-2\""));
        let line_id = format!("\"id\": \"{line}\"");
        edit(&dir.join("plot.json"), &line_id, &format!("\"arc\": \"rising\",\n      {line_id}"));

        // Every save that touches those items keeps what was added.
        let store = ProjectStore::open(&dir).unwrap();
        let mut document = store.load_document(scene.id).unwrap();
        document.replace_plain_text("New text.");
        store.save_document(&document).unwrap();
        let mut tree = store.load_binder().unwrap();
        let node = tree.nodes.iter_mut().find(|node| node.id == scene.id).unwrap();
        node.title = "Renamed".into();
        node.parent_id = None;
        store.persist_binder(&tree).unwrap();
        let mut cast = store.load_cast().unwrap();
        cast.characters[0].summary = "Changed.".into();
        store.persist_cast(&cast).unwrap();
        let mut grid = store.load_plot().unwrap();
        grid.lines[0].title = "Escape".into();
        store.persist_plot(&grid).unwrap();

        let scene_text = fs::read_to_string(&scene_file).unwrap();
        assert!(scene_text.contains("<meta name=\"pov\" content=\"Mara\">"));
        assert!(scene_text.contains("<title>Renamed</title>") && scene_text.contains("New text."));
        let mara_text = fs::read_to_string(&mara_file).unwrap();
        assert!(mara_text.contains("<meta name=\"eyes\" content=\"grey\">") && mara_text.contains("Changed."));
        assert!(fs::read_to_string(dir.join("binder.json")).unwrap().contains("\"tag\":\"draft-2\""));
        let plot = fs::read_to_string(dir.join("plot.json")).unwrap();
        assert!(plot.contains("\"arc\": \"rising\"") && plot.contains("Escape"));

        // Restoring a backup writes every file again, and keeps them too.
        let mut store = store;
        let backup = store.backup().unwrap();
        store.restore_backup(&backup).unwrap();
        assert!(fs::read_to_string(&scene_file).unwrap().contains("name=\"pov\""));
        assert!(fs::read_to_string(dir.join("binder.json")).unwrap().contains("draft-2"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn restoring_a_backup_puts_the_files_back_too() {
        let dir = temp("restore");
        let mut store = ProjectStore::create(&dir, "Restored").unwrap();
        let scene = scene_of(&store);
        let mut document = store.load_document(scene.id).unwrap();
        document.replace_plain_text("Before the backup.");
        store.save_document(&document).unwrap();
        let backup = store.backup().unwrap();
        document.replace_plain_text("After the backup.");
        store.save_document(&document).unwrap();

        store.restore_backup(&backup).unwrap();
        assert!(fs::read_to_string(file_of(&store, scene.id)).unwrap().contains("Before the backup."));
        drop(store);
        let store = ProjectStore::open(&dir).unwrap();
        assert_eq!(store.load_document(scene.id).unwrap().to_plain_text(), "Before the backup.");
        let _ = fs::remove_dir_all(dir);
    }
}
