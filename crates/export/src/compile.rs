use writing_studio_core::Id;
use writing_studio_document::Document;
use writing_studio_project::{BinderTree, CompileProfile, NodeKind, SceneSeparator};

pub struct CompileInput<'a> {
    pub title: &'a str,
    /// When set, only this book is compiled. Its title is the front matter.
    pub book_id: Option<Id>,
    pub tree: &'a BinderTree,
    pub documents: &'a [Document],
    /// The title page, the headings, what goes between scenes, and the page.
    pub profile: &'a CompileProfile,
}

pub struct CompiledSection<'a> {
    pub heading_level: u8,
    /// The heading, or `None` for a scene the profile sets off without one.
    pub title: Option<String>,
    pub document: Option<&'a Document>,
    /// What the section is in the binder. `None` for the title page.
    pub kind: Option<NodeKind>,
    /// The first section after the title page.
    pub after_title_page: bool,
    /// A scene break or an empty line between this scene and the one before
    /// it in the same chapter.
    pub break_before: Option<SceneSeparator>,
}

/// The book in reading order, as the profile sets it. An item left out of
/// compile is skipped with what it holds, and so is the trash. Parts and
/// chapters are numbered among those compiled, across the book.
pub fn collect_sections<'a>(input: &'a CompileInput<'a>) -> Vec<CompiledSection<'a>> {
    let mut walker = Walker { input, sections: Vec::new(), chapters: 0, parts: 0 };
    let title_page = input.profile.title_page;
    if title_page {
        walker.sections.push(CompiledSection {
            heading_level: 1,
            title: Some(input.title.to_string()),
            document: None,
            kind: None,
            after_title_page: false,
            break_before: None,
        });
    }
    let start_depth = if title_page { 2 } else { 1 };
    if let Some(book_id) = input.book_id {
        let Some(book) = input.tree.get(book_id) else {
            return walker.sections;
        };
        if book.kind != NodeKind::Book || input.tree.is_in_trash(book.id) || book.left_out {
            return walker.sections;
        }
        if title_page {
            if let Some(page) = walker.sections.first_mut() {
                page.title = Some(book.title.clone());
            }
        }
        walker.walk(book.id, start_depth);
    } else if let Some(draft) = input.tree.draft_root() {
        walker.walk(draft.id, start_depth);
    }
    let mut sections = walker.sections;
    if title_page {
        if let Some(next) = sections.get_mut(1) {
            next.after_title_page = true;
        }
    }
    sections
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
                    (Some(profile.chapters.heading(&profile.chapter_word, self.chapters, &node.title)), None)
                }
                NodeKind::Part => {
                    self.parts += 1;
                    (Some(profile.parts.heading(&profile.part_word, self.parts, &node.title)), None)
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
            let document = node.document_id.and_then(|id| self.input.documents.iter().find(|doc| doc.id == id));
            self.sections.push(CompiledSection {
                heading_level: depth.clamp(1, 6),
                title,
                document,
                kind: Some(node.kind),
                after_title_page: false,
                break_before,
            });
            self.walk(node.id, depth.saturating_add(1).min(6));
        }
    }
}
