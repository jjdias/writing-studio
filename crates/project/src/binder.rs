use serde::{Deserialize, Serialize};
use writing_studio_core::{Id, Timestamp};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    DraftRoot,
    ResearchRoot,
    TrashRoot,
    Folder,
    Book,
    Part,
    Chapter,
    Scene,
    Note,
}

impl NodeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DraftRoot => "draft_root",
            Self::ResearchRoot => "research_root",
            Self::TrashRoot => "trash_root",
            Self::Folder => "folder",
            Self::Book => "book",
            Self::Part => "part",
            Self::Chapter => "chapter",
            Self::Scene => "scene",
            Self::Note => "note",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "draft_root" => Self::DraftRoot,
            "research_root" => Self::ResearchRoot,
            "trash_root" => Self::TrashRoot,
            "folder" => Self::Folder,
            "book" => Self::Book,
            "part" => Self::Part,
            "chapter" => Self::Chapter,
            "scene" => Self::Scene,
            "note" => Self::Note,
            _ => return None,
        })
    }

    pub fn can_hold_document(self) -> bool {
        matches!(self, Self::Scene | Self::Note | Self::Chapter)
    }

    pub fn icon_label(self) -> &'static str {
        match self {
            Self::DraftRoot => "Manuscript",
            Self::ResearchRoot => "Research",
            Self::TrashRoot => "Trash",
            Self::Folder => "Folder",
            Self::Book => "Book",
            Self::Part => "Part",
            Self::Chapter => "Chapter",
            Self::Scene => "Scene",
            Self::Note => "Note",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BinderNode {
    pub id: Id,
    pub parent_id: Option<Id>,
    pub kind: NodeKind,
    pub title: String,
    pub sort_index: i64,
    pub document_id: Option<Id>,
    pub synopsis: String,
    pub label: String,
    pub status: String,
    pub color: String,
    pub deleted_at: Option<Timestamp>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl BinderNode {
    pub fn new(kind: NodeKind, title: impl Into<String>, parent_id: Option<Id>, sort_index: i64) -> Self {
        let now = Timestamp::now();
        let kind_copy = kind;
        Self {
            id: Id::new(),
            parent_id,
            kind,
            title: title.into(),
            sort_index,
            document_id: if kind_copy.can_hold_document() {
                Some(Id::new())
            } else {
                None
            },
            synopsis: String::new(),
            label: String::new(),
            status: String::from("draft"),
            color: String::new(),
            deleted_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn is_root(&self) -> bool {
        matches!(
            self.kind,
            NodeKind::DraftRoot | NodeKind::ResearchRoot | NodeKind::TrashRoot
        )
    }
}

#[derive(Clone, Debug)]
pub struct BinderTree {
    pub nodes: Vec<BinderNode>,
}

impl BinderTree {
    pub fn new(nodes: Vec<BinderNode>) -> Self {
        Self { nodes }
    }

    pub fn children(&self, parent: Option<Id>) -> Vec<&BinderNode> {
        let mut kids: Vec<&BinderNode> = self
            .nodes
            .iter()
            .filter(|node| node.parent_id == parent && node.deleted_at.is_none())
            .collect();
        kids.sort_by_key(|node| node.sort_index);
        kids
    }

    pub fn get(&self, id: Id) -> Option<&BinderNode> {
        self.nodes.iter().find(|node| node.id == id)
    }

    pub fn get_mut(&mut self, id: Id) -> Option<&mut BinderNode> {
        self.nodes.iter_mut().find(|node| node.id == id)
    }

    pub fn next_sort_index(&self, parent: Option<Id>) -> i64 {
        self.children(parent)
            .last()
            .map(|node| node.sort_index + 1)
            .unwrap_or(0)
    }

    pub fn draft_root(&self) -> Option<&BinderNode> {
        self.nodes.iter().find(|node| node.kind == NodeKind::DraftRoot)
    }

    pub fn research_root(&self) -> Option<&BinderNode> {
        self.nodes
            .iter()
            .find(|node| node.kind == NodeKind::ResearchRoot)
    }

    pub fn total_word_count<F>(&self, mut words_for: F) -> u32
    where
        F: FnMut(Id) -> u32,
    {
        self.nodes
            .iter()
            .filter(|node| node.deleted_at.is_none())
            .filter_map(|node| node.document_id)
            .map(|id| words_for(id))
            .sum()
    }
}
