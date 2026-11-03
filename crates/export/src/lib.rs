//! Export must never depend on the editor widget — only on documents and binder order.

use writing_studio_document::Document;
use writing_studio_project::BinderTree;

pub fn manuscript_plain_text(tree: &BinderTree, docs: &[Document]) -> String {
    let mut chunks = Vec::new();
    collect(&mut chunks, tree, None, docs);
    chunks.join("\n\n")
}

fn collect(out: &mut Vec<String>, tree: &BinderTree, parent: Option<writing_studio_core::Id>, docs: &[Document]) {
    for node in tree.children(parent) {
        if let Some(doc_id) = node.document_id {
            if let Some(doc) = docs.iter().find(|doc| doc.id == doc_id) {
                out.push(format!("# {}\n\n{}", node.title, doc.to_plain_text()));
            }
        }
        collect(out, tree, Some(node.id), docs);
    }
}
