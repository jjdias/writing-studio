//! What the mind map keeps beyond the binder: ideas, lines between nodes,
//! and where the writer put nodes by hand. Stored as `mindmap.json`; story
//! links stay in `bible.json`, as they are the bible's.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use writing_studio_core::Id;

/// Keys a file holds that this program does not know, kept and written back.
pub type OtherKeys = BTreeMap<String, serde_json::Value>;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MindMap {
    #[serde(default)]
    pub ideas: Vec<Idea>,
    #[serde(default)]
    pub lines: Vec<MapLine>,
    #[serde(default)]
    pub positions: Vec<MapPosition>,
    #[serde(flatten)]
    pub other: OtherKeys,
}

/// A node that is not in the book: never compiled, never counted.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Idea {
    pub id: Id,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub note: String,
    /// The node it hangs from: the book, any binder item, or another idea;
    /// `None` for a loose idea.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attached: Option<Id>,
    #[serde(flatten)]
    pub other: OtherKeys,
}

/// A line between two nodes, with an optional word on it ("echoes"). Not a
/// binder move and not a story link.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MapLine {
    pub id: Id,
    pub from: Id,
    pub to: Id,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub label: String,
    #[serde(flatten)]
    pub other: OtherKeys,
}

/// Where the writer put a node, in map units, by the node's id: a binder
/// item, an idea, or a story entry.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MapPosition {
    pub id: Id,
    pub x: f64,
    pub y: f64,
    #[serde(flatten)]
    pub other: OtherKeys,
}

impl MindMap {
    pub fn is_empty(&self) -> bool {
        self.ideas.is_empty() && self.lines.is_empty() && self.positions.is_empty() && self.other.is_empty()
    }

    pub fn idea(&self, id: Id) -> Option<&Idea> {
        self.ideas.iter().find(|idea| idea.id == id)
    }

    /// A new idea hanging from `attached`, or loose. Returns its id.
    pub fn add_idea(&mut self, attached: Option<Id>) -> Id {
        let id = Id::new();
        self.ideas.push(Idea { id, title: "New idea".into(), note: String::new(), attached, other: OtherKeys::new() });
        id
    }

    /// Take an idea away, with its lines and position. Ideas that hung from
    /// it hang from what it hung from.
    pub fn remove_idea(&mut self, id: Id) -> bool {
        let Some(at) = self.ideas.iter().position(|idea| idea.id == id) else {
            return false;
        };
        let up = self.ideas.remove(at).attached;
        for idea in &mut self.ideas {
            if idea.attached == Some(id) {
                idea.attached = up;
            }
        }
        self.forget(&[id]);
        true
    }

    /// Join two nodes with a line, unless one already joins them.
    pub fn add_line(&mut self, from: Id, to: Id) -> Option<Id> {
        let joined = self.lines.iter().any(|line| (line.from, line.to) == (from, to) || (line.from, line.to) == (to, from));
        if from == to || joined {
            return None;
        }
        let id = Id::new();
        self.lines.push(MapLine { id, from, to, label: String::new(), other: OtherKeys::new() });
        Some(id)
    }

    /// Keep a node where the writer put it.
    pub fn place(&mut self, id: Id, x: f64, y: f64) {
        match self.positions.iter_mut().find(|kept| kept.id == id) {
            Some(kept) => (kept.x, kept.y) = (x, y),
            None => self.positions.push(MapPosition { id, x, y, other: OtherKeys::new() }),
        }
    }

    pub fn position(&self, id: Id) -> Option<(f64, f64)> {
        self.positions.iter().find(|kept| kept.id == id).map(|kept| (kept.x, kept.y))
    }

    /// Drop every line to and position of these ids, and loosen ideas that
    /// hung from them: what was deleted for good is no longer on the map.
    pub fn forget(&mut self, gone: &[Id]) -> bool {
        let before = (self.lines.len(), self.positions.len());
        self.lines.retain(|line| !gone.contains(&line.from) && !gone.contains(&line.to));
        self.positions.retain(|kept| !gone.contains(&kept.id));
        let mut loosened = false;
        for idea in &mut self.ideas {
            if idea.attached.is_some_and(|up| gone.contains(&up)) {
                idea.attached = None;
                loosened = true;
            }
        }
        loosened || before != (self.lines.len(), self.positions.len())
    }

    /// An idea became a binder item: its lines and the ideas that hung from
    /// it go with the new id, and it is no longer an idea.
    pub fn idea_became(&mut self, idea: Id, item: Id) {
        for line in &mut self.lines {
            if line.from == idea {
                line.from = item;
            }
            if line.to == idea {
                line.to = item;
            }
        }
        for other in &mut self.ideas {
            if other.attached == Some(idea) {
                other.attached = Some(item);
            }
        }
        self.ideas.retain(|kept| kept.id != idea);
        self.positions.retain(|kept| kept.id != idea);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ideas_lines_and_positions_follow_what_happens_to_their_nodes() {
        let (scene, chapter) = (Id::new(), Id::new());
        let mut map = MindMap::default();
        assert!(map.is_empty());
        let idea = map.add_idea(Some(chapter));
        let under = map.add_idea(Some(idea));
        assert!(map.add_line(idea, scene).is_some());
        assert!(map.add_line(scene, idea).is_none(), "one line joins two nodes");
        assert!(map.add_line(scene, scene).is_none());
        map.place(idea, 10.0, 20.0);
        map.place(idea, 30.0, 40.0);
        assert_eq!(map.position(idea), Some((30.0, 40.0)));

        // Made a scene: its line and the idea under it follow the new item.
        let made = Id::new();
        map.idea_became(idea, made);
        assert_eq!((map.lines[0].from, map.lines[0].to), (made, scene));
        assert_eq!(map.idea(under).unwrap().attached, Some(made));
        assert!(map.idea(idea).is_none() && map.position(idea).is_none());

        // Deleted for good: its line goes, and the idea under it is loose.
        assert!(map.forget(&[made]));
        assert!(map.lines.is_empty());
        assert_eq!(map.idea(under).unwrap().attached, None);
        assert!(map.remove_idea(under) && map.ideas.is_empty());
    }

    #[test]
    fn keys_the_program_does_not_know_are_kept() {
        let text = r#"{"ideas":[{"id":"8f8b6a3e-7f43-4b7b-9a51-2f1f7a7c2d10","title":"Storm","mood":"dark"}],"version_note":"hand"}"#;
        let map: MindMap = serde_json::from_str(text).unwrap();
        assert_eq!(map.ideas[0].other["mood"], "dark");
        let back = serde_json::to_string(&map).unwrap();
        assert!(back.contains(r#""mood":"dark""#) && back.contains(r#""version_note":"hand""#), "{back}");
        assert!(!back.contains("attached"), "an empty field is not written");
    }
}
