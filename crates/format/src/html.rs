//! Document blocks to and from the HTML a scene file holds.
//!
//! HTML can say everything the document model can, so a scene is stored
//! exactly: nothing is dropped and nothing has to be escaped beyond `&`,
//! `<`, and `>`. Every block is on its own line, so that a change to one
//! paragraph is a change to one line of the file.
//!
//! This is not the HTML the editor exchanges with the document
//! (`writing_studio_document`'s `to_html`). That one follows what a browser
//! does to an editable page and loses a few things on the way, such as an
//! empty paragraph. This one is a storage format with one spelling for each
//! thing. The dialect is described in `documentation/PROJECT_FORMAT.md`.

use writing_studio_core::Id;
use writing_studio_document::{
    normalize_link, Block, BlockKind, Highlight, Inline, ListItem, Marks, NoteKind,
};

/// The blocks as a scene file stores them. This changes nothing a reader
/// could see: runs with the same formatting are joined, empty runs go, a
/// link that is not a safe address is taken off its text, a blank note is no
/// note, and carriage returns are removed.
pub fn stored_blocks(blocks: &[Block]) -> Vec<Block> {
    let mut out: Vec<Block> = blocks
        .iter()
        .map(|block| Block {
            id: block.id,
            kind: match &block.kind {
                BlockKind::Paragraph { inlines } => BlockKind::Paragraph {
                    inlines: stored_inlines(inlines),
                },
                BlockKind::Heading { level, inlines } => BlockKind::Heading {
                    level: (*level).clamp(1, 6),
                    inlines: stored_inlines(inlines),
                },
                BlockKind::Quote { inlines } => BlockKind::Quote {
                    inlines: stored_inlines(inlines),
                },
                BlockKind::Code { inlines } => BlockKind::Code {
                    inlines: stored_inlines(inlines),
                },
                BlockKind::List { ordered, items } => BlockKind::List {
                    ordered: *ordered,
                    items: items
                        .iter()
                        .map(|item| ListItem {
                            id: item.id,
                            inlines: stored_inlines(&item.inlines),
                        })
                        .collect(),
                },
                BlockKind::SceneBreak => BlockKind::SceneBreak,
            },
        })
        .collect();
    if out.is_empty() {
        out.push(Block {
            id: Id::new(),
            kind: BlockKind::Paragraph { inlines: Vec::new() },
        });
    }
    out
}

fn stored_inlines(inlines: &[Inline]) -> Vec<Inline> {
    let mut out: Vec<Inline> = Vec::new();
    for inline in inlines {
        let text = inline.text.replace('\r', "");
        if text.is_empty() {
            continue;
        }
        let mut marks = inline.marks.clone();
        marks.link = inline.marks.link.as_deref().and_then(normalize_link);
        match inline.marks.note() {
            Some(note) => marks.comment = Some(note.replace('\r', "")),
            None => {
                marks.comment = None;
                marks.comment_kind = NoteKind::default();
                marks.comment_resolved = false;
            }
        }
        match out.last_mut() {
            Some(last) if last.marks == marks => last.text.push_str(&text),
            _ => out.push(Inline {
                id: inline.id,
                text,
                marks,
            }),
        }
    }
    out
}

/// Write blocks as HTML, one block to a line. A list and a code block take
/// several lines.
pub fn blocks_to_html(blocks: &[Block]) -> String {
    let mut out = String::new();
    for block in stored_blocks(blocks) {
        match &block.kind {
            BlockKind::Paragraph { inlines } => element(&mut out, "p", inlines),
            BlockKind::Heading { level, inlines } => element(&mut out, &format!("h{level}"), inlines),
            BlockKind::Quote { inlines } => element(&mut out, "blockquote", inlines),
            BlockKind::Code { inlines } => {
                // A reader of HTML drops one line break straight after
                // `<pre>`, so one is always written there.
                out.push_str("<pre>\n");
                write_inlines(&mut out, inlines, true);
                out.push_str("</pre>\n");
            }
            BlockKind::List { ordered, items } => {
                let tag = if *ordered { "ol" } else { "ul" };
                out.push_str(&format!("<{tag}>\n"));
                for item in items {
                    element(&mut out, "li", &item.inlines);
                }
                out.push_str(&format!("</{tag}>\n"));
            }
            BlockKind::SceneBreak => out.push_str("<hr>\n"),
        }
    }
    out
}

fn element(out: &mut String, tag: &str, inlines: &[Inline]) {
    out.push_str(&format!("<{tag}>"));
    write_inlines(out, inlines, false);
    out.push_str(&format!("</{tag}>\n"));
}

/// The formatting of a run, outermost first. A note is outermost so that a
/// note over text that is partly bold or partly highlighted is written once.
#[derive(Clone, PartialEq, Eq, Default)]
struct Layers {
    note: Option<(String, NoteKind, bool)>,
    highlight: Option<Highlight>,
    link: Option<String>,
    simple: [bool; 4],
}

const LAYERS: usize = 7;
const SIMPLE_TAGS: [&str; 4] = ["strong", "em", "s", "u"];

impl Layers {
    fn of(marks: &Marks) -> Self {
        Self {
            note: marks
                .comment
                .clone()
                .map(|note| (note, marks.comment_kind, marks.comment_resolved)),
            highlight: marks.highlight,
            link: marks.link.clone(),
            simple: [marks.bold, marks.italic, marks.strikethrough, marks.underline],
        }
    }

    fn same_at(&self, other: &Self, layer: usize) -> bool {
        match layer {
            0 => self.note == other.note,
            1 => self.highlight == other.highlight,
            2 => self.link == other.link,
            _ => self.simple[layer - 3] == other.simple[layer - 3],
        }
    }

    fn open_tag(&self, layer: usize) -> Option<String> {
        match layer {
            0 => self.note.as_ref().map(|(note, kind, resolved)| {
                let mut tag = format!("<span class=\"note\" data-note=\"{}\"", attribute(note));
                if *kind != NoteKind::Comment {
                    tag.push_str(&format!(" data-kind=\"{}\"", kind.as_str()));
                }
                if *resolved {
                    tag.push_str(" data-resolved=\"true\"");
                }
                tag.push('>');
                tag
            }),
            1 => self.highlight.map(|color| match color {
                Highlight::Yellow => "<mark>".to_string(),
                other => format!("<mark class=\"{}\">", other.as_str()),
            }),
            2 => self
                .link
                .as_ref()
                .map(|url| format!("<a href=\"{}\">", attribute(url))),
            _ => self.simple[layer - 3].then(|| format!("<{}>", SIMPLE_TAGS[layer - 3])),
        }
    }

    fn close_tag(&self, layer: usize) -> Option<&'static str> {
        match layer {
            0 => self.note.as_ref().map(|_| "</span>"),
            1 => self.highlight.map(|_| "</mark>"),
            2 => self.link.as_ref().map(|_| "</a>"),
            _ => self.simple[layer - 3].then(|| ["</strong>", "</em>", "</s>", "</u>"][layer - 3]),
        }
    }
}

fn write_inlines(out: &mut String, inlines: &[Inline], code: bool) {
    let mut open = Layers::default();
    let close_from = |out: &mut String, open: &Layers, from: usize| {
        for layer in (from..LAYERS).rev() {
            if let Some(tag) = open.close_tag(layer) {
                out.push_str(tag);
            }
        }
    };
    for inline in inlines {
        let want = Layers::of(&inline.marks);
        if let Some(first) = (0..LAYERS).find(|layer| !open.same_at(&want, *layer)) {
            close_from(out, &open, first);
            for layer in first..LAYERS {
                if let Some(tag) = want.open_tag(layer) {
                    out.push_str(&tag);
                }
            }
            open = want;
        }
        for ch in inline.text.chars() {
            match ch {
                '&' => out.push_str("&amp;"),
                '<' => out.push_str("&lt;"),
                '>' => out.push_str("&gt;"),
                '\u{a0}' => out.push_str("&nbsp;"),
                '\n' if !code => out.push_str("<br>"),
                _ => out.push(ch),
            }
        }
    }
    close_from(out, &open, 0);
}

/// Text inside a double-quoted attribute, on one line.
pub(crate) fn attribute(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\n' => out.push_str("&#10;"),
            '\t' => out.push_str("&#9;"),
            '\u{a0}' => out.push_str("&nbsp;"),
            _ => out.push(ch),
        }
    }
    out
}

/// One piece of a file: text between tags, or a tag.
pub(crate) enum Token {
    Text(String),
    Open(String, Vec<(String, String)>),
    Close(String),
}

/// Cut HTML into text and tags. Comments, the doctype, and everything
/// inside `<script>` and `<style>` are left out: a scene file is text, and
/// nothing in one is ever run.
pub(crate) fn tokens(html: &str) -> Vec<Token> {
    let mut out = Vec::new();
    let mut rest = html;
    while !rest.is_empty() {
        let Some(start) = rest.find('<') else {
            out.push(Token::Text(rest.to_string()));
            break;
        };
        if start > 0 {
            out.push(Token::Text(rest[..start].to_string()));
        }
        rest = &rest[start..];
        if let Some(after) = rest.strip_prefix("<!--") {
            rest = after.find("-->").map_or("", |end| &after[end + 3..]);
            continue;
        }
        let looks_like_tag = rest[1..]
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_alphabetic() || ch == '/' || ch == '!' || ch == '?');
        let Some(end) = rest.find('>').filter(|_| looks_like_tag) else {
            // A `<` that starts no tag is a character someone typed.
            out.push(Token::Text("<".to_string()));
            rest = &rest[1..];
            continue;
        };
        let inner = rest[1..end].trim();
        rest = &rest[end + 1..];
        if inner.starts_with('!') || inner.starts_with('?') {
            continue;
        }
        if let Some(name) = inner.strip_prefix('/') {
            out.push(Token::Close(name.trim().to_ascii_lowercase()));
            continue;
        }
        let inner = inner.trim_end_matches('/').trim_end();
        let name_end = inner.find(char::is_whitespace).unwrap_or(inner.len());
        let name = inner[..name_end].to_ascii_lowercase();
        if name == "script" || name == "style" {
            let closing = format!("</{name}");
            let lower = rest.to_ascii_lowercase();
            rest = match lower.find(&closing) {
                Some(at) => rest[at..].find('>').map_or("", |end| &rest[at + end + 1..]),
                None => "",
            };
            continue;
        }
        out.push(Token::Open(name, attributes(&inner[name_end..])));
    }
    out
}

fn attributes(mut rest: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    loop {
        rest = rest.trim_start();
        if rest.is_empty() {
            return out;
        }
        let name_end = rest
            .find(|ch: char| ch == '=' || ch.is_whitespace())
            .unwrap_or(rest.len());
        let name = rest[..name_end].to_ascii_lowercase();
        rest = rest[name_end..].trim_start();
        let Some(after) = rest.strip_prefix('=') else {
            // An attribute with no value, such as `data-resolved`, is on.
            out.push((name, "true".to_string()));
            continue;
        };
        let after = after.trim_start();
        let (value, next) = match after.chars().next() {
            Some(quote @ ('"' | '\'')) => {
                let body = &after[1..];
                let end = body.find(quote).unwrap_or(body.len());
                (&body[..end], body.get(end + 1..).unwrap_or(""))
            }
            _ => {
                let end = after.find(char::is_whitespace).unwrap_or(after.len());
                (&after[..end], &after[end..])
            }
        };
        out.push((name, decode(value)));
        rest = next;
    }
}

pub(crate) fn find<'a>(attributes: &'a [(String, String)], name: &str) -> Option<&'a str> {
    attributes
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.as_str())
}

/// Turn `&amp;`, `&#10;`, and the like back into characters. Anything that
/// is not one of them stays as it was typed.
pub(crate) fn decode(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find('&') {
        out.push_str(&rest[..start]);
        rest = &rest[start..];
        let found = rest[1..]
            .char_indices()
            .take(10)
            .find(|(_, ch)| *ch == ';')
            .and_then(|(end, _)| {
                let name = &rest[1..1 + end];
                let ch = match name {
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
                ch.map(|ch| (ch, end + 2))
            });
        match found {
            Some((ch, length)) => {
                out.push(ch);
                rest = &rest[length..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// A piece of a block while it is being read.
enum Piece {
    Text(String, Marks),
    /// Line breaks and indentation in the file between tags or words. The
    /// writer never puts one inside a block; a person tidying the file by
    /// hand does. It reads as one space, or as nothing at the edge of a block.
    Wrap(Marks),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Open {
    Paragraph,
    Heading(u8),
    Quote,
    Code,
    Item,
}

struct Reader {
    blocks: Vec<Block>,
    open: Option<Open>,
    pieces: Vec<Piece>,
    /// The list being read, and whether it is numbered.
    list: Option<(bool, Vec<ListItem>)>,
    marks: Marks,
    /// What each open formatting tag changed, so its end can put it back.
    undo: Vec<(String, Marks)>,
    /// The next text is the first inside `<pre>`.
    code_start: bool,
}

/// Read HTML into blocks. The reader takes what the writer produces back
/// exactly. It also reads a file someone tidied or edited by hand: tags it
/// does not know are passed over and their text is kept.
pub fn html_to_blocks(html: &str) -> Vec<Block> {
    let mut reader = Reader {
        blocks: Vec::new(),
        open: None,
        pieces: Vec::new(),
        list: None,
        marks: Marks::default(),
        undo: Vec::new(),
        code_start: false,
    };
    for token in tokens(&html.replace('\r', "")) {
        match token {
            Token::Text(text) => reader.text(&text),
            Token::Open(name, attributes) => reader.start(&name, &attributes),
            Token::Close(name) => reader.end(&name),
        }
    }
    reader.close_block(false);
    reader.close_list();
    if reader.blocks.is_empty() {
        reader.blocks.push(Block {
            id: Id::new(),
            kind: BlockKind::Paragraph { inlines: Vec::new() },
        });
    }
    reader.blocks
}

impl Reader {
    fn text(&mut self, raw: &str) {
        let mut text = decode(raw);
        if self.open == Some(Open::Code) {
            if std::mem::take(&mut self.code_start) {
                if let Some(rest) = text.strip_prefix('\n') {
                    text = rest.to_string();
                }
            }
            if !text.is_empty() {
                self.pieces.push(Piece::Text(text, self.marks.clone()));
            }
            return;
        }
        // Whitespace that holds a line break is how the file is laid out,
        // not something in the text.
        let mut rest = text.as_str();
        while !rest.is_empty() {
            let Some(line_break) = rest.find('\n') else {
                self.pieces.push(Piece::Text(rest.to_string(), self.marks.clone()));
                break;
            };
            let before = rest[..line_break].trim_end_matches([' ', '\t']);
            if !before.is_empty() {
                self.pieces.push(Piece::Text(before.to_string(), self.marks.clone()));
            }
            self.pieces.push(Piece::Wrap(self.marks.clone()));
            rest = rest[line_break..].trim_start_matches([' ', '\t', '\n']);
        }
    }

    fn start(&mut self, name: &str, attributes: &[(String, String)]) {
        match name {
            "p" | "div" => self.open_block(Open::Paragraph),
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                self.open_block(Open::Heading(name.as_bytes()[1] - b'0'));
            }
            "blockquote" => self.open_block(Open::Quote),
            "pre" => {
                self.open_block(Open::Code);
                self.code_start = true;
            }
            "ul" | "ol" => {
                self.close_block(false);
                self.close_list();
                self.list = Some((name == "ol", Vec::new()));
            }
            "li" => {
                self.close_block(false);
                if self.list.is_none() {
                    self.list = Some((false, Vec::new()));
                }
                self.open = Some(Open::Item);
            }
            "hr" => {
                self.close_block(false);
                self.close_list();
                self.blocks.push(Block {
                    id: Id::new(),
                    kind: BlockKind::SceneBreak,
                });
            }
            "br" => self.pieces.push(Piece::Text("\n".into(), self.marks.clone())),
            _ => self.format(name, attributes),
        }
    }

    fn format(&mut self, name: &str, attributes: &[(String, String)]) {
        let before = self.marks.clone();
        match name {
            "strong" | "b" => self.marks.bold = true,
            "em" | "i" => self.marks.italic = true,
            "s" | "del" | "strike" => self.marks.strikethrough = true,
            "u" => self.marks.underline = true,
            "a" => self.marks.link = find(attributes, "href").and_then(normalize_link),
            "mark" => {
                self.marks.highlight = Some(
                    find(attributes, "class")
                        .and_then(|class| class.split_whitespace().find_map(Highlight::from_id))
                        .unwrap_or(Highlight::Yellow),
                );
            }
            "span" => {
                let note = find(attributes, "data-note").map(str::trim);
                if let Some(note) = note.filter(|note| !note.is_empty()) {
                    self.marks.comment = Some(note.to_string());
                    self.marks.comment_kind = find(attributes, "data-kind")
                        .and_then(NoteKind::from_id)
                        .unwrap_or_default();
                    self.marks.comment_resolved = find(attributes, "data-resolved") == Some("true");
                }
            }
            // A tag this program does not know. Its text is kept.
            _ => return,
        }
        self.undo.push((name.to_string(), before));
    }

    fn end(&mut self, name: &str) {
        match name {
            "p" | "div" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "blockquote" | "pre" | "li" => {
                self.close_block(true);
            }
            "ul" | "ol" => {
                self.close_block(false);
                self.close_list();
            }
            _ => {
                if let Some(at) = self.undo.iter().rposition(|(open, _)| open == name) {
                    self.marks = self.undo[at].1.clone();
                    self.undo.truncate(at);
                }
            }
        }
    }

    fn open_block(&mut self, kind: Open) {
        self.close_block(false);
        self.close_list();
        self.open = Some(kind);
    }

    /// End the block being read. Text outside any block becomes a paragraph.
    /// A block ended by its own end tag is kept even when it is empty, which
    /// is how an empty paragraph is stored. One that ends because another
    /// block starts inside it, as in `<blockquote><p>`, is kept only if it
    /// holds text.
    fn close_block(&mut self, by_its_end_tag: bool) {
        let pieces = std::mem::take(&mut self.pieces);
        let inlines = join(pieces, self.open == Some(Open::Code));
        if inlines.is_empty() && !by_its_end_tag {
            self.open = None;
            return;
        }
        let kind = match self.open.take() {
            None if inlines.is_empty() => return,
            None | Some(Open::Paragraph) => BlockKind::Paragraph { inlines },
            Some(Open::Heading(level)) => BlockKind::Heading { level, inlines },
            Some(Open::Quote) => BlockKind::Quote { inlines },
            Some(Open::Code) => BlockKind::Code { inlines },
            Some(Open::Item) => {
                if let Some((_, items)) = &mut self.list {
                    items.push(ListItem {
                        id: Id::new(),
                        inlines,
                    });
                }
                return;
            }
        };
        self.close_list();
        self.blocks.push(Block { id: Id::new(), kind });
    }

    fn close_list(&mut self) {
        if let Some((ordered, items)) = self.list.take() {
            self.blocks.push(Block {
                id: Id::new(),
                kind: BlockKind::List { ordered, items },
            });
        }
    }
}

fn join(mut pieces: Vec<Piece>, code: bool) -> Vec<Inline> {
    if !code {
        while matches!(pieces.last(), Some(Piece::Wrap(_))) {
            pieces.pop();
        }
        let leading = pieces
            .iter()
            .take_while(|piece| matches!(piece, Piece::Wrap(_)))
            .count();
        pieces.drain(..leading);
    }
    let mut out: Vec<Inline> = Vec::new();
    for piece in pieces {
        let (text, marks) = match piece {
            Piece::Text(text, marks) => (text, marks),
            Piece::Wrap(marks) => (" ".to_string(), marks),
        };
        match out.last_mut() {
            Some(last) if last.marks == marks => last.text.push_str(&text),
            _ => out.push(Inline {
                id: Id::new(),
                text,
                marks,
            }),
        }
    }
    out
}
