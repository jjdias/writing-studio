//! Where each node of the mind map goes. The book is in the middle; its
//! parts, chapters, and scenes branch out to the right and the left, in
//! binder order: the first half of its top-level items to the right, top to
//! bottom, the rest to the left. Each branch takes the height of what it
//! holds, so no two nodes overlap. A folded node's branch is not laid out.
//!
//! Ideas sit in a row under the book. A node the writer dragged somewhere
//! keeps that place,
//! and a binder node takes its branch along; everything else stays laid out
//! around it.
//!
//! Plain arithmetic on the binder, so it is tested here; the map pane only
//! draws what this returns.

use writing_studio_core::Id;
use writing_studio_project::{BinderTree, NodeKind};

/// A node's size and the room between nodes, in map units (pixels at
/// a zoom of 1).
pub const NODE_WIDTH: f64 = 184.0;
pub const NODE_HEIGHT: f64 = 46.0;
const ROW_GAP: f64 = 12.0;
/// From one depth's centers to the next.
const COLUMN: f64 = 236.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Middle,
    Right,
    Left,
}

/// What a node of the map is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapKind {
    /// A book, part, chapter, folder, or scene.
    Binder,
    /// An idea, which is not in the book.
    Idea,
}

/// A tree to lay out: what is directly under a node, and what each node is.
pub struct Tree<'a> {
    pub under: &'a dyn Fn(Id) -> Vec<Id>,
    pub kind_of: &'a dyn Fn(Id) -> MapKind,
}

/// A node besides the binder's, an idea, and the node it is drawn joined
/// to: what it hangs from.
#[derive(Clone, Debug, PartialEq)]
pub struct Extra {
    pub id: Id,
    pub kind: MapKind,
    pub joined: Vec<Id>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MapNode {
    pub id: Id,
    pub kind: MapKind,
    pub parent: Option<Id>,
    pub depth: usize,
    pub side: Side,
    /// The center of the node.
    pub x: f64,
    pub y: f64,
    pub has_children: bool,
    pub folded: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MapLayout {
    /// The middle first, then each branch in binder order, parents before
    /// their children.
    pub nodes: Vec<MapNode>,
    pub min_x: f64,
    pub max_x: f64,
    pub min_y: f64,
    pub max_y: f64,
}

impl MapLayout {
    pub fn get(&self, id: Id) -> Option<&MapNode> {
        self.nodes.iter().find(|node| node.id == id)
    }

    pub fn width(&self) -> f64 {
        self.max_x - self.min_x
    }

    pub fn height(&self) -> f64 {
        self.max_y - self.min_y
    }
}

/// What the map shows under an item: its books, parts, chapters, folders,
/// and scenes, outside the trash, in binder order.
pub fn branches(binder: &BinderTree, id: Id) -> Vec<Id> {
    binder
        .children(Some(id))
        .into_iter()
        .filter(|node| {
            matches!(node.kind, NodeKind::Book | NodeKind::Part | NodeKind::Chapter | NodeKind::Folder | NodeKind::Scene)
        })
        .filter(|node| !binder.is_in_trash(node.id))
        .map(|node| node.id)
        .collect()
}

/// The map of everything under `root` in the binder, with the nodes in
/// `folded` closed.
pub fn layout(binder: &BinderTree, root: Id, folded: &[Id]) -> MapLayout {
    if binder.get(root).is_none() {
        return MapLayout::default();
    }
    let under = |id: Id| branches(binder, id);
    let kind_of = |_: Id| MapKind::Binder;
    layout_tree(&Tree { under: &under, kind_of: &kind_of }, root, folded)
}

/// Any tree laid out as the book is: `root` in the middle, the first half
/// of what is under it to the right, the rest to the left.
pub fn layout_tree(tree: &Tree<'_>, root: Id, folded: &[Id]) -> MapLayout {
    let mut map = MapLayout::default();
    let top = (tree.under)(root);
    let right_count = top.len().div_ceil(2);
    let (right, left) = top.split_at(right_count);
    map.nodes.push(MapNode {
        id: root,
        kind: (tree.kind_of)(root),
        parent: None,
        depth: 0,
        side: Side::Middle,
        x: 0.0,
        y: 0.0,
        has_children: !top.is_empty(),
        folded: false,
    });
    for (side, ids) in [(Side::Right, right), (Side::Left, left)] {
        let total = stack_height(tree, ids, folded, 1);
        let mut top_edge = -total / 2.0;
        for id in ids {
            let height = branch_height(tree, *id, folded, 1);
            place(tree, &mut map, *id, root, 1, side, top_edge, folded);
            top_edge += height + ROW_GAP;
        }
    }
    map.measure();
    map
}

/// Nodes the writer placed by hand go where they were put (`placed`, by
/// id, as centers), each taking its branch along, unless a node in it was
/// placed too.
pub fn place_by_hand(map: &mut MapLayout, placed: &[(Id, f64, f64)]) {
    let placed_ids: Vec<Id> = placed.iter().map(|(id, _, _)| *id).collect();
    for (id, x, y) in placed {
        let Some(node) = map.get(*id) else {
            continue;
        };
        let (dx, dy) = (x - node.x, y - node.y);
        let mut moving = vec![*id];
        let mut at = 0;
        while at < moving.len() && moving.len() < 100_000 {
            let parent = moving[at];
            moving.extend(
                map.nodes
                    .iter()
                    .filter(|child| child.parent == Some(parent) && !placed_ids.contains(&child.id))
                    .map(|child| child.id),
            );
            at += 1;
        }
        for node in map.nodes.iter_mut().filter(|node| moving.contains(&node.id)) {
            node.x += dx;
            node.y += dy;
        }
    }
    map.measure();
}

/// The binder's map with ideas added, and the nodes the writer
/// placed by hand where they were put (`placed`, by id, as centers).
pub fn layout_all(binder: &BinderTree, root: Id, folded: &[Id], extras: &[Extra], placed: &[(Id, f64, f64)]) -> MapLayout {
    let mut map = layout(binder, root, folded);
    if map.nodes.is_empty() {
        return map;
    }
    // Ideas in a row under the tree, centered.
    let row: Vec<&Extra> = extras.iter().filter(|extra| extra.kind == MapKind::Idea).collect();
    let step = NODE_WIDTH + ROW_GAP * 2.0;
    let first = -(row.len().saturating_sub(1) as f64) * step / 2.0;
    let y = map.max_y + NODE_HEIGHT * 1.5;
    for (at, extra) in row.into_iter().enumerate() {
        map.nodes.push(MapNode {
            id: extra.id,
            kind: MapKind::Idea,
            parent: None,
            depth: 0,
            side: Side::Middle,
            x: first + at as f64 * step,
            y,
            has_children: false,
            folded: false,
        });
    }
    place_by_hand(&mut map, placed);
    map
}

impl MapLayout {
    fn measure(&mut self) {
        let half = (NODE_WIDTH / 2.0, NODE_HEIGHT / 2.0);
        self.min_x = self.nodes.iter().map(|node| node.x - half.0).fold(f64::INFINITY, f64::min);
        self.max_x = self.nodes.iter().map(|node| node.x + half.0).fold(f64::NEG_INFINITY, f64::max);
        self.min_y = self.nodes.iter().map(|node| node.y - half.1).fold(f64::INFINITY, f64::min);
        self.max_y = self.nodes.iter().map(|node| node.y + half.1).fold(f64::NEG_INFINITY, f64::max);
    }
}

fn is_folded(folded: &[Id], id: Id) -> bool {
    folded.contains(&id)
}

/// The height a branch takes: its node, or what it holds stacked, whichever
/// is taller.
fn branch_height(tree: &Tree<'_>, id: Id, folded: &[Id], depth: usize) -> f64 {
    let held = (tree.under)(id);
    if held.is_empty() || is_folded(folded, id) || depth > 64 {
        return NODE_HEIGHT;
    }
    stack_height(tree, &held, folded, depth + 1).max(NODE_HEIGHT)
}

fn stack_height(tree: &Tree<'_>, ids: &[Id], folded: &[Id], depth: usize) -> f64 {
    let heights: f64 = ids.iter().map(|id| branch_height(tree, *id, folded, depth)).sum();
    heights + ROW_GAP * ids.len().saturating_sub(1) as f64
}

#[allow(clippy::too_many_arguments)]
fn place(tree: &Tree<'_>, map: &mut MapLayout, id: Id, parent: Id, depth: usize, side: Side, top_edge: f64, folded: &[Id]) {
    let held = (tree.under)(id);
    let closed = is_folded(folded, id);
    let height = branch_height(tree, id, folded, depth);
    let sign = if side == Side::Left { -1.0 } else { 1.0 };
    map.nodes.push(MapNode {
        id,
        kind: (tree.kind_of)(id),
        parent: Some(parent),
        depth,
        side,
        x: sign * COLUMN * depth as f64,
        y: top_edge + height / 2.0,
        has_children: !held.is_empty(),
        folded: closed && !held.is_empty(),
    });
    if closed || depth > 64 {
        return;
    }
    let mut child_top = top_edge + (height - stack_height(tree, &held, folded, depth + 1)) / 2.0;
    for child in held {
        let child_height = branch_height(tree, child, folded, depth + 1);
        place(tree, map, child, id, depth + 1, side, child_top, folded);
        child_top += child_height + ROW_GAP;
    }
}

/// A key that moves the chosen node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Up,
    Down,
    /// Toward the screen's left: out of a right-side branch, deeper into a
    /// left-side one.
    Left,
    Right,
}

/// The node an arrow key moves to from `from`: up and down among the nodes
/// with the same parent on the same side; left and right toward the middle
/// or away from it, as the branch runs.
pub fn step(map: &MapLayout, from: Id, key: Step) -> Option<Id> {
    let node = map.get(from)?;
    let outward = match (node.side, key) {
        (Side::Right, Step::Right) | (Side::Left, Step::Left) => Some(true),
        (Side::Right, Step::Left) | (Side::Left, Step::Right) => Some(false),
        (Side::Middle, Step::Right) => {
            return map.nodes.iter().find(|child| child.parent == Some(from) && child.side == Side::Right).map(|n| n.id);
        }
        (Side::Middle, Step::Left) => {
            return map.nodes.iter().find(|child| child.parent == Some(from) && child.side == Side::Left).map(|n| n.id);
        }
        _ => None,
    };
    match outward {
        Some(true) => map.nodes.iter().find(|child| child.parent == Some(from)).map(|child| child.id),
        Some(false) => node.parent,
        None => {
            let mut siblings: Vec<&MapNode> =
                map.nodes.iter().filter(|other| other.parent == node.parent && other.side == node.side).collect();
            siblings.sort_by(|one, other| one.y.total_cmp(&other.y));
            let at = siblings.iter().position(|other| other.id == from)?;
            let next = if key == Step::Up { at.checked_sub(1)? } else { at + 1 };
            siblings.get(next).map(|other| other.id)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use writing_studio_project::BinderNode;

    /// A book with three chapters; the first holds two scenes, the second
    /// one, the third none. A scene in the trash is left off.
    fn book() -> (BinderTree, Id, [Id; 3], [Id; 3]) {
        let draft = BinderNode::new(NodeKind::DraftRoot, "Manuscript", None, 0);
        let trash = BinderNode::new(NodeKind::TrashRoot, "Trash", None, 2);
        let book = BinderNode::new(NodeKind::Book, "Tide", Some(draft.id), 0);
        let one = BinderNode::new(NodeKind::Chapter, "One", Some(book.id), 0);
        let two = BinderNode::new(NodeKind::Chapter, "Two", Some(book.id), 1);
        let three = BinderNode::new(NodeKind::Chapter, "Three", Some(book.id), 2);
        let a = BinderNode::new(NodeKind::Scene, "A", Some(one.id), 0);
        let b = BinderNode::new(NodeKind::Scene, "B", Some(one.id), 1);
        let c = BinderNode::new(NodeKind::Scene, "C", Some(two.id), 0);
        let gone = BinderNode::new(NodeKind::Scene, "Gone", Some(trash.id), 0);
        let ids = (book.id, [one.id, two.id, three.id], [a.id, b.id, c.id]);
        let tree = BinderTree::new(vec![draft, trash, book, one, two, three, a, b, c, gone]);
        (tree, ids.0, ids.1, ids.2)
    }

    fn overlaps(one: &MapNode, other: &MapNode) -> bool {
        (one.x - other.x).abs() < NODE_WIDTH && (one.y - other.y).abs() < NODE_HEIGHT + ROW_GAP / 2.0
    }

    #[test]
    fn every_node_is_placed_in_binder_order_and_none_overlap() {
        let (tree, book, chapters, scenes) = book();
        let map = layout(&tree, book, &[]);
        let ids: Vec<Id> = map.nodes.iter().map(|node| node.id).collect();
        assert_eq!(ids, [book, chapters[0], scenes[0], scenes[1], chapters[1], scenes[2], chapters[2]]);
        let side = |id: Id| map.get(id).unwrap().side;
        assert_eq!((side(chapters[0]), side(chapters[1]), side(chapters[2])), (Side::Right, Side::Right, Side::Left));
        assert!(map.get(scenes[0]).unwrap().x > map.get(chapters[0]).unwrap().x, "a branch runs outward");
        assert!(map.get(chapters[2]).unwrap().x < 0.0, "the left side runs left");
        assert!(map.get(scenes[0]).unwrap().y < map.get(scenes[1]).unwrap().y, "siblings top to bottom");
        assert!(map.get(scenes[1]).unwrap().y < map.get(scenes[2]).unwrap().y, "and branch after branch");
        for (at, one) in map.nodes.iter().enumerate() {
            for other in &map.nodes[at + 1..] {
                assert!(!overlaps(one, other), "{:?} and {:?} overlap", one.id, other.id);
            }
        }
        assert!(map.width() > 0.0 && map.height() >= NODE_HEIGHT);
    }

    #[test]
    fn a_folded_branch_keeps_its_node_and_hides_what_it_holds() {
        let (tree, book, chapters, scenes) = book();
        let map = layout(&tree, book, &[chapters[0]]);
        let one = map.get(chapters[0]).unwrap();
        assert!(one.folded && one.has_children);
        assert!(map.get(scenes[0]).is_none() && map.get(scenes[2]).is_some());
        assert!(!map.get(chapters[2]).unwrap().has_children);
        assert!(layout(&tree, Id::new(), &[]).nodes.is_empty());
    }

    #[test]
    fn ideas_go_under_the_tree_and_placed_nodes_where_put() {
        let (tree, book, chapters, scenes) = book();
        let idea = Id::new();
        let extras = [Extra { id: idea, kind: MapKind::Idea, joined: vec![scenes[0]] }];
        let plain = layout(&tree, book, &[]);
        let map = layout_all(&tree, book, &[], &extras, &[]);
        assert!(map.get(idea).unwrap().y > plain.max_y, "an idea under the tree");
        assert_eq!(map.get(idea).unwrap().kind, MapKind::Idea);

        // Chapter One placed by hand takes its scenes along; its scene B,
        // placed too, stays where it was put.
        let one = map.get(chapters[0]).unwrap().clone();
        let a = map.get(scenes[0]).unwrap().clone();
        let placed = [(chapters[0], one.x + 100.0, one.y - 50.0), (scenes[1], 900.0, 900.0), (idea, -5.0, 7.0)];
        let moved = layout_all(&tree, book, &[], &extras, &placed);
        assert_eq!((moved.get(chapters[0]).unwrap().x, moved.get(chapters[0]).unwrap().y), (one.x + 100.0, one.y - 50.0));
        assert_eq!((moved.get(scenes[0]).unwrap().x, moved.get(scenes[0]).unwrap().y), (a.x + 100.0, a.y - 50.0));
        assert_eq!((moved.get(scenes[1]).unwrap().x, moved.get(scenes[1]).unwrap().y), (900.0, 900.0));
        assert_eq!((moved.get(idea).unwrap().x, moved.get(idea).unwrap().y), (-5.0, 7.0));
        assert_eq!(moved.get(chapters[1]).unwrap().x, map.get(chapters[1]).unwrap().x, "the others stay");
        assert!(moved.max_x >= 900.0 + NODE_WIDTH / 2.0, "the bounds take it in");
    }

    /// Any tree lays out as the book does: a middle, two branches, and one
    /// nested two deep.
    #[test]
    fn any_tree_lays_out_as_the_book_does() {
        let (middle, places, people, country, city) = (Id::new(), Id::new(), Id::new(), Id::new(), Id::new());
        let under = move |id: Id| -> Vec<Id> {
            if id == middle {
                vec![places, people]
            } else if id == places {
                vec![country]
            } else if id == country {
                vec![city]
            } else {
                Vec::new()
            }
        };
        let kind_of = move |id: Id| if id == city { MapKind::Idea } else { MapKind::Binder };
        let map = layout_tree(&Tree { under: &under, kind_of: &kind_of }, middle, &[]);
        let ids: Vec<Id> = map.nodes.iter().map(|node| node.id).collect();
        assert_eq!(ids, [middle, places, country, city, people]);
        assert_eq!((map.get(city).unwrap().depth, map.get(city).unwrap().kind), (3, MapKind::Idea));
        assert_eq!(map.get(people).unwrap().side, Side::Left);
        let mut moved = map.clone();
        place_by_hand(&mut moved, &[(country, 0.0, 400.0)]);
        assert_eq!(moved.get(city).unwrap().y - moved.get(country).unwrap().y, map.get(city).unwrap().y - map.get(country).unwrap().y, "its branch goes along");
    }

    #[test]
    fn arrow_keys_move_along_a_branch_and_between_siblings() {
        let (tree, book, chapters, scenes) = book();
        let map = layout(&tree, book, &[]);
        assert_eq!(step(&map, book, Step::Right), Some(chapters[0]));
        assert_eq!(step(&map, book, Step::Left), Some(chapters[2]));
        assert_eq!(step(&map, chapters[0], Step::Right), Some(scenes[0]), "outward on the right");
        assert_eq!(step(&map, scenes[0], Step::Left), Some(chapters[0]), "back toward the middle");
        assert_eq!(step(&map, scenes[0], Step::Down), Some(scenes[1]));
        assert_eq!(step(&map, scenes[1], Step::Down), None, "the last of its siblings");
        assert_eq!(step(&map, chapters[0], Step::Down), Some(chapters[1]));
        assert_eq!(step(&map, chapters[2], Step::Right), Some(book), "on the left, right goes inward");
        assert_eq!(step(&map, book, Step::Up), None);
    }
}
