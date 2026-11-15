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

// ---- The stored format: HTML ----
//
// These cover what the Markdown tests above used to cover when Markdown was
// the stored format: that a scene file reads back as the same document.

use crate::{blocks_to_html, html_to_blocks, stored_blocks};

/// Write as HTML, read back, and check that nothing at all changed.
fn assert_stored_exactly(blocks: &[Block]) -> String {
    let wanted = stored_blocks(blocks);
    let text = blocks_to_html(blocks);
    let read = html_to_blocks(&text);
    assert!(
        same_content(&wanted, &read),
        "not the same after reading back.\nfile:\n{text}\nwanted: {wanted:#?}\nread: {read:#?}"
    );
    assert_eq!(blocks_to_html(&read), text, "the second write differs");
    text
}

#[test]
fn html_puts_each_block_on_its_own_line() {
    let note = Marks {
        comment: Some("which one?".into()),
        comment_kind: NoteKind::Question,
        ..Marks::default()
    };
    let item = |text: &str| ListItem {
        id: Id::new(),
        inlines: vec![Inline::text(text)],
    };
    let blocks = vec![
        Block::heading(2, "The Harbor"),
        paragraph(vec![
            run("The ", note.clone()),
            run(
                "harbor",
                Marks {
                    highlight: Some(Highlight::Green),
                    bold: true,
                    ..note.clone()
                },
            ),
            run(" was quiet & cold.\nNobody waited.", Marks::default()),
        ]),
        Block::paragraph(""),
        Block {
            id: Id::new(),
            kind: BlockKind::List {
                ordered: true,
                items: vec![item("rope"), item("")],
            },
        },
        Block {
            id: Id::new(),
            kind: BlockKind::Code {
                inlines: vec![run("depth = 3.5\n  # fathoms", Marks::default())],
            },
        },
        Block {
            id: Id::new(),
            kind: BlockKind::SceneBreak,
        },
    ];
    assert_eq!(
        assert_stored_exactly(&blocks),
        concat!(
            "<h2>The Harbor</h2>\n",
            "<p><span class=\"note\" data-note=\"which one?\" data-kind=\"question\">The ",
            "<mark class=\"green\"><strong>harbor</strong></mark></span>",
            " was quiet &amp; cold.<br>Nobody waited.</p>\n",
            "<p></p>\n",
            "<ol>\n<li>rope</li>\n<li></li>\n</ol>\n",
            "<pre>\ndepth = 3.5\n  # fathoms</pre>\n",
            "<hr>\n",
        )
    );
}

#[test]
fn html_keeps_what_markdown_has_to_drop() {
    let blocks = vec![
        // Bold on the spaces at its edges, and a line break at the very end.
        paragraph(vec![run(" bold ", bold()), run("after\n", Marks::default())]),
        // Formatting inside a code block.
        Block {
            id: Id::new(),
            kind: BlockKind::Code {
                inlines: vec![run("\nlet ", Marks::default()), run("x", bold())],
            },
        },
        // A list with nothing in it, and text that is only spaces.
        Block {
            id: Id::new(),
            kind: BlockKind::List {
                ordered: false,
                items: Vec::new(),
            },
        },
        Block::paragraph("  \t "),
        Block::paragraph("no\u{a0}break"),
    ];
    assert_eq!(
        assert_stored_exactly(&blocks),
        concat!(
            "<p><strong> bold </strong>after<br></p>\n",
            "<pre>\n\nlet <strong>x</strong></pre>\n",
            "<ul>\n</ul>\n",
            "<p>  \t </p>\n",
            "<p>no&nbsp;break</p>\n",
        )
    );
    // The Markdown export of the same blocks is not the same document.
    assert!(!same_content(&canonical_blocks(&blocks), &stored_blocks(&blocks)));
}

#[test]
fn html_tidied_or_written_by_hand_is_read_without_losing_text() {
    let read = html_to_blocks(concat!(
        "<!-- a comment -->\r\n",
        "<p>\n    A line that was\n    wrapped <b>and indented</b>\n</p>\n",
        "<div>A div is a paragraph, <unknown>and this stays</unknown>, 1 < 2 &copy; AT&T</div>",
        "Loose text is a paragraph too.",
        "<blockquote><p>A paragraph in a quote</p></blockquote>",
        "<UL><LI>one<li>two</UL>",
        "<p><span data-note=\"x\" data-resolved>noted</span> <mark>lit</mark> <i>it</i> <del>gone</del></p>",
    ));
    let text = |block: &Block| -> String {
        match &block.kind {
            BlockKind::Paragraph { inlines } => inlines.iter().map(|i| i.text.as_str()).collect(),
            other => panic!("unexpected {other:?}"),
        }
    };
    assert_eq!(text(&read[0]), "A line that was wrapped and indented");
    assert_eq!(
        text(&read[1]),
        "A div is a paragraph, and this stays, 1 < 2 &copy; AT&T"
    );
    assert_eq!(text(&read[2]), "Loose text is a paragraph too.");
    assert_eq!(text(&read[3]), "A paragraph in a quote");
    let BlockKind::List { items, .. } = &read[4].kind else {
        panic!("expected a list");
    };
    assert_eq!(items.len(), 2);
    let BlockKind::Paragraph { inlines } = &read[5].kind else {
        panic!("expected a paragraph");
    };
    assert!(inlines[0].marks.comment_resolved);
    assert_eq!(inlines[2].marks.highlight, Some(Highlight::Yellow));
    assert!(inlines[4].marks.italic && inlines[6].marks.strikethrough);
    assert_eq!(read.len(), 6);
}

#[test]
fn nothing_in_a_scene_file_is_ever_run() {
    let read = html_to_blocks(concat!(
        "<script>alert('never')</script><style>p { color: red }</style>",
        "<p onclick=\"alert(1)\"><a href=\"javascript:alert(1)\">a link</a> text</p>",
    ));
    assert_eq!(read.len(), 1);
    let BlockKind::Paragraph { inlines } = &read[0].kind else {
        panic!("expected a paragraph");
    };
    assert_eq!(inlines.len(), 1);
    assert_eq!(inlines[0].text, "a link text");
    assert!(inlines[0].marks.link.is_none());

    // And an address that is not safe is not written into a file a browser opens.
    let unsafe_link = Marks {
        link: Some("javascript:alert(1)".into()),
        ..Marks::default()
    };
    let written = blocks_to_html(&[paragraph(vec![run("x <script>", unsafe_link)])]);
    assert_eq!(written, "<p>x &lt;script&gt;</p>\n");
}

#[test]
fn two_thousand_random_documents_are_stored_exactly() {
    let mut dice = Dice(0xD15C);
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
        assert_stored_exactly(&blocks);
    }
}
