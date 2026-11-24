//! Document blocks to Markdown.
//!
//! The dialect is described in `documentation/PROJECT_FORMAT.md`. Blocks
//! are separated by one blank line. Text is escaped so that a character the
//! writer typed is never read as formatting, here or in another program.

use writing_studio_document::{Block, BlockKind, Highlight, Inline, Marks, NoteKind};

use crate::canonical::canonical_blocks;

/// The text a note kind is written with, in front of the note.
pub(crate) fn kind_prefix(kind: NoteKind) -> &'static str {
    match kind {
        NoteKind::Comment => "Comment: ",
        NoteKind::Question => "Question: ",
        NoteKind::Revise => "Revise: ",
        NoteKind::Check => "Check: ",
    }
}

pub(crate) const RESOLVED_PREFIX: &str = "(resolved) ";

/// How a note on text, and a highlight, are written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoteStyle {
    /// As CriticMarkup, with highlights, note kinds, and resolved notes:
    /// everything the scene holds.
    Critic,
    /// For a reader: open notes as numbered footnotes, defined after all
    /// the text. Resolved notes and highlights are left out, and so are
    /// empty paragraphs, which Markdown would show as `<br>`.
    Footnotes,
}

/// Write blocks as Markdown, with notes as CriticMarkup. The blocks are put
/// in canonical form first.
pub fn blocks_to_markdown(blocks: &[Block]) -> String {
    write_markdown(blocks, NoteStyle::Critic)
}

pub fn write_markdown(blocks: &[Block], style: NoteStyle) -> String {
    let blocks = match style {
        NoteStyle::Critic => canonical_blocks(blocks),
        NoteStyle::Footnotes => canonical_blocks(&for_a_reader(blocks)),
    };
    let mut notes: Vec<String> = Vec::new();
    let mut chunks: Vec<String> = Vec::new();
    // The marker the previous list used, so that two lists in a row are not
    // read as one.
    let mut last_list: Option<(bool, bool)> = None;
    for block in &blocks {
        let mut this_list = None;
        let chunk = match &block.kind {
            BlockKind::Paragraph { inlines } if inlines.is_empty() && style == NoteStyle::Footnotes => continue,
            BlockKind::Paragraph { inlines } => {
                if inlines.is_empty() {
                    "<br>".to_string()
                } else {
                    write_line(inlines, Breaks::Backslash(""), false, style, &mut notes)
                }
            }
            BlockKind::Heading { level, inlines } => {
                let marks = "#".repeat(usize::from(*level));
                if inlines.is_empty() {
                    marks
                } else {
                    format!("{marks} {}", write_line(inlines, Breaks::Tag, true, style, &mut notes))
                }
            }
            BlockKind::Quote { inlines } => {
                format!("> {}", write_line(inlines, Breaks::Backslash("> "), false, style, &mut notes))
                    .trim_end_matches(' ')
                    .to_string()
            }
            BlockKind::Code { inlines } => {
                let text: String = inlines.iter().map(|inline| inline.text.as_str()).collect();
                let longest = text
                    .lines()
                    .map(|line| line.trim_start().chars().take_while(|ch| *ch == '`').count())
                    .max()
                    .unwrap_or(0);
                let fence = "`".repeat(longest.max(2) + 1);
                format!("{fence}\n{text}\n{fence}")
            }
            BlockKind::List { ordered, items } => {
                let alternate = matches!(last_list, Some((was, alt)) if was == *ordered && !alt);
                this_list = Some((*ordered, alternate));
                items
                    .iter()
                    .enumerate()
                    .map(|(index, item)| {
                        let marker = match (*ordered, alternate) {
                            (false, false) => "-".to_string(),
                            (false, true) => "+".to_string(),
                            (true, false) => format!("{}.", index + 1),
                            (true, true) => format!("{})", index + 1),
                        };
                        if item.inlines.is_empty() {
                            marker
                        } else {
                            format!("{marker} {}", write_line(&item.inlines, Breaks::Tag, false, style, &mut notes))
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            BlockKind::SceneBreak => "* * *".to_string(),
        };
        last_list = this_list;
        chunks.push(chunk);
    }
    for (index, note) in notes.iter().enumerate() {
        let text = vec![Inline::text(note.replace('\n', " "))];
        let body = write_line(&text, Breaks::Tag, false, NoteStyle::Critic, &mut Vec::new());
        chunks.push(format!("[^{}]: {body}", index + 1));
    }
    let mut out = chunks.join("\n\n");
    out.push('\n');
    out
}

/// How a line break inside a block is written.
#[derive(Clone, Copy)]
enum Breaks {
    /// A backslash at the end of the line; the next line starts with this.
    Backslash(&'static str),
    /// `<br>`, for a block that has to stay on one line.
    Tag,
}

struct LineWriter {
    out: String,
    breaks: Breaks,
    escape_hash: bool,
    /// Where the text of the current line starts in `out`.
    line_at: usize,
    /// Nothing but spaces has been written on this line yet.
    at_start: bool,
    /// Only digits have been written on this line, so `.` or `)` next would
    /// make it a numbered list.
    only_digits: bool,
}

impl LineWriter {
    fn raw(&mut self, text: &str) {
        self.out.push_str(text);
        self.at_start = false;
        self.only_digits = false;
    }

    fn end_line(&mut self) {
        // Spaces at the end of a line are dropped by editors and mean a line
        // break to Markdown, so they are written as character references.
        let line = &self.out[self.line_at..];
        let kept = line.trim_end_matches([' ', '\t']).len();
        let tail: String = line[kept..]
            .chars()
            .map(|ch| if ch == ' ' { "&#32;" } else { "&#9;" })
            .collect();
        self.out.truncate(self.line_at + kept);
        self.out.push_str(&tail);
    }

    fn line_break(&mut self) {
        match self.breaks {
            Breaks::Tag => self.raw("<br>"),
            Breaks::Backslash(prefix) => {
                self.end_line();
                self.out.push_str("\\\n");
                self.out.push_str(prefix);
                self.line_at = self.out.len();
                self.at_start = true;
                self.only_digits = false;
            }
        }
    }

    fn text(&mut self, text: &str) {
        let chars: Vec<char> = text.chars().collect();
        // A link to a note, `[[Title]]`, is written as it is.
        let mut skip_to = 0;
        for (index, ch) in chars.iter().copied().enumerate() {
            if index < skip_to {
                continue;
            }
            // Not before `(`, where `](` would make it a Markdown link.
            let link_here = crate::links::link_at(&chars[index..]).filter(|(length, _, _)| chars.get(index + length) != Some(&'('));
            if let Some((length, _, _)) = link_here {
                let link: String = chars[index..index + length].iter().collect();
                self.raw(&link);
                skip_to = index + length;
                continue;
            }
            let next = chars.get(index + 1).copied();
            if ch == '\n' {
                self.line_break();
                continue;
            }
            if self.at_start {
                match ch {
                    ' ' => {
                        self.out.push_str("&#32;");
                        continue;
                    }
                    '\t' => {
                        self.out.push_str("&#9;");
                        continue;
                    }
                    // `=` too: a line of it under text makes that text a heading.
                    '#' | '>' | '=' => {
                        self.out.push('\\');
                    }
                    '-' | '+' => {
                        // A list marker, or a line of dashes.
                        if next.is_none_or(|n| n == ' ' || n == '\t' || n == '\n' || n == '-') {
                            self.out.push('\\');
                        }
                    }
                    _ => {}
                }
                self.only_digits = ch.is_ascii_digit();
                self.at_start = false;
                if matches!(ch, '#' | '>' | '=' | '-' | '+') {
                    self.out.push(ch);
                    continue;
                }
            } else if self.only_digits {
                if matches!(ch, '.' | ')') {
                    self.out.push('\\');
                }
                self.only_digits = ch.is_ascii_digit();
            }
            match ch {
                '\\' | '*' | '_' | '`' | '[' | ']' | '<' | '{' | '}' => {
                    self.out.push('\\');
                    self.out.push(ch);
                }
                // Another reader takes `~word~` as well as `~~word~~` for
                // strikethrough, so a tilde is always escaped.
                '~' => self.out.push_str("\\~"),
                // One `=` is only a character. Two together, or one that
                // would sit against formatting, could be read as part of a
                // highlight.
                '=' => {
                    let previous = index.checked_sub(1).map(|at| chars[at]);
                    if previous.is_none_or(|p| p == ch) || next.is_none_or(|n| n == ch) {
                        self.out.push('\\');
                    }
                    self.out.push(ch);
                }
                '#' if self.escape_hash => self.out.push_str("\\#"),
                // Written out, so that it can be seen and is not taken for a space.
                '\u{a0}' => self.out.push_str("&nbsp;"),
                // `![` starts an image, and a link may come straight after.
                '!' if next.is_none() => self.out.push_str("\\!"),
                '&' if looks_like_reference(&chars[index + 1..]) => self.out.push_str("\\&"),
                _ => self.out.push(ch),
            }
        }
    }
}

/// `&name;`, `&#12;`, or `&#x1F;` would be read as one character.
fn looks_like_reference(rest: &[char]) -> bool {
    let mut seen = 0;
    for (index, ch) in rest.iter().enumerate() {
        if *ch == ';' {
            return seen > 0;
        }
        if ch.is_ascii_alphanumeric() || (index == 0 && *ch == '#') {
            seen += usize::from(*ch != '#');
            continue;
        }
        return false;
    }
    false
}

/// The blocks with only what a reader is shown: highlights and resolved
/// notes are taken off, and a note's kind is written into its text.
fn for_a_reader(blocks: &[Block]) -> Vec<Block> {
    let mut document = writing_studio_document::Document::new(writing_studio_core::Id::new());
    document.blocks = blocks.to_vec();
    document.edit_marks(|marks| {
        marks.highlight = None;
        let note = marks.reader_note();
        marks.comment = note;
        marks.footnote = None;
        marks.comment_kind = NoteKind::Comment;
        marks.comment_resolved = false;
    });
    document.blocks
}

fn write_line(
    inlines: &[Inline],
    breaks: Breaks,
    escape_hash: bool,
    style: NoteStyle,
    notes: &mut Vec<String>,
) -> String {
    let mut writer = LineWriter {
        out: String::new(),
        breaks,
        escape_hash,
        line_at: 0,
        at_start: true,
        only_digits: false,
    };
    let mut start = 0;
    while start < inlines.len() {
        let marks = &inlines[start].marks;
        let end = run_end(inlines, start, inlines.len(), same_review);
        let wrapped = style == NoteStyle::Critic && (marks.comment.is_some() || marks.highlight.is_some());
        if wrapped {
            writer.raw("{==");
        }
        let mut link_start = start;
        while link_start < end {
            let link = inlines[link_start].marks.link.clone();
            let link_end = run_end(inlines, link_start, end, |a, b| a.link == b.link);
            if link.is_some() {
                writer.raw("[");
            }
            write_formatted(&mut writer, &inlines[link_start..link_end]);
            if let Some(url) = link {
                writer.raw(&format!("]({})", link_target(&url)));
            }
            link_start = link_end;
        }
        if wrapped {
            writer.raw("==}");
            writer.raw(&review_suffix(marks));
        }
        if let (NoteStyle::Footnotes, Some(note)) = (style, marks.comment.as_ref()) {
            notes.push(note.clone());
            writer.raw(&format!("[^{}]", notes.len()));
        }
        start = end;
    }
    writer.end_line();
    writer.out
}

fn run_end(inlines: &[Inline], start: usize, limit: usize, same: fn(&Marks, &Marks) -> bool) -> usize {
    let mut end = start + 1;
    while end < limit && same(&inlines[start].marks, &inlines[end].marks) {
        end += 1;
    }
    end
}

fn same_review(a: &Marks, b: &Marks) -> bool {
    a.comment == b.comment
        && a.comment_kind == b.comment_kind
        && a.comment_resolved == b.comment_resolved
        && a.highlight == b.highlight
}

/// Bold outside italic outside strikethrough outside underline.
const BOLD: usize = 0;
const ITALIC: usize = 1;
const STRIKE: usize = 2;
const UNDERLINE: usize = 3;
const MARKDOWN: [&str; 4] = ["**", "*", "~~", "<u>"];
const TAG_OPEN: [&str; 4] = ["<strong>", "<em>", "<s>", "<u>"];
const TAG_CLOSE: [&str; 4] = ["</strong>", "</em>", "</s>", "</u>"];

fn wanted(marks: &Marks) -> [bool; 4] {
    [marks.bold, marks.italic, marks.strikethrough, marks.underline]
}

/// What sits next to a `*` or `~~`. Other Markdown readers decide from this
/// whether the mark opens or closes anything.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Beside {
    /// The start or end of the line, or a space.
    Space,
    Letter,
    /// Punctuation, a symbol, or other formatting.
    Mark,
}

fn beside(ch: Option<char>) -> Beside {
    match ch {
        None => Beside::Space,
        Some(ch) if ch.is_whitespace() => Beside::Space,
        Some(ch) if ch.is_alphanumeric() => Beside::Letter,
        Some(_) => Beside::Mark,
    }
}

/// `**` opens bold for another reader unless it has a letter before it and a
/// mark after it, as in `word**“quoted”**`.
fn cannot_open(before: Beside, after: Beside) -> bool {
    after == Beside::Space || (after == Beside::Mark && before == Beside::Letter)
}

/// The mirror: `**` closes unless it has a mark before it and a letter after.
fn cannot_close(before: Beside, after: Beside) -> bool {
    before == Beside::Space || (before == Beside::Mark && after == Beside::Letter)
}

/// Which formatting is open, and how each was opened.
#[derive(Default)]
struct Formatting {
    open: [bool; 4],
    /// Opened with a tag such as `<strong>`, not with `**`.
    tag: [bool; 4],
    /// Where the opening mark is in the output.
    at: [usize; 4],
}

impl LineWriter {
    fn before(&self) -> Beside {
        if self.out.len() == self.line_at {
            Beside::Space
        } else {
            beside(self.out.chars().next_back())
        }
    }
}

impl Formatting {
    fn open(&mut self, writer: &mut LineWriter, index: usize, tag: bool) {
        self.open[index] = true;
        self.tag[index] = tag;
        self.at[index] = writer.out.len();
        writer.raw(if tag { TAG_OPEN[index] } else { MARKDOWN[index] });
    }

    fn close(&mut self, writer: &mut LineWriter, index: usize) {
        writer.raw(if self.tag[index] || index == UNDERLINE {
            TAG_CLOSE[index]
        } else {
            MARKDOWN[index]
        });
        self.open[index] = false;
        self.tag[index] = false;
    }

    /// Change an opening `**` that is already written into `<strong>`,
    /// because the place it has to close would not be read as closing it.
    fn use_tag(&mut self, writer: &mut LineWriter, index: usize) {
        if self.tag[index] {
            return;
        }
        let at = self.at[index];
        let grown = TAG_OPEN[index].len() - MARKDOWN[index].len();
        writer
            .out
            .replace_range(at..at + MARKDOWN[index].len(), TAG_OPEN[index]);
        for other in 0..4 {
            if self.open[other] && self.at[other] > at {
                self.at[other] += grown;
            }
        }
        if writer.line_at > at {
            writer.line_at += grown;
        }
        self.tag[index] = true;
        // `word***x` opened bold and italic together. With italic now a tag,
        // the `**` is left between a letter and a mark, where it opens
        // nothing, so bold becomes a tag too.
        if index == ITALIC
            && self.open[BOLD]
            && !self.tag[BOLD]
            && self.at[BOLD] + MARKDOWN[BOLD].len() == at
            && beside(writer.out[..self.at[BOLD]].chars().next_back()) == Beside::Letter
        {
            self.use_tag(writer, BOLD);
        }
    }

    /// Close and open formatting so that exactly `want` is open. `next` is
    /// the first character of the text that follows, if any follows before
    /// the end of this stretch.
    fn set(&mut self, writer: &mut LineWriter, want: [bool; 4], next: Option<char>) {
        let Some(first) = (0..4).find(|index| self.open[*index] != want[*index]) else {
            return;
        };
        let opens_any = (first..4).any(|index| want[index]);
        // What follows everything that closes here.
        let after_closing = if opens_any { Beside::Mark } else { beside(next) };

        let closing = |me: &Self, index: usize| index >= first && me.open[index];
        if closing(self, UNDERLINE) {
            self.close(writer, UNDERLINE);
        }
        if closing(self, STRIKE) {
            let stars_follow = closing(self, ITALIC) || closing(self, BOLD);
            let after = if stars_follow { Beside::Mark } else { after_closing };
            if cannot_close(writer.before(), after) {
                self.use_tag(writer, STRIKE);
            }
            self.close(writer, STRIKE);
        }
        // `*` and `**` side by side are read as one run of stars.
        let closed_stars = closing(self, ITALIC) || closing(self, BOLD);
        if closed_stars {
            let before = writer.before();
            for index in [ITALIC, BOLD] {
                if !closing(self, index) {
                    continue;
                }
                let tag_follows = index == ITALIC && closing(self, BOLD) && self.tag[BOLD];
                let after = if tag_follows { Beside::Mark } else { after_closing };
                let before = if index == BOLD && self.tag[ITALIC] {
                    Beside::Mark
                } else {
                    before
                };
                if cannot_close(before, after) {
                    self.use_tag(writer, index);
                }
            }
            for index in [ITALIC, BOLD] {
                if closing(self, index) {
                    self.close(writer, index);
                }
            }
        }

        let opening = |index: usize| index >= first && want[index];
        let inner_opens = |index: usize| (index + 1..4).any(opening);
        if opening(BOLD) || opening(ITALIC) {
            // A run that closes and opens at once, as in `**a***b*`, is read
            // differently by different readers. Tags say it plainly.
            let innermost = if opening(ITALIC) { ITALIC } else { BOLD };
            let after = if inner_opens(innermost) {
                Beside::Mark
            } else {
                beside(next)
            };
            // With bold already open, a `*` that has marks or letters on both
            // sides can be read as closing it instead of opening italic.
            let before = writer.before();
            let other_open = self.open[BOLD] || self.open[ITALIC];
            let could_close = other_open && !cannot_close(before, after);
            let tag = closed_stars || could_close || cannot_open(before, after);
            for index in [BOLD, ITALIC] {
                if opening(index) {
                    self.open(writer, index, tag);
                }
            }
        }
        if opening(STRIKE) {
            let after = if opening(UNDERLINE) { Beside::Mark } else { beside(next) };
            let tag = cannot_open(writer.before(), after);
            self.open(writer, STRIKE, tag);
        }
        if opening(UNDERLINE) {
            self.open(writer, UNDERLINE, true);
        }
    }
}

fn write_formatted(writer: &mut LineWriter, inlines: &[Inline]) {
    let mut formatting = Formatting::default();
    for inline in inlines {
        formatting.set(writer, wanted(&inline.marks), inline.text.chars().next());
        writer.text(&inline.text);
    }
    formatting.set(writer, [false; 4], None);
}

/// A link address is written as it is, or inside `<` `>` when it holds a
/// space or a parenthesis.
fn link_target(url: &str) -> String {
    if url.chars().any(|ch| matches!(ch, ' ' | '(' | ')' | '<' | '>')) {
        let escaped: String = url
            .chars()
            .flat_map(|ch| match ch {
                '\\' | '<' | '>' => vec!['\\', ch],
                _ => vec![ch],
            })
            .collect();
        format!("<{escaped}>")
    } else {
        url.replace('\\', "\\\\")
    }
}

/// What follows `==}`: the highlight color when one has to be said, then
/// the note.
fn review_suffix(marks: &Marks) -> String {
    let mut out = String::new();
    let note = marks.comment.as_deref();
    match marks.highlight {
        // A highlight with nothing after it is yellow. With a note after it,
        // the color has to be written, or the text would read as noted only.
        Some(Highlight::Yellow) if note.is_none() => {}
        Some(color) => out.push_str(&format!("{{.{}}}", color.as_str())),
        None => {}
    }
    if let Some(note) = note {
        let mut body = String::new();
        if marks.comment_resolved {
            body.push_str(RESOLVED_PREFIX);
        }
        let reads_as_prefix = note.starts_with(RESOLVED_PREFIX)
            || NoteKind::ALL
                .into_iter()
                .any(|kind| note.starts_with(kind_prefix(kind)));
        if marks.comment_kind != NoteKind::Comment || reads_as_prefix {
            body.push_str(kind_prefix(marks.comment_kind));
        }
        for ch in note.chars() {
            match ch {
                '\\' | '<' | '>' | '{' | '}' => {
                    body.push('\\');
                    body.push(ch);
                }
                '\n' => body.push_str("\\n"),
                _ => body.push(ch),
            }
        }
        out.push_str(&format!("{{>>{body}<<}}"));
    }
    out
}
