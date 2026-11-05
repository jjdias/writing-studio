use dioxus::prelude::*;
use writing_studio_core::Id;
use writing_studio_corkboard::cards_from_binder;
use writing_studio_document::InlineNote;
use writing_studio_editor::EditorSession;
use writing_studio_manuscript::{outline_rows, row_word_count};
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
                button { onclick: move |_| state.write().export_markdown(), "Markdown" }
                button { onclick: move |_| state.write().export_docx(), "DOCX" }
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
    let html = snapshot.editor_html.clone();
    let epoch = snapshot.editor_epoch;
    let can_edit = snapshot.session.is_some();
    let notes = snapshot
        .session
        .as_ref()
        .map(|session| session.document().comments())
        .unwrap_or_default();
    let initial_html = if html.trim().is_empty() {
        "<p><br></p>".into()
    } else {
        html
    };

    rsx! {
        div { class: "editor-pane",
            div { class: "editor-meta",
                p { class: "eyebrow", "{kind}" }
                h2 { "{heading}" }
            }
            FormatToolbar { enabled: can_edit }
            if !notes.is_empty() {
                ul { class: "comment-list",
                    for note in notes {
                        li {
                            span { class: "comment-anchor", "{comment_anchor(&note)}" }
                            span { "{note.comment}" }
                        }
                    }
                }
            }
            div {
                key: "{epoch}",
                class: "manuscript rich",
                contenteditable: can_edit,
                "data-manuscript": "true",
                dangerous_inner_html: "{initial_html}",
                oninput: move |_| read_manuscript_html(state),
                onblur: move |_| {
                    read_manuscript_html(state);
                    state.write().flush_editor();
                },
            }
        }
    }
}

fn read_manuscript_html(mut state: Signal<StudioState>) {
    spawn(async move {
        let mut eval = document::eval(
            r#"
            const el = document.querySelector('[data-manuscript]');
            dioxus.send(el ? el.innerHTML : '');
            "#,
        );
        if let Ok(html) = eval.recv::<String>().await {
            state.write().apply_editor_html(html);
        }
    });
}

fn comment_anchor(note: &InlineNote) -> String {
    let trimmed = note.text.trim();
    let label = if trimmed.is_empty() { "Comment" } else { trimmed };
    let count = label.chars().count();
    if count <= 42 {
        label.to_string()
    } else {
        format!("{}…", label.chars().take(42).collect::<String>())
    }
}

fn run_format_command(mut state: Signal<StudioState>, command: &'static str, argument: &'static str) {
    spawn(async move {
        let script = format!(
            r#"
            const el = document.querySelector('[data-manuscript]');
            if (el) {{ el.focus(); }}
            document.execCommand('{command}', false, {arg});
            dioxus.send(el ? el.innerHTML : '');
            "#,
            command = command,
            arg = if argument.is_empty() {
                "null".into()
            } else {
                format!("'{argument}'")
            }
        );
        let mut eval = document::eval(&script);
        if let Ok(html) = eval.recv::<String>().await {
            state.write().apply_editor_html(html);
        }
    });
}

fn run_editor_script(mut state: Signal<StudioState>, body: &'static str) {
    spawn(async move {
        let script = format!(
            r#"
            const el = document.querySelector('[data-manuscript]');
            if (el) {{ el.focus(); }}
            {body}
            dioxus.send(el ? el.innerHTML : '');
            "#
        );
        let mut eval = document::eval(&script);
        if let Ok(html) = eval.recv::<String>().await {
            state.write().apply_editor_html(html);
        }
    });
}

#[component]
fn FormatToolbar(enabled: bool) -> Element {
    let state = use_context::<Signal<StudioState>>();
    rsx! {
        div { class: "format-toolbar",
            div { class: "format-group",
                button { disabled: !enabled, onclick: move |_| run_format_command(state, "bold", ""), "Bold" }
                button { disabled: !enabled, onclick: move |_| run_format_command(state, "italic", ""), "Italic" }
                button { disabled: !enabled, onclick: move |_| run_format_command(state, "underline", ""), "Underline" }
                button { disabled: !enabled, onclick: move |_| run_format_command(state, "strikeThrough", ""), "Strike" }
                button { disabled: !enabled, onclick: move |_| run_editor_script(state, LINK_SCRIPT), "Link" }
                button { disabled: !enabled, onclick: move |_| run_format_command(state, "unlink", ""), "Unlink" }
                button { disabled: !enabled, onclick: move |_| run_editor_script(state, COMMENT_SCRIPT), "Comment" }
                button { disabled: !enabled, onclick: move |_| run_editor_script(state, UNCOMMENT_SCRIPT), "Uncomment" }
            }
            div { class: "format-group",
                button { disabled: !enabled, onclick: move |_| run_format_command(state, "formatBlock", "H1"), "H1" }
                button { disabled: !enabled, onclick: move |_| run_format_command(state, "formatBlock", "H2"), "H2" }
                button { disabled: !enabled, onclick: move |_| run_format_command(state, "formatBlock", "H3"), "H3" }
                button { disabled: !enabled, onclick: move |_| run_format_command(state, "formatBlock", "P"), "Body" }
                button { disabled: !enabled, onclick: move |_| run_format_command(state, "formatBlock", "BLOCKQUOTE"), "Quote" }
                button { disabled: !enabled, onclick: move |_| run_format_command(state, "insertUnorderedList", ""), "List" }
                button { disabled: !enabled, onclick: move |_| run_format_command(state, "insertOrderedList", ""), "Numbered" }
                button { disabled: !enabled, onclick: move |_| run_format_command(state, "formatBlock", "PRE"), "Code" }
                button { disabled: !enabled, onclick: move |_| run_format_command(state, "insertHorizontalRule", ""), "Break" }
            }
        }
    }
}

const LINK_SCRIPT: &str = r#"
const sel = window.getSelection();
if (sel && sel.rangeCount && !sel.isCollapsed) {
    const saved = sel.getRangeAt(0).cloneRange();
    const raw = window.prompt('Link address');
    if (raw && raw.trim()) {
        let url = raw.trim();
        if (!/^[a-z][a-z0-9+.-]*:/i.test(url)) {
            url = 'https://' + url;
        }
        if (/^(https?:|mailto:)/i.test(url)) {
            sel.removeAllRanges();
            sel.addRange(saved);
            document.execCommand('createLink', false, url);
        }
    }
}
"#;

const COMMENT_SCRIPT: &str = r#"
const sel = window.getSelection();
if (sel && sel.rangeCount && !sel.isCollapsed && el && el.contains(sel.getRangeAt(0).commonAncestorContainer)) {
    const saved = sel.getRangeAt(0).cloneRange();
    const raw = window.prompt('Comment on the selection');
    if (raw && raw.trim()) {
        sel.removeAllRanges();
        sel.addRange(saved);
        const span = document.createElement('span');
        span.className = 'ws-comment';
        span.setAttribute('data-comment', raw.trim());
        span.title = raw.trim();
        const range = sel.getRangeAt(0);
        try {
            range.surroundContents(span);
        } catch (err) {
            span.appendChild(range.extractContents());
            range.insertNode(span);
        }
    }
}
"#;

const UNCOMMENT_SCRIPT: &str = r#"
const sel = window.getSelection();
if (sel && sel.rangeCount && el) {
    const range = sel.getRangeAt(0);
    const spans = Array.from(el.querySelectorAll('span[data-comment]'));
    spans.forEach((span) => {
        const hit = range.collapsed
            ? span.contains(range.startContainer)
            : (range.intersectsNode ? range.intersectsNode(span) : false);
        if (!hit) return;
        const parent = span.parentNode;
        while (span.firstChild) parent.insertBefore(span.firstChild, span);
        parent.removeChild(span);
    });
}
"#;

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
    let mut books: Vec<OwnedOutlineBook> = Vec::new();
    for line in outline_rows(&snapshot.binder) {
        let Some(chapter) = snapshot.binder.get(line.chapter_id) else {
            continue;
        };
        let scene = line.scene_id.and_then(|id| snapshot.binder.get(id));
        let meta = scene.unwrap_or(chapter);
        let row = OwnedOutlineRow {
            chapter_id: line.chapter_id,
            scene_id: line.scene_id,
            open_id: meta.id,
            chapter_title: chapter.title.clone(),
            scene_title: scene.map(|node| node.title.clone()).unwrap_or_default(),
            status: meta.status.clone(),
            synopsis: meta.synopsis.clone(),
            words: row_word_count(line.document_id, |id| words_for_document(&snapshot, id)),
        };
        if books.last().map(|book| book.id) != Some(line.book_id) {
            let title = snapshot
                .binder
                .get(line.book_id)
                .map(|node| node.title.clone())
                .unwrap_or_default();
            books.push(OwnedOutlineBook {
                id: line.book_id,
                title,
                rows: Vec::new(),
            });
        }
        if let Some(book) = books.last_mut() {
            book.rows.push(row);
        }
    }
    rsx! {
        div { class: "outliner",
            if books.is_empty() {
                p { class: "hint", "Add a chapter in the binder to outline it here." }
            }
            for book in books {
                h3 { "{book.title}" }
                table {
                    thead {
                        tr {
                            th { "Chapter" }
                            th { "Scene" }
                            th { "Status" }
                            th { "Synopsis" }
                            th { "Words" }
                            th {}
                        }
                    }
                    tbody {
                        for row in book.rows {
                            tr {
                                td {
                                    input {
                                        class: "cell-input",
                                        value: "{row.chapter_title}",
                                        oninput: move |event| {
                                            state.write().update_node_title(row.chapter_id, event.value());
                                        },
                                    }
                                }
                                td {
                                    if let Some(scene_id) = row.scene_id {
                                        input {
                                            class: "cell-input",
                                            value: "{row.scene_title}",
                                            oninput: move |event| {
                                                state.write().update_node_title(scene_id, event.value());
                                            },
                                        }
                                    } else {
                                        span { class: "hint", "—" }
                                    }
                                }
                                td {
                                    input {
                                        class: "cell-input",
                                        value: "{row.status}",
                                        oninput: move |event| {
                                            state.write().update_node_status(row.open_id, event.value());
                                        },
                                    }
                                }
                                td {
                                    input {
                                        class: "cell-input",
                                        value: "{row.synopsis}",
                                        oninput: move |event| {
                                            state.write().update_node_synopsis(row.open_id, event.value());
                                        },
                                    }
                                }
                                td { class: "words", "{row.words}" }
                                td {
                                    button {
                                        onclick: move |_| {
                                            state.write().select_node(row.open_id);
                                            state.write().mode = WorkspaceMode::Writing;
                                        },
                                        "Open"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn words_for_document(snapshot: &StudioState, id: Id) -> u32 {
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
    id: Id,
    title: String,
    rows: Vec<OwnedOutlineRow>,
}

struct OwnedOutlineRow {
    chapter_id: Id,
    scene_id: Option<Id>,
    open_id: Id,
    chapter_title: String,
    scene_title: String,
    status: String,
    synopsis: String,
    words: u32,
}
