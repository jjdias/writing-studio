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
use writing_studio_characters::{Cast, Character, CharacterRelationship, Motivation};
use writing_studio_core::{Error, Id, Result, Timestamp, PROJECT_FORMAT_FILE};
use writing_studio_diagnostics::SceneBeats;
use writing_studio_document::{Block, BlockKind, Document, Inline};
use writing_studio_format::{file_name, Header, SceneFile, SceneHeader, Value as HeaderValue};
use writing_studio_plotting::{PlotGrid, PlotPoint, Plotline};
use writing_studio_project::{
    normalize_binder_color, BinderNode, BinderTree, Keyword, NodeKind, ProjectMeta, ProjectSettings, SavedFilter,
};
use writing_studio_research::{ResearchFile, ResearchItem, ResearchLibrary};
use writing_studio_worldbuilding::{Culture, Location, LoreEntry, TimelineEvent, World};

use crate::ProjectStore;

/// The format number a folder of files carries in `FORMAT`.
pub const FILES_FORMAT_VERSION: u32 = 2;

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
const STYLESHEET: &str = include_str!("../assets/scene.css");

/// Everything a project holds, in memory.
#[derive(Clone, Debug)]
pub struct ProjectContent {
    pub meta: ProjectMeta,
    pub binder: BinderTree,
    /// The text of each binder item that has text. In format 2 a document
    /// has the id of its binder item.
    pub documents: Vec<Document>,
    pub beats: HashMap<Id, SceneBeats>,
    pub plot: PlotGrid,
    pub cast: Cast,
    pub world: World,
    pub library: ResearchLibrary,
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
    /// Names in the head of the item's own HTML file.
    pub head: Header,
    /// Keys of the item's line in `binder.json`, `bible.json`, or `plot.json`.
    /// For the project, the keys of `project.json`, and its unknown settings
    /// under `settings`.
    pub json: Map<String, Value>,
    /// Keys of the entry's lines in `scene_links` of `bible.json`, by scene.
    pub links: HashMap<Id, Map<String, Value>>,
}

impl Extras {
    fn is_empty(&self) -> bool {
        self.head.entries.is_empty() && self.json.is_empty() && self.links.is_empty()
    }
}

/// The keys of each kind of line that the program writes. Any other key is
/// kept as an extra; one of these never is, so that an extra cannot stand in
/// for what the program knows.
const BINDER_KEYS: [&str; 13] = [
    "id", "kind", "parent", "deleted", "restore_parent", "file", "status", "title", "synopsis", "label",
    "color", "keywords", "left_out",
];
const ENTRY_KEYS: [&str; 2] = ["id", "file"];
const RESEARCH_FILE_KEYS: [&str; 4] = ["id", "note", "name", "stored"];
const PLOTLINE_KEYS: [&str; 4] = ["id", "title", "color", "points"];
const POINT_KEYS: [&str; 4] = ["id", "title", "summary", "scene"];
const RELATIONSHIP_KEYS: [&str; 4] = ["id", "from", "to", "kind"];
const LINK_KEYS: [&str; 2] = ["from", "scene"];
const PROJECT_KEYS: [&str; 5] = ["format", "id", "title", "created", "settings"];
/// Names in a scene's head that the program writes. `title` is the `<title>`;
/// a `<meta name="title">` beside it is not kept.
const SCENE_HEAD: [&str; 9] = [
    "id", "title", "synopsis", "status", "label", "color", "goal", "conflict", "outcome",
];

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
            beats: store.load_scene_beats()?,
            plot: store.load_plot()?,
            cast: store.load_cast()?,
            world: store.load_world()?,
            library: store.load_library()?,
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
        path = format!("{folder}/{}-{id}.html", file_name(title, id).trim_end_matches(".html"));
    }
    used.insert(path.clone());
    path
}

fn text_blocks(text: &str) -> Vec<Block> {
    Document::from_plain_text(Id::new(), text).blocks
}

fn blocks_text(blocks: Vec<Block>) -> String {
    let mut document = Document::new(Id::new());
    document.blocks = blocks;
    document.to_plain_text()
}

/// An entry of the bible as a file: its name as the title, its text as the
/// body, and the rest as named values.
fn entry_file(
    id: Id,
    title: &str,
    text: &str,
    other: Vec<(&str, HeaderValue)>,
    extras: Option<&Extras>,
) -> SceneFile {
    let mut header = Header::default();
    let known: Vec<&str> = other.iter().map(|(key, _)| *key).collect();
    for (key, value) in other {
        match value {
            HeaderValue::Text(text) => header.set_text(key, &text),
            HeaderValue::List(items) => header.set_list(key, &items),
        }
    }
    add_head_extras(&mut header, extras, &known);
    SceneFile {
        header: SceneHeader {
            id: Some(id),
            title: title.to_string(),
            other: header,
            ..SceneHeader::default()
        },
        blocks: text_blocks(text),
    }
}

fn optional_id(id: Option<Id>) -> HeaderValue {
    HeaderValue::Text(id.map(|id| id.to_string()).unwrap_or_default())
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
    // and the stylesheet are not overwritten, so they can be changed.
    // `.gitattributes` keeps git on any system from changing line endings,
    // and from treating research files as text.
    for (name, text) in [
        ("README.md", README),
        ("scene.css", STYLESHEET),
        (".gitignore", ".local/\n"),
        (".gitattributes", "* text=auto eol=lf\nattachments/** -text\n"),
    ] {
        if !root.join(name).exists() {
            write_atomic(&root.join(name), text.as_bytes())?;
        }
    }
    Ok((rendered.files, written))
}

/// Mark the folder as a format 2 project. This is the last step of making
/// one, so that a folder that says 2 always has its files.
pub fn write_format_2_marker(root: &Path) -> Result<()> {
    write_atomic(
        &root.join(PROJECT_FORMAT_FILE),
        format!("writing-studio-project {FILES_FORMAT_VERSION}\n").as_bytes(),
    )
}

/// The file of one item with text: a scene, or a chapter written without
/// scenes.
pub(crate) fn scene_text(
    node: &BinderNode,
    beats: &SceneBeats,
    blocks: &[Block],
    extras: Option<&Extras>,
) -> String {
    let mut other = Header::default();
    add_head_extras(&mut other, extras, &[]);
    SceneFile {
        header: SceneHeader {
            id: Some(node.id),
            title: node.title.clone(),
            synopsis: node.synopsis.clone(),
            status: node.status.clone(),
            label: node.label.clone(),
            color: node.color.clone(),
            goal: beats.goal.clone(),
            conflict: beats.conflict.clone(),
            outcome: beats.outcome.clone(),
            other,
        },
        blocks: blocks.to_vec(),
    }
    .to_html()
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
        match paths.get(&node.id).filter(|_| node.document_id.is_some()) {
            Some(path) => {
                item.insert("file".into(), json!(path));
            }
            None => {
                // An item with no text of its own has nowhere else to keep these.
                item.insert("status".into(), json!(node.status));
                for (key, value) in [
                    ("title", &node.title),
                    ("synopsis", &node.synopsis),
                    ("label", &node.label),
                    ("color", &node.color),
                ] {
                    if !value.is_empty() {
                        item.insert(key.into(), json!(value));
                    }
                }
            }
        }
        if !node.keywords.is_empty() {
            item.insert("keywords".into(), json!(node.keywords));
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

/// The bible: each entry's id, file, and text, and then `bible.json`, which
/// orders them. `known` has the files entries already have; `used` gathers
/// every name that is taken.
pub(crate) fn bible_files(
    cast: &Cast,
    world: &World,
    library: &ResearchLibrary,
    known: &HashMap<Id, String>,
    used: &mut HashSet<String>,
    extras: &HashMap<Id, Extras>,
) -> (Vec<(Id, String, SceneFile)>, String) {
    let mut entries: Vec<(Id, String, SceneFile)> = Vec::new();
    // The bible: one file for each entry, and one list that orders them.
    let mut order: Vec<(&str, Vec<Value>)> = Vec::new();
    let mut links: Vec<Value> = Vec::new();
    let mut entry = |folder: &'static str,
                     list: &mut Vec<Value>,
                     id: Id,
                     title: &str,
                     scenes: &[Id],
                     file: SceneFile| {
        let path = place(known, used, folder, title, id);
        entries.push((id, path.clone(), file));
        let mut item = Map::new();
        item.insert("id".into(), json!(id.to_string()));
        item.insert("file".into(), json!(path));
        add_json_extras(&mut item, extras.get(&id), &ENTRY_KEYS);
        list.push(Value::Object(item));
        links.extend(scenes.iter().map(|scene| {
            let mut link = Map::new();
            link.insert("from".into(), json!(id.to_string()));
            link.insert("scene".into(), json!(scene.to_string()));
            let found = extras.get(&id).and_then(|found| found.links.get(scene));
            for (key, value) in found.into_iter().flatten() {
                if !LINK_KEYS.contains(&key.as_str()) {
                    link.insert(key.clone(), value.clone());
                }
            }
            Value::Object(link)
        }));
    };

    let mut list = Vec::new();
    for person in sorted(&cast.characters, |person| person.sort_index) {
        let motivations: Vec<String> = sorted(&cast.motivations, |item| item.sort_index)
            .into_iter()
            .filter(|item| item.character_id == person.id)
            .map(|item| item.body.clone())
            .collect();
        let file = entry_file(
            person.id,
            &person.name,
            &person.summary,
            vec![
                ("role", HeaderValue::Text(person.role.clone())),
                ("motivation", HeaderValue::List(motivations)),
            ],
            extras.get(&person.id),
        );
        entry("characters", &mut list, person.id, &person.name, &person.scene_ids, file);
    }
    order.push(("characters", std::mem::take(&mut list)));

    for place_ in sorted(&world.locations, |item| item.sort_index) {
        let file = entry_file(
            place_.id,
            &place_.name,
            &place_.summary,
            vec![("culture", optional_id(place_.culture_id))],
            extras.get(&place_.id),
        );
        entry("places", &mut list, place_.id, &place_.name, &place_.scene_ids, file);
    }
    order.push(("places", std::mem::take(&mut list)));

    for culture in sorted(&world.cultures, |item| item.sort_index) {
        let file = entry_file(culture.id, &culture.name, &culture.summary, Vec::new(), extras.get(&culture.id));
        entry("cultures", &mut list, culture.id, &culture.name, &culture.scene_ids, file);
    }
    order.push(("cultures", std::mem::take(&mut list)));

    for lore in sorted(&world.lore, |item| item.sort_index) {
        let file = entry_file(
            lore.id,
            &lore.title,
            &lore.body,
            vec![
                ("place", optional_id(lore.location_id)),
                ("culture", optional_id(lore.culture_id)),
            ],
            extras.get(&lore.id),
        );
        entry("lore", &mut list, lore.id, &lore.title, &lore.scene_ids, file);
    }
    order.push(("lore", std::mem::take(&mut list)));

    for event in sorted(&world.events, |item| item.sort_index) {
        let file = entry_file(
            event.id,
            &event.title,
            &event.summary,
            vec![
                ("when", HeaderValue::Text(event.when_label.clone())),
                ("place", optional_id(event.location_id)),
            ],
            extras.get(&event.id),
        );
        entry("timeline", &mut list, event.id, &event.title, &event.scene_ids, file);
    }
    order.push(("timeline", std::mem::take(&mut list)));

    for note in sorted(&library.items, |item| item.sort_index) {
        let file = entry_file(note.id, &note.title, &note.notes, Vec::new(), extras.get(&note.id));
        entry("research", &mut list, note.id, &note.title, &note.scene_ids, file);
    }
    order.push(("research", std::mem::take(&mut list)));

    order.push((
        "research_files",
        library
            .files
            .iter()
            .map(|file| {
                let mut item = Map::new();
                item.insert("id".into(), json!(file.id.to_string()));
                item.insert("note".into(), json!(file.item_id.to_string()));
                item.insert("name".into(), json!(file.file_name));
                item.insert("stored".into(), json!(file.stored_name));
                add_json_extras(&mut item, extras.get(&file.id), &RESEARCH_FILE_KEYS);
                Value::Object(item)
            })
            .collect(),
    ));
    order.push((
        "relationships",
        sorted(&cast.relationships, |item| item.sort_index)
            .into_iter()
            .map(|item| {
                let mut line = Map::new();
                line.insert("id".into(), json!(item.id.to_string()));
                line.insert("from".into(), json!(item.from_id.to_string()));
                line.insert("to".into(), json!(item.to_id.to_string()));
                line.insert("kind".into(), json!(item.kind));
                add_json_extras(&mut line, extras.get(&item.id), &RELATIONSHIP_KEYS);
                Value::Object(line)
            })
            .collect(),
    ));
    order.push(("scene_links", links));

    (entries, lined(&order))
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

    for node in content.binder.nodes.iter().filter(|node| node.document_id.is_some()) {
        let path = place(&content.files, &mut used, "manuscript", &node.title, node.id);
        if content.missing.contains(&node.id) {
            files.insert(node.id, path);
            continue;
        }
        let beats = content.beats.get(&node.id).cloned().unwrap_or_default();
        let blocks = documents.get(&node.id).map(|document| document.blocks.as_slice()).unwrap_or_default();
        contents.push((path.clone(), scene_text(node, &beats, blocks, content.extras.get(&node.id))));
        files.insert(node.id, path);
    }
    contents.push(("binder.json".into(), binder_json(&content.binder, &files, &content.extras)));

    let (entries, bible) = bible_files(
        &content.cast,
        &content.world,
        &content.library,
        &content.files,
        &mut used,
        &content.extras,
    );
    for (id, path, file) in entries {
        contents.push((path.clone(), file.to_html()));
        files.insert(id, path);
    }
    contents.push(("bible.json".into(), bible));
    contents.push(("plot.json".into(), plot_json(&content.plot, &content.extras)?));
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
    let Ok(html) = read_noted(&full, path, seen) else {
        return Ok(Err("the file is missing or could not be read".into()));
    };
    let file = SceneFile::parse_html(&html);
    Ok(match file.header.id {
        Some(found) if found != id => Err("the id in the file is not the one it is listed under".into()),
        _ => Ok(file),
    })
}

fn header_id(file: &SceneFile, path: &str, key: &str) -> Result<Option<Id>> {
    match file.header.other.text(key) {
        None => Ok(None),
        Some(text) => text
            .parse()
            .map(Some)
            .map_err(|_| bad(path, format!("“{key}” is not an id"))),
    }
}

/// Add the extra keys of the bible's lines that have no file of their own:
/// `scene_links`, kept with the entry they come from, and `relationships`.
fn with_lines(
    mut extras: HashMap<Id, Extras>,
    links: HashMap<Id, HashMap<Id, Map<String, Value>>>,
    lines: Vec<(Id, Map<String, Value>)>,
) -> HashMap<Id, Extras> {
    for (from, found) in links {
        extras.entry(from).or_default().links = found;
    }
    for (id, found) in lines.into_iter().filter(|(_, found)| !found.is_empty()) {
        extras.entry(id).or_default().json = found;
    }
    extras
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
    let mut beats = HashMap::new();
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
        node.synopsis = text_of(item, "synopsis");
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
            node.document_id = Some(id);
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
            found.head = unknown_head(&file.header.other, &[]);
            node.title = file.header.title;
            node.synopsis = file.header.synopsis;
            node.status = file.header.status;
            node.label = file.header.label;
            node.color = file.header.color;
            let scene_beats = SceneBeats {
                goal: file.header.goal,
                conflict: file.header.conflict,
                outcome: file.header.outcome,
            };
            if !scene_beats.is_empty() {
                beats.insert(id, scene_beats);
            }
            let mut document = Document::new(id);
            document.blocks = with_a_run_in_every_block(file.blocks);
            documents.push(document);
        }
        keep(id, found);
        nodes.push(node);
    }

    // Scene files no line of the binder names: added by hand, or by git.
    for (path, file) in unlisted(root, "manuscript", &files, &seen, &mut problem)? {
        let id = file.header.id.filter(|id| !ids.contains(id)).unwrap_or_else(Id::new);
        ids.insert(id);
        let parent = recovered_folder(&mut nodes);
        let fallback = path.rsplit('/').next().unwrap_or(&path).trim_end_matches(".html").to_string();
        let title = if file.header.title.trim().is_empty() { fallback } else { file.header.title.clone() };
        let mut node = BinderNode::new(NodeKind::Scene, title, parent, i64::MAX);
        node.id = id;
        node.document_id = Some(id);
        node.synopsis = file.header.synopsis.clone();
        node.label = file.header.label.clone();
        node.color = file.header.color.clone();
        if !file.header.status.is_empty() {
            node.status = file.header.status.clone();
        }
        let scene_beats = SceneBeats {
            goal: file.header.goal.clone(),
            conflict: file.header.conflict.clone(),
            outcome: file.header.outcome.clone(),
        };
        if !scene_beats.is_empty() {
            beats.insert(id, scene_beats);
        }
        let mut document = Document::new(id);
        document.blocks = with_a_run_in_every_block(file.blocks);
        documents.push(document);
        nodes.push(node);
        files.insert(id, path.clone());
        problem(&path, "was not in the binder; it was added to Recovered");
        repaired = true;
    }
    if check_binder(&mut nodes, &mut problem)? {
        repaired = true;
    }

    // The bible.
    let bible = JsonFile::read(root, "bible.json", &seen)?;
    let mut scene_links: HashMap<Id, Vec<Id>> = HashMap::new();
    let mut link_extras: HashMap<Id, HashMap<Id, Map<String, Value>>> = HashMap::new();
    let mut line_extras: Vec<(Id, Map<String, Value>)> = Vec::new();
    for link in bible.list("scene_links") {
        let (from, scene) = (bible.id(link, "from")?, bible.id(link, "scene")?);
        scene_links.entry(from).or_default().push(scene);
        let unknown = unknown_keys(link, &LINK_KEYS);
        if !unknown.is_empty() {
            link_extras.entry(from).or_default().insert(scene, unknown);
        }
    }
    // `head` is the names the program reads from this kind of entry's head.
    let mut entries = |key: &str, head: &[&str]| -> Result<Vec<(Id, i64, String, SceneFile)>> {
        let mut out = Vec::new();
        for (index, item) in bible.list(key).into_iter().enumerate() {
            let id = bible.id(item, "id")?;
            let path = text_of(item, "file");
            if !ids.insert(id) {
                problem("bible.json", &format!("entry {id} was listed twice; the second line was left out"));
                repaired = true;
                continue;
            }
            let file = match read_entry(root, "bible.json", &path, id, &seen)? {
                Ok(file) => file,
                Err(why) => {
                    // Taken out of the list. A file that is still there comes
                    // back below, as a file no list names.
                    problem(&path, &format!("{why}; it was taken out of the bible"));
                    repaired = true;
                    continue;
                }
            };
            files.insert(id, path.clone());
            keep(
                id,
                Extras {
                    head: unknown_head(&file.header.other, head),
                    json: unknown_keys(item, &ENTRY_KEYS),
                    ..Extras::default()
                },
            );
            out.push((id, index as i64, path, file));
        }
        for (path, file) in unlisted(root, key, &files, &seen, &mut problem)? {
            let id = file.header.id.filter(|id| !ids.contains(id)).unwrap_or_else(Id::new);
            ids.insert(id);
            files.insert(id, path.clone());
            keep(
                id,
                Extras {
                    head: unknown_head(&file.header.other, head),
                    ..Extras::default()
                },
            );
            problem(&path, "was not in bible.json; it was added");
            repaired = true;
            out.push((id, out.len() as i64, path, file));
        }
        Ok(out)
    };
    let links_of = |id: Id| scene_links.get(&id).cloned().unwrap_or_default();

    let mut cast = Cast::default();
    for (id, index, _, file) in entries("characters", &["role", "motivation"])? {
        for (at, body) in file.header.other.list("motivation").unwrap_or_default().iter().enumerate() {
            cast.motivations.push(Motivation {
                id: Id::new(),
                character_id: id,
                body: body.clone(),
                sort_index: at as i64,
                created_at: now,
                updated_at: now,
            });
        }
        // One motivation is written as a single value, not a list.
        if let Some(body) = file.header.other.text("motivation") {
            cast.motivations.push(Motivation {
                id: Id::new(),
                character_id: id,
                body: body.to_string(),
                sort_index: 0,
                created_at: now,
                updated_at: now,
            });
        }
        cast.characters.push(Character {
            id,
            name: file.header.title.clone(),
            role: file.header.other.text("role").unwrap_or_default().to_string(),
            summary: blocks_text(file.blocks),
            sort_index: index,
            scene_ids: links_of(id),
            created_at: now,
            updated_at: now,
        });
    }
    for (index, item) in bible.list("relationships").into_iter().enumerate() {
        // Lines written before relationships had ids get one now.
        let id = bible.optional_id(item, "id")?.unwrap_or_else(Id::new);
        line_extras.push((id, unknown_keys(item, &RELATIONSHIP_KEYS)));
        cast.relationships.push(CharacterRelationship {
            id,
            from_id: bible.id(item, "from")?,
            to_id: bible.id(item, "to")?,
            kind: text_of(item, "kind"),
            sort_index: index as i64,
            created_at: now,
            updated_at: now,
        });
    }

    let mut world = World::default();
    for (id, index, path, file) in entries("places", &["culture"])? {
        world.locations.push(Location {
            id,
            name: file.header.title.clone(),
            culture_id: header_id(&file, &path, "culture")?,
            summary: blocks_text(file.blocks),
            sort_index: index,
            scene_ids: links_of(id),
            created_at: now,
            updated_at: now,
        });
    }
    for (id, index, _, file) in entries("cultures", &[])? {
        world.cultures.push(Culture {
            id,
            name: file.header.title.clone(),
            summary: blocks_text(file.blocks),
            sort_index: index,
            scene_ids: links_of(id),
            created_at: now,
            updated_at: now,
        });
    }
    for (id, index, path, file) in entries("lore", &["place", "culture"])? {
        world.lore.push(LoreEntry {
            id,
            title: file.header.title.clone(),
            location_id: header_id(&file, &path, "place")?,
            culture_id: header_id(&file, &path, "culture")?,
            body: blocks_text(file.blocks),
            sort_index: index,
            scene_ids: links_of(id),
            created_at: now,
            updated_at: now,
        });
    }
    for (id, index, path, file) in entries("timeline", &["when", "place"])? {
        world.events.push(TimelineEvent {
            id,
            title: file.header.title.clone(),
            when_label: file.header.other.text("when").unwrap_or_default().to_string(),
            location_id: header_id(&file, &path, "place")?,
            summary: blocks_text(file.blocks),
            sort_index: index,
            scene_ids: links_of(id),
            created_at: now,
            updated_at: now,
        });
    }

    let mut library = ResearchLibrary::default();
    for (id, index, _, file) in entries("research", &[])? {
        library.items.push(ResearchItem {
            id,
            title: file.header.title.clone(),
            notes: blocks_text(file.blocks),
            sort_index: index,
            scene_ids: links_of(id),
            created_at: now,
            updated_at: now,
        });
    }
    for item in bible.list("research_files") {
        let stored = text_of(item, "stored");
        keep(
            bible.id(item, "id")?,
            Extras {
                json: unknown_keys(item, &RESEARCH_FILE_KEYS),
                ..Extras::default()
            },
        );
        // The name is used to open a file under attachments/.
        inside(&root.join("attachments"), "bible.json", &stored)?;
        library.files.push(ResearchFile {
            id: bible.id(item, "id")?,
            item_id: bible.id(item, "note")?,
            file_name: text_of(item, "name"),
            stored_name: stored,
            created_at: now,
        });
    }

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
        beats,
        plot,
        cast,
        world,
        library,
        files,
        extras: with_lines(extras, link_extras, line_extras),
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
        .filter(|name| name.ends_with(".html") && !name.starts_with('.') && !name.contains(".conflict-"))
        .map(|name| format!("{folder}/{name}"))
        .filter(|path| !listed.contains(path))
        .collect();
    names.sort();
    Ok(names
        .into_iter()
        .filter_map(|path| match read_noted(&root.join(&path), &path, seen) {
            Ok(text) => Some((path, SceneFile::parse_html(&text))),
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
    for kind in [NodeKind::DraftRoot, NodeKind::ResearchRoot, NodeKind::TrashRoot] {
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
/// document with the id `id`.
pub fn scene_document(html: &str, id: Id) -> Document {
    let mut document = Document::new(id);
    document.blocks = with_a_run_in_every_block(SceneFile::parse_html(html).blocks);
    document
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
        first.synopsis = "Two lines.\nOf synopsis.".into();
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
        let mut beats = HashMap::new();
        beats.insert(
            scene.id,
            SceneBeats {
                goal: "Reach the boat".into(),
                conflict: String::new(),
                outcome: "It has gone".into(),
            },
        );
        store.persist_scene_beats(&beats).unwrap();

        let (mara, pilot, quay, guild) = (Id::new(), Id::new(), Id::new(), Id::new());
        let person = |id, name: &str, index| Character {
            id,
            name: name.into(),
            role: "lead".into(),
            summary: "Late.\n\nAlways late.".into(),
            sort_index: index,
            scene_ids: vec![scene.id],
            created_at: now,
            updated_at: now,
        };
        let motive = |body: &str, index| Motivation {
            id: Id::new(),
            character_id: mara,
            body: body.into(),
            sort_index: index,
            created_at: now,
            updated_at: now,
        };
        store
            .persist_cast(&Cast {
                characters: vec![person(pilot, "The Pilot", 1), person(mara, "Mara Vance", 0)],
                motivations: vec![motive("Leave", 0), motive("Be forgiven", 1)],
                relationships: vec![CharacterRelationship {
                    id: Id::new(),
                    from_id: mara,
                    to_id: pilot,
                    kind: "owes".into(),
                    sort_index: 0,
                    created_at: now,
                    updated_at: now,
                }],
            })
            .unwrap();
        store
            .persist_world(&World {
                locations: vec![Location {
                    id: quay,
                    name: "North Quay".into(),
                    summary: "Stone.".into(),
                    culture_id: Some(guild),
                    sort_index: 0,
                    scene_ids: vec![scene.id],
                    created_at: now,
                    updated_at: now,
                }],
                cultures: vec![Culture {
                    id: guild,
                    name: "Pilots' Guild".into(),
                    summary: String::new(),
                    sort_index: 0,
                    scene_ids: Vec::new(),
                    created_at: now,
                    updated_at: now,
                }],
                lore: vec![LoreEntry {
                    id: Id::new(),
                    title: "Tide tables".into(),
                    body: "A promise.".into(),
                    location_id: Some(quay),
                    culture_id: None,
                    sort_index: 0,
                    scene_ids: Vec::new(),
                    created_at: now,
                    updated_at: now,
                }],
                events: vec![TimelineEvent {
                    id: Id::new(),
                    title: "The boat leaves".into(),
                    when_label: "Dawn".into(),
                    summary: String::new(),
                    location_id: Some(quay),
                    sort_index: 0,
                    scene_ids: Vec::new(),
                    created_at: now,
                    updated_at: now,
                }],
            })
            .unwrap();
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
        write_format_2_marker(&first).unwrap();
        assert_eq!(fs::read_to_string(first.join("FORMAT")).unwrap(), "writing-studio-project 2\n");
        assert_eq!(files[".gitignore"], ".local/\n");
        assert!(files.contains_key("README.md") && files.contains_key("scene.css"));
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
        assert_eq!(before.synopsis, "Two lines.\nOf synopsis.");
        assert_eq!(before.document_id, Some(before.id));

        // Text, beats, and the note come back.
        let harbor = read
            .documents
            .iter()
            .find(|document| document.to_plain_text().contains("north quay"))
            .unwrap();
        assert_eq!(harbor.review_items().len(), 1);
        assert_eq!(read.beats[&harbor.id].outcome, "It has gone");
        assert_eq!(read.documents.len(), content.documents.len());

        // The bible, in its order, with its links.
        let names: Vec<&str> = read.cast.characters.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["Mara Vance", "The Pilot"]);
        assert_eq!(read.cast.characters[0].summary, "Late.\n\nAlways late.");
        assert_eq!(read.cast.characters[0].scene_ids, vec![harbor.id]);
        let motives: Vec<&str> = read.cast.motivations.iter().map(|m| m.body.as_str()).collect();
        assert_eq!(motives, ["Leave", "Be forgiven"]);
        assert_eq!(read.cast.relationships[0].kind, "owes");
        assert_eq!(read.world.locations[0].culture_id, Some(read.world.cultures[0].id));
        assert_eq!(read.world.lore[0].location_id, Some(read.world.locations[0].id));
        assert_eq!(read.world.events[0].when_label, "Dawn");
        assert_eq!(read.plot.points[0].scene_id, Some(harbor.id));
        assert_eq!(read.library.items.len(), content.library.items.len());

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
            keyword: "Flashback".into(),
            status: String::new(),
            label: String::new(),
            text: "harbor".into(),
        });
        let tagged = temp("tagged");
        write_project_files(&tagged, &content).unwrap();
        let read = read_project_files(&tagged).unwrap();
        assert_eq!(read.binder.get(scene).unwrap().keywords, ["Flashback"]);
        assert_eq!(read.binder.get(book).unwrap().keywords, ["Draft two"]);
        assert_eq!(read.binder.keywords, content.binder.keywords);
        assert_eq!(read.binder.filters, content.binder.filters);

        // What another machine wrote is checked: an empty or overlong
        // keyword, and a color that is not a binder color, do not come in.
        let path = tagged.join("binder.json");
        let edited = fs::read_to_string(&path)
            .unwrap()
            .replace(r#""keywords":["Flashback"]"#, &format!(r#""keywords":["  ","{}","Flashback"]"#, "x".repeat(90)))
            .replace(r#""color":"gold""#, r#""color":"chartreuse""#);
        fs::write(&path, edited).unwrap();
        let checked = read_project_files(&tagged).unwrap();
        let words = &checked.binder.get(scene).unwrap().keywords;
        assert_eq!(words.len(), 2);
        assert!(words.iter().all(|word| !word.trim().is_empty() && word.chars().count() <= 40));
        assert!(checked.binder.keywords.iter().all(|keyword| keyword.color != "chartreuse"));

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
        for path in ["../outside.html", "/etc/passwd", "manuscript/../../x.html", ""] {
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
        let path = dir.join("deep").join("scene.html");
        write_atomic(&path, b"first").unwrap();
        write_atomic(&path, b"second").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "second");
        assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
        let _ = fs::remove_dir_all(dir);
    }
}

/// The longest keyword read from a file, as the binder keeps them.
const KEYWORD_LIMIT: usize = 40;

/// An item's keywords from its `binder.json` line: strings only, trimmed,
/// none empty, none too long.
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
