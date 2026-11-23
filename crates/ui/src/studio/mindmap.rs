use super::*;
use crate::mindmap::{layout, step, MapLayout, MapNode, Side, Step, NODE_HEIGHT, NODE_WIDTH};

/// Room around the map, in map units.
const MARGIN: f64 = 32.0;

/// The manuscript as a mind map: the book in the middle, its parts,
/// chapters, and scenes as branches. A click chooses a node for the
/// inspector, a double click opens a scene on the page; the arrows move,
/// Enter opens, and Space folds. The filter and the keyword menu dim what
/// does not match, so the shape of the book stays in view.
#[component]
pub(super) fn MindMapPane() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let Some(root) = state.read().map_root() else {
        return rsx! {};
    };
    let map = {
        let current = state.read();
        layout(&current.binder, root, &current.folded_map)
    };
    let has_scenes = !state.read().binder.scenes_under(root).is_empty();
    let query = state.read().map_query.clone();
    let zoom = state.read().map_zoom;
    let chosen = state.read().selected.filter(|id| map.get(*id).is_some());
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
                if let Some(id) = chosen {
                    ChosenNote { id }
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
                || (state.chosen_allows(node.id)
                    && state.binder.keywords_match(node.id, &parsed.with, false, &parsed.without)
                    && binder_matches(&state.binder, node.id, &parsed.text, &hits))
        })
        .map(|node| node.id)
        .collect()
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
    rsx! {
        g {
            class: "{class}",
            transform: "translate({node.x - NODE_WIDTH / 2.0} {node.y - NODE_HEIGHT / 2.0})",
            role: "treeitem",
            "aria-label": "{item.title}",
            "aria-selected": "{chosen}",
            onclick: move |_| state.write().select_node(id),
            ondoubleclick: move |_| {
                if scene {
                    state.write().open_linked_scene(id);
                }
            },
            title { "{item.title}" }
            rect { width: "{NODE_WIDTH}", height: "{NODE_HEIGHT}", rx: "9" }
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

/// A scene's synopsis and word count, under the map, once it is chosen.
#[component]
fn ChosenNote(id: Id) -> Element {
    let state = use_context::<Signal<StudioState>>();
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

/// A title cut to fit its node, with an ellipsis.
fn shorten(text: &str, longest: usize) -> String {
    let text = text.trim();
    if text.chars().count() <= longest {
        return text.to_string();
    }
    let cut: String = text.chars().take(longest - 1).collect();
    format!("{}…", cut.trim_end())
}

/// The arrows move the chosen node, Enter opens a scene, Space folds.
fn map_key(mut state: Signal<StudioState>, root: Id, event: KeyboardEvent) {
    let key = match event.key() {
        Key::ArrowUp => Some(Step::Up),
        Key::ArrowDown => Some(Step::Down),
        Key::ArrowLeft => Some(Step::Left),
        Key::ArrowRight => Some(Step::Right),
        _ => None,
    };
    let map = {
        let current = state.read();
        layout(&current.binder, root, &current.folded_map)
    };
    let from = state.read().selected.filter(|id| map.get(*id).is_some()).unwrap_or(root);
    if let Some(key) = key {
        event.prevent_default();
        if let Some(next) = step(&map, from, key) {
            state.write().select_node(next);
        } else if state.read().selected != Some(from) {
            state.write().select_node(from);
        }
        return;
    }
    match event.key() {
        Key::Enter => {
            event.prevent_default();
            if state.read().binder.get(from).is_some_and(|node| node.kind == NodeKind::Scene) {
                state.write().open_linked_scene(from);
            }
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
