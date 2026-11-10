use std::collections::HashSet;
use std::rc::Rc;

use dioxus::prelude::*;
use writing_studio_core::Id;
use writing_studio_corkboard::{card_matches, cards_from_binder, group_cards, CorkGroup};
use writing_studio_document::InlineNote;
use writing_studio_editor::EditorSession;
use writing_studio_manuscript::{outline_rows, row_word_count};
use writing_studio_project::{
    binder_color_hex, drag_width, format_elapsed, BinderNode, NodeKind, PanelChrome, PanelLayout,
    PanelWidths, SidebarSide, Theme, BINDER_COLORS, INSPECTOR_WIDTH_DEFAULT, INSPECTOR_WIDTH_MAX,
    INSPECTOR_WIDTH_MIN, SIDEBAR_WIDTH_DEFAULT, SIDEBAR_WIDTH_MAX, SIDEBAR_WIDTH_MIN,
};
use writing_studio_search::matches_query;

use crate::shortcuts::{app_shortcut, AppShortcut};
use crate::state::{BibleLink, CompiledFormat, StudioState, WorkspaceMode, WorldSection};

const STYLES: &str = include_str!("../assets/style.css");

#[component]
pub fn App() -> Element {
    use_context_provider(|| Signal::new(StudioState::default()));
    use_context_provider(|| Signal::new(PanelChrome::default()));
    use_context_provider(|| Signal::new(PanelWidths::default()));
    use_context_provider(|| Signal::new(None::<ResizeDrag>));
    rsx! {
        style { "{STYLES}" }
        Studio {}
    }
}

#[component]
fn Studio() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let chrome = use_context::<Signal<PanelChrome>>();
    let widths = use_context::<Signal<PanelWidths>>();
    let resize = use_context::<Signal<Option<ResizeDrag>>>();
    use_future(move || async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(400)).await;
            let (save, roll, label) = {
                let current = state.peek();
                (
                    current.autosave_ready(),
                    current.writing_day_needs_roll(),
                    current.session_label_due(),
                )
            };
            if !(save || roll || label) {
                continue;
            }
            let mut current = state.write();
            if save {
                current.autosave_tick();
            }
            if roll {
                current.writing_clock_tick();
            }
            if label {
                current.note_session_label();
            }
        }
    });
    let open = state.read().is_open();
    let theme = state.read().theme();

    rsx! {
        div {
            class: match theme {
                Theme::Dark => "shell theme-dark",
                Theme::Light => "shell theme-light",
                Theme::Paper => "shell theme-paper",
            },
            tabindex: 0,
            onmousemove: move |event| {
                let Some(drag) = *resize.peek() else {
                    return;
                };
                let delta = (event.client_coordinates().x - drag.origin_x).round() as i32;
                let (min, max) = resize_limits(drag.kind);
                let width = drag_width(drag.origin_width, delta, drag.positive, min, max);
                let mut next = *widths.peek();
                let unchanged = match drag.kind {
                    ResizeKind::Sidebar => next.sidebar == width,
                    ResizeKind::Inspector => next.inspector == width,
                };
                if unchanged {
                    return;
                }
                match drag.kind {
                    ResizeKind::Sidebar => next.sidebar = width,
                    ResizeKind::Inspector => next.inspector = width,
                }
                let mut widths = widths;
                widths.set(next);
            },
            onmouseup: move |_| finish_resize(state, widths, resize),
            onmouseleave: move |_| finish_resize(state, widths, resize),
            onkeydown: move |event| {
                let key = match event.key() {
                    Key::Character(value) => value,
                    Key::Escape => "escape".into(),
                    _ => return,
                };
                let ctrl = event.modifiers().ctrl() || event.modifiers().meta();
                let Some(action) = app_shortcut(
                    &key,
                    ctrl,
                    event.modifiers().shift(),
                    event.modifiers().alt(),
                ) else {
                    return;
                };
                if apply_shortcut(state, chrome, widths, action) {
                    event.prevent_default();
                }
            },
            if open { Workspace {} } else { Welcome {} }
        }
    }
}

#[component]
fn Welcome() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let chrome = use_context::<Signal<PanelChrome>>();
    let widths = use_context::<Signal<PanelWidths>>();
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
                    button { class: "primary", onclick: move |_| create_project(state, chrome, widths), "New project" }
                    button { onclick: move |_| open_project(state, chrome, widths), "Open project" }
                }
                p { class: "hint", "Projects are folders with a SQLite database. Everything works offline." }
            }
        }
    }
}

#[component]
fn Workspace() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let status = state.read().status.clone();
    let mode = state.read().mode;
    let focus = state.read().focus_mode;
    let words = state.read().word_count();
    let project_words = state.read().project_word_count();
    let book_words = state.read().compile_book_word_count();
    let book_title = state.read().compile_book_title();
    let goal = state.read().goal();
    let daily_goal = state.read().daily_goal();
    let today_words = state.read().words_today_at(project_words);
    let session_words = state.read().words_this_session_at(project_words);
    let session_time = format_elapsed(state.read().session_shown_secs);
    let unsaved = state.read().has_unsaved();
    let theme = state.read().theme();
    let chrome = use_context::<Signal<PanelChrome>>();
    let arrangement = chrome.read();
    let sidebar_left = arrangement.sidebar_side == SidebarSide::Left;
    let show_sidebar = !focus && arrangement.sidebar_visible;
    let show_inspector = !focus
        && arrangement.inspector_visible
        && matches!(mode, WorkspaceMode::Writing | WorkspaceMode::History);
    let tab_title = {
        let current = state.read();
        if current.mode == WorkspaceMode::Writing {
            current
                .selected_node()
                .map(|node| node.title.clone())
                .unwrap_or_else(|| "Untitled".into())
        } else {
            current.mode.label().into()
        }
    };

    rsx! {
        div { class: if focus { "workbench focus" } else { "workbench" },
            nav { class: "activity",
                for candidate in [
                    WorkspaceMode::Writing,
                    WorkspaceMode::Corkboard,
                    WorkspaceMode::Outliner,
                    WorkspaceMode::Plot,
                    WorkspaceMode::Characters,
                    WorkspaceMode::World,
                    WorkspaceMode::Research,
                    WorkspaceMode::History,
                ] {
                    button {
                        class: if mode == candidate { "activity-btn active" } else { "activity-btn" },
                        title: "{candidate.label()} ({mode_shortcut(candidate)}). Click again to hide the binder.",
                        onclick: move |_| select_activity(state, chrome, candidate),
                        ModeGlyph { mode: candidate }
                    }
                }
                div { class: "activity-spacer" }
                button {
                    class: if arrangement.inspector_visible { "activity-btn active" } else { "activity-btn" },
                    title: "Inspector (Ctrl+Alt+I)",
                    onclick: move |_| toggle_inspector(state, chrome),
                    svg { class: "glyph", view_box: "0 0 16 16",
                        path { d: "M1.5 2.5h13v11h-13zM10 2.5v11", fill: "none", stroke: "currentColor", stroke_width: "1.2" }
                    }
                }
                button {
                    class: if focus { "activity-btn active" } else { "activity-btn" },
                    title: "Zen mode (Ctrl+Shift+F)",
                    onclick: move |_| {
                        let next = !state.read().focus_mode;
                        state.write().focus_mode = next;
                    },
                    svg { class: "glyph", view_box: "0 0 16 16",
                        path { d: "M2 6V2h4M10 2h4v4M14 10v4h-4M6 14H2v-4", fill: "none", stroke: "currentColor", stroke_width: "1.2" }
                    }
                }
            }
            if show_sidebar && sidebar_left {
                Explorer {}
                Sash { kind: ResizeKind::Sidebar, positive: true }
            }
            if show_inspector && !sidebar_left {
                InspectorPane {}
                Sash { kind: ResizeKind::Inspector, positive: true }
            }
            div { class: "editor-group",
                div { class: "tabs",
                    div { class: "tab active",
                        span { class: "tab-label", "{tab_title}" }
                        if unsaved {
                            span { class: "tab-dirty", title: "Unsaved changes" }
                        }
                    }
                    div { class: "editor-actions",
                        button { title: "Save (Ctrl+S)", onclick: move |_| state.write().save_all(), "Save" }
                        button { title: "Backup", onclick: move |_| state.write().backup(), "Backup" }
                        CompileBookSelect {}
                        button { title: "Compile the chosen book to Markdown", onclick: move |_| export_project(state, CompiledFormat::Markdown), "Markdown" }
                        button { title: "Compile the chosen book to DOCX", onclick: move |_| export_project(state, CompiledFormat::Docx), "DOCX" }
                    }
                }
                main { class: "center",
                    {match mode {
                        WorkspaceMode::Writing => rsx! { EditorPane {} },
                        WorkspaceMode::Corkboard => rsx! { CorkboardPane {} },
                        WorkspaceMode::Outliner => rsx! { OutlinerPane {} },
                        WorkspaceMode::Plot => rsx! { PlotPane {} },
                        WorkspaceMode::Characters => rsx! { CharactersPane {} },
                        WorkspaceMode::World => rsx! { WorldPane {} },
                        WorkspaceMode::Research => rsx! { ResearchPane {} },
                        WorkspaceMode::History => rsx! { HistoryPane {} },
                    }}
                }
            }
            if show_inspector && sidebar_left {
                Sash { kind: ResizeKind::Inspector, positive: false }
                InspectorPane {}
            }
            if show_sidebar && !sidebar_left {
                Sash { kind: ResizeKind::Sidebar, positive: false }
                Explorer {}
            }
        }
        footer { class: "status",
            div { class: "status-stats",
                span { "Words: {words}" }
                span { "Session: {session_words} · {session_time}" }
                span { class: "goal-stat",
                    "Today: {today_words} / "
                    input {
                        class: "goal-input",
                        title: "Daily word target",
                        value: "{daily_goal}",
                        oninput: move |event| apply_goal(state, event.value(), true),
                        onblur: move |_| state.write().save_all(),
                    }
                }
                span { class: "goal-stat",
                    title: "Words and target for the book being compiled",
                    "{book_title}: {book_words} / "
                    input {
                        class: "goal-input",
                        title: "Word target for the book being compiled",
                        value: "{goal}",
                        oninput: move |event| apply_goal(state, event.value(), false),
                        onblur: move |_| state.write().save_all(),
                    }
                }
            }
            select {
                class: "theme-select",
                title: "Color theme, saved in this project",
                value: "{theme.as_str()}",
                onchange: move |event| {
                    if let Some(next) = Theme::from_id(&event.value()) {
                        state.write().set_theme(next);
                    }
                },
                for item in [Theme::Dark, Theme::Light, Theme::Paper] {
                    option { value: "{item.as_str()}", "{item.label()}" }
                }
            }
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum ResizeKind {
    Sidebar,
    Inspector,
}

#[derive(Clone, Copy)]
struct ResizeDrag {
    kind: ResizeKind,
    origin_x: f64,
    origin_width: u32,
    positive: bool,
}

fn resize_limits(kind: ResizeKind) -> (u32, u32) {
    match kind {
        ResizeKind::Sidebar => (SIDEBAR_WIDTH_MIN, SIDEBAR_WIDTH_MAX),
        ResizeKind::Inspector => (INSPECTOR_WIDTH_MIN, INSPECTOR_WIDTH_MAX),
    }
}

fn publish_layout(
    state: Signal<StudioState>,
    mut chrome: Signal<PanelChrome>,
    mut widths: Signal<PanelWidths>,
) {
    let layout = state.peek().panel_layout();
    chrome.set(layout.chrome());
    widths.set(layout.widths());
}

fn persist_layout(mut state: Signal<StudioState>, layout: PanelLayout) {
    state.write().store_layout(layout);
}

fn persist_chrome(state: Signal<StudioState>, chrome: PanelChrome) {
    let next = state.peek().panel_layout().with_chrome(chrome);
    persist_layout(state, next);
}

fn toggle_sidebar(state: Signal<StudioState>, mut chrome: Signal<PanelChrome>) {
    let mut next = *chrome.peek();
    next.sidebar_visible = !next.sidebar_visible;
    chrome.set(next);
    persist_chrome(state, next);
}

fn toggle_inspector(state: Signal<StudioState>, mut chrome: Signal<PanelChrome>) {
    let mut next = *chrome.peek();
    next.inspector_visible = !next.inspector_visible;
    chrome.set(next);
    persist_chrome(state, next);
}

fn reveal_sidebar(state: Signal<StudioState>, mut chrome: Signal<PanelChrome>) -> bool {
    if chrome.peek().sidebar_visible {
        return false;
    }
    let mut next = *chrome.peek();
    next.sidebar_visible = true;
    chrome.set(next);
    persist_chrome(state, next);
    true
}

fn select_activity(mut state: Signal<StudioState>, chrome: Signal<PanelChrome>, mode: WorkspaceMode) {
    let same = state.peek().mode == mode;
    let visible = chrome.peek().sidebar_visible;
    let focused = state.peek().focus_mode;
    if same && visible && !focused {
        toggle_sidebar(state, chrome);
        return;
    }
    {
        let mut current = state.write();
        current.mode = mode;
        current.focus_mode = false;
    }
    reveal_sidebar(state, chrome);
}

fn flip_sidebar(state: Signal<StudioState>, mut chrome: Signal<PanelChrome>) {
    let mut next = *chrome.peek();
    next.sidebar_side = match next.sidebar_side {
        SidebarSide::Left => SidebarSide::Right,
        SidebarSide::Right => SidebarSide::Left,
    };
    chrome.set(next);
    persist_chrome(state, next);
}

fn finish_resize(
    state: Signal<StudioState>,
    widths: Signal<PanelWidths>,
    mut resize: Signal<Option<ResizeDrag>>,
) {
    if resize.peek().is_none() {
        return;
    }
    resize.set(None);
    let next = state.peek().panel_layout().with_widths(*widths.peek());
    persist_layout(state, next);
}

#[component]
fn Sash(kind: ResizeKind, positive: bool) -> Element {
    let mut resize = use_context::<Signal<Option<ResizeDrag>>>();
    let mut widths = use_context::<Signal<PanelWidths>>();
    let state = use_context::<Signal<StudioState>>();
    rsx! {
        div {
            class: "sash",
            title: "Drag to resize. Double-click to reset.",
            onmousedown: move |event| {
                event.prevent_default();
                let origin_width = match kind {
                    ResizeKind::Sidebar => widths.peek().sidebar,
                    ResizeKind::Inspector => widths.peek().inspector,
                };
                resize.set(Some(ResizeDrag {
                    kind,
                    origin_x: event.client_coordinates().x,
                    origin_width,
                    positive,
                }));
            },
            ondoubleclick: move |event| {
                event.prevent_default();
                let mut next = *widths.peek();
                match kind {
                    ResizeKind::Sidebar => next.sidebar = SIDEBAR_WIDTH_DEFAULT,
                    ResizeKind::Inspector => next.inspector = INSPECTOR_WIDTH_DEFAULT,
                }
                widths.set(next);
                let layout = state.peek().panel_layout().with_widths(next);
                persist_layout(state, layout);
            },
        }
    }
}

#[component]
fn Explorer() -> Element {
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
                button { title: "New note", onclick: move |_| state.write().add_node(NodeKind::Note), "Note" }
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
fn InspectorPane() -> Element {
    let widths = use_context::<Signal<PanelWidths>>();
    let width = widths.read().inspector;
    rsx! {
        aside {
            class: "secondary",
            style: "width: {width}px; flex: 0 0 {width}px;",
            div { class: "view-title", "Inspector" }
            Inspector {}
        }
    }
}

#[component]
fn BinderTreeView() -> Element {
    let state = use_context::<Signal<StudioState>>();
    let snapshot = state.read().clone();
    let roots: Vec<Id> = snapshot
        .binder
        .children(None)
        .into_iter()
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
fn BinderNodeView(id: Id, depth: u32, hits: Rc<HashSet<Id>>) -> Element {
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
            if !label.is_empty() {
                span { class: "binder-label", "{label}" }
            }
        }
        for child in children {
            BinderNodeView { id: child, depth: depth + 1, hits: hits.clone() }
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
            div { class: "breadcrumb", "{kind} › {heading}" }
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

fn read_manuscript_html(state: Signal<StudioState>) {
    spawn(async move {
        // Leave the webview event before running script. Evaluating script
        // inside the input callback deadlocks WebView2 and the window stops.
        tokio::time::sleep(std::time::Duration::from_millis(16)).await;
        let mut eval = document::eval(
            r#"
            const el = document.querySelector('[data-manuscript]');
            dioxus.send(el ? el.innerHTML : '');
            "#,
        );
        if let Ok(html) = eval.recv::<String>().await {
            commit_editor_html(state, html);
        }
    });
}

fn commit_editor_html(mut state: Signal<StudioState>, html: String) {
    if state.read().same_editor_html(&html) {
        return;
    }
    state.write().apply_editor_html(html);
}

#[component]
fn ModeGlyph(mode: WorkspaceMode) -> Element {
    match mode {
        WorkspaceMode::Writing => rsx! {
            svg { class: "glyph", view_box: "0 0 16 16",
                path { d: "M2.5 1.5h6.2L13.5 6v8.5h-11z", fill: "none", stroke: "currentColor", stroke_width: "1.2" }
                path { d: "M8.5 1.8V6H13.2", fill: "none", stroke: "currentColor", stroke_width: "1.2" }
            }
        },
        WorkspaceMode::Corkboard => rsx! {
            svg { class: "glyph", view_box: "0 0 16 16",
                path { d: "M2 2h5v5H2zM9 2h5v5H9zM2 9h5v5H2zM9 9h5v5H9z", fill: "none", stroke: "currentColor", stroke_width: "1.2" }
            }
        },
        WorkspaceMode::Outliner => rsx! {
            svg { class: "glyph", view_box: "0 0 16 16",
                path { d: "M2 3.5h12M2 8h12M2 12.5h12", fill: "none", stroke: "currentColor", stroke_width: "1.2" }
            }
        },
        WorkspaceMode::Plot => rsx! {
            svg { class: "glyph", view_box: "0 0 16 16",
                path { d: "M2 4h12M2 8h12M2 12h12", fill: "none", stroke: "currentColor", stroke_width: "1.2" }
                circle { cx: "5", cy: "4", r: "1.2", fill: "currentColor" }
                circle { cx: "9", cy: "8", r: "1.2", fill: "currentColor" }
                circle { cx: "7", cy: "12", r: "1.2", fill: "currentColor" }
            }
        },
        WorkspaceMode::Characters => rsx! {
            svg { class: "glyph", view_box: "0 0 16 16",
                circle { cx: "8", cy: "5", r: "2.2", fill: "none", stroke: "currentColor", stroke_width: "1.2" }
                path { d: "M3.5 13.2c.8-2.4 2.4-3.6 4.5-3.6s3.7 1.2 4.5 3.6", fill: "none", stroke: "currentColor", stroke_width: "1.2" }
            }
        },
        WorkspaceMode::World => rsx! {
            svg { class: "glyph", view_box: "0 0 16 16",
                circle { cx: "8", cy: "6.2", r: "2.3", fill: "none", stroke: "currentColor", stroke_width: "1.2" }
                path { d: "M8 8.5v4.2M5.2 12.7h5.6", fill: "none", stroke: "currentColor", stroke_width: "1.2" }
            }
        },
        WorkspaceMode::Research => rsx! {
            svg { class: "glyph", view_box: "0 0 16 16",
                path { d: "M3 2.5h7l3 3V13.5H3z", fill: "none", stroke: "currentColor", stroke_width: "1.2" }
                path { d: "M10 2.7V5.5h2.7M5 8h6M5 10.5h4", fill: "none", stroke: "currentColor", stroke_width: "1.2" }
            }
        },
        WorkspaceMode::History => rsx! {
            svg { class: "glyph", view_box: "0 0 16 16",
                path { d: "M8 2.5a5.5 5.5 0 1 1-4.8 2.8", fill: "none", stroke: "currentColor", stroke_width: "1.2" }
                path { d: "M2.2 2.5v3.2h3.2M8 5.2V8l2 1.4", fill: "none", stroke: "currentColor", stroke_width: "1.2" }
            }
        },
    }
}

fn mode_shortcut(mode: WorkspaceMode) -> &'static str {
    match mode {
        WorkspaceMode::Writing => "Ctrl+1",
        WorkspaceMode::Corkboard => "Ctrl+2",
        WorkspaceMode::Outliner => "Ctrl+3",
        WorkspaceMode::Plot => "Ctrl+5",
        WorkspaceMode::Characters => "Ctrl+6",
        WorkspaceMode::World => "Ctrl+7",
        WorkspaceMode::Research => "Ctrl+8",
        WorkspaceMode::History => "Ctrl+4",
    }
}

fn apply_shortcut(
    mut state: Signal<StudioState>,
    chrome: Signal<PanelChrome>,
    widths: Signal<PanelWidths>,
    action: AppShortcut,
) -> bool {
    let open = state.read().is_open();
    match action {
        AppShortcut::Save => {
            if open {
                state.write().save_all();
            }
        }
        AppShortcut::Undo => state.write().undo(),
        AppShortcut::Redo => state.write().redo(),
        AppShortcut::Snapshot => {
            if !open {
                return false;
            }
            state.write().snapshot();
        }
        AppShortcut::Find => focus_find(state, chrome),
        AppShortcut::ToggleFocus => {
            if !open {
                return false;
            }
            let next = !state.read().focus_mode;
            state.write().focus_mode = next;
        }
        AppShortcut::Writing => return set_mode(state, WorkspaceMode::Writing),
        AppShortcut::Corkboard => return set_mode(state, WorkspaceMode::Corkboard),
        AppShortcut::Outliner => return set_mode(state, WorkspaceMode::Outliner),
        AppShortcut::Plot => return set_mode(state, WorkspaceMode::Plot),
        AppShortcut::Characters => return set_mode(state, WorkspaceMode::Characters),
        AppShortcut::World => return set_mode(state, WorkspaceMode::World),
        AppShortcut::Research => return set_mode(state, WorkspaceMode::Research),
        AppShortcut::History => return set_mode(state, WorkspaceMode::History),
        AppShortcut::NewScene => {
            if open {
                state.write().add_node(NodeKind::Scene);
            } else {
                create_project(state, chrome, widths);
            }
        }
        AppShortcut::NewChapter => {
            if !open {
                return false;
            }
            state.write().add_node(NodeKind::Chapter);
        }
        AppShortcut::OpenProject => open_project(state, chrome, widths),
        AppShortcut::ClearOrExitFocus => return clear_or_exit_focus(state),
        AppShortcut::ToggleSidebar => {
            if !open {
                return false;
            }
            if state.read().focus_mode {
                state.write().focus_mode = false;
            }
            toggle_sidebar(state, chrome);
        }
        AppShortcut::ToggleInspector => {
            if !open {
                return false;
            }
            if state.read().focus_mode {
                state.write().focus_mode = false;
            }
            toggle_inspector(state, chrome);
        }
    }
    true
}

fn set_mode(mut state: Signal<StudioState>, mode: WorkspaceMode) -> bool {
    if !state.read().is_open() {
        return false;
    }
    state.write().mode = mode;
    true
}

fn clear_or_exit_focus(mut state: Signal<StudioState>) -> bool {
    if state.read().focus_mode {
        state.write().focus_mode = false;
        return true;
    }
    if state.read().mode == WorkspaceMode::Corkboard && !state.read().cork_query.is_empty() {
        state.write().cork_query.clear();
        return true;
    }
    if !state.read().search.is_empty() {
        state.write().search.clear();
        return true;
    }
    false
}

fn focus_find(mut state: Signal<StudioState>, chrome: Signal<PanelChrome>) {
    if !state.read().is_open() {
        return;
    }
    if state.read().mode == WorkspaceMode::Corkboard {
        focus_dom("cork-filter", false);
        return;
    }
    let mut wait = false;
    if state.read().focus_mode {
        state.write().focus_mode = false;
        wait = true;
    }
    if reveal_sidebar(state, chrome) {
        wait = true;
    }
    focus_dom("binder-search", wait);
}

fn focus_dom(id: &'static str, wait_for_layout: bool) {
    spawn(async move {
        if wait_for_layout {
            tokio::time::sleep(std::time::Duration::from_millis(40)).await;
        }
        let script = format!("document.getElementById('{id}')?.focus(); dioxus.send(true);");
        let mut eval = document::eval(&script);
        let _ = eval.recv::<bool>().await;
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

fn run_format_command(state: Signal<StudioState>, command: &'static str, argument: &'static str) {
    spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(16)).await;
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
            commit_editor_html(state, html);
        }
    });
}

fn run_editor_script(state: Signal<StudioState>, body: &'static str) {
    spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(16)).await;
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
            commit_editor_html(state, html);
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
            if !node.kind.is_root() {
                label { "Label" }
                input {
                    value: "{node.label}",
                    placeholder: "Short tag, such as subplot",
                    oninput: move |event| state.write().update_selected_label(event.value()),
                    onblur: move |_| state.write().save_all(),
                }
                label { "Color" }
                div { class: "swatch-row",
                    for swatch in BINDER_COLORS {
                        button {
                            class: if node.color == swatch.id { "swatch-btn active" } else { "swatch-btn" },
                            style: if swatch.hex.is_empty() { String::new() } else { format!("background: {}; color: #1c1915", swatch.hex) },
                            onclick: move |_| {
                                state.write().update_selected_color(swatch.id.to_string());
                                state.write().save_all();
                            },
                            "{swatch.name}"
                        }
                    }
                }
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
    let chrome = use_context::<Signal<PanelChrome>>();
    let widths = use_context::<Signal<PanelWidths>>();
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
                h2 { "Writing sessions" }
                p { class: "hint", "Sittings saved in this project. Today’s count starts again each local day." }
                WritingSessions {}
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
                                    onclick: move |_| {
                                        state.write().request_backup_restore(item.path.clone());
                                        publish_layout(state, chrome, widths);
                                    },
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
fn WritingSessions() -> Element {
    let sessions = use_context::<Signal<StudioState>>().read().stored_sessions();
    rsx! {
        if sessions.is_empty() {
            p { class: "hint", "None yet. This sitting and today’s words stay on the status bar. A sitting is stored here the next time you open the project, if it added words." }
        } else {
            ul { class: "history-list wide",
                for item in sessions {
                    li { class: "history-item",
                        div {
                            strong { "{item.day}" }
                            span { class: "card-meta", "{item.words} words · {format_elapsed(item.seconds)}" }
                        }
                    }
                }
            }
        }
    }
}

fn create_project(
    mut state: Signal<StudioState>,
    chrome: Signal<PanelChrome>,
    widths: Signal<PanelWidths>,
) {
    let Some(folder) = pick_folder("Choose a folder for the new project") else {
        return;
    };
    state.write().create_project_in(folder);
    publish_layout(state, chrome, widths);
}

fn open_project(
    mut state: Signal<StudioState>,
    chrome: Signal<PanelChrome>,
    widths: Signal<PanelWidths>,
) {
    let Some(folder) = pick_folder("Open a Writing Studio project folder") else {
        return;
    };
    state.write().open_project_at(folder);
    publish_layout(state, chrome, widths);
}

fn export_project(mut state: Signal<StudioState>, format: CompiledFormat) {
    let Some(export) = state.write().prepare_export(format) else {
        return;
    };
    let Some(path) = pick_save_file(export.filter_name, export.extension, export.file_name.clone()) else {
        return;
    };
    match std::fs::write(&path, export.bytes) {
        Ok(()) => {
            state.write().status = format!("Compiled {} to {}", export.kind_label, path.display());
        }
        Err(err) => state.write().status = format!("Could not write {}: {err}", export.kind_label),
    }
}

fn pick_file(title: &str) -> Option<std::path::PathBuf> {
    let title = title.to_string();
    std::thread::spawn(move || rfd::FileDialog::new().set_title(title).pick_file())
        .join()
        .ok()
        .flatten()
}

fn pick_folder(title: &str) -> Option<std::path::PathBuf> {
    let title = title.to_string();
    // The dialog runs on its own thread so the webview thread is not inside
    // the Windows file-dialog message loop. That loop was freezing the window.
    std::thread::spawn(move || rfd::FileDialog::new().set_title(title).pick_folder())
        .join()
        .ok()
        .flatten()
}

fn pick_save_file(filter_name: &str, extension: &str, file_name: String) -> Option<std::path::PathBuf> {
    let filter_name = filter_name.to_string();
    let extension = extension.to_string();
    let title = format!("Compile {filter_name}");
    std::thread::spawn(move || {
        rfd::FileDialog::new()
            .set_title(title)
            .add_filter(filter_name, &[extension])
            .set_file_name(file_name)
            .save_file()
    })
    .join()
    .ok()
    .flatten()
}

fn apply_goal(mut state: Signal<StudioState>, value: String, daily: bool) {
    let Ok(goal) = value.parse::<u32>() else {
        return;
    };
    let unchanged = if daily {
        state.read().daily_goal() == goal
    } else {
        state.read().goal() == goal
    };
    if unchanged {
        return;
    }
    if daily {
        state.write().set_daily_goal(goal);
    } else {
        state.write().set_compile_book_goal(goal);
    }
}

fn CompileBookSelect() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let books = state.read().manuscript_book_choices();
    let current = state.read().compile_book_id();
    if books.is_empty() {
        return rsx! {};
    }
    let value = current.map(|id| id.to_string()).unwrap_or_default();
    rsx! {
        select {
            class: "compile-select",
            title: "Current book. Compile, the word target, and the binder follow this book.",
            value: "{value}",
            onchange: move |event| {
                if let Ok(id) = event.value().parse::<Id>() {
                    state.write().set_compile_book(id);
                }
            },
            for (id, title) in books {
                option { value: "{id}", "{title}" }
            }
        }
    }
}

fn SeriesScopeSelect() -> Element {
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
            option { value: "", "Whole series" }
            for (id, title) in books {
                option { value: "{id}", "{title}" }
            }
        }
    }
}

#[derive(Clone, PartialEq)]
struct NamedId {
    id: Id,
    name: String,
}

#[derive(Clone, PartialEq)]
enum WorldForm {
    Location {
        id: Id,
        name: String,
        summary: String,
        culture_id: String,
    },
    Culture {
        id: Id,
        name: String,
        summary: String,
    },
    Lore {
        id: Id,
        title: String,
        body: String,
        location_id: String,
        culture_id: String,
    },
    Event {
        id: Id,
        title: String,
        when_label: String,
        summary: String,
        location_id: String,
    },
}

#[derive(Clone, PartialEq)]
struct ResearchNoteView {
    id: Id,
    title: String,
    notes: String,
    files: Vec<ResearchFileView>,
}

#[derive(Clone, PartialEq)]
struct ResearchFileView {
    id: Id,
    file_name: String,
}

#[component]
fn ResearchPane() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let snapshot = state.read().clone();
    let notes: Vec<NamedId> = snapshot
        .library
        .items_in_order()
        .into_iter()
        .map(|item| NamedId {
            id: item.id,
            name: item.title.clone(),
        })
        .collect();
    let shown = snapshot
        .selected_research
        .filter(|id| notes.iter().any(|note| note.id == *id))
        .or_else(|| notes.first().map(|note| note.id));
    let note = shown.and_then(|id| research_note(&snapshot.library, id));
    let confirm = snapshot.confirm_research;
    rsx! {
        div { class: "cast-pane",
            aside { class: "cast-list",
                button {
                    class: "primary",
                    onclick: move |_| state.write().add_research_item(),
                    "Add note"
                }
                if notes.is_empty() {
                    p { class: "hint", "No research notes yet." }
                }
                for note in notes {
                    {
                        let id = note.id;
                        let active = shown == Some(id);
                        let class = if active { "cast-name active" } else { "cast-name" };
                        rsx! {
                            button {
                                class: "{class}",
                                key: "{id}",
                                onclick: move |_| state.write().select_research_item(id),
                                "{note.name}"
                            }
                        }
                    }
                }
            }
            div { class: "cast-profile",
                if let Some(note) = note {
                    ResearchNote { note, confirm }
                } else {
                    p { class: "hint", "Research notes and their files stay in this project. Files are copied into attachments/." }
                }
            }
        }
    }
}

fn research_note(library: &writing_studio_research::ResearchLibrary, id: Id) -> Option<ResearchNoteView> {
    let item = library.items.iter().find(|item| item.id == id)?;
    Some(ResearchNoteView {
        id: item.id,
        title: item.title.clone(),
        notes: item.notes.clone(),
        files: library
            .files_for(id)
            .into_iter()
            .map(|file| ResearchFileView {
                id: file.id,
                file_name: file.file_name.clone(),
            })
            .collect(),
    })
}

#[component]
fn ResearchNote(note: ResearchNoteView, confirm: Option<Id>) -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let id = note.id;
    let title = note.title.clone();
    let notes = note.notes.clone();
    let files = note.files.clone();
    let delete_label = if confirm == Some(id) { "Delete note" } else { "Delete" };
    rsx! {
        label { "Title" }
        input {
            value: "{title}",
            oninput: move |event| state.write().set_research_title(id, event.value()),
        }
        label { "Notes" }
        textarea {
            value: "{notes}",
            oninput: move |event| state.write().set_research_notes(id, event.value()),
        }
        SceneLinks { owner: id, kind: BibleLink::Research }
        section { class: "cast-block",
            h3 { "Files" }
            p { class: "hint", "A copy is stored in this project's attachments folder." }
            button {
                onclick: move |_| {
                    let Some(path) = pick_file("Attach a research file") else { return };
                    state.write().attach_research_file(id, path);
                },
                "Attach file"
            }
            if files.is_empty() {
                p { class: "hint", "No files attached." }
            }
            for file in files {
                {
                    let file_id = file.id;
                    let name = file.file_name.clone();
                    let remove_label = if confirm == Some(file_id) { "Remove file" } else { "Remove" };
                    rsx! {
                        div { class: "cast-row", key: "{file_id}",
                            span { "{name}" }
                            button {
                                onclick: move |_| open_research_file(state, file_id),
                                "Open"
                            }
                            button {
                                onclick: move |_| state.write().remove_research_file(file_id),
                                "{remove_label}"
                            }
                        }
                    }
                }
            }
        }
        div { class: "cast-actions",
            button {
                onclick: move |_| state.write().remove_research_item(id),
                "{delete_label}"
            }
        }
    }
}

fn open_research_file(mut state: Signal<StudioState>, file_id: Id) {
    let Some(path) = state.read().research_file_path(file_id) else {
        state.write().status = "That attachment is missing.".into();
        return;
    };
    if !path.is_file() {
        state.write().status = format!("File is not in attachments/: {}", path.display());
        return;
    }
    match open_with_default_app(&path) {
        Ok(()) => state.write().status = format!("Opened {}", path.file_name().and_then(|name| name.to_str()).unwrap_or("file")),
        Err(err) => state.write().status = format!("Could not open file: {err}"),
    }
}

fn open_with_default_app(path: &std::path::Path) -> std::io::Result<()> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", &path.display().to_string()])
            .spawn()?;
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(path).spawn()?;
        return Ok(());
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open").arg(path).spawn()?;
        return Ok(());
    }
    #[allow(unreachable_code)]
    Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "cannot open files on this system"))
}

#[component]
fn WorldPane() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let snapshot = state.read().clone();
    let section = snapshot.world_section;
    let items = world_items(&snapshot.world, section);
    let shown = snapshot
        .selected_world
        .filter(|id| items.iter().any(|item| item.id == *id))
        .or_else(|| items.first().map(|item| item.id));
    let form = shown.and_then(|id| world_form(&snapshot.world, section, id));
    let places = named_places(&snapshot.world);
    let cultures = named_cultures(&snapshot.world);
    let confirm = snapshot.confirm_world;
    let add_label = section.add_label();
    rsx! {
        div { class: "cast-pane",
            aside { class: "cast-list",
                for candidate in WorldSection::ALL {
                    button {
                        class: if section == candidate { "mode active" } else { "mode" },
                        onclick: move |_| state.write().set_world_section(candidate),
                        "{candidate.label()}"
                    }
                }
                button {
                    class: "primary",
                    onclick: move |_| state.write().add_world_item(),
                    "{add_label}"
                }
                if items.is_empty() {
                    p { class: "hint", "Nothing in this section yet." }
                }
                for item in items {
                    {
                        let id = item.id;
                        let active = shown == Some(id);
                        let class = if active { "cast-name active" } else { "cast-name" };
                        rsx! {
                            button {
                                class: "{class}",
                                key: "{id}",
                                onclick: move |_| state.write().select_world_item(id),
                                "{item.name}"
                            }
                        }
                    }
                }
            }
            div { class: "cast-profile",
                if let Some(form) = form {
                    WorldEditor { form, places, cultures, confirm }
                } else {
                    p { class: "hint", "Locations, cultures, lore, and the timeline stay in this project. Pick a section and add an entry." }
                }
            }
        }
    }
}

fn world_items(world: &writing_studio_worldbuilding::World, section: WorldSection) -> Vec<NamedId> {
    match section {
        WorldSection::Locations => world
            .locations_in_order()
            .into_iter()
            .map(|item| NamedId { id: item.id, name: item.name.clone() })
            .collect(),
        WorldSection::Cultures => world
            .cultures_in_order()
            .into_iter()
            .map(|item| NamedId { id: item.id, name: item.name.clone() })
            .collect(),
        WorldSection::Lore => world
            .lore_in_order()
            .into_iter()
            .map(|item| NamedId {
                id: item.id,
                name: item.title.clone(),
            })
            .collect(),
        WorldSection::Timeline => world
            .events_in_order()
            .into_iter()
            .map(|item| {
                let name = if item.when_label.is_empty() {
                    item.title.clone()
                } else {
                    format!("{} · {}", item.when_label, item.title)
                };
                NamedId { id: item.id, name }
            })
            .collect(),
    }
}

fn world_form(world: &writing_studio_worldbuilding::World, section: WorldSection, id: Id) -> Option<WorldForm> {
    match section {
        WorldSection::Locations => {
            let place = world.locations.iter().find(|item| item.id == id)?;
            Some(WorldForm::Location {
                id: place.id,
                name: place.name.clone(),
                summary: place.summary.clone(),
                culture_id: optional_text(place.culture_id),
            })
        }
        WorldSection::Cultures => {
            let culture = world.cultures.iter().find(|item| item.id == id)?;
            Some(WorldForm::Culture {
                id: culture.id,
                name: culture.name.clone(),
                summary: culture.summary.clone(),
            })
        }
        WorldSection::Lore => {
            let entry = world.lore.iter().find(|item| item.id == id)?;
            Some(WorldForm::Lore {
                id: entry.id,
                title: entry.title.clone(),
                body: entry.body.clone(),
                location_id: optional_text(entry.location_id),
                culture_id: optional_text(entry.culture_id),
            })
        }
        WorldSection::Timeline => {
            let event = world.events.iter().find(|item| item.id == id)?;
            Some(WorldForm::Event {
                id: event.id,
                title: event.title.clone(),
                when_label: event.when_label.clone(),
                summary: event.summary.clone(),
                location_id: optional_text(event.location_id),
            })
        }
    }
}

fn named_places(world: &writing_studio_worldbuilding::World) -> Vec<NamedId> {
    world
        .locations_in_order()
        .into_iter()
        .map(|item| NamedId { id: item.id, name: item.name.clone() })
        .collect()
}

fn named_cultures(world: &writing_studio_worldbuilding::World) -> Vec<NamedId> {
    world
        .cultures_in_order()
        .into_iter()
        .map(|item| NamedId { id: item.id, name: item.name.clone() })
        .collect()
}

fn optional_text(id: Option<Id>) -> String {
    id.map(|id| id.to_string()).unwrap_or_default()
}

fn parse_optional_id(value: &str) -> Option<Id> {
    let value = value.trim();
    if value.is_empty() { None } else { value.parse().ok() }
}

#[component]
fn WorldEditor(form: WorldForm, places: Vec<NamedId>, cultures: Vec<NamedId>, confirm: Option<Id>) -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let id = match &form {
        WorldForm::Location { id, .. }
        | WorldForm::Culture { id, .. }
        | WorldForm::Lore { id, .. }
        | WorldForm::Event { id, .. } => *id,
    };
    let delete_label = if confirm == Some(id) { "Delete entry" } else { "Delete" };
    let link = match &form {
        WorldForm::Location { .. } => BibleLink::Location,
        WorldForm::Culture { .. } => BibleLink::Culture,
        WorldForm::Lore { .. } => BibleLink::Lore,
        WorldForm::Event { .. } => BibleLink::Timeline,
    };
    rsx! {
        match form {
            WorldForm::Location { id, name, summary, culture_id } => rsx! {
                label { "Name" }
                input {
                    value: "{name}",
                    oninput: move |event| state.write().set_location_name(id, event.value()),
                }
                label { "Description" }
                textarea {
                    value: "{summary}",
                    oninput: move |event| state.write().set_location_summary(id, event.value()),
                }
                label { "Culture" }
                select {
                    value: "{culture_id}",
                    onchange: move |event| state.write().set_location_culture(id, parse_optional_id(&event.value())),
                    option { value: "", "No culture" }
                    for culture in cultures {
                        option { value: "{culture.id}", "{culture.name}" }
                    }
                }
            },
            WorldForm::Culture { id, name, summary } => rsx! {
                label { "Name" }
                input {
                    value: "{name}",
                    oninput: move |event| state.write().set_culture_name(id, event.value()),
                }
                label { "Description" }
                textarea {
                    value: "{summary}",
                    oninput: move |event| state.write().set_culture_summary(id, event.value()),
                }
            },
            WorldForm::Lore { id, title, body, location_id, culture_id } => rsx! {
                label { "Title" }
                input {
                    value: "{title}",
                    oninput: move |event| state.write().set_lore_title(id, event.value()),
                }
                label { "Lore" }
                textarea {
                    value: "{body}",
                    oninput: move |event| state.write().set_lore_body(id, event.value()),
                }
                label { "Location" }
                select {
                    value: "{location_id}",
                    onchange: move |event| state.write().set_lore_location(id, parse_optional_id(&event.value())),
                    option { value: "", "No location" }
                    for place in places.clone() {
                        option { value: "{place.id}", "{place.name}" }
                    }
                }
                label { "Culture" }
                select {
                    value: "{culture_id}",
                    onchange: move |event| state.write().set_lore_culture(id, parse_optional_id(&event.value())),
                    option { value: "", "No culture" }
                    for culture in cultures.clone() {
                        option { value: "{culture.id}", "{culture.name}" }
                    }
                }
            },
            WorldForm::Event { id, title, when_label, summary, location_id } => rsx! {
                label { "When" }
                input {
                    title: "A label such as Year 12 or the spring before the war",
                    value: "{when_label}",
                    oninput: move |event| state.write().set_event_when(id, event.value()),
                }
                label { "Title" }
                input {
                    value: "{title}",
                    oninput: move |event| state.write().set_event_title(id, event.value()),
                }
                label { "What happened" }
                textarea {
                    value: "{summary}",
                    oninput: move |event| state.write().set_event_summary(id, event.value()),
                }
                label { "Location" }
                select {
                    value: "{location_id}",
                    onchange: move |event| state.write().set_event_location(id, parse_optional_id(&event.value())),
                    option { value: "", "No location" }
                    for place in places.clone() {
                        option { value: "{place.id}", "{place.name}" }
                    }
                }
                div { class: "cast-actions",
                    button {
                        onclick: move |_| state.write().shift_world_event(id, true),
                        "Earlier"
                    }
                    button {
                        onclick: move |_| state.write().shift_world_event(id, false),
                        "Later"
                    }
                }
            },
        }
        SceneLinks { owner: id, kind: link }
        div { class: "cast-actions",
            button {
                onclick: move |_| state.write().remove_world_item(id),
                "{delete_label}"
            }
        }
    }
}

#[derive(Clone, PartialEq)]
struct CastListItem {
    id: Id,
    name: String,
}

#[derive(Clone, PartialEq)]
struct MotiveView {
    id: Id,
    body: String,
}

#[derive(Clone, PartialEq)]
struct RelationView {
    id: Id,
    to_id: Id,
    kind: String,
}

#[derive(Clone, PartialEq)]
struct ProfileView {
    id: Id,
    name: String,
    role: String,
    summary: String,
    motivations: Vec<MotiveView>,
    relationships: Vec<RelationView>,
}

#[component]
fn CharactersPane() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let snapshot = state.read().clone();
    let people: Vec<CastListItem> = snapshot
        .cast
        .characters_in_order()
        .into_iter()
        .map(|character| CastListItem {
            id: character.id,
            name: character.name.clone(),
        })
        .collect();
    let shown = snapshot.selected_character.filter(|id| people.iter().any(|person| person.id == *id))
        .or_else(|| people.first().map(|person| person.id));
    let profile = shown.and_then(|id| profile_view(&snapshot.cast, id));
    let others: Vec<CastListItem> = people
        .iter()
        .filter(|person| Some(person.id) != shown)
        .cloned()
        .collect();
    let confirm = snapshot.confirm_character;
    rsx! {
        div { class: "cast-pane",
            aside { class: "cast-list",
                button {
                    class: "primary",
                    onclick: move |_| state.write().add_character(),
                    "Add character"
                }
                if people.is_empty() {
                    p { class: "hint", "No characters yet." }
                }
                for person in people {
                    {
                        let id = person.id;
                        let active = shown == Some(id);
                        let class = if active { "cast-name active" } else { "cast-name" };
                        rsx! {
                            button {
                                class: "{class}",
                                key: "{id}",
                                onclick: move |_| state.write().select_character(id),
                                "{person.name}"
                            }
                        }
                    }
                }
            }
            div { class: "cast-profile",
                if let Some(profile) = profile {
                    CharacterProfile { profile, others, confirm }
                } else {
                    p { class: "hint", "Add a character to write a profile, motivations, and relationships. They stay in this project." }
                }
            }
        }
    }
}

fn profile_view(cast: &writing_studio_characters::Cast, id: Id) -> Option<ProfileView> {
    let character = cast.characters.iter().find(|character| character.id == id)?;
    Some(ProfileView {
        id: character.id,
        name: character.name.clone(),
        role: character.role.clone(),
        summary: character.summary.clone(),
        motivations: cast
            .motivations_for(id)
            .into_iter()
            .map(|item| MotiveView {
                id: item.id,
                body: item.body.clone(),
            })
            .collect(),
        relationships: cast
            .relationships_from(id)
            .into_iter()
            .map(|item| RelationView {
                id: item.id,
                to_id: item.to_id,
                kind: item.kind.clone(),
            })
            .collect(),
    })
}

#[component]
fn CharacterProfile(profile: ProfileView, others: Vec<CastListItem>, confirm: Option<Id>) -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let id = profile.id;
    let name = profile.name.clone();
    let role = profile.role.clone();
    let summary = profile.summary.clone();
    let motivations = profile.motivations.clone();
    let relationships = profile.relationships.clone();
    let delete_label = if confirm == Some(id) {
        "Delete character"
    } else {
        "Delete"
    };
    rsx! {
        label { "Name" }
        input {
            title: "Character name",
            value: "{name}",
            oninput: move |event| state.write().set_character_name(id, event.value()),
        }
        label { "Role" }
        input {
            title: "Role in the story",
            placeholder: "Protagonist, rival, mentor…",
            value: "{role}",
            oninput: move |event| state.write().set_character_role(id, event.value()),
        }
        label { "Profile" }
        textarea {
            title: "Who they are",
            value: "{summary}",
            oninput: move |event| state.write().set_character_summary(id, event.value()),
        }
        section { class: "cast-block",
            h3 { "Motivations" }
            p { class: "hint", "What they want, fear, or need. Order is the order you keep them in." }
            button {
                onclick: move |_| state.write().add_motivation(id),
                "Add motivation"
            }
            if motivations.is_empty() {
                p { class: "hint", "No motivations yet." }
            }
            for motive in motivations {
                {
                    let motive_id = motive.id;
                    let body = motive.body.clone();
                    rsx! {
                        div { class: "cast-row", key: "{motive_id}",
                            input {
                                title: "Motivation",
                                value: "{body}",
                                oninput: move |event| state.write().set_motivation_body(motive_id, event.value()),
                            }
                            button {
                                title: "Move earlier",
                                onclick: move |_| state.write().shift_motivation(motive_id, true),
                                "Earlier"
                            }
                            button {
                                title: "Move later",
                                onclick: move |_| state.write().shift_motivation(motive_id, false),
                                "Later"
                            }
                            button {
                                onclick: move |_| state.write().remove_motivation(motive_id),
                                "Remove"
                            }
                        }
                    }
                }
            }
        }
        section { class: "cast-block",
            h3 { "Relationships" }
            p { class: "hint", "How this character is connected to someone else in the cast." }
            button {
                onclick: move |_| state.write().add_relationship(id),
                "Add relationship"
            }
            if relationships.is_empty() {
                p { class: "hint", "No relationships yet." }
            }
            for relation in relationships {
                {
                    let relation_id = relation.id;
                    let kind = relation.kind.clone();
                    let to_id = relation.to_id;
                    let choices = others.clone();
                    rsx! {
                        div { class: "cast-row", key: "{relation_id}",
                            input {
                                title: "Relationship",
                                placeholder: "sibling, rival, owes a debt…",
                                value: "{kind}",
                                oninput: move |event| state.write().set_relationship_kind(relation_id, event.value()),
                            }
                            select {
                                title: "Other character",
                                value: "{to_id}",
                                onchange: move |event| {
                                    if let Some(target) = event.value().parse().ok() {
                                        state.write().set_relationship_target(relation_id, target);
                                    }
                                },
                                for person in choices {
                                    option { value: "{person.id}", "{person.name}" }
                                }
                            }
                            button {
                                onclick: move |_| state.write().remove_relationship(relation_id),
                                "Remove"
                            }
                        }
                    }
                }
            }
        }
        SceneLinks { owner: id, kind: BibleLink::Character }
        div { class: "cast-actions",
            button {
                onclick: move |_| state.write().remove_character(id),
                "{delete_label}"
            }
        }
    }
}

#[derive(Clone, PartialEq)]
struct PlotPointView {
    id: Id,
    title: String,
    summary: String,
    scene_id: Option<Id>,
}

#[derive(Clone, PartialEq)]
struct PlotLineView {
    id: Id,
    title: String,
    color: String,
    points: Vec<PlotPointView>,
}

#[component]
fn PlotPane() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let snapshot = state.read().clone();
    let scenes = scene_choices(&snapshot.binder);
    let confirm = snapshot.confirm_plotline;
    let lines: Vec<PlotLineView> = snapshot
        .plot
        .lines_in_order()
        .into_iter()
        .map(|line| PlotLineView {
            id: line.id,
            title: line.title.clone(),
            color: line.color.clone(),
            points: snapshot
                .plot
                .points_on(line.id)
                .into_iter()
                .map(|point| PlotPointView {
                    id: point.id,
                    title: point.title.clone(),
                    summary: point.summary.clone(),
                    scene_id: point.scene_id,
                })
                .collect(),
        })
        .collect();
    rsx! {
        div { class: "plot-pane",
            div { class: "plot-toolbar",
                button {
                    class: "primary",
                    onclick: move |_| state.write().add_plotline(),
                    "Add plotline"
                }
            }
            p { class: "hint", "Each row is a plotline. Plot points stay in order on that line and can open a manuscript scene. Titles save with the project." }
            if lines.is_empty() {
                p { class: "hint", "No plotlines yet." }
            }
            for line in lines {
                PlotLineRow { line, scenes: scenes.clone(), confirm, binder_scenes: linked_scene_labels(&snapshot, &scenes) }
            }
        }
    }
}

#[derive(Clone, PartialEq)]
struct LinkedScene {
    id: Id,
    label: String,
}

fn linked_scene_labels(snapshot: &StudioState, scenes: &[(Id, String)]) -> Vec<LinkedScene> {
    snapshot
        .plot
        .points
        .iter()
        .filter_map(|point| point.scene_id)
        .filter(|id| !scenes.iter().any(|(scene, _)| scene == id))
        .map(|id| LinkedScene {
            id,
            label: scene_caption(&snapshot.binder, id),
        })
        .collect()
}

#[component]
fn PlotLineRow(line: PlotLineView, scenes: Vec<(Id, String)>, confirm: Option<Id>, binder_scenes: Vec<LinkedScene>) -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let line_id = line.id;
    let color = line.color.clone();
    let title = line.title.clone();
    let stripe = binder_color_hex(&color);
    let border = if stripe.is_empty() {
        String::new()
    } else {
        format!("border-top-color: {stripe}")
    };
    let delete_label = if confirm == Some(line_id) {
        "Delete plotline"
    } else {
        "Delete"
    };
    let points = line.points.clone();
    rsx! {
        section { class: "plot-row", key: "{line_id}",
            div { class: "plot-row-head",
                input {
                    class: "plot-line-title",
                    title: "Plotline title",
                    value: "{title}",
                    oninput: move |event| {
                        state.write().set_plotline_title(line_id, event.value());
                    },
                }
                select {
                    class: "plot-color",
                    title: "Plotline color",
                    value: "{color}",
                    onchange: move |event| {
                        state.write().set_plotline_color(line_id, event.value());
                    },
                    for swatch in BINDER_COLORS {
                        option { value: "{swatch.id}", "{swatch.name}" }
                    }
                }
                button {
                    onclick: move |_| state.write().add_plot_point(line_id),
                    "Add point"
                }
                button {
                    onclick: move |_| state.write().remove_plotline(line_id),
                    "{delete_label}"
                }
            }
            div { class: "plot-points",
                if points.is_empty() {
                    p { class: "hint", "No plot points on this line." }
                }
                for point in points {
                    PlotPointCard { point, scenes: scenes.clone(), border: border.clone(), extras: binder_scenes.clone() }
                }
            }
        }
    }
}

#[component]
fn PlotPointCard(point: PlotPointView, scenes: Vec<(Id, String)>, border: String, extras: Vec<LinkedScene>) -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let point_id = point.id;
    let title = point.title.clone();
    let summary = point.summary.clone();
    let scene_id = point.scene_id;
    let scene_value = scene_id.map(|id| id.to_string()).unwrap_or_default();
    let mut options = scenes;
    if let Some(id) = scene_id {
        if let Some(extra) = extras.into_iter().find(|extra| extra.id == id) {
            options.push((extra.id, extra.label));
        }
    }
    rsx! {
        article { class: "plot-card", style: "{border}", key: "{point_id}",
            input {
                title: "Plot point title",
                value: "{title}",
                oninput: move |event| {
                    state.write().set_plot_point_title(point_id, event.value());
                },
            }
            textarea {
                title: "What happens at this point",
                value: "{summary}",
                oninput: move |event| {
                    state.write().set_plot_point_summary(point_id, event.value());
                },
            }
            select {
                title: "Linked manuscript scene",
                value: "{scene_value}",
                onchange: move |event| {
                    state.write().set_plot_point_scene(point_id, parse_scene_id(&event.value()));
                },
                option { value: "", "No scene" }
                for (id, label) in options {
                    option { value: "{id}", "{label}" }
                }
            }
            div { class: "plot-card-actions",
                button {
                    title: "Move earlier on this plotline",
                    onclick: move |_| state.write().shift_plot_point(point_id, true),
                    "Earlier"
                }
                button {
                    title: "Move later on this plotline",
                    onclick: move |_| state.write().shift_plot_point(point_id, false),
                    "Later"
                }
                button {
                    disabled: scene_id.is_none(),
                    onclick: move |_| {
                        if let Some(id) = scene_id {
                            state.write().open_linked_scene(id);
                        }
                    },
                    "Open"
                }
                button {
                    onclick: move |_| state.write().remove_plot_point(point_id),
                    "Remove"
                }
            }
        }
    }
}

#[component]
fn SceneLinks(owner: Id, kind: BibleLink) -> Element {
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
                    option { value: "", "Link a scene…" }
                    for (scene_id, label) in choices {
                        option { value: "{scene_id}", "{label}" }
                    }
                }
            }
        }
    }
}

fn scene_choices(binder: &writing_studio_project::BinderTree) -> Vec<(Id, String)> {
    let mut scenes: Vec<(Id, String)> = binder
        .nodes
        .iter()
        .filter(|node| node.kind == NodeKind::Scene && !binder.is_in_trash(node.id))
        .map(|node| (node.id, scene_caption(binder, node.id)))
        .collect();
    scenes.sort_by(|left, right| left.1.to_lowercase().cmp(&right.1.to_lowercase()));
    scenes
}

fn scene_caption(binder: &writing_studio_project::BinderTree, id: Id) -> String {
    binder.scene_caption(id)
}

fn parse_scene_id(value: &str) -> Option<Id> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        value.parse().ok()
    }
}

#[component]
fn CorkboardPane() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let snapshot = state.read().clone();
    let query = snapshot.cork_query.clone();
    let group = snapshot.cork_group;
    let scope = snapshot.series_scope();
    let cards: Vec<_> = cards_from_binder(&snapshot.binder, |id| words_for_document(&snapshot, id))
        .into_iter()
        .filter(|card| {
            scope
                .map(|book| snapshot.binder.book_of(card.node_id) == Some(book))
                .unwrap_or(true)
        })
        .filter(|card| card_matches(card, &query))
        .collect();
    let sections = group_cards(&cards, group);
    rsx! {
        div { class: "cork-pane",
            div { class: "cork-tools",
                SeriesScopeSelect {}
                input {
                    id: "cork-filter",
                    class: "cork-filter",
                    placeholder: "Filter scenes…",
                    title: "Ctrl+F",
                    value: "{query}",
                    oninput: move |event| state.write().cork_query = event.value(),
                }
                for candidate in CorkGroup::ALL {
                    button {
                        class: if group == candidate { "mode active" } else { "mode" },
                        onclick: move |_| state.write().cork_group = candidate,
                        "{candidate.label()}"
                    }
                }
            }
            p { class: "hint", "Drag a card onto another scene to place it just after that scene. Colors come from the binder." }
            if sections.is_empty() {
                p { class: "hint", "No scenes match this filter." }
            }
            for section in sections {
                section { class: "cork-section",
                    h3 { "{section.title}" }
                    div { class: "corkboard",
                        for card in section.cards {
                            CorkCardView { card }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn CorkCardView(card: writing_studio_corkboard::CorkCard) -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let id = card.node_id;
    let hex = binder_color_hex(&card.color);
    let dragging_id = state.read().dragging;
    let dragging = dragging_id == Some(id);
    let drop_target = state.read().drop_target == Some(id);
    let drop_ok = dragging_id.is_some_and(|dragged| state.read().binder.drop_allowed(dragged, id));
    let mut class = String::from("card");
    if dragging {
        class.push_str(" dragging");
    }
    if drop_target && drop_ok {
        class.push_str(" drop-target");
    }
    if drop_target && !drop_ok {
        class.push_str(" drop-invalid");
    }
    let stripe = if hex.is_empty() {
        String::new()
    } else {
        format!("border-top: 6px solid {hex}")
    };
    rsx! {
        button {
            class: "{class}",
            style: "{stripe}",
            draggable: true,
            onclick: move |_| {
                let suppress = state.write().ignore_next_click;
                state.write().ignore_next_click = false;
                if suppress {
                    return;
                }
                state.write().select_node(id);
                state.write().mode = WorkspaceMode::Writing;
            },
            ondragstart: move |event| {
                event.stop_propagation();
                state.write().begin_drag(id);
                state.write().ignore_next_click = true;
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
            h3 { "{card.title}" }
            p {
                if card.synopsis.is_empty() {
                    "No synopsis yet."
                } else {
                    "{card.synopsis}"
                }
            }
            span { class: "card-meta", "{card.status} · {card.word_count} words" }
            if !card.label.is_empty() {
                span { class: "card-label", "{card.label}" }
            }
        }
    }
}

#[component]
fn OutlinerPane() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let snapshot = state.read().clone();
    let mut books: Vec<OwnedOutlineBook> = Vec::new();
    let scope = snapshot.series_scope();
    for line in outline_rows(&snapshot.binder) {
        if scope.is_some_and(|book| line.book_id != book) {
            continue;
        }
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
            div { class: "cork-tools",
                SeriesScopeSelect {}
            }
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

fn node_in_series_scope(
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

fn binder_matches(
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

fn binder_item_class(
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
