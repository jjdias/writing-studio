use super::*;
use crate::mindmap::{step, MapKind, MapLayout, MapNode, Side, Step, NODE_HEIGHT, NODE_WIDTH};
use crate::state::MapStory;

/// Room around the map, in map units.
const MARGIN: f64 = 32.0;

/// The manuscript as a mind map: the book in the middle, its parts,
/// chapters, and scenes as branches. A click chooses a node for the
/// inspector, a double click or Ctrl+Enter opens a scene on the page; the
/// arrows move, Space folds, Tab adds a child and Enter the next one beside
/// it, F2 renames, Delete moves to the trash. A node dragged onto another
/// moves in the binder. The filter and the keyword menu dim what does not
/// match, so the shape of the book stays in view.
///
/// Around the book: ideas, in a row under it, and the story's entries, over
/// it, from the Story menu; lines between any two nodes, drawn from **Join**;
/// and nodes dropped on an empty part of the map stay where they were put,
/// until **Tidy**.
#[component]
pub(super) fn MindMapPane() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let Some(root) = state.read().map_root() else {
        return rsx! {};
    };
    let map = state.read().map_layout(root);
    let has_scenes = !state.read().binder.scenes_under(root).is_empty();
    let query = state.read().map_query.clone();
    let zoom = state.read().map_zoom;
    let chosen = state.read().map_chosen().filter(|id| map.get(*id).is_some());
    let story = state.read().map_story;
    let placed = !state.read().mindmap.positions.is_empty();
    let joining = state.read().map_join.is_some();
    let lines: Vec<writing_studio_project::MapLine> = state
        .read()
        .mindmap
        .lines
        .iter()
        .filter(|line| map.get(line.from).is_some() && map.get(line.to).is_some())
        .cloned()
        .collect();
    let joins: Vec<(Id, Id, MapKind)> = state
        .read()
        .map_extras(&map.nodes.iter().map(|node| node.id).collect::<Vec<_>>())
        .into_iter()
        .flat_map(|extra| extra.joined.into_iter().map(move |to| (extra.id, to, extra.kind)))
        .filter(|(from, to, _)| map.get(*from).is_some() && map.get(*to).is_some())
        .collect();
    let lit = lit_nodes(&state.read(), &map, &query);
    let (left, top) = (map.min_x - MARGIN, map.min_y - MARGIN);
    let (width, height) = (map.width() + 2.0 * MARGIN, map.height() + 2.0 * MARGIN);
    let size = match zoom {
        None => ("100%".to_string(), "100%".to_string()),
        Some(zoom) => (format!("{}px", (width * zoom).round()), format!("{}px", (height * zoom).round())),
    };
    rsx! {
        div { class: "map-pane",
            div { class: "view-tools",
                SeriesScopeSelect {}
                KeywordSelect {}
                input {
                    id: "map-filter",
                    class: "map-filter",
                    placeholder: "Dim what does not match…",
                    title: "Ctrl+F. #plot for a keyword, -#plot to leave it out.",
                    value: "{query}",
                    oninput: move |event| state.write().map_query = event.value(),
                }
                button {
                    title: "An idea that is not in the book, hanging from the chosen node",
                    onclick: move |_| {
                        let attached = state.read().map_chosen().or(Some(root));
                        state.write().add_idea(attached);
                    },
                    "+ Idea"
                }
                if let Some(from) = chosen {
                    button {
                        class: if joining { "active" } else { "" },
                        title: "Draw a line from the chosen node to the next one clicked",
                        onclick: move |_| state.write().start_join(from),
                        "Join"
                    }
                }
                select {
                    aria_label: "Story entries on the map",
                    title: "Show the characters, places, and notes linked to what is on the map",
                    onchange: move |event| {
                        let value = event.value();
                        let shown = STORY_KINDS.into_iter().find(|kind| kind.heading() == value);
                        state.write().map_story = match (value.as_str(), shown) {
                            ("All", _) => MapStory::All,
                            (_, Some(kind)) => MapStory::Kind(kind),
                            _ => MapStory::Hidden,
                        };
                    },
                    option { value: "", selected: story == MapStory::Hidden, "No story entries" }
                    option { value: "All", selected: story == MapStory::All, "All story entries" }
                    for kind in STORY_KINDS {
                        option { value: "{kind.heading()}", selected: story == MapStory::Kind(kind), "{kind.heading()}" }
                    }
                }
                if placed {
                    button { title: "Put every node back where the map lays it out", onclick: move |_| state.write().tidy_map(), "Tidy" }
                }
                div { class: "map-zoom", role: "group", aria_label: "Zoom",
                    button { title: "Zoom out (Ctrl+wheel)", onclick: move |_| state.write().zoom_map(Some(1.0 / 1.25)), "−" }
                    button { class: if zoom.is_none() { "active" } else { "" }, title: "Fit the map to the window", onclick: move |_| state.write().zoom_map(None), "Fit" }
                    button { title: "Zoom in (Ctrl+wheel)", onclick: move |_| state.write().zoom_map(Some(1.25)), "+" }
                }
            }
            if !has_scenes {
                EmptyState {
                    mode: WorkspaceMode::MindMap,
                    title: "No scenes yet",
                    body: "The book is in the middle of the map, and its parts, chapters, and scenes branch out from it.",
                    action: "Add a scene",
                    onclick: move |_| {
                        state.write().add_node(NodeKind::Scene);
                    },
                }
            } else {
                div {
                    class: "map-scroll",
                    tabindex: "0",
                    role: "tree",
                    aria_label: "Mind map of the manuscript. Arrows move, Enter opens, Space folds.",
                    onkeydown: move |event: KeyboardEvent| map_key(state, root, event),
                    // A node let go on an empty part of the map stays there.
                    onmouseup: move |event: MouseEvent| {
                        let Some(dragged) = state.read().dragging else {
                            return;
                        };
                        state.write().end_drag();
                        let point = event.client_coordinates();
                        let moved = state
                            .write()
                            .map_drag_start
                            .take()
                            .is_some_and(|(x, y)| (point.x - x).abs() + (point.y - y).abs() > 6.0);
                        if moved {
                            place_at_pointer(state, dragged, point.x, point.y);
                        }
                    },
                    onmouseleave: move |_| {
                        if state.read().dragging.is_some() {
                            state.write().end_drag();
                        }
                    },
                    onwheel: move |event: WheelEvent| {
                        if event.modifiers().ctrl() {
                            event.prevent_default();
                            let up = event.delta().strip_units().y < 0.0;
                            state.write().zoom_map(Some(if up { 1.1 } else { 1.0 / 1.1 }));
                        }
                    },
                    svg {
                        class: if zoom.is_none() { "map fit" } else { "map" },
                        view_box: "{left} {top} {width} {height}",
                        width: "{size.0}",
                        height: "{size.1}",
                        "preserveAspectRatio": "xMidYMid meet",
                        for node in map.nodes.iter().filter(|node| node.parent.is_some()) {
                            path { class: "map-edge", key: "edge-{node.id}", d: "{edge(&map, node)}" }
                        }
                        for (from, to, kind) in joins {
                            path {
                                class: if kind == MapKind::Idea { "map-join idea" } else { "map-join entry" },
                                key: "join-{from}-{to}",
                                d: "{straight(&map, from, to)}",
                            }
                        }
                        for line in lines {
                            g { class: "map-line", key: "line-{line.id}",
                                path { d: "{straight(&map, line.from, line.to)}" }
                                if !line.label.is_empty() {
                                    text { x: "{middle(&map, line.from, line.to).0}", y: "{middle(&map, line.from, line.to).1 - 4.0}", "text-anchor": "middle", "{line.label}" }
                                }
                            }
                        }
                        for node in map.nodes.clone() {
                            MapNodeView {
                                key: "{node.id}",
                                node: node.clone(),
                                chosen: chosen == Some(node.id),
                                dim: !lit.contains(&node.id),
                            }
                        }
                    }
                }
                if let Some(id) = state.read().renaming.filter(|id| map.get(*id).is_some()) {
                    div { class: "map-note map-rename",
                        span { class: "hint", "Name: " }
                        RenameField { id, name: state.read().binder.get(id).map(|node| node.title.clone()).unwrap_or_default() }
                    }
                } else if let Some(id) = chosen {
                    ChosenNote { id }
                    LinesNote { id }
                }
            }
        }
    }
}

/// The nodes the filter and the chosen keyword or story entry leave lit;
/// the middle always is.
fn lit_nodes(state: &StudioState, map: &MapLayout, query: &str) -> Vec<Id> {
    let parsed = KeywordQuery::parse(query);
    let hits = state.manuscript_index.borrow().search(&parsed.text);
    map.nodes
        .iter()
        .filter(|node| {
            node.parent.is_none()
                || node.kind != MapKind::Binder
                || (state.chosen_allows(node.id)
                    && state.binder.keywords_match(node.id, &parsed.with, false, &parsed.without)
                    && binder_matches(&state.binder, node.id, &parsed.text, &hits))
        })
        .map(|node| node.id)
        .collect()
}

/// The kinds of story entry, in the Story menu's order.
const STORY_KINDS: [BibleLink; 6] = [
    BibleLink::Character,
    BibleLink::Location,
    BibleLink::Culture,
    BibleLink::Lore,
    BibleLink::Timeline,
    BibleLink::Research,
];

/// A straight line between two nodes' centers.
fn straight(map: &MapLayout, from: Id, to: Id) -> String {
    match (map.get(from), map.get(to)) {
        (Some(one), Some(other)) => format!("M{} {} L{} {}", one.x, one.y, other.x, other.y),
        _ => String::new(),
    }
}

/// The middle of a line between two nodes, for its word.
fn middle(map: &MapLayout, from: Id, to: Id) -> (f64, f64) {
    match (map.get(from), map.get(to)) {
        (Some(one), Some(other)) => ((one.x + other.x) / 2.0, (one.y + other.y) / 2.0),
        _ => (0.0, 0.0),
    }
}

/// Keep a node where the pointer let it go: the page turns the pointer's
/// place on screen into the map's units.
fn place_at_pointer(mut state: Signal<StudioState>, id: Id, x: f64, y: f64) {
    spawn(async move {
        let script = format!(
            "const svg = document.querySelector('.map-scroll svg'); const m = svg && svg.getScreenCTM(); \
             if (!m) {{ dioxus.send(null); }} else {{ const p = svg.createSVGPoint(); p.x = {x}; p.y = {y}; \
             const q = p.matrixTransform(m.inverse()); dioxus.send([q.x, q.y]); }}"
        );
        let mut eval = document::eval(&script);
        if let Ok(Some([x, y])) = eval.recv::<Option<[f64; 2]>>().await {
            state.write().place_on_map(id, x, y);
        }
    });
}

/// A curve from a node's parent to the node, edge to edge.
fn edge(map: &MapLayout, node: &MapNode) -> String {
    let Some(parent) = node.parent.and_then(|id| map.get(id)) else {
        return String::new();
    };
    let half = NODE_WIDTH / 2.0;
    let (from, to) = if node.side == Side::Left {
        (parent.x - half, node.x + half)
    } else {
        (parent.x + half, node.x - half)
    };
    let middle = (from + to) / 2.0;
    format!("M{from} {} C{middle} {} {middle} {} {to} {}", parent.y, parent.y, node.y, node.y)
}

#[component]
fn MapNodeView(node: MapNode, chosen: bool, dim: bool) -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let id = node.id;
    if node.kind != MapKind::Binder {
        return rsx! { ExtraNodeView { node, chosen, dim } };
    }
    let Some(item) = state.read().binder.get(id).cloned() else {
        return rsx! {};
    };
    let colors: Vec<String> = state.read().keyword_colors_of(&item).into_iter().map(|(_, hex)| hex).filter(|hex| !hex.is_empty()).collect();
    let title: String = shorten(&item.title, 24);
    let meta = if item.kind == NodeKind::Scene {
        status_name(&item.status)
    } else {
        let held = state.read().binder.scenes_under(id).len();
        let what = item.kind.icon_label();
        match held {
            1 => format!("{what} · 1 scene"),
            many => format!("{what} · {many} scenes"),
        }
    };
    let mut class = format!("map-node kind-{}", item.kind.icon_label().to_lowercase());
    if chosen {
        class.push_str(" chosen");
    }
    if dim {
        class.push_str(" dim");
    }
    let fold_x = if node.side == Side::Left { 0.0 } else { NODE_WIDTH };
    let scene = item.kind == NodeKind::Scene;
    // While another node is dragged, where it would land: before, in, or
    // after this one, and whether it may.
    let dragged = state.read().dragging.filter(|dragged| *dragged != id);
    let mut edge_line = None;
    if let Some(dragged) = dragged.filter(|_| state.read().drop_target == Some(id)) {
        let place = state.read().drop_place;
        let allowed = state.read().map_drop_allowed(dragged, id);
        class.push_str(match (allowed, place) {
            (false, _) => " drop-invalid",
            (true, DropPlace::Before) => " drop-before",
            (true, DropPlace::After) => " drop-after",
            (true, DropPlace::Inside) => " drop-inside",
        });
        edge_line = match (allowed, place) {
            (true, DropPlace::Before) => Some(-3.0),
            (true, DropPlace::After) => Some(NODE_HEIGHT + 3.0),
            _ => None,
        };
    }
    if state.read().dragging == Some(id) {
        class.push_str(" dragging");
    }
    let movable = node.parent.is_some() && !item.kind.is_root();
    // What is not a binder node is let go on the whole node.
    let binder_drag = dragged.is_some_and(|dragged| state.read().binder.get(dragged).is_some());
    let zone = move |place: DropPlace| {
        move |_| {
            let fits = state.peek().drag_fits_inside(id) || !binder_drag;
            let place = if place == DropPlace::Inside && !fits { DropPlace::After } else { place };
            if state.peek().dragging.is_some_and(|dragged| dragged != id) {
                state.write().set_binder_drop(id, place);
            }
        }
    };
    rsx! {
        g {
            class: "{class}",
            transform: "translate({node.x - NODE_WIDTH / 2.0} {node.y - NODE_HEIGHT / 2.0})",
            role: "treeitem",
            "aria-label": "{item.title}",
            "aria-selected": "{chosen}",
            onmousedown: move |event: MouseEvent| start_map_drag(state, id, movable, &event),
            onmouseup: move |event| end_map_drag(state, id, event),
            onclick: move |_| {
                let suppress = std::mem::take(&mut state.write().ignore_next_click);
                if !suppress {
                    state.write().choose_on_map(id);
                }
            },
            ondoubleclick: move |_| {
                if scene {
                    state.write().open_linked_scene(id);
                }
            },
            title { "{item.title}" }
            rect { width: "{NODE_WIDTH}", height: "{NODE_HEIGHT}", rx: "9" }
            if let Some(y) = edge_line {
                line { class: "map-drop-line", x1: "0", y1: "{y}", x2: "{NODE_WIDTH}", y2: "{y}" }
            }
            if dragged.is_some() && !binder_drag {
                rect { class: "map-zone", width: "{NODE_WIDTH}", height: "{NODE_HEIGHT}", onmouseenter: zone(DropPlace::Inside) }
            }
            if binder_drag {
                rect { class: "map-zone", width: "{NODE_WIDTH}", height: "{NODE_HEIGHT / 4.0}", onmouseenter: zone(DropPlace::Before) }
                rect { class: "map-zone", y: "{NODE_HEIGHT / 4.0}", width: "{NODE_WIDTH}", height: "{NODE_HEIGHT / 2.0}", onmouseenter: zone(DropPlace::Inside) }
                rect { class: "map-zone", y: "{NODE_HEIGHT * 0.75}", width: "{NODE_WIDTH}", height: "{NODE_HEIGHT / 4.0}", onmouseenter: zone(DropPlace::After) }
            }
            text { class: "map-title", x: "12", y: "19", "{title}" }
            text { class: "map-meta", x: "12", y: "36", "{meta}" }
            for (at, hex) in colors.into_iter().take(5).enumerate() {
                circle { cx: "{NODE_WIDTH - 12.0 - at as f64 * 11.0}", cy: "12", r: "4", fill: "{hex}" }
            }
            if node.has_children {
                g {
                    class: "map-fold",
                    transform: "translate({fold_x} {NODE_HEIGHT / 2.0})",
                    onclick: move |event| {
                        event.stop_propagation();
                        state.write().toggle_map_fold(id);
                    },
                    title { if node.folded { "Show what is in it (Space)" } else { "Fold it (Space)" } }
                    circle { r: "8" }
                    text { x: "0", y: "4", "text-anchor": "middle", if node.folded { "+" } else { "−" } }
                }
            }
        }
    }
}

/// Start dragging a node with the main button, noting where, so that a
/// click is not taken for a move.
fn start_map_drag(mut state: Signal<StudioState>, id: Id, movable: bool, event: &MouseEvent) {
    if movable && event.trigger_button() == Some(dioxus::html::input_data::MouseButton::Primary) {
        let point = event.client_coordinates();
        state.write().begin_drag(id);
        state.write().map_drag_start = Some((point.x, point.y));
    }
}

/// Let go of a drag over a node: onto another node it is dropped there;
/// over the node itself it was a click.
fn end_map_drag(mut state: Signal<StudioState>, id: Id, event: MouseEvent) {
    let dropping = state.read().dragging.is_some_and(|dragged| dragged != id) && state.read().drop_target == Some(id);
    if dropping {
        event.stop_propagation();
        state.write().ignore_next_click = true;
        state.write().map_drag_start = None;
        state.write().drop_on_map(id);
    } else if state.read().dragging.is_some() {
        state.write().end_drag();
        state.write().map_drag_start = None;
        event.stop_propagation();
    }
}

/// An idea or a story entry on the map: dashed for an idea, with its kind
/// for an entry. A double click opens an entry's page.
#[component]
fn ExtraNodeView(node: MapNode, chosen: bool, dim: bool) -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let id = node.id;
    let (title, meta, entry) = if node.kind == MapKind::Idea {
        let Some(idea) = state.read().mindmap.idea(id).cloned() else {
            return rsx! {};
        };
        (idea.title, "idea".to_string(), false)
    } else {
        let Some(found) = state.read().story_entry(id) else {
            return rsx! {};
        };
        (found.name, found.kind.noun().to_string(), true)
    };
    let mut class = if entry { "map-node kind-entry".to_string() } else { "map-node kind-idea".to_string() };
    if chosen {
        class.push_str(" chosen");
    }
    if dim {
        class.push_str(" dim");
    }
    if state.read().dragging == Some(id) {
        class.push_str(" dragging");
    }
    let target = state.read().dragging.filter(|dragged| *dragged != id && state.read().drop_target == Some(id));
    if let Some(dragged) = target {
        class.push_str(if state.read().map_drop_allowed(dragged, id) { " drop-inside" } else { " drop-invalid" });
    }
    let dragged_other = state.read().dragging.is_some_and(|dragged| dragged != id);
    rsx! {
        g {
            class: "{class}",
            transform: "translate({node.x - NODE_WIDTH / 2.0} {node.y - NODE_HEIGHT / 2.0})",
            role: "treeitem",
            "aria-label": "{title}, {meta}",
            onmousedown: move |event: MouseEvent| start_map_drag(state, id, true, &event),
            onmouseup: move |event| end_map_drag(state, id, event),
            onclick: move |_| {
                let suppress = std::mem::take(&mut state.write().ignore_next_click);
                if !suppress {
                    state.write().choose_on_map(id);
                }
            },
            ondoubleclick: move |_| {
                if entry {
                    state.write().open_story_entry(id);
                }
            },
            rect { width: "{NODE_WIDTH}", height: "{NODE_HEIGHT}", rx: "18" }
            text { class: "map-title", x: "14", y: "19", "{shorten(&title, 24)}" }
            text { class: "map-meta", x: "14", y: "36", "{meta}" }
            if dragged_other {
                rect {
                    class: "map-zone",
                    width: "{NODE_WIDTH}",
                    height: "{NODE_HEIGHT}",
                    onmouseenter: move |_| state.write().set_binder_drop(id, DropPlace::Inside),
                }
            }
        }
    }
}

/// The lines from the chosen node, each with its word to write and × to
/// take it away.
#[component]
fn LinesNote(id: Id) -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let lines: Vec<writing_studio_project::MapLine> =
        state.read().mindmap.lines.iter().filter(|line| line.from == id || line.to == id).cloned().collect();
    if lines.is_empty() {
        return rsx! {};
    }
    let name_of = |other: Id| -> String {
        let current = state.read();
        current
            .binder
            .get(other)
            .map(|node| node.title.clone())
            .or_else(|| current.mindmap.idea(other).map(|idea| idea.title.clone()))
            .or_else(|| current.story_entry(other).map(|entry| entry.name))
            .unwrap_or_else(|| "something in the trash".into())
    };
    rsx! {
        div { class: "map-note map-lines",
            for line in lines {
                {
                    let other = if line.from == id { line.to } else { line.from };
                    let line_id = line.id;
                    rsx! {
                        div { class: "map-line-row", key: "{line_id}",
                            span { "— {name_of(other)}" }
                            input {
                                class: "map-line-label",
                                placeholder: "A word on the line",
                                value: "{line.label}",
                                onchange: move |event| state.write().set_line_label(line_id, event.value()),
                            }
                            button { class: "keyword-remove", title: "Take the line away", onclick: move |_| state.write().remove_line(line_id), "×" }
                        }
                    }
                }
            }
        }
    }
}

/// A scene's synopsis and word count, under the map, once it is chosen.
#[component]
fn ChosenNote(id: Id) -> Element {
    let state = use_context::<Signal<StudioState>>();
    if let Some(idea) = state.read().mindmap.idea(id).cloned() {
        return rsx! { IdeaNote { idea } };
    }
    if let Some(entry) = state.read().story_entry(id) {
        return rsx! {
            div { class: "map-note",
                strong { "{entry.name}" }
                span { class: "card-meta", " · {entry.kind.noun()}. Drag it onto a scene or chapter to link it there; double-click opens it." }
            }
        };
    }
    let Some(item) = state.read().binder.get(id).cloned().filter(|item| item.kind == NodeKind::Scene) else {
        return rsx! {};
    };
    let words = item.document_id.map(|document| words_for_document(&state.read(), document)).unwrap_or(0);
    rsx! {
        div { class: "map-note",
            strong { "{item.title}" }
            span { class: "card-meta", " · {words} words. Double-click or Enter opens it." }
            if item.synopsis.trim().is_empty() {
                p { class: "hint", "No synopsis yet." }
            } else {
                p { "{item.synopsis}" }
            }
        }
    }
}

/// An idea's title and note to write, and what can be done with it.
#[component]
fn IdeaNote(idea: writing_studio_project::Idea) -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let id = idea.id;
    rsx! {
        div { class: "map-note map-idea", key: "{id}",
            input {
                id: "idea-title-{id}",
                class: "map-idea-title",
                aria_label: "The idea's title",
                value: "{idea.title}",
                onchange: move |event| state.write().set_idea_title(id, event.value()),
            }
            textarea {
                aria_label: "The idea's note",
                placeholder: "A short note. It becomes the synopsis if the idea becomes a scene.",
                value: "{idea.note}",
                onchange: move |event| state.write().set_idea_note(id, event.value()),
            }
            div { class: "map-idea-actions",
                button {
                    title: "A new scene in the chapter it hangs from, or where a new scene would go",
                    onclick: move |_| {
                        state.write().idea_to_scene(id, None);
                    },
                    "Make it a scene"
                }
                button { class: "danger", onclick: move |_| state.write().remove_idea(id), "Remove idea" }
            }
        }
    }
}

/// A title cut to fit its node, with an ellipsis.
fn shorten(text: &str, longest: usize) -> String {
    let text = text.trim();
    if text.chars().count() <= longest {
        return text.to_string();
    }
    let cut: String = text.chars().take(longest - 1).collect();
    format!("{}…", cut.trim_end())
}

/// The arrows move the chosen node and Space folds it. Tab adds a child
/// and Enter the next one beside it, each named as it is added; Ctrl+Enter
/// opens a scene; F2 renames; Delete moves to the trash, with Undo.
fn map_key(mut state: Signal<StudioState>, root: Id, event: KeyboardEvent) {
    let key = match event.key() {
        Key::ArrowUp => Some(Step::Up),
        Key::ArrowDown => Some(Step::Down),
        Key::ArrowLeft => Some(Step::Left),
        Key::ArrowRight => Some(Step::Right),
        _ => None,
    };
    let map = state.read().map_layout(root);
    let from = state.read().map_chosen().filter(|id| map.get(*id).is_some()).unwrap_or(root);
    if let Some(key) = key {
        event.prevent_default();
        if let Some(next) = step(&map, from, key) {
            state.write().choose_on_map(next);
        } else if state.read().map_chosen() != Some(from) {
            state.write().choose_on_map(from);
        }
        return;
    }
    let ctrl = event.modifiers().ctrl() || event.modifiers().meta();
    let shift = event.modifiers().shift();
    match event.key() {
        Key::Enter if ctrl => {
            event.prevent_default();
            if state.read().binder.get(from).is_some_and(|node| node.kind == NodeKind::Scene) {
                state.write().open_linked_scene(from);
            }
        }
        Key::Delete if state.read().mindmap.idea(from).is_some() => {
            event.prevent_default();
            state.write().remove_idea(from);
        }
        Key::F2 if state.read().mindmap.idea(from).is_some() => {
            event.prevent_default();
            select_field(format!("idea-title-{from}"));
        }
        Key::Enter | Key::Tab if !shift && state.read().binder.get(from).is_some() => {
            event.prevent_default();
            let child = event.key() == Key::Tab;
            if let Some(added) = state.write().add_from_map(from, child) {
                select_field(format!("rename-{added}"));
            }
        }
        Key::F2 => {
            event.prevent_default();
            state.write().start_rename(from);
            select_field(format!("rename-{from}"));
        }
        Key::Delete if from != root => {
            event.prevent_default();
            state.write().trash_item(from);
        }
        Key::Character(space) if space == " " => {
            event.prevent_default();
            if map.get(from).is_some_and(|node| node.has_children) {
                state.write().toggle_map_fold(from);
            }
        }
        _ => {}
    }
}
