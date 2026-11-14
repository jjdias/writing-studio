//! The form of a document that a Markdown file can hold exactly.
//!
//! Almost everything the editor produces is kept as it is. A few things are
//! not, because Markdown has no way to say them that other programs read
//! correctly. Canonical form removes exactly those, so that writing a
//! canonical document to Markdown and reading it back gives the same
//! document:
//!
//! - Bold, italic, and strikethrough do not cover the spaces at the start or
//!   end of the formatted stretch. `**word **` is not bold in most Markdown
//!   readers; `**word** ` is.
//! - A code block is plain text. Formatting, links, notes, and highlights
//!   inside one are dropped.
//! - Neighboring runs with the same formatting are one run, and empty runs
//!   are gone. A list with no items is gone.
//! - A link that would not be accepted as a link is removed from the text it
//!   was on. A note that is blank is no note.
//! - Carriage returns are removed.
//! - A line break at the very end of a paragraph, heading, quote, or list
//!   item is removed. It shows nothing, the editor leaves one behind as a
//!   place for the caret, and Markdown cannot end a paragraph with one.

use writing_studio_core::Id;
use writing_studio_document::{normalize_link, Block, BlockKind, Inline, ListItem, Marks, NoteKind};

/// The blocks in canonical form. Ids are kept where a block survives.
pub fn canonical_blocks(blocks: &[Block]) -> Vec<Block> {
    let mut out: Vec<Block> = blocks
        .iter()
        .filter_map(|block| {
            let kind = match &block.kind {
                BlockKind::Paragraph { inlines } => BlockKind::Paragraph {
                    inlines: canonical_inlines(inlines, false),
                },
                BlockKind::Heading { level, inlines } => BlockKind::Heading {
                    level: (*level).clamp(1, 6),
                    inlines: canonical_inlines(inlines, false),
                },
                BlockKind::Quote { inlines } => BlockKind::Quote {
                    inlines: canonical_inlines(inlines, false),
                },
                BlockKind::Code { inlines } => BlockKind::Code {
                    inlines: canonical_inlines(inlines, true),
                },
                BlockKind::List { ordered, items } => {
                    if items.is_empty() {
                        return None;
                    }
                    BlockKind::List {
                        ordered: *ordered,
                        items: items
                            .iter()
                            .map(|item| ListItem {
                                id: item.id,
                                inlines: canonical_inlines(&item.inlines, false),
                            })
                            .collect(),
                    }
                }
                BlockKind::SceneBreak => BlockKind::SceneBreak,
            };
            Some(Block { id: block.id, kind })
        })
        .collect();
    if out.is_empty() {
        out.push(Block::paragraph(""));
        if let BlockKind::Paragraph { inlines } = &mut out[0].kind {
            inlines.clear();
        }
    }
    out
}

/// Whether two documents hold the same content. Ids are not compared.
pub fn same_content(left: &[Block], right: &[Block]) -> bool {
    left.len() == right.len()
        && left.iter().zip(right).all(|(a, b)| match (&a.kind, &b.kind) {
            (BlockKind::Paragraph { inlines: x }, BlockKind::Paragraph { inlines: y })
            | (BlockKind::Quote { inlines: x }, BlockKind::Quote { inlines: y })
            | (BlockKind::Code { inlines: x }, BlockKind::Code { inlines: y }) => same_inlines(x, y),
            (
                BlockKind::Heading { level: l, inlines: x },
                BlockKind::Heading { level: m, inlines: y },
            ) => l == m && same_inlines(x, y),
            (
                BlockKind::List { ordered: o, items: x },
                BlockKind::List { ordered: p, items: y },
            ) => {
                o == p
                    && x.len() == y.len()
                    && x.iter().zip(y).all(|(i, j)| same_inlines(&i.inlines, &j.inlines))
            }
            (BlockKind::SceneBreak, BlockKind::SceneBreak) => true,
            _ => false,
        })
}

fn same_inlines(left: &[Inline], right: &[Inline]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(a, b)| a.text == b.text && a.marks == b.marks)
}

fn canonical_marks(marks: &Marks, literal: bool) -> Marks {
    if literal {
        return Marks::default();
    }
    let mut out = marks.clone();
    out.link = marks.link.as_deref().and_then(normalize_link);
    match marks.note() {
        Some(note) => out.comment = Some(note.replace('\r', "")),
        None => {
            out.comment = None;
            out.comment_kind = NoteKind::default();
            out.comment_resolved = false;
        }
    }
    out
}

/// What a formatted stretch may not be split across in the file: a note, a
/// highlight, or a link each wrap the text they are on.
fn same_wrapper(a: &Marks, b: &Marks) -> bool {
    a.comment == b.comment
        && a.comment_kind == b.comment_kind
        && a.comment_resolved == b.comment_resolved
        && a.highlight == b.highlight
        && a.link == b.link
}

fn canonical_inlines(inlines: &[Inline], literal: bool) -> Vec<Inline> {
    let mut chars: Vec<(char, Marks)> = Vec::new();
    for inline in inlines {
        let marks = canonical_marks(&inline.marks, literal);
        chars.extend(
            inline
                .text
                .chars()
                .filter(|ch| *ch != '\r')
                .map(|ch| (ch, marks.clone())),
        );
    }

    if !literal {
        while chars.last().is_some_and(|(ch, _)| *ch == '\n') {
            chars.pop();
        }
    }

    // Bold is written outside italic, and italic outside strikethrough, so a
    // stretch of an inner one ends wherever an outer one changes.
    type Get = fn(&Marks) -> bool;
    type Clear = fn(&mut Marks);
    let passes: [(Get, Clear); 3] = [
        (|m| m.bold, |m| m.bold = false),
        (|m| m.italic, |m| m.italic = false),
        (|m| m.strikethrough, |m| m.strikethrough = false),
    ];
    for (index, (get, clear)) in passes.iter().enumerate() {
        let outer = |m: &Marks| -> (bool, bool) {
            match index {
                0 => (false, false),
                1 => (m.bold, false),
                _ => (m.bold, m.italic),
            }
        };
        let mut start = 0;
        while start < chars.len() {
            if !get(&chars[start].1) {
                start += 1;
                continue;
            }
            let mut end = start + 1;
            while end < chars.len()
                && get(&chars[end].1)
                && same_wrapper(&chars[start].1, &chars[end].1)
                && outer(&chars[start].1) == outer(&chars[end].1)
            {
                end += 1;
            }
            let mut first = start;
            while first < end && chars[first].0.is_whitespace() {
                clear(&mut chars[first].1);
                first += 1;
            }
            let mut last = end;
            while last > first && chars[last - 1].0.is_whitespace() {
                clear(&mut chars[last - 1].1);
                last -= 1;
            }
            start = end;
        }
    }

    let mut out: Vec<Inline> = Vec::new();
    for (ch, marks) in chars {
        match out.last_mut() {
            Some(last) if last.marks == marks => last.text.push(ch),
            _ => out.push(Inline {
                id: Id::new(),
                text: ch.to_string(),
                marks,
            }),
        }
    }
    out
}
