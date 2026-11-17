use super::*;

/// The class of a button that deletes or replaces something: outlined in the
/// danger color, and filled while it waits for the second click.
pub(super) fn danger(armed: bool) -> &'static str {
    if armed { "danger armed" } else { "danger" }
}

/// The label of a button that deletes `name` on its second click. The
/// second click names what it deletes; Escape cancels it.
pub(super) fn confirm_label(armed: bool, verb: &str, name: &str) -> String {
    let name = name.trim();
    match (armed, name.is_empty()) {
        (false, _) => verb.to_string(),
        (true, true) => format!("{verb} untitled"),
        (true, false) => format!("{verb} “{name}”"),
    }
}

pub(super) fn SeriesScopeSelect() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let books = state.read().manuscript_book_choices();
    let current = state.read().series_scope();
    let value = current.map(|id| id.to_string()).unwrap_or_default();
    rsx! {
        select {
            class: "book-select",
            title: "Show one book or the whole series",
            value: "{value}",
            onchange: move |event| {
                let raw = event.value();
                let next = if raw.is_empty() {
                    None
                } else {
                    raw.parse::<Id>().ok()
                };
                state.write().set_series_scope(next);
            },
            aria_label: "Book shown",
            // The webview ignores `value` on a menu, so each choice says
            // whether it is the one shown.
            option { value: "", selected: current.is_none(), "Whole series" }
            for (id, title) in books {
                option { value: "{id}", selected: current == Some(id), "{title}" }
            }
        }
    }
}

#[derive(Clone, PartialEq)]
pub(super) struct NamedId {
    pub(super) id: Id,
    pub(super) name: String,
}

#[component]
pub(super) fn SceneLinks(owner: Id, kind: BibleLink) -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let snapshot = state.read().clone();
    let linked = snapshot.bible_scene_ids(kind, owner);
    let choices = scene_choices(&snapshot.binder)
        .into_iter()
        .filter(|(id, _)| !linked.contains(id))
        .collect::<Vec<_>>();
    rsx! {
        section { class: "cast-block",
            h3 { "Scenes" }
            p { class: "hint", "Links point at scenes in this project. The scene text stays in the manuscript." }
            if linked.is_empty() {
                p { class: "hint", "No scenes linked." }
            }
            for scene_id in linked {
                {
                    let label = scene_caption(&snapshot.binder, scene_id);
                    rsx! {
                        div { class: "cast-row", key: "{scene_id}",
                            span { "{label}" }
                            button {
                                onclick: move |_| state.write().open_linked_scene(scene_id),
                                "Open"
                            }
                            button {
                                onclick: move |_| state.write().unlink_bible_scene(kind, owner, scene_id),
                                "Remove"
                            }
                        }
                    }
                }
            }
            if !choices.is_empty() {
                select {
                    title: "Link a manuscript scene",
                    value: "",
                    onchange: move |event| {
                        if let Some(scene_id) = parse_scene_id(&event.value()) {
                            state.write().link_bible_scene(kind, owner, scene_id);
                        }
                    },
                    option { value: "", selected: true, "Link a scene…" }
                    for (scene_id, label) in choices {
                        option { value: "{scene_id}", "{label}" }
                    }
                }
            }
        }
    }
}

pub(super) fn scene_choices(binder: &writing_studio_project::BinderTree) -> Vec<(Id, String)> {
    let mut scenes: Vec<(Id, String)> = binder
        .nodes
        .iter()
        .filter(|node| node.kind == NodeKind::Scene && !binder.is_in_trash(node.id))
        .map(|node| (node.id, scene_caption(binder, node.id)))
        .collect();
    scenes.sort_by(|left, right| left.1.to_lowercase().cmp(&right.1.to_lowercase()));
    scenes
}

pub(super) fn scene_caption(binder: &writing_studio_project::BinderTree, id: Id) -> String {
    binder.scene_caption(id)
}

pub(super) fn parse_scene_id(value: &str) -> Option<Id> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        value.parse().ok()
    }
}

pub(super) fn words_for_document(snapshot: &StudioState, id: Id) -> u32 {
    if snapshot
        .session
        .as_ref()
        .is_some_and(|session| session.document_id() == id)
    {
        return snapshot.session.as_ref().map(EditorSession::word_count).unwrap_or(0);
    }
    snapshot
        .store
        .as_ref()
        .map(|store| store.borrow().document_word_count(id))
        .unwrap_or(0)
}

/// What goes in an empty mode, with its picture and the button that adds
/// the first one.
#[component]
pub(super) fn EmptyState(
    mode: WorkspaceMode,
    #[props(into)] title: String,
    #[props(into)] body: String,
    #[props(into)] action: String,
    onclick: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        div { class: "empty-state",
            div { class: "empty-picture", aria_hidden: "true", ModeGlyph { mode } }
            h3 { "{title}" }
            p { "{body}" }
            button { class: "primary", onclick: move |event| onclick.call(event), "{action}" }
        }
    }
}
