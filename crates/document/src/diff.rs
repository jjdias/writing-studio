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
    let mut removed: Vec<&str> = Vec::new();
    let mut added: Vec<&str> = Vec::new();

    for (op, line) in sequence_diff(&old_lines, &new_lines) {
        match op {
            DiffOp::Removed => removed.push(line),
            DiffOp::Added => added.push(line),
            DiffOp::Same => {
                flush_gap(&mut diff, &mut removed, &mut added);
                diff.paragraphs.push(whole(ParagraphChange::Same, line));
            }
        }
    }
    flush_gap(&mut diff, &mut removed, &mut added);
    diff
}

/// Turn a run of removed and added paragraphs into output. Paragraphs are
/// paired in order; a pair that still shares enough words becomes one
/// `Changed` paragraph.
fn flush_gap(diff: &mut DocumentDiff, removed: &mut Vec<&str>, added: &mut Vec<&str>) {
    let pairs = removed.len().min(added.len());
    for index in 0..pairs {
        let old_tokens = tokens(removed[index]);
        let new_tokens = tokens(added[index]);
        let ops = sequence_diff(&old_tokens, &new_tokens);
        if similar(&ops, &old_tokens, &new_tokens) {
            let spans = merge_spans(ops);
            count_words(diff, &spans);
            diff.paragraphs.push(DiffParagraph {
                change: ParagraphChange::Changed,
                spans,
            });
        } else {
            push_whole(diff, ParagraphChange::Removed, removed[index]);
            push_whole(diff, ParagraphChange::Added, added[index]);
        }
    }
    for line in &removed[pairs..] {
        push_whole(diff, ParagraphChange::Removed, line);
    }
    for line in &added[pairs..] {
        push_whole(diff, ParagraphChange::Added, line);
    }
    removed.clear();
    added.clear();
}

fn push_whole(diff: &mut DocumentDiff, change: ParagraphChange, line: &str) {
    let paragraph = whole(change, line);
    count_words(diff, &paragraph.spans);
    diff.paragraphs.push(paragraph);
}

fn whole(change: ParagraphChange, line: &str) -> DiffParagraph {
    let op = match change {
        ParagraphChange::Removed => DiffOp::Removed,
        ParagraphChange::Added => DiffOp::Added,
        ParagraphChange::Same | ParagraphChange::Changed => DiffOp::Same,
    };
    DiffParagraph {
        change,
        spans: vec![DiffSpan {
            op,
            text: line.to_string(),
        }],
    }
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
    let total = words(old) + words(new);
    if total == 0 {
        return true;
    }
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
