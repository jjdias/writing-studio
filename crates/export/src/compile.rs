use writing_studio_core::Id;
use writing_studio_document::Document;
use writing_studio_project::{BinderTree, NodeKind};

pub struct CompileInput<'a> {
    pub title: &'a str,
    pub include_title_page: bool,
    pub tree: &'a BinderTree,
    pub documents: &'a [Document],
}

pub struct CompiledSection<'a> {
    pub heading_level: u8,
    pub title: String,
    pub document: Option<&'a Document>,
}

pub fn collect_sections<'a>(input: &'a CompileInput<'a>) -> Vec<CompiledSection<'a>> {
    let mut sections = Vec::new();
    if input.include_title_page {
        sections.push(CompiledSection {
            heading_level: 1,
            title: input.title.to_string(),
            document: None,
        });
    }
    let start_depth = if input.include_title_page { 2 } else { 1 };
    if let Some(draft) = input.tree.draft_root() {
        walk(input, draft.id, start_depth, &mut sections);
    }
    sections
}

fn walk<'a>(
    input: &'a CompileInput<'a>,
    parent: Id,
    depth: u8,
    sections: &mut Vec<CompiledSection<'a>>,
) {
    for node in input.tree.children(Some(parent)) {
        if node.kind.is_root() || input.tree.is_in_trash(node.id) {
            continue;
        }
        if matches!(node.kind, NodeKind::Note | NodeKind::Folder) {
            walk(input, node.id, depth, sections);
            continue;
        }
        let document = node
            .document_id
            .and_then(|id| input.documents.iter().find(|doc| doc.id == id));
        sections.push(CompiledSection {
            heading_level: depth.clamp(1, 6),
            title: node.title.clone(),
            document,
        });
        walk(input, node.id, depth.saturating_add(1).min(6), sections);
    }
}
