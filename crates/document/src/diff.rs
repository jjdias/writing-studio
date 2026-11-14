//! Text comparison between two versions of a document.
//!
//! Paragraphs are matched first. A paragraph that was rewritten is then
//! compared word by word. Only text is compared; bold, links, and inline
//! notes are not part of the result.

use crate::Document;

/// Largest table the comparison will build. Past this, a pair of texts is
/// reported as removed and added without looking for shared parts.
const MAX_CELLS: usize = 4_000_000;

/// A rewritten paragraph is shown word by word only when at least this share
/// of its words survived. Below it, the old and new paragraphs are listed
/// separately.
const PAIR_SIMILARITY: f32 = 0.4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiffOp {
    Same,
    Removed,
    Added,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffSpan {
    pub op: DiffOp,
    pub text: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParagraphChange {
    Same,
    Removed,
    Added,
    Changed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffParagraph {
    pub change: ParagraphChange,
    pub spans: Vec<DiffSpan>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DocumentDiff {
    pub paragraphs: Vec<DiffParagraph>,
    pub words_added: u32,
    pub words_removed: u32,
}

impl DocumentDiff {
    pub fn is_unchanged(&self) -> bool {
        self.paragraphs
            .iter()
            .all(|paragraph| paragraph.change == ParagraphChange::Same)
    }

    pub fn changed_paragraphs(&self) -> usize {
        self.paragraphs
            .iter()
            .filter(|paragraph| paragraph.change != ParagraphChange::Same)
            .count()
    }
}

pub fn compare_documents(old: &Document, new: &Document) -> DocumentDiff {
    compare_text(&old.to_plain_text(), &new.to_plain_text())
}

pub fn compare_text(old: &str, new: &str) -> DocumentDiff {
    let old_lines: Vec<&str> = old.split('\n').collect();
    let new_lines: Vec<&str> = new.split('\n').collect();
    let mut diff = DocumentDiff::default();
    for unit in compare_units(&old_lines, &new_lines) {
        count_words(&mut diff, &unit.spans);
        diff.paragraphs.push(DiffParagraph {
            change: unit.change,
            spans: unit.spans,
        });
    }
    diff
}

/// One paragraph of a comparison, with where it sits on each side.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DiffUnit {
    pub change: ParagraphChange,
    pub spans: Vec<DiffSpan>,
    /// Index on the old side. `None` for an added paragraph.
    pub old: Option<usize>,
    /// Index on the new side. `None` for a removed paragraph.
    pub new: Option<usize>,
    /// Where an added paragraph would go on the old side.
    pub old_at: usize,
    /// Where a removed paragraph would go on the new side.
    pub new_at: usize,
}

/// Compare two lists of paragraphs. Paragraphs are matched whole first; what
/// is left between matches is handled by [`flush_gap`].
pub(crate) fn compare_units(old: &[&str], new: &[&str]) -> Vec<DiffUnit> {
    let mut units = Vec::new();
    let mut gap = Gap::default();
    let (mut old_at, mut new_at) = (0usize, 0usize);
    for (op, line) in sequence_diff(old, new) {
        match op {
            DiffOp::Removed => {
                gap.removed.push(line);
                old_at += 1;
            }
            DiffOp::Added => {
                gap.added.push(line);
                new_at += 1;
            }
            DiffOp::Same => {
                flush_gap(&mut units, &mut gap);
                units.push(DiffUnit {
                    change: ParagraphChange::Same,
                    spans: whole(DiffOp::Same, line),
                    old: Some(old_at),
                    new: Some(new_at),
                    old_at,
                    new_at,
                });
                old_at += 1;
                new_at += 1;
                gap.old_start = old_at;
                gap.new_start = new_at;
            }
        }
    }
    flush_gap(&mut units, &mut gap);
    units
}

#[derive(Default)]
struct Gap<'a> {
    removed: Vec<&'a str>,
    added: Vec<&'a str>,
    old_start: usize,
    new_start: usize,
}

/// Turn a run of removed and added paragraphs into output. Paragraphs are
/// paired in order; a pair that still shares enough words becomes one
/// `Changed` paragraph.
fn flush_gap(units: &mut Vec<DiffUnit>, gap: &mut Gap) {
    let pairs = gap.removed.len().min(gap.added.len());
    let removed_unit = |index: usize, new_at: usize| DiffUnit {
        change: ParagraphChange::Removed,
        spans: whole(DiffOp::Removed, gap.removed[index]),
        old: Some(gap.old_start + index),
        new: None,
        old_at: gap.old_start + index,
        new_at,
    };
    let added_unit = |index: usize, old_at: usize| DiffUnit {
        change: ParagraphChange::Added,
        spans: whole(DiffOp::Added, gap.added[index]),
        old: None,
        new: Some(gap.new_start + index),
        old_at,
        new_at: gap.new_start + index,
    };
    for index in 0..pairs {
        let old_tokens = tokens(gap.removed[index]);
        let new_tokens = tokens(gap.added[index]);
        let ops = sequence_diff(&old_tokens, &new_tokens);
        if similar(&ops, &old_tokens, &new_tokens) {
            units.push(DiffUnit {
                change: ParagraphChange::Changed,
                spans: merge_spans(ops),
                old: Some(gap.old_start + index),
                new: Some(gap.new_start + index),
                old_at: gap.old_start + index,
                new_at: gap.new_start + index,
            });
        } else {
            units.push(removed_unit(index, gap.new_start + index));
            units.push(added_unit(index, gap.old_start + index + 1));
        }
    }
    for index in pairs..gap.removed.len() {
        units.push(removed_unit(index, gap.new_start + pairs));
    }
    for index in pairs..gap.added.len() {
        units.push(added_unit(index, gap.old_start + pairs));
    }
    gap.old_start += gap.removed.len();
    gap.new_start += gap.added.len();
    gap.removed.clear();
    gap.added.clear();
}

fn whole(op: DiffOp, line: &str) -> Vec<DiffSpan> {
    vec![DiffSpan {
        op,
        text: line.to_string(),
    }]
}

fn count_words(diff: &mut DocumentDiff, spans: &[DiffSpan]) {
    for span in spans {
        let words = span.text.split_whitespace().count() as u32;
        match span.op {
            DiffOp::Added => diff.words_added += words,
            DiffOp::Removed => diff.words_removed += words,
            DiffOp::Same => {}
        }
    }
}

fn similar(ops: &[(DiffOp, &str)], old: &[&str], new: &[&str]) -> bool {
    let words = |tokens: &[&str]| tokens.iter().filter(|token| is_word(token)).count();
    // A blank paragraph that gained text, or text that was cleared, is one
    // change to that paragraph.
    if words(old) == 0 || words(new) == 0 {
        return true;
    }
    let total = words(old) + words(new);
    let kept = ops
        .iter()
        .filter(|(op, token)| *op == DiffOp::Same && is_word(token))
        .count();
    (2 * kept) as f32 / total as f32 >= PAIR_SIMILARITY
}

fn is_word(token: &str) -> bool {
    !token.trim().is_empty()
}

/// Split a paragraph into words and the whitespace between them, so that the
/// pieces join back into the original text.
fn tokens(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut in_space: Option<bool> = None;
    for (index, ch) in text.char_indices() {
        let space = ch.is_whitespace();
        if in_space.is_some_and(|previous| previous != space) {
            out.push(&text[start..index]);
            start = index;
        }
        in_space = Some(space);
    }
    if start < text.len() {
        out.push(&text[start..]);
    }
    out
}

fn merge_spans(ops: Vec<(DiffOp, &str)>) -> Vec<DiffSpan> {
    let mut spans: Vec<DiffSpan> = Vec::new();
    for (op, text) in ops {
        match spans.last_mut() {
            Some(last) if last.op == op => last.text.push_str(text),
            _ => spans.push(DiffSpan {
                op,
                text: text.to_string(),
            }),
        }
    }
    spans
}

/// Longest-common-subsequence diff. Within a changed run, removals come
/// before additions.
fn sequence_diff<'a>(old: &[&'a str], new: &[&'a str]) -> Vec<(DiffOp, &'a str)> {
    let prefix = old
        .iter()
        .zip(new.iter())
        .take_while(|(a, b)| a == b)
        .count();
    let suffix = old[prefix..]
        .iter()
        .rev()
        .zip(new[prefix..].iter().rev())
        .take_while(|(a, b)| a == b)
        .count();
    let old_mid = &old[prefix..old.len() - suffix];
    let new_mid = &new[prefix..new.len() - suffix];

    let mut out: Vec<(DiffOp, &str)> = Vec::with_capacity(old.len().max(new.len()));
    out.extend(old[..prefix].iter().map(|item| (DiffOp::Same, *item)));
    middle_diff(old_mid, new_mid, &mut out);
    out.extend(
        old[old.len() - suffix..]
            .iter()
            .map(|item| (DiffOp::Same, *item)),
    );
    out
}

fn middle_diff<'a>(old: &[&'a str], new: &[&'a str], out: &mut Vec<(DiffOp, &'a str)>) {
    let rows = old.len();
    let cols = new.len();
    if rows == 0 || cols == 0 || rows.saturating_mul(cols) > MAX_CELLS {
        out.extend(old.iter().map(|item| (DiffOp::Removed, *item)));
        out.extend(new.iter().map(|item| (DiffOp::Added, *item)));
        return;
    }

    // table[i][j] is the length of the common subsequence of old[i..] and new[j..].
    let width = cols + 1;
    let mut table = vec![0u32; (rows + 1) * width];
    for i in (0..rows).rev() {
        for j in (0..cols).rev() {
            table[i * width + j] = if old[i] == new[j] {
                table[(i + 1) * width + j + 1] + 1
            } else {
                table[(i + 1) * width + j].max(table[i * width + j + 1])
            };
        }
    }

    let (mut i, mut j) = (0, 0);
    while i < rows && j < cols {
        if old[i] == new[j] {
            out.push((DiffOp::Same, old[i]));
            i += 1;
            j += 1;
        } else if table[(i + 1) * width + j] >= table[i * width + j + 1] {
            out.push((DiffOp::Removed, old[i]));
            i += 1;
        } else {
            out.push((DiffOp::Added, new[j]));
            j += 1;
        }
    }
    out.extend(old[i..].iter().map(|item| (DiffOp::Removed, *item)));
    out.extend(new[j..].iter().map(|item| (DiffOp::Added, *item)));
}

#[cfg(test)]
mod tests {
    use super::*;
    use writing_studio_core::Id;

    fn side(diff: &DocumentDiff, skip: DiffOp) -> String {
        diff.paragraphs
            .iter()
            .filter(|paragraph| {
                !matches!(
                    (paragraph.change, skip),
                    (ParagraphChange::Removed, DiffOp::Removed)
                        | (ParagraphChange::Added, DiffOp::Added)
                )
            })
            .map(|paragraph| {
                paragraph
                    .spans
                    .iter()
                    .filter(|span| span.op != skip)
                    .map(|span| span.text.as_str())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn identical_text_has_no_changes() {
        let diff = compare_text("One.\n\nTwo.", "One.\n\nTwo.");
        assert!(diff.is_unchanged());
        assert_eq!(diff.paragraphs.len(), 3);
        assert_eq!((diff.words_added, diff.words_removed), (0, 0));
    }

    #[test]
    fn rewritten_paragraph_is_compared_word_by_word() {
        let diff = compare_text(
            "She opened the door slowly.",
            "She opened the red door.",
        );
        assert_eq!(diff.paragraphs.len(), 1);
        let paragraph = &diff.paragraphs[0];
        assert_eq!(paragraph.change, ParagraphChange::Changed);
        assert!(paragraph
            .spans
            .iter()
            .any(|span| span.op == DiffOp::Added && span.text.contains("red")));
        assert!(paragraph
            .spans
            .iter()
            .any(|span| span.op == DiffOp::Removed && span.text.contains("slowly.")));
        assert_eq!(diff.words_added, 2);
        assert_eq!(diff.words_removed, 2);
    }

    #[test]
    fn added_and_removed_paragraphs_keep_their_neighbors() {
        let diff = compare_text("First.\nSecond.\nThird.", "First.\nThird.\nFourth.");
        let changes: Vec<_> = diff.paragraphs.iter().map(|p| p.change).collect();
        assert_eq!(
            changes,
            vec![
                ParagraphChange::Same,
                ParagraphChange::Removed,
                ParagraphChange::Same,
                ParagraphChange::Added,
            ]
        );
        assert_eq!(diff.changed_paragraphs(), 2);
        assert_eq!((diff.words_added, diff.words_removed), (1, 1));
    }

    #[test]
    fn unrelated_paragraphs_are_not_interleaved() {
        let diff = compare_text(
            "The harbor was quiet at dawn.",
            "Nobody answered when Mara knocked twice.",
        );
        let changes: Vec<_> = diff.paragraphs.iter().map(|p| p.change).collect();
        assert_eq!(
            changes,
            vec![ParagraphChange::Removed, ParagraphChange::Added]
        );
    }

    #[test]
    fn units_say_where_each_paragraph_sits_on_both_sides() {
        let old = ["First.", "Second.", "Third."];
        let new = ["First.", "Third.", "The harbor at dawn.", "Fourth, said Mara."];
        let units = compare_units(&old, &new);
        let places: Vec<_> = units
            .iter()
            .map(|unit| (unit.change, unit.old, unit.new, unit.old_at, unit.new_at))
            .collect();
        assert_eq!(
            places,
            vec![
                (ParagraphChange::Same, Some(0), Some(0), 0, 0),
                (ParagraphChange::Removed, Some(1), None, 1, 1),
                (ParagraphChange::Same, Some(2), Some(1), 2, 1),
                (ParagraphChange::Added, None, Some(2), 3, 2),
                (ParagraphChange::Added, None, Some(3), 3, 3),
            ]
        );
    }

    #[test]
    fn a_blank_paragraph_that_gained_text_is_one_change() {
        let diff = compare_text("", "She opened the door.");
        assert_eq!(diff.paragraphs.len(), 1);
        assert_eq!(diff.paragraphs[0].change, ParagraphChange::Changed);
        assert_eq!((diff.words_added, diff.words_removed), (4, 0));
    }

    #[test]
    fn both_sides_can_be_rebuilt_from_the_result() {
        let old = "One  two three.\n\nA line that goes away.\nShared end.";
        let new = "One two, three and four.\n\nShared end.\nA new last line.";
        let diff = compare_text(old, new);
        assert_eq!(side(&diff, DiffOp::Added), old);
        assert_eq!(side(&diff, DiffOp::Removed), new);
    }

    #[test]
    fn compares_documents_by_their_text() {
        let id = Id::new();
        let old = Document::from_plain_text(id, "Original scene.");
        let new = Document::from_plain_text(id, "Changed scene.");
        let diff = compare_documents(&old, &new);
        assert_eq!(diff.paragraphs[0].change, ParagraphChange::Changed);
        assert_eq!((diff.words_added, diff.words_removed), (1, 1));
    }
}
