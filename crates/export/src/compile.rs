use writing_studio_core::Id;
use writing_studio_document::{BlockKind, Document, Inline, Marks};
use writing_studio_project::{BinderTree, CompileProfile, NodeKind, SceneSeparator};

pub struct CompileInput<'a> {
    pub title: &'a str,
    /// When set, only this book is compiled. Its title is the front matter.
    pub book_id: Option<Id>,
    pub tree: &'a BinderTree,
    pub documents: &'a [Document],
    /// The title page, the headings, what goes between scenes, the matter,
    /// and the page.
    pub profile: &'a CompileProfile,
    /// The writer's name, for the title page, the copyright line, and "Also by".
    pub author: &'a str,
    /// The book's subtitle, for the title page.
    pub subtitle: &'a str,
    /// The year the copyright line gives.
    pub year: i32,
}

/// A page of front or back matter: made by the compile, not a scene.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MatterPage {
    /// The section's title is the book's.
    Title { subtitle: String, author: String },
    Copyright(String),
    /// One line of the dedication to each entry.
    Dedication(Vec<String>),
    Contents(Vec<ContentsEntry>),
    /// One paragraph to each entry.
    AboutAuthor(Vec<String>),
    AlsoBy(Vec<String>),
}

/// A part or chapter in the table of contents. `depth` is 0 for the
/// outermost ones compiled.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContentsEntry {
    pub depth: u8,
    pub title: String,
}

pub struct CompiledSection<'a> {
    pub heading_level: u8,
    /// The heading, or `None` for a scene the profile sets off without one.
    pub title: Option<String>,
    pub document: Option<&'a Document>,
    /// What the section is in the binder. `None` for matter.
    pub kind: Option<NodeKind>,
    /// Set for a page of front or back matter.
    pub matter: Option<MatterPage>,
    /// The first section of the text after the front matter.
    pub after_front_matter: bool,
    /// A scene break or an empty line between this scene and the one before
    /// it in the same chapter.
    pub break_before: Option<SceneSeparator>,
}

impl CompiledSection<'_> {
    fn matter(heading_level: u8, title: Option<String>, page: MatterPage) -> Self {
        Self {
            heading_level,
            title,
            document: None,
            kind: None,
            matter: Some(page),
            after_front_matter: false,
            break_before: None,
        }
    }
}

/// The book in reading order, as the profile sets it, between its front and
/// back matter. An item left out of compile is skipped with what it holds,
/// and so is the trash. Parts and chapters are numbered among those
/// compiled, across the book. A matter page with no text is left out.
pub fn collect_sections<'a>(input: &'a CompileInput<'a>) -> Vec<CompiledSection<'a>> {
    let profile = input.profile;
    let (root, title) = match input.book_id {
        Some(book_id) => match input.tree.get(book_id) {
            Some(book) if book.kind == NodeKind::Book && !input.tree.is_in_trash(book.id) && !book.left_out => {
                (Some(book.id), book.title.clone())
            }
            _ => (None, input.title.to_string()),
        },
        None => (input.tree.draft_root().map(|draft| draft.id), input.title.to_string()),
    };
    // Under a title page the book's own heading is the first level, so
    // its parts and chapters start one below it.
    let depth = if profile.title_page { 2 } else { 1 };
    let mut walker = Walker { input, sections: Vec::new(), chapters: 0, parts: 0 };
    if let Some(root) = root {
        walker.walk(root, depth);
    }
    let mut body = walker.sections;

    let matter = &profile.matter;
    let author = input.author.trim();
    let mut sections = Vec::new();
    if profile.title_page {
        let page = MatterPage::Title { subtitle: input.subtitle.trim().to_string(), author: author.to_string() };
        sections.push(CompiledSection::matter(1, Some(title), page));
    }
    if matter.copyright {
        let line = matter.copyright_line(input.year, author);
        if !line.is_empty() {
            sections.push(CompiledSection::matter(depth, None, MatterPage::Copyright(line)));
        }
    }
    if matter.dedication {
        let lines = lines_of(&matter.dedication_text);
        if !lines.is_empty() {
            sections.push(CompiledSection::matter(depth, None, MatterPage::Dedication(lines)));
        }
    }
    if matter.contents {
        let entries = contents_of(&body);
        if !entries.is_empty() {
            sections.push(CompiledSection::matter(depth, Some(profile.contents_heading()), MatterPage::Contents(entries)));
        }
    }
    if !sections.is_empty() {
        if let Some(first) = body.first_mut() {
            first.after_front_matter = true;
        }
    }
    sections.append(&mut body);
    if matter.about_author {
        let paragraphs = lines_of(&matter.about_author_text);
        if !paragraphs.is_empty() {
            let page = MatterPage::AboutAuthor(paragraphs);
            sections.push(CompiledSection::matter(depth, Some(profile.about_author_heading()), page));
        }
    }
    if matter.also_by {
        let titles = matter.also_by_titles();
        if !titles.is_empty() {
            let heading = profile.also_by_heading(author);
            sections.push(CompiledSection::matter(depth, Some(heading), MatterPage::AlsoBy(titles)));
        }
    }
    sections
}

fn lines_of(text: &str) -> Vec<String> {
    text.lines().map(str::trim).filter(|line| !line.is_empty()).map(String::from).collect()
}

/// The compiled parts and chapters, as their headings read.
pub(crate) fn contents_of(body: &[CompiledSection<'_>]) -> Vec<ContentsEntry> {
    let headed = || {
        body.iter().filter(|section| matches!(section.kind, Some(NodeKind::Part | NodeKind::Chapter)))
    };
    let top = headed().map(|section| section.heading_level).min().unwrap_or(1);
    headed()
        .filter_map(|section| {
            let title = section.title.clone()?;
            Some(ContentsEntry { depth: section.heading_level - top, title })
        })
        .collect()
}

/// Whether a document has words or a scene break to compile.
fn has_content(doc: &Document) -> bool {
    doc.blocks
        .iter()
        .any(|block| matches!(block.kind, BlockKind::SceneBreak) || !block.plain_text().trim().is_empty())
}

struct Walker<'a> {
    input: &'a CompileInput<'a>,
    sections: Vec<CompiledSection<'a>>,
    chapters: usize,
    parts: usize,
}

impl<'a> Walker<'a> {
    fn walk(&mut self, parent: Id, depth: u8) {
        let profile = self.input.profile;
        for node in self.input.tree.children(Some(parent)) {
            if node.kind.is_root() || self.input.tree.is_in_trash(node.id) || node.left_out {
                continue;
            }
            if matches!(node.kind, NodeKind::Note | NodeKind::Folder) {
                self.walk(node.id, depth);
                continue;
            }
            let (title, break_before) = match node.kind {
                NodeKind::Chapter => {
                    self.chapters += 1;
                    (Some(profile.chapters.heading(profile.language, &profile.chapter_word, self.chapters, &node.title)), None)
                }
                NodeKind::Part => {
                    self.parts += 1;
                    (Some(profile.parts.heading(profile.language, &profile.part_word, self.parts, &node.title)), None)
                }
                NodeKind::Scene => {
                    // A scene right after another scene is set off from it;
                    // the first one under a heading needs nothing.
                    let follows_scene = self.sections.last().is_some_and(|last| last.kind == Some(NodeKind::Scene));
                    match profile.scenes {
                        SceneSeparator::Title => (Some(node.title.clone()), None),
                        separator @ (SceneSeparator::Break | SceneSeparator::Blank) if follows_scene => {
                            (None, Some(separator))
                        }
                        _ => (None, None),
                    }
                }
                _ => (Some(node.title.clone()), None),
            };
            // A new chapter or part has text that is one empty paragraph,
            // which every format would write as an empty line.
            let document = node
                .document_id
                .and_then(|id| self.input.documents.iter().find(|doc| doc.id == id))
                .filter(|doc| has_content(doc));
            self.sections.push(CompiledSection {
                heading_level: depth.clamp(1, 6),
                title,
                document,
                kind: Some(node.kind),
                matter: None,
                after_front_matter: false,
                break_before,
            });
            self.walk(node.id, depth.saturating_add(1).min(6));
        }
    }
}

/// The documents with every `[[link]]` resolved for a reader. A link to a
/// note compiled as footnotes becomes a footnote: on the
/// link's own words with `[[Title|words]]`, or after the words before it
/// with `[[Title]]`, and its text is the note's opening, up to its first
/// heading. Any other link reads as its words, the title when it has none.
pub fn resolve_note_links(tree: &BinderTree, documents: &[Document]) -> Vec<Document> {
    use std::collections::HashMap;
    let texts: HashMap<Id, &Document> = documents.iter().map(|document| (document.id, document)).collect();
    let mut footnotes: HashMap<String, String> = HashMap::new();
    for node in &tree.nodes {
        // Each note has its own setting; setting a folder sets what is in it.
        if node.kind != NodeKind::Note || node.deleted_at.is_some() || !tree.is_in_notes(node.id) || !node.footnotes {
            continue;
        }
        let opening = node.document_id.and_then(|id| texts.get(&id)).map(|document| opening_text(document)).unwrap_or_default();
        if !opening.is_empty() {
            footnotes.entry(node.title.trim().to_lowercase()).or_insert(opening);
        }
    }
    documents
        .iter()
        .map(|document| {
            let mut resolved = document.clone();
            for block in &mut resolved.blocks {
                for inlines in writing_studio_document::inline_runs_mut(&mut block.kind) {
                    *inlines = resolve_inlines(inlines, &footnotes);
                }
            }
            resolved
        })
        .collect()
}

/// A note's opening: its text up to its first heading, as one paragraph,
/// with its own links read as their words.
fn opening_text(document: &Document) -> String {
    let mut parts = Vec::new();
    for block in &document.blocks {
        if matches!(block.kind, BlockKind::Heading { .. }) {
            break;
        }
        let text = block.plain_text();
        if !text.trim().is_empty() {
            parts.push(words_of_links(text.trim()));
        }
    }
    parts.join(" ")
}

fn words_of_links(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut at = 0;
    for link in writing_studio_format::wikilinks(text) {
        out.extend(&chars[at..link.start]);
        out.push_str(&link.shown);
        at = link.end;
    }
    out.extend(&chars[at..]);
    out
}

fn resolve_inlines(inlines: &[Inline], footnotes: &std::collections::HashMap<String, String>) -> Vec<Inline> {
    let mut out: Vec<Inline> = Vec::new();
    for inline in inlines {
        let links = writing_studio_format::wikilinks(&inline.text);
        if links.is_empty() {
            out.push(inline.clone());
            continue;
        }
        let chars: Vec<char> = inline.text.chars().collect();
        let mut at = 0;
        let piece = |text: String, marks: Marks| Inline { id: Id::new(), text, marks };
        for link in links {
            let before: String = chars[at..link.start].iter().collect();
            if !before.is_empty() {
                out.push(piece(before, inline.marks.clone()));
            }
            let footnote = footnotes.get(&link.title.trim().to_lowercase());
            let has_words = link.shown != link.title;
            match footnote {
                // `[[Title]]`: the number goes after the words before it.
                Some(text) if !has_words && out.last().is_some_and(|last| !last.text.trim().is_empty()) => {
                    let last = out.last_mut().expect("checked above");
                    last.marks.footnote = Some(match last.marks.footnote.take() {
                        Some(earlier) => format!("{earlier} {text}"),
                        None => text.clone(),
                    });
                }
                Some(text) => {
                    let mut marks = inline.marks.clone();
                    marks.footnote = Some(text.clone());
                    out.push(piece(link.shown.clone(), marks));
                }
                None => out.push(piece(link.shown.clone(), inline.marks.clone())),
            }
            at = link.end;
        }
        let after: String = chars[at..].iter().collect();
        if !after.is_empty() {
            out.push(piece(after, inline.marks.clone()));
        }
    }
    out
}
