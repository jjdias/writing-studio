//! What another Markdown reader makes of the files this program writes.
//!
//! The files are meant to be usable without this program. So the text and
//! the bold in them are read here with `pulldown-cmark`, an independent
//! CommonMark reader, and compared with what was written.

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use writing_studio_core::Id;
use writing_studio_document::{Block, BlockKind, Inline, Marks};
use writing_studio_format::{blocks_to_markdown, canonical_blocks};

struct Dice(u64);

impl Dice {
    fn roll(&mut self, sides: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 33) as usize) % sides
    }
}

const PIECES: &[&str] = &[
    "word", "Mara", " ", "  ", "\n", "é", "“quoted”", "—", ".", ",", "*", "**", "***", "_", "~",
    "~~", "=", "==", "{", "}", "{==", "==}", "[", "]", "](", "(", ")", "<", ">", "<u>", "<br>",
    "\\", "#", "# ", "- ", "+ ", "1. ", "2) ", "> ", "---", "```", "`", "&", "&amp;", "&#32;",
    "|", "!", "a", "b c", "\t",
];

struct Seen {
    blocks: Vec<String>,
    text: String,
    bold: String,
    italic: String,
    link: String,
}

fn read_with_another_reader(markdown: &str) -> Seen {
    let mut seen = Seen {
        blocks: Vec::new(),
        text: String::new(),
        bold: String::new(),
        italic: String::new(),
        link: String::new(),
    };
    let (mut bold, mut italic, mut link) = (0, 0, 0);
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    for event in Parser::new_ext(markdown, options) {
        let piece: Option<String> = match event {
            Event::Start(tag) => {
                match tag {
                    Tag::Strong => bold += 1,
                    Tag::Emphasis => italic += 1,
                    Tag::Link { .. } => link += 1,
                    Tag::Strikethrough => {}
                    other => seen.blocks.push(format!("{other:?}")),
                }
                None
            }
            Event::End(tag) => {
                match tag {
                    TagEnd::Strong => bold -= 1,
                    TagEnd::Emphasis => italic -= 1,
                    TagEnd::Link => link -= 1,
                    _ => {}
                }
                None
            }
            Event::Text(text) | Event::Code(text) => Some(text.to_string()),
            Event::SoftBreak => Some(" ".into()),
            Event::HardBreak => Some("\n".into()),
            // Tags are passed through by a Markdown reader and shown by
            // whatever displays the result, so they count as formatting.
            Event::InlineHtml(html) | Event::Html(html) => {
                match html.trim() {
                    "<strong>" => bold += 1,
                    "</strong>" => bold -= 1,
                    "<em>" => italic += 1,
                    "</em>" => italic -= 1,
                    _ => {}
                }
                html.starts_with("<br").then(|| "\n".to_string())
            }
            other => {
                seen.blocks.push(format!("{other:?}"));
                None
            }
        };
        if let Some(piece) = piece {
            seen.text.push_str(&piece);
            if bold > 0 {
                seen.bold.push_str(&piece);
            }
            if italic > 0 {
                seen.italic.push_str(&piece);
            }
            if link > 0 {
                seen.link.push_str(&piece);
            }
        }
    }
    seen
}

fn random_paragraph(dice: &mut Dice, formatted: bool) -> Vec<Inline> {
    (0..1 + dice.roll(4))
        .map(|_| {
            let text: String = (0..1 + dice.roll(4))
                .map(|_| PIECES[dice.roll(PIECES.len())])
                .collect();
            let mut marks = Marks::default();
            if formatted {
                marks.bold = dice.roll(3) == 0;
                marks.italic = dice.roll(3) == 0;
                marks.underline = dice.roll(5) == 0;
                marks.strikethrough = dice.roll(6) == 0;
                if dice.roll(6) == 0 {
                    marks.link = Some("https://example.com/a_(b)".into());
                }
            }
            Inline {
                id: Id::new(),
                text,
                marks,
            }
        })
        .collect()
}

fn collect(inlines: &[Inline], keep: fn(&Marks) -> bool) -> String {
    inlines
        .iter()
        .filter(|inline| keep(&inline.marks))
        .map(|inline| inline.text.as_str())
        .collect()
}

/// Text with no formatting at all: whatever the writer typed, another reader
/// sees one paragraph with exactly that text and nothing bold, italic, or
/// linked.
#[test]
fn another_reader_sees_plain_text_as_the_same_plain_text() {
    let mut dice = Dice(99);
    for _ in 0..20_000 {
        let inlines = random_paragraph(&mut dice, false);
        let blocks = canonical_blocks(&[Block {
            id: Id::new(),
            kind: BlockKind::Paragraph { inlines },
        }]);
        let BlockKind::Paragraph { inlines } = &blocks[0].kind else {
            unreachable!()
        };
        let text = collect(inlines, |_| true);
        if text.is_empty() {
            continue;
        }
        let markdown = blocks_to_markdown(&blocks);
        let seen = read_with_another_reader(&markdown);
        assert_eq!(seen.blocks, ["Paragraph"], "file: {markdown:?}");
        assert_eq!(seen.text, text, "file: {markdown:?}");
        assert!(
            seen.bold.is_empty() && seen.italic.is_empty() && seen.link.is_empty(),
            "file: {markdown:?}"
        );
    }
}

const WORDS: &[&str] = &[
    "Mara", "ran", "for", "the", "harbor", "tide", "was", "not", "again", "she", "thought", "é",
    "naïve", "don’t", "o'clock", "1840", "boat",
];
const AFTER_A_WORD: &[&str] = &["", "", "", "", ",", ".", "!", "?", "…", "”", ".”", ";", ":", ")"];
const BEFORE_A_WORD: &[&str] = &["", "", "", "", "", "“", "(", "‘"];

/// Text shaped like prose: words with their punctuation, and formatting
/// that starts and stops between words or inside one. Another reader sees
/// the same words with the same bold, italic, and links.
#[test]
fn another_reader_sees_formatted_prose_the_same() {
    let mut dice = Dice(7);
    for _ in 0..20_000 {
        let mut inlines: Vec<Inline> = Vec::new();
        for _ in 0..1 + dice.roll(5) {
            let mut text = String::new();
            for word in 0..1 + dice.roll(4) {
                if word > 0 || dice.roll(4) > 0 {
                    text.push(' ');
                }
                text.push_str(BEFORE_A_WORD[dice.roll(BEFORE_A_WORD.len())]);
                text.push_str(WORDS[dice.roll(WORDS.len())]);
                text.push_str(AFTER_A_WORD[dice.roll(AFTER_A_WORD.len())]);
            }
            if dice.roll(3) == 0 {
                text.push(' ');
            }
            let mut marks = Marks {
                bold: dice.roll(3) == 0,
                italic: dice.roll(3) == 0,
                underline: dice.roll(6) == 0,
                ..Marks::default()
            };
            if dice.roll(8) == 0 {
                marks.link = Some("https://example.com/tides".into());
            }
            inlines.push(Inline {
                id: Id::new(),
                text,
                marks,
            });
        }
        let blocks = canonical_blocks(&[Block {
            id: Id::new(),
            kind: BlockKind::Paragraph { inlines },
        }]);
        let BlockKind::Paragraph { inlines } = &blocks[0].kind else {
            unreachable!()
        };
        let markdown = blocks_to_markdown(&blocks);
        let seen = read_with_another_reader(&markdown);
        assert_eq!(seen.text, collect(inlines, |_| true), "file: {markdown:?}");
        assert_eq!(seen.bold, collect(inlines, |m| m.bold), "bold in file: {markdown:?}");
        assert_eq!(seen.italic, collect(inlines, |m| m.italic), "italic in file: {markdown:?}");
        assert_eq!(seen.link, collect(inlines, |m| m.link.is_some()), "link in file: {markdown:?}");
    }
}

/// Formatting pressed against every awkward character. Here another reader
/// can, rarely, disagree about what is bold or italic, and show a `*` as
/// itself. It never loses a letter or a digit.
#[test]
fn another_reader_never_loses_a_word_even_in_the_worst_case() {
    let mut dice = Dice(7);
    let (mut cases, mut differ) = (0, 0);
    for _ in 0..20_000 {
        let inlines = random_paragraph(&mut dice, true);
        let blocks = canonical_blocks(&[Block {
            id: Id::new(),
            kind: BlockKind::Paragraph { inlines },
        }]);
        let BlockKind::Paragraph { inlines } = &blocks[0].kind else {
            unreachable!()
        };
        let markdown = blocks_to_markdown(&blocks);
        let seen = read_with_another_reader(&markdown);
        let letters = |text: &str| -> String { text.chars().filter(|ch| ch.is_alphanumeric()).collect() };
        assert_eq!(
            letters(&seen.text),
            letters(&collect(inlines, |_| true)),
            "file: {markdown:?}"
        );
        cases += 1;
        let same = seen.text == collect(inlines, |_| true)
            && seen.bold == collect(inlines, |m| m.bold)
            && seen.italic == collect(inlines, |m| m.italic)
            && seen.link == collect(inlines, |m| m.link.is_some());
        if !same {
            differ += 1;
        }
    }
    println!("{differ} of {cases} worst-case paragraphs show different formatting in another reader");
    // About one in three hundred when this was written. Hold it under one in
    // a hundred, so that a change that makes it much worse is noticed.
    assert!(differ * 100 < cases, "{differ} of {cases}");
}
