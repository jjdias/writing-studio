//! Merging two copies of a project from the last version they shared.
//!
//! It works on three [`ProjectContent`] values and touches no files. No
//! merge drops text without showing it: where both copies changed the same
//! paragraphs differently, the scene keeps both, and the conflict is settled
//! with the track-changes panel.

use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use serde::Serialize;
use serde_json::Value;
use writing_studio_core::{Id, Result};
use writing_studio_diagnostics::SceneBeats;
use writing_studio_document::{Block, Document};
use writing_studio_format::blocks_to_markdown;
use writing_studio_project::{BinderNode, BinderTree};
use writing_studio_storage::{repair_binder, ProjectContent};

/// Something both copies changed differently. The merge kept this
/// machine's version, except for paragraphs, where it kept both.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Conflict {
    pub item: Id,
    pub what: String,
}

#[derive(Clone, Debug)]
pub struct Merged {
    pub content: ProjectContent,
    pub conflicts: Vec<Conflict>,
    /// For each scene with paragraphs in conflict: the scene with only this
    /// machine's paragraphs. Saved as the scene's track-changes baseline, it
    /// makes the other machine's paragraphs show as changes to accept or
    /// reject.
    pub baselines: Vec<Document>,
}

/// What a record holds, without what differs between any two reads of the
/// same files: times, and positions (order is merged on its own).
fn key<T: Serialize>(item: &T) -> Value {
    let mut value = serde_json::to_value(item).unwrap_or(Value::Null);
    if let Some(map) = value.as_object_mut() {
        for field in ["created_at", "updated_at", "sort_index"] {
            map.remove(field);
        }
    }
    value
}

/// The three-way choice for one thing that exists in some of the copies.
/// `None` means it is deleted. An edit on one side wins over a deletion on
/// the other.
fn choose<T: Clone>(
    base: Option<&T>,
    ours: Option<&T>,
    theirs: Option<&T>,
    same: impl Fn(&T, &T) -> bool,
) -> (Option<T>, bool) {
    match (base, ours, theirs) {
        (_, Some(o), Some(t)) => match base {
            Some(b) if same(o, b) => (Some(t.clone()), false),
            Some(b) if same(t, b) => (Some(o.clone()), false),
            _ if same(o, t) => (Some(o.clone()), false),
            _ => (Some(o.clone()), true),
        },
        (None, Some(o), None) => (Some(o.clone()), false),
        (None, None, Some(t)) => (Some(t.clone()), false),
        (Some(b), Some(o), None) => {
            if same(o, b) { (None, false) } else { (Some(o.clone()), true) }
        }
        (Some(b), None, Some(t)) => {
            if same(t, b) { (None, false) } else { (Some(t.clone()), true) }
        }
        _ => (None, false),
    }
}

/// Merge lists of records with ids. The order is this machine's, with
/// records added on the other placed after the record they followed there.
fn records<T: Clone + Serialize>(
    base: &[T],
    ours: &[T],
    theirs: &[T],
    id: impl Fn(&T) -> Id,
    label: &str,
    conflicts: &mut Vec<Conflict>,
) -> Vec<T> {
    let find = |list: &'_ [T], at: Id| list.iter().find(|item| id(item) == at).cloned();
    let ids: Vec<Id> = in_merged_order(
        &ours.iter().map(&id).collect::<Vec<_>>(),
        &theirs.iter().map(&id).collect::<Vec<_>>(),
    );
    ids.into_iter()
        .filter_map(|at| {
            let (b, o, t) = (find(base, at), find(ours, at), find(theirs, at));
            let (chosen, conflict) = choose(b.as_ref(), o.as_ref(), t.as_ref(), |x, y| key(x) == key(y));
            if conflict {
                conflicts.push(Conflict {
                    item: at,
                    what: format!("a {label} was changed on both machines; this machine's version was kept"),
                });
            }
            chosen
        })
        .collect()
}

/// Ours in order, then each id only theirs has: after the id it follows in
/// theirs, or else before the id it comes before there, or else last.
fn in_merged_order<K: Copy + Eq + Hash>(ours: &[K], theirs: &[K]) -> Vec<K> {
    let mut out: Vec<K> = ours.to_vec();
    let mut seen: HashSet<K> = ours.iter().copied().collect();
    for (index, item) in theirs.iter().enumerate() {
        if !seen.insert(*item) {
            continue;
        }
        let find = |other: &K| out.iter().position(|o| o == other);
        let at = theirs[..index]
            .iter()
            .rev()
            .find_map(find)
            .map(|before| before + 1)
            .or_else(|| theirs[index + 1..].iter().find_map(find))
            .unwrap_or(out.len());
        out.insert(at, *item);
    }
    out
}

/// Merge a list of values with no ids of their own, by what they hold: a
/// value stays if both copies have it, or if the copy that has it added it.
fn values<T: Clone + Serialize>(base: &[T], ours: &[T], theirs: &[T]) -> Vec<T> {
    // Their ids are made afresh on every read, so they are not compared.
    let held = |item: &T| {
        let mut value = key(item);
        if let Some(map) = value.as_object_mut() {
            map.remove("id");
        }
        value.to_string()
    };
    let keys = |list: &[T]| list.iter().map(held).collect::<HashSet<_>>();
    let (b, o, t) = (keys(base), keys(ours), keys(theirs));
    let mut out: Vec<T> = ours
        .iter()
        .filter(|item| t.contains(&held(item)) || !b.contains(&held(item)))
        .cloned()
        .collect();
    out.extend(theirs.iter().filter(|item| !o.contains(&held(item)) && !b.contains(&held(item))).cloned());
    out
}

/// Three-way choice for one field; `true` when both changed it differently.
fn field<T: Clone + PartialEq>(base: &T, ours: &T, theirs: &T) -> (T, bool) {
    if ours == base {
        (theirs.clone(), false)
    } else {
        (ours.clone(), theirs != base && theirs != ours)
    }
}

/// Merge two lists of things known by a key: what one side added is kept,
/// what one side took away is gone, and a thing both sides changed keeps
/// this machine's version. Order is this machine's, then the other's new ones.
fn merge_by_key<T: Clone + PartialEq, K: PartialEq>(
    base: &[T],
    ours: &[T],
    theirs: &[T],
    key: impl Fn(&T) -> K,
) -> Vec<T> {
    let find = |list: &[T], wanted: &K| list.iter().find(|item| key(item) == *wanted).cloned();
    let mut merged = Vec::new();
    for mine in ours {
        let k = key(mine);
        match (find(base, &k), find(theirs, &k)) {
            // The other machine took it away and this one did not change it.
            (Some(was), None) if was == *mine => {}
            (Some(was), Some(other)) => merged.push(field(&was, mine, &other).0),
            _ => merged.push(mine.clone()),
        }
    }
    for other in theirs {
        let k = key(other);
        if find(ours, &k).is_none() && find(base, &k).is_none() {
            merged.push(other.clone());
        }
    }
    merged
}

pub fn merge(base: &ProjectContent, ours: &ProjectContent, theirs: &ProjectContent) -> Result<Merged> {
    let mut conflicts = Vec::new();
    let mut baselines = Vec::new();
    let mut content = ours.clone();

    // The binder, with each item's text and beats.
    let nodes = |c: &ProjectContent| c.binder.nodes.iter().map(|n| (n.id, n.clone())).collect::<HashMap<_, _>>();
    let docs = |c: &ProjectContent| c.documents.iter().map(|d| (d.id, d.clone())).collect::<HashMap<_, _>>();
    let (bn, on, tn) = (nodes(base), nodes(ours), nodes(theirs));
    let (bd, od, td) = (docs(base), docs(ours), docs(theirs));
    let none = SceneBeats::default();
    let beats_of = |c: &ProjectContent, id: Id| c.beats.get(&id).cloned().unwrap_or_default();
    let unchanged = |b: &BinderNode, x: &BinderNode, xd: &HashMap<Id, Document>, xc: &ProjectContent| {
        same_node(b, x)
            && same_text(bd.get(&b.id), xd.get(&x.id))
            && beats_of(base, b.id) == beats_of(xc, x.id)
    };
    let mut merged_nodes = Vec::new();
    let mut documents = Vec::new();
    let mut beats = HashMap::new();
    let all: Vec<Id> = in_merged_order(
        &ours.binder.nodes.iter().map(|n| n.id).collect::<Vec<_>>(),
        &theirs.binder.nodes.iter().map(|n| n.id).collect::<Vec<_>>(),
    );
    for id in all {
        let (b, o, t) = (bn.get(&id), on.get(&id), tn.get(&id));
        let node = match (b, o, t) {
            (Some(b), Some(o), Some(t)) => merge_node(b, o, t, &mut conflicts),
            (Some(b), Some(o), None) if unchanged(b, o, &od, ours) => continue,
            (Some(b), None, Some(t)) if unchanged(b, t, &td, theirs) => continue,
            (_, Some(o), _) => o.clone(),
            (_, None, Some(t)) => t.clone(),
            _ => continue,
        };
        if b.is_some() && (o.is_none() || t.is_none()) {
            conflicts.push(Conflict {
                item: id,
                what: format!("“{}” was deleted on one machine and changed on the other; it was kept", node.title),
            });
        }
        if node.document_id.is_some() {
            let (text, kept_both) = merge_text(bd.get(&id), od.get(&id), td.get(&id));
            if let Some((merged, baseline)) = kept_both {
                conflicts.push(Conflict {
                    item: id,
                    what: format!(
                        "“{}” had the same paragraphs changed on both machines; both versions are in the scene, as tracked changes",
                        node.title
                    ),
                });
                baselines.push(baseline);
                documents.push(merged);
            } else if let Some(text) = text {
                documents.push(text);
            }
            let (b, o, t) = (
                base.beats.get(&id).unwrap_or(&none),
                ours.beats.get(&id).unwrap_or(&none),
                theirs.beats.get(&id).unwrap_or(&none),
            );
            let mut clash = false;
            let mut pick = |b: &String, o: &String, t: &String| {
                let (value, c) = field(b, o, t);
                clash |= c;
                value
            };
            let merged = SceneBeats {
                goal: pick(&b.goal, &o.goal, &t.goal),
                conflict: pick(&b.conflict, &o.conflict, &t.conflict),
                outcome: pick(&b.outcome, &o.outcome, &t.outcome),
            };
            if clash {
                conflicts.push(Conflict {
                    item: id,
                    what: format!("the goal, conflict, or outcome of “{}” was changed on both machines; this machine's was kept", node.title),
                });
            }
            if !merged.is_empty() {
                beats.insert(id, merged);
            }
        }
        merged_nodes.push(node);
    }
    order_siblings(&mut merged_nodes, ours, theirs);
    repair_binder(&mut merged_nodes)?;
    content.binder = BinderTree::new(merged_nodes);
    content.binder.keywords = merge_by_key(&base.binder.keywords, &ours.binder.keywords, &theirs.binder.keywords, |keyword| keyword.name.to_lowercase());
    content.binder.filters = merge_by_key(&base.binder.filters, &ours.binder.filters, &theirs.binder.filters, |filter| filter.id.to_string());
    content.documents = documents;
    content.beats = beats;

    // The bible and the plot grid.
    let c = &mut conflicts;
    content.cast.characters = records(&base.cast.characters, &ours.cast.characters, &theirs.cast.characters, |x| x.id, "character", c);
    content.cast.motivations = values(&base.cast.motivations, &ours.cast.motivations, &theirs.cast.motivations);
    content.cast.relationships = values(&base.cast.relationships, &ours.cast.relationships, &theirs.cast.relationships);
    content.world.locations = records(&base.world.locations, &ours.world.locations, &theirs.world.locations, |x| x.id, "place", c);
    content.world.cultures = records(&base.world.cultures, &ours.world.cultures, &theirs.world.cultures, |x| x.id, "culture", c);
    content.world.lore = records(&base.world.lore, &ours.world.lore, &theirs.world.lore, |x| x.id, "lore entry", c);
    content.world.events = records(&base.world.events, &ours.world.events, &theirs.world.events, |x| x.id, "timeline event", c);
    content.library.items = records(&base.library.items, &ours.library.items, &theirs.library.items, |x| x.id, "research note", c);
    content.library.files = records(&base.library.files, &ours.library.files, &theirs.library.files, |x| x.id, "research file", c);
    content.plot.lines = records(&base.plot.lines, &ours.plot.lines, &theirs.plot.lines, |x| x.id, "plotline", c);
    content.plot.points = records(&base.plot.points, &ours.plot.points, &theirs.plot.points, |x| x.id, "plot point", c);
    // The mind map's ideas, lines, and hand-placed nodes, each by its id.
    let (bm, om, tm) = (&base.mindmap, &ours.mindmap, &theirs.mindmap);
    content.mindmap.ideas = merge_by_key(&bm.ideas, &om.ideas, &tm.ideas, |idea| idea.id);
    content.mindmap.lines = merge_by_key(&bm.lines, &om.lines, &tm.lines, |line| line.id);
    content.mindmap.positions = merge_by_key(&bm.positions, &om.positions, &tm.positions, |kept| kept.id);
    renumber(&mut content);

    // The title and the settings that go with the book.
    let (title, clash) = field(&base.meta.title, &ours.meta.title, &theirs.meta.title);
    content.meta.title = title;
    if clash {
        conflicts.push(Conflict {
            item: ours.meta.id,
            what: "the project's title was changed on both machines; this machine's was kept".into(),
        });
    }
    content.meta.settings = merge_settings(&base.meta, &ours.meta, &theirs.meta)?;

    // Each item keeps the file it had; an item from the other machine keeps its name there.
    for (id, path) in &theirs.files {
        content.files.entry(*id).or_insert_with(|| path.clone());
    }
    for (id, extras) in &theirs.extras {
        content.extras.entry(*id).or_insert_with(|| extras.clone());
    }
    Ok(Merged {
        content,
        conflicts,
        baselines,
    })
}

fn same_node(a: &BinderNode, b: &BinderNode) -> bool {
    (a.parent_id, a.kind, &a.title, &a.synopsis, &a.status, &a.label, &a.color, &a.keywords, a.left_out, a.deleted_at, a.restore_parent_id)
        == (b.parent_id, b.kind, &b.title, &b.synopsis, &b.status, &b.label, &b.color, &b.keywords, b.left_out, b.deleted_at, b.restore_parent_id)
}

/// Each field of an item on its own: one side's change is taken; a field
/// both changed differently keeps this machine's.
fn merge_node(b: &BinderNode, o: &BinderNode, t: &BinderNode, conflicts: &mut Vec<Conflict>) -> BinderNode {
    let mut node = o.clone();
    let mut clash = false;
    macro_rules! take {
        ($($field:ident),*) => {$(
            let (value, c) = field(&b.$field, &o.$field, &t.$field);
            node.$field = value;
            clash |= c;
        )*};
    }
    take!(parent_id, title, synopsis, status, label, color, keywords, notes, footnotes, links, left_out, deleted_at, restore_parent_id);
    if clash {
        conflicts.push(Conflict {
            item: o.id,
            what: format!("“{}” had the same detail changed on both machines; this machine's was kept", node.title),
        });
    }
    node
}

/// One string for each paragraph, so that paragraphs can be compared.
fn paragraphs(document: &Document) -> Vec<String> {
    document.blocks.iter().map(|block| blocks_to_markdown(std::slice::from_ref(block))).collect()
}

fn same_text(a: Option<&Document>, b: Option<&Document>) -> bool {
    a.map(paragraphs) == b.map(paragraphs)
}

/// Merge a scene's text by paragraph. The second value is set when both
/// sides changed the same paragraphs: the scene with both versions, and the
/// scene with only this machine's.
#[allow(clippy::type_complexity)]
fn merge_text(
    base: Option<&Document>,
    ours: Option<&Document>,
    theirs: Option<&Document>,
) -> (Option<Document>, Option<(Document, Document)>) {
    let (Some(o), Some(t)) = (ours, theirs) else {
        return (ours.or(theirs).cloned(), None);
    };
    let (op, tp) = (paragraphs(o), paragraphs(t));
    if op == tp {
        return (Some(o.clone()), None);
    }
    let Some(b) = base else {
        // Added on both machines with one id: keep both versions.
        let both = with_blocks(o, [o.blocks.clone(), t.blocks.clone()].concat());
        return (None, Some((both, o.clone())));
    };
    let bp = paragraphs(b);
    if op == bp {
        return (Some(with_blocks(o, t.blocks.clone())), None);
    }
    if tp == bp {
        return (Some(o.clone()), None);
    }
    let (in_ours, in_theirs) = (matches(&bp, &op), matches(&bp, &tp));
    let (mut merged, mut baseline) = (Vec::new(), Vec::new());
    let mut clash = false;
    let (mut bi, mut oi, mut ti) = (0, 0, 0);
    loop {
        // The next base paragraph both sides kept as it was.
        let anchor = (bi..bp.len()).find(|&k| {
            matches!((in_ours[k], in_theirs[k]), (Some(x), Some(y)) if x >= oi && y >= ti)
        });
        let (bend, oend, tend) = match anchor {
            Some(k) => (k, in_ours[k].unwrap_or(oi), in_theirs[k].unwrap_or(ti)),
            None => (bp.len(), op.len(), tp.len()),
        };
        let (bs, os, ts) = (&bp[bi..bend], &op[oi..oend], &tp[ti..tend]);
        let (ob, tb) = (&o.blocks[oi..oend], &t.blocks[ti..tend]);
        if os == bs {
            merged.extend_from_slice(tb);
            baseline.extend_from_slice(tb);
        } else if ts == bs || os == ts {
            merged.extend_from_slice(ob);
            baseline.extend_from_slice(ob);
        } else {
            clash = true;
            merged.extend_from_slice(ob);
            merged.extend_from_slice(tb);
            baseline.extend_from_slice(ob);
        }
        let Some(k) = anchor else { break };
        merged.push(o.blocks[oend].clone());
        baseline.push(o.blocks[oend].clone());
        (bi, oi, ti) = (k + 1, oend + 1, tend + 1);
    }
    if clash {
        (None, Some((with_blocks(o, merged), with_blocks(o, baseline))))
    } else {
        (Some(with_blocks(o, merged)), None)
    }
}

fn with_blocks(document: &Document, blocks: Vec<Block>) -> Document {
    let mut out = document.clone();
    out.blocks = blocks;
    out
}

/// For each paragraph of `from`, where it is in `to`, by the longest run of
/// paragraphs the two have in common, in order.
fn matches(from: &[String], to: &[String]) -> Vec<Option<usize>> {
    let (n, m) = (from.len(), to.len());
    let mut table = vec![vec![0u32; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            table[i][j] = if from[i] == to[j] {
                table[i + 1][j + 1] + 1
            } else {
                table[i + 1][j].max(table[i][j + 1])
            };
        }
    }
    let mut out = vec![None; n];
    let (mut i, mut j) = (0, 0);
    while i < n && j < m {
        if from[i] == to[j] {
            out[i] = Some(j);
            i += 1;
            j += 1;
        } else if table[i + 1][j] >= table[i][j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    out
}

/// Siblings in this machine's order; an item placed by the other machine
/// goes after the sibling it follows there.
fn order_siblings(nodes: &mut [BinderNode], ours: &ProjectContent, theirs: &ProjectContent) {
    let children = |c: &ProjectContent, parent: Option<Id>| {
        let mut list: Vec<&BinderNode> = c.binder.nodes.iter().filter(|n| n.parent_id == parent).collect();
        list.sort_by_key(|node| node.sort_index);
        list.into_iter().map(|node| node.id).collect::<Vec<_>>()
    };
    let parents: HashSet<Option<Id>> = nodes.iter().map(|node| node.parent_id).collect();
    let mut position: HashMap<Id, i64> = HashMap::new();
    for parent in parents {
        let here: HashSet<Id> = nodes.iter().filter(|n| n.parent_id == parent).map(|n| n.id).collect();
        let order = in_merged_order(&children(ours, parent), &children(theirs, parent));
        let mut placed: Vec<Id> = order.into_iter().filter(|id| here.contains(id)).collect();
        // Moved here on neither side's list of this parent: last.
        let rest: Vec<Id> = nodes
            .iter()
            .filter(|n| n.parent_id == parent && !placed.contains(&n.id))
            .map(|n| n.id)
            .collect();
        placed.extend(rest);
        for (index, id) in placed.into_iter().enumerate() {
            position.insert(id, index as i64);
        }
    }
    for node in nodes.iter_mut() {
        node.sort_index = position.get(&node.id).copied().unwrap_or(i64::MAX);
    }
}

/// Positions follow the merged order of each list.
fn renumber(content: &mut ProjectContent) {
    macro_rules! number {
        ($($list:expr),*) => {$(
            for (index, item) in $list.iter_mut().enumerate() {
                item.sort_index = index as i64;
            }
        )*};
    }
    number!(
        content.cast.characters,
        content.cast.motivations,
        content.cast.relationships,
        content.world.locations,
        content.world.cultures,
        content.world.lore,
        content.world.events,
        content.library.items,
        content.plot.lines,
        content.plot.points
    );
}

/// Each setting on its own. The project's own words are the union of what
/// each side added, less what either removed.
fn merge_settings(
    base: &writing_studio_project::ProjectMeta,
    ours: &writing_studio_project::ProjectMeta,
    theirs: &writing_studio_project::ProjectMeta,
) -> Result<writing_studio_project::ProjectSettings> {
    let as_map = |meta: &writing_studio_project::ProjectMeta| {
        serde_json::to_value(&meta.settings)
            .ok()
            .and_then(|value| value.as_object().cloned())
            .unwrap_or_default()
    };
    let (b, o, t) = (as_map(base), as_map(ours), as_map(theirs));
    let mut out = o.clone();
    for (name, value) in &o {
        let (bv, tv) = (b.get(name).unwrap_or(&Value::Null), t.get(name).unwrap_or(&Value::Null));
        out.insert(name.clone(), field(bv, value, tv).0);
    }
    let words = |m: &writing_studio_project::ProjectMeta| m.settings.spelling.words.clone();
    let mut merged: writing_studio_project::ProjectSettings = serde_json::from_value(Value::Object(out))
        .map_err(|err| writing_studio_core::Error::message(err.to_string()))?;
    let mut list = values(&words(base), &words(ours), &words(theirs));
    list.sort();
    list.dedup();
    merged.spelling.words = list;
    Ok(merged)
}

#[cfg(test)]
mod tests {
    use super::*;
    use writing_studio_project::NodeKind;
    use writing_studio_storage::{read_project_files, ProjectStore};

    /// A project read from its files, as a sync reads one.
    fn base() -> ProjectContent {
        let dir = std::env::temp_dir().join(format!("ws-merge-{}", Id::new()));
        let store = ProjectStore::create(&dir, "Merge").unwrap();
        let scene = store.load_binder().unwrap().nodes.into_iter().find(|n| n.kind == NodeKind::Scene).unwrap();
        store.save_document(&Document::from_plain_text(scene.id, "One.\nTwo.\nThree.")).unwrap();
        drop(store);
        let content = read_project_files(&dir).unwrap();
        let _ = std::fs::remove_dir_all(dir);
        content
    }

    fn scene(content: &ProjectContent) -> Id {
        content.binder.nodes.iter().find(|n| n.kind == NodeKind::Scene).unwrap().id
    }

    fn text(content: &ProjectContent, id: Id) -> String {
        content.documents.iter().find(|d| d.id == id).map(Document::to_plain_text).unwrap_or_default()
    }

    fn set_text(content: &mut ProjectContent, id: Id, value: &str) {
        let document = content.documents.iter_mut().find(|d| d.id == id).unwrap();
        *document = Document::from_plain_text(id, value);
    }

    fn node(content: &mut ProjectContent, id: Id) -> &mut BinderNode {
        content.binder.nodes.iter_mut().find(|n| n.id == id).unwrap()
    }

    /// Each machine's ideas, lines, and placed nodes are kept, by id; one
    /// taken away on one machine is gone.
    #[test]
    fn the_mind_maps_of_two_machines_are_merged_by_id() {
        let mut base = base();
        let id = scene(&base);
        let kept = base.mindmap.add_idea(Some(id));
        let gone = base.mindmap.add_idea(None);
        let (mut ours, mut theirs) = (base.clone(), base.clone());
        let mine = ours.mindmap.add_idea(None);
        ours.mindmap.place(id, 10.0, 20.0);
        let line = theirs.mindmap.add_line(kept, id).unwrap();
        theirs.mindmap.remove_idea(gone);
        let merged = merge(&base, &ours, &theirs).unwrap().content.mindmap;
        let ideas: Vec<Id> = merged.ideas.iter().map(|idea| idea.id).collect();
        assert_eq!(ideas, [kept, mine]);
        assert_eq!(merged.lines.iter().map(|line| line.id).collect::<Vec<_>>(), [line]);
        assert_eq!(merged.position(id), Some((10.0, 20.0)));
    }

    #[test]
    fn edits_to_different_paragraphs_are_both_kept() {
        let base = base();
        let id = scene(&base);
        let (mut ours, mut theirs) = (base.clone(), base.clone());
        set_text(&mut ours, id, "One, changed here.\nTwo.\nThree.");
        set_text(&mut theirs, id, "One.\nTwo.\nThree, changed there.\nFour.");
        let merged = merge(&base, &ours, &theirs).unwrap();
        assert_eq!(text(&merged.content, id), "One, changed here.\nTwo.\nThree, changed there.\nFour.");
        assert!(merged.conflicts.is_empty() && merged.baselines.is_empty());
    }

    #[test]
    fn the_same_paragraph_changed_twice_keeps_both_as_a_tracked_change() {
        let base = base();
        let id = scene(&base);
        let (mut ours, mut theirs) = (base.clone(), base.clone());
        set_text(&mut ours, id, "One.\nTwo, here.\nThree.");
        set_text(&mut theirs, id, "One.\nTwo, there.\nThree.");
        let merged = merge(&base, &ours, &theirs).unwrap();
        assert_eq!(text(&merged.content, id), "One.\nTwo, here.\nTwo, there.\nThree.");
        assert_eq!(merged.baselines.len(), 1);
        assert_eq!(merged.baselines[0].to_plain_text(), "One.\nTwo, here.\nThree.");
        assert_eq!(merged.conflicts.len(), 1, "{:?}", merged.conflicts);
        // The baseline marks their paragraph as the one change to settle.
        let scene_text = merged.content.documents.iter().find(|d| d.id == id).unwrap();
        let changes = writing_studio_document::tracked_changes(&merged.baselines[0], scene_text);
        assert_eq!(changes.len(), 1);
    }

    #[test]
    fn a_scene_added_on_one_machine_lands_after_the_scene_it_followed() {
        let base = base();
        let first = scene(&base);
        let chapter = base.binder.get(first).unwrap().parent_id;
        let (mut ours, mut theirs) = (base.clone(), base.clone());
        node(&mut ours, chapter.unwrap()).title = "Chapter One".into();
        let mut added = BinderNode::new(NodeKind::Scene, "Added there", chapter, 1);
        added.document_id = Some(added.id);
        theirs.documents.push(Document::from_plain_text(added.id, "New."));
        theirs.binder.nodes.push(added.clone());
        let merged = merge(&base, &ours, &theirs).unwrap();
        let tree = &merged.content.binder;
        assert_eq!(tree.get(chapter.unwrap()).unwrap().title, "Chapter One");
        let order: Vec<&str> = tree.children(chapter).iter().map(|n| n.title.as_str()).collect();
        assert_eq!(order, ["Scene 1", "Added there"]);
        assert_eq!(text(&merged.content, added.id), "New.");
    }

    #[test]
    fn a_deletion_loses_to_an_edit_and_wins_over_nothing() {
        let base = base();
        let id = scene(&base);
        let deleted = |content: &ProjectContent| {
            let mut out = content.clone();
            out.binder.nodes.retain(|n| n.id != id);
            out.documents.retain(|d| d.id != id);
            out
        };
        let mut edited = base.clone();
        set_text(&mut edited, id, "Kept, because it was changed.");
        let merged = merge(&base, &edited, &deleted(&base)).unwrap();
        assert_eq!(text(&merged.content, id), "Kept, because it was changed.");
        assert_eq!(merged.conflicts.len(), 1);
        let merged = merge(&base, &deleted(&base), &edited).unwrap();
        assert_eq!(text(&merged.content, id), "Kept, because it was changed.");
        let merged = merge(&base, &base, &deleted(&base)).unwrap();
        assert!(merged.content.binder.get(id).is_none());
    }

    #[test]
    fn details_of_one_item_are_merged_one_by_one() {
        let base = base();
        let id = scene(&base);
        let (mut ours, mut theirs) = (base.clone(), base.clone());
        node(&mut ours, id).title = "Title here".into();
        node(&mut theirs, id).synopsis = "Synopsis there".into();
        theirs.beats.insert(id, SceneBeats { goal: "Goal there".into(), ..SceneBeats::default() });
        node(&mut ours, id).status = "done".into();
        node(&mut theirs, id).status = "revised".into();
        let merged = merge(&base, &ours, &theirs).unwrap();
        let result = merged.content.binder.get(id).unwrap();
        assert_eq!((result.title.as_str(), result.synopsis.as_str()), ("Title here", "Synopsis there"));
        assert_eq!(result.status, "done");
        assert_eq!(merged.content.beats[&id].goal, "Goal there");
        assert_eq!(merged.conflicts.len(), 1);
    }

    #[test]
    fn the_bible_and_the_project_words_merge_by_entry() {
        let base = base();
        let (mut ours, mut theirs) = (base.clone(), base.clone());
        let mara = ours.cast.add_character();
        ours.cast.set_name(mara, "Mara".into());
        let rose = theirs.cast.add_character();
        theirs.cast.set_name(rose, "Rose".into());
        ours.meta.settings.spelling.words = vec!["zhul".into()];
        theirs.meta.settings.spelling.words = vec!["mara".into()];
        let merged = merge(&base, &ours, &theirs).unwrap();
        let names: Vec<&str> = merged.content.cast.characters.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["Mara", "Rose"]);
        assert_eq!(merged.content.meta.settings.spelling.words, ["mara", "zhul"]);
        assert!(merged.content.files.len() >= base.files.len());
    }

    #[test]
    fn moves_that_would_make_a_loop_end_in_recovered() {
        let base = base();
        let id = scene(&base);
        let chapter = base.binder.get(id).unwrap().parent_id.unwrap();
        // A folder both machines have. Here the chapter goes into it; there
        // it goes into the chapter.
        let mut base = base;
        let folder = BinderNode::new(NodeKind::Folder, "Box", base.binder.get(chapter).unwrap().parent_id, 9);
        base.binder.nodes.push(folder.clone());
        let (mut ours, mut theirs) = (base.clone(), base.clone());
        node(&mut ours, chapter).parent_id = Some(folder.id);
        node(&mut theirs, folder.id).parent_id = Some(chapter);
        let merged = merge(&base, &ours, &theirs).unwrap();
        let tree = &merged.content.binder;
        let recovered = tree.nodes.iter().find(|n| n.title == "Recovered").expect("Recovered");
        assert!(tree.nodes.iter().any(|n| n.parent_id == Some(recovered.id)));
        assert_eq!(text(&merged.content, id), "One.\nTwo.\nThree.");
    }

    #[test]
    fn merging_when_only_one_side_changed_gives_that_side() {
        let base = base();
        let id = scene(&base);
        let mut changed = base.clone();
        set_text(&mut changed, id, "Only here.");
        node(&mut changed, id).title = "Renamed".into();
        for (ours, theirs) in [(&changed, &base), (&base, &changed)] {
            let merged = merge(&base, ours, theirs).unwrap();
            assert_eq!(text(&merged.content, id), "Only here.");
            assert_eq!(merged.content.binder.get(id).unwrap().title, "Renamed");
            assert!(merged.conflicts.is_empty());
        }
        let same = merge(&base, &base, &base).unwrap();
        assert_eq!(same.content.binder.nodes.len(), base.binder.nodes.len());
        assert!(same.conflicts.is_empty());
    }
}

#[cfg(test)]
mod list_merge_tests {
    use super::merge_by_key;

    #[test]
    fn a_list_keeps_both_sides_additions_and_honors_removals() {
        let base = vec![("a", 1), ("b", 1)];
        // This machine changed a and added c; the other took b away and added d.
        let ours = vec![("a", 2), ("b", 1), ("c", 1)];
        let theirs = vec![("a", 1), ("d", 1)];
        let merged = merge_by_key(&base, &ours, &theirs, |item| item.0);
        assert_eq!(merged, [("a", 2), ("c", 1), ("d", 1)]);
        // Changed on both sides: this machine's stays.
        let theirs = vec![("a", 3), ("b", 1)];
        assert_eq!(merge_by_key(&base, &ours, &theirs, |item| item.0)[0], ("a", 2));
    }
}
