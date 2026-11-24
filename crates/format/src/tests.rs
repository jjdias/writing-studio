use writing_studio_core::Id;
use writing_studio_document::{
    Block, BlockKind, Document, Highlight, Inline, ListItem, Marks, NoteKind,
};

use crate::{blocks_to_markdown, canonical_blocks, markdown_to_blocks, same_content};

fn run(text: &str, marks: Marks) -> Inline {
    Inline {
        id: Id::new(),
        text: text.into(),
        marks,
    }
}

fn paragraph(inlines: Vec<Inline>) -> Block {
    Block {
        id: Id::new(),
        kind: BlockKind::Paragraph { inlines },
    }
}

fn bold() -> Marks {
    Marks {
        bold: true,
        ..Marks::default()
    }
}

/// Write, read, and check that nothing changed; then check that writing the
/// result gives the same file.
fn assert_round_trip(blocks: &[Block]) -> String {
    let wanted = canonical_blocks(blocks);
    let text = blocks_to_markdown(blocks);
    let read = markdown_to_blocks(&text);
    assert!(
        same_content(&wanted, &read),
        "not the same after reading back.\nfile:\n{text}\nwanted: {wanted:#?}\nread: {read:#?}"
    );
    assert_eq!(blocks_to_markdown(&read), text, "the second write differs");
    text
}

#[test]
fn a_scene_reads_as_plain_markdown() {
    let mut document = Document::new(Id::new());
    document.replace_html(concat!(
        "<h2>The Harbor</h2>",
        "<p>Mara ran for the <strong>north</strong> quay. <em>Not again</em>, she thought.</p>",
        "<p>“Wait,” said the pilot.<br>Nobody waited.</p>",
        "<blockquote>Tide tables are a promise.</blockquote>",
        "<ul><li>rope</li><li>lantern</li></ul>",
        "<hr>",
        "<p>See <a href=\"https://example.com/tides\">the tables</a>.</p>",
    ));
    let text = assert_round_trip(&document.blocks);
    assert_eq!(
        text,
        concat!(
            "## The Harbor\n\n",
            "Mara ran for the **north** quay. *Not again*, she thought.\n\n",
            "“Wait,” said the pilot.\\\nNobody waited.\n\n",
            "> Tide tables are a promise.\n\n",
            "- rope\n- lantern\n\n",
            "* * *\n\n",
            "See [the tables](https://example.com/tides).\n",
        )
    );
}

#[test]
fn notes_and_highlights_use_critic_markup() {
    let note = |text: &str, kind, resolved, highlight| Marks {
        comment: Some(text.into()),
        comment_kind: kind,
        comment_resolved: resolved,
        highlight,
        ..Marks::default()
    };
    let blocks = vec![
        paragraph(vec![
            run("The ", Marks::default()),
            run("harbor", note("which one?", NoteKind::Question, false, None)),
            run(" was ", Marks::default()),
            run(
                "quiet",
                Marks {
                    highlight: Some(Highlight::Yellow),
                    ..Marks::default()
                },
            ),
            run(" and ", Marks::default()),
            run(
                "cold",
                Marks {
                    highlight: Some(Highlight::Green),
                    ..Marks::default()
                },
            ),
            run(".", Marks::default()),
        ]),
        paragraph(vec![
            run("Too long.", note("cut", NoteKind::Revise, true, Some(Highlight::Yellow))),
            run(" ", Marks::default()),
            run("Fine.", note("Question: is it?", NoteKind::Comment, false, None)),
        ]),
    ];
    let text = assert_round_trip(&blocks);
    assert_eq!(
        text,
        concat!(
            "The {==harbor==}{>>Question: which one?<<} was {==quiet==} and {==cold==}{.green}.\n\n",
            "{==Too long.==}{.yellow}{>>(resolved) Revise: cut<<} ",
            "{==Fine.==}{>>Comment: Question: is it?<<}\n",
        )
    );
}

/// For a reader: open notes become footnotes, with their kind; resolved
/// notes, highlights, and empty paragraphs are left out.
#[test]
fn footnote_style_shows_a_reader_only_the_open_notes() {
    let note = |text: &str, kind, resolved| Marks {
        comment: Some(text.into()),
        comment_kind: kind,
        comment_resolved: resolved,
        highlight: Some(Highlight::Green),
        ..Marks::default()
    };
    let blocks = vec![
        paragraph(vec![
            run("The ", Marks::default()),
            run("harbor", note("which one?", NoteKind::Question, false)),
            run(" and ", Marks::default()),
            run("quay", note("done", NoteKind::Comment, true)),
            run(".", Marks::default()),
        ]),
        Block::paragraph(""),
        Block {
            id: Id::new(),
            kind: BlockKind::Code {
                inlines: vec![run("let x = 1;", Marks::default())],
            },
        },
    ];
    assert_eq!(
        crate::write_markdown(&blocks, crate::NoteStyle::Footnotes),
        "The harbor[^1] and quay.\n\n```\nlet x = 1;\n```\n\n[^1]: Question: which one?\n"
    );
}

#[test]
fn a_note_over_text_that_is_partly_formatted_is_still_one_note() {
    let noted = Marks {
        comment: Some("check".into()),
        ..Marks::default()
    };
    let blocks = vec![paragraph(vec![
        run("harbor ", noted.clone()),
        run(
            "was",
            Marks {
                bold: true,
                highlight: Some(Highlight::Yellow),
                ..noted.clone()
            },
        ),
        run(
            " quiet",
            Marks {
                highlight: Some(Highlight::Yellow),
                ..Marks::default()
            },
        ),
    ])];
    let text = assert_round_trip(&blocks);
    assert_eq!(
        text,
        "{==harbor ==}{>>check<<}{==**was**==}{.yellow}{>>check<<}{== quiet==}\n"
    );
    let mut document = Document::new(Id::new());
    document.blocks = markdown_to_blocks(&text);
    let items = document.review_items();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].text, "harbor was");
    assert_eq!(items[1].text, "was quiet");
}

#[test]
fn what_the_writer_typed_is_never_read_as_formatting() {
    for line in [
        "# not a heading",
        "- not a list",
        "+ nor this",
        "1. not numbered",
        "12) nor this",
        "> not a quote",
        "---",
        "***",
        "* * *",
        "```",
        "<br>",
        "<u>not underlined</u>",
        "a *star* and _underscore_ and `tick`",
        "[not](a link) and {==not==} a {>>note<<}",
        "x == y ~~ z",
        "AT&T &amp; &#32; &nbsp; & alone",
        "  two leading spaces and two trailing  ",
        "\tTab first",
        "back\\slash and a last one\\",
        "= alone",
        "a ~tilde~ pair",
        "",
        " ",
    ] {
        let blocks = vec![paragraph(vec![run(line, Marks::default())])];
        let text = assert_round_trip(&blocks);
        let read = markdown_to_blocks(&text);
        let BlockKind::Paragraph { inlines } = &read[0].kind else {
            panic!("{line:?} was read as {:?}", read[0].kind);
        };
        let back: String = inlines.iter().map(|inline| inline.text.as_str()).collect();
        assert_eq!(back, line);
        assert!(inlines.iter().all(|inline| inline.marks == Marks::default()));
    }
}

#[test]
fn formatting_keeps_off_the_spaces_at_its_edges() {
    let blocks = vec![paragraph(vec![
        run("a", Marks::default()),
        run(" bold ", bold()),
        run("b", Marks::default()),
    ])];
    assert_eq!(assert_round_trip(&blocks), "a **bold** b\n");

    // Underline does cover its spaces: the line under a space can be seen.
    let underline = Marks {
        underline: true,
        ..Marks::default()
    };
    let blocks = vec![paragraph(vec![run(" under ", underline)])];
    assert_eq!(assert_round_trip(&blocks), "<u> under </u>\n");
}

#[test]
fn a_line_break_at_the_end_of_a_block_is_dropped() {
    let blocks = vec![
        Block::paragraph("line\n"),
        Block::paragraph("\n\n"),
        Block::heading(2, "Title\n"),
    ];
    assert_eq!(assert_round_trip(&blocks), "line\n\n<br>\n\n## Title\n");
}

#[test]
fn bold_and_italic_change_in_every_order() {
    let marks = |bold, italic| Marks {
        bold,
        italic,
        ..Marks::default()
    };
    let states = [(false, false), (true, false), (false, true), (true, true)];
    for first in states {
        for second in states {
            for third in states {
                let blocks = vec![paragraph(vec![
                    run("a", marks(first.0, first.1)),
                    run("b", marks(second.0, second.1)),
                    run("c", marks(third.0, third.1)),
                ])];
                assert_round_trip(&blocks);
            }
        }
    }
}

#[test]
fn every_kind_of_block_round_trips_with_its_awkward_cases() {
    let item = |text: &str| ListItem {
        id: Id::new(),
        inlines: vec![Inline::text(text)],
    };
    let list = |ordered, items: Vec<ListItem>| Block {
        id: Id::new(),
        kind: BlockKind::List { ordered, items },
    };
    let code = |text: &str| Block {
        id: Id::new(),
        kind: BlockKind::Code {
            inlines: vec![Inline::text(text)],
        },
    };
    let blocks = vec![
        Block::heading(1, "C# and # marks #"),
        Block::heading(6, ""),
        Block {
            id: Id::new(),
            kind: BlockKind::Heading {
                level: 3,
                inlines: vec![Inline::text("two\nlines")],
            },
        },
        Block {
            id: Id::new(),
            kind: BlockKind::Quote {
                inlines: vec![Inline::text("first\n\n> third")],
            },
        },
        Block {
            id: Id::new(),
            kind: BlockKind::Quote {
                inlines: vec![Inline::text("")],
            },
        },
        // Two lists in a row stay two lists.
        list(false, vec![item("one"), item(""), item("- dash\nnext line")]),
        list(false, vec![item("another list")]),
        list(true, vec![item("first"), item("second")]),
        list(true, vec![item("again")]),
        code("let x = 1;\n\n    indented *not italic*\n"),
        code(""),
        code("```\nfence inside\n````"),
        // Indented backticks inside the code do not end it.
        code(" ```\n   ````"),
        Block {
            id: Id::new(),
            kind: BlockKind::SceneBreak,
        },
        Block::paragraph(""),
        Block::paragraph("line\n\nafter a blank line"),
        Block::paragraph("\nstarts with a break"),
    ];
    let text = assert_round_trip(&blocks);
    assert!(text.contains("# C\\# and \\# marks \\#\n"));
    assert!(text.contains("- one\n-\n- \\- dash<br>next line\n\n+ another list\n"));
    assert!(text.contains("1. first\n2. second\n\n1) again\n"));
    assert!(text.contains("`````\n```\nfence inside\n````\n`````\n"));
}

#[test]
fn links_keep_any_address_and_code_blocks_drop_formatting() {
    for url in [
        "https://example.com/a_(b)",
        "notes page.html",
        "mailto:mara@example.com",
        "https://example.com/a\\b<c>",
    ] {
        let link = Marks {
            link: Some(url.into()),
            italic: true,
            ..Marks::default()
        };
        let blocks = vec![paragraph(vec![run("here", link), run(" after", Marks::default())])];
        let read = markdown_to_blocks(&assert_round_trip(&blocks));
        let BlockKind::Paragraph { inlines } = &read[0].kind else {
            panic!("expected a paragraph");
        };
        assert_eq!(inlines[0].marks.link.as_deref(), Some(url));
    }

    let blocks = vec![Block {
        id: Id::new(),
        kind: BlockKind::Code {
            inlines: vec![run("a", bold()), run("b", Marks::default())],
        },
    }];
    assert_eq!(assert_round_trip(&blocks), "```\nab\n```\n");
}

#[test]
fn hand_written_markdown_is_read_without_losing_text() {
    let read = markdown_to_blocks(concat!(
        "A line that was\nwrapped by hand, with *italic **and bold** inside* it.\r\n",
        "\n",
        "***\n",
        "\n",
        "* star item\n* second\n  carried on\n",
        "\n",
        "An unclosed [bracket, a lone {== and a <span>tag</span> stay.\n",
        "```\ncode with no end",
    ));
    let text = |index: usize| -> String {
        match &read[index].kind {
            BlockKind::Paragraph { inlines } | BlockKind::Code { inlines } => {
                inlines.iter().map(|inline| inline.text.as_str()).collect()
            }
            other => panic!("unexpected {other:?}"),
        }
    };
    assert_eq!(text(0), "A line that was wrapped by hand, with italic and bold inside it.");
    let BlockKind::Paragraph { inlines } = &read[0].kind else {
        unreachable!()
    };
    assert!(inlines.iter().any(|inline| inline.marks.bold && inline.marks.italic));
    assert!(matches!(read[1].kind, BlockKind::SceneBreak));
    let BlockKind::List { ordered: false, items } = &read[2].kind else {
        panic!("expected a list");
    };
    assert_eq!(items.len(), 2);
    assert_eq!(items[1].inlines[0].text, "second carried on");
    assert_eq!(
        text(3),
        "An unclosed [bracket, a lone {== and a <span>tag</span> stay."
    );
    assert_eq!(text(4), "code with no end");
    assert!(matches!(markdown_to_blocks("")[0].kind, BlockKind::Paragraph { .. }));
}

/// A small generator of repeatable pseudo-random numbers, so the test below
/// needs no extra dependency and fails the same way every time.
struct Dice(u64);

impl Dice {
    fn roll(&mut self, sides: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 33) as usize) % sides
    }

    fn pick<'a>(&mut self, from: &[&'a str]) -> &'a str {
        from[self.roll(from.len())]
    }
}

const PIECES: &[&str] = &[
    "word", "Mara", " ", "  ", "\t", "\n", "é", "“quoted”", "—", ".", ",", "*", "**", "***", "_",
    "~", "~~", "=", "==", "{", "}", "{==", "==}", "{>>", "<<}", "{.green}", "[", "]", "](", "(",
    ")", "<", ">", "<u>", "</u>", "<br>", "\\", "\\\\", "#", "# ", "- ", "+ ", "1. ", "2) ", "> ",
    "---", "```", "`", "&", "&amp;", "&#32;", "&#x41;", "http://example.com", "|", "!", "\r",
    "</p>", "<p>", "<pre>", "</pre>", "<!--", "-->", "<script>", "\"", "'", "\u{a0}", "&nbsp;", "&lt;",
];
const LINKS: &[&str] = &[
    "https://example.com",
    "https://example.com/a_(b)",
    "notes page.html",
    "javascript:alert(1)",
    "",
];
const NOTES: &[&str] = &[
    "check the tide",
    "Question: really?",
    "(resolved) done",
    "two\nlines",
    "a <<} b {>> c",
    "back\\slash \\n",
    "   ",
];

fn random_inlines(dice: &mut Dice) -> Vec<Inline> {
    (0..dice.roll(6))
        .map(|_| {
            let text: String = (0..1 + dice.roll(4)).map(|_| dice.pick(PIECES)).collect();
            let mut marks = Marks {
                bold: dice.roll(3) == 0,
                italic: dice.roll(3) == 0,
                underline: dice.roll(5) == 0,
                strikethrough: dice.roll(5) == 0,
                ..Marks::default()
            };
            if dice.roll(5) == 0 {
                marks.link = Some(dice.pick(LINKS).to_string());
            }
            if dice.roll(4) == 0 {
                marks.comment = Some(dice.pick(NOTES).to_string());
                marks.comment_kind = NoteKind::ALL[dice.roll(4)];
                marks.comment_resolved = dice.roll(3) == 0;
            }
            if dice.roll(5) == 0 {
                marks.highlight = Some(Highlight::ALL[dice.roll(4)]);
            }
            run(&text, marks)
        })
        .collect()
}

#[test]
fn two_thousand_random_documents_round_trip() {
    let mut dice = Dice(0x5EED);
    for _ in 0..2000 {
        let blocks: Vec<Block> = (0..dice.roll(6))
            .map(|_| {
                let kind = match dice.roll(8) {
                    0 => BlockKind::Heading {
                        level: 1 + dice.roll(6) as u8,
                        inlines: random_inlines(&mut dice),
                    },
                    1 => BlockKind::Quote {
                        inlines: random_inlines(&mut dice),
                    },
                    2 => BlockKind::Code {
                        inlines: random_inlines(&mut dice),
                    },
                    3 => BlockKind::List {
                        ordered: dice.roll(2) == 0,
                        items: (0..dice.roll(4))
                            .map(|_| ListItem {
                                id: Id::new(),
                                inlines: random_inlines(&mut dice),
                            })
                            .collect(),
                    },
                    4 => BlockKind::SceneBreak,
                    _ => BlockKind::Paragraph {
                        inlines: random_inlines(&mut dice),
                    },
                };
                Block { id: Id::new(), kind }
            })
            .collect();
        assert_round_trip(&blocks);
    }
}

/// What the Markdown page sends while the writer is typing: marks not yet
/// closed, a list item or quote just started, a heading with no space. Each
/// is read without losing a character, and reads the same once written
/// back, so a scene left like this opens again as it was left.
#[test]
fn text_half_typed_on_the_page_reads_back_stably() {
    // Each with the letters and digits it should keep: a list's numbers
    // are its marker, not text.
    for (typed, wanted) in [
        ("It was **dar", "Itwasdar"),
        ("It was *dark", "Itwasdark"),
        ("==bright", "bright"),
        ("{==bright==}{>>why", "brightwhy"),
        ("[the harbor](", "theharbor"),
        ("#Heading with no space", "Headingwithnospace"),
        ("- ", ""),
        ("- one
- ", "one"),
        ("1. one
2. ", "one"),
        ("> ", ""),
        ("> She said
>", "Shesaid"),
        ("```
let x", "letx"),
        ("* * ", ""),
        ("trailing space ", "trailingspace"),
    ] {
        let read = markdown_to_blocks(typed);
        let written = blocks_to_markdown(&read);
        let again = markdown_to_blocks(&written);
        assert!(same_content(&read, &again), "{typed:?} read again differs:\n{written}");
        assert_eq!(blocks_to_markdown(&again), written, "{typed:?} is not stable");
        // Nothing the writer typed is gone, except the marks that made a
        // block and the spaces Markdown trims.
        let text: String = Document { blocks: read, ..Document::new(Id::new()) }.to_plain_text();
        let kept: String = text.chars().filter(|c| c.is_alphanumeric()).collect();
        assert_eq!(kept, wanted, "{typed:?} lost text");
    }
}

/// `==text==`, as other Markdown editors write a highlight, reads as a
/// yellow one; written back, it is `{==text==}`, which reads the same.
#[test]
fn double_equals_is_a_yellow_highlight() {
    let blocks = markdown_to_blocks("A ==bright== day, a == b, and ==open");
    let BlockKind::Paragraph { inlines } = &blocks[0].kind else {
        panic!("a paragraph");
    };
    let lit: Vec<(&str, Option<Highlight>)> = inlines.iter().map(|inline| (inline.text.as_str(), inline.marks.highlight)).collect();
    assert_eq!(
        lit,
        vec![("A ", None), ("bright", Some(Highlight::Yellow)), (" day, a == b, and ==open", None)]
    );
    assert_eq!(blocks_to_markdown(&blocks), "A {==bright==} day, a \\=\\= b, and \\=\\=open\n");
}

/// A footnote becomes a note on the word before its mark, and its line
/// goes. One with no text, or no word before it, stays as written.
#[test]
fn footnotes_are_read_as_notes_on_the_word_before_them() {
    let text = concat!(
        "The ship sailed[^1] at dawn, *slowly*[^tide].\n",
        "\n",
        "Code `x[^1]` and [^1] alone and a gap[^9].\n",
        "\n",
        "[^1]: Which ship? {Check}\n",
        "[^tide]: The tide was out.\n",
    );
    let blocks = markdown_to_blocks(text);
    assert_eq!(blocks.len(), 2, "the footnote lines are gone: {blocks:#?}");
    let BlockKind::Paragraph { inlines } = &blocks[0].kind else {
        panic!("a paragraph");
    };
    let noted: Vec<(&str, Option<&str>, bool)> = inlines
        .iter()
        .map(|inline| (inline.text.as_str(), inline.marks.comment.as_deref(), inline.marks.italic))
        .collect();
    assert_eq!(
        noted,
        vec![
            ("The ship ", None, false),
            ("sailed", Some("Which ship? {Check}"), false),
            (" at dawn, ", None, false),
            ("slowly", Some("The tide was out."), true),
            (".", None, false),
        ]
    );
    let BlockKind::Paragraph { inlines } = &blocks[1].kind else {
        panic!("a paragraph");
    };
    let text: String = inlines.iter().map(|inline| inline.text.as_str()).collect();
    // The reader has no inline code, so the backticks are text; the mark
    // inside them is still not taken for a footnote.
    assert_eq!(text, "Code `x[^1]` and [^1] alone and a gap[^9].");
    // Written back as notes, they read the same.
    let written = blocks_to_markdown(&blocks);
    assert!(same_content(&blocks, &markdown_to_blocks(&written)), "{written}");
}

/// `_italic_` and `__bold__`, as another editor or a person writes them,
/// read as `*` and `**` do; underscores inside a word are text.
#[test]
fn underscores_are_emphasis_except_inside_a_word() {
    let blocks = markdown_to_blocks("A _soft_ and __loud__ snake_case_name.");
    let BlockKind::Paragraph { inlines } = &blocks[0].kind else {
        panic!("a paragraph");
    };
    let read: Vec<(&str, bool, bool)> = inlines.iter().map(|inline| (inline.text.as_str(), inline.marks.italic, inline.marks.bold)).collect();
    assert_eq!(
        read,
        vec![
            ("A ", false, false),
            ("soft", true, false),
            (" and ", false, false),
            ("loud", false, true),
            (" snake_case_name.", false, false),
        ]
    );
}
