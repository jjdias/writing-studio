use writing_studio_core::Id;
use writing_studio_project::{BinderTree, NodeKind};

#[derive(Clone, Debug)]
pub struct CorkCard {
    pub node_id: Id,
    pub title: String,
    pub synopsis: String,
    pub status: String,
    pub color: String,
    pub word_count: u32,
}

pub fn cards_from_binder<F>(tree: &BinderTree, mut words_for: F) -> Vec<CorkCard>
where
    F: FnMut(Id) -> u32,
{
    let mut cards: Vec<CorkCard> = tree
        .nodes
        .iter()
        .filter(|node| {
            node.kind == NodeKind::Scene && node.deleted_at.is_none() && !tree.is_in_trash(node.id)
        })
        .map(|node| CorkCard {
            node_id: node.id,
            title: node.title.clone(),
            synopsis: node.synopsis.clone(),
            status: node.status.clone(),
            color: node.color.clone(),
            word_count: node.document_id.map(&mut words_for).unwrap_or(0),
        })
        .collect();
    cards.sort_by(|a, b| a.title.cmp(&b.title));
    cards
}
