//! A project as a folder of HTML and JSON files: project format 2.
//!
//! This writes everything a project holds into the files described in
//! `documentation/PROJECT_FORMAT_2.md`, and reads them back. The desktop app
//! does not open a project this way yet; it still uses the SQLite file. This
//! is the part that the switch will stand on.
//!
//! Every file is written whole to a temporary name and then renamed over the
//! old one, so a crash during a save leaves the old file or the new one.

use std::collections::{HashMap, HashSet};
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
use writing_studio_project::{BinderNode, BinderTree, NodeKind, ProjectMeta, ProjectSettings};
use writing_studio_research::{ResearchFile, ResearchItem, ResearchLibrary};
use writing_studio_worldbuilding::{Culture, Location, LoreEntry, TimelineEvent, World};

use crate::ProjectStore;

/// The format number a folder of files carries in `FORMAT`.
pub const FILES_FORMAT_VERSION: u32 = 2;

/// Settings that every machine working on the project shares. Every other
/// setting is about one machine's window and writing habits, and stays in
/// `.local/`.
const SHARED_SETTINGS: [&str; 5] = [
    "daily_word_goal",
    "project_word_goal",
    "compile_title_page",
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
}

impl ProjectContent {
    /// Read a format 1 project out of its database. Each document is given
    /// the id of its binder item, which is how format 2 identifies text.
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
fn inside(root: &Path, file: &str, relative: &str) -> Result<PathBuf> {
    let path = Path::new(relative);
    let plain = !relative.is_empty()
        && !relative.contains('\\')
        && path.components().all(|part| matches!(part, Component::Normal(_)));
    if !plain {
        return Err(bad(file, format!("“{relative}” is not a path inside the project")));
    }
    Ok(root.join(path))
}

/// Give an item its file. A name that is taken, which needs two titles and
/// two ids that start alike, gets more of the id.
fn place(
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
fn entry_file(id: Id, title: &str, text: &str, other: Vec<(&str, HeaderValue)>) -> String {
    let mut header = Header::default();
    for (key, value) in other {
        match value {
            HeaderValue::Text(text) => header.set_text(key, &text),
            HeaderValue::List(items) => header.set_list(key, &items),
        }
    }
    SceneFile {
        header: SceneHeader {
            id: Some(id),
            title: title.to_string(),
            other: header,
            ..SceneHeader::default()
        },
        blocks: text_blocks(text),
    }
    .to_html()
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

/// Write a project into `root` as format 2 files. Files of items that are no
/// longer in the project are left where they are; nothing is deleted.
/// Returns the file each item was written to.
pub fn write_project_files(root: &Path, content: &ProjectContent) -> Result<HashMap<Id, String>> {
    let rendered = render(content)?;
    for (path, text) in &rendered.contents {
        write_atomic(&inside(root, "the project", path)?, text.as_bytes())?;
    }
    // What makes the folder readable and safe to share. The reader's guide
    // and the stylesheet are not overwritten, so they can be changed.
    for (name, text) in [("README.md", README), ("scene.css", STYLESHEET), (".gitignore", ".local/\n")] {
        if !root.join(name).exists() {
            write_atomic(&root.join(name), text.as_bytes())?;
        }
    }
    write_atomic(
        &root.join(PROJECT_FORMAT_FILE),
        format!("writing-studio-project {FILES_FORMAT_VERSION}\n").as_bytes(),
    )?;
    Ok(rendered.files)
}

/// Check that the files in `root` hold `content`: read them back and compare
/// what they would be written as with what `content` is written as. A
/// conversion runs this before it trusts the new files.
pub fn verify_project_files(root: &Path, content: &ProjectContent) -> Result<()> {
    let mut wanted = content.clone();
    let read = read_project_files(root)?;
    // File names were chosen when the files were written.
    wanted.files = read.files.clone();
    let (wanted, found) = (render(&wanted)?, render(&read)?);
    let found: HashMap<&str, &str> = found
        .contents
        .iter()
        .map(|(path, text)| (path.as_str(), text.as_str()))
        .collect();
    for (path, text) in &wanted.contents {
        if found.get(path.as_str()) != Some(&text.as_str()) {
            return Err(bad(path, "the file does not hold what the project holds"));
        }
    }
    if found.len() != wanted.contents.len() {
        return Err(Error::message("the files hold more than the project does"));
    }
    Ok(())
}

fn render(content: &ProjectContent) -> Result<Rendered> {
    let mut contents: Vec<(String, String)> = Vec::new();
    let mut files: HashMap<Id, String> = HashMap::new();
    let mut used: HashSet<String> = HashSet::new();
    let documents: HashMap<Id, &Document> = content.documents.iter().map(|doc| (doc.id, doc)).collect();

    // The manuscript, and the binder that orders it.
    let mut items = Vec::new();
    for node in in_tree_order(&content.binder) {
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
        if node.document_id.is_some() {
            let path = place(&content.files, &mut used, "manuscript", &node.title, node.id);
            let beats = content.beats.get(&node.id).cloned().unwrap_or_default();
            let scene = SceneFile {
                header: SceneHeader {
                    id: Some(node.id),
                    title: node.title.clone(),
                    synopsis: node.synopsis.clone(),
                    status: node.status.clone(),
                    label: node.label.clone(),
                    color: node.color.clone(),
                    goal: beats.goal,
                    conflict: beats.conflict,
                    outcome: beats.outcome,
                    other: Header::default(),
                },
                blocks: documents
                    .get(&node.id)
                    .map(|document| document.blocks.clone())
                    .unwrap_or_default(),
            };
            contents.push((path.clone(), scene.to_html()));
            item.insert("file".into(), json!(path));
            files.insert(node.id, path);
        } else {
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
        items.push(Value::Object(item));
    }
    contents.push(("binder.json".into(), lined(&[("items", items)])));

    // The bible: one file for each entry, and one list that orders them.
    let mut order: Vec<(&str, Vec<Value>)> = Vec::new();
    let mut links: Vec<Value> = Vec::new();
    let mut entry = |folder: &'static str,
                     list: &mut Vec<Value>,
                     id: Id,
                     title: &str,
                     scenes: &[Id],
                     html: String| {
        let path = place(&content.files, &mut used, folder, title, id);
        contents.push((path.clone(), html));
        list.push(json!({ "id": id.to_string(), "file": path }));
        links.extend(
            scenes
                .iter()
                .map(|scene| json!({ "from": id.to_string(), "scene": scene.to_string() })),
        );
        files.insert(id, path);
    };

    let mut list = Vec::new();
    for person in sorted(&content.cast.characters, |person| person.sort_index) {
        let motivations: Vec<String> = sorted(&content.cast.motivations, |item| item.sort_index)
            .into_iter()
            .filter(|item| item.character_id == person.id)
            .map(|item| item.body.clone())
            .collect();
        let html = entry_file(
            person.id,
            &person.name,
            &person.summary,
            vec![
                ("role", HeaderValue::Text(person.role.clone())),
                ("motivation", HeaderValue::List(motivations)),
            ],
        );
        entry("characters", &mut list, person.id, &person.name, &person.scene_ids, html);
    }
    order.push(("characters", std::mem::take(&mut list)));

    for place_ in sorted(&content.world.locations, |item| item.sort_index) {
        let html = entry_file(
            place_.id,
            &place_.name,
            &place_.summary,
            vec![("culture", optional_id(place_.culture_id))],
        );
        entry("places", &mut list, place_.id, &place_.name, &place_.scene_ids, html);
    }
    order.push(("places", std::mem::take(&mut list)));

    for culture in sorted(&content.world.cultures, |item| item.sort_index) {
        let html = entry_file(culture.id, &culture.name, &culture.summary, Vec::new());
        entry("cultures", &mut list, culture.id, &culture.name, &culture.scene_ids, html);
    }
    order.push(("cultures", std::mem::take(&mut list)));

    for lore in sorted(&content.world.lore, |item| item.sort_index) {
        let html = entry_file(
            lore.id,
            &lore.title,
            &lore.body,
            vec![
                ("place", optional_id(lore.location_id)),
                ("culture", optional_id(lore.culture_id)),
            ],
        );
        entry("lore", &mut list, lore.id, &lore.title, &lore.scene_ids, html);
    }
    order.push(("lore", std::mem::take(&mut list)));

    for event in sorted(&content.world.events, |item| item.sort_index) {
        let html = entry_file(
            event.id,
            &event.title,
            &event.summary,
            vec![
                ("when", HeaderValue::Text(event.when_label.clone())),
                ("place", optional_id(event.location_id)),
            ],
        );
        entry("timeline", &mut list, event.id, &event.title, &event.scene_ids, html);
    }
    order.push(("timeline", std::mem::take(&mut list)));

    for note in sorted(&content.library.items, |item| item.sort_index) {
        let html = entry_file(note.id, &note.title, &note.notes, Vec::new());
        entry("research", &mut list, note.id, &note.title, &note.scene_ids, html);
    }
    order.push(("research", std::mem::take(&mut list)));

    order.push((
        "research_files",
        content
            .library
            .files
            .iter()
            .map(|file| {
                json!({
                    "id": file.id.to_string(),
                    "note": file.item_id.to_string(),
                    "name": file.file_name,
                    "stored": file.stored_name,
                })
            })
            .collect(),
    ));
    order.push((
        "relationships",
        sorted(&content.cast.relationships, |item| item.sort_index)
            .into_iter()
            .map(|item| {
                json!({
                    "from": item.from_id.to_string(),
                    "to": item.to_id.to_string(),
                    "kind": item.kind,
                })
            })
            .collect(),
    ));
    order.push(("scene_links", links));
    contents.push(("bible.json".into(), lined(&order)));

    // The plot grid.
    let plotlines: Vec<Value> = sorted(&content.plot.lines, |line| line.sort_index)
        .into_iter()
        .map(|line| {
            let points: Vec<Value> = sorted(&content.plot.points, |point| point.sort_index)
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
                    Value::Object(item)
                })
                .collect();
            json!({
                "id": line.id.to_string(),
                "title": line.title,
                "color": line.color,
                "points": points,
            })
        })
        .collect();
    contents.push(("plot.json".into(), pretty(&json!({ "plotlines": plotlines }))?));

    // Settings: the shared ones with the project, the rest for this machine.
    let settings = serde_json::to_value(&content.meta.settings).map_err(|err| Error::message(err.to_string()))?;
    let (mut shared, mut local) = (Map::new(), Map::new());
    for (key, value) in settings.as_object().cloned().unwrap_or_default() {
        if SHARED_SETTINGS.contains(&key.as_str()) {
            shared.insert(key, value);
        } else {
            local.insert(key, value);
        }
    }
    let project = json!({
        "format": FILES_FORMAT_VERSION,
        "id": content.meta.id.to_string(),
        "title": content.meta.title,
        "created": content.meta.created_at.to_rfc3339(),
        "settings": shared,
    });
    contents.push(("project.json".into(), pretty(&project)?));
    contents.push((".local/settings.json".into(), pretty(&Value::Object(local))?));
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

impl<'a> JsonFile<'a> {
    fn read(root: &Path, name: &'a str) -> Result<Self> {
        let text = fs::read_to_string(root.join(name)).map_err(|err| bad(name, err))?;
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
fn read_entry(root: &Path, listed_in: &str, path: &str, id: Id) -> Result<SceneFile> {
    let html = fs::read_to_string(inside(root, listed_in, path)?).map_err(|err| bad(path, err))?;
    let file = SceneFile::parse_html(&html);
    match file.header.id {
        Some(found) if found != id => Err(bad(path, "the id in the file is not the one it is listed under")),
        _ => Ok(file),
    }
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

/// Read a format 2 project from its files. Times of creation and change are
/// not kept in the files, so every record reads as made now.
pub fn read_project_files(root: &Path) -> Result<ProjectContent> {
    let format = fs::read_to_string(root.join(PROJECT_FORMAT_FILE)).map_err(|err| bad(PROJECT_FORMAT_FILE, err))?;
    let version = format.split_whitespace().nth(1).and_then(|n| n.parse::<u32>().ok());
    if version != Some(FILES_FORMAT_VERSION) {
        return Err(bad(PROJECT_FORMAT_FILE, "this folder is not a format 2 project"));
    }
    let now = Timestamp::now();
    let mut files: HashMap<Id, String> = HashMap::new();

    // Settings.
    let project = JsonFile::read(root, "project.json")?;
    let mut settings = serde_json::to_value(ProjectSettings::default())
        .ok()
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default();
    let local: Value = fs::read_to_string(root.join(".local").join("settings.json"))
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
    let settings: ProjectSettings =
        serde_json::from_value(Value::Object(settings)).map_err(|err| bad("project.json", err))?;
    let created = Timestamp::parse(&text_of(&project.value, "created")).unwrap_or(now);
    let meta = ProjectMeta {
        id: project.id(&project.value, "id")?,
        format_version: FILES_FORMAT_VERSION,
        title: text_of(&project.value, "title"),
        created_at: created,
        updated_at: now,
        settings,
    };

    // The binder and the manuscript.
    let binder_file = JsonFile::read(root, "binder.json")?;
    let mut nodes = Vec::new();
    let mut documents = Vec::new();
    let mut beats = HashMap::new();
    let mut next_index: HashMap<Option<Id>, i64> = HashMap::new();
    for item in binder_file.list("items") {
        let id = binder_file.id(item, "id")?;
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
        if let Some(status) = item.get("status").and_then(Value::as_str) {
            node.status = status.to_string();
        }
        node.deleted_at = item
            .get("deleted")
            .and_then(Value::as_str)
            .map(|text| Timestamp::parse(text).unwrap_or(now));
        node.restore_parent_id = binder_file.optional_id(item, "restore_parent")?;
        node.document_id = None;
        if let Some(path) = item.get("file").and_then(Value::as_str) {
            let file = read_entry(root, "binder.json", path, id)?;
            node.title = file.header.title;
            node.synopsis = file.header.synopsis;
            node.status = file.header.status;
            node.label = file.header.label;
            node.color = file.header.color;
            node.document_id = Some(id);
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
            files.insert(id, path.to_string());
        }
        nodes.push(node);
    }

    // The bible.
    let bible = JsonFile::read(root, "bible.json")?;
    let mut scene_links: HashMap<Id, Vec<Id>> = HashMap::new();
    for link in bible.list("scene_links") {
        scene_links
            .entry(bible.id(link, "from")?)
            .or_default()
            .push(bible.id(link, "scene")?);
    }
    let mut entries = |key: &str| -> Result<Vec<(Id, i64, String, SceneFile)>> {
        let mut out = Vec::new();
        for (index, item) in bible.list(key).into_iter().enumerate() {
            let id = bible.id(item, "id")?;
            let path = text_of(item, "file");
            let file = read_entry(root, "bible.json", &path, id)?;
            files.insert(id, path.clone());
            out.push((id, index as i64, path, file));
        }
        Ok(out)
    };
    let links_of = |id: Id| scene_links.get(&id).cloned().unwrap_or_default();

    let mut cast = Cast::default();
    for (id, index, _, file) in entries("characters")? {
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
        cast.relationships.push(CharacterRelationship {
            id: Id::new(),
            from_id: bible.id(item, "from")?,
            to_id: bible.id(item, "to")?,
            kind: text_of(item, "kind"),
            sort_index: index as i64,
            created_at: now,
            updated_at: now,
        });
    }

    let mut world = World::default();
    for (id, index, path, file) in entries("places")? {
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
    for (id, index, _, file) in entries("cultures")? {
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
    for (id, index, path, file) in entries("lore")? {
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
    for (id, index, path, file) in entries("timeline")? {
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
    for (id, index, _, file) in entries("research")? {
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
    let plot_file = JsonFile::read(root, "plot.json")?;
    let mut plot = PlotGrid::default();
    for (index, line) in plot_file.list("plotlines").into_iter().enumerate() {
        let line_id = plot_file.id(line, "id")?;
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
            plot.points.push(PlotPoint {
                id: plot_file.id(point, "id")?,
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

    Ok(ProjectContent {
        meta,
        binder: BinderTree::new(nodes),
        documents,
        beats,
        plot,
        cast,
        world,
        library,
        files,
    })
}

/// The rest of the program expects a block to hold at least one run, as the
/// editor leaves it. A stored empty paragraph has none.
fn with_a_run_in_every_block(mut blocks: Vec<Block>) -> Vec<Block> {
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
        let (content, source) = sample();
        let first = temp("first");
        let placed = write_project_files(&first, &content).unwrap();
        let read = read_project_files(&first).unwrap();

        verify_project_files(&first, &content).unwrap();
        // A scene changed behind the program's back is caught.
        let (_, scene_path) = placed.iter().find(|(_, path)| path.starts_with("manuscript/")).unwrap();
        let kept = fs::read_to_string(first.join(scene_path)).unwrap();
        fs::write(first.join(scene_path), kept.replace("</body>", "<p>extra</p></body>")).unwrap();
        assert!(verify_project_files(&first, &content).is_err());
        fs::write(first.join(scene_path), kept).unwrap();

        // The files say where each item is, and reading keeps those names.
        assert_eq!(read.files, placed);
        let second = temp("second");
        write_project_files(&second, &read).unwrap();
        assert_eq!(all_files(&first), all_files(&second));

        let files = all_files(&first);
        assert_eq!(files["FORMAT"], "writing-studio-project 2\n");
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
        let before = read.binder.nodes.iter().find(|node| node.label == "A").unwrap();
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
    fn a_format_1_folder_is_not_read_as_files() {
        let (_, source) = sample();
        let error = read_project_files(&source).unwrap_err().to_string();
        assert!(error.contains("not a format 2 project"), "{error}");
        let _ = fs::remove_dir_all(source);
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
