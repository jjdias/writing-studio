//! What a scene needs around it, in one place: the scene, where it sits in
//! the book and what comes before and after, its plot points, and every
//! character, place, and note linked to it, directly or through the chapter,
//! part, or book it is in.
//!
//! It reads the project's own models and nothing else (not the window, not
//! the files), so anything added later can ask for a scene's context with a
//! project's data: the inspector shows it, and `to_markdown` is what an
//! assistant would read. Nothing here sends anything anywhere.

use writing_studio_characters::Cast;
use writing_studio_core::Id;
use writing_studio_plotting::PlotGrid;
use writing_studio_project::{BinderTree, NodeKind};
use writing_studio_research::ResearchLibrary;
use writing_studio_worldbuilding::World;

/// The kinds of entry a binder item can be linked to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryKind {
    Character,
    Location,
    Culture,
    Lore,
    Timeline,
    Research,
}

impl EntryKind {
    /// What an entry of this kind is, in a chip or a list: "character".
    pub fn noun(self) -> &'static str {
        match self {
            Self::Character => "character",
            Self::Location => "place",
            Self::Culture => "culture",
            Self::Lore => "lore",
            Self::Timeline => "event",
            Self::Research => "research",
        }
    }

    /// The heading over this kind's entries.
    pub fn heading(self) -> &'static str {
        match self {
            Self::Character => "Characters",
            Self::Location => "Places",
            Self::Culture => "Cultures",
            Self::Lore => "Lore",
            Self::Timeline => "Events",
            Self::Research => "Research",
        }
    }
}

/// A character, place, culture, lore entry, timeline event, or research
/// note, with what it says and the binder items it is linked to: scenes, and
/// chapters, parts, or books, whose link reaches every scene in them.
#[derive(Clone, Debug, PartialEq)]
pub struct StoryEntry {
    pub kind: EntryKind,
    pub id: Id,
    pub name: String,
    /// Its own words: a character's role and summary, a place's or
    /// culture's summary, lore's body, an event's time and summary, a
    /// research note's notes.
    pub text: String,
    pub links: Vec<Id>,
    /// The entry of its kind it is inside, if any.
    pub parent: Option<Id>,
    /// What it is, in the writer's words: "Country", "Father".
    pub label: String,
}

/// Every entry of the story, characters first, each kind in its own order.
/// An entry with no name is called by its kind: "Unnamed character".
pub fn story_entries(cast: &Cast, world: &World, library: &ResearchLibrary) -> Vec<StoryEntry> {
    let mut entries = Vec::new();
    let mut push = |kind: EntryKind, id: Id, name: &str, parts: &[&str], links: &[Id], parent: Option<Id>, label: &str| {
        let name = if name.trim().is_empty() { format!("Unnamed {}", kind.noun()) } else { name.trim().to_string() };
        let text = parts.iter().map(|part| part.trim()).filter(|part| !part.is_empty()).collect::<Vec<_>>().join("\n\n");
        entries.push(StoryEntry { kind, id, name, text, links: links.to_vec(), parent, label: label.trim().to_string() });
    };
    for item in cast.characters_in_order() {
        push(EntryKind::Character, item.id, &item.name, &[&item.role, &item.summary], &item.linked_ids, item.parent_id, &item.label);
    }
    for item in world.locations_in_order() {
        push(EntryKind::Location, item.id, &item.name, &[&item.summary], &item.linked_ids, item.parent_id, &item.label);
    }
    for item in world.cultures_in_order() {
        push(EntryKind::Culture, item.id, &item.name, &[&item.summary], &item.linked_ids, item.parent_id, &item.label);
    }
    for item in world.lore_in_order() {
        push(EntryKind::Lore, item.id, &item.title, &[&item.body], &item.linked_ids, item.parent_id, &item.label);
    }
    for item in world.events_in_order() {
        push(EntryKind::Timeline, item.id, &item.title, &[&item.when_label, &item.summary], &item.linked_ids, item.parent_id, &item.label);
    }
    for item in library.items_in_order() {
        push(EntryKind::Research, item.id, &item.title, &[&item.notes], &item.linked_ids, item.parent_id, &item.label);
    }
    entries
}

/// How two entries are connected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tie {
    /// A connection the writer named ("lives in"); its relationship's id.
    Named(Id),
    /// One is inside the other in its kind's tree.
    Inside,
    /// A field of one names the other: a place's culture, lore's place or
    /// culture, an event's place.
    Field,
}

/// A connection from one entry to another, with its name.
#[derive(Clone, Debug, PartialEq)]
pub struct Connection {
    pub from: Id,
    pub to: Id,
    pub name: String,
    pub tie: Tie,
}

/// Every connection between the story's entries: named ones, being inside
/// another, and the fields that name another entry.
pub fn connections(cast: &Cast, world: &World, library: &ResearchLibrary) -> Vec<Connection> {
    let mut found = Vec::new();
    let mut tie = |from: Id, to: Id, name: &str, tie: Tie| {
        found.push(Connection { from, to, name: name.to_string(), tie });
    };
    for item in &cast.relationships {
        let name = if item.kind.trim().is_empty() { "connected to" } else { item.kind.trim() };
        tie(item.from_id, item.to_id, name, Tie::Named(item.id));
    }
    for entry in story_entries(cast, world, library) {
        if let Some(parent) = entry.parent {
            tie(entry.id, parent, "inside", Tie::Inside);
        }
    }
    for place in &world.locations {
        if let Some(culture) = place.culture_id {
            tie(place.id, culture, "of the culture", Tie::Field);
        }
    }
    for lore in &world.lore {
        if let Some(place) = lore.location_id {
            tie(lore.id, place, "about the place", Tie::Field);
        }
        if let Some(culture) = lore.culture_id {
            tie(lore.id, culture, "about the culture", Tie::Field);
        }
    }
    for event in &world.events {
        if let Some(place) = event.location_id {
            tie(event.id, place, "takes place in", Tie::Field);
        }
    }
    found
}

/// The item a link reaches `id` through: `id` itself, or the nearest
/// chapter, part, or book it is in that the entry is linked to.
pub fn reached_through(binder: &BinderTree, entry: &StoryEntry, id: Id) -> Option<Id> {
    let mut at = Some(id);
    let mut guard = 0;
    while let Some(node) = at.and_then(|id| binder.get(id)) {
        if entry.links.contains(&node.id) {
            return Some(node.id);
        }
        at = node.parent_id;
        guard += 1;
        if guard > 10_000 {
            break;
        }
    }
    None
}

/// Whether an entry reaches a binder item, directly or through what it is in.
pub fn reaches(binder: &BinderTree, entry: &StoryEntry, id: Id) -> bool {
    reached_through(binder, entry, id).is_some()
}

/// An entry that nests within its kind: a city inside a country, a son
/// under his father. What every kind of entry shares for its tree.
pub trait Nested {
    fn id(&self) -> Id;
    fn parent(&self) -> Option<Id>;
    fn set_parent(&mut self, parent: Option<Id>);
    fn sort_index(&self) -> i64;
    fn set_sort_index(&mut self, index: i64);
}

macro_rules! nested {
    ($($kind:ty),*) => {$(
        impl Nested for $kind {
            fn id(&self) -> Id { self.id }
            fn parent(&self) -> Option<Id> { self.parent_id }
            fn set_parent(&mut self, parent: Option<Id>) { self.parent_id = parent; }
            fn sort_index(&self) -> i64 { self.sort_index }
            fn set_sort_index(&mut self, index: i64) { self.sort_index = index; }
        }
    )*};
}

nested!(
    writing_studio_characters::Character,
    writing_studio_worldbuilding::Location,
    writing_studio_worldbuilding::Culture,
    writing_studio_worldbuilding::LoreEntry,
    writing_studio_worldbuilding::TimelineEvent,
    writing_studio_research::ResearchItem
);

/// The entries directly inside `parent` (the top for `None`), in order. An
/// entry whose parent is gone is at the top.
pub fn children_of<T: Nested>(items: &[T], parent: Option<Id>) -> Vec<Id> {
    let known = |id: Id| items.iter().any(|item| item.id() == id);
    let mut found: Vec<&T> = items
        .iter()
        .filter(|item| match item.parent() {
            Some(up) if known(up) => Some(up) == parent,
            _ => parent.is_none(),
        })
        .collect();
    found.sort_by_key(|item| item.sort_index());
    found.into_iter().map(|item| item.id()).collect()
}

/// A kind's entries as tree rows, each with its depth and whether it holds
/// others, leaving out what is inside one in `folded`.
pub fn tree_rows<T: Nested>(items: &[T], folded: &[Id]) -> Vec<(Id, u32, bool)> {
    fn walk<T: Nested>(items: &[T], parent: Option<Id>, depth: u32, folded: &[Id], rows: &mut Vec<(Id, u32, bool)>) {
        for id in children_of(items, parent) {
            let held = !children_of(items, Some(id)).is_empty();
            rows.push((id, depth, held));
            if held && !folded.contains(&id) && depth < 64 {
                walk(items, Some(id), depth + 1, folded, rows);
            }
        }
    }
    let mut rows = Vec::new();
    walk(items, None, 0, folded, &mut rows);
    rows
}

/// Whether `id` is `ancestor` or inside it.
pub fn is_within<T: Nested>(items: &[T], id: Id, ancestor: Id) -> bool {
    let mut at = Some(id);
    let mut guard = 0;
    while let Some(here) = at {
        if here == ancestor {
            return true;
        }
        at = items.iter().find(|item| item.id() == here).and_then(|item| item.parent());
        guard += 1;
        if guard > 10_000 {
            break;
        }
    }
    false
}

/// Put an entry inside `parent`, after `after` among what it holds (first
/// with `None`). An entry cannot go inside itself or what it holds.
/// Returns whether anything moved.
pub fn move_entry<T: Nested>(items: &mut [T], id: Id, parent: Option<Id>, after: Option<Id>) -> bool {
    if parent.is_some_and(|parent| is_within(items, parent, id)) || after == Some(id) {
        return false;
    }
    let mut order: Vec<Id> = children_of(items, parent).into_iter().filter(|other| *other != id).collect();
    let at = after.and_then(|after| order.iter().position(|other| *other == after)).map(|at| at + 1).unwrap_or(0);
    order.insert(at.min(order.len()), id);
    for item in items.iter_mut() {
        if item.id() == id {
            item.set_parent(parent);
        }
        if let Some(index) = order.iter().position(|other| *other == item.id()) {
            item.set_sort_index(index as i64);
        }
    }
    true
}

/// What a scene's context is read from: the project's models.
#[derive(Clone, Copy)]
pub struct Sources<'a> {
    pub binder: &'a BinderTree,
    pub cast: &'a Cast,
    pub world: &'a World,
    pub library: &'a ResearchLibrary,
    pub plot: &'a PlotGrid,
}

/// The scene before or after, by its title and synopsis.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Neighbor {
    pub title: String,
    pub synopsis: String,
}

/// A plot point that points at the scene, with its plotline.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlotPointNote {
    pub line: String,
    pub title: String,
    pub summary: String,
}

/// An entry as the scene's context holds it.
#[derive(Clone, Debug, PartialEq)]
pub struct ContextEntry {
    pub kind: EntryKind,
    pub id: Id,
    pub name: String,
    pub text: String,
    /// The chapter, part, or book whose link brings it, or `None` when the
    /// scene itself is linked to it.
    pub through: Option<String>,
}

/// Everything a scene needs, in the order it is read.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneContext {
    pub title: String,
    pub synopsis: String,
    pub status: String,
    /// What the writer wants to remember about it.
    pub notes: String,
    /// Its keywords, with those of the chapter, part, or book it is in.
    pub keywords: Vec<String>,
    /// Where it is, from the book down: "Book: Tide", "Chapter: Chapter 1".
    pub place: Vec<String>,
    pub before: Option<Neighbor>,
    pub after: Option<Neighbor>,
    pub plot_points: Vec<PlotPointNote>,
    /// Characters first, then the other kinds, each in its own order.
    pub entries: Vec<ContextEntry>,
}

/// An item's keywords with those of what it is in, each once.
fn keywords_of(binder: &BinderTree, id: Id) -> Vec<String> {
    let mut keywords: Vec<String> = Vec::new();
    for word in binder.keywords_through(id) {
        if !keywords.iter().any(|kept| writing_studio_project::same_keyword(kept, word)) {
            keywords.push(word.to_string());
        }
    }
    keywords
}

/// Where an item is, from the top down: "Book: Tide", "Chapter: Chapter 1".
fn place_of(binder: &BinderTree, id: Id) -> Vec<String> {
    let mut place = Vec::new();
    let mut at = binder.get(id).and_then(|node| node.parent_id);
    while let Some(up) = at.and_then(|id| binder.get(id)) {
        if up.kind.is_root() || place.len() > 64 {
            break;
        }
        place.push(format!("{}: {}", up.kind.icon_label(), up.title));
        at = up.parent_id;
    }
    place.reverse();
    place
}

/// A scene's context, or `None` when the id is not a scene in the binder.
pub fn scene_context(sources: &Sources<'_>, scene: Id) -> Option<SceneContext> {
    let binder = sources.binder;
    let node = binder.get(scene).filter(|node| node.kind == NodeKind::Scene)?;
    let keywords = keywords_of(binder, scene);
    let place = place_of(binder, scene);

    // The scenes before and after, in the book's reading order.
    let order: Vec<Id> = binder
        .book_of(scene)
        .map(|book| binder.scenes_under(book).into_iter().map(|(_, id)| id).collect())
        .unwrap_or_default();
    let neighbor = |id: Option<&Id>| {
        id.and_then(|id| binder.get(*id))
            .map(|node| Neighbor { title: node.title.clone(), synopsis: node.synopsis.trim().to_string() })
    };
    let at = order.iter().position(|id| *id == scene);
    let before = at.and_then(|at| at.checked_sub(1)).and_then(|at| neighbor(order.get(at)));
    let after = at.and_then(|at| neighbor(order.get(at + 1)));

    let mut plot_points = Vec::new();
    for line in sources.plot.lines_in_order() {
        let mut points: Vec<_> = sources
            .plot
            .points
            .iter()
            .filter(|point| point.plotline_id == line.id && point.scene_id == Some(scene))
            .collect();
        points.sort_by_key(|point| point.sort_index);
        plot_points.extend(points.into_iter().map(|point| PlotPointNote {
            line: line.title.clone(),
            title: point.title.clone(),
            summary: point.summary.trim().to_string(),
        }));
    }

    let entries = story_entries(sources.cast, sources.world, sources.library)
        .into_iter()
        .filter_map(|entry| {
            let through = reached_through(binder, &entry, scene)?;
            Some(ContextEntry {
                kind: entry.kind,
                id: entry.id,
                name: entry.name,
                text: entry.text,
                through: (through != scene).then(|| binder.get(through).map(|node| node.title.clone()).unwrap_or_default()),
            })
        })
        .collect();

    Some(SceneContext {
        title: node.title.clone(),
        synopsis: node.synopsis.trim().to_string(),
        status: node.status.trim().to_string(),
        notes: node.notes.trim().to_string(),
        keywords,
        place,
        before,
        after,
        plot_points,
        entries,
    })
}

impl SceneContext {
    /// The context as plain Markdown, at most `limit` characters. When it is
    /// longer, the entries' text goes first, from the end of the list, and
    /// the last line says how many were shortened; if that is not enough,
    /// the end is cut, and that is said too.
    pub fn to_markdown(&self, limit: usize) -> String {
        let mut kept_text = self.entries.len();
        loop {
            let text = self.render(kept_text);
            let dropped = self.entries.len() - kept_text;
            let note = match dropped {
                0 => String::new(),
                1 => format!("\n_The text of 1 entry was left out, to keep this under {limit} characters._\n"),
                many => format!("\n_The text of {many} entries was left out, to keep this under {limit} characters._\n"),
            };
            if text.chars().count() + note.chars().count() <= limit {
                return text + &note;
            }
            if kept_text == 0 {
                let cut_note = format!("\n_Cut here, to keep this under {limit} characters._\n");
                let room = limit.saturating_sub(cut_note.chars().count());
                let cut: String = text.chars().take(room).collect();
                return cut + &cut_note;
            }
            kept_text -= 1;
        }
    }

    /// The Markdown, with the text of the first `kept_text` entries.
    fn render(&self, kept_text: usize) -> String {
        let mut out = format!("# {}\n\n", self.title.trim());
        let mut facts = Vec::new();
        if !self.status.is_empty() {
            facts.push(format!("Status: {}", self.status));
        }
        if !self.keywords.is_empty() {
            facts.push(format!("Keywords: {}", self.keywords.join(", ")));
        }
        if !self.place.is_empty() {
            facts.push(self.place.join(" › "));
        }
        for fact in facts {
            out.push_str(&format!("{fact}  \n"));
        }
        out.push('\n');
        if !self.synopsis.is_empty() {
            out.push_str(&format!("## Synopsis\n\n{}\n\n", self.synopsis));
        }
        if !self.notes.is_empty() {
            out.push_str(&format!("## Notes\n\n{}\n\n", self.notes));
        }
        for (heading, neighbor) in [("Before", &self.before), ("After", &self.after)] {
            if let Some(neighbor) = neighbor {
                out.push_str(&format!("## {heading}\n\n**{}**", neighbor.title.trim()));
                if !neighbor.synopsis.is_empty() {
                    out.push_str(&format!(": {}", neighbor.synopsis));
                }
                out.push_str("\n\n");
            }
        }
        if !self.plot_points.is_empty() {
            out.push_str("## Plot points\n\n");
            for point in &self.plot_points {
                out.push_str(&format!("- **{}**, {}", point.line.trim(), point.title.trim()));
                if !point.summary.is_empty() {
                    out.push_str(&format!(": {}", point.summary));
                }
                out.push('\n');
            }
            out.push('\n');
        }
        let mut heading = None;
        for (index, entry) in self.entries.iter().enumerate() {
            if heading != Some(entry.kind) {
                out.push_str(&format!("## {}\n\n", entry.kind.heading()));
                heading = Some(entry.kind);
            }
            out.push_str(&format!("### {}\n\n", entry.name));
            if let Some(from) = &entry.through {
                out.push_str(&format!("_Through {from}._\n\n"));
            }
            if index < kept_text && !entry.text.is_empty() {
                out.push_str(&format!("{}\n\n", entry.text));
            }
        }
        out.trim_end().to_string() + "\n"
    }
}

// ---- The context of a chapter, a part, a book, or a folder ----

/// One item a level holds, as its outline shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutlineItem {
    /// 0 for what the level holds directly, 1 for what that holds.
    pub depth: usize,
    pub kind: NodeKind,
    pub title: String,
    pub status: String,
    pub synopsis: String,
}

/// An entry linked to the level or to anything in it.
#[derive(Clone, Debug, PartialEq)]
pub struct LevelEntry {
    pub kind: EntryKind,
    pub name: String,
    pub text: String,
    /// How many of the level's scenes it reaches.
    pub scenes: usize,
}

/// What is read about a level: its own context, then what it holds, each
/// with its synopsis, then the plot points and entries in it. A level is
/// read from the synopses of the level below, not from their text.
#[derive(Clone, Debug, PartialEq)]
pub struct LevelContext {
    pub kind: NodeKind,
    pub title: String,
    pub synopsis: String,
    pub status: String,
    pub notes: String,
    /// A folder's own text: what the writer says it holds. Read from the
    /// document by the caller, since this crate reads only the models.
    pub about: String,
    pub keywords: Vec<String>,
    pub place: Vec<String>,
    pub outline: Vec<OutlineItem>,
    pub plot_points: Vec<PlotPointNote>,
    pub entries: Vec<LevelEntry>,
}

/// The context of a chapter, a part, a book, or a folder; `None` for a
/// scene (see [`scene_context`]), a note, a root, or an id not in the binder.
pub fn level_context(sources: &Sources<'_>, id: Id) -> Option<LevelContext> {
    let binder = sources.binder;
    let node = binder
        .get(id)
        .filter(|node| matches!(node.kind, NodeKind::Chapter | NodeKind::Part | NodeKind::Book | NodeKind::Folder))?;

    let mut outline = Vec::new();
    for child in binder.children(Some(id)) {
        outline.push(outline_item(child, 0));
        for grandchild in binder.children(Some(child.id)) {
            outline.push(outline_item(grandchild, 1));
        }
    }

    let scenes: Vec<Id> = binder.scenes_under(id).into_iter().map(|(_, scene)| scene).collect();
    let mut plot_points = Vec::new();
    for line in sources.plot.lines_in_order() {
        let mut points: Vec<_> = sources
            .plot
            .points
            .iter()
            .filter(|point| point.plotline_id == line.id && point.scene_id.is_some_and(|scene| scenes.contains(&scene)))
            .collect();
        points.sort_by_key(|point| point.sort_index);
        plot_points.extend(points.into_iter().map(|point| PlotPointNote {
            line: line.title.clone(),
            title: point.title.clone(),
            summary: point.summary.trim().to_string(),
        }));
    }

    let entries = story_entries(sources.cast, sources.world, sources.library)
        .into_iter()
        .filter_map(|entry| {
            let reached = scenes.iter().filter(|scene| reaches(binder, &entry, **scene)).count();
            let linked_here = reaches(binder, &entry, id);
            (reached > 0 || linked_here).then(|| LevelEntry {
                kind: entry.kind,
                name: entry.name,
                text: entry.text,
                scenes: reached,
            })
        })
        .collect();

    Some(LevelContext {
        kind: node.kind,
        title: node.title.clone(),
        synopsis: node.synopsis.trim().to_string(),
        status: node.status.trim().to_string(),
        notes: node.notes.trim().to_string(),
        about: String::new(),
        keywords: keywords_of(binder, id),
        place: place_of(binder, id),
        outline,
        plot_points,
        entries,
    })
}

fn outline_item(node: &writing_studio_project::BinderNode, depth: usize) -> OutlineItem {
    OutlineItem {
        depth,
        kind: node.kind,
        title: node.title.trim().to_string(),
        status: node.status.trim().to_string(),
        synopsis: node.synopsis.trim().to_string(),
    }
}

/// How much of a level's context is kept when it must be shortened.
#[derive(Clone, Copy)]
struct Kept {
    /// Synopses kept down to this depth: 2 for all, 1 for what the level
    /// holds directly, 0 for none.
    synopses: usize,
    /// The text of the first this many entries.
    entry_text: usize,
}

impl LevelContext {
    /// The context as plain Markdown, at most `limit` characters. When it is
    /// longer, the synopses of the deepest items go first, then those of the
    /// items the level holds directly, then the entries' text from the end
    /// of the list, and the last line says what was left out; if that is not
    /// enough, the end is cut, and that is said too.
    pub fn to_markdown(&self, limit: usize) -> String {
        let mut kept = Kept { synopses: 2, entry_text: self.entries.len() };
        loop {
            let text = self.render(kept);
            let mut left_out = Vec::new();
            match kept.synopses {
                0 => left_out.push("every synopsis in the outline".to_string()),
                1 => left_out.push("the synopses of the outline's deepest items".to_string()),
                _ => {}
            }
            match self.entries.len() - kept.entry_text {
                0 => {}
                1 => left_out.push("the text of 1 entry".to_string()),
                many => left_out.push(format!("the text of {many} entries")),
            }
            let note = if left_out.is_empty() {
                String::new()
            } else {
                format!("\n_Left out, to keep this under {limit} characters: {}._\n", left_out.join(" and "))
            };
            if text.chars().count() + note.chars().count() <= limit {
                return text + &note;
            }
            if kept.synopses > 0 {
                kept.synopses -= 1;
            } else if kept.entry_text > 0 {
                kept.entry_text -= 1;
            } else {
                let cut_note = format!("\n_Cut here, to keep this under {limit} characters._\n");
                let room = limit.saturating_sub(cut_note.chars().count());
                let cut: String = text.chars().take(room).collect();
                return cut + &cut_note;
            }
        }
    }

    fn render(&self, kept: Kept) -> String {
        let mut out = format!("# {}\n\n", self.title.trim());
        let mut facts = vec![self.kind.icon_label().to_string()];
        if !self.status.is_empty() {
            facts.push(format!("Status: {}", self.status));
        }
        if !self.keywords.is_empty() {
            facts.push(format!("Keywords: {}", self.keywords.join(", ")));
        }
        if !self.place.is_empty() {
            facts.push(self.place.join(" › "));
        }
        for fact in facts {
            out.push_str(&format!("{fact}  \n"));
        }
        out.push('\n');
        if !self.synopsis.is_empty() {
            out.push_str(&format!("## Synopsis\n\n{}\n\n", self.synopsis));
        }
        if !self.about.is_empty() {
            out.push_str(&format!("## About\n\n{}\n\n", self.about));
        }
        if !self.notes.is_empty() {
            out.push_str(&format!("## Notes\n\n{}\n\n", self.notes));
        }
        if !self.outline.is_empty() {
            out.push_str("## What it holds\n\n");
            for item in &self.outline {
                let indent = "  ".repeat(item.depth);
                out.push_str(&format!("{indent}- **{}**", item.title));
                if !item.status.is_empty() {
                    out.push_str(&format!(" ({})", item.status));
                }
                if item.depth < kept.synopses && !item.synopsis.is_empty() {
                    out.push_str(&format!(": {}", item.synopsis.replace('\n', " ")));
                }
                out.push('\n');
            }
            out.push('\n');
        }
        if !self.plot_points.is_empty() {
            out.push_str("## Plot points\n\n");
            for point in &self.plot_points {
                out.push_str(&format!("- **{}**, {}", point.line.trim(), point.title.trim()));
                if !point.summary.is_empty() {
                    out.push_str(&format!(": {}", point.summary));
                }
                out.push('\n');
            }
            out.push('\n');
        }
        let mut heading = None;
        for (index, entry) in self.entries.iter().enumerate() {
            if heading != Some(entry.kind) {
                out.push_str(&format!("## {}\n\n", entry.kind.heading()));
                heading = Some(entry.kind);
            }
            out.push_str(&format!("### {}\n\n", entry.name));
            match entry.scenes {
                0 => {}
                1 => out.push_str("_In 1 scene._\n\n"),
                many => out.push_str(&format!("_In {many} scenes._\n\n")),
            }
            if index < kept.entry_text && !entry.text.is_empty() {
                out.push_str(&format!("{}\n\n", entry.text));
            }
        }
        out.trim_end().to_string() + "\n"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use writing_studio_project::BinderNode;

    struct Story {
        binder: BinderTree,
        cast: Cast,
        world: World,
        library: ResearchLibrary,
        plot: PlotGrid,
        scenes: [Id; 3],
        chapter: Id,
    }

    /// Tide: chapter One holds Arrival and Harbor, chapter Two holds Dawn.
    /// Mara is linked to chapter One, the Pilot and Port Ely to Harbor, and
    /// a research note with long notes to Harbor; the lore is linked to
    /// nothing.
    fn story() -> Story {
        let draft = BinderNode::new(NodeKind::DraftRoot, "Manuscript", None, 0);
        let book = BinderNode::new(NodeKind::Book, "Tide", Some(draft.id), 0);
        let mut one = BinderNode::new(NodeKind::Chapter, "One", Some(book.id), 0);
        let two = BinderNode::new(NodeKind::Chapter, "Two", Some(book.id), 1);
        let mut arrival = BinderNode::new(NodeKind::Scene, "Arrival", Some(one.id), 0);
        let mut harbor = BinderNode::new(NodeKind::Scene, "Harbor", Some(one.id), 1);
        let mut dawn = BinderNode::new(NodeKind::Scene, "Dawn", Some(two.id), 0);
        arrival.synopsis = "The ship comes in.".into();
        harbor.synopsis = "Mara looks for the pilot.".into();
        harbor.status = "Revise".into();
        harbor.keywords = vec!["night".into()];
        one.keywords = vec!["act one".into()];
        dawn.synopsis = "They leave.".into();
        let scenes = [arrival.id, harbor.id, dawn.id];
        let chapter = one.id;
        let binder = BinderTree::new(vec![draft, book, one, two, arrival, harbor, dawn]);

        let mut cast = Cast::new();
        let mara = cast.add_character();
        cast.set_name(mara, "Mara Vance".into());
        cast.set_summary(mara, "Always late.".into());
        let pilot = cast.add_character();
        cast.set_name(pilot, "The Pilot".into());
        cast.characters.iter_mut().find(|item| item.id == mara).unwrap().linked_ids = vec![chapter];
        cast.characters.iter_mut().find(|item| item.id == pilot).unwrap().linked_ids = vec![scenes[1]];

        let mut world = World::new();
        let port = world.add_location();
        let place = world.locations.iter_mut().find(|item| item.id == port).unwrap();
        place.name = "Port Ely".into();
        place.summary = "A harbor town.".into();
        place.linked_ids = vec![scenes[1]];
        let war = world.add_lore();
        world.lore.iter_mut().find(|item| item.id == war).unwrap().title = "The Old War".into();

        let mut library = ResearchLibrary::new();
        let note = library.add_item();
        library.set_title(note, "Tides".into());
        library.set_notes(note, "x".repeat(400));
        library.items.iter_mut().find(|item| item.id == note).unwrap().linked_ids = vec![scenes[1]];

        let mut plot = PlotGrid::new();
        let line = plot.add_line();
        plot.lines.iter_mut().find(|item| item.id == line).unwrap().title = "The search".into();
        let point = plot.add_point(line).unwrap();
        plot.set_point_title(point, "She finds him".into());
        plot.set_point_scene(point, Some(scenes[1]));

        Story { binder, cast, world, library, plot, scenes, chapter }
    }

    fn sources(story: &Story) -> Sources<'_> {
        Sources { binder: &story.binder, cast: &story.cast, world: &story.world, library: &story.library, plot: &story.plot }
    }

    #[test]
    fn a_scene_context_holds_the_scene_its_place_its_plot_and_its_entries() {
        let story = story();
        let context = scene_context(&sources(&story), story.scenes[1]).unwrap();
        assert_eq!((context.title.as_str(), context.status.as_str()), ("Harbor", "Revise"));
        assert_eq!(context.keywords, ["night", "act one"], "the chapter's keywords too");
        assert_eq!(context.place, ["Book: Tide", "Chapter: One"]);
        assert_eq!(context.before.as_ref().unwrap().title, "Arrival");
        assert_eq!(context.after.as_ref().unwrap().synopsis, "They leave.", "the next scene, in the next chapter");
        assert_eq!(context.plot_points[0].line, "The search");

        let entries: Vec<(&str, Option<&str>)> =
            context.entries.iter().map(|entry| (entry.name.as_str(), entry.through.as_deref())).collect();
        assert_eq!(
            entries,
            [("Mara Vance", Some("One")), ("The Pilot", None), ("Port Ely", None), ("Tides", None)],
            "characters first, Mara through her chapter, the unlinked lore left out",
        );

        // The first scene is reached only by Mara, through the chapter.
        let first = scene_context(&sources(&story), story.scenes[0]).unwrap();
        assert!(first.before.is_none());
        assert_eq!(first.entries.len(), 1);
        assert!(scene_context(&sources(&story), story.chapter).is_none(), "a chapter is not a scene");
    }

    #[test]
    fn a_book_and_a_chapter_are_read_from_the_synopses_below_them() {
        let mut story = story();
        let book = story.binder.book_of(story.chapter).unwrap();
        story.binder.nodes.iter_mut().find(|node| node.id == book).unwrap().notes = "Ends at sea.".into();
        let context = level_context(&sources(&story), book).unwrap();
        assert_eq!(context.title, "Tide");
        assert_eq!(context.notes, "Ends at sea.");
        let outline: Vec<(usize, &str, &str)> = context
            .outline
            .iter()
            .map(|item| (item.depth, item.title.as_str(), item.synopsis.as_str()))
            .collect();
        assert_eq!(
            outline,
            [
                (0, "One", ""),
                (1, "Arrival", "The ship comes in."),
                (1, "Harbor", "Mara looks for the pilot."),
                (0, "Two", ""),
                (1, "Dawn", "They leave."),
            ]
        );
        // Mara is linked to chapter One, so she is in both of its scenes;
        // the lore is linked to nothing and is not here.
        let entries: Vec<(&str, usize)> = context.entries.iter().map(|entry| (entry.name.as_str(), entry.scenes)).collect();
        assert!(entries.contains(&("Mara Vance", 2)) && entries.contains(&("The Pilot", 1)), "{entries:?}");
        assert!(!entries.iter().any(|(name, _)| *name == "The Old War"));
        assert_eq!(context.plot_points[0].title, "She finds him");

        let chapter = level_context(&sources(&story), story.chapter).unwrap();
        assert_eq!(chapter.keywords, ["act one"]);
        assert_eq!(chapter.place, ["Book: Tide"]);
        assert_eq!(chapter.outline.len(), 2);
        assert!(level_context(&sources(&story), story.scenes[0]).is_none(), "a scene has its own context");

        // Shortened: the deepest synopses go first, and that is said.
        let full = context.to_markdown(100_000);
        assert!(full.contains("## What it holds\n\n- **One** (draft)\n  - **Arrival** (draft): The ship comes in.\n"), "{full}");
        assert!(full.contains("## Notes\n\nEnds at sea."));
        let limit = full.chars().count() - 1;
        let short = context.to_markdown(limit);
        assert!(short.chars().count() <= limit);
        // The outline's synopses went first, then the entries' text from the
        // end of the list: the research note's long notes, not Mara's line.
        assert!(!short.contains("The ship comes in.") && short.contains("every synopsis in the outline"), "{short}");
        assert!(short.contains("Always late.") && !short.contains(&"x".repeat(400)), "{short}");
        let tiny = context.to_markdown(120);
        assert!(tiny.chars().count() <= 120 && tiny.contains("Cut here"), "{tiny}");
    }

    #[test]
    fn the_markdown_drops_entry_text_from_the_end_first_and_says_so() {
        let story = story();
        let context = scene_context(&sources(&story), story.scenes[1]).unwrap();
        let full = context.to_markdown(10_000);
        for part in ["# Harbor", "Status: Revise", "Book: Tide › Chapter: One", "## Before", "**Arrival**: The ship comes in.",
            "## Plot points", "- **The search**, She finds him", "## Characters", "### Mara Vance", "_Through One._",
            "Always late.", "## Places", "A harbor town.", "## Research"]
        {
            assert!(full.contains(part), "{part} missing:\n{full}");
        }
        assert!(!full.contains("left out"));

        let limit = full.chars().count() - 100;
        let short = context.to_markdown(limit);
        assert!(short.chars().count() <= limit);
        assert!(!short.contains(&"x".repeat(400)), "the last entry's text goes first");
        assert!(short.contains("A harbor town.") && short.contains("### Tides"), "the others stay, and its name");
        assert!(short.contains("The text of 1 entry was left out"), "{short}");

        let tiny = context.to_markdown(120);
        assert!(tiny.chars().count() <= 120 && tiny.contains("Cut here"), "{tiny}");
    }

    #[test]
    fn entries_nest_within_their_kind_and_never_inside_themselves() {
        let mut world = World::new();
        let ids: Vec<Id> = (0..4).map(|_| world.add_location()).collect();
        let (country, city, quarter, pub_) = (ids[0], ids[1], ids[2], ids[3]);
        assert!(move_entry(&mut world.locations, city, Some(country), None));
        assert!(move_entry(&mut world.locations, quarter, Some(city), None));
        assert!(move_entry(&mut world.locations, pub_, Some(quarter), None));
        let rows = tree_rows(&world.locations, &[]);
        let depths: Vec<(Id, u32, bool)> = rows;
        assert_eq!(depths, [(country, 0, true), (city, 1, true), (quarter, 2, true), (pub_, 3, false)]);
        assert!(!move_entry(&mut world.locations, country, Some(pub_), None), "not inside what it holds");
        assert!(tree_rows(&world.locations, &[city]).len() == 2, "folded");

        // A second city goes after the first; the pub moves up a level.
        let other = world.add_location();
        assert!(move_entry(&mut world.locations, other, Some(country), Some(city)));
        assert_eq!(children_of(&world.locations, Some(country)), [city, other]);
        assert!(move_entry(&mut world.locations, pub_, Some(country), None));
        assert_eq!(children_of(&world.locations, Some(country)), [pub_, city, other]);
        assert!(is_within(&world.locations, quarter, country) && !is_within(&world.locations, country, quarter));
    }


    #[test]
    fn connections_hold_named_ones_nesting_and_fields() {
        let mut cast = Cast::new();
        let father = cast.add_character();
        let son = cast.add_character();
        let mut world = World::new();
        let city = world.add_location();
        let culture = world.add_culture();
        world.locations[0].culture_id = Some(culture);
        cast.characters.iter_mut().find(|item| item.id == son).unwrap().parent_id = Some(father);
        let lives = cast.connect(son, city, " lives in ".into()).unwrap();
        assert!(cast.connect(son, city, "Lives in".into()).is_none(), "once");
        let all = connections(&cast, &world, &ResearchLibrary::new());
        let names: Vec<(Id, Id, &str, Tie)> = all.iter().map(|c| (c.from, c.to, c.name.as_str(), c.tie)).collect();
        assert!(names.contains(&(son, city, "lives in", Tie::Named(lives))));
        assert!(names.contains(&(son, father, "inside", Tie::Inside)));
        assert!(names.contains(&(city, culture, "of the culture", Tie::Field)));
        assert!(cast.disconnect_all(city));
        assert!(connections(&cast, &world, &ResearchLibrary::new()).iter().all(|c| !matches!(c.tie, Tie::Named(_))));
    }

}
