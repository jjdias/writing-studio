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
use writing_studio_project::{BinderNode, BinderTree, MindMap, ProjectMeta};
use writing_studio_research::ResearchLibrary;
use writing_studio_templates::blank_novel;
use writing_studio_worldbuilding::World;

use crate::files::{
    folder_of,
    has_own_file,
    read_scene_file,
    text_fingerprint, unknown_head, with_a_run_in_every_block, write_project_files_noted,
    bible_files, Extras, Problem, binder_json, inside, mindmap_json, place, plot_json, read_mindmap, read_project_files,
    scene_text, settings_json, MINDMAP_FILE,
    write_atomic, write_format_marker, ProjectContent,
    LOCAL_SETTINGS,
};
use crate::{open_connection, row_to_node, schema, sqlite, ProjectStore};

const LOCAL_DIR: &str = ".local";
/// The cache's copy of `mindmap.json`, in `binder_lists`.
const MINDMAP_LIST: &str = "mindmap";
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
    /// A fingerprint of what each file held when the program last read or
    /// wrote it. A file that holds something else was changed outside.
    on_disk: RefCell<HashMap<String, u64>>,
    /// Files that were changed outside the program and would have been
    /// written over, and where each was set aside, since the last time the
    /// window asked.
    set_aside: RefCell<Vec<SetAside>>,
    /// Items whose file could not be read when the project opened. Their
    /// file is never written over.
    missing: RefCell<HashSet<Id>>,
    /// What was wrong with the files when the project opened.
    problems: RefCell<Vec<Problem>>,
    /// Files are left alone: while the cache is being filled from them, and
    /// for a project opened only to be read.
    paused: Cell<bool>,
}

/// A file that was changed outside the program, kept under another name so
/// that a save could not write over it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SetAside {
    /// The file, inside the project folder.
    pub file: String,
    /// Where what it held was kept.
    pub kept_as: String,
}

fn fingerprint(parts: &[&str]) -> u64 {
    let mut hasher = DefaultHasher::new();
    parts.hash(&mut hasher);
    hasher.finish()
}

fn head_fingerprint(node: &BinderNode, beats: &SceneBeats) -> u64 {
    let keywords = node.keywords.join("\n");
    fingerprint(&[
        &keywords,
        &node.notes,
        if node.footnotes { "footnotes" } else { "" },
        &node.links.iter().map(Id::to_string).collect::<Vec<_>>().join(","),
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

/// The folders a kept file can be in: the project folder, and each folder
/// of Markdown files.
const KEPT_FOLDERS: [&str; 9] = ["", "manuscript", "notes", "characters", "places", "cultures", "lore", "timeline", "research"];

/// Whether `path` is a project folder, of this format or an earlier one.
fn is_project(path: &Path) -> bool {
    ["project.json", PROJECT_FORMAT_FILE, PROJECT_DB_FILE].iter().any(|marker| path.join(marker).is_file())
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

/// Make `.local/` and `attachments/`, after checking that neither is a link
/// out of the project: `.local` holds the cache and backups, and git does not
/// ignore a link of that name.
fn local_folders(root: &Path) -> Result<()> {
    for folder in [LOCAL_DIR, "attachments"] {
        fs::create_dir_all(inside(root, "the project", folder)?)?;
    }
    Ok(())
}

fn cache_path(root: &Path) -> PathBuf {
    root.join(LOCAL_DIR).join(CACHE_FILE)
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
        self.root.join(LOCAL_DIR).join("backups")
    }

    /// Where a new project goes when the writer chooses `folder`: `folder`
    /// itself when it is an empty `.writingstudio` folder, which the writer
    /// made for it, or else a new `folder_name` folder inside it. A folder
    /// that is a project, or is inside one, is refused, so a project never
    /// ends up inside another.
    pub fn new_project_root(folder: &Path, folder_name: &str) -> Result<PathBuf> {
        if let Some(project) = folder.ancestors().find(|path| is_project(path)) {
            let name = project.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
            return Err(Error::message(format!(
                "that folder is the project “{name}” or inside it; choose a folder outside any project"
            )));
        }
        let named_for_a_project = folder.extension().is_some_and(|extension| extension == "writingstudio");
        // The folder picker can return a folder that is gone, such as one
        // deleted while the picker still listed it. Treat it as empty.
        let empty = !folder.exists() || fs::read_dir(folder).is_ok_and(|mut entries| entries.next().is_none());
        if named_for_a_project && empty {
            return Ok(folder.to_path_buf());
        }
        Ok(folder.join(folder_name))
    }

    pub fn create(root: impl AsRef<Path>, title: &str) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root)?;
        if root.join("project.json").exists() || root.join(PROJECT_DB_FILE).exists() || cache_path(&root).exists() {
            return Err(Error::message("a project already exists in this folder"));
        }
        local_folders(&root)?;

        let cache = cache_path(&root);
        let store = Self::with_cache(root, &cache, true)?;
        let seeded = blank_novel(title);
        // The note templates, once, when the project is made: a template the
        // writer deletes stays deleted.
        let templates = inside(&store.root, "templates", crate::TEMPLATES_DIR)?;
        fs::create_dir_all(&templates)?;
        for (name, text) in writing_studio_templates::NOTE_TEMPLATES {
            write_atomic(&templates.join(name), text.as_bytes())?;
        }
        store.persist_meta_cached(&seeded.meta)?;
        for node in &seeded.nodes {
            store.upsert_node_cached(node)?;
        }
        for document in &seeded.documents {
            store.save_document_cached(document)?;
        }
        store.write_files_from_cache()?;
        write_format_marker(&store.root)?;
        store.mirror.paused.set(false);
        Ok(store)
    }

    /// Open a project to work in it. A folder made by an earlier version of
    /// the program is refused: this version does not convert projects.
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        let version = format_version(&root)?;
        if version < 2 || (root.join(PROJECT_DB_FILE).exists() && !root.join("project.json").exists()) {
            return Err(Error::message(
                "this project was made by an earlier test version of Writing Studio, which kept it in one database; \
                 this version keeps each scene as a file and cannot open it",
            ));
        }
        if version == 2 {
            return Err(Error::message(
                "this project was made by an earlier test version of Writing Studio, which kept each scene as an HTML file; \
                 this version keeps each scene as a Markdown file and cannot open it",
            ));
        }
        if version < PROJECT_FORMAT_VERSION {
            return Err(Error::message(
                "this project was made by an earlier test version of Writing Studio, which kept characters, places, and research apart; \
                 this version keeps them all as notes and cannot open it",
            ));
        }
        if !root.join("project.json").exists() {
            return Err(Error::ProjectNotFound(root));
        }
        local_folders(&root)?;
        let cache = cache_path(&root);
        let store = Self::with_cache(root, &cache, true)?;
        let content = read_project_files(&store.root)?;
        store.fill_cache(&content)?;
        store.mirror.remember(&content, content.files.clone(), content.read.clone());
        *store.mirror.problems.borrow_mut() = content.problems.clone();
        store.mirror.paused.set(false);
        if content.repaired {
            // Recovered and moved items are written into the files now, so the
            // files say what the window shows.
            store.write_files_from_cache()?;
        }
        Ok(store)
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
        self.persist_mindmap_cached(&content.mindmap)?;
        self.persist_library_cached(&content.library)
    }

    /// Write every file from what the cache holds. Used when a project is
    /// made, and after a backup is restored into the cache.
    pub(crate) fn write_files_from_cache(&self) -> Result<()> {
        let mut content = ProjectContent::from_store(self)?;
        content.files = self.mirror.paths.borrow().clone();
        content.extras = self.mirror.extras.borrow().clone();
        content.missing = self.mirror.missing.borrow().clone();
        let (files, written) = write_project_files_noted(&self.root, &content)?;
        self.mirror.remember(&content, files, written);
        // An empty map writes no file, so one left from before is emptied.
        if content.mindmap.is_empty() && self.root.join(MINDMAP_FILE).exists() {
            self.write_small(MINDMAP_FILE, &mindmap_json(&content.mindmap)?)?;
        }
        Ok(())
    }

    fn mirroring(&self) -> bool {
        !self.mirror.paused.get()
    }

    /// Write a file inside the project folder, unless it already holds this
    /// text. If the file was changed outside the program since the program
    /// last read or wrote it, what it holds is first kept under another name,
    /// so that nothing written elsewhere is lost.
    fn write_file(&self, path: &str, text: &str) -> Result<()> {
        let mark = text_fingerprint(text);
        let known = self.mirror.on_disk.borrow().get(path).copied();
        if known == Some(mark) {
            return Ok(());
        }
        let full = inside(&self.root, "the project", path)?;
        match fs::read_to_string(&full) {
            Ok(current) => {
                let current_mark = text_fingerprint(&current);
                if current_mark == mark {
                    self.mirror.on_disk.borrow_mut().insert(path.to_string(), mark);
                    return Ok(());
                }
                if known != Some(current_mark) {
                    self.set_aside(path, &current)?;
                }
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(err.into()),
        }
        write_atomic(&full, text.as_bytes())?;
        self.mirror.on_disk.borrow_mut().insert(path.to_string(), mark);
        Ok(())
    }

    fn write_small(&self, path: &str, text: &str) -> Result<()> {
        self.write_file(path, text)
    }

    /// Delete a file the writer deleted the contents of. A file that was
    /// changed outside since the program last saw it is kept under another
    /// name instead.
    fn remove_file(&self, path: &str) -> Result<()> {
        let full = inside(&self.root, "the project", path)?;
        let known = self.mirror.on_disk.borrow_mut().remove(path);
        match fs::read_to_string(&full) {
            Ok(current) if known != Some(text_fingerprint(&current)) => {
                self.set_aside(path, &current)?;
            }
            Ok(_) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(err) => return Err(err.into()),
        }
        match fs::remove_file(full) {
            Err(err) if err.kind() != std::io::ErrorKind::NotFound => Err(err.into()),
            _ => Ok(()),
        }
    }

    /// Keep `text`, which `path` held, beside it as
    /// `name.conflict-<time>.ext`.
    fn set_aside(&self, path: &str, text: &str) -> Result<()> {
        let stamp = writing_studio_core::Timestamp::now()
            .inner()
            .format("%Y-%m-%dT%H-%M-%S%.3fZ")
            .to_string();
        let (stem, extension) = match path.rsplit_once('.') {
            Some((stem, extension)) if !stem.ends_with('/') && !extension.contains('/') => {
                (stem, format!(".{extension}"))
            }
            _ => (path, String::new()),
        };
        let kept_as = format!("{stem}.conflict-{stamp}{extension}");
        write_atomic(&inside(&self.root, "the project", &kept_as)?, text.as_bytes())?;
        self.mirror.set_aside.borrow_mut().push(SetAside {
            file: path.to_string(),
            kept_as,
        });
        Ok(())
    }

    /// What was wrong with the files when the project opened, and what was
    /// done about it.
    pub fn problems(&self) -> Vec<Problem> {
        self.mirror.problems.borrow().clone()
    }

    /// The file this item is kept in, inside the project folder.
    pub fn file_of(&self, id: Id) -> Option<String> {
        self.mirror.paths.borrow().get(&id).cloned()
    }

    /// Whether this item's file could not be read when the project opened.
    pub fn is_missing(&self, id: Id) -> bool {
        self.mirror.missing.borrow().contains(&id)
    }

    /// The files that were changed outside the program and set aside since
    /// the last call.
    pub fn take_set_aside(&self) -> Vec<SetAside> {
        std::mem::take(&mut *self.mirror.set_aside.borrow_mut())
    }

    /// The files kept beside a project file because it had been changed
    /// outside: `name.conflict-<time>.ext`, newest first. `file` is the one
    /// each was kept beside.
    pub fn kept_files(&self) -> Result<Vec<SetAside>> {
        let mut found: Vec<(String, SetAside)> = Vec::new();
        for folder in KEPT_FOLDERS {
            let Ok(entries) = fs::read_dir(self.root.join(folder)) else {
                continue;
            };
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                let Some((stem, rest)) = name.split_once(".conflict-") else {
                    continue;
                };
                if !entry.file_type()?.is_file() {
                    continue;
                }
                // The time ends with `Z`; the extension, if any, follows it.
                let (stamp, extension) = rest.split_once('Z').unwrap_or((rest, ""));
                let prefix = if folder.is_empty() { String::new() } else { format!("{folder}/") };
                let kept = SetAside {
                    file: format!("{prefix}{stem}{extension}"),
                    kept_as: format!("{prefix}{name}"),
                };
                found.push((stamp.to_string(), kept));
            }
        }
        found.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.kept_as.cmp(&b.1.kept_as)));
        Ok(found.into_iter().map(|(_, kept)| kept).collect())
    }

    /// What a kept file holds.
    pub fn read_kept_file(&self, kept_as: &str) -> Result<String> {
        Ok(fs::read_to_string(self.kept_path(kept_as)?)?)
    }

    /// Delete a kept file, once the writer has taken what they need from it.
    pub fn remove_kept_file(&self, kept_as: &str) -> Result<()> {
        Ok(fs::remove_file(self.kept_path(kept_as)?)?)
    }

    fn kept_path(&self, kept_as: &str) -> Result<PathBuf> {
        if !kept_as.contains(".conflict-") {
            return Err(Error::message(format!("“{kept_as}” is not a kept file")));
        }
        inside(&self.root, "kept file", kept_as)
    }

    /// Whether a file other than a scene's (the lists, and the bible's
    /// entries) holds something other than what the program last read or
    /// wrote, or is gone. Reads each of them; a scene is checked on its own
    /// with `changed_outside`.
    pub fn others_changed_outside(&self) -> Result<bool> {
        let known: Vec<(String, u64)> = self
            .mirror
            .on_disk
            .borrow()
            .iter()
            .filter(|(path, _)| {
                !path.starts_with("manuscript/")
                    && !path.starts_with(".local/")
                    && (path.ends_with(".json") || path.ends_with(".md"))
            })
            .map(|(path, print)| (path.clone(), *print))
            .collect();
        for (path, print) in known {
            match fs::read_to_string(inside(&self.root, "the project", &path)?) {
                Ok(current) if text_fingerprint(&current) == print => {}
                Ok(_) => return Ok(true),
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(true),
                Err(err) => return Err(err.into()),
            }
        }
        Ok(false)
    }

    /// Whether the file of this item holds something other than what the
    /// program last read or wrote. Reads that one file.
    pub fn changed_outside(&self, id: Id) -> Result<bool> {
        let Some(path) = self.mirror.paths.borrow().get(&id).cloned() else {
            return Ok(false);
        };
        let known = self.mirror.on_disk.borrow().get(&path).copied();
        match fs::read_to_string(inside(&self.root, "the project", &path)?) {
            Ok(current) => Ok(known != Some(text_fingerprint(&current))),
            // A file that is gone is not read again; the next save writes it.
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(err) => Err(err.into()),
        }
    }

    /// Read the file of one scene again, after it was changed outside the
    /// program, and put what it holds in the cache. Returns the item and its
    /// beats and text as the file has them, or `None` if the item has no
    /// file.
    pub fn reread_scene(&self, id: Id) -> Result<Option<(BinderNode, SceneBeats, Document)>> {
        let Some(path) = self.mirror.paths.borrow().get(&id).cloned() else {
            return Ok(None);
        };
        let Some(mut node) = self.item_of_document(id)? else {
            return Ok(None);
        };
        let text = fs::read_to_string(inside(&self.root, "the project", &path)?)?;
        let file = read_scene_file(&text).map_err(|why| Error::message(format!("{path}: {why}")))?;
        if file.header.id.is_some_and(|found| found != id) {
            return Err(Error::message(format!(
                "{path}: the id in the file is not the one it is listed under"
            )));
        }
        node.title = file.header.title;
        node.synopsis = file.header.synopsis;
        node.status = file.header.status;
        node.label = file.header.label;
        node.color = file.header.color;
        let beats = SceneBeats {
            goal: file.header.goal,
            conflict: file.header.conflict,
            outcome: file.header.outcome,
        };
        let mut document = Document::new(id);
        document.blocks = with_a_run_in_every_block(file.blocks);

        self.upsert_node_cached(&node)?;
        self.save_document_cached(&document)?;
        let mut all_beats = self.load_scene_beats()?;
        all_beats.insert(id, beats.clone());
        self.persist_scene_beats_cached(&all_beats)?;

        let extras_head = unknown_head(&file.header.other, &[]);
        let mut extras = self.mirror.extras.borrow_mut();
        let entry = extras.entry(id).or_default();
        entry.head = extras_head;
        drop(extras);
        self.mirror.heads.borrow_mut().insert(id, head_fingerprint(&node, &beats));
        self.mirror.on_disk.borrow_mut().insert(path, text_fingerprint(&text));
        Ok(Some((node, beats, document)))
    }

    /// Write the scene file of every item whose title, synopsis, status,
    /// label, color, or beats changed, or that has no file yet. An item whose
    /// head is unchanged is not read or written.
    fn write_changed_scenes(&self, tree: &BinderTree, beats: &HashMap<Id, SceneBeats>) -> Result<()> {
        let none = SceneBeats::default();
        for node in &tree.nodes {
            if !has_own_file(node) || self.is_missing(node.id) {
                continue;
            }
            let scene_beats = beats.get(&node.id).unwrap_or(&none);
            let mark = head_fingerprint(node, scene_beats);
            let has_file = self.mirror.paths.borrow().contains_key(&node.id);
            if has_file && self.mirror.heads.borrow().get(&node.id) == Some(&mark) {
                continue;
            }
            let blocks = node
                .document_id
                .and_then(|document_id| self.load_document(document_id).ok())
                .map(|document| document.blocks)
                .unwrap_or_default();
            let path = self.mirror.path_for(node.id, folder_of(tree, node.id), &node.title);
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
        for (id, path, file) in &entries {
            self.write_small(path, &file.to_markdown())?;
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

        }
        Ok(())
    }

    /// The binder item whose text has this id.
    fn item_of_document(&self, document_id: Id) -> Result<Option<BinderNode>> {
        self.conn
            .query_row(
                "SELECT id, parent_id, kind, title, sort_index, document_id, synopsis, label, status, color,
                        deleted_at, created_at, updated_at, restore_parent_id, keywords, left_out
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
        if self.is_missing(document.id) {
            let path = self.mirror.paths.borrow().get(&document.id).cloned().unwrap_or_default();
            return Err(Error::message(format!(
                "{path} could not be read when the project opened, so it is not written over; fix or restore the file, then open the project again"
            )));
        }
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
        self.mirror.missing.borrow_mut().remove(&id);
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
            let (project, local) = settings_json(meta, self.mirror.extras.borrow().get(&meta.id))?;
            self.write_small("project.json", &project)?;
            self.write_small(LOCAL_SETTINGS, &local)?;
        }
        self.persist_meta_cached(meta)
    }

    /// The mind map's ideas, lines, and positions. The cache keeps a copy
    /// as one JSON value, so a backup holds it and a restore brings it back;
    /// a cache without one reads `mindmap.json`.
    pub fn load_mindmap(&self) -> Result<MindMap> {
        match self.load_binder_list(MINDMAP_LIST)? {
            Some(json) => serde_json::from_str(&json).map_err(|err| Error::message(format!("{MINDMAP_FILE}: {err}"))),
            None => read_mindmap(&self.root, &RefCell::new(HashMap::new())),
        }
    }

    /// Write `mindmap.json`, and the cache's copy. A project that never had
    /// a map does not get an empty file.
    pub fn persist_mindmap(&self, map: &MindMap) -> Result<()> {
        if self.mirroring() && (!map.is_empty() || self.root.join(MINDMAP_FILE).exists()) {
            self.write_small(MINDMAP_FILE, &mindmap_json(map)?)?;
        }
        self.persist_mindmap_cached(map)
    }

    fn persist_mindmap_cached(&self, map: &MindMap) -> Result<()> {
        let json = serde_json::to_string(map).map_err(|err| Error::message(err.to_string()))?;
        self.store_binder_list(MINDMAP_LIST, json)
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
    fn remember(&self, content: &ProjectContent, files: HashMap<Id, String>, on_disk: HashMap<String, u64>) {
        let none = SceneBeats::default();
        *self.heads.borrow_mut() = content
            .binder
            .nodes
            .iter()
            .filter(|node| has_own_file(node))
            .map(|node| (node.id, head_fingerprint(node, content.beats.get(&node.id).unwrap_or(&none))))
            .collect();
        *self.paths.borrow_mut() = files;
        *self.extras.borrow_mut() = content.extras.clone();
        *self.missing.borrow_mut() = content.missing.clone();
        *self.on_disk.borrow_mut() = on_disk;
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

    #[test]
    fn a_book_has_a_file_of_its_details_and_a_move_changes_only_the_binder() {
        let dir = temp("levels");
        let store = ProjectStore::create(&dir, "Levels").unwrap();
        let mut tree = store.load_binder().unwrap();
        let book = tree.nodes.iter().find(|node| node.kind == NodeKind::Book).unwrap().id;
        let node = tree.nodes.iter_mut().find(|node| node.id == book).unwrap();
        node.synopsis = "A tide, a pilot, a promise.".into();
        node.keywords = vec!["act/one".into()];
        store.persist_binder(&tree).unwrap();
        let book_file = file_of(&store, book);
        let text = fs::read_to_string(&book_file).unwrap();
        assert!(text.contains("\nsynopsis: A tide, a pilot, a promise.\n") && text.contains("tags:\n  - act/one\n"), "{text}");
        let binder = fs::read_to_string(dir.join("binder.json")).unwrap();
        assert!(!binder.contains("A tide") && !binder.contains("act/one"), "{binder}");

        // Text a person writes under a book's front matter is kept.
        fs::write(&book_file, format!("{text}\nWhat this book is for.\n")).unwrap();
        drop(store);
        let store = ProjectStore::open(&dir).unwrap();
        let mut tree = store.load_binder().unwrap();
        tree.nodes.iter_mut().find(|node| node.id == book).unwrap().synopsis = "Changed.".into();
        store.persist_binder(&tree).unwrap();
        let text = fs::read_to_string(&book_file).unwrap();
        assert!(text.contains("synopsis: Changed.") && text.contains("What this book is for."), "{text}");

        // Moving a scene writes the binder and no item's file.
        let scene = scene_of(&store);
        let before: Vec<(PathBuf, String)> = fs::read_dir(dir.join("manuscript"))
            .unwrap()
            .flatten()
            .map(|entry| (entry.path(), fs::read_to_string(entry.path()).unwrap()))
            .collect();
        let mut tree = store.load_binder().unwrap();
        let folder = tree.nodes.iter().find(|node| node.kind == NodeKind::Chapter).map(|node| node.id);
        let node = tree.nodes.iter_mut().find(|node| node.id == scene.id).unwrap();
        node.sort_index += 10;
        node.parent_id = folder.or(node.parent_id);
        store.persist_binder(&tree).unwrap();
        for (path, text) in before {
            assert_eq!(fs::read_to_string(&path).unwrap(), text, "{} was written", path.display());
        }
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn notes_on_any_level_are_kept_in_its_file() {
        let dir = temp("notes");
        let store = ProjectStore::create(&dir, "Noted").unwrap();
        let scene = scene_of(&store).id;
        let mut tree = store.load_binder().unwrap();
        let book = tree.nodes.iter().find(|node| node.kind == NodeKind::Book).unwrap().id;
        for (id, notes) in [(scene, "Mara should not be trusted yet."), (book, "Ends at sea.\nNo epilogue.")] {
            tree.nodes.iter_mut().find(|node| node.id == id).unwrap().notes = notes.into();
        }
        store.persist_binder(&tree).unwrap();
        assert!(fs::read_to_string(file_of(&store, scene)).unwrap().contains("\nnotes: Mara should not be trusted yet.\n"));
        assert!(fs::read_to_string(file_of(&store, book)).unwrap().contains("notes: |-\n  Ends at sea.\n  No epilogue.\n"));
        drop(store);
        let store = ProjectStore::open(&dir).unwrap();
        let tree = store.load_binder().unwrap();
        assert_eq!(tree.get(scene).unwrap().notes, "Mara should not be trusted yet.");
        assert_eq!(tree.get(book).unwrap().notes, "Ends at sea.\nNo epilogue.");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_new_project_is_files_with_a_cache_beside_them() {
        let dir = temp("new");
        let store = ProjectStore::create(&dir, "New Novel").unwrap();
        assert_eq!(
            fs::read_to_string(dir.join(PROJECT_FORMAT_FILE)).unwrap(),
            "writing-studio-project 4\n"
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

    /// An entry inside another of its kind keeps its place through the
    /// cache, `bible.json`, and a reopening.
    #[test]
    fn an_entry_inside_another_keeps_its_parent() {
        let dir = temp("parents");
        let store = ProjectStore::create(&dir, "Nested").unwrap();
        let mut world = store.load_world().unwrap();
        let country = world.add_location();
        let city = world.add_location();
        world.locations.iter_mut().find(|place| place.id == city).unwrap().parent_id = Some(country);
        store.persist_world(&world).unwrap();
        assert_eq!(store.load_world().unwrap().locations.iter().find(|place| place.id == city).unwrap().parent_id, Some(country));
        let bible = fs::read_to_string(dir.join("bible.json")).unwrap();
        assert!(bible.contains(&format!("\"parent\":\"{country}\"")), "{bible}");
        drop(store);
        let reopened = ProjectStore::open(&dir).unwrap();
        let read = reopened.load_world().unwrap();
        assert_eq!(read.locations.iter().find(|place| place.id == city).unwrap().parent_id, Some(country));
        assert_eq!(read.locations.iter().find(|place| place.id == country).unwrap().parent_id, None);
        let _ = fs::remove_dir_all(dir);
    }

    /// `mindmap.json` comes only once the map has something of its own,
    /// reads back, keeps keys the program does not know, and is written
    /// with the rest of the files.
    #[test]
    fn the_mind_map_file_reads_back_and_keeps_unknown_keys() {
        let dir = temp("mindmap");
        let store = ProjectStore::create(&dir, "Map").unwrap();
        assert!(!dir.join(MINDMAP_FILE).exists(), "a project without ideas has no file");
        assert!(store.load_mindmap().unwrap().is_empty());
        store.persist_mindmap(&MindMap::default()).unwrap();
        assert!(!dir.join(MINDMAP_FILE).exists());

        let scene = scene_of(&store).id;
        let mut map = MindMap::default();
        let idea = map.add_idea(Some(scene));
        map.add_line(idea, scene).unwrap();
        map.place(scene, 12.5, -40.0);
        store.persist_mindmap(&map).unwrap();
        assert_eq!(store.load_mindmap().unwrap(), map);

        let text = fs::read_to_string(dir.join(MINDMAP_FILE)).unwrap();
        let edited = text.replace("\"title\":\"New idea\"", "\"title\":\"New idea\",\"mood\":\"dark\"");
        fs::write(dir.join(MINDMAP_FILE), format!("{}  ,\"made_by\": \"hand\"\n}}\n", edited.trim_end().trim_end_matches('}')))
            .unwrap();
        let read = read_project_files(&dir).unwrap();
        assert_eq!(read.mindmap.ideas[0].other["mood"], "dark");
        assert_eq!(read.mindmap.other["made_by"], "hand");
        drop(store);
        let mut reopened = ProjectStore::open(&dir).unwrap();
        reopened.write_files_from_cache().unwrap();
        let written = fs::read_to_string(dir.join(MINDMAP_FILE)).unwrap();
        assert!(written.contains("\"mood\":\"dark\"") && written.contains("\"made_by\": \"hand\""), "{written}");

        // A backup holds the map: emptied since, it comes back on restore.
        let backup = reopened.backup().unwrap();
        reopened.persist_mindmap(&MindMap::default()).unwrap();
        assert!(reopened.load_mindmap().unwrap().is_empty());
        reopened.restore_backup(&backup).unwrap();
        assert_eq!(reopened.load_mindmap().unwrap().ideas.len(), 1);
        assert!(fs::read_to_string(dir.join(MINDMAP_FILE)).unwrap().contains("New idea"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_new_project_never_goes_inside_another_and_uses_an_empty_folder_made_for_it() {
        let parent = temp("new-root");
        fs::create_dir_all(&parent).unwrap();
        // A plain folder gets a project folder inside it.
        assert_eq!(ProjectStore::new_project_root(&parent, "Tide.writingstudio").unwrap(), parent.join("Tide.writingstudio"));
        // An empty folder named for a project is used as it is.
        let made = parent.join("Tide.writingstudio");
        fs::create_dir_all(&made).unwrap();
        assert_eq!(ProjectStore::new_project_root(&made, "Tide.writingstudio").unwrap(), made);
        // So is one that is gone, as when the picker still lists a deleted folder.
        let gone = parent.join("Gone.writingstudio");
        assert_eq!(ProjectStore::new_project_root(&gone, "Gone.writingstudio").unwrap(), gone);
        // Once it holds a project, neither it nor a folder inside it can hold another.
        drop(ProjectStore::create(&made, "Tide").unwrap());
        assert!(ProjectStore::new_project_root(&made, "Tide.writingstudio").is_err());
        let inside = made.join("manuscript");
        let refused = ProjectStore::new_project_root(&inside, "Other.writingstudio").unwrap_err().to_string();
        assert!(refused.contains("Tide.writingstudio"), "{refused}");
        // A folder named for a project that holds other things gets one inside.
        let busy = parent.join("Busy.writingstudio");
        fs::create_dir_all(&busy).unwrap();
        fs::write(busy.join("notes.txt"), "x").unwrap();
        assert_eq!(ProjectStore::new_project_root(&busy, "Busy.writingstudio").unwrap(), busy.join("Busy.writingstudio"));
        let _ = fs::remove_dir_all(parent);
    }

    #[test]
    fn keywords_and_filters_outlast_other_saves_and_closing_the_project() {
        let dir = temp("keywords");
        let store = ProjectStore::create(&dir, "Tagged Novel").unwrap();
        let scene = scene_of(&store);
        let mut tree = store.load_binder().unwrap();
        tree.add_keyword(scene.id, "Flashback");
        tree.get_mut(scene.id).unwrap().left_out = true;
        tree.filters.push(writing_studio_project::SavedFilter {
            id: Id::new(),
            name: "Flashbacks".into(),
            keywords: vec!["Flashback".into()],
            any: false,
            without: Vec::new(),
            entries: Vec::new(),
            status: String::new(),
            text: String::new(),
        });
        store.persist_binder(&tree).unwrap();

        // Saves that write the binder file from the cache keep them: a new
        // item, the scene's text, and its beats.
        let chapter = tree.nodes.iter().find(|node| node.kind == NodeKind::Chapter).unwrap().id;
        store
            .upsert_node(&BinderNode::new(NodeKind::Scene, "Another", Some(chapter), 9))
            .unwrap();
        let mut document = store.load_document(scene.id).unwrap();
        document.replace_plain_text("The river ran uphill.");
        store.save_document(&document).unwrap();
        store.persist_scene_beats(&HashMap::new()).unwrap();
        let cached = store.load_binder().unwrap();
        assert_eq!(cached.get(scene.id).unwrap().keywords, ["Flashback"]);
        assert!(cached.get(scene.id).unwrap().left_out);
        assert!(fs::read_to_string(dir.join("binder.json")).unwrap().contains(r#""left_out":true"#));
        assert_eq!(cached.filters.len(), 1);
        assert!(fs::read_to_string(dir.join("binder.json")).unwrap().contains("Flashback"));
        drop(store);

        // Opened again, from the files and the cache.
        let reopened = ProjectStore::open(&dir).unwrap();
        let read = reopened.load_binder().unwrap();
        assert_eq!(read.get(scene.id).unwrap().keywords, ["Flashback"]);
        assert!(read.get(scene.id).unwrap().left_out, "left out of compile outlasts closing");
        assert!(!read.get(chapter).unwrap().left_out);
        assert_eq!(read.keyword_color("flashback"), tree.keyword_color("Flashback"));
        assert_eq!(read.filters, tree.filters);
        drop(reopened);
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
        assert!(fs::read_to_string(&path).unwrap().contains("\n\nShe opened the door.\n"));

        // A new title is written into the scene's file. The file keeps its
        // name, and the binder file, which holds no titles of scenes, is
        // left alone.
        let mut tree = store.load_binder().unwrap();
        tree.nodes.iter_mut().find(|node| node.id == scene.id).unwrap().title = "The Door".into();
        store.persist_binder(&tree).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("\ntitle: The Door\n") && text.contains("She opened the door."));
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
        assert!(fs::read_to_string(&path).unwrap().contains("\ngoal: Get in\n"));

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
    fn a_folder_from_an_earlier_version_is_refused_and_left_alone() {
        let dir = temp("earlier");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(PROJECT_FORMAT_FILE), "writing-studio-project 1\n").unwrap();
        fs::write(dir.join(PROJECT_DB_FILE), b"the old database").unwrap();
        let error = ProjectStore::open(&dir).err().expect("refused").to_string();
        assert!(error.contains("earlier test version"), "{error}");
        // Nothing was written: no cache, no files.
        let names: Vec<_> = fs::read_dir(&dir).unwrap().flatten().map(|entry| entry.file_name()).collect();
        assert_eq!(names.len(), 2);
        // Making a new project there is refused too, so the database stays.
        assert!(ProjectStore::create(&dir, "Over it").is_err());
        assert_eq!(fs::read(dir.join(PROJECT_DB_FILE)).unwrap(), b"the old database");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn notes_are_files_in_notes_and_a_new_project_has_two_folders() {
        let dir = temp("notes-folder");
        let store = ProjectStore::create(&dir, "Noted").unwrap();
        let tree = store.load_binder().unwrap();
        let root = tree.notes_root().unwrap().id;
        let folders: Vec<(String, bool)> = tree.children(Some(root)).iter().map(|node| (node.title.clone(), node.footnotes)).collect();
        assert_eq!(folders, [("Footnotes".to_string(), true), ("Research".to_string(), false)]);
        let footnotes = tree.children(Some(root))[0].id;
        let folder_file = file_of(&store, footnotes);
        assert!(folder_file.starts_with(dir.join("notes")), "{}", folder_file.display());
        let folder_text = fs::read_to_string(&folder_file).unwrap();
        assert!(folder_text.contains("\nfootnotes: \"true\"\n"), "{folder_text}");
        assert!(folder_text.ends_with("---\n"), "a folder has no text: {folder_text}");

        // A note's file is in notes/ too.
        let mut tree = tree;
        let note = BinderNode::new(NodeKind::Note, "Smith 2019", Some(footnotes), 0);
        let note_id = note.id;
        tree.nodes.push(note);
        store.persist_binder(&tree).unwrap();
        assert!(file_of(&store, note_id).starts_with(dir.join("notes")));
        drop(store);

        // A note added by hand comes into Notes.
        fs::write(dir.join("notes/by-hand.md"), scene_file(None, "Found", "A source.")).unwrap();
        let store = ProjectStore::open(&dir).unwrap();
        let tree = store.load_binder().unwrap();
        let found = tree.nodes.iter().find(|node| node.title == "Found").unwrap();
        assert_eq!((found.kind, found.parent_id), (NodeKind::Note, Some(root)));
        drop(store);

        // A format 3 project is refused, and left alone.
        fs::write(dir.join(PROJECT_FORMAT_FILE), "writing-studio-project 3\n").unwrap();
        let error = ProjectStore::open(&dir).err().expect("refused").to_string();
        assert!(error.contains("as notes") && error.contains("cannot open it"), "{error}");
        assert_eq!(fs::read_to_string(dir.join(PROJECT_FORMAT_FILE)).unwrap(), "writing-studio-project 3\n");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_project_with_html_scene_files_is_refused_and_left_alone() {
        let dir = temp("format-2");
        let store = ProjectStore::create(&dir, "Older").unwrap();
        drop(store);
        fs::write(dir.join(PROJECT_FORMAT_FILE), "writing-studio-project 2\n").unwrap();
        let before = fs::read_to_string(dir.join("binder.json")).unwrap();
        let error = ProjectStore::open(&dir).err().expect("refused").to_string();
        assert!(error.contains("HTML file") && error.contains("cannot open it"), "{error}");
        assert_eq!(fs::read_to_string(dir.join("binder.json")).unwrap(), before);
        assert_eq!(fs::read_to_string(dir.join(PROJECT_FORMAT_FILE)).unwrap(), "writing-studio-project 2\n");
        let _ = fs::remove_dir_all(dir);
    }

    /// The text of a scene file for `id`, as a person might write one.
    /// A scene file as a person or another program might write it.
    fn scene_file(id: Option<Id>, title: &str, text: &str) -> String {
        let id = id.map(|id| format!("id: {id}\n")).unwrap_or_default();
        format!("---\n{id}title: {title}\n---\n\n{text}\n")
    }

    fn titles_in(store: &ProjectStore, folder: Id) -> Vec<String> {
        let tree = store.load_binder().unwrap();
        tree.children(Some(folder)).iter().map(|node| node.title.clone()).collect()
    }

    fn recovered(store: &ProjectStore) -> Id {
        let tree = store.load_binder().unwrap();
        tree.nodes.iter().find(|node| node.title == "Recovered").expect("a Recovered folder").id
    }

    #[test]
    fn a_missing_scene_file_is_marked_and_never_written_over() {
        let dir = temp("missing");
        let store = ProjectStore::create(&dir, "Gaps").unwrap();
        let scene = scene_of(&store);
        let path = file_of(&store, scene.id);
        let note = store.load_binder().unwrap().nodes.into_iter().find(|node| node.kind == NodeKind::Chapter).unwrap();
        let other = file_of(&store, note.id);
        drop(store);

        fs::remove_file(&path).unwrap();
        // A file that holds another item's id is not this item's either.
        fs::write(&other, scene_file(Some(Id::new()), "Someone else", "Not mine.")).unwrap();
        let store = ProjectStore::open(&dir).unwrap();
        assert!(store.is_missing(scene.id) && store.is_missing(note.id));
        assert_eq!(store.problems().len(), 2, "{:?}", store.problems());
        assert_eq!(scene_of(&store).title, path.file_name().unwrap().to_string_lossy());
        let error = store.save_document(&Document::from_plain_text(scene.id, "x")).unwrap_err();
        assert!(error.to_string().contains("not written over"), "{error}");
        assert!(store.load_or_create_document(scene.id).is_err());
        assert!(!path.exists());
        assert!(fs::read_to_string(&other).unwrap().contains("Not mine."));
        // The binder still lists both, so restoring the files brings them back.
        let binder = fs::read_to_string(dir.join("binder.json")).unwrap();
        assert!(binder.contains(&scene.id.to_string()) && binder.contains(&note.id.to_string()));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn files_added_by_hand_come_into_recovered() {
        let dir = temp("hand");
        let store = ProjectStore::create(&dir, "Found").unwrap();
        let scene = scene_of(&store);
        drop(store);
        let fresh = Id::new();
        fs::write(dir.join("manuscript/a-new.md"), scene_file(Some(fresh), "With an id", "One.")).unwrap();
        fs::write(dir.join("manuscript/b-plain.md"), "Two.\n").unwrap();
        fs::write(dir.join("manuscript/c-copy.md"), scene_file(Some(scene.id), "A copy", "Three.")).unwrap();
        fs::write(dir.join("manuscript/x.conflict-2026.md"), scene_file(None, "Set aside", "Skip.")).unwrap();
        fs::create_dir_all(dir.join("characters")).unwrap();
        fs::write(dir.join("characters/rose.md"), scene_file(None, "Rose", "A pilot.")).unwrap();

        let store = ProjectStore::open(&dir).unwrap();
        assert_eq!(store.problems().len(), 4, "{:?}", store.problems());
        assert_eq!(titles_in(&store, recovered(&store)), ["With an id", "b-plain", "A copy"]);
        let tree = store.load_binder().unwrap();
        let copy = tree.nodes.iter().find(|node| node.title == "A copy").unwrap();
        assert_ne!(copy.id, scene.id);
        assert!(tree.get(fresh).is_some());
        assert_eq!(store.load_document(copy.id).unwrap().to_plain_text(), "Three.");
        assert_eq!(store.load_cast().unwrap().characters[0].name, "Rose");
        // The copy's file now carries its own id, and the binder lists all three.
        assert!(fs::read_to_string(dir.join("manuscript/c-copy.md")).unwrap().contains(&copy.id.to_string()));
        assert!(!fs::read_to_string(dir.join("binder.json")).unwrap().contains("conflict"));
        drop(store);
        let again = ProjectStore::open(&dir).unwrap();
        assert!(again.problems().is_empty(), "{:?}", again.problems());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_binder_changed_by_hand_is_made_a_tree_again() {
        let dir = temp("tree");
        let store = ProjectStore::create(&dir, "Tree").unwrap();
        let scene = scene_of(&store);
        let chapter = scene.parent_id.unwrap();
        drop(store);
        let path = dir.join("binder.json");
        let text = fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        let scene_line = lines.iter().find(|line| line.contains(&format!("\"id\":\"{}\"", scene.id))).unwrap();
        let chapter_line = lines.iter().find(|line| line.contains(&format!("\"id\":\"{chapter}\""))).unwrap();
        // The scene twice, the chapter its own parent's child in a loop, and
        // an item whose parent is nowhere.
        let ghost = Id::new();
        let looped = chapter_line.replace(r#""parent":""#, &format!(r#""parent":"{}","old":""#, scene.id));
        let stray = format!(r#"    {{"id":"{}","kind":"folder","parent":"{ghost}","status":"draft","title":"Stray"}},"#, Id::new());
        let edited = text
            .replace(chapter_line, &looped)
            .replace(scene_line, &format!("{scene_line}\n{scene_line}\n{stray}"));
        fs::write(&path, edited).unwrap();

        let store = ProjectStore::open(&dir).unwrap();
        let problems: Vec<String> = store.problems().iter().map(ToString::to_string).collect();
        assert!(problems.iter().any(|p| p.contains("listed twice")), "{problems:?}");
        // One item of the loop is moved, which is enough; the scene stays in
        // its chapter.
        let mut moved = titles_in(&store, recovered(&store));
        moved.sort();
        assert_eq!(moved, ["Chapter 1", "Stray"]);
        assert_eq!(store.load_binder().unwrap().get(scene.id).unwrap().parent_id, Some(chapter));
        assert_eq!(store.load_document(scene.id).unwrap().to_plain_text(), "Start writing this scene.\nEvery scene is saved independently.");
        drop(store);
        assert!(ProjectStore::open(&dir).unwrap().problems().is_empty());

        // Without its roots it is not a project.
        let mut binder: serde_json::Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        binder["items"].as_array_mut().unwrap().retain(|item| item["kind"] != "trash_root");
        fs::write(&path, binder.to_string()).unwrap();
        let error = ProjectStore::open(&dir).err().expect("refused").to_string();
        assert!(error.contains("trash_root"), "{error}");
        let _ = fs::remove_dir_all(dir);
    }

    /// Every file under a folder, to show nothing was written there.
    #[cfg(unix)]
    fn listing(dir: &Path) -> Vec<PathBuf> {
        let mut out = Vec::new();
        let mut stack = vec![dir.to_path_buf()];
        while let Some(folder) = stack.pop() {
            for entry in fs::read_dir(&folder).into_iter().flatten().flatten() {
                if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                    stack.push(entry.path());
                }
                out.push(entry.path());
            }
        }
        out.sort();
        out
    }

    #[cfg(unix)]
    #[test]
    fn links_that_lead_out_of_the_project_are_refused() {
        use std::os::unix::fs::symlink;
        let outside = temp("outside");
        fs::create_dir_all(&outside).unwrap();
        let refused = |dir: &Path| {
            let before = listing(&outside);
            let error = ProjectStore::open(dir).err().expect("refused").to_string();
            assert!(error.contains("through a link"), "{error}");
            assert_eq!(listing(&outside), before, "something was written outside");
        };

        // The manuscript folder is a link to a folder outside.
        let dir = temp("linked-folder");
        drop(ProjectStore::create(&dir, "Linked").unwrap());
        fs::rename(dir.join("manuscript"), outside.join("manuscript")).unwrap();
        symlink(outside.join("manuscript"), dir.join("manuscript")).unwrap();
        refused(&dir);

        // A scene file is a link to a file outside, or to nothing.
        let dir = temp("linked-file");
        let store = ProjectStore::create(&dir, "Linked").unwrap();
        let file = file_of(&store, scene_of(&store).id);
        drop(store);
        fs::rename(&file, outside.join("secret.md")).unwrap();
        symlink(outside.join("secret.md"), &file).unwrap();
        refused(&dir);
        fs::remove_file(&file).unwrap();
        symlink(outside.join("nothing.md"), &file).unwrap();
        refused(&dir);

        // `.local`, where the cache is written, is a link outside.
        let dir = temp("linked-local");
        drop(ProjectStore::create(&dir, "Linked").unwrap());
        fs::remove_dir_all(dir.join(".local")).unwrap();
        fs::create_dir_all(outside.join("local")).unwrap();
        symlink(outside.join("local"), dir.join(".local")).unwrap();
        refused(&dir);
        assert!(!outside.join("local/cache.sqlite").exists());

        // A link that stays inside the project is fine.
        let dir = temp("linked-inside");
        let store = ProjectStore::create(&dir, "Inside").unwrap();
        let scene = scene_of(&store);
        let file = file_of(&store, scene.id);
        drop(store);
        fs::rename(&file, dir.join("kept.md")).unwrap();
        symlink(dir.join("kept.md"), &file).unwrap();
        assert!(ProjectStore::open(&dir).unwrap().load_document(scene.id).is_ok());
        let _ = fs::remove_dir_all(outside);
    }

    #[test]
    fn a_deleted_character_file_takes_the_character_out() {
        let dir = temp("cast-gone");
        let store = ProjectStore::create(&dir, "Cast").unwrap();
        let mut cast = Cast::new();
        let mara = cast.add_character();
        cast.set_name(mara, "Mara".into());
        store.persist_cast(&cast).unwrap();
        let path = file_of(&store, mara);
        drop(store);
        fs::remove_file(path).unwrap();
        let store = ProjectStore::open(&dir).unwrap();
        assert!(store.load_cast().unwrap().characters.is_empty());
        assert_eq!(store.problems().len(), 1);
        assert!(!fs::read_to_string(dir.join("bible.json")).unwrap().contains(&mara.to_string()));
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
        assert!(fs::read_to_string(&path).unwrap().contains("\ntitle: Mara Vance\n"));
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
        let tom = cast.add_character();
        let bond = cast.connect(mara, tom, String::new()).unwrap();
        cast.link_scene(mara, scene.id);
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
        // A key of its own at the end of the front matter.
        edit(&scene_file, "\n---\n", "\npov: Mara\n---\n");
        edit(&mara_file, "\n---\n", "\neyes: grey\n---\n");
        let id = format!("\"id\":\"{}\"", scene.id);
        edit(&dir.join("binder.json"), &id, &format!("{id},\"tag\":\"draft-2\""));
        let line_id = format!("\"id\": \"{line}\"");
        edit(&dir.join("plot.json"), &line_id, &format!("\"arc\": \"rising\",\n      {line_id}"));
        edit(&dir.join("project.json"), "\"format\"", "\"series\": \"North\",\n  \"format\"");
        edit(&dir.join("project.json"), "\"settings\": {", "\"settings\": {\n    \"house_style\": \"plain\",");
        let bond_id = format!("\"id\":\"{bond}\"");
        edit(&dir.join("bible.json"), &bond_id, &format!("{bond_id},\"since\":\"spring\""));
        let link = format!("\"item\":\"{}\"", scene.id);
        edit(&dir.join("bible.json"), &link, &format!("{link},\"weight\":\"main\""));

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
        let mut meta = store.load_meta().unwrap();
        meta.title = "Kept again".into();
        meta.settings.daily_word_goal = 900;
        store.persist_meta(&meta).unwrap();
        let mut grid = store.load_plot().unwrap();
        grid.lines[0].title = "Escape".into();
        store.persist_plot(&grid).unwrap();

        let scene_text = fs::read_to_string(&scene_file).unwrap();
        assert!(scene_text.contains("\npov: Mara\n"));
        assert!(scene_text.contains("\ntitle: Renamed\n") && scene_text.contains("New text."));
        let mara_text = fs::read_to_string(&mara_file).unwrap();
        assert!(mara_text.contains("\neyes: grey\n") && mara_text.contains("Changed."));
        assert!(fs::read_to_string(dir.join("binder.json")).unwrap().contains("\"tag\":\"draft-2\""));
        let plot = fs::read_to_string(dir.join("plot.json")).unwrap();
        assert!(plot.contains("\"arc\": \"rising\"") && plot.contains("Escape"));
        let project = fs::read_to_string(dir.join("project.json")).unwrap();
        assert!(project.contains("\"series\": \"North\"") && project.contains("\"house_style\": \"plain\""));
        assert!(project.contains("Kept again") && project.contains("900"));
        let bible = fs::read_to_string(dir.join("bible.json")).unwrap();
        assert!(bible.contains("\"since\":\"spring\"") && bible.contains("\"weight\":\"main\""));

        // Restoring a backup writes every file again, and keeps them too.
        let mut store = store;
        let backup = store.backup().unwrap();
        store.restore_backup(&backup).unwrap();
        assert!(fs::read_to_string(&scene_file).unwrap().contains("\npov: Mara\n"));
        assert!(fs::read_to_string(dir.join("binder.json")).unwrap().contains("draft-2"));
        let _ = fs::remove_dir_all(dir);
    }

    /// The outside version that was set aside for `file`, read back.
    fn kept_text(store: &ProjectStore, set: &[SetAside], file: &Path) -> String {
        let relative = file.strip_prefix(&store.root).unwrap().to_string_lossy().replace('\\', "/");
        let found = set.iter().find(|item| item.file == relative).expect("set aside");
        assert!(found.kept_as.contains(".conflict-"));
        fs::read_to_string(store.root.join(&found.kept_as)).unwrap()
    }

    #[test]
    fn kept_files_are_listed_and_a_list_changed_outside_is_seen() {
        let dir = temp("kept");
        let store = ProjectStore::create(&dir, "Kept").unwrap();
        let scene = scene_of(&store);
        let scene_file = file_of(&store, scene.id);
        assert!(!store.others_changed_outside().unwrap());

        // A scene changed outside is not the others' business.
        fs::write(&scene_file, fs::read_to_string(&scene_file).unwrap() + " ").unwrap();
        assert!(!store.others_changed_outside().unwrap());
        let mut document = store.load_document(scene.id).unwrap();
        document.replace_plain_text("Mine.");
        store.save_document(&document).unwrap();
        let kept = store.kept_files().unwrap();
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].file, store.file_of(scene.id).unwrap());
        assert!(store.read_kept_file(&kept[0].kept_as).unwrap().ends_with(' '));

        // The binder changed outside is.
        let binder = dir.join("binder.json");
        fs::write(&binder, fs::read_to_string(&binder).unwrap().replace("\"kind\"", "\"x\":1,\"kind\"")).unwrap();
        assert!(store.others_changed_outside().unwrap());

        assert!(store.remove_kept_file("binder.json").is_err());
        assert!(store.read_kept_file("../outside.conflict-1Z.md").is_err());
        store.remove_kept_file(&kept[0].kept_as).unwrap();
        assert!(store.kept_files().unwrap().is_empty());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_file_changed_outside_is_set_aside_and_never_written_over() {
        let dir = temp("outside");
        let store = ProjectStore::create(&dir, "Outside").unwrap();
        let scene = scene_of(&store);
        let scene_file = file_of(&store, scene.id);
        let mut document = store.load_document(scene.id).unwrap();
        document.replace_plain_text("Typed in the program.");
        store.save_document(&document).unwrap();
        assert!(store.take_set_aside().is_empty());

        // Another program changes the scene, then the program saves it.
        let outside = fs::read_to_string(&scene_file)
            .unwrap()
            .replace("Typed in the program.", "Typed in a text editor.");
        fs::write(&scene_file, &outside).unwrap();
        document.replace_plain_text("Typed in the program, then more.");
        store.save_document(&document).unwrap();
        assert!(fs::read_to_string(&scene_file).unwrap().contains("then more."));
        let set = store.take_set_aside();
        assert_eq!(set.len(), 1);
        assert_eq!(kept_text(&store, &set, &scene_file), outside);
        // Saving again does not set it aside a second time.
        document.replace_plain_text("And again.");
        store.save_document(&document).unwrap();
        assert!(store.take_set_aside().is_empty());

        // The same for the binder file.
        let binder = dir.join("binder.json");
        let edited = fs::read_to_string(&binder).unwrap().replace("\"kind\"", "\"note\":\"x\",\"kind\"");
        fs::write(&binder, &edited).unwrap();
        let added = BinderNode::new(NodeKind::Scene, "Added", scene.parent_id, 9);
        store.upsert_node(&added).unwrap();
        let set = store.take_set_aside();
        assert_eq!(kept_text(&store, &set, &binder), edited);
        assert!(fs::read_to_string(&binder).unwrap().contains(&added.id.to_string()));

        // A file changed outside is not deleted with its scene; it is kept.
        let doomed = file_of(&store, scene.id);
        let changed = fs::read_to_string(&doomed).unwrap().replace("And again.", "Changed before the delete.");
        fs::write(&doomed, &changed).unwrap();
        store.delete_document(scene.id).unwrap();
        assert!(!doomed.exists());
        let set = store.take_set_aside();
        assert_eq!(kept_text(&store, &set, &doomed), changed);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn a_scene_changed_outside_can_be_read_again() {
        let dir = temp("reread");
        let store = ProjectStore::create(&dir, "Reread").unwrap();
        let scene = scene_of(&store);
        let scene_file = file_of(&store, scene.id);
        assert!(!store.changed_outside(scene.id).unwrap());

        let text = fs::read_to_string(&scene_file).unwrap();
        let text = text
            .replace("\ntitle: Scene 1\n", "\ntitle: Harbor\n")
            .replacen("\n---\n", "\ngoal: Leave\n---\n", 1);
        let body_at = text.find("\n---\n").unwrap() + "\n---\n".len();
        let text = format!("{}\nFrom git.\n", &text[..body_at]);
        fs::write(&scene_file, &text).unwrap();
        assert!(store.changed_outside(scene.id).unwrap());

        let (node, beats, document) = store.reread_scene(scene.id).unwrap().unwrap();
        assert_eq!(node.title, "Harbor");
        assert_eq!(beats.goal, "Leave");
        assert_eq!(document.to_plain_text(), "From git.");
        assert!(!store.changed_outside(scene.id).unwrap());
        assert_eq!(store.load_document(scene.id).unwrap().to_plain_text(), "From git.");
        assert_eq!(store.load_scene_beats().unwrap()[&scene.id].goal, "Leave");

        // Saving after reading it again sets nothing aside.
        let mut document = document;
        document.replace_plain_text("From git, then edited.");
        store.save_document(&document).unwrap();
        assert!(store.take_set_aside().is_empty());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn one_scene_comes_back_from_a_backup_and_nothing_else_changes() {
        let dir = temp("restore-scene");
        let store = ProjectStore::create(&dir, "One scene").unwrap();
        let scene = scene_of(&store);
        let mut document = store.load_document(scene.id).unwrap();
        document.replace_plain_text("Before the backup.");
        store.save_document(&document).unwrap();
        let backup = store.backup().unwrap();
        document.replace_plain_text("After the backup.");
        store.save_document(&document).unwrap();
        let later = BinderNode::new(NodeKind::Scene, "Later", scene.parent_id, 9);
        store.upsert_node(&later).unwrap();

        let restored = store.restore_scene_from_backup(&backup, scene.id).unwrap();
        assert_eq!(restored.to_plain_text(), "Before the backup.");
        assert!(fs::read_to_string(file_of(&store, scene.id)).unwrap().contains("Before the backup."));
        assert!(store.load_binder().unwrap().nodes.iter().any(|node| node.id == later.id));
        let kept = store.list_snapshots(scene.id).unwrap();
        assert!(kept.iter().any(|snapshot| snapshot.preview == "After the backup."));
        // A scene the backup does not hold is refused, and stays as it is.
        assert!(store.restore_scene_from_backup(&backup, later.id).is_err());
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
