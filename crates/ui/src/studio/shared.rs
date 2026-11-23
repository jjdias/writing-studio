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
    // A link to an item in the trash hides until it comes back, or goes
    // when the trash is emptied.
    let linked: Vec<Id> = snapshot
        .bible_scene_ids(kind, owner)
        .into_iter()
        .filter(|id| !snapshot.binder.is_in_trash(*id))
        .collect();
    let choices = link_choices(&snapshot.binder)
        .into_iter()
        .filter(|(id, _)| !linked.contains(id))
        .collect::<Vec<_>>();
    rsx! {
        section { class: "cast-block",
            h3 { "In the manuscript" }
            p { class: "hint", "Scenes, or chapters, parts, and books for every scene in them. The text stays in the manuscript; each scene's inspector shows the same links." }
            if linked.is_empty() {
                p { class: "hint", "Not linked yet." }
            }
            for scene_id in linked {
                {
                    let label = item_caption(&snapshot.binder, scene_id);
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
                    title: "Link a scene, or a chapter, part, or book for all its scenes",
                    value: "",
                    onchange: move |event| {
                        if let Some(scene_id) = parse_scene_id(&event.value()) {
                            state.write().link_bible_scene(kind, owner, scene_id);
                        }
                    },
                    option { value: "", selected: true, "Link a scene or chapter…" }
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

/// What a story entry can be linked to: the manuscript's scenes, and its
/// books, parts, and chapters, outside the trash, by caption.
pub(super) fn link_choices(binder: &writing_studio_project::BinderTree) -> Vec<(Id, String)> {
    let mut items: Vec<(Id, String)> = binder
        .nodes
        .iter()
        .filter(|node| matches!(node.kind, NodeKind::Scene | NodeKind::Chapter | NodeKind::Part | NodeKind::Book))
        .filter(|node| binder.in_manuscript(node.id) && !binder.is_in_trash(node.id))
        .map(|node| (node.id, item_caption(binder, node.id)))
        .collect();
    items.sort_by(|left, right| left.1.to_lowercase().cmp(&right.1.to_lowercase()));
    items
}

/// A scene as `scene_caption` gives it; a book, part, or chapter as the
/// path to it, with what it is: "Tide / Chapter 3 (chapter)".
pub(super) fn item_caption(binder: &writing_studio_project::BinderTree, id: Id) -> String {
    let Some(node) = binder.get(id) else {
        return "Missing item".into();
    };
    if node.kind == NodeKind::Scene {
        return binder.scene_caption(id);
    }
    let mut path = vec![node.title.clone()];
    let mut at = node.parent_id;
    while let Some(up) = at.and_then(|up| binder.get(up)) {
        if up.kind.is_root() || path.len() > 64 {
            break;
        }
        path.push(up.title.clone());
        at = up.parent_id;
    }
    path.reverse();
    format!("{} ({})", path.join(" / "), node.kind.icon_label().to_lowercase())
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

/// Narrow the mind map or the outliner to scenes with one keyword. It
/// shows only once the project has keywords.
#[component]
pub(super) fn KeywordSelect() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let known: Vec<String> = state.read().binder.keyword_choices();
    let current = state.read().keyword_filter.clone().unwrap_or_default();
    if known.is_empty() {
        return rsx! {};
    }
    rsx! {
        select {
            class: "book-select",
            aria_label: "Keyword",
            title: "Show only scenes with this keyword",
            onchange: move |event| {
                let value = event.value();
                state.write().keyword_filter = (!value.is_empty()).then_some(value);
            },
            option { value: "", selected: current.is_empty(), "Any keyword" }
            for word in known {
                option { value: "{word}", selected: same_keyword(&word, &current), "{word}" }
            }
        }
    }
}

/// The statuses offered for a scene, in the order work goes.
pub(super) const STATUSES: [&str; 5] = ["To do", "Draft", "Revise", "Done", "On hold"];

/// A status as it is offered, so "draft" reads "Draft". A status that is not
/// one of the offered ones, from an older project, starts with a capital.
pub(super) fn status_name(status: &str) -> String {
    let status = status.trim();
    if let Some(offered) = STATUSES.iter().find(|offered| offered.eq_ignore_ascii_case(status)) {
        return offered.to_string();
    }
    let mut chars = status.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// A dropdown of the statuses. A status not offered, from an older project,
/// is kept as a choice so opening the menu never loses it. With `any`, the
/// first choice is "Any", for a filter.
#[component]
pub(super) fn StatusSelect(id: String, current: String, any: bool, onchange: EventHandler<String>) -> Element {
    let current = status_name(&current);
    let unlisted = (!current.is_empty() && !STATUSES.contains(&current.as_str())).then(|| current.clone());
    rsx! {
        select {
            id: "{id}",
            onchange: move |event| onchange.call(event.value()),
            if any {
                option { value: "", selected: current.is_empty(), "Any" }
            } else if current.is_empty() {
                option { value: "", selected: true, "No status" }
            }
            for status in STATUSES {
                option { value: "{status}", selected: current == status, "{status}" }
            }
            if let Some(other) = unlisted {
                option { value: "{other}", selected: true, "{other}" }
            }
        }
    }
}
