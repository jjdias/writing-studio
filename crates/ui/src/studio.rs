use dioxus::prelude::*;
use writing_studio_core::Id;
use writing_studio_corkboard::cards_from_binder;
use writing_studio_manuscript::build_outline;
use writing_studio_project::{BinderNode, NodeKind};
use writing_studio_search::matches_query;

use crate::state::{StudioState, WorkspaceMode};

const STYLES: &str = include_str!("../assets/style.css");

#[component]
pub fn App() -> Element {
    use_context_provider(|| Signal::new(StudioState::default()));
    rsx! {
        style { "{STYLES}" }
        Studio {}
    }
}

#[component]
fn Studio() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    use_future(move || async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(400)).await;
            state.write().autosave_tick();
        }
    });
    let open = state.read().is_open();

    rsx! {
        div {
            class: "shell",
            tabindex: 0,
            onkeydown: move |event| {
                let ctrl = event.modifiers().ctrl() || event.modifiers().meta();
                if ctrl && event.key() == Key::Character("s".into()) {
                    event.prevent_default();
                    state.write().save_all();
                }
                if ctrl && event.key() == Key::Character("z".into()) {
                    event.prevent_default();
                    state.write().undo();
                }
                if ctrl && event.key() == Key::Character("y".into()) {
                    event.prevent_default();
                    state.write().redo();
                }
            },
            if open { Workspace {} } else { Welcome {} }
        }
    }
}

#[component]
fn Welcome() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let title = state.read().new_title.clone();
    rsx! {
        div { class: "welcome",
            div { class: "welcome-card",
                p { class: "eyebrow", "Local-first writing studio" }
                h1 { "Writing Studio" }
                p { class: "lede",
                    "Organize a manuscript the way Scrivener does, then write scenes without needing an account or the cloud."
                }
                label { "Project title" }
                input {
                    value: "{title}",
                    oninput: move |event| state.write().new_title = event.value(),
                }
                div { class: "welcome-actions",
                    button { class: "primary", onclick: move |_| state.write().create_project(), "New project" }
                    button { onclick: move |_| state.write().open_project(), "Open project" }
                }
                p { class: "hint", "Projects are folders with a SQLite database. Everything works offline." }
            }
        }
    }
}

#[component]
fn Workspace() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let title = state.read().project_title();
    let status = state.read().status.clone();
    let mode = state.read().mode;
    let focus = state.read().focus_mode;
    let words = state.read().word_count();
    let project_words = state.read().project_word_count();
    let goal = state.read().goal();
    let search = state.read().search.clone();
    let unsaved = state.read().has_unsaved();
    let selected_in_trash = state
        .read()
        .selected
        .is_some_and(|id| state.read().binder.is_in_trash(id));
    let confirm_empty = state.read().confirm_empty_trash;

    rsx! {
        header { class: "topbar",
            div { class: "brand",
                span { class: "logo", "WS" }
                strong { "{title}" }
            }
            input {
                class: "search",
                placeholder: "Filter binder…",
                value: "{search}",
                oninput: move |event| state.write().search = event.value(),
            }
            nav { class: "modes",
                for candidate in [
                    WorkspaceMode::Writing,
                    WorkspaceMode::Corkboard,
                    WorkspaceMode::Outliner,
                    WorkspaceMode::History,
                ] {
                    button {
                        class: if mode == candidate { "mode active" } else { "mode" },
                        onclick: move |_| state.write().mode = candidate,
                        "{candidate.label()}"
                    }
                }
            }
            div { class: "top-actions",
                button { onclick: move |_| state.write().save_all(), "Save" }
                button { onclick: move |_| state.write().backup(), "Backup" }
                button {
                    onclick: move |_| {
                        let next = !state.read().focus_mode;
                        state.write().focus_mode = next;
                    },
                    if focus { "Exit focus" } else { "Focus" }
                }
            }
        }
        div { class: if focus { "workspace focus" } else { "workspace" },
            if !focus {
                aside { class: "binder-pane",
                    div { class: "pane-title", "Binder" }
                    div { class: "binder-tools",
                        button { onclick: move |_| state.write().add_node(NodeKind::Chapter), "Chapter" }
                        button { onclick: move |_| state.write().add_node(NodeKind::Scene), "Scene" }
                        button { onclick: move |_| state.write().add_node(NodeKind::Note), "Note" }
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
                    p { class: "hint binder-hint", "Drag items to reorder, nest, or drop on Trash." }
                    BinderTreeView {}
                }
            }
            main { class: "center",
                {match mode {
                    WorkspaceMode::Writing => rsx! { EditorPane {} },
                    WorkspaceMode::Corkboard => rsx! { CorkboardPane {} },
                    WorkspaceMode::Outliner => rsx! { OutlinerPane {} },
                    WorkspaceMode::History => rsx! { HistoryPane {} },
                }}
            }
            if !focus && matches!(mode, WorkspaceMode::Writing | WorkspaceMode::History) {
                aside { class: "inspector",
                    Inspector {}
                }
            }
        }
        footer { class: "status",
            span { "Words: {words}" }
            span { "Project: {project_words} / {goal}" }
            span {
                if unsaved {
                    "Unsaved · {status}"
                } else {
                    "Offline · {status}"
                }
            }
        }
    }
}

#[component]
fn BinderTreeView() -> Element {
    let state = use_context::<Signal<StudioState>>();
    let roots: Vec<Id> = state
        .read()
        .binder
        .children(None)
        .into_iter()
        .map(|node| node.id)
        .collect();
    rsx! {
        div { class: "binder",
            for id in roots {
                BinderNodeView { id, depth: 0 }
            }
        }
    }
}

#[component]
fn BinderNodeView(id: Id, depth: u32) -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let snapshot = state.read().clone();
    let Some(node) = snapshot.binder.get(id).cloned() else {
        return rsx! {};
    };
    if !matches_query(&node.title, &snapshot.search) && snapshot.search.trim().len() > 0 {
        let child_match = snapshot.binder.children(Some(id)).iter().any(|child| {
            matches_query(&child.title, &snapshot.search)
        });
        if !child_match {
            return rsx! {};
        }
    }
    let selected = snapshot.selected == Some(id);
    let dragging = snapshot.dragging == Some(id);
    let drop_target = snapshot.drop_target == Some(id);
    let trashed = snapshot.binder.is_in_trash(id) && node.kind != NodeKind::TrashRoot;
    let can_drag = !node.kind.is_root();
    let children: Vec<Id> = snapshot
        .binder
        .children(Some(id))
        .into_iter()
        .map(|child| child.id)
        .collect();
    let pad = 8 + depth * 14;
    let class = binder_item_class(selected, dragging, drop_target, trashed);

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
            span { class: "kind", "{node.kind.icon_label()}" }
            span { "{node.title}" }
        }
        for child in children {
            BinderNodeView { id: child, depth: depth + 1 }
        }
    }
}

#[component]
fn EditorPane() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let snapshot = state.read().clone();
    let heading = snapshot
        .selected_node()
        .map(|node| node.title.clone())
        .unwrap_or_else(|| "Select a scene".into());
    let kind = snapshot
        .selected_node()
        .map(|node| node.kind.icon_label())
        .unwrap_or("Document");
    let text = snapshot.editor_text.clone();
    let can_edit = snapshot.session.is_some();

    rsx! {
        div { class: "editor-pane",
            div { class: "editor-meta",
                p { class: "eyebrow", "{kind}" }
                h2 { "{heading}" }
            }
            textarea {
                class: "manuscript",
                disabled: !can_edit,
                value: "{text}",
                oninput: move |event| state.write().apply_editor_text(event.value()),
                onblur: move |_| state.write().flush_editor(),
                placeholder: "Write this scene. Changes save locally a few seconds after you stop typing, on blur, or with Ctrl+S.",
            }
        }
    }
}

#[component]
fn Inspector() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let node: Option<BinderNode> = state.read().selected_node().cloned();
    rsx! {
        div { class: "pane-title", "Inspector" }
        if let Some(node) = node {
            label { "Title" }
            input {
                value: "{node.title}",
                oninput: move |event| state.write().update_selected_title(event.value()),
                onblur: move |_| state.write().save_all(),
            }
            label { "Synopsis" }
            textarea {
                class: "synopsis",
                value: "{node.synopsis}",
                oninput: move |event| state.write().update_selected_synopsis(event.value()),
                onblur: move |_| state.write().save_all(),
            }
            label { "Status" }
            input {
                value: "{node.status}",
                oninput: move |event| state.write().update_selected_status(event.value()),
                onblur: move |_| state.write().save_all(),
            }
            if state.read().binder.is_in_trash(node.id) && node.kind != NodeKind::TrashRoot {
                button { class: "primary", onclick: move |_| state.write().restore_selected(), "Restore from trash" }
            } else if !node.kind.is_root() {
                button { onclick: move |_| state.write().trash_selected(), "Move to trash" }
            }
            button { onclick: move |_| state.write().snapshot(), "Snapshot this document" }
            SnapshotList {}
        } else {
            p { class: "hint", "Select a binder item to edit metadata." }
        }
    }
}

#[component]
fn SnapshotList() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let snapshots = state.read().snapshots_for_selected();
    rsx! {
        div { class: "pane-title history-title", "Snapshots" }
        if snapshots.is_empty() {
            p { class: "hint", "No snapshots yet. Capture one to keep a local copy of this document." }
        } else {
            ul { class: "history-list",
                for item in snapshots {
                    li { class: "history-item",
                        div {
                            strong { "{item.label}" }
                            span { class: "card-meta", "{item.created_label()} · {item.word_count} words" }
                            p { class: "preview", "{item.preview}" }
                        }
                        button {
                            onclick: move |_| state.write().restore_snapshot(item.id),
                            "Restore"
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn HistoryPane() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let snapshots = state.read().snapshots_for_selected();
    let backups = state.read().backups();
    let confirm = state.read().confirm_backup.clone();
    let heading = state
        .read()
        .selected_node()
        .map(|node| node.title.clone())
        .unwrap_or_else(|| "No document selected".into());
    rsx! {
        div { class: "history-pane",
            section {
                h2 { "Document snapshots" }
                p { class: "hint", "Local copies of “{heading}”. Restoring writes the previous text as a pre-restore snapshot." }
                if snapshots.is_empty() {
                    p { class: "hint", "Select a scene or note, then snapshot it from the inspector." }
                } else {
                    ul { class: "history-list wide",
                        for item in snapshots {
                            li { class: "history-item",
                                div {
                                    strong { "{item.label}" }
                                    span { class: "card-meta", "{item.created_label()} · {item.word_count} words" }
                                    p { class: "preview", "{item.preview}" }
                                }
                                button {
                                    onclick: move |_| state.write().restore_snapshot(item.id),
                                    "Restore"
                                }
                            }
                        }
                    }
                }
            }
            section {
                h2 { "Project backups" }
                p { class: "hint", "SQLite copies in this project’s backups folder. Restore is local only and asks for a second click." }
                div { class: "binder-tools",
                    button { class: "primary", onclick: move |_| state.write().backup(), "Write backup now" }
                }
                if backups.is_empty() {
                    p { class: "hint", "No backups yet. Use Backup in the toolbar or the button above." }
                } else {
                    ul { class: "history-list wide",
                        for item in backups {
                            li { class: "history-item",
                                div {
                                    strong { "{item.file_name}" }
                                    span { class: "card-meta", "{item.modified_label} · {item.size_label()}" }
                                }
                                button {
                                    class: if confirm.as_ref() == Some(&item.path) { "primary" } else { "" },
                                    onclick: move |_| state.write().request_backup_restore(item.path.clone()),
                                    if confirm.as_ref() == Some(&item.path) { "Confirm restore" } else { "Restore" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn CorkboardPane() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let snapshot = state.read().clone();
    let cards = {
        let words = |id| {
            snapshot
                .store
                .as_ref()
                .map(|store| store.borrow().document_word_count(id))
                .unwrap_or(0)
        };
        cards_from_binder(&snapshot.binder, words)
    };
    rsx! {
        div { class: "corkboard",
            for card in cards {
                button {
                    class: "card",
                    onclick: move |_| {
                        state.write().mode = WorkspaceMode::Writing;
                        state.write().select_node(card.node_id);
                    },
                    h3 { "{card.title}" }
                    p {
                        if card.synopsis.is_empty() {
                            "No synopsis yet."
                        } else {
                            "{card.synopsis}"
                        }
                    }
                    span { class: "card-meta", "{card.status} · {card.word_count} words" }
                }
            }
        }
    }
}

#[component]
fn OutlinerPane() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let snapshot = state.read().clone();
    let outline = build_outline(&snapshot.binder);
    let books: Vec<OwnedOutlineBook> = outline
        .books
        .iter()
        .map(|book| OwnedOutlineBook {
            title: book.node.title.clone(),
            rows: book
                .chapters
                .iter()
                .flat_map(|chapter| {
                    if chapter.scenes.is_empty() {
                        vec![OwnedOutlineRow {
                            select_id: chapter.node.id,
                            open_writing: false,
                            chapter_title: chapter.node.title.clone(),
                            scene_title: "—".into(),
                            status: chapter.node.status.clone(),
                            synopsis: chapter.node.synopsis.clone(),
                        }]
                    } else {
                        chapter
                            .scenes
                            .iter()
                            .map(|scene| OwnedOutlineRow {
                                select_id: scene.id,
                                open_writing: true,
                                chapter_title: chapter.node.title.clone(),
                                scene_title: scene.title.clone(),
                                status: scene.status.clone(),
                                synopsis: scene.synopsis.clone(),
                            })
                            .collect()
                    }
                })
                .collect(),
        })
        .collect();
    rsx! {
        div { class: "outliner",
            for book in books {
                h3 { "{book.title}" }
                table {
                    thead {
                        tr {
                            th { "Chapter" }
                            th { "Scene" }
                            th { "Status" }
                            th { "Synopsis" }
                        }
                    }
                    tbody {
                        for row in book.rows {
                            tr {
                                onclick: move |_| {
                                    if row.open_writing {
                                        state.write().mode = WorkspaceMode::Writing;
                                    }
                                    state.write().select_node(row.select_id);
                                },
                                td { "{row.chapter_title}" }
                                td { "{row.scene_title}" }
                                td { "{row.status}" }
                                td { "{row.synopsis}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn binder_item_class(selected: bool, dragging: bool, drop_target: bool, trashed: bool) -> String {
    let mut class = String::from("binder-item");
    if selected {
        class.push_str(" selected");
    }
    if dragging {
        class.push_str(" dragging");
    }
    if drop_target {
        class.push_str(" drop-target");
    }
    if trashed {
        class.push_str(" trashed");
    }
    class
}

struct OwnedOutlineBook {
    title: String,
    rows: Vec<OwnedOutlineRow>,
}

struct OwnedOutlineRow {
    select_id: Id,
    open_writing: bool,
    chapter_title: String,
    scene_title: String,
    status: String,
    synopsis: String,
}
