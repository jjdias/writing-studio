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
}

/// Every entry of the story, characters first, each kind in its own order.
/// An entry with no name is called by its kind: "Unnamed character".
pub fn story_entries(cast: &Cast, world: &World, library: &ResearchLibrary) -> Vec<StoryEntry> {
    let mut entries = Vec::new();
    let mut push = |kind: EntryKind, id: Id, name: &str, parts: &[&str], links: &[Id]| {
        let name = if name.trim().is_empty() { format!("Unnamed {}", kind.noun()) } else { name.trim().to_string() };
        let text = parts.iter().map(|part| part.trim()).filter(|part| !part.is_empty()).collect::<Vec<_>>().join("\n\n");
        entries.push(StoryEntry { kind, id, name, text, links: links.to_vec() });
    };
    for item in cast.characters_in_order() {
        push(EntryKind::Character, item.id, &item.name, &[&item.role, &item.summary], &item.linked_ids);
    }
    for item in world.locations_in_order() {
        push(EntryKind::Location, item.id, &item.name, &[&item.summary], &item.linked_ids);
    }
    for item in world.cultures_in_order() {
        push(EntryKind::Culture, item.id, &item.name, &[&item.summary], &item.linked_ids);
    }
    for item in world.lore_in_order() {
        push(EntryKind::Lore, item.id, &item.title, &[&item.body], &item.linked_ids);
    }
    for item in world.events_in_order() {
        push(EntryKind::Timeline, item.id, &item.title, &[&item.when_label, &item.summary], &item.linked_ids);
    }
    for item in library.items_in_order() {
        push(EntryKind::Research, item.id, &item.title, &[&item.notes], &item.linked_ids);
    }
    entries
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

/// A scene's context, or `None` when the id is not a scene in the binder.
pub fn scene_context(sources: &Sources<'_>, scene: Id) -> Option<SceneContext> {
    let binder = sources.binder;
    let node = binder.get(scene).filter(|node| node.kind == NodeKind::Scene)?;

    let mut keywords: Vec<String> = Vec::new();
    for word in binder.keywords_through(scene) {
        if !keywords.iter().any(|kept| writing_studio_project::same_keyword(kept, word)) {
            keywords.push(word.to_string());
        }
    }

    let mut place = Vec::new();
    let mut at = node.parent_id;
    while let Some(up) = at.and_then(|id| binder.get(id)) {
        if up.kind.is_root() || place.len() > 64 {
            break;
        }
        place.push(format!("{}: {}", up.kind.icon_label(), up.title));
        at = up.parent_id;
    }
    place.reverse();

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
}
