//! Tracked changes without a server.
//!
//! Tracking keeps a copy of the document from the moment it was turned on,
//! the baseline. The changes are whatever differs between that copy and the
//! text now, one per paragraph. Accepting a change copies it into the
//! baseline. Rejecting it puts the baseline's paragraph back in the text.
//!
//! Only text is tracked. A paragraph whose words are the same on both sides
//! is not a change, whatever happened to its formatting, notes, or highlights.

use writing_studio_core::Timestamp;

use crate::diff::{compare_units, DiffSpan, ParagraphChange};
use crate::model::{Block, BlockKind, Document};

/// One paragraph that was rewritten, added, or removed since tracking began.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrackedChange {
    /// `Changed`, `Added`, or `Removed`. Never `Same`.
    pub change: ParagraphChange,
    pub spans: Vec<DiffSpan>,
    /// The block in the baseline. `None` for an added paragraph.
    pub baseline_block: Option<usize>,
    /// The block in the current text. `None` for a removed paragraph.
    pub current_block: Option<usize>,
    /// Where an added paragraph goes in the baseline when it is accepted.
    pub baseline_at: usize,
    /// Where a removed paragraph goes in the current text when it is rejected.
    pub current_at: usize,
}

/// Changes in document order. Indexes are only good until either document
/// changes, so ask again after every accept or reject.
pub fn tracked_changes(baseline: &Document, current: &Document) -> Vec<TrackedChange> {
    let old: Vec<String> = baseline.blocks.iter().map(block_text).collect();
    let new: Vec<String> = current.blocks.iter().map(block_text).collect();
    let old: Vec<&str> = old.iter().map(String::as_str).collect();
    let new: Vec<&str> = new.iter().map(String::as_str).collect();
    compare_units(&old, &new)
        .into_iter()
        .filter(|unit| unit.change != ParagraphChange::Same)
        .map(|unit| TrackedChange {
            change: unit.change,
            spans: unit.spans,
            baseline_block: unit.old,
            current_block: unit.new,
            baseline_at: unit.old_at,
            current_at: unit.new_at,
        })
        .collect()
}

/// Keep a change: the baseline takes the current paragraph.
pub fn accept_change(baseline: &mut Document, current: &Document, change: &TrackedChange) -> bool {
    apply(
        &mut baseline.blocks,
        &current.blocks,
        change.baseline_block,
        change.current_block,
        change.baseline_at,
    )
}

/// Undo a change: the current text takes the baseline's paragraph back.
pub fn reject_change(baseline: &Document, current: &mut Document, change: &TrackedChange) -> bool {
    let changed = apply(
        &mut current.blocks,
        &baseline.blocks,
        change.current_block,
        change.baseline_block,
        change.current_at,
    );
    if changed {
        current.updated_at = Timestamp::now();
    }
    changed
}

/// Make `target` agree with `source` for one paragraph. `mine` is the
/// paragraph in `target`, `theirs` the one in `source`; either can be absent.
fn apply(
    target: &mut Vec<Block>,
    source: &[Block],
    mine: Option<usize>,
    theirs: Option<usize>,
    insert_at: usize,
) -> bool {
    match (mine, theirs.and_then(|index| source.get(index))) {
        (Some(mine), Some(block)) if mine < target.len() => target[mine] = block.clone(),
        (Some(mine), None) if theirs.is_none() && mine < target.len() => {
            target.remove(mine);
            if target.is_empty() {
                target.push(Block::paragraph(""));
            }
        }
        (None, Some(block)) => target.insert(insert_at.min(target.len()), block.clone()),
        _ => return false,
    }
    true
}

/// The text a block is compared by. A scene break has no text of its own, so
/// it gets a mark that keeps it apart from an empty paragraph.
fn block_text(block: &Block) -> String {
    match block.kind {
        BlockKind::SceneBreak => "* * *".into(),
        _ => block.plain_text(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::DiffOp;
    use crate::model::{Inline, Marks};
    use writing_studio_core::Id;

    fn document(lines: &[&str]) -> Document {
        Document::from_plain_text(Id::new(), &lines.join("\n"))
    }

    fn kinds(baseline: &Document, current: &Document) -> Vec<ParagraphChange> {
        tracked_changes(baseline, current)
            .iter()
            .map(|change| change.change)
            .collect()
    }

    #[test]
    fn nothing_is_tracked_until_the_text_changes() {
        let baseline = document(&["One.", "Two."]);
        let mut current = baseline.clone();
        assert!(tracked_changes(&baseline, &current).is_empty());

        // Bold is not a tracked change.
        if let BlockKind::Paragraph { inlines } = &mut current.blocks[0].kind {
            inlines[0].marks.bold = true;
        }
        assert!(tracked_changes(&baseline, &current).is_empty());
    }

    #[test]
    fn accepting_every_change_makes_the_baseline_match() {
        let mut baseline = document(&["She opened the door.", "It was cold.", "Nobody spoke."]);
        let current = document(&[
            "She opened the red door.",
            "Nobody spoke.",
            "The harbor waited below the hill.",
        ]);
        assert_eq!(
            kinds(&baseline, &current),
            vec![
                ParagraphChange::Changed,
                ParagraphChange::Removed,
                ParagraphChange::Added
            ]
        );
        let mut steps = 0;
        while let Some(change) = tracked_changes(&baseline, &current).into_iter().last() {
            assert!(accept_change(&mut baseline, &current, &change));
            steps += 1;
        }
        assert_eq!(steps, 3);
        assert_eq!(baseline.to_plain_text(), current.to_plain_text());
    }

    #[test]
    fn rejecting_every_change_in_any_order_restores_the_baseline() {
        let baseline = document(&["One.", "Two goes away.", "Three also goes.", "Four stays here."]);
        let edited = document(&["Zero is new.", "One.", "Four stays right here.", "Five."]);
        for from_the_end in [false, true] {
            let mut current = edited.clone();
            loop {
                let changes = tracked_changes(&baseline, &current);
                let Some(change) = (if from_the_end {
                    changes.last()
                } else {
                    changes.first()
                }) else {
                    break;
                };
                assert!(reject_change(&baseline, &mut current, change));
            }
            assert_eq!(current.to_plain_text(), baseline.to_plain_text());
        }
    }

    #[test]
    fn rejecting_one_change_leaves_the_others() {
        let baseline = document(&["She opened the door.", "Nobody spoke."]);
        let mut current = document(&["She opened the red door.", "Nobody spoke.", "A new ending."]);
        let changes = tracked_changes(&baseline, &current);
        assert_eq!(changes.len(), 2);
        assert!(changes[0]
            .spans
            .iter()
            .any(|span| span.op == DiffOp::Added && span.text.contains("red")));
        assert!(reject_change(&baseline, &mut current, &changes[0]));
        assert_eq!(
            current.to_plain_text(),
            "She opened the door.\nNobody spoke.\nA new ending."
        );
        assert_eq!(kinds(&baseline, &current), vec![ParagraphChange::Added]);
    }

    #[test]
    fn rejecting_a_rewrite_brings_back_the_old_formatting() {
        let mut baseline = document(&["The harbor was quiet."]);
        if let BlockKind::Paragraph { inlines } = &mut baseline.blocks[0].kind {
            *inlines = vec![
                Inline::text("The "),
                Inline {
                    id: Id::new(),
                    text: "harbor".into(),
                    marks: Marks {
                        italic: true,
                        ..Marks::default()
                    },
                },
                Inline::text(" was quiet."),
            ];
        }
        let mut current = document(&["The harbor was loud."]);
        let change = tracked_changes(&baseline, &current).remove(0);
        assert!(reject_change(&baseline, &mut current, &change));
        assert!(current.to_html().contains("<em>harbor</em>"));
    }

    #[test]
    fn deleting_everything_and_rejecting_it_never_leaves_an_empty_document() {
        let baseline = document(&["Only line."]);
        let mut current = document(&["Only line.", "Extra."]);
        let change = tracked_changes(&baseline, &current).remove(0);
        assert!(reject_change(&baseline, &mut current, &change));
        assert_eq!(current.blocks.len(), 1);

        let mut emptied = Document::new(Id::new());
        let cleared = tracked_changes(&baseline, &emptied);
        assert_eq!(cleared.len(), 1);
        assert_eq!(cleared[0].change, ParagraphChange::Changed);
        assert!(reject_change(&baseline, &mut emptied, &cleared[0]));
        assert_eq!(emptied.to_plain_text(), "Only line.");
    }

    #[test]
    fn a_scene_break_is_tracked_apart_from_an_empty_paragraph() {
        let baseline = document(&["One.", "", "Two."]);
        let mut current = baseline.clone();
        current.blocks[1].kind = BlockKind::SceneBreak;
        let changes = tracked_changes(&baseline, &current);
        assert_eq!(changes.len(), 1);
        assert!(reject_change(&baseline, &mut current, &changes[0]));
        assert!(matches!(current.blocks[1].kind, BlockKind::Paragraph { .. }));
    }
}
