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
    let selected = state.read().selected;
    let selected_in_trash = selected.is_some_and(|id| state.read().binder.is_in_trash(id));
    let confirm_empty = state.read().confirm_empty_trash;
    rsx! {
        aside {
            class: "sidebar",
            style: "width: {width}px; flex: 0 0 {width}px;",
            div { class: "view-title",
                span { "{mode.view_title()}" }
                button {
                    class: "view-action",
                    title: "Move the binder to the other side",
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
                button { title: "New book under the manuscript", onclick: move |_| state.write().add_node(NodeKind::Book), "Book" }
                SeriesScopeSelect {}
            }
            div { class: "binder-tools",
                button { title: "New part", onclick: move |_| state.write().add_node(NodeKind::Part), "Part" }
                button { title: "New chapter (Ctrl+Shift+N)", onclick: move |_| state.write().add_node(NodeKind::Chapter), "Chapter" }
                button { title: "New scene (Ctrl+N)", onclick: move |_| state.write().add_node(NodeKind::Scene), "Scene" }
                button { title: "New folder", onclick: move |_| state.write().add_node(NodeKind::Folder), "Folder" }
                button {
                    title: "Bring Markdown files in as new scenes",
                    onclick: move |_| {
                        let files = pick_markdown_files();
                        if !files.is_empty() {
                            state.write().import_markdown_files(files);
                        }
                    },
                    "Import"
                }
            }
            div { class: "binder-tools",
                button { onclick: move |_| state.write().trash_selected(), "Trash" }
                if selected_in_trash {
                    button { class: "primary", onclick: move |_| state.write().restore_selected(), "Restore" }
                }
                button {
                    class: if confirm_empty { "primary" } else { "" },
                    onclick: move |_| state.write().empty_trash(),
                    if confirm_empty { "Confirm empty" } else { "Empty trash" }
                }
            }
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

    rsx! {
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
            span { class: "kind", "{node.kind.icon_label()}" }
            span { class: "binder-title", "{node.title}" }
            if missing {
                span { class: "binder-label", title: "This item's file could not be read. History says which file.", "missing" }
            }
            if !label.is_empty() {
                span { class: "binder-label", "{label}" }
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
