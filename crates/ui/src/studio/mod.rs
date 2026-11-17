//! The window. Each area is its own module; they share names through
//! `use super::*`, so an item one area uses from another is `pub(super)`.

use std::collections::HashSet;
use std::rc::Rc;

use dioxus::prelude::*;
use writing_studio_core::Id;
use writing_studio_corkboard::{card_matches, cards_from_binder, group_cards, CorkGroup};
use writing_studio_document::{
    DiffOp, Highlight, NoteKind, ParagraphChange, ReviewItem, ReviewKind, ReviewTarget,
    TrackedChange,
};
use writing_studio_editor::EditorSession;
use writing_studio_manuscript::{outline_rows, row_word_count};
use writing_studio_project::{
    binder_color_hex, drag_width, format_elapsed, BinderNode, NodeKind, PanelChrome, PanelLayout,
    PanelWidths, SidebarSide, Theme, BINDER_COLORS, INSPECTOR_WIDTH_DEFAULT, INSPECTOR_WIDTH_MAX,
    INSPECTOR_WIDTH_MIN, SIDEBAR_WIDTH_DEFAULT, SIDEBAR_WIDTH_MAX, SIDEBAR_WIDTH_MIN,
};
use writing_studio_diagnostics::{BookContext, SceneFacts};
use writing_studio_search::matches_query;
use writing_studio_spellcheck::UnknownWord;

use crate::shortcuts::{app_shortcut, AppShortcut};
use crate::state::{Beat, BibleLink, CompiledFormat, Earlier, StudioState, WorkspaceMode, WorldSection};

mod layout;
mod binder;
mod inspector;
mod editor;
mod commands;
mod history;
mod dialogs;
mod shared;
mod world;
mod research;
mod characters;
mod plot;
mod corkboard;
mod outliner;

use layout::*;
use binder::*;
use inspector::*;
use editor::*;
use commands::*;
use history::*;
use dialogs::*;
use shared::*;
use world::*;
use research::*;
use characters::*;
use plot::*;
use corkboard::*;
use outliner::*;

const STYLES: &str = include_str!("../../assets/style.css");
const REVIEW_JS: &str = include_str!("../../assets/review.js");
const SPELL_JS: &str = include_str!("../../assets/spell.js");
/// Unknown words shown as buttons before the rest are counted as "more".
const SPELL_WORDS_SHOWN: usize = 60;

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
    // Coming back to the window is when files changed by a text editor or by
    // git are read again, before the writer types over them.
    dioxus::desktop::use_wry_event_handler(move |event, _| {
        match event {
            dioxus::desktop::tao::event::Event::WindowEvent {
                event: dioxus::desktop::WindowEvent::Focused(true),
                ..
            } => state.write().check_files_on_disk(),
            // Closing commits for the next sync, without the network.
            dioxus::desktop::tao::event::Event::WindowEvent {
                event: dioxus::desktop::WindowEvent::CloseRequested,
                ..
            } => state.write().commit_on_close(),
            _ => {}
        }
    });
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
                p { class: "hint", "A project is a folder of plain files that any browser can open. Everything works offline." }
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
    let book_words = state.read().compile_book_word_count();
    let book_title = state.read().compile_book_title();
    let goal = state.read().goal();
    let daily_goal = state.read().daily_goal();
    let today_words = state.read().current_book_words_today();
    let session_words = state.read().current_book_words_this_session();
    let session_time = format_elapsed(state.read().session_shown_secs);
    let unsaved = state.read().has_unsaved();
    let theme = state.read().theme();
    let spelling_value = state.read().spelling_language().unwrap_or_default();
    let syncing = state.read().syncing;
    let connected = state.read().sync_connected();
    let spelling_choices = state.read().spelling_choices();
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
                        if connected {
                            button {
                                title: "Send this machine's changes and bring in the others', through git",
                                disabled: syncing,
                                onclick: move |_| run_sync(state),
                                if syncing { "Syncing…" } else { "Sync" }
                            }
                        }
                        button {
                            title: "Export the whole project as a folder of Markdown files, one for each scene and entry, for another writing program",
                            onclick: move |_| {
                                if let Some(parent) = pick_folder("Choose where to put the Markdown folder") {
                                    state.write().export_markdown_folder(parent);
                                }
                            },
                            "Markdown folder"
                        }
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
                title: "Spellcheck dictionary, saved in this project. More dictionaries can go in the project's dictionaries folder.",
                value: "{spelling_value}",
                onchange: move |event| {
                    let value = event.value();
                    state.write().set_spelling((!value.is_empty()).then_some(value));
                },
                option { value: "", selected: spelling_value.is_empty(), "Spelling: off" }
                for item in spelling_choices {
                    option { value: "{item.id}", selected: item.id == spelling_value, "Spelling: {item.label}" }
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
