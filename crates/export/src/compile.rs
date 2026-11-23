use writing_studio_core::Id;
use writing_studio_document::{BlockKind, Document};
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
