//! Markdown to document blocks.
//!
//! The reader accepts everything the writer produces, and reads it back
//! exactly. It is also forgiving about what a person might type into the
//! file by hand: anything it does not recognize stays in the text as it was
//! written.

use writing_studio_core::Id;
use writing_studio_document::{
    normalize_link, Block, BlockKind, Highlight, Inline, ListItem, Marks, NoteKind,
};

use crate::write::{kind_prefix, RESOLVED_PREFIX};

/// Read Markdown into blocks, in canonical form. There is always at least
/// one block.
pub fn markdown_to_blocks(text: &str) -> Vec<Block> {
    let lines: Vec<&str> = text
        .split('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line))
        .collect();
    let mut blocks: Vec<Block> = Vec::new();
    let mut at = 0;
    while at < lines.len() {
        let line = lines[at];
        let trimmed = line.trim();
        if trimmed.is_empty() {
            at += 1;
            continue;
        }
        if let Some(fence) = fence_length(trimmed) {
            let mut body: Vec<&str> = Vec::new();
            at += 1;
            while at < lines.len() && !closes_fence(lines[at], fence) {
                body.push(lines[at]);
                at += 1;
            }
            at += 1;
            let code = body.join("\n");
            blocks.push(block(BlockKind::Code {
                inlines: plain_inlines(&code),
            }));
            continue;
        }
        if trimmed == "<br>" {
            blocks.push(block(BlockKind::Paragraph { inlines: Vec::new() }));
            at += 1;
            continue;
        }
        if is_scene_break(trimmed) {
            blocks.push(block(BlockKind::SceneBreak));
            at += 1;
            continue;
        }
        if let Some((level, rest)) = heading(trimmed) {
            blocks.push(block(BlockKind::Heading {
                level,
                inlines: parse_inlines(rest),
            }));
            at += 1;
            continue;
        }
        if trimmed.starts_with('>') {
            let mut body: Vec<&str> = Vec::new();
            while at < lines.len() && lines[at].trim_start().starts_with('>') {
                let rest = &lines[at].trim_start()[1..];
                body.push(rest.strip_prefix(' ').unwrap_or(rest).trim());
                at += 1;
            }
            blocks.push(block(BlockKind::Quote {
                inlines: parse_inlines(&body.join("\n")),
            }));
            continue;
        }
        if let Some((marker, _)) = list_item(trimmed) {
            let mut items: Vec<String> = Vec::new();
            while at < lines.len() {
                let here = lines[at];
                match list_item(here.trim()) {
                    Some((found, rest)) if found == marker => items.push(rest.to_string()),
                    // An indented line under an item carries that item on.
                    None if here.starts_with("  ") && !here.trim().is_empty() => {
                        if let Some(last) = items.last_mut() {
                            last.push('\n');
                            last.push_str(here.trim());
                        }
                    }
                    _ => break,
                }
                at += 1;
            }
            blocks.push(block(BlockKind::List {
                ordered: matches!(marker, Marker::Dot | Marker::Paren),
                items: items
                    .iter()
                    .map(|item| ListItem {
                        id: Id::new(),
                        inlines: parse_inlines(item),
                    })
                    .collect(),
            }));
            continue;
        }
        let mut body: Vec<&str> = vec![trimmed];
        at += 1;
        while at < lines.len() && !starts_block(lines[at].trim()) {
            body.push(lines[at].trim());
            at += 1;
        }
        blocks.push(block(BlockKind::Paragraph {
            inlines: parse_inlines(&body.join("\n")),
        }));
    }
    if blocks.is_empty() {
        blocks.push(block(BlockKind::Paragraph { inlines: Vec::new() }));
    }
    blocks
}

fn block(kind: BlockKind) -> Block {
    Block { id: Id::new(), kind }
}

fn plain_inlines(text: &str) -> Vec<Inline> {
    if text.is_empty() {
        Vec::new()
    } else {
        vec![Inline::text(text)]
    }
}

/// A line that ends the paragraph before it.
fn starts_block(trimmed: &str) -> bool {
    trimmed.is_empty()
        || trimmed == "<br>"
        || trimmed.starts_with('>')
        || fence_length(trimmed).is_some()
        || is_scene_break(trimmed)
        || heading(trimmed).is_some()
        || list_item(trimmed).is_some()
}

fn fence_length(trimmed: &str) -> Option<usize> {
    let ticks = trimmed.chars().take_while(|ch| *ch == '`').count();
    (ticks >= 3 && !trimmed[ticks..].contains('`')).then_some(ticks)
}

fn closes_fence(line: &str, open: usize) -> bool {
    // Only at the very start of the line, so that an indented line of
    // backticks inside the code stays code.
    let trimmed = line.trim_end();
    trimmed.len() >= open && trimmed.chars().all(|ch| ch == '`')
}

/// Three or more `*`, `-`, or `_`, with or without spaces between them.
fn is_scene_break(trimmed: &str) -> bool {
    let marks: Vec<char> = trimmed.chars().filter(|ch| !ch.is_whitespace()).collect();
    marks.len() >= 3
        && matches!(marks[0], '*' | '-' | '_')
        && marks.iter().all(|ch| *ch == marks[0])
}

fn heading(trimmed: &str) -> Option<(u8, &str)> {
    let level = trimmed.chars().take_while(|ch| *ch == '#').count();
    if !(1..=6).contains(&level) {
        return None;
    }
    let rest = &trimmed[level..];
    if rest.is_empty() {
        return Some((level as u8, ""));
    }
    rest.starts_with([' ', '\t']).then(|| (level as u8, rest.trim()))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Marker {
    Dash,
    Plus,
    Star,
    Dot,
    Paren,
}

fn list_item(trimmed: &str) -> Option<(Marker, &str)> {
    fn after(marker: Marker, rest: &str) -> Option<(Marker, &str)> {
        if rest.is_empty() {
            Some((marker, ""))
        } else {
            rest.starts_with([' ', '\t']).then(|| (marker, rest.trim()))
        }
    }
    match trimmed.chars().next()? {
        '-' => after(Marker::Dash, &trimmed[1..]),
        '+' => after(Marker::Plus, &trimmed[1..]),
        '*' => after(Marker::Star, &trimmed[1..]),
        ch if ch.is_ascii_digit() => {
            let digits = trimmed.chars().take_while(char::is_ascii_digit).count();
            if digits > 9 {
                return None;
            }
            match trimmed[digits..].chars().next()? {
                '.' => after(Marker::Dot, &trimmed[digits + 1..]),
                ')' => after(Marker::Paren, &trimmed[digits + 1..]),
                _ => None,
            }
        }
        _ => None,
    }
}

struct InlineReader {
    chars: Vec<char>,
    at: usize,
    out: Vec<(String, Marks)>,
    text: String,
    marks: Marks,
    /// Bold and italic in the order they were opened, innermost last.
    stars: Vec<char>,
    /// Where the text inside an open `[` starts in `out`.
    link_from: Option<usize>,
    /// Where the text inside an open `{==` starts in `out`.
    review_from: Option<usize>,
}

/// Read the text of one block. A line break in `text` that does not follow
/// a backslash is a wrapped line and reads as a space.
pub(crate) fn parse_inlines(text: &str) -> Vec<Inline> {
    let mut reader = InlineReader {
        chars: text.chars().collect(),
        at: 0,
        out: Vec::new(),
        text: String::new(),
        marks: Marks::default(),
        stars: Vec::new(),
        link_from: None,
        review_from: None,
    };
    reader.run();
    let mut inlines: Vec<Inline> = Vec::new();
    for (text, marks) in reader.out {
        if text.is_empty() {
            continue;
        }
        match inlines.last_mut() {
            Some(last) if last.marks == marks => last.text.push_str(&text),
            _ => inlines.push(Inline {
                id: Id::new(),
                text,
                marks,
            }),
        }
    }
    inlines
}

impl InlineReader {
    fn flush(&mut self) {
        if !self.text.is_empty() {
            self.out
                .push((std::mem::take(&mut self.text), self.marks.clone()));
        }
    }

    fn rest_starts_with(&self, pattern: &str) -> bool {
        let pattern: Vec<char> = pattern.chars().collect();
        self.chars[self.at..].starts_with(&pattern)
    }

    /// The position of `pattern` after `from`, skipping escaped characters.
    fn find(&self, from: usize, pattern: &str) -> Option<usize> {
        let pattern: Vec<char> = pattern.chars().collect();
        let mut at = from;
        while at < self.chars.len() {
            if self.chars[at] == '\\' {
                at += 2;
                continue;
            }
            if self.chars[at..].starts_with(&pattern) {
                return Some(at);
            }
            at += 1;
        }
        None
    }

    fn run(&mut self) {
        while self.at < self.chars.len() {
            let ch = self.chars[self.at];
            match ch {
                '\\' => self.escape(),
                '\n' => {
                    self.text.push(' ');
                    self.at += 1;
                }
                '*' => self.stars(),
                '~' if self.rest_starts_with("~~") => {
                    self.flush();
                    self.marks.strikethrough = !self.marks.strikethrough;
                    self.at += 2;
                }
                '<' => self.tag(),
                '&' => self.reference(),
                '[' if self.link_from.is_none() && self.link_close(self.at + 1).is_some() => {
                    self.flush();
                    self.link_from = Some(self.out.len());
                    self.at += 1;
                }
                ']' if self.link_from.is_some() && self.chars.get(self.at + 1) == Some(&'(') => {
                    self.close_link();
                }
                '{' if self.review_from.is_none()
                    && self.rest_starts_with("{==")
                    && self.find(self.at + 3, "==}").is_some() =>
                {
                    self.flush();
                    self.review_from = Some(self.out.len());
                    self.at += 3;
                }
                '=' if self.review_from.is_some() && self.rest_starts_with("==}") => {
                    self.close_review();
                }
                _ => {
                    self.text.push(ch);
                    self.at += 1;
                }
            }
        }
        self.flush();
    }

    fn escape(&mut self) {
        match self.chars.get(self.at + 1).copied() {
            // A backslash at the end of a line is a line break.
            None | Some('\n') => {
                self.text.push('\n');
                self.at += 2;
            }
            Some(next) if next.is_ascii_punctuation() => {
                self.text.push(next);
                self.at += 2;
            }
            Some(_) => {
                self.text.push('\\');
                self.at += 1;
            }
        }
    }

    /// A run of `*`. One is italic and two are bold. The writer puts bold
    /// outside italic, so a run closes what is open, innermost first, and
    /// then opens what is left over.
    fn stars(&mut self) {
        let mut count = self.chars[self.at..]
            .iter()
            .take_while(|ch| **ch == '*')
            .count();
        self.at += count;
        self.flush();
        while count > 0 {
            match self.stars.last().copied() {
                // `*a **b** c*`: bold inside italic, as a person might write it.
                Some('i') if count == 2 => {
                    self.toggle_bold();
                    count = 0;
                }
                Some('i') => {
                    self.marks.italic = false;
                    self.stars.pop();
                    count -= 1;
                }
                Some(_) if count >= 2 => {
                    self.marks.bold = false;
                    self.stars.pop();
                    count -= 2;
                }
                _ if count >= 2 && !self.marks.bold => {
                    self.toggle_bold();
                    count -= 2;
                }
                _ => {
                    self.marks.italic = true;
                    self.stars.push('i');
                    count -= 1;
                }
            }
        }
    }

    fn toggle_bold(&mut self) {
        if self.marks.bold {
            self.marks.bold = false;
            self.stars.retain(|kind| *kind != 'b');
        } else {
            self.marks.bold = true;
            self.stars.push('b');
        }
    }

    /// The few tags that mean formatting. The writer uses the ones for bold,
    /// italic, and strikethrough where `**`, `*`, or `~~` would not be read
    /// the same way by every Markdown reader.
    fn tag(&mut self) {
        type Change = fn(&mut Marks, bool);
        let formats: [(&str, Change); 7] = [
            ("u", |marks, on| marks.underline = on),
            ("strong", |marks, on| marks.bold = on),
            ("b", |marks, on| marks.bold = on),
            ("em", |marks, on| marks.italic = on),
            ("i", |marks, on| marks.italic = on),
            ("s", |marks, on| marks.strikethrough = on),
            ("del", |marks, on| marks.strikethrough = on),
        ];
        for (name, change) in formats {
            for (tag, on) in [(format!("<{name}>"), true), (format!("</{name}>"), false)] {
                if self.rest_starts_with(&tag) {
                    self.flush();
                    change(&mut self.marks, on);
                    if !on {
                        // A tag can close what stars opened, in a file
                        // written by hand.
                        let (bold, italic) = (self.marks.bold, self.marks.italic);
                        self.stars
                            .retain(|kind| (*kind == 'b' && bold) || (*kind == 'i' && italic));
                    }
                    self.at += tag.chars().count();
                    return;
                }
            }
        }
        for tag in ["<br>", "<br/>", "<br />"] {
            if self.rest_starts_with(tag) {
                self.text.push('\n');
                self.at += tag.chars().count();
                return;
            }
        }
        self.text.push('<');
        self.at += 1;
    }

    /// `&#32;`, `&#x20;`, and the few named ones a person is likely to type.
    fn reference(&mut self) {
        let end = self.chars[self.at..]
            .iter()
            .take(10)
            .position(|ch| *ch == ';');
        if let Some(end) = end {
            let name: String = self.chars[self.at + 1..self.at + end].iter().collect();
            let found = match name.as_str() {
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "quot" => Some('"'),
                "apos" => Some('\''),
                "nbsp" => Some('\u{a0}'),
                _ => name
                    .strip_prefix("#x")
                    .or_else(|| name.strip_prefix("#X"))
                    .map(|hex| u32::from_str_radix(hex, 16))
                    .or_else(|| name.strip_prefix('#').map(str::parse::<u32>))
                    .and_then(Result::ok)
                    .and_then(char::from_u32),
            };
            if let Some(found) = found {
                self.text.push(found);
                self.at += end + 1;
                return;
            }
        }
        self.text.push('&');
        self.at += 1;
    }

    /// The `]` that ends a link opened before `from`, when it is followed
    /// by an address in parentheses.
    fn link_close(&self, from: usize) -> Option<usize> {
        let close = self.find(from, "](")?;
        self.link_address(close + 2).map(|_| close)
    }

    /// The address that starts at `from`, and where the text goes on after
    /// its closing parenthesis.
    fn link_address(&self, from: usize) -> Option<(String, usize)> {
        let angled = self.chars.get(from) == Some(&'<');
        let mut at = if angled { from + 1 } else { from };
        let mut address = String::new();
        while at < self.chars.len() {
            let ch = self.chars[at];
            match ch {
                '\\' if at + 1 < self.chars.len() => {
                    address.push(self.chars[at + 1]);
                    at += 2;
                    continue;
                }
                '>' if angled => {
                    return (self.chars.get(at + 1) == Some(&')')).then_some((address, at + 2));
                }
                ')' if !angled => return Some((address, at + 1)),
                '\n' => return None,
                _ => address.push(ch),
            }
            at += 1;
        }
        None
    }

    fn close_link(&mut self) {
        let Some((address, after)) = self.link_address(self.at + 2) else {
            self.text.push(']');
            self.at += 1;
            return;
        };
        self.flush();
        let from = self.link_from.take().unwrap_or(self.out.len());
        let link = normalize_link(&address);
        for (_, marks) in &mut self.out[from..] {
            marks.link = link.clone();
        }
        self.at = after;
    }

    fn close_review(&mut self) {
        self.flush();
        self.at += 3;
        let from = self.review_from.take().unwrap_or(self.out.len());

        let mut highlight = None;
        if self.rest_starts_with("{.") {
            if let Some(end) = self.find(self.at, "}") {
                let name: String = self.chars[self.at + 2..end].iter().collect();
                if let Some(color) = Highlight::from_id(&name) {
                    highlight = Some(color);
                    self.at = end + 1;
                }
            }
        }

        let mut note: Option<(String, NoteKind, bool)> = None;
        if self.rest_starts_with("{>>") {
            if let Some(end) = self.find(self.at + 3, "<<}") {
                note = read_note(&self.chars[self.at + 3..end]);
                self.at = end + 3;
            }
        }
        // `{==text==}` with nothing after it is a yellow highlight.
        if highlight.is_none() && note.is_none() {
            highlight = Some(Highlight::Yellow);
        }
        for (_, marks) in &mut self.out[from..] {
            marks.highlight = highlight;
            if let Some((text, kind, resolved)) = &note {
                marks.comment = Some(text.clone());
                marks.comment_kind = *kind;
                marks.comment_resolved = *resolved;
            }
        }
    }
}

/// The body of `{>>…<<}`: an optional `(resolved) `, an optional kind, then
/// the note. A blank note is no note.
fn read_note(body: &[char]) -> Option<(String, NoteKind, bool)> {
    let mut text = String::new();
    let mut at = 0;
    while at < body.len() {
        match (body[at], body.get(at + 1)) {
            ('\\', Some('n')) => {
                text.push('\n');
                at += 2;
            }
            ('\\', Some(next)) => {
                text.push(*next);
                at += 2;
            }
            (ch, _) => {
                text.push(ch);
                at += 1;
            }
        }
    }
    let mut rest = text.as_str();
    let resolved = rest.starts_with(RESOLVED_PREFIX);
    if resolved {
        rest = &rest[RESOLVED_PREFIX.len()..];
    }
    let mut kind = NoteKind::Comment;
    for candidate in NoteKind::ALL {
        if let Some(after) = rest.strip_prefix(kind_prefix(candidate)) {
            kind = candidate;
            rest = after;
            break;
        }
    }
    let rest = rest.trim();
    (!rest.is_empty()).then(|| (rest.to_string(), kind, resolved))
}
