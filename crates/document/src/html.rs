//! Restricted HTML used by the desktop editor.
//!
//! This is an interchange format for the structured document model, not a
//! general HTML engine. Unknown tags are ignored; script never round-trips.

use writing_studio_core::Id;

use crate::model::{normalize_link, Block, BlockKind, Document, Inline, ListItem, Marks};

impl Document {
    pub fn to_html(&self) -> String {
        self.blocks.iter().map(block_to_html).collect()
    }

    pub fn replace_html(&mut self, html: &str) {
        let parsed = parse_html(html);
        self.blocks = reuse_ids(&self.blocks, parsed);
        self.updated_at = writing_studio_core::Timestamp::now();
    }
}

fn block_to_html(block: &Block) -> String {
    match &block.kind {
        BlockKind::Paragraph { inlines } => format!("<p>{}</p>", inlines_to_html(inlines)),
        BlockKind::Heading { level, inlines } => {
            let level = (*level).clamp(1, 6);
            format!("<h{level}>{}</h{level}>", inlines_to_html(inlines))
        }
        BlockKind::Quote { inlines } => {
            format!("<blockquote>{}</blockquote>", inlines_to_html(inlines))
        }
        BlockKind::List { ordered, items } => {
            let tag = if *ordered { "ol" } else { "ul" };
            let body: String = items
                .iter()
                .map(|item| format!("<li>{}</li>", inlines_to_html(&item.inlines)))
                .collect();
            format!("<{tag}>{body}</{tag}>")
        }
        BlockKind::Code { inlines } => format!("<pre>{}</pre>", inlines_to_html(inlines)),
        BlockKind::SceneBreak => "<hr>".into(),
    }
}

fn inlines_to_html(inlines: &[Inline]) -> String {
    if inlines.is_empty() {
        return String::new();
    }
    inlines.iter().map(inline_to_html).collect()
}

fn inline_to_html(inline: &Inline) -> String {
    let mut body = escape_text(&inline.text);
    if body.is_empty() {
        return body;
    }
    if inline.marks.strikethrough {
        body = format!("<s>{body}</s>");
    }
    if inline.marks.underline {
        body = format!("<u>{body}</u>");
    }
    if inline.marks.italic {
        body = format!("<em>{body}</em>");
    }
    if inline.marks.bold {
        body = format!("<strong>{body}</strong>");
    }
    if let Some(url) = inline.marks.link.as_deref().and_then(normalize_link) {
        body = format!("<a href=\"{}\">{body}</a>", escape_attr(&url));
    }
    if let Some(note) = inline
        .marks
        .comment
        .as_deref()
        .map(str::trim)
        .filter(|note| !note.is_empty())
    {
        let attr = escape_attr(note);
        body = format!(
            "<span class=\"ws-comment\" data-comment=\"{attr}\" title=\"{attr}\">{body}</span>"
        );
    }
    body
}

fn escape_text(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_attr(text: &str) -> String {
    escape_text(text).replace('"', "&quot;")
}

fn parse_html(html: &str) -> Vec<Block> {
    let tokens = tokenize(html);
    let mut blocks = Vec::new();
    let mut marks = Marks::default();
    let mut comment_stack: Vec<Option<String>> = Vec::new();
    let mut current_inlines: Vec<Inline> = Vec::new();
    let mut list_items: Vec<ListItem> = Vec::new();
    let mut list_ordered = false;
    let mut in_list = false;
    let mut block_tag: Option<OpenBlock> = None;

    for token in tokens {
        match token {
            Token::Text(text) => {
                let text = unescape(&text);
                if text.is_empty() {
                    continue;
                }
                push_text(&mut current_inlines, &text, &marks);
            }
            Token::Start(tag, attrs) => match tag.as_str() {
                "p" | "div" => {
                    if !in_list {
                        flush_named_block(&mut blocks, &mut current_inlines, block_tag);
                        block_tag = Some(OpenBlock::Paragraph);
                    }
                }
                "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                    flush_named_block(&mut blocks, &mut current_inlines, block_tag);
                    let level = tag.as_bytes()[1].saturating_sub(b'0');
                    block_tag = Some(OpenBlock::Heading(level.clamp(1, 6)));
                }
                "blockquote" => {
                    flush_named_block(&mut blocks, &mut current_inlines, block_tag);
                    block_tag = Some(OpenBlock::Quote);
                }
                "pre" => {
                    close_list(&mut blocks, &mut list_items, list_ordered, &mut in_list);
                    flush_named_block(&mut blocks, &mut current_inlines, block_tag);
                    block_tag = Some(OpenBlock::Code);
                }
                "ul" | "ol" => {
                    flush_named_block(&mut blocks, &mut current_inlines, block_tag);
                    in_list = true;
                    list_ordered = tag == "ol";
                    list_items.clear();
                    block_tag = None;
                }
                "li" => {
                    current_inlines.clear();
                }
                "hr" => {
                    close_list(&mut blocks, &mut list_items, list_ordered, &mut in_list);
                    flush_named_block(&mut blocks, &mut current_inlines, block_tag);
                    block_tag = None;
                    blocks.push(Block {
                        id: Id::new(),
                        kind: BlockKind::SceneBreak,
                    });
                }
                "br" => push_text(&mut current_inlines, "\n", &marks),
                "strong" | "b" => marks.bold = true,
                "em" | "i" => marks.italic = true,
                "u" => marks.underline = true,
                "s" | "strike" | "del" => marks.strikethrough = true,
                "a" => {
                    marks.link = attrs
                        .iter()
                        .find(|(name, _)| name == "href")
                        .and_then(|(_, value)| normalize_link(&unescape(value)));
                }
                "span" => {
                    comment_stack.push(marks.comment.clone());
                    if let Some(note) = attrs
                        .iter()
                        .find(|(name, _)| name == "data-comment")
                        .map(|(_, value)| unescape(value).trim().to_string())
                        .filter(|note| !note.is_empty())
                    {
                        marks.comment = Some(note);
                    }
                }
                _ => {}
            },
            Token::End(tag) => match tag.as_str() {
                "p" | "div" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "blockquote" | "pre" => {
                    if !(in_list && matches!(tag.as_str(), "p" | "div")) {
                        flush_named_block(&mut blocks, &mut current_inlines, block_tag);
                        block_tag = None;
                    }
                }
                "li" => {
                    list_items.push(ListItem {
                        id: Id::new(),
                        inlines: take_inlines(&mut current_inlines),
                    });
                }
                "ul" | "ol" => {
                    if !list_items.is_empty() {
                        blocks.push(Block {
                            id: Id::new(),
                            kind: BlockKind::List {
                                ordered: list_ordered,
                                items: std::mem::take(&mut list_items),
                            },
                        });
                    }
                    in_list = false;
                }
                "strong" | "b" => marks.bold = false,
                "em" | "i" => marks.italic = false,
                "u" => marks.underline = false,
                "s" | "strike" | "del" => marks.strikethrough = false,
                "a" => marks.link = None,
                "span" => marks.comment = comment_stack.pop().flatten(),
                _ => {}
            },
        }
    }
    flush_named_block(&mut blocks, &mut current_inlines, block_tag);
    if blocks.is_empty() {
        blocks.push(Block::paragraph(""));
    }
    blocks
}

#[derive(Clone, Copy)]
enum OpenBlock {
    Paragraph,
    Heading(u8),
    Quote,
    Code,
}

fn close_list(
    blocks: &mut Vec<Block>,
    list_items: &mut Vec<ListItem>,
    list_ordered: bool,
    in_list: &mut bool,
) {
    if !*in_list {
        return;
    }
    if !list_items.is_empty() {
        blocks.push(Block {
            id: Id::new(),
            kind: BlockKind::List {
                ordered: list_ordered,
                items: std::mem::take(list_items),
            },
        });
    }
    *in_list = false;
}

fn flush_named_block(blocks: &mut Vec<Block>, inlines: &mut Vec<Inline>, tag: Option<OpenBlock>) {
    if inlines.is_empty() {
        return;
    }
    let inlines = take_inlines(inlines);
    let kind = match tag.unwrap_or(OpenBlock::Paragraph) {
        OpenBlock::Heading(level) => BlockKind::Heading {
            level: level.clamp(1, 6),
            inlines,
        },
        OpenBlock::Quote => BlockKind::Quote { inlines },
        OpenBlock::Code => BlockKind::Code { inlines },
        OpenBlock::Paragraph => BlockKind::Paragraph { inlines },
    };
    blocks.push(Block {
        id: Id::new(),
        kind,
    });
}

fn take_inlines(inlines: &mut Vec<Inline>) -> Vec<Inline> {
    if inlines.is_empty() {
        vec![Inline::text("")]
    } else {
        std::mem::take(inlines)
    }
}

fn push_text(inlines: &mut Vec<Inline>, text: &str, marks: &Marks) {
    if let Some(last) = inlines.last_mut() {
        if last.marks == *marks {
            last.text.push_str(text);
            return;
        }
    }
    inlines.push(Inline {
        id: Id::new(),
        text: text.to_string(),
        marks: marks.clone(),
    });
}

fn reuse_ids(old: &[Block], mut new: Vec<Block>) -> Vec<Block> {
    for (index, block) in new.iter_mut().enumerate() {
        let Some(previous) = old.get(index) else {
            continue;
        };
        block.id = previous.id;
        match (&previous.kind, &mut block.kind) {
            (BlockKind::Paragraph { inlines: old_in }, BlockKind::Paragraph { inlines: new_in })
            | (BlockKind::Quote { inlines: old_in }, BlockKind::Quote { inlines: new_in })
            | (
                BlockKind::Heading { inlines: old_in, .. },
                BlockKind::Heading { inlines: new_in, .. },
            )
            | (BlockKind::Code { inlines: old_in }, BlockKind::Code { inlines: new_in }) => {
                zip_inlines(old_in, new_in)
            }
            (BlockKind::List { items: old_items, .. }, BlockKind::List { items: new_items, .. }) => {
                for (old_item, new_item) in old_items.iter().zip(new_items.iter_mut()) {
                    new_item.id = old_item.id;
                    zip_inlines(&old_item.inlines, &mut new_item.inlines);
                }
            }
            _ => {}
        }
    }
    new
}

fn zip_inlines(old: &[Inline], new: &mut [Inline]) {
    for (old_inline, new_inline) in old.iter().zip(new.iter_mut()) {
        new_inline.id = old_inline.id;
    }
}

#[derive(Debug)]
enum Token {
    Text(String),
    Start(String, Vec<(String, String)>),
    End(String),
}

fn tokenize(html: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = html.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '<' {
            let end = chars[i + 1..]
                .iter()
                .position(|ch| *ch == '>')
                .map(|pos| i + 1 + pos);
            let Some(end) = end else {
                tokens.push(Token::Text(chars[i..].iter().collect()));
                break;
            };
            let inner: String = chars[i + 1..end].iter().collect();
            i = end + 1;
            let inner = inner.trim();
            if inner.starts_with("!--") || inner.starts_with('!') || inner.starts_with('?') {
                continue;
            }
            if let Some(name) = inner.strip_prefix('/') {
                tokens.push(Token::End(name.split_whitespace().next().unwrap_or(name).to_lowercase()));
            } else {
                let self_closing = inner.ends_with('/');
                let cleaned = inner.trim_end_matches('/').trim();
                let (name, attrs) = parse_start_tag(cleaned);
                if name == "br" || self_closing && name == "br" {
                    tokens.push(Token::Start(name, attrs));
                } else if self_closing {
                    tokens.push(Token::Start(name.clone(), attrs));
                    tokens.push(Token::End(name));
                } else {
                    tokens.push(Token::Start(name, attrs));
                }
            }
        } else {
            let start = i;
            while i < chars.len() && chars[i] != '<' {
                i += 1;
            }
            tokens.push(Token::Text(chars[start..i].iter().collect()));
        }
    }
    tokens
}

fn parse_start_tag(tag: &str) -> (String, Vec<(String, String)>) {
    let mut parts = tag.split_whitespace();
    let name = parts.next().unwrap_or("div").to_lowercase();
    let rest = tag[name.len()..].trim();
    let mut attrs = Vec::new();
    let mut rest = rest;
    while !rest.is_empty() {
        let Some(eq) = rest.find('=') else {
            break;
        };
        let key = rest[..eq].trim().to_lowercase();
        rest = rest[eq + 1..].trim();
        let (value, next) = if let Some(stripped) = rest.strip_prefix('"') {
            let end = stripped.find('"').unwrap_or(stripped.len());
            (stripped[..end].to_string(), stripped.get(end + 1..).unwrap_or(""))
        } else if let Some(stripped) = rest.strip_prefix('\'') {
            let end = stripped.find('\'').unwrap_or(stripped.len());
            (stripped[..end].to_string(), stripped.get(end + 1..).unwrap_or(""))
        } else {
            let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
            (rest[..end].to_string(), rest.get(end..).unwrap_or(""))
        };
        attrs.push((key, value));
        rest = next.trim();
    }
    (name, attrs)
}

fn unescape(text: &str) -> String {
    text.replace("&nbsp;", " ")
        .replace("&#160;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace('\u{a0}', " ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{join_inlines, Document};

    #[test]
    fn html_round_trips_marks_and_blocks() {
        let mut doc = Document::new(Id::new());
        doc.blocks = vec![
            Block {
                id: Id::new(),
                kind: BlockKind::Heading {
                    level: 2,
                    inlines: vec![Inline::text("Title")],
                },
            },
            Block {
                id: Id::new(),
                kind: BlockKind::Paragraph {
                    inlines: vec![
                        Inline::text("Hello "),
                        Inline {
                            id: Id::new(),
                            text: "bold".into(),
                            marks: Marks {
                                bold: true,
                                ..Marks::default()
                            },
                        },
                    ],
                },
            },
            Block {
                id: Id::new(),
                kind: BlockKind::Quote {
                    inlines: vec![Inline::text("Quoted")],
                },
            },
        ];
        let html = doc.to_html();
        assert!(html.contains("<h2>Title</h2>"));
        assert!(html.contains("<strong>bold</strong>"));
        let mut restored = Document::new(doc.id);
        restored.replace_html(&html);
        assert_eq!(restored.to_plain_text(), "Title\nHello bold\nQuoted");
        let paragraph = restored.blocks[1].plain_text();
        assert_eq!(paragraph, "Hello bold");
        match &restored.blocks[1].kind {
            BlockKind::Paragraph { inlines } => {
                assert!(inlines.iter().any(|inline| inline.marks.bold && inline.text == "bold"));
            }
            _ => panic!("expected paragraph"),
        }
    }

    #[test]
    fn lists_round_trip() {
        let mut doc = Document::new(Id::new());
        doc.blocks = vec![Block {
            id: Id::new(),
            kind: BlockKind::List {
                ordered: false,
                items: vec![
                    ListItem {
                        id: Id::new(),
                        inlines: vec![Inline::text("One")],
                    },
                    ListItem {
                        id: Id::new(),
                        inlines: vec![Inline::text("Two")],
                    },
                ],
            },
        }];
        let html = doc.to_html();
        let mut restored = Document::new(doc.id);
        restored.replace_html(&html);
        match &restored.blocks[0].kind {
            BlockKind::List { ordered, items } => {
                assert!(!*ordered);
                assert_eq!(items.len(), 2);
                assert_eq!(join_inlines(&items[1].inlines), "Two");
            }
            _ => panic!("expected list"),
        }
    }

    #[test]
    fn links_comments_code_and_scene_breaks_round_trip() {
        let mut doc = Document::new(Id::new());
        doc.blocks = vec![
            Block {
                id: Id::new(),
                kind: BlockKind::Paragraph {
                    inlines: vec![Inline {
                        id: Id::new(),
                        text: "harbor".into(),
                        marks: Marks {
                            link: Some("https://example.com/tide".into()),
                            comment: Some("check the tide".into()),
                            ..Marks::default()
                        },
                    }],
                },
            },
            Block {
                id: Id::new(),
                kind: BlockKind::Code {
                    inlines: vec![Inline::text("let x = 1;")],
                },
            },
            Block {
                id: Id::new(),
                kind: BlockKind::SceneBreak,
            },
            Block::paragraph("After"),
        ];
        let html = doc.to_html();
        assert!(html.contains("href=\"https://example.com/tide\""));
        assert!(html.contains("data-comment=\"check the tide\""));
        assert!(html.contains("<pre>"));
        assert!(html.contains("<hr>"));
        let mut restored = Document::new(doc.id);
        restored.replace_html(&html);
        assert_eq!(restored.blocks.len(), 4);
        match &restored.blocks[0].kind {
            BlockKind::Paragraph { inlines } => {
                assert_eq!(
                    inlines[0].marks.link.as_deref(),
                    Some("https://example.com/tide")
                );
                assert_eq!(inlines[0].marks.comment.as_deref(), Some("check the tide"));
            }
            other => panic!("expected paragraph, got {other:?}"),
        }
        assert!(matches!(restored.blocks[1].kind, BlockKind::Code { .. }));
        assert!(matches!(restored.blocks[2].kind, BlockKind::SceneBreak));
        assert_eq!(restored.blocks[3].plain_text(), "After");
    }

    #[test]
    fn script_links_are_dropped() {
        let mut doc = Document::new(Id::new());
        doc.replace_html(r#"<p><a href="javascript:alert(1)">nope</a></p>"#);
        match &doc.blocks[0].kind {
            BlockKind::Paragraph { inlines } => {
                assert_eq!(inlines[0].text, "nope");
                assert!(inlines[0].marks.link.is_none());
            }
            other => panic!("expected paragraph, got {other:?}"),
        }
    }
}
