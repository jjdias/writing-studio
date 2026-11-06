//! Corkboard cards derived from manuscript scenes, in binder order.

use writing_studio_core::Id;
use writing_studio_project::{BinderTree, NodeKind, BINDER_COLORS};

#[derive(Clone, Debug, PartialEq)]
pub struct CorkCard {
    pub node_id: Id,
    pub container: String,
    pub title: String,
    pub synopsis: String,
    pub status: String,
    pub label: String,
    pub color: String,
    pub word_count: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CorkGroup {
    #[default]
    Manuscript,
    Chapter,
    Label,
    Color,
    Status,
}

impl CorkGroup {
    pub const ALL: [CorkGroup; 5] = [
        CorkGroup::Manuscript,
        CorkGroup::Chapter,
        CorkGroup::Label,
        CorkGroup::Color,
        CorkGroup::Status,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Manuscript => "Order",
            Self::Chapter => "Chapter",
            Self::Label => "Label",
            Self::Color => "Color",
            Self::Status => "Status",
        }
    }
}

#[derive(Clone, Debug)]
pub struct CorkSection {
    pub title: String,
    pub cards: Vec<CorkCard>,
}

pub fn cards_from_binder<F>(tree: &BinderTree, mut words_for: F) -> Vec<CorkCard>
where
    F: FnMut(Id) -> u32,
{
    let mut cards = Vec::new();
    if let Some(draft) = tree.draft_root() {
        walk(tree, draft.id, "Manuscript", &mut words_for, &mut cards);
    }
    cards
}

fn walk<F>(tree: &BinderTree, parent: Id, container: &str, words_for: &mut F, cards: &mut Vec<CorkCard>)
where
    F: FnMut(Id) -> u32,
{
    for child in tree.children(Some(parent)) {
        if tree.is_in_trash(child.id) {
            continue;
        }
        match child.kind {
            NodeKind::Scene => cards.push(CorkCard {
                node_id: child.id,
                container: container.to_string(),
                title: child.title.clone(),
                synopsis: child.synopsis.clone(),
                status: child.status.clone(),
                label: child.label.clone(),
                color: child.color.clone(),
                word_count: child.document_id.map(&mut *words_for).unwrap_or(0),
            }),
            NodeKind::Chapter | NodeKind::Folder | NodeKind::Book | NodeKind::Part => {
                walk(tree, child.id, &child.title, words_for, cards);
            }
            _ => {}
        }
    }
}

pub fn card_matches(card: &CorkCard, query: &str) -> bool {
    let query = query.trim();
    if query.is_empty() {
        return true;
    }
    let query = query.to_lowercase();
    [
        card.title.as_str(),
        card.synopsis.as_str(),
        card.label.as_str(),
        card.status.as_str(),
        card.container.as_str(),
    ]
    .iter()
    .any(|field| field.to_lowercase().contains(&query))
}

pub fn group_cards(cards: &[CorkCard], group: CorkGroup) -> Vec<CorkSection> {
    if cards.is_empty() {
        return Vec::new();
    }
    if group == CorkGroup::Manuscript {
        return vec![CorkSection {
            title: "Manuscript order".into(),
            cards: cards.to_vec(),
        }];
    }
    let mut sections: Vec<CorkSection> = Vec::new();
    for card in cards {
        let title = group_title(card, group);
        if let Some(section) = sections.iter_mut().find(|section| section.title == title) {
            section.cards.push(card.clone());
        } else {
            sections.push(CorkSection {
                title,
                cards: vec![card.clone()],
            });
        }
    }
    sections
}

fn group_title(card: &CorkCard, group: CorkGroup) -> String {
    match group {
        CorkGroup::Manuscript => "Manuscript order".into(),
        CorkGroup::Chapter => named(&card.container, "Untitled"),
        CorkGroup::Label => named(&card.label, "No label"),
        CorkGroup::Status => named(&card.status, "No status"),
        CorkGroup::Color => {
            if card.color.trim().is_empty() {
                "No color".into()
            } else {
                BINDER_COLORS
                    .iter()
                    .find(|swatch| swatch.id == card.color.trim())
                    .map(|swatch| swatch.name.to_string())
                    .unwrap_or_else(|| card.color.clone())
            }
        }
    }
}

fn named(value: &str, empty: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        empty.to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use writing_studio_project::BinderNode;

    fn sample() -> BinderTree {
        let draft = BinderNode::new(NodeKind::DraftRoot, "Manuscript", None, 0);
        let book = BinderNode::new(NodeKind::Book, "Book", Some(draft.id), 0);
        let chapter_a = BinderNode::new(NodeKind::Chapter, "Chapter 1", Some(book.id), 0);
        let mut zebra = BinderNode::new(NodeKind::Scene, "Zebra", Some(chapter_a.id), 0);
        zebra.synopsis = "harbor at night".into();
        zebra.label = "subplot".into();
        zebra.color = "gold".into();
        zebra.status = "draft".into();
        let mut alpha = BinderNode::new(NodeKind::Scene, "Alpha", Some(chapter_a.id), 1);
        alpha.label = String::new();
        alpha.color = String::new();
        alpha.status = "revise".into();
        let chapter_b = BinderNode::new(NodeKind::Chapter, "Chapter 2", Some(book.id), 1);
        let mut later = BinderNode::new(NodeKind::Scene, "Later", Some(chapter_b.id), 0);
        later.label = "subplot".into();
        later.color = "blue".into();
        later.status = "draft".into();
        BinderTree::new(vec![draft, book, chapter_a, zebra, alpha, chapter_b, later])
    }

    #[test]
    fn cards_follow_binder_order_not_title() {
        let cards = cards_from_binder(&sample(), |_| 3);
        let titles: Vec<_> = cards.iter().map(|card| card.title.as_str()).collect();
        assert_eq!(titles, vec!["Zebra", "Alpha", "Later"]);
        assert_eq!(cards[0].container, "Chapter 1");
        assert_eq!(cards[0].word_count, 3);
        assert_eq!(cards[2].container, "Chapter 2");
    }

    #[test]
    fn filter_matches_synopsis_and_label() {
        let cards = cards_from_binder(&sample(), |_| 0);
        let harbor: Vec<_> = cards
            .iter()
            .filter(|card| card_matches(card, "Harbor"))
            .map(|card| card.title.as_str())
            .collect();
        assert_eq!(harbor, vec!["Zebra"]);
        let subplot: Vec<_> = cards
            .iter()
            .filter(|card| card_matches(card, "subplot"))
            .map(|card| card.title.as_str())
            .collect();
        assert_eq!(subplot, vec!["Zebra", "Later"]);
    }

    #[test]
    fn groups_keep_manuscript_order_inside_each_bucket() {
        let cards = cards_from_binder(&sample(), |_| 0);
        let by_label = group_cards(&cards, CorkGroup::Label);
        assert_eq!(by_label[0].title, "subplot");
        assert_eq!(
            by_label[0]
                .cards
                .iter()
                .map(|card| card.title.as_str())
                .collect::<Vec<_>>(),
            vec!["Zebra", "Later"]
        );
        assert_eq!(by_label[1].title, "No label");
        let by_color = group_cards(&cards, CorkGroup::Color);
        assert_eq!(by_color[0].title, "Gold");
        assert_eq!(by_color[1].title, "No color");
        assert_eq!(by_color[2].title, "Blue");
        let by_chapter = group_cards(&cards, CorkGroup::Chapter);
        assert_eq!(by_chapter.len(), 2);
        assert_eq!(by_chapter[1].cards.len(), 1);
    }
}
