//! What a scene needs around it, in one place: the scene, where it sits in
//! the book and what comes before and after, its plot points, and every
//! character, place, and note linked to it, directly or through the chapter,
//! part, or book it is in.
//!
//! It reads the project's own models and nothing else (not the window, not
//! the files), so anything added later can ask for a scene's context with a
//! project's data: the inspector shows it, and `to_markdown` is what an
//! assistant would read. Nothing here sends anything anywhere.

use writing_studio_core::Id;
use writing_studio_plotting::PlotGrid;
use writing_studio_project::{BinderTree, NodeKind};

/// What a context is read from: the project's models, and, from the caller
/// that has the files, a note's text and the note titles an item's text
/// links to with `[[…]]`, in lowercase.
#[derive(Clone, Copy)]
pub struct Sources<'a> {
    pub binder: &'a BinderTree,
    pub plot: &'a PlotGrid,
    pub note_text: &'a dyn Fn(Id) -> String,
    pub mentions: &'a dyn Fn(Id) -> Vec<String>,
}

/// The notes in binder order, folder by folder: each with its folder's title.
fn notes_in_order(binder: &BinderTree) -> Vec<(&writing_studio_project::BinderNode, String)> {
    fn walk<'b>(binder: &'b BinderTree, parent: Id, folder: &str, out: &mut Vec<(&'b writing_studio_project::BinderNode, String)>) {
        for child in binder.children(Some(parent)) {
            match child.kind {
                NodeKind::Note => out.push((child, folder.to_string())),
                NodeKind::Folder => walk(binder, child.id, &child.title, out),
                _ => {}
            }
        }
    }
    let mut out = Vec::new();
    if let Some(root) = binder.notes_root() {
        walk(binder, root.id, "Notes", &mut out);
    }
    out
}

/// How a note reaches an item: linked to it from Context, or mentioned in
/// its text (`Some(None)`), or linked to something it is in (`Some(Some(that))`).
fn note_reaches(sources: &Sources<'_>, note: &writing_studio_project::BinderNode, item: Id) -> Option<Option<Id>> {
    let title = note.title.trim().to_lowercase();
    if note.links.contains(&item) || (sources.mentions)(item).iter().any(|seen| seen.trim() == title) {
        return Some(None);
    }
    let mut up = sources.binder.get(item).and_then(|node| node.parent_id);
    while let Some(id) = up {
        if note.links.contains(&id) {
            return Some(Some(id));
        }
        up = sources.binder.get(id).and_then(|node| node.parent_id);
    }
    None
}

/// The scene before or after, by its title and summary: the first
/// paragraph of its context.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Neighbor {
    pub title: String,
    pub summary: String,
}

/// A plot point that points at the scene, with its plotline.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlotPointNote {
    pub line: String,
    pub title: String,
    pub summary: String,
}

/// A note as the scene's context holds it, with the folder it is in.
#[derive(Clone, Debug, PartialEq)]
pub struct ContextEntry {
    pub folder: String,
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
    pub status: String,
    /// What the writer says about it, all of it.
    pub context: String,
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
            .map(|node| Neighbor { title: node.title.clone(), summary: node.summary() })
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

    let entries = notes_in_order(binder)
        .into_iter()
        .filter_map(|(note, folder)| {
            let through = note_reaches(sources, note, scene)?;
            Some(ContextEntry {
                folder,
                id: note.id,
                name: note.title.clone(),
                text: (sources.note_text)(note.id).trim().to_string(),
                through: through.map(|id| binder.get(id).map(|node| node.title.clone()).unwrap_or_default()),
            })
        })
        .collect();

    Some(SceneContext {
        title: node.title.clone(),
        status: node.status.trim().to_string(),
        context: node.context.trim().to_string(),
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
        if !self.context.is_empty() {
            out.push_str(&format!("## Context\n\n{}\n\n", self.context));
        }
        for (heading, neighbor) in [("Before", &self.before), ("After", &self.after)] {
            if let Some(neighbor) = neighbor {
                out.push_str(&format!("## {heading}\n\n**{}**", neighbor.title.trim()));
                if !neighbor.summary.is_empty() {
                    out.push_str(&format!(": {}", neighbor.summary.replace('\n', " ")));
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
        let mut heading: Option<&str> = None;
        for (index, entry) in self.entries.iter().enumerate() {
            if heading != Some(entry.folder.as_str()) {
                out.push_str(&format!("## {}\n\n", entry.folder));
                heading = Some(entry.folder.as_str());
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
    /// The first paragraph of its context.
    pub summary: String,
}

/// A note linked to the level or to anything in it, with its folder.
#[derive(Clone, Debug, PartialEq)]
pub struct LevelEntry {
    pub folder: String,
    pub name: String,
    pub text: String,
    /// How many of the level's scenes it reaches.
    pub scenes: usize,
}

/// What is read about a level: its own context, then what it holds, each
/// with its summary, then the plot points and entries in it. A level is
/// read from the summaries of the level below, not from their text.
#[derive(Clone, Debug, PartialEq)]
pub struct LevelContext {
    pub kind: NodeKind,
    pub title: String,
    pub status: String,
    /// What the writer says about it, all of it.
    pub context: String,
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

    let entries = notes_in_order(binder)
        .into_iter()
        .filter(|(note, _)| note.id != id)
        .filter_map(|(note, folder)| {
            let reached = scenes.iter().filter(|scene| note_reaches(sources, note, **scene).is_some()).count();
            let linked_here = note_reaches(sources, note, id).is_some();
            (reached > 0 || linked_here).then(|| LevelEntry {
                folder,
                name: note.title.clone(),
                text: (sources.note_text)(note.id).trim().to_string(),
                scenes: reached,
            })
        })
        .collect();

    Some(LevelContext {
        kind: node.kind,
        title: node.title.clone(),
        status: node.status.trim().to_string(),
        context: node.context.trim().to_string(),
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
        summary: node.summary(),
    }
}

/// How much of a level's context is kept when it must be shortened.
#[derive(Clone, Copy)]
struct Kept {
    /// Summaries kept down to this depth: 2 for all, 1 for what the level
    /// holds directly, 0 for none.
    summaries: usize,
    /// The text of the first this many entries.
    entry_text: usize,
}

impl LevelContext {
    /// The context as plain Markdown, at most `limit` characters. When it is
    /// longer, the summaries of the deepest items go first, then those of the
    /// items the level holds directly, then the entries' text from the end
    /// of the list, and the last line says what was left out; if that is not
    /// enough, the end is cut, and that is said too.
    pub fn to_markdown(&self, limit: usize) -> String {
        let mut kept = Kept { summaries: 2, entry_text: self.entries.len() };
        loop {
            let text = self.render(kept);
            let mut left_out = Vec::new();
            match kept.summaries {
                0 => left_out.push("every summary in the outline".to_string()),
                1 => left_out.push("the summaries of the outline's deepest items".to_string()),
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
            if kept.summaries > 0 {
                kept.summaries -= 1;
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
        if !self.context.is_empty() {
            out.push_str(&format!("## Context\n\n{}\n\n", self.context));
        }
        if !self.about.is_empty() {
            out.push_str(&format!("## About\n\n{}\n\n", self.about));
        }
        if !self.outline.is_empty() {
            out.push_str("## What it holds\n\n");
            for item in &self.outline {
                let indent = "  ".repeat(item.depth);
                out.push_str(&format!("{indent}- **{}**", item.title));
                if !item.status.is_empty() {
                    out.push_str(&format!(" ({})", item.status));
                }
                if item.depth < kept.summaries && !item.summary.is_empty() {
                    out.push_str(&format!(": {}", item.summary.replace('\n', " ")));
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
        let mut heading: Option<&str> = None;
        for (index, entry) in self.entries.iter().enumerate() {
            if heading != Some(entry.folder.as_str()) {
                out.push_str(&format!("## {}\n\n", entry.folder));
                heading = Some(entry.folder.as_str());
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
        plot: PlotGrid,
        scenes: [Id; 3],
        chapter: Id,
        note_text: Box<dyn Fn(Id) -> String>,
        mentions: Box<dyn Fn(Id) -> Vec<String>>,
    }

    /// Tide: chapter One holds Arrival and Harbor, chapter Two holds Dawn.
    /// Notes: Mara is linked to chapter One, the Pilot and Port Ely to
    /// Harbor, and a long research note to Harbor; the lore is linked to
    /// nothing.
    fn story() -> Story {
        let draft = BinderNode::new(NodeKind::DraftRoot, "Manuscript", None, 0);
        let book = BinderNode::new(NodeKind::Book, "Tide", Some(draft.id), 0);
        let mut one = BinderNode::new(NodeKind::Chapter, "One", Some(book.id), 0);
        let two = BinderNode::new(NodeKind::Chapter, "Two", Some(book.id), 1);
        let mut arrival = BinderNode::new(NodeKind::Scene, "Arrival", Some(one.id), 0);
        let mut harbor = BinderNode::new(NodeKind::Scene, "Harbor", Some(one.id), 1);
        let mut dawn = BinderNode::new(NodeKind::Scene, "Dawn", Some(two.id), 0);
        arrival.context = "The ship comes in.\n\nNot shown beside the next scene.".into();
        harbor.context = "Mara looks for the pilot.".into();
        harbor.status = "Revise".into();
        harbor.keywords = vec!["night".into()];
        one.keywords = vec!["act one".into()];
        dawn.context = "They leave.".into();
        let scenes = [arrival.id, harbor.id, dawn.id];
        let chapter = one.id;
        let binder = BinderTree::new(vec![draft, book, one, two, arrival, harbor, dawn]);

        let mut plot = PlotGrid::new();
        let line = plot.add_line();
        plot.lines.iter_mut().find(|item| item.id == line).unwrap().title = "The search".into();
        let point = plot.add_point(line).unwrap();
        plot.set_point_title(point, "She finds him".into());
        plot.set_point_scene(point, Some(scenes[1]));

        // The notes, in folders; the lore is linked to nothing.
        let mut binder = binder;
        let notes_root = BinderNode::new(NodeKind::NotesRoot, "Notes", None, 1);
        let root = notes_root.id;
        binder.nodes.push(notes_root);
        let mut texts = std::collections::HashMap::new();
        for (index, (folder, items)) in [
            ("Characters", vec![("Mara Vance", "Always late.", vec![chapter]), ("The Pilot", "", vec![scenes[1]])]),
            ("Places", vec![("Port Ely", "A harbor town.", vec![scenes[1]])]),
            ("Lore", vec![("The Old War", "", vec![])]),
            ("Research", vec![("Tides", &"x".repeat(400)[..], vec![scenes[1]])]),
        ]
        .into_iter()
        .enumerate()
        {
            let folder_node = BinderNode::new(NodeKind::Folder, folder, Some(root), index as i64);
            let folder_id = folder_node.id;
            binder.nodes.push(folder_node);
            for (at, (title, text, links)) in items.into_iter().enumerate() {
                let mut note = BinderNode::new(NodeKind::Note, title, Some(folder_id), at as i64);
                note.links = links;
                texts.insert(note.id, text.to_string());
                binder.nodes.push(note);
            }
        }
        let note_text: Box<dyn Fn(Id) -> String> = Box::new(move |id| texts.get(&id).cloned().unwrap_or_default());
        let mentions: Box<dyn Fn(Id) -> Vec<String>> = Box::new(|_| Vec::new());
        Story { binder, plot, scenes, chapter, note_text, mentions }
    }

    fn sources(story: &Story) -> Sources<'_> {
        Sources {
            binder: &story.binder,
            plot: &story.plot,
            note_text: &*story.note_text,
            mentions: &*story.mentions,
        }
    }

    #[test]
    fn a_scene_context_holds_the_scene_its_place_its_plot_and_its_entries() {
        let story = story();
        let context = scene_context(&sources(&story), story.scenes[1]).unwrap();
        assert_eq!((context.title.as_str(), context.status.as_str()), ("Harbor", "Revise"));
        assert_eq!(context.keywords, ["night", "act one"], "the chapter's keywords too");
        assert_eq!(context.place, ["Book: Tide", "Chapter: One"]);
        assert_eq!(context.before.as_ref().unwrap().title, "Arrival");
        assert_eq!(context.after.as_ref().unwrap().summary, "They leave.", "the next scene, in the next chapter");
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
    fn a_book_and_a_chapter_are_read_from_the_summaries_below_them() {
        let mut story = story();
        let book = story.binder.book_of(story.chapter).unwrap();
        story.binder.nodes.iter_mut().find(|node| node.id == book).unwrap().context = "Ends at sea.".into();
        let context = level_context(&sources(&story), book).unwrap();
        assert_eq!(context.title, "Tide");
        assert_eq!(context.context, "Ends at sea.");
        let outline: Vec<(usize, &str, &str)> = context
            .outline
            .iter()
            .map(|item| (item.depth, item.title.as_str(), item.summary.as_str()))
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

        // Shortened: the deepest summaries go first, and that is said.
        let full = context.to_markdown(100_000);
        assert!(full.contains("## What it holds\n\n- **One** (draft)\n  - **Arrival** (draft): The ship comes in.\n"), "{full}");
        assert!(full.contains("## Context\n\nEnds at sea."));
        let limit = full.chars().count() - 1;
        let short = context.to_markdown(limit);
        assert!(short.chars().count() <= limit);
        // The outline's summaries went first, then the entries' text from the
        // end of the list: the research note's long notes, not Mara's line.
        assert!(!short.contains("The ship comes in.") && short.contains("every summary in the outline"), "{short}");
        assert!(short.contains("Always late.") && !short.contains(&"x".repeat(400)), "{short}");
        let tiny = context.to_markdown(120);
        assert!(tiny.chars().count() <= 120 && tiny.contains("Cut here"), "{tiny}");
    }

    #[test]
    fn a_note_mentioned_in_a_scene_is_in_its_context() {
        let mut story = story();
        let dawn = story.scenes[2];
        story.mentions = Box::new(move |item| if item == dawn { vec!["port ely".into()] } else { Vec::new() });
        let context = scene_context(&sources(&story), dawn).unwrap();
        let entries: Vec<(&str, &str, Option<&str>)> =
            context.entries.iter().map(|entry| (entry.folder.as_str(), entry.name.as_str(), entry.through.as_deref())).collect();
        assert_eq!(entries, [("Places", "Port Ely", None)]);
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
        assert!(!full.contains("Not shown beside"), "only the scene before's first paragraph");

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
