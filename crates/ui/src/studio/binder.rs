use super::*;

#[component]
pub(super) fn Explorer() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let chrome = use_context::<Signal<PanelChrome>>();
    let widths = use_context::<Signal<PanelWidths>>();
    let width = widths.read().sidebar;
    let title = state.read().project_title();
    let mode = state.read().mode;
    let search = state.read().search.clone();
    let side = chrome.read().sidebar_side;
    let add_open = state.read().open_menu == Some(FrameMenu::Add);
    let add = move |kind: NodeKind| {
        move |_| {
            state.write().open_menu = None;
            state.write().add_node(kind);
        }
    };
    rsx! {
        aside {
            class: "sidebar",
            style: "width: {width}px; flex: 0 0 {width}px;",
            div { class: "view-title",
                span { "{mode.view_title()}" }
                button {
                    class: "view-action",
                    title: "Move the binder to the other side",
                    aria_label: "Move the binder to the other side",
                    onclick: move |_| flip_sidebar(state, chrome),
                    if side == SidebarSide::Left { "→" } else { "←" }
                }
            }
            div { class: "section-label", "{title}" }
            input {
                id: "binder-search",
                class: "search",
                placeholder: "Search titles and text",
                title: "Ctrl+F",
                value: "{search}",
                oninput: move |event| state.write().search = event.value(),
            }
            div { class: "binder-tools",
                button { title: "New chapter (Ctrl+Shift+N)", onclick: move |_| state.write().add_node(NodeKind::Chapter), "Chapter" }
                button { title: "New scene (Ctrl+N)", onclick: move |_| state.write().add_node(NodeKind::Scene), "Scene" }
                div { class: "menu-anchor",
                    button {
                        class: if add_open { "menu-btn open" } else { "menu-btn" },
                        title: "Add a book, a part, or a folder",
                        aria_haspopup: "menu",
                        aria_expanded: "{add_open}",
                        onclick: move |_| state.write().toggle_menu(FrameMenu::Add),
                        "Add"
                    }
                    if add_open {
                        div { class: "menu", role: "menu", aria_label: "Add",
                            button { class: "menu-item", role: "menuitem", title: "Another novel under the manuscript", onclick: add(NodeKind::Book), "Book" }
                            button { class: "menu-item", role: "menuitem", onclick: add(NodeKind::Part), "Part" }
                            button { class: "menu-item", role: "menuitem", onclick: add(NodeKind::Folder), "Folder" }
                        }
                    }
                }
            }
            div { class: "binder-tools", SeriesScopeSelect {} }
            BinderTreeView {}
        }
    }
}

#[component]
pub(super) fn BinderTreeView() -> Element {
    let state = use_context::<Signal<StudioState>>();
    let snapshot = state.read().clone();
    let roots: Vec<Id> = snapshot
        .binder
        .children(None)
        .into_iter()
        .filter(|node| node.kind != NodeKind::ResearchRoot)
        .map(|node| node.id)
        .collect();
    let hits = Rc::new(snapshot.content_hits());
    rsx! {
        div { class: "binder",
            for id in roots {
                BinderNodeView { id, depth: 0, hits: hits.clone() }
            }
        }
    }
}

#[component]
pub(super) fn BinderNodeView(id: Id, depth: u32, hits: Rc<HashSet<Id>>) -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let snapshot = state.read().clone();
    let Some(node) = snapshot.binder.get(id).cloned() else {
        return rsx! {};
    };
    if !node_in_series_scope(&snapshot.binder, id, snapshot.series_scope()) {
        return rsx! {};
    }
    if !binder_matches(&snapshot.binder, id, &snapshot.search, &hits) {
        return rsx! {};
    }
    let selected = snapshot.selected == Some(id);
    let dragging = snapshot.dragging == Some(id);
    let drop_target = snapshot.drop_target == Some(id);
    let drop_ok = snapshot
        .dragging
        .is_some_and(|dragged| snapshot.binder.drop_allowed(dragged, id));
    let trashed = snapshot.binder.is_in_trash(id) && node.kind != NodeKind::TrashRoot;
    let can_drag = !node.kind.is_root();
    let children: Vec<Id> = snapshot
        .binder
        .children(Some(id))
        .into_iter()
        .map(|child| child.id)
        .collect();
    let pad = 8 + depth * 16;
    let class = binder_item_class(selected, dragging, drop_target, drop_ok, trashed);
    let missing = snapshot.is_missing(id);
    let class = if missing { format!("{class} missing") } else { class };
    let color = binder_color_hex(&node.color);
    let label = node.label.clone();
    let is_trash = node.kind == NodeKind::TrashRoot;

    rsx! {
        div { class: "binder-row",
        button {
            class: "{class}",
            style: "padding-left: {pad}px",
            draggable: can_drag,
            onclick: move |_| state.write().select_node(id),
            ondragstart: move |event| {
                event.stop_propagation();
                state.write().begin_drag(id);
            },
            ondragover: move |event| {
                event.prevent_default();
                event.stop_propagation();
                state.write().set_drop_target(id);
            },
            ondrop: move |event| {
                event.prevent_default();
                event.stop_propagation();
                state.write().drop_on(id);
            },
            ondragend: move |_| state.write().end_drag(),
            if !color.is_empty() {
                span { class: "swatch", style: "background: {color}", title: "{label}" }
            }
            KindGlyph { kind: node.kind }
            span { class: "binder-title", "{node.title}" }
            if missing {
                span { class: "binder-label", title: "This item's file could not be read. History says which file.", "missing" }
            }
            if !label.is_empty() {
                span { class: "binder-label", "{label}" }
            }
        }
        if is_trash {
            TrashActions {}
        }
        }
        for child in children {
            BinderNodeView { id: child, depth: depth + 1, hits: hits.clone() }
        }
    }
}

pub(super) fn node_in_series_scope(
    tree: &writing_studio_project::BinderTree,
    id: Id,
    scope: Option<Id>,
) -> bool {
    let Some(book) = scope else {
        return true;
    };
    if tree.get(id).is_some_and(|node| node.kind.is_root()) {
        return true;
    }
    if !tree.in_manuscript(id) {
        return true;
    }
    tree.book_of(id) == Some(book)
}

pub(super) fn binder_matches(
    tree: &writing_studio_project::BinderTree,
    id: Id,
    query: &str,
    hits: &HashSet<Id>,
) -> bool {
    if query.trim().is_empty() {
        return true;
    }
    let Some(node) = tree.get(id) else {
        return false;
    };
    if matches_query(&node.title, query) || matches_query(&node.label, query) {
        return true;
    }
    if node.document_id.is_some_and(|document_id| hits.contains(&document_id)) {
        return true;
    }
    tree.children(Some(id))
        .iter()
        .any(|child| binder_matches(tree, child.id, query, hits))
}

pub(super) fn binder_item_class(
    selected: bool,
    dragging: bool,
    drop_target: bool,
    drop_ok: bool,
    trashed: bool,
) -> String {
    let mut class = String::from("binder-item");
    if selected {
        class.push_str(" selected");
    }
    if dragging {
        class.push_str(" dragging");
    }
    if drop_target && drop_ok {
        class.push_str(" drop-target");
    }
    if drop_target && !drop_ok {
        class.push_str(" drop-invalid");
    }
    if trashed {
        class.push_str(" trashed");
    }
    class
}

/// The kind of a binder item, as a small icon with the kind as its name.
#[component]
pub(super) fn KindGlyph(kind: NodeKind) -> Element {
    let shape = match kind {
        NodeKind::DraftRoot | NodeKind::Book => "M3 2.5h8.5a1 1 0 011 1v10H4a1 1 0 01-1-1zM3 12.5a1 1 0 011-1h8.5",
        NodeKind::Part => "M2.5 4h11M2.5 8h11M2.5 12h11",
        NodeKind::Chapter => "M3.5 2.5h9v11h-9zM6 5.5h4M6 8h4",
        NodeKind::Scene | NodeKind::Note => "M4 2.5h5.2L12 5.3v8.2H4zM9 2.7v2.8h2.8",
        NodeKind::Folder | NodeKind::ResearchRoot => "M2 4.5h4l1.2 1.5H14v6.5H2z",
        NodeKind::TrashRoot => "M3 4.5h10M6.5 4.5V3h3v1.5M4.5 4.5l.7 9h5.6l.7-9",
    };
    rsx! {
        svg {
            class: "kind-glyph",
            view_box: "0 0 16 16",
            role: "img",
            "aria-label": "{kind.icon_label()}",
            title { "{kind.icon_label()}" }
            path { d: "{shape}", fill: "none", stroke: "currentColor", stroke_width: "1.2", stroke_linejoin: "round" }
        }
    }
}

/// Moving to the trash, restoring, and emptying it, on the Trash row and
/// away from the buttons that add.
#[component]
pub(super) fn TrashActions() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let mut open = use_signal(|| false);
    let selected = state.read().selected;
    let can_trash = selected.is_some_and(|id| {
        let current = state.read();
        current.binder.get(id).is_some_and(|node| !node.kind.is_root()) && !current.binder.is_in_trash(id)
    });
    let selected_in_trash = selected.is_some_and(|id| {
        let current = state.read();
        current.binder.is_in_trash(id) && current.binder.get(id).is_some_and(|node| !node.kind.is_root())
    });
    let confirm_empty = state.read().confirm_empty_trash;
    rsx! {
        button {
            class: if open() { "row-menu open" } else { "row-menu" },
            title: "Trash actions",
            aria_label: "Trash actions",
            aria_expanded: "{open()}",
            onclick: move |_| open.set(!open()),
            "⋯"
        }
        if open() {
            div { class: "row-actions",
                button {
                    disabled: !can_trash,
                    title: "Move the selected item to the trash",
                    onclick: move |_| state.write().trash_selected(),
                    "Move selected here"
                }
                if selected_in_trash {
                    button { class: "primary", onclick: move |_| state.write().restore_selected(), "Restore selected" }
                }
                button {
                    class: danger(confirm_empty),
                    title: "Delete everything in the trash for good. Asks for a second click.",
                    onclick: move |_| state.write().empty_trash(),
                    if confirm_empty { "Confirm empty" } else { "Empty trash" }
                }
            }
        }
    }
}
