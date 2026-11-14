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
    Code { inlines: Vec<Inline> },
    SceneBreak,
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
    /// Writer note attached to this run. Omitted from older documents.
    #[serde(default)]
    pub comment: Option<String>,
    /// What the note is for. Only meaningful when `comment` is set.
    #[serde(default)]
    pub comment_kind: NoteKind,
    /// A resolved note stays in the document but is left out of compile.
    #[serde(default)]
    pub comment_resolved: bool,
    /// Highlighter color on this run, with or without a note.
    #[serde(default)]
    pub highlight: Option<Highlight>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoteKind {
    #[default]
    Comment,
    Question,
    Revise,
    Check,
}

impl NoteKind {
    pub const ALL: [NoteKind; 4] = [
        NoteKind::Comment,
        NoteKind::Question,
        NoteKind::Revise,
        NoteKind::Check,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            NoteKind::Comment => "comment",
            NoteKind::Question => "question",
            NoteKind::Revise => "revise",
            NoteKind::Check => "check",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            NoteKind::Comment => "Comment",
            NoteKind::Question => "Question",
            NoteKind::Revise => "Revise",
            NoteKind::Check => "Check",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_str() == id)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Highlight {
    Yellow,
    Green,
    Blue,
    Pink,
}

impl Highlight {
    pub const ALL: [Highlight; 4] = [
        Highlight::Yellow,
        Highlight::Green,
        Highlight::Blue,
        Highlight::Pink,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Highlight::Yellow => "yellow",
            Highlight::Green => "green",
            Highlight::Blue => "blue",
            Highlight::Pink => "pink",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Highlight::Yellow => "Yellow",
            Highlight::Green => "Green",
            Highlight::Blue => "Blue",
            Highlight::Pink => "Pink",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|color| color.as_str() == id)
    }
}

impl Marks {
    /// The note text, trimmed, when this run carries one.
    pub fn note(&self) -> Option<&str> {
        self.comment
            .as_deref()
            .map(str::trim)
            .filter(|note| !note.is_empty())
    }

    /// The note as compile writes it: open notes only, with the kind in
    /// front when it is not a plain comment.
    pub fn compile_note(&self) -> Option<String> {
        let note = self.note()?;
        if self.comment_resolved {
            return None;
        }
        Some(match self.comment_kind {
            NoteKind::Comment => note.to_string(),
            kind => format!("{}: {note}", kind.label()),
        })
    }

    fn clear_note(&mut self) {
        self.comment = None;
        self.comment_kind = NoteKind::default();
        self.comment_resolved = false;
    }
}

/// Which entry of [`Document::review_items`] an edit is for. Notes and
/// highlights are numbered separately, each in document order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReviewTarget {
    Note(usize),
    Highlight(usize),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReviewKind {
    Note {
        comment: String,
        kind: NoteKind,
        resolved: bool,
    },
    Highlight(Highlight),
}

/// One note or one highlight, with the text it sits on. Neighboring runs in
/// a paragraph that carry the same note are one item, so text that is partly
/// bold is not listed twice.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewItem {
    pub target: ReviewTarget,
    pub text: String,
    pub kind: ReviewKind,
}

/// Where one inline sits, and which note and highlight it belongs to.
struct ReviewSlot {
    block: usize,
    run: usize,
    inline: usize,
    note: Option<usize>,
    highlight: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InlineNote {
    pub text: String,
    pub comment: String,
}

/// Keep http(s) and mailto links. Reject other schemes, including `javascript:`.
pub fn normalize_link(url: &str) -> Option<String> {
    let url = url.trim();
    if url.is_empty() || url.chars().any(|ch| ch.is_control()) {
        return None;
    }
    let lower = url.to_ascii_lowercase();
    if lower.starts_with("https://") || lower.starts_with("http://") || lower.starts_with("mailto:")
    {
        return Some(url.to_string());
    }
    if lower.contains(':') {
        return None;
    }
    Some(url.to_string())
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

    pub fn comments(&self) -> Vec<InlineNote> {
        let mut notes = Vec::new();
        for block in &self.blocks {
            for inlines in inline_runs(&block.kind) {
                for inline in inlines {
                    let Some(comment) = inline.marks.comment.as_ref() else {
                        continue;
                    };
                    let comment = comment.trim();
                    if comment.is_empty() {
                        continue;
                    }
                    notes.push(InlineNote {
                        text: inline.text.clone(),
                        comment: comment.to_string(),
                    });
                }
            }
        }
        notes
    }

    /// Notes and highlights in document order.
    pub fn review_items(&self) -> Vec<ReviewItem> {
        let mut items: Vec<ReviewItem> = Vec::new();
        let mut note_at: Vec<usize> = Vec::new();
        let mut highlight_at: Vec<usize> = Vec::new();
        for slot in self.review_slots() {
            let inline = &inline_runs(&self.blocks[slot.block].kind)[slot.run][slot.inline];
            if let Some(index) = slot.note {
                if index == note_at.len() {
                    note_at.push(items.len());
                    items.push(ReviewItem {
                        target: ReviewTarget::Note(index),
                        text: String::new(),
                        kind: ReviewKind::Note {
                            comment: inline.marks.note().unwrap_or_default().to_string(),
                            kind: inline.marks.comment_kind,
                            resolved: inline.marks.comment_resolved,
                        },
                    });
                }
                items[note_at[index]].text.push_str(&inline.text);
            }
            if let (Some(index), Some(color)) = (slot.highlight, inline.marks.highlight) {
                if index == highlight_at.len() {
                    highlight_at.push(items.len());
                    items.push(ReviewItem {
                        target: ReviewTarget::Highlight(index),
                        text: String::new(),
                        kind: ReviewKind::Highlight(color),
                    });
                }
                items[highlight_at[index]].text.push_str(&inline.text);
            }
        }
        items
    }

    pub fn set_note_resolved(&mut self, note: usize, resolved: bool) -> bool {
        self.edit_review(ReviewTarget::Note(note), |marks| {
            marks.comment_resolved = resolved
        })
    }

    pub fn set_note_kind(&mut self, note: usize, kind: NoteKind) -> bool {
        self.edit_review(ReviewTarget::Note(note), |marks| marks.comment_kind = kind)
    }

    /// Blank text is refused. Use [`Document::remove_review`] to delete a note.
    pub fn set_note_text(&mut self, note: usize, text: &str) -> bool {
        let text = text.trim();
        if text.is_empty() {
            return false;
        }
        self.edit_review(ReviewTarget::Note(note), |marks| {
            marks.comment = Some(text.to_string())
        })
    }

    /// Take a note or a highlight off its text. The text stays.
    pub fn remove_review(&mut self, target: ReviewTarget) -> bool {
        self.edit_review(target, |marks| match target {
            ReviewTarget::Note(_) => marks.clear_note(),
            ReviewTarget::Highlight(_) => marks.highlight = None,
        })
    }

    fn edit_review(&mut self, target: ReviewTarget, mut change: impl FnMut(&mut Marks)) -> bool {
        let mut changed = false;
        for slot in self.review_slots() {
            let hit = match target {
                ReviewTarget::Note(index) => slot.note == Some(index),
                ReviewTarget::Highlight(index) => slot.highlight == Some(index),
            };
            if !hit {
                continue;
            }
            let mut runs = inline_runs_mut(&mut self.blocks[slot.block].kind);
            let marks = &mut runs[slot.run][slot.inline].marks;
            let before = marks.clone();
            change(marks);
            changed |= *marks != before;
        }
        if changed {
            self.updated_at = Timestamp::now();
        }
        changed
    }

    /// Let `change` rewrite the text of each run. It returns whether it
    /// changed that run. Formatting, notes, and highlights stay on the run.
    pub fn edit_text_runs(&mut self, mut change: impl FnMut(&mut String) -> bool) -> bool {
        let mut changed = false;
        for block in &mut self.blocks {
            for inlines in inline_runs_mut(&mut block.kind) {
                for inline in inlines.iter_mut() {
                    changed |= change(&mut inline.text);
                }
            }
        }
        if changed {
            self.updated_at = Timestamp::now();
        }
        changed
    }

    fn review_slots(&self) -> Vec<ReviewSlot> {
        let mut slots = Vec::new();
        let (mut notes, mut highlights) = (0usize, 0usize);
        for (block, item) in self.blocks.iter().enumerate() {
            for (run, inlines) in inline_runs(&item.kind).into_iter().enumerate() {
                let mut open_note: Option<(&str, NoteKind, bool)> = None;
                let mut open_highlight: Option<Highlight> = None;
                for (inline, piece) in inlines.iter().enumerate() {
                    if piece.text.is_empty() {
                        continue;
                    }
                    let marks = &piece.marks;
                    let note = marks
                        .note()
                        .map(|text| (text, marks.comment_kind, marks.comment_resolved));
                    if note.is_some() && note != open_note {
                        notes += 1;
                    }
                    if marks.highlight.is_some() && marks.highlight != open_highlight {
                        highlights += 1;
                    }
                    open_note = note;
                    open_highlight = marks.highlight;
                    slots.push(ReviewSlot {
                        block,
                        run,
                        inline,
                        note: note.map(|_| notes - 1),
                        highlight: marks.highlight.map(|_| highlights - 1),
                    });
                }
            }
        }
        slots
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
            BlockKind::List { items, .. } => items
                .iter()
                .map(|item| join_inlines(&item.inlines))
                .collect::<Vec<_>>()
                .join("\n"),
            BlockKind::SceneBreak => String::new(),
            _ => inline_runs(&self.kind)
                .into_iter()
                .map(join_inlines)
                .collect::<Vec<_>>()
                .join("\n"),
        }
    }
}

fn inline_runs(kind: &BlockKind) -> Vec<&[Inline]> {
    match kind {
        BlockKind::Paragraph { inlines }
        | BlockKind::Heading { inlines, .. }
        | BlockKind::Quote { inlines }
        | BlockKind::Code { inlines } => vec![inlines.as_slice()],
        BlockKind::List { items, .. } => items.iter().map(|item| item.inlines.as_slice()).collect(),
        BlockKind::SceneBreak => Vec::new(),
    }
}

fn inline_runs_mut(kind: &mut BlockKind) -> Vec<&mut Vec<Inline>> {
    match kind {
        BlockKind::Paragraph { inlines }
        | BlockKind::Heading { inlines, .. }
        | BlockKind::Quote { inlines }
        | BlockKind::Code { inlines } => vec![inlines],
        BlockKind::List { items, .. } => items.iter_mut().map(|item| &mut item.inlines).collect(),
        BlockKind::SceneBreak => Vec::new(),
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

    #[test]
    fn older_marks_without_comment_still_load() {
        let inline = Inline {
            id: Id::new(),
            text: "Harbor".into(),
            marks: Marks {
                link: Some("https://example.com/tide".into()),
                ..Marks::default()
            },
        };
        let mut value = serde_json::to_value(&inline).unwrap();
        value["marks"].as_object_mut().unwrap().remove("comment");
        let restored: Inline = serde_json::from_value(value).unwrap();
        assert!(restored.marks.comment.is_none());
        assert_eq!(
            restored.marks.link.as_deref(),
            Some("https://example.com/tide")
        );
    }

    #[test]
    fn comments_skip_blank_notes_and_scene_breaks() {
        let mut doc = Document::new(Id::new());
        doc.blocks = vec![
            Block {
                id: Id::new(),
                kind: BlockKind::Paragraph {
                    inlines: vec![Inline {
                        id: Id::new(),
                        text: "harbor".into(),
                        marks: Marks {
                            comment: Some("  check the tide  ".into()),
                            ..Marks::default()
                        },
                    }],
                },
            },
            Block {
                id: Id::new(),
                kind: BlockKind::SceneBreak,
            },
            Block {
                id: Id::new(),
                kind: BlockKind::Paragraph {
                    inlines: vec![Inline {
                        id: Id::new(),
                        text: "skip".into(),
                        marks: Marks {
                            comment: Some("   ".into()),
                            ..Marks::default()
                        },
                    }],
                },
            },
        ];
        let notes = doc.comments();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].text, "harbor");
        assert_eq!(notes[0].comment, "check the tide");
        assert_eq!(doc.to_plain_text(), "harbor\n\nskip");
    }

    fn marked(text: &str, marks: Marks) -> Inline {
        Inline {
            id: Id::new(),
            text: text.into(),
            marks,
        }
    }

    fn review_document() -> Document {
        let tide = Marks {
            comment: Some("check the tide".into()),
            comment_kind: NoteKind::Check,
            ..Marks::default()
        };
        let mut doc = Document::new(Id::new());
        doc.blocks = vec![
            Block {
                id: Id::new(),
                kind: BlockKind::Paragraph {
                    inlines: vec![
                        Inline::text("The "),
                        marked("harbor ", tide.clone()),
                        marked(
                            "was",
                            Marks {
                                bold: true,
                                highlight: Some(Highlight::Yellow),
                                ..tide.clone()
                            },
                        ),
                        marked(
                            " quiet",
                            Marks {
                                highlight: Some(Highlight::Yellow),
                                ..Marks::default()
                            },
                        ),
                        Inline::text("."),
                    ],
                },
            },
            Block {
                id: Id::new(),
                kind: BlockKind::Paragraph {
                    inlines: vec![marked("Too long?", {
                        Marks {
                            comment: Some("cut".into()),
                            ..Marks::default()
                        }
                    })],
                },
            },
        ];
        doc
    }

    #[test]
    fn review_items_join_runs_that_share_a_note_or_a_highlight() {
        let items = review_document().review_items();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].target, ReviewTarget::Note(0));
        assert_eq!(items[0].text, "harbor was");
        assert_eq!(
            items[0].kind,
            ReviewKind::Note {
                comment: "check the tide".into(),
                kind: NoteKind::Check,
                resolved: false,
            }
        );
        assert_eq!(items[1].target, ReviewTarget::Highlight(0));
        assert_eq!(items[1].text, "was quiet");
        assert_eq!(items[1].kind, ReviewKind::Highlight(Highlight::Yellow));
        assert_eq!(items[2].target, ReviewTarget::Note(1));
        assert_eq!(items[2].text, "Too long?");
    }

    #[test]
    fn review_edits_reach_every_run_of_the_item_and_leave_the_text() {
        let mut doc = review_document();
        let text = doc.to_plain_text();

        assert!(doc.set_note_resolved(0, true));
        assert!(!doc.set_note_resolved(0, true));
        assert!(doc.set_note_kind(1, NoteKind::Revise));
        assert!(doc.set_note_text(1, "  cut this line  "));
        assert!(!doc.set_note_text(1, "   "));
        let items = doc.review_items();
        assert_eq!(items.len(), 3);
        assert!(matches!(items[0].kind, ReviewKind::Note { resolved: true, .. }));
        assert_eq!(items[0].text, "harbor was");
        assert_eq!(
            items[2].kind,
            ReviewKind::Note {
                comment: "cut this line".into(),
                kind: NoteKind::Revise,
                resolved: false,
            }
        );

        assert!(doc.remove_review(ReviewTarget::Highlight(0)));
        assert!(doc.remove_review(ReviewTarget::Note(0)));
        assert!(!doc.remove_review(ReviewTarget::Note(5)));
        let items = doc.review_items();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].target, ReviewTarget::Note(0));
        assert_eq!(items[0].text, "Too long?");
        assert_eq!(doc.to_plain_text(), text);
    }

    #[test]
    fn compile_note_skips_resolved_notes_and_names_the_kind() {
        let mut marks = Marks {
            comment: Some(" is this 1840? ".into()),
            comment_kind: NoteKind::Question,
            ..Marks::default()
        };
        assert_eq!(marks.compile_note().as_deref(), Some("Question: is this 1840?"));
        marks.comment_kind = NoteKind::Comment;
        assert_eq!(marks.compile_note().as_deref(), Some("is this 1840?"));
        marks.comment_resolved = true;
        assert!(marks.compile_note().is_none());
    }

    #[test]
    fn marks_saved_before_review_fields_still_load() {
        let mut value = serde_json::to_value(Inline::text("Harbor")).unwrap();
        let marks = value["marks"].as_object_mut().unwrap();
        marks.remove("comment_kind");
        marks.remove("comment_resolved");
        marks.remove("highlight");
        marks.insert("comment".into(), "tide".into());
        let restored: Inline = serde_json::from_value(value).unwrap();
        assert_eq!(restored.marks.note(), Some("tide"));
        assert_eq!(restored.marks.comment_kind, NoteKind::Comment);
        assert!(!restored.marks.comment_resolved);
        assert!(restored.marks.highlight.is_none());
    }

    #[test]
    fn normalize_link_rejects_script_urls() {
        assert_eq!(
            normalize_link(" https://example.com/a "),
            Some("https://example.com/a".into())
        );
        assert_eq!(
            normalize_link("mailto:writer@example.com"),
            Some("mailto:writer@example.com".into())
        );
        assert!(normalize_link("javascript:alert(1)").is_none());
        assert!(normalize_link("data:text/html,hi").is_none());
    }
}
