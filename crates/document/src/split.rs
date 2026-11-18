//! Cutting a document in two at a place in its text, for splitting a scene.
//!
//! The place is counted the way the page counts it: characters of text, with
//! a line break inside a paragraph counting for nothing, because the page
//! shows it as a break and not as a character. In code, where the page keeps
//! the line breaks as text, they count.

use writing_studio_core::Id;

use crate::diff::{compare_units, ParagraphChange};
use crate::model::{Block, BlockKind, Document, Inline, ListItem};
use crate::track::block_text;

/// Where a document is cut: `offset` characters into block `block`. At
/// offset 0 the whole block goes after the cut.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SplitPoint {
    pub block: usize,
    pub offset: usize,
}

impl Document {
    /// The cut `at` characters into the text as the page shows it. `None`
    /// when either side would have no words, as at the very start or end.
    /// At the end of a paragraph the cut goes before the next one, so the
    /// first part does not end with an empty paragraph.
    pub fn split_point(&self, at: usize) -> Option<SplitPoint> {
        let mut seen = 0;
        let mut found = None;
        for (index, block) in self.blocks.iter().enumerate() {
            let length = shown_length(block);
            if at < seen + length {
                found = Some(SplitPoint { block: index, offset: at - seen });
                break;
            }
            seen += length;
        }
        let point = found?;
        let (head, tail) = split_blocks(&self.blocks, point);
        (has_words(&head) && has_words(&tail)).then_some(point)
    }
}

/// The blocks before and after `point`. A paragraph cut in the middle keeps
/// its id in the first part, with its formatting, notes, and highlights on
/// each side.
pub fn split_blocks(blocks: &[Block], point: SplitPoint) -> (Vec<Block>, Vec<Block>) {
    let Some(block) = blocks.get(point.block) else {
        return (blocks.to_vec(), Vec::new());
    };
    let mut head = blocks[..point.block].to_vec();
    let mut tail = Vec::new();
    if point.offset == 0 {
        tail.push(block.clone());
    } else {
        let (first, second) = split_block(block, point.offset);
        head.push(first);
        tail.extend(second);
    }
    tail.extend_from_slice(&blocks[point.block + 1..]);
    (head, tail)
}

/// The tracking baseline cut to match `point` in `current`, so each part
/// keeps the changes in its own text. A paragraph that is the same on both
/// sides is cut at the same place. One that was rewritten stays whole in the
/// first part, where it shows as changed, and the text after the cut shows as
/// added in the second.
pub fn split_baseline(baseline: &Document, current: &Document, point: SplitPoint) -> (Vec<Block>, Vec<Block>) {
    let old: Vec<String> = baseline.blocks.iter().map(block_text).collect();
    let new: Vec<String> = current.blocks.iter().map(block_text).collect();
    let old: Vec<&str> = old.iter().map(String::as_str).collect();
    let new: Vec<&str> = new.iter().map(String::as_str).collect();
    let units = compare_units(&old, &new);
    let cut = match units.iter().find(|unit| unit.new == Some(point.block)) {
        Some(unit) => match (unit.change, unit.old) {
            (ParagraphChange::Same, Some(old)) if point.offset > 0 => {
                return split_blocks(&baseline.blocks, SplitPoint { block: old, offset: point.offset });
            }
            (_, Some(old)) if point.offset == 0 => old,
            (_, Some(old)) => old + 1,
            (_, None) => unit.old_at,
        },
        None => baseline.blocks.len(),
    };
    let cut = cut.min(baseline.blocks.len());
    (baseline.blocks[..cut].to_vec(), baseline.blocks[cut..].to_vec())
}

fn has_words(blocks: &[Block]) -> bool {
    blocks.iter().any(|block| !block.plain_text().trim().is_empty())
}

fn shown_length(block: &Block) -> usize {
    let code = matches!(block.kind, BlockKind::Code { .. });
    let count = |inlines: &[Inline]| -> usize {
        inlines
            .iter()
            .map(|inline| inline.text.chars().filter(|ch| code || *ch != '\n').count())
            .sum()
    };
    match &block.kind {
        BlockKind::Paragraph { inlines }
        | BlockKind::Heading { inlines, .. }
        | BlockKind::Quote { inlines }
        | BlockKind::Code { inlines } => count(inlines),
        BlockKind::List { items, .. } => items.iter().map(|item| count(&item.inlines)).sum(),
        BlockKind::SceneBreak => 0,
    }
}

/// The block up to `offset` and, when anything is left, a new block of the
/// same kind with the rest.
fn split_block(block: &Block, offset: usize) -> (Block, Option<Block>) {
    let code = matches!(block.kind, BlockKind::Code { .. });
    let rest = |kind: BlockKind| Block { id: Id::new(), kind };
    match &block.kind {
        BlockKind::List { ordered, items } => {
            let mut left = offset;
            let mut head_items = Vec::new();
            let mut tail_items = Vec::new();
            for item in items {
                let length = shown_length(&Block {
                    id: item.id,
                    kind: BlockKind::Paragraph { inlines: item.inlines.clone() },
                });
                if !tail_items.is_empty() || left == 0 {
                    tail_items.push(item.clone());
                } else if left >= length {
                    left -= length;
                    head_items.push(item.clone());
                } else {
                    let (first, second) = split_inlines(&item.inlines, left, false);
                    left = 0;
                    head_items.push(ListItem { id: item.id, inlines: first });
                    tail_items.push(ListItem { id: Id::new(), inlines: second });
                }
            }
            let head = Block { id: block.id, kind: BlockKind::List { ordered: *ordered, items: head_items } };
            let tail = (!tail_items.is_empty()).then(|| rest(BlockKind::List { ordered: *ordered, items: tail_items }));
            (head, tail)
        }
        BlockKind::Paragraph { inlines }
        | BlockKind::Heading { inlines, .. }
        | BlockKind::Quote { inlines }
        | BlockKind::Code { inlines } => {
            let (first, second) = split_inlines(inlines, offset, code);
            let head = Block { id: block.id, kind: with_inlines(&block.kind, first) };
            let tail = (!second.is_empty()).then(|| rest(with_inlines(&block.kind, second)));
            (head, tail)
        }
        BlockKind::SceneBreak => (block.clone(), None),
    }
}

fn with_inlines(kind: &BlockKind, inlines: Vec<Inline>) -> BlockKind {
    match kind {
        BlockKind::Heading { level, .. } => BlockKind::Heading { level: *level, inlines },
        BlockKind::Quote { .. } => BlockKind::Quote { inlines },
        BlockKind::Code { .. } => BlockKind::Code { inlines },
        _ => BlockKind::Paragraph { inlines },
    }
}

/// Runs up to `offset` shown characters, and the rest. A run cut in two
/// keeps its marks on both sides. Outside code, a line break at the cut is
/// dropped, so neither part starts or ends with an empty line.
fn split_inlines(inlines: &[Inline], offset: usize, code: bool) -> (Vec<Inline>, Vec<Inline>) {
    let mut left = offset;
    let mut head = Vec::new();
    let mut tail = Vec::new();
    for inline in inlines {
        if left == 0 {
            tail.push(inline.clone());
            continue;
        }
        let mut cut = inline.text.len();
        for (index, ch) in inline.text.char_indices() {
            if left == 0 {
                cut = index;
                break;
            }
            if code || ch != '\n' {
                left -= 1;
            }
        }
        let (first, second) = inline.text.split_at(cut);
        if !first.is_empty() {
            head.push(Inline { id: inline.id, text: first.to_string(), marks: inline.marks.clone() });
        }
        if !second.is_empty() {
            tail.push(Inline { id: Id::new(), text: second.to_string(), marks: inline.marks.clone() });
        }
    }
    if !code {
        if let Some(last) = head.last_mut() {
            last.text = last.text.trim_end_matches('\n').to_string();
        }
        if let Some(first) = tail.first_mut() {
            first.text = first.text.trim_start_matches('\n').to_string();
        }
        head.retain(|inline| !inline.text.is_empty());
        tail.retain(|inline| !inline.text.is_empty());
    }
    (head, tail)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Marks;

    fn document(lines: &[&str]) -> Document {
        Document::from_plain_text(Id::new(), &lines.join("\n"))
    }

    fn texts(blocks: &[Block]) -> Vec<String> {
        blocks.iter().map(Block::plain_text).collect()
    }

    #[test]
    fn a_cut_inside_a_paragraph_keeps_each_side_of_it() {
        let doc = document(&["One two.", "Three four."]);
        // "One two." is 8 characters; 3 more is after "Thr".
        let point = doc.split_point(11).unwrap();
        assert_eq!(point, SplitPoint { block: 1, offset: 3 });
        let (head, tail) = split_blocks(&doc.blocks, point);
        assert_eq!(texts(&head), ["One two.", "Thr"]);
        assert_eq!(texts(&tail), ["ee four."]);
        assert_eq!(head[1].id, doc.blocks[1].id);
        assert_ne!(tail[0].id, doc.blocks[1].id);
    }

    #[test]
    fn at_the_end_of_a_paragraph_the_next_one_starts_the_second_part() {
        let doc = document(&["One.", "", "Two."]);
        let point = doc.split_point(4).unwrap();
        assert_eq!(point, SplitPoint { block: 2, offset: 0 });
        let (head, tail) = split_blocks(&doc.blocks, point);
        assert_eq!(texts(&head), ["One.", ""]);
        assert_eq!(texts(&tail), ["Two."]);
    }

    #[test]
    fn no_cut_where_a_side_would_have_no_words() {
        let doc = document(&["One.", "Two."]);
        assert_eq!(doc.split_point(0), None);
        assert_eq!(doc.split_point(8), None);
        assert_eq!(doc.split_point(99), None);
    }

    #[test]
    fn a_note_on_the_cut_run_goes_to_both_sides() {
        let mut doc = Document::new(Id::new());
        let marks = Marks { comment: Some("Check this".into()), ..Marks::default() };
        doc.blocks = vec![Block {
            id: Id::new(),
            kind: BlockKind::Paragraph {
                inlines: vec![Inline::text("Plain "), Inline { id: Id::new(), text: "noted words".into(), marks }],
            },
        }];
        let point = doc.split_point(11).unwrap();
        let (head, tail) = split_blocks(&doc.blocks, point);
        let notes = |blocks: Vec<Block>| {
            let mut part = Document::new(Id::new());
            part.blocks = blocks;
            part.comments().len()
        };
        assert_eq!(texts(&head), ["Plain noted"]);
        assert_eq!(texts(&tail), [" words"]);
        assert_eq!(notes(head), 1);
        assert_eq!(notes(tail), 1);
    }

    #[test]
    fn a_list_is_cut_between_or_inside_its_items() {
        let mut doc = document(&["Intro."]);
        doc.blocks.push(Block {
            id: Id::new(),
            kind: BlockKind::List {
                ordered: false,
                items: vec![
                    ListItem { id: Id::new(), inlines: vec![Inline::text("Apples")] },
                    ListItem { id: Id::new(), inlines: vec![Inline::text("Pears")] },
                ],
            },
        });
        // After "Intro." and "App".
        let (head, tail) = split_blocks(&doc.blocks, doc.split_point(9).unwrap());
        assert_eq!(texts(&head), ["Intro.", "App"]);
        assert_eq!(texts(&tail), ["les\nPears"]);
        // After "Apples", the second item starts the second part.
        let (head, tail) = split_blocks(&doc.blocks, doc.split_point(12).unwrap());
        assert_eq!(texts(&head), ["Intro.", "Apples"]);
        assert_eq!(texts(&tail), ["Pears"]);
    }

    #[test]
    fn the_baseline_is_cut_so_each_part_keeps_its_own_changes() {
        let baseline = document(&["Same start.", "Old middle.", "Same end."]);
        let current = document(&["Same start.", "New middle.", "Same end."]);
        // Inside the unchanged first paragraph: cut at the same place.
        let point = current.split_point(4).unwrap();
        let (head, tail) = split_baseline(&baseline, &current, point);
        assert_eq!(texts(&head), ["Same"]);
        assert_eq!(texts(&tail), [" start.", "Old middle.", "Same end."]);
        // Inside the rewritten paragraph: it stays whole in the first part.
        let point = current.split_point(15).unwrap();
        let (head, tail) = split_baseline(&baseline, &current, point);
        assert_eq!(texts(&head), ["Same start.", "Old middle."]);
        assert_eq!(texts(&tail), ["Same end."]);
        // Before the last paragraph.
        let point = current.split_point(22).unwrap();
        let (head, tail) = split_baseline(&baseline, &current, point);
        assert_eq!(texts(&head), ["Same start.", "Old middle."]);
        assert_eq!(texts(&tail), ["Same end."]);
    }
}
