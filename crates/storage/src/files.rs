//! A project as a folder of HTML and JSON files: project format 2.
//!
//! This writes everything a project holds into the files described in
//! `documentation/PROJECT_FORMAT.md`, and reads them back. The store
//! (`mirror.rs`) keeps them in step with every save.
//!
//! Every file is written whole to a temporary name and then renamed over the
//! old one, so a crash during a save leaves the old file or the new one.

use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};

use serde_json::{json, Map, Value};
use writing_studio_core::{Error, Id, Result, Timestamp, PROJECT_FORMAT_FILE};
use writing_studio_document::{Block, BlockKind, Document, Inline};
use writing_studio_format::{file_name, Header, SceneFile, SceneHeader};
use writing_studio_plotting::{PlotGrid, PlotPoint, Plotline};
use writing_studio_project::{
    normalize_binder_color, BinderNode, BinderTree, Keyword, MindMap, NodeKind, ProjectMeta, ProjectSettings, SavedFilter,
};

use crate::ProjectStore;

/// The format number a folder of files carries in `FORMAT`.
pub const FILES_FORMAT_VERSION: u32 = writing_studio_core::PROJECT_FORMAT_VERSION;

/// Settings that every machine working on the project shares. Every other
/// setting is about one machine's window and writing habits, and stays in
/// `.local/`.
const SHARED_SETTINGS: [&str; 7] = [
    "daily_word_goal",
    "project_word_goal",
    "compile_profiles",
    "author",
    "book_subtitles",
    "book_word_goals",
    "spelling",
];

const README: &str = include_str!("../assets/project_readme.md");

/// Everything a project holds, in memory.
#[derive(Clone, Debug)]
pub struct ProjectContent {
    pub meta: ProjectMeta,
    pub binder: BinderTree,
    /// The text of each binder item that has text. In format 2 a document
    /// has the id of its binder item.
    pub documents: Vec<Document>,
    pub plot: PlotGrid,
    /// Ideas, lines, and hand-placed nodes of the mind map (`mindmap.json`).
    pub mindmap: MindMap,
    /// The file each item is kept in, relative to the project folder. A file
    /// keeps the name it was first given; an item with no entry here gets a
    /// name when it is written.
    pub files: HashMap<Id, String>,
    /// What the files say about an item that the program does not use, by
    /// the item's id. Only items that have some are here.
    pub extras: HashMap<Id, Extras>,
    /// A fingerprint of what each file held when it was read, by its path.
    /// The store compares a file with this before writing over it.
    pub read: HashMap<String, u64>,
    /// What was wrong with the files, and what was done about it.
    pub problems: Vec<Problem>,
    /// Items whose file could not be read. They stay in the binder, and
    /// their file is never written over.
    pub missing: HashSet<Id>,
    /// Something was recovered or repaired while reading, so the files must
    /// be written again to say so.
    pub repaired: bool,
}

/// Something wrong with a project's files, found when it was read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Problem {
    /// The file, inside the project folder.
    pub file: String,
    pub message: String,
}

impl std::fmt::Display for Problem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.file, self.message)
    }
}

/// The folder at the top of the manuscript that recovered items go in.
const RECOVERED: &str = "Recovered";

/// A fingerprint of a file's text, to tell whether it has changed.
pub(crate) fn text_fingerprint(text: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

/// Entries a file holds that this program does not know: added by a person,
/// or by a later version of the program. They are read with the item and
/// written back after the entries the program knows, so that saving never
/// removes them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Extras {
    /// Keys in the front matter of the item's own file.
    pub head: Header,
    /// Keys of the item's line in `binder.json` or `plot.json`.
    /// For the project, the keys of `project.json`, and its unknown settings
    /// under `settings`.
    pub json: Map<String, Value>,
    /// Text under the front matter of a book's, part's, or folder's file,
    /// which have no text of their own: kept and written back as found.
    pub body: String,
}

impl Extras {
    fn is_empty(&self) -> bool {
        self.head.entries.is_empty() && self.json.is_empty() && self.body.is_empty()
    }
}

/// The keys of each kind of line that the program writes. Any other key is
/// kept as an extra; one of these never is, so that an extra cannot stand in
/// for what the program knows.
const BINDER_KEYS: [&str; 13] = [
    "id", "kind", "parent", "deleted", "restore_parent", "file", "status", "title", "context", "label",
    "color", "keywords", "left_out",
];
const PLOTLINE_KEYS: [&str; 4] = ["id", "title", "color", "points"];
const POINT_KEYS: [&str; 4] = ["id", "title", "summary", "scene"];
const PROJECT_KEYS: [&str; 5] = ["format", "id", "title", "created", "settings"];
/// Keys in an item's front matter that the program writes. Any other is
/// kept as found, the `synopsis`, `notes`, and beats of a file from before
/// Phase 15 among them.
const SCENE_HEAD: [&str; 9] = ["id", "title", "context", "status", "label", "color", "tags", "footnotes", "links"];

fn unknown_keys(item: &Value, known: &[&str]) -> Map<String, Value> {
    item.as_object()
        .into_iter()
        .flatten()
        .filter(|(key, _)| !known.contains(&key.as_str()))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

pub(crate) fn unknown_head(header: &Header, known: &[&str]) -> Header {
    Header {
        entries: header
            .entries
            .iter()
            .filter(|(key, _)| !known.contains(&key.as_str()) && !SCENE_HEAD.contains(&key.as_str()))
            .cloned()
            .collect(),
    }
}

/// Add an item's extra keys to its line, after the ones the program wrote.
fn add_json_extras(item: &mut Map<String, Value>, extras: Option<&Extras>, known: &[&str]) {
    for (key, value) in extras.into_iter().flat_map(|extras| &extras.json) {
        if !known.contains(&key.as_str()) && !item.contains_key(key) {
            item.insert(key.clone(), value.clone());
        }
    }
}

/// Add an item's extra names to the head of its file.
fn add_head_extras(header: &mut Header, extras: Option<&Extras>, known: &[&str]) {
    for (key, value) in extras.into_iter().flat_map(|extras| &extras.head.entries) {
        let taken = known.contains(&key.as_str())
            || SCENE_HEAD.contains(&key.as_str())
            || header.entries.iter().any(|(name, _)| name == key);
        if !taken {
            header.entries.push((key.clone(), value.clone()));
        }
    }
}

impl ProjectContent {
    /// Read the project out of the cache. Each document is given the id of
    /// its binder item, which is how the files identify text.
    pub fn from_store(store: &ProjectStore) -> Result<Self> {
        let mut binder = store.load_binder()?;
        let mut documents = Vec::new();
        for node in &mut binder.nodes {
            let Some(document_id) = node.document_id else {
                continue;
            };
            // An item that was never opened has no text yet.
            let mut document = store
                .load_document(document_id)
                .unwrap_or_else(|_| Document::new(document_id));
            document.id = node.id;
            node.document_id = Some(node.id);
            documents.push(document);
        }
        Ok(Self {
            meta: store.load_meta()?,
            binder,
            documents,
            plot: store.load_plot()?,
            mindmap: store.load_mindmap()?,
            files: HashMap::new(),
            extras: HashMap::new(),
            read: HashMap::new(),
            problems: Vec::new(),
            missing: HashSet::new(),
            repaired: false,
        })
    }
}

fn bad(file: &str, what: impl std::fmt::Display) -> Error {
    Error::message(format!("{file}: {what}"))
}

/// Write `contents` to `path` so that the file is never seen half written.
pub fn write_atomic(path: &Path, contents: &[u8]) -> Result<()> {
    let Some(parent) = path.parent() else {
        return Err(Error::message("a file needs a folder to be written in"));
    };
    fs::create_dir_all(parent)?;
    let name = path.file_name().and_then(|name| name.to_str()).unwrap_or("file");
    let temporary = parent.join(format!(".{name}.{}.tmp", Id::new()));
    let written = (|| -> std::io::Result<()> {
        let mut file = fs::File::create(&temporary)?;
        file.write_all(contents)?;
        file.sync_all()?;
        fs::rename(&temporary, path)
    })();
    if written.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    Ok(written?)
}

/// A path from inside a project file, checked so that it cannot name
/// anything outside the project folder.
///
/// A project can come from someone else, through git, and git carries
/// symbolic links. So the path is also followed on disk: if a link on the
/// way leads out of the project folder, or nowhere, the path is refused.
pub(crate) fn inside(root: &Path, file: &str, relative: &str) -> Result<PathBuf> {
    let path = Path::new(relative);
    let plain = !relative.is_empty()
        && !relative.contains('\\')
        && path.components().all(|part| matches!(part, Component::Normal(_)));
    if !plain {
        return Err(bad(file, format!("“{relative}” is not a path inside the project")));
    }
    let full = root.join(path);
    // A folder that does not exist yet has no links in it.
    let Ok(top) = fs::canonicalize(root) else {
        return Ok(full);
    };
    let mut nearest = full.as_path();
    while fs::symlink_metadata(nearest).is_err() {
        match nearest.parent() {
            Some(parent) if parent.starts_with(root) => nearest = parent,
            _ => break,
        }
    }
    let through_a_link = || bad(file, format!("“{relative}” leads outside the project through a link"));
    let resolved = fs::canonicalize(nearest).map_err(|_| through_a_link())?;
    if !resolved.starts_with(&top) {
        return Err(through_a_link());
    }
    Ok(full)
}

/// A path inside the project folder, checked as every path from a project
/// file is: no `..`, nothing absolute, and no link that leads outside.
pub fn project_path(root: &Path, relative: &str) -> Result<PathBuf> {
    inside(root, "the project", relative)
}

/// Give an item its file. A name that is taken, which needs two titles and
/// two ids that start alike, gets more of the id.
pub(crate) fn place(
    files: &HashMap<Id, String>,
    used: &mut HashSet<String>,
    folder: &str,
    title: &str,
    id: Id,
) -> String {
    if let Some(known) = files.get(&id) {
        used.insert(known.clone());
        return known.clone();
    }
    let mut path = format!("{folder}/{}", file_name(title, id));
    if used.contains(&path) || files.values().any(|taken| *taken == path) {
        path = format!("{folder}/{}-{id}.md", file_name(title, id).trim_end_matches(".md"));
    }
    used.insert(path.clone());
    path
}

/// JSON with one item of each list on its own line, so that adding, moving,
/// or changing an item changes one line of the file.
fn lined(sections: &[(&str, Vec<Value>)]) -> String {
    let mut out = String::from("{\n");
    for (index, (name, items)) in sections.iter().enumerate() {
        out.push_str(&format!("  {}: [", Value::from(*name)));
        for (at, item) in items.iter().enumerate() {
            out.push_str(if at == 0 { "\n    " } else { ",\n    " });
            out.push_str(&item.to_string());
        }
        out.push_str(if items.is_empty() { "]" } else { "\n  ]" });
        out.push_str(if index + 1 == sections.len() { "\n" } else { ",\n" });
    }
    out.push_str("}\n");
    out
}

/// `mindmap.json`: its ideas, lines, and positions one to a line, then any
/// keys the program does not know.
pub(crate) fn mindmap_json(map: &MindMap) -> Result<String> {
    let list = |items: Result<Value>| -> Result<Vec<Value>> {
        Ok(items?.as_array().cloned().unwrap_or_default())
    };
    let value = |item: serde_json::Result<Value>| item.map_err(|err| Error::message(err.to_string()));
    let mut text = lined(&[
        ("ideas", list(value(serde_json::to_value(&map.ideas)))?),
        ("lines", list(value(serde_json::to_value(&map.lines)))?),
        ("positions", list(value(serde_json::to_value(&map.positions)))?),
    ]);
    if !map.other.is_empty() {
        text.truncate(text.len() - "\n}\n".len());
        for (key, item) in &map.other {
            text.push_str(&format!(",\n  {}: {}", Value::from(key.as_str()), item));
        }
        text.push_str("\n}\n");
    }
    Ok(text)
}

/// Read `mindmap.json`, or an empty map when the project has none.
pub(crate) fn read_mindmap(root: &Path, seen: &RefCell<HashMap<String, u64>>) -> Result<MindMap> {
    if !root.join(MINDMAP_FILE).exists() {
        return Ok(MindMap::default());
    }
    let text = read_noted(&root.join(MINDMAP_FILE), MINDMAP_FILE, seen)?;
    serde_json::from_str(&text).map_err(|err| bad(MINDMAP_FILE, err))
}

pub(crate) const MINDMAP_FILE: &str = "mindmap.json";

fn pretty(value: &Value) -> Result<String> {
    let mut text = serde_json::to_string_pretty(value).map_err(|err| Error::message(err.to_string()))?;
    text.push('\n');
    Ok(text)
}

/// Binder items in the order they are written: a parent, then its children
/// in order. The order of the lines is the order of the items; nothing is
/// numbered, so moving one item moves one line.
fn in_tree_order(tree: &BinderTree) -> Vec<&BinderNode> {
    let mut children: HashMap<Option<Id>, Vec<&BinderNode>> = HashMap::new();
    let known: HashSet<Id> = tree.nodes.iter().map(|node| node.id).collect();
    for node in &tree.nodes {
        // An item whose parent is gone is written at the top level, not lost.
        let parent = node.parent_id.filter(|parent| known.contains(parent));
        children.entry(parent).or_default().push(node);
    }
    for list in children.values_mut() {
        list.sort_by_key(|node| node.sort_index);
    }
    let mut out = Vec::new();
    let mut waiting: Vec<&BinderNode> = children.get(&None).cloned().unwrap_or_default();
    waiting.reverse();
    let mut seen = HashSet::new();
    while let Some(node) = waiting.pop() {
        if !seen.insert(node.id) {
            continue;
        }
        out.push(node);
        if let Some(list) = children.get(&Some(node.id)) {
            waiting.extend(list.iter().rev());
        }
    }
    out
}

/// A project as the files it is stored in, before any of them is written.
struct Rendered {
    /// The file each item is in.
    files: HashMap<Id, String>,
    /// Each file's path inside the project, and its text.
    contents: Vec<(String, String)>,
}

/// Write a project into `root` as format 2 files, all but the `FORMAT`
/// marker. Files of items that are no longer in the project are left where
/// they are; nothing is deleted. Returns the file each item was written to.
pub fn write_project_files(root: &Path, content: &ProjectContent) -> Result<HashMap<Id, String>> {
    Ok(write_project_files_noted(root, content)?.0)
}

/// The same, also returning a fingerprint of each file that was written.
pub(crate) fn write_project_files_noted(
    root: &Path,
    content: &ProjectContent,
) -> Result<(HashMap<Id, String>, HashMap<String, u64>)> {
    let rendered = render(content)?;
    let mut written = HashMap::new();
    for (path, text) in &rendered.contents {
        write_atomic(&inside(root, "the project", path)?, text.as_bytes())?;
        written.insert(path.clone(), text_fingerprint(text));
    }
    // What makes the folder readable and safe to share. The reader's guide
    // is not overwritten, so it can be changed.
    // `.gitattributes` keeps git on any system from changing line endings,
    // and from treating research files as text.
    for (name, text) in [
        ("README.md", README),
        (".gitignore", ".local/\n"),
        (".gitattributes", "* text=auto eol=lf\nattachments/** -text\n"),
    ] {
        if !root.join(name).exists() {
            write_atomic(&root.join(name), text.as_bytes())?;
        }
    }
    Ok((rendered.files, written))
}

/// Mark the folder as a project of this format. This is the last step of
/// making one, so that a folder that says so always has its files.
pub fn write_format_marker(root: &Path) -> Result<()> {
    write_atomic(
        &root.join(PROJECT_FORMAT_FILE),
        format!("writing-studio-project {FILES_FORMAT_VERSION}\n").as_bytes(),
    )
}

/// The folder an item's file goes in: `notes/` for what is under the Notes
/// root, `manuscript/` for the rest.
pub(crate) fn folder_of(tree: &BinderTree, id: Id) -> &'static str {
    if tree.is_in_notes(id) { "notes" } else { "manuscript" }
}

/// Whether a binder item has a file of its own: every item but the three
/// roots. A scene, a chapter, or a note keeps its text there; a book, a part,
/// or a folder its details.
pub(crate) fn has_own_file(node: &BinderNode) -> bool {
    !node.is_root()
}

/// The file of one binder item: its details as front matter, its keywords
/// as `tags`, and its text.
pub(crate) fn scene_text(node: &BinderNode, blocks: &[Block], extras: Option<&Extras>) -> String {
    let mut other = Header::default();
    if !node.keywords.is_empty() {
        other.set_list("tags", &node.keywords);
    }
    if node.footnotes {
        other.set_text("footnotes", "true");
    }
    let links: Vec<String> = node.links.iter().map(Id::to_string).collect();
    other.set_list("links", &links);
    add_head_extras(&mut other, extras, &[]);
    // A book, a part, or a folder has no text; what a person wrote under its
    // front matter is kept.
    let kept;
    let blocks = match extras {
        Some(found) if node.document_id.is_none() && !found.body.is_empty() => {
            kept = writing_studio_format::markdown_to_blocks(&found.body);
            kept.as_slice()
        }
        _ => blocks,
    };
    SceneFile {
        header: SceneHeader {
            id: Some(node.id),
            title: node.title.clone(),
            context: node.context.clone(),
            status: node.status.clone(),
            label: node.label.clone(),
            color: node.color.clone(),
            other,
        },
        blocks: blocks.to_vec(),
    }
    .to_markdown()
}

/// `binder.json`. `paths` has the file of every item with text.
pub(crate) fn binder_json(
    tree: &BinderTree,
    paths: &HashMap<Id, String>,
    extras: &HashMap<Id, Extras>,
) -> String {
    let mut items = Vec::new();
    for node in in_tree_order(tree) {
        let mut item = Map::new();
        item.insert("id".into(), json!(node.id.to_string()));
        item.insert("kind".into(), json!(node.kind.as_str()));
        if let Some(parent) = node.parent_id {
            item.insert("parent".into(), json!(parent.to_string()));
        }
        if let Some(deleted) = node.deleted_at {
            item.insert("deleted".into(), json!(deleted.to_rfc3339()));
        }
        if let Some(parent) = node.restore_parent_id {
            item.insert("restore_parent".into(), json!(parent.to_string()));
        }
        match paths.get(&node.id).filter(|_| has_own_file(node)) {
            Some(path) => {
                item.insert("file".into(), json!(path));
            }
            None => {
                // A root has no file, so its details stay here.
                // An item with no text of its own has nowhere else to keep these.
                item.insert("status".into(), json!(node.status));
                for (key, value) in [
                    ("title", &node.title),
                    ("context", &node.context),
                    ("label", &node.label),
                    ("color", &node.color),
                ] {
                    if !value.is_empty() {
                        item.insert(key.into(), json!(value));
                    }
                }
                if !node.keywords.is_empty() {
                    item.insert("keywords".into(), json!(node.keywords));
                }
            }
        }
        if node.left_out {
            item.insert("left_out".into(), json!(true));
        }
        add_json_extras(&mut item, extras.get(&node.id), &BINDER_KEYS);
        items.push(Value::Object(item));
    }
    // The keyword colors and saved filters, only once there are any, so a
    // project that has none keeps the file it had.
    let mut sections = vec![("items", items)];
    if !tree.keywords.is_empty() {
        sections.push(("keywords", tree.keywords.iter().map(|keyword| json!(keyword)).collect()));
    }
    if !tree.filters.is_empty() {
        sections.push(("filters", tree.filters.iter().map(|filter| json!(filter)).collect()));
    }
    lined(&sections)
}

pub(crate) fn plot_json(plot: &PlotGrid, extras: &HashMap<Id, Extras>) -> Result<String> {
    let plotlines: Vec<Value> = sorted(&plot.lines, |line| line.sort_index)
        .into_iter()
        .map(|line| {
            let points: Vec<Value> = sorted(&plot.points, |point| point.sort_index)
                .into_iter()
                .filter(|point| point.plotline_id == line.id)
                .map(|point| {
                    let mut item = Map::new();
                    item.insert("id".into(), json!(point.id.to_string()));
                    item.insert("title".into(), json!(point.title));
                    item.insert("summary".into(), json!(point.summary));
                    if let Some(scene) = point.scene_id {
                        item.insert("scene".into(), json!(scene.to_string()));
                    }
                    add_json_extras(&mut item, extras.get(&point.id), &POINT_KEYS);
                    Value::Object(item)
                })
                .collect();
            let mut item = Map::new();
            item.insert("id".into(), json!(line.id.to_string()));
            item.insert("title".into(), json!(line.title));
            item.insert("color".into(), json!(line.color));
            item.insert("points".into(), json!(points));
            add_json_extras(&mut item, extras.get(&line.id), &PLOTLINE_KEYS);
            Value::Object(item)
        })
        .collect();
    pretty(&json!({ "plotlines": plotlines }))
}

/// `project.json`, and this machine's `.local/settings.json`.
pub(crate) fn settings_json(meta: &ProjectMeta, extras: Option<&Extras>) -> Result<(String, String)> {
    let settings = serde_json::to_value(&meta.settings).map_err(|err| Error::message(err.to_string()))?;
    let (mut shared, mut local) = (Map::new(), Map::new());
    for (key, value) in settings.as_object().cloned().unwrap_or_default() {
        if SHARED_SETTINGS.contains(&key.as_str()) {
            shared.insert(key, value);
        } else {
            local.insert(key, value);
        }
    }
    let unknown = extras.and_then(|found| found.json.get("settings")).and_then(Value::as_object);
    for (key, value) in unknown.into_iter().flatten() {
        if !shared.contains_key(key) && !local.contains_key(key) {
            shared.insert(key.clone(), value.clone());
        }
    }
    let mut project = Map::new();
    project.insert("format".into(), json!(FILES_FORMAT_VERSION));
    project.insert("id".into(), json!(meta.id.to_string()));
    project.insert("title".into(), json!(meta.title));
    project.insert("created".into(), json!(meta.created_at.to_rfc3339()));
    project.insert("settings".into(), Value::Object(shared));
    add_json_extras(&mut project, extras, &PROJECT_KEYS);
    Ok((pretty(&Value::Object(project))?, pretty(&Value::Object(local))?))
}

pub(crate) const LOCAL_SETTINGS: &str = ".local/settings.json";

fn render(content: &ProjectContent) -> Result<Rendered> {
    let mut contents: Vec<(String, String)> = Vec::new();
    let mut files: HashMap<Id, String> = HashMap::new();
    let mut used: HashSet<String> = HashSet::new();
    let documents: HashMap<Id, &Document> = content.documents.iter().map(|doc| (doc.id, doc)).collect();

    for node in content.binder.nodes.iter().filter(|node| has_own_file(node)) {
        let folder = folder_of(&content.binder, node.id);
        let path = place(&content.files, &mut used, folder, &node.title, node.id);
        if content.missing.contains(&node.id) {
            files.insert(node.id, path);
            continue;
        }
        let blocks = documents.get(&node.id).map(|document| document.blocks.as_slice()).unwrap_or_default();
        contents.push((path.clone(), scene_text(node, blocks, content.extras.get(&node.id))));
        files.insert(node.id, path);
    }
    contents.push(("binder.json".into(), binder_json(&content.binder, &files, &content.extras)));

    contents.push(("plot.json".into(), plot_json(&content.plot, &content.extras)?));
    // Only once the map has something of its own: a project without the
    // file has no ideas or lines.
    if !content.mindmap.is_empty() {
        contents.push((MINDMAP_FILE.into(), mindmap_json(&content.mindmap)?));
    }
    let (project, local) = settings_json(&content.meta, content.extras.get(&content.meta.id))?;
    contents.push(("project.json".into(), project));
    contents.push((LOCAL_SETTINGS.into(), local));
    Ok(Rendered { files, contents })
}

fn sorted<T>(items: &[T], key: fn(&T) -> i64) -> Vec<&T> {
    let mut out: Vec<&T> = items.iter().collect();
    out.sort_by_key(|item| key(item));
    out
}

struct JsonFile<'a> {
    name: &'a str,
    value: Value,
}

/// Read a file of the project, and note what it held.
fn read_noted(path: &Path, name: &str, seen: &RefCell<HashMap<String, u64>>) -> Result<String> {
    let text = fs::read_to_string(path).map_err(|err| bad(name, err))?;
    seen.borrow_mut().insert(name.to_string(), text_fingerprint(&text));
    Ok(text)
}

impl<'a> JsonFile<'a> {
    fn read(root: &Path, name: &'a str, seen: &RefCell<HashMap<String, u64>>) -> Result<Self> {
        let text = read_noted(&root.join(name), name, seen)?;
        let value = serde_json::from_str(&text).map_err(|err| bad(name, err))?;
        Ok(Self { name, value })
    }

    fn list(&self, key: &str) -> Vec<&Value> {
        self.value
            .get(key)
            .and_then(Value::as_array)
            .map(|items| items.iter().collect())
            .unwrap_or_default()
    }

    fn id(&self, item: &Value, key: &str) -> Result<Id> {
        self.optional_id(item, key)?
            .ok_or_else(|| bad(self.name, format!("an item has no “{key}”")))
    }

    fn optional_id(&self, item: &Value, key: &str) -> Result<Option<Id>> {
        match item.get(key).and_then(Value::as_str) {
            None => Ok(None),
            Some(text) => text
                .parse()
                .map(Some)
                .map_err(|_| bad(self.name, format!("“{text}” is not an id"))),
        }
    }
}

fn text_of(item: &Value, key: &str) -> String {
    item.get(key).and_then(Value::as_str).unwrap_or_default().to_string()
}

/// An HTML file named by a JSON file, read and checked to be the entry it
/// is listed as.
/// Read an item's file. A path that leaves the project is an error; a file
/// that is missing or belongs to another item is the inner error, which the
/// caller reports and goes on.
fn read_entry(
    root: &Path,
    listed_in: &str,
    path: &str,
    id: Id,
    seen: &RefCell<HashMap<String, u64>>,
) -> Result<std::result::Result<SceneFile, String>> {
    let full = inside(root, listed_in, path)?;
    let Ok(text) = read_noted(&full, path, seen) else {
        return Ok(Err("the file is missing or could not be read".into()));
    };
    let file = match read_scene_file(&text) {
        Ok(file) => file,
        Err(problem) => return Ok(Err(problem)),
    };
    Ok(match file.header.id {
        Some(found) if found != id => Err("the id in the file is not the one it is listed under".into()),
        _ => Ok(file),
    })
}

/// Read a project from its format 2 files. It does not look at `FORMAT`;
/// whoever opens the folder decides what it is. Times of creation and change
/// are not kept in the files, so every record reads as made now.
pub fn read_project_files(root: &Path) -> Result<ProjectContent> {
    let now = Timestamp::now();
    let seen: RefCell<HashMap<String, u64>> = RefCell::new(HashMap::new());
    let mut files: HashMap<Id, String> = HashMap::new();
    let mut extras: HashMap<Id, Extras> = HashMap::new();
    let mut keep = |id: Id, found: Extras| {
        if !found.is_empty() {
            extras.insert(id, found);
        }
    };
    let mut problems: Vec<Problem> = Vec::new();
    let mut missing: HashSet<Id> = HashSet::new();
    let mut ids: HashSet<Id> = HashSet::new();
    let mut repaired = false;
    let mut problem = |file: &str, message: &str| {
        problems.push(Problem {
            file: file.to_string(),
            message: message.to_string(),
        })
    };

    // Settings.
    let project = JsonFile::read(root, "project.json", &seen)?;
    let mut settings = serde_json::to_value(ProjectSettings::default())
        .ok()
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default();
    let local: Value = read_noted(&root.join(LOCAL_SETTINGS), LOCAL_SETTINGS, &seen)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or(Value::Null);
    for (key, value) in local.as_object().into_iter().flatten() {
        if !SHARED_SETTINGS.contains(&key.as_str()) {
            settings.insert(key.clone(), value.clone());
        }
    }
    for (key, value) in project.value.get("settings").and_then(Value::as_object).into_iter().flatten() {
        if SHARED_SETTINGS.contains(&key.as_str()) {
            settings.insert(key.clone(), value.clone());
        }
    }
    let unknown_settings: Map<String, Value> = project
        .value
        .get("settings")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .filter(|(key, _)| !settings.contains_key(key.as_str()))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    let settings: ProjectSettings =
        serde_json::from_value(Value::Object(settings)).map_err(|err| bad("project.json", err))?;
    let created = Timestamp::parse(&text_of(&project.value, "created")).unwrap_or(now);
    let mut project_extras = unknown_keys(&project.value, &PROJECT_KEYS);
    if !unknown_settings.is_empty() {
        project_extras.insert("settings".into(), Value::Object(unknown_settings));
    }
    let meta = ProjectMeta {
        id: project.id(&project.value, "id")?,
        format_version: FILES_FORMAT_VERSION,
        title: text_of(&project.value, "title"),
        created_at: created,
        updated_at: now,
        settings,
    };
    keep(meta.id, Extras { json: project_extras, ..Extras::default() });

    // The binder and the manuscript.
    let binder_file = JsonFile::read(root, "binder.json", &seen)?;
    let mut nodes = Vec::new();
    let mut documents = Vec::new();
    let mut next_index: HashMap<Option<Id>, i64> = HashMap::new();
    for item in binder_file.list("items") {
        let id = binder_file.id(item, "id")?;
        if !ids.insert(id) {
            problem("binder.json", &format!("item {id} was listed twice; the second line was left out"));
            repaired = true;
            continue;
        }
        let kind_name = text_of(item, "kind");
        let kind = NodeKind::parse(&kind_name)
            .ok_or_else(|| bad("binder.json", format!("“{kind_name}” is not a kind of item")))?;
        let parent = binder_file.optional_id(item, "parent")?;
        let index = next_index.entry(parent).or_insert(0);
        let mut node = BinderNode::new(kind, text_of(item, "title"), parent, *index);
        *index += 1;
        node.id = id;
        node.context = text_of(item, "context");
        node.label = text_of(item, "label");
        node.color = text_of(item, "color");
        node.keywords = keywords_of(item);
        node.left_out = item.get("left_out").and_then(Value::as_bool).unwrap_or(false);
        if let Some(status) = item.get("status").and_then(Value::as_str) {
            node.status = status.to_string();
        }
        node.deleted_at = item
            .get("deleted")
            .and_then(Value::as_str)
            .map(|text| Timestamp::parse(text).unwrap_or(now));
        node.restore_parent_id = binder_file.optional_id(item, "restore_parent")?;
        node.document_id = None;
        let mut found = Extras {
            json: unknown_keys(item, &BINDER_KEYS),
            ..Extras::default()
        };
        if let Some(path) = item.get("file").and_then(Value::as_str) {
            files.insert(id, path.to_string());
            let holds_text = node.kind.can_hold_document();
            if holds_text {
                node.document_id = Some(id);
            }
            let file = match read_entry(root, "binder.json", path, id, &seen)? {
                Ok(file) => file,
                Err(why) => {
                    // The item stays, marked, and its file is never written
                    // over. Its title was in the file, so it shows the file's
                    // name, which says what to restore.
                    problem(path, &why);
                    node.title = path.rsplit('/').next().unwrap_or(path).to_string();
                    missing.insert(id);
                    keep(id, found);
                    nodes.push(node);
                    continue;
                }
            };
            found.head = unknown_head(&file.header.other, &["tags"]);
            // Keywords are in the file; a file from before they moved there
            // has none, and the binder line's are kept.
            let tags = tags_of(&file.header.other);
            if !tags.is_empty() {
                node.keywords = tags;
            }
            node.footnotes = file.header.other.text("footnotes").is_some_and(|value| value.trim() == "true");
            node.links = match (file.header.other.list("links"), file.header.other.text("links")) {
                (Some(list), _) => list.iter().filter_map(|id| id.trim().parse().ok()).collect(),
                (None, Some(one)) => one.trim().parse().ok().into_iter().collect(),
                _ => Vec::new(),
            };
            node.title = file.header.title;
            node.context = file.header.context;
            node.status = file.header.status;
            node.label = file.header.label;
            node.color = file.header.color;
            if !holds_text {
                let written = file.blocks.iter().any(|block| !block.plain_text().trim().is_empty());
                if written {
                    found.body = writing_studio_format::blocks_to_markdown(&file.blocks);
                }
                keep(id, found);
                nodes.push(node);
                continue;
            }
            let mut document = Document::new(id);
            document.blocks = with_a_run_in_every_block(file.blocks);
            documents.push(document);
        }
        keep(id, found);
        nodes.push(node);
    }

    // Files no line of the binder names, added by hand or by git: a scene
    // into Recovered, a note into the Notes root.
    let found_scenes = unlisted(root, "manuscript", &files, &seen, &mut problem)?
        .into_iter()
        .map(|found| (NodeKind::Scene, found));
    let found_notes = unlisted(root, "notes", &files, &seen, &mut problem)?
        .into_iter()
        .map(|found| (NodeKind::Note, found));
    let found: Vec<_> = found_scenes.chain(found_notes).collect();
    for (kind, (path, file)) in found {
        let id = file.header.id.filter(|id| !ids.contains(id)).unwrap_or_else(Id::new);
        ids.insert(id);
        let (parent, put_in) = if kind == NodeKind::Note {
            (nodes.iter().find(|node| node.kind == NodeKind::NotesRoot).map(|node| node.id), "Notes")
        } else {
            (recovered_folder(&mut nodes), RECOVERED)
        };
        let fallback = path.rsplit('/').next().unwrap_or(&path).trim_end_matches(".md").to_string();
        let title = if file.header.title.trim().is_empty() { fallback } else { file.header.title.clone() };
        let mut node = BinderNode::new(kind, title, parent, i64::MAX);
        node.id = id;
        node.document_id = Some(id);
        node.context = file.header.context.clone();
        node.label = file.header.label.clone();
        node.color = file.header.color.clone();
        if !file.header.status.is_empty() {
            node.status = file.header.status.clone();
        }
        let mut document = Document::new(id);
        document.blocks = with_a_run_in_every_block(file.blocks);
        documents.push(document);
        nodes.push(node);
        files.insert(id, path.clone());
        problem(&path, &format!("was not in the binder; it was added to {put_in}"));
        repaired = true;
    }
    if check_binder(&mut nodes, &mut problem)? {
        repaired = true;
    }

    let mindmap = read_mindmap(root, &seen)?;

    // The plot grid.
    let plot_file = JsonFile::read(root, "plot.json", &seen)?;
    let mut plot = PlotGrid::default();
    for (index, line) in plot_file.list("plotlines").into_iter().enumerate() {
        let line_id = plot_file.id(line, "id")?;
        keep(
            line_id,
            Extras {
                json: unknown_keys(line, &PLOTLINE_KEYS),
                ..Extras::default()
            },
        );
        plot.lines.push(Plotline {
            id: line_id,
            title: text_of(line, "title"),
            color: text_of(line, "color"),
            sort_index: index as i64,
            created_at: now,
            updated_at: now,
        });
        let points = line.get("points").and_then(Value::as_array).cloned().unwrap_or_default();
        for (at, point) in points.iter().enumerate() {
            let point_id = plot_file.id(point, "id")?;
            keep(
                point_id,
                Extras {
                    json: unknown_keys(point, &POINT_KEYS),
                    ..Extras::default()
                },
            );
            plot.points.push(PlotPoint {
                id: point_id,
                plotline_id: line_id,
                title: text_of(point, "title"),
                summary: text_of(point, "summary"),
                scene_id: plot_file.optional_id(point, "scene")?,
                sort_index: at as i64,
                created_at: now,
                updated_at: now,
            });
        }
    }

    let mut binder = BinderTree::new(nodes);
    binder.keywords = binder_file
        .list("keywords")
        .into_iter()
        .filter_map(|value| serde_json::from_value::<Keyword>(value.clone()).ok())
        .filter_map(|keyword| {
            let name: String = keyword.name.trim().chars().take(KEYWORD_LIMIT).collect();
            (!name.is_empty()).then(|| Keyword { name, color: normalize_binder_color(&keyword.color) })
        })
        .collect();
    // A filter took one `keyword` (or, before that, a `label`) until it could
    // take several. Nothing converts it: the project is refused, saying why.
    if binder_file.list("filters").iter().any(|filter| filter.get("keyword").is_some() || filter.get("label").is_some()) {
        return Err(bad(
            "binder.json",
            "a saved filter was written by an earlier version: it has \"keyword\" where this version reads \"keywords\", a list.              Change \"keyword\": \"Flashback\" to \"keywords\": [\"Flashback\"] in that filter (and drop any \"label\"), or remove the filter, then open the project again.",
        ));
    }
    binder.filters = binder_file
        .list("filters")
        .into_iter()
        .filter_map(|value| serde_json::from_value::<SavedFilter>(value.clone()).ok())
        .collect();
    // Labels are keywords now; a project from before keeps them that way.
    binder.fold_labels_into_keywords();

    Ok(ProjectContent {
        meta,
        binder,
        documents,
        plot,
        mindmap,
        files,
        extras,
        read: seen.into_inner(),
        problems,
        missing,
        repaired,
    })
}

/// The HTML files in `folder` that no list names. Temporary files and
/// versions set aside after a change outside the program are not items.
fn unlisted(
    root: &Path,
    folder: &str,
    files: &HashMap<Id, String>,
    seen: &RefCell<HashMap<String, u64>>,
    problem: &mut impl FnMut(&str, &str),
) -> Result<Vec<(String, SceneFile)>> {
    let listed: HashSet<&String> = files.values().collect();
    let Ok(entries) = fs::read_dir(inside(root, folder, folder)?) else {
        return Ok(Vec::new());
    };
    let mut names: Vec<String> = entries
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".md") && !name.starts_with('.') && !name.contains(".conflict-"))
        .map(|name| format!("{folder}/{name}"))
        .filter(|path| !listed.contains(path))
        .collect();
    names.sort();
    Ok(names
        .into_iter()
        .filter_map(|path| match read_noted(&root.join(&path), &path, seen).map(|text| read_scene_file(&text)) {
            Ok(Ok(file)) => Some((path, file)),
            Ok(Err(why)) => {
                problem(&path, &format!("{why}, and was left out"));
                None
            }
            Err(_) => {
                problem(&path, "could not be read, and was left out");
                None
            }
        })
        .collect())
}

/// The Recovered folder at the top of the manuscript, made if it is not
/// there yet. `None` when there is no manuscript to put it in.
fn recovered_folder(nodes: &mut Vec<BinderNode>) -> Option<Id> {
    let draft = nodes.iter().find(|node| node.kind == NodeKind::DraftRoot)?.id;
    let found = nodes
        .iter()
        .find(|node| node.kind == NodeKind::Folder && node.title == RECOVERED && node.parent_id == Some(draft));
    if let Some(folder) = found {
        return Some(folder.id);
    }
    let folder = BinderNode::new(NodeKind::Folder, RECOVERED, Some(draft), -1);
    let id = folder.id;
    nodes.push(folder);
    Some(id)
}

/// Make a binder a tree again, as opening a project does: anything whose
/// parent is not there, or that is in a loop, moves to Recovered. Returns
/// what was moved.
pub fn repair_binder(nodes: &mut Vec<BinderNode>) -> Result<Vec<Problem>> {
    let mut problems = Vec::new();
    check_binder(nodes, &mut |file: &str, message: &str| {
        problems.push(Problem {
            file: file.to_string(),
            message: message.to_string(),
        })
    })?;
    Ok(problems)
}

/// Make the binder a tree again after the files were changed outside the
/// program. An item whose parent is not there, or that is inside a loop of
/// parents, is moved into Recovered. Returns whether anything was moved.
/// A binder without exactly one of each root is not a project.
fn check_binder(nodes: &mut Vec<BinderNode>, problem: &mut impl FnMut(&str, &str)) -> Result<bool> {
    for kind in [NodeKind::DraftRoot, NodeKind::NotesRoot, NodeKind::TrashRoot] {
        let count = nodes.iter().filter(|node| node.kind == kind).count();
        if count != 1 {
            return Err(bad(
                "binder.json",
                format!("there must be one “{}” item, and there are {count}", kind.as_str()),
            ));
        }
    }
    let roots: HashSet<Id> = nodes.iter().filter(|node| node.kind.is_root()).map(|node| node.id).collect();
    let mut moved = false;
    // Move one stranded item at a time: moving one item of a loop is enough
    // to break it, and leaves the rest where it was.
    loop {
        let parents: HashMap<Id, Option<Id>> = nodes.iter().map(|node| (node.id, node.parent_id)).collect();
        let reaches_a_root = |id: Id| {
            let mut at = Some(id);
            for _ in 0..=parents.len() {
                match at {
                    Some(here) if roots.contains(&here) => return parents.get(&here) == Some(&None),
                    Some(here) => at = parents.get(&here).copied().flatten(),
                    None => return false,
                }
            }
            false
        };
        let Some(stranded) = nodes.iter().position(|node| !node.kind.is_root() && !reaches_a_root(node.id)) else {
            return Ok(moved);
        };
        let folder = recovered_folder(nodes);
        let node = &mut nodes[stranded];
        node.parent_id = folder;
        problem(
            "binder.json",
            &format!("“{}” was not inside the manuscript, research, or trash; it was moved to Recovered", node.title),
        );
        moved = true;
    }
}

/// The rest of the program expects a block to hold at least one run, as the
/// editor leaves it. A stored empty paragraph has none.
/// The text of a scene file, such as one from an earlier commit, as the
/// document with the id `id`. A file whose front matter cannot be read is
/// read as text, so nothing in it is lost.
pub fn scene_document(text: &str, id: Id) -> Document {
    let blocks = match SceneFile::parse_markdown(text) {
        Ok(file) => file.blocks,
        Err(_) => writing_studio_format::markdown_to_blocks(text),
    };
    let mut document = Document::new(id);
    document.blocks = with_a_run_in_every_block(blocks);
    document
}

/// A file of the project with text, read: its front matter and its text, or
/// why its front matter could not be read.
pub(crate) fn read_scene_file(text: &str) -> std::result::Result<SceneFile, String> {
    SceneFile::parse_markdown(text).map_err(|err| format!("its details at the top could not be read ({err})"))
}

pub(crate) fn with_a_run_in_every_block(mut blocks: Vec<Block>) -> Vec<Block> {
    for block in &mut blocks {
        match &mut block.kind {
            BlockKind::Paragraph { inlines }
            | BlockKind::Heading { inlines, .. }
            | BlockKind::Quote { inlines }
            | BlockKind::Code { inlines } => {
                if inlines.is_empty() {
                    inlines.push(Inline::text(""));
                }
            }
            BlockKind::List { items, .. } => {
                for item in items {
                    if item.inlines.is_empty() {
                        item.inlines.push(Inline::text(""));
                    }
                }
            }
            BlockKind::SceneBreak => {}
        }
    }
    blocks
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn temp(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("ws-files-{tag}-{}", Id::new()))
    }

    /// Every file under a folder, by its path, except this machine's own.
    fn all_files(root: &Path) -> BTreeMap<String, String> {
        fn walk(root: &Path, folder: &Path, out: &mut BTreeMap<String, String>) {
            for entry in fs::read_dir(folder).unwrap().flatten() {
                let path = entry.path();
                let name = path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
                if path.is_dir() {
                    walk(root, &path, out);
                } else {
                    out.insert(name, fs::read_to_string(&path).unwrap_or_default());
                }
            }
        }
        let mut out = BTreeMap::new();
        walk(root, root, &mut out);
        out
    }

    /// A format 1 project with something in every part of it.
    fn sample() -> (ProjectContent, PathBuf) {
        let dir = temp("source");
        let store = ProjectStore::create(&dir, "Tide Novel").unwrap();
        let now = Timestamp::now();
        let mut tree = store.load_binder().unwrap();
        let scene = tree.nodes.iter().find(|node| node.kind == NodeKind::Scene).unwrap().clone();
        let chapter = scene.parent_id.unwrap();
        // A second scene in front of the first, a title that needs care, and
        // a scene in the trash.
        let mut first = BinderNode::new(NodeKind::Scene, "Before <Dawn> & after", Some(chapter), -1);
        first.document_id = Some(Id::new());
        first.context = "Two lines.\nOf context.\n\nAnd a second [[Mara]] paragraph.".into();
        first.label = "A".into();
        let trash = tree.nodes.iter().find(|node| node.kind == NodeKind::TrashRoot).unwrap().id;
        let mut gone = BinderNode::new(NodeKind::Scene, "Cut", Some(trash), 0);
        gone.document_id = Some(Id::new());
        gone.deleted_at = Some(now);
        gone.restore_parent_id = Some(chapter);
        tree.nodes.push(first.clone());
        tree.nodes.push(gone.clone());
        store.persist_binder(&tree).unwrap();

        let mut document = store.load_document(scene.document_id.unwrap()).unwrap();
        document.replace_html(concat!(
            "<p>Mara ran for the <strong>north </strong>quay.</p>",
            "<p><span class=\"ws-comment\" data-comment=\"which?\">The harbor</span> was quiet.</p>",
        ));
        store.save_document(&document).unwrap();

        let line = Id::new();
        store
            .persist_plot(&PlotGrid {
                lines: vec![Plotline {
                    id: line,
                    title: "Escape".into(),
                    color: "gold".into(),
                    sort_index: 0,
                    created_at: now,
                    updated_at: now,
                }],
                points: vec![PlotPoint {
                    id: Id::new(),
                    plotline_id: line,
                    title: "Missed".into(),
                    summary: "She is late.".into(),
                    scene_id: Some(scene.id),
                    sort_index: 0,
                    created_at: now,
                    updated_at: now,
                }],
            })
            .unwrap();
        let mut meta = store.load_meta().unwrap();
        meta.settings.daily_word_goal = 750;
        meta.settings.spelling.words = vec!["mara".into(), "zhul".into()];
        meta.settings.theme = writing_studio_project::Theme::Paper;
        store.persist_meta(&meta).unwrap();

        (ProjectContent::from_store(&store).unwrap(), dir)
    }

    #[test]
    fn a_project_written_as_files_reads_back_and_writes_the_same_files() {
        let (mut content, source) = sample();
        // Reading turns labels into keywords, so the files compare once that is done.
        content.binder.fold_labels_into_keywords();
        let first = temp("first");
        let placed = write_project_files(&first, &content).unwrap();
        let read = read_project_files(&first).unwrap();

        // The files say where each item is, and reading keeps those names.
        assert_eq!(read.files, placed);
        let second = temp("second");
        write_project_files(&second, &read).unwrap();
        assert_eq!(all_files(&first), all_files(&second));

        let files = all_files(&first);
        write_format_marker(&first).unwrap();
        assert_eq!(fs::read_to_string(first.join("FORMAT")).unwrap(), "writing-studio-project 5\n");
        assert_eq!(files[".gitignore"], ".local/\n");
        assert!(files.contains_key("README.md") && !files.contains_key("scene.css"));
        assert!(!files.keys().any(|name| name.ends_with(".tmp")));

        // The binder is in reading order: the scene put in front comes first.
        let titles: Vec<&str> = read
            .binder
            .nodes
            .iter()
            .filter(|node| node.kind == NodeKind::Scene)
            .map(|node| node.title.as_str())
            .collect();
        assert_eq!(titles[0], "Before <Dawn> & after");
        assert!(titles.contains(&"Cut"));
        let cut = read.binder.nodes.iter().find(|node| node.title == "Cut").unwrap();
        assert!(cut.deleted_at.is_some() && cut.restore_parent_id.is_some());
        // Its label came back as a keyword.
        let before = read.binder.nodes.iter().find(|node| node.keywords.iter().any(|word| word == "A")).unwrap();
        assert!(before.label.is_empty());
        assert_eq!(before.context, "Two lines.\nOf context.\n\nAnd a second [[Mara]] paragraph.");
        assert_eq!(before.document_id, Some(before.id));

        // Text and the note come back.
        let harbor = read
            .documents
            .iter()
            .find(|document| document.to_plain_text().contains("north quay"))
            .unwrap();
        assert_eq!(harbor.review_items().len(), 1);
        assert_eq!(read.documents.len(), content.documents.len());

        assert_eq!(read.plot.points[0].scene_id, Some(harbor.id));

        // Shared settings travel with the project; the theme stays here.
        assert_eq!(read.meta.title, "Tide Novel");
        assert_eq!(read.meta.settings.daily_word_goal, 750);
        assert_eq!(read.meta.settings.spelling.words, ["mara", "zhul"]);
        assert_eq!(read.meta.settings.theme, writing_studio_project::Theme::Paper);
        assert!(!files["project.json"].contains("theme"));
        assert!(files[".local/settings.json"].contains("paper"));
        // On another machine there is no `.local`, and the theme is the usual one.
        fs::remove_dir_all(first.join(".local")).unwrap();
        let elsewhere = read_project_files(&first).unwrap();
        assert_eq!(elsewhere.meta.settings.theme, ProjectSettings::default().theme);
        assert_eq!(elsewhere.meta.settings.daily_word_goal, 750);

        for dir in [source, first, second] {
            let _ = fs::remove_dir_all(dir);
        }
    }

    #[test]
    fn keywords_and_saved_filters_read_back_and_a_project_without_them_keeps_its_file() {
        let (mut content, source) = sample();
        // A project with no labels, which reading would turn into keywords.
        for node in &mut content.binder.nodes {
            node.label.clear();
        }
        let plain = temp("plain");
        write_project_files(&plain, &content).unwrap();
        let before = fs::read_to_string(plain.join("binder.json")).unwrap();
        assert!(!before.contains("keywords") && !before.contains("filters"));

        let scene = content.binder.nodes.iter().find(|node| node.kind == NodeKind::Scene).unwrap().id;
        let book = content.binder.nodes.iter().find(|node| node.kind == NodeKind::Book).unwrap().id;
        content.binder.add_keyword(scene, "Flashback");
        content.binder.add_keyword(book, "Draft two");
        content.binder.filters.push(SavedFilter {
            id: Id::new(),
            name: "Flashbacks".into(),
            keywords: vec!["Flashback".into(), "Draft two".into()],
            any: true,
            without: vec!["Mara".into()],
            entries: vec![Id::new()],
            status: String::new(),
            text: "harbor".into(),
        });
        let tagged = temp("tagged");
        write_project_files(&tagged, &content).unwrap();
        let read = read_project_files(&tagged).unwrap();
        assert_eq!(read.binder.get(scene).unwrap().keywords, ["Flashback"]);
        assert_eq!(read.binder.get(book).unwrap().keywords, ["Draft two"]);
        assert_eq!(read.binder.keywords, content.binder.keywords);
        assert_eq!(read.binder.filters, content.binder.filters);

        // Keywords are in each item's own file, not in the binder.
        let scene_file = tagged.join(&read.files[&scene]);
        assert!(fs::read_to_string(&scene_file).unwrap().contains("tags:\n  - Flashback\n"));
        assert!(fs::read_to_string(tagged.join(&read.files[&book])).unwrap().contains("tags:\n  - Draft two\n"));
        let path = tagged.join("binder.json");
        assert!(!fs::read_to_string(&path).unwrap().contains("\"keywords\":[\"Flashback\"]"));

        // What another machine wrote is checked: an empty or overlong
        // keyword, and a color that is not a binder color, do not come in.
        let edited = fs::read_to_string(&scene_file).unwrap().replace(
            "tags:\n  - Flashback\n",
            &format!("tags:\n  - \"  \"\n  - {}\n  - {}\n  - Flashback\n", "x".repeat(90), "y".repeat(300)),
        );
        fs::write(&scene_file, edited).unwrap();
        let colors = fs::read_to_string(&path).unwrap().replace(r#""color":"gold""#, r#""color":"chartreuse""#);
        fs::write(&path, colors).unwrap();
        let checked = read_project_files(&tagged).unwrap();
        let words = &checked.binder.get(scene).unwrap().keywords;
        assert_eq!(words.len(), 3);
        assert_eq!(words[0].chars().count(), 90, "a long keyword from elsewhere is kept whole");
        assert_eq!(words[1].chars().count(), 200, "past the sanity limit it is cut");
        assert!(checked.binder.keywords.iter().all(|keyword| keyword.color != "chartreuse"));

        // A filter from before filters took several keywords is refused,
        // with what to change; nothing is converted.
        let old = fs::read_to_string(&path).unwrap().replacen(r#""keywords":["#, r#""keyword":"Flashback","old":["#, 1);
        assert!(old.contains(r#""keyword":"Flashback""#), "{old}");
        fs::write(&path, old).unwrap();
        let refused = read_project_files(&tagged).unwrap_err().to_string();
        assert!(refused.contains("binder.json") && refused.contains("\"keywords\""), "{refused}");

        for dir in [source, plain, tagged] {
            let _ = fs::remove_dir_all(dir);
        }
    }

    #[test]
    fn a_path_that_leaves_the_project_folder_is_refused() {
        let (content, source) = sample();
        let dir = temp("escape");
        write_project_files(&dir, &content).unwrap();
        let binder = fs::read_to_string(dir.join("binder.json")).unwrap();
        for path in ["../outside.md", "/etc/passwd", "manuscript/../../x.md", ""] {
            let mut lines: Vec<String> = binder.lines().map(str::to_string).collect();
            let at = lines.iter().position(|line| line.contains("\"file\"")).unwrap();
            let start = lines[at].find("\"file\":\"").unwrap() + 8;
            let end = start + lines[at][start..].find('"').unwrap();
            lines[at].replace_range(start..end, path);
            fs::write(dir.join("binder.json"), lines.join("\n")).unwrap();
            let error = read_project_files(&dir).unwrap_err().to_string();
            assert!(error.contains("not a path inside the project"), "{path:?}: {error}");
        }
        for dir in [source, dir] {
            let _ = fs::remove_dir_all(dir);
        }
    }

    #[test]
    fn an_atomic_write_replaces_the_file_and_leaves_nothing_behind() {
        let dir = temp("atomic");
        let path = dir.join("deep").join("scene.md");
        write_atomic(&path, b"first").unwrap();
        write_atomic(&path, b"second").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "second");
        assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
        let _ = fs::remove_dir_all(dir);
    }
}

/// The longest keyword read from a file. Typing stops at 40 characters
/// (`tidy_keyword`); a file from another machine or a hand edit may hold a
/// longer one, which is kept whole up to here, so that two long keywords
/// are not cut into one. Past this it is not a keyword but a mistake.
const KEYWORD_LIMIT: usize = 200;

/// An item's keywords from its `binder.json` line: strings only, trimmed,
/// none empty, none too long.
/// The keywords in a file's front matter: `tags`, a list, or one line.
fn tags_of(header: &Header) -> Vec<String> {
    let words: Vec<String> = match (header.list("tags"), header.text("tags")) {
        (Some(list), _) => list.to_vec(),
        (None, Some(line)) => vec![line.to_string()],
        _ => Vec::new(),
    };
    words
        .into_iter()
        .map(|word| word.trim().chars().take(KEYWORD_LIMIT).collect::<String>())
        .filter(|word| !word.is_empty())
        .collect()
}

fn keywords_of(item: &Value) -> Vec<String> {
    item.get("keywords")
        .and_then(Value::as_array)
        .map(|words| {
            words
                .iter()
                .filter_map(Value::as_str)
                .map(|word| word.trim().chars().take(KEYWORD_LIMIT).collect::<String>())
                .filter(|word| !word.is_empty())
                .collect()
        })
        .unwrap_or_default()
}
