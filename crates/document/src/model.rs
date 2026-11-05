use serde::{Deserialize, Serialize};
use writing_studio_core::{Id, Timestamp};

use crate::DOCUMENT_SCHEMA_VERSION;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Document {
    pub id: Id,
    pub schema_version: u32,
    pub blocks: Vec<Block>,
    pub updated_at: Timestamp,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Block {
    pub id: Id,
    pub kind: BlockKind,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BlockKind {
    Paragraph { inlines: Vec<Inline> },
    Heading { level: u8, inlines: Vec<Inline> },
    Quote { inlines: Vec<Inline> },
    List { ordered: bool, items: Vec<ListItem> },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ListItem {
    pub id: Id,
    pub inlines: Vec<Inline>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Inline {
    pub id: Id,
    pub text: String,
    pub marks: Marks,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Marks {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub link: Option<String>,
}

impl Document {
    pub fn new(id: Id) -> Self {
        Self {
            id,
            schema_version: DOCUMENT_SCHEMA_VERSION,
            blocks: vec![Block::paragraph("")],
            updated_at: Timestamp::now(),
        }
    }

    pub fn from_plain_text(id: Id, text: &str) -> Self {
        let mut doc = Self::new(id);
        doc.replace_plain_text(text);
        doc
    }

    pub fn replace_plain_text(&mut self, text: &str) {
        let blocks: Vec<Block> = if text.is_empty() {
            vec![Block::paragraph("")]
        } else {
            text.split('\n')
                .map(|line| Block::paragraph(line))
                .collect()
        };
        self.blocks = blocks;
        self.updated_at = Timestamp::now();
    }

    pub fn to_plain_text(&self) -> String {
        self.blocks
            .iter()
            .map(Block::plain_text)
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn word_count(&self) -> u32 {
        count_words(&self.to_plain_text())
    }

    pub fn char_count(&self) -> u32 {
        self.to_plain_text().chars().count() as u32
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

impl Block {
    pub fn paragraph(text: impl Into<String>) -> Self {
        Self {
            id: Id::new(),
            kind: BlockKind::Paragraph {
                inlines: vec![Inline::text(text)],
            },
        }
    }

    pub fn heading(level: u8, text: impl Into<String>) -> Self {
        Self {
            id: Id::new(),
            kind: BlockKind::Heading {
                level: level.clamp(1, 6),
                inlines: vec![Inline::text(text)],
            },
        }
    }

    pub fn plain_text(&self) -> String {
        match &self.kind {
            BlockKind::Paragraph { inlines }
            | BlockKind::Heading { inlines, .. }
            | BlockKind::Quote { inlines } => join_inlines(inlines),
            BlockKind::List { items, .. } => items
                .iter()
                .map(|item| join_inlines(&item.inlines))
                .collect::<Vec<_>>()
                .join("\n"),
        }
    }
}

impl Inline {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            id: Id::new(),
            text: text.into(),
            marks: Marks::default(),
        }
    }
}

pub(crate) fn join_inlines(inlines: &[Inline]) -> String {
    inlines.iter().map(|inline| inline.text.as_str()).collect()
}

fn count_words(text: &str) -> u32 {
    text.split_whitespace().filter(|part| !part.is_empty()).count() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_plain_text_as_blocks() {
        let id = Id::new();
        let doc = Document::from_plain_text(id, "One\n\nTwo words here");
        assert_eq!(doc.blocks.len(), 3);
        assert_eq!(doc.to_plain_text(), "One\n\nTwo words here");
        assert_eq!(doc.word_count(), 4);
    }

    #[test]
    fn json_round_trip_preserves_ids() {
        let doc = Document::from_plain_text(Id::new(), "Hello");
        let json = doc.to_json().unwrap();
        let restored = Document::from_json(&json).unwrap();
        assert_eq!(restored.id, doc.id);
        assert_eq!(restored.to_plain_text(), "Hello");
    }
}
