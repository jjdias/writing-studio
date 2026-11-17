use super::*;


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

/// Restoring and emptying, at the top of the Trash's scene list, away
/// from the buttons that add. Moving to the trash is in the inspector, or a drop on Trash.
#[component]
pub(super) fn TrashActions() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let selected_in_trash = state.read().selected.is_some_and(|id| {
        let current = state.read();
        current.binder.is_in_trash(id) && current.binder.get(id).is_some_and(|node| !node.kind.is_root())
    });
    let confirm_empty = state.read().confirm_empty_trash;
    rsx! {
        div { class: "row-actions",
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
