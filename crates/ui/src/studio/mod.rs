//! The window. Each area is its own module; they share names through
//! `use super::*`, so an item one area uses from another is `pub(super)`.

use std::collections::HashSet;

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
    binder_color_hex, drag_width, format_elapsed, BinderNode, DropPlace, NodeKind, PanelChrome, PanelLayout,
    PanelWidths, ProseFont, Theme, BINDER_COLORS, INSPECTOR_WIDTH_DEFAULT, INSPECTOR_WIDTH_MAX,
    INSPECTOR_WIDTH_MIN, SIDEBAR_WIDTH_DEFAULT, SIDEBAR_WIDTH_MAX, SIDEBAR_WIDTH_MIN,
};
use writing_studio_diagnostics::{BookContext, SceneFacts};
use writing_studio_search::matches_query;
use writing_studio_spellcheck::UnknownWord;

use crate::shortcuts::{app_shortcut, AppShortcut, IN_THE_MANUSCRIPT, LISTED};
use crate::state::{
    Beat, BibleLink, CompiledFormat, Earlier, FoldedPanel, FrameMenu, ReviewTab, SceneCard, SceneGroup, StudioState, WorkspaceMode,
    WorldSection,
};

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
mod library;

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
use library::*;

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
    use_context_provider(|| Signal::new(initial_window_width()));
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
    let window_width = use_context::<Signal<WindowWidth>>();
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
            dioxus::desktop::tao::event::Event::WindowEvent {
                event: dioxus::desktop::WindowEvent::Resized(size),
                ..
            } => track_window_width(window_width, *size),
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
    let show_shortcuts = state.read().show_shortcuts;
    let theme = state.read().theme();
    let font = state.read().prose_font();

    rsx! {
        div {
            class: "shell theme-{theme.as_str()} prose-{font.as_str()}",
            // Focusable by a click, so keys reach the handler, but not a stop for Tab.
            tabindex: -1,
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
                let fold = fold_for(window_width.peek().0, *widths.peek());
                if apply_shortcut(state, chrome, widths, fold, action) {
                    event.prevent_default();
                }
            },
            if open { Workspace {} } else { Welcome {} }
            if show_shortcuts { ShortcutsDialog {} }
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
    let mode = state.read().mode;
    let focus = state.read().focus_mode;
    let unsaved = state.read().has_unsaved();
    let overlay = state.read().overlay;
    let menu_open = state.read().open_menu.is_some();
    let chrome = use_context::<Signal<PanelChrome>>();
    let arrangement = *chrome.read();
    let fold = current_fold();
    let inspector_mode = mode.shows_inspector();
    let list_mode = mode.shows_binder();
    // The library is on screen in every mode; the scene list where scenes
    // are chosen. Both fold away together in a narrow window.
    let show_library = !focus && arrangement.sidebar_visible && !fold.binder;
    let show_list = show_library && list_mode;
    let show_inspector =
        !focus && arrangement.inspector_visible && inspector_mode && !fold.inspector;
    let overlay_library = !focus && fold.binder && overlay == Some(FoldedPanel::Binder);
    let overlay_inspector =
        !focus && fold.inspector && inspector_mode && overlay == Some(FoldedPanel::Inspector);
    let library_on = if fold.binder { overlay_library } else { arrangement.sidebar_visible };
    let inspector_on = inspector_mode && if fold.inspector { overlay_inspector } else { arrangement.inspector_visible };
    let manuscript_view = matches!(mode, WorkspaceMode::Writing | WorkspaceMode::Corkboard | WorkspaceMode::Outliner);
    let bar_title = {
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
        if menu_open {
            div { class: "menu-backdrop", onclick: move |_| state.write().open_menu = None }
        }
        div { class: if focus { "workbench focus" } else { "workbench" },
            // A panel over the page comes where it shows, so Tab reaches it
            // in the order it is seen.
            if overlay_library {
                div { class: "overlay-backdrop", onclick: move |_| state.write().overlay = None }
                div { class: "overlay overlay-left",
                    Library {}
                    if list_mode { SceneList {} }
                }
            }
            if show_library { Library {} }
            if show_list {
                SceneList {}
                Sash { kind: ResizeKind::Sidebar, positive: true }
            }
            div { class: "editor-group",
                if !focus {
                    header { class: "work-bar",
                        IconButton {
                            name: "Library (Ctrl+Alt+B)",
                            open: library_on,
                            onclick: move |_| toggle_binder(state, chrome, fold),
                            path { d: "M1.5 2.5h13v11h-13zM6 2.5v11", fill: "none", stroke: "currentColor", stroke_width: "1.2" }
                        }
                        if manuscript_view {
                            div { class: "view-switch", role: "group", aria_label: "Show the scenes as",
                                for (view, label) in [(WorkspaceMode::Writing, "Page"), (WorkspaceMode::Corkboard, "Cards"), (WorkspaceMode::Outliner, "Outline")] {
                                    button {
                                        class: if mode == view { "view-option active" } else { "view-option" },
                                        aria_pressed: "{mode == view}",
                                        title: "{view.label()} ({mode_shortcut(view)})",
                                        onclick: move |_| {
                                            state.write().choose_mode(view);
                                        },
                                        "{label}"
                                    }
                                }
                            }
                        }
                        span { class: "work-title", title: "{bar_title}", "{bar_title}" }
                        if unsaved {
                            span { class: "tab-dirty", role: "img", aria_label: "Unsaved changes", title: "Unsaved changes" }
                        }
                        div { class: "work-spacer" }
                        IconButton {
                            name: "Focus (Ctrl+Shift+F)",
                            open: false,
                            onclick: move |_| {
                                let next = !state.read().focus_mode;
                                state.write().focus_mode = next;
                            },
                            path { d: "M2 6V2h4M10 2h4v4M14 10v4h-4M6 14H2v-4", fill: "none", stroke: "currentColor", stroke_width: "1.2" }
                        }
                        if inspector_mode {
                            IconButton {
                                name: "Inspector (Ctrl+Alt+I)",
                                open: inspector_on,
                                onclick: move |_| toggle_inspector_panel(state, chrome, fold),
                                path { d: "M1.5 2.5h13v11h-13zM10 2.5v11", fill: "none", stroke: "currentColor", stroke_width: "1.2" }
                            }
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
            if show_inspector {
                Sash { kind: ResizeKind::Inspector, positive: false }
                InspectorPane {}
            }
            if overlay_inspector {
                div { class: "overlay-backdrop", onclick: move |_| state.write().overlay = None }
                div { class: "overlay overlay-right", InspectorPane {} }
            }
        }
        if focus {
            // Focus mode keeps only the page and how many words are on it.
            div { class: "focus-count", aria_live: "polite", "{state.read().word_count()} words" }
        } else {
            StatusBar {}
        }
    }
}

/// Every shortcut, from Help or Ctrl+/. Escape or Close puts it away.
#[component]
fn ShortcutsDialog() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    rsx! {
        div { class: "dialog-backdrop", onclick: move |_| state.write().show_shortcuts = false }
        div { class: "dialog", role: "dialog", aria_modal: "true", aria_labelledby: "shortcuts-title",
            div { class: "dialog-head",
                h2 { id: "shortcuts-title", "Keyboard shortcuts" }
                button {
                    onmounted: move |event| async move {
                        let _ = event.set_focus(true).await;
                    },
                    onclick: move |_| state.write().show_shortcuts = false,
                    "Close"
                }
            }
            table { class: "shortcut-table",
                tbody {
                    for item in LISTED {
                        tr {
                            td { "{item.what}" }
                            td { kbd { "{item.chord()}" } }
                        }
                    }
                    tr {
                        td { "Close a menu or a panel, cancel a waiting delete, leave focus mode, or clear a search" }
                        td { kbd { "Escape" } }
                    }
                }
            }
            h3 { "In the manuscript" }
            table { class: "shortcut-table",
                tbody {
                    for (what, chord) in IN_THE_MANUSCRIPT {
                        tr {
                            td { "{what}" }
                            td { kbd { "{chord}" } }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn ProjectMenu() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let book_title = state.read().compile_book_title();
    // Each item closes the menu before it acts, so a file dialog does not
    // open with the menu still showing behind it.
    rsx! {
        div { class: "menu menu-end", role: "menu", aria_label: "Project",
            button {
                class: "menu-item",
                role: "menuitem",
                onclick: move |_| {
                    state.write().open_menu = None;
                    state.write().save_all();
                },
                span { "Save" }
                span { class: "menu-hint", "Ctrl+S" }
            }
            button {
                class: "menu-item",
                role: "menuitem",
                onclick: move |_| {
                    state.write().open_menu = None;
                    state.write().backup();
                },
                "Backup"
            }
            div { class: "menu-rule" }
            button {
                class: "menu-item",
                role: "menuitem",
                title: "Bring Markdown files in as new scenes",
                onclick: move |_| {
                    state.write().open_menu = None;
                    let files = pick_markdown_files();
                    if !files.is_empty() {
                        state.write().import_markdown_files(files);
                    }
                },
                "Import Markdown files…"
            }
            div { class: "menu-rule" }
            div { class: "menu-label", "Export {book_title}" }
            button {
                class: "menu-item",
                role: "menuitem",
                onclick: move |_| {
                    state.write().open_menu = None;
                    export_project(state, CompiledFormat::Markdown);
                },
                "Markdown…"
            }
            button {
                class: "menu-item",
                role: "menuitem",
                onclick: move |_| {
                    state.write().open_menu = None;
                    export_project(state, CompiledFormat::Docx);
                },
                "DOCX…"
            }
            div { class: "menu-label", "Export the whole project" }
            button {
                class: "menu-item",
                role: "menuitem",
                title: "A folder of Markdown files, one for each scene and entry, for another writing program",
                onclick: move |_| {
                    state.write().open_menu = None;
                    if let Some(parent) = pick_folder("Choose where to put the Markdown folder") {
                        state.write().export_markdown_folder(parent);
                    }
                },
                "Markdown folder…"
            }
        }
    }
}

#[component]
fn SettingsPanel() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let theme = state.read().theme();
    let font = state.read().prose_font();
    let spelling_value = state.read().spelling_language().unwrap_or_default();
    let spelling_choices = state.read().spelling_choices();
    rsx! {
        div { class: "menu menu-end popover", role: "dialog", aria_label: "Settings",
            label { r#for: "setting-font", "Manuscript font" }
            select {
                id: "setting-font",
                title: "The typeface the manuscript is set in, saved for this computer",
                value: "{font.as_str()}",
                onchange: move |event| {
                    if let Some(next) = ProseFont::from_id(&event.value()) {
                        state.write().set_prose_font(next);
                    }
                },
                for item in ProseFont::ALL {
                    option { value: "{item.as_str()}", selected: item == font, "{item.label()}" }
                }
            }
            label { r#for: "setting-theme", "Theme" }
            select {
                id: "setting-theme",
                title: "Color theme, saved in this project",
                value: "{theme.as_str()}",
                onchange: move |event| {
                    if let Some(next) = Theme::from_id(&event.value()) {
                        state.write().set_theme(next);
                    }
                },
                for item in [Theme::Dark, Theme::Light, Theme::Paper] {
                    option { value: "{item.as_str()}", selected: item == theme, "{item.label()}" }
                }
            }
            label { r#for: "setting-spelling", "Spelling" }
            select {
                id: "setting-spelling",
                title: "Spellcheck dictionary, saved in this project. More dictionaries can go in the project's dictionaries folder.",
                value: "{spelling_value}",
                onchange: move |event| {
                    let value = event.value();
                    state.write().set_spelling((!value.is_empty()).then_some(value));
                },
                option { value: "", selected: spelling_value.is_empty(), "Off" }
                for item in spelling_choices {
                    option { value: "{item.id}", selected: item.id == spelling_value, "{item.label}" }
                }
            }
        }
    }
}

/// Quiet counts, in order of use, then the save state. The bar never wraps:
/// a count that does not fit drops out whole, and the save state stays.
#[component]
fn StatusBar() -> Element {
    let state = use_context::<Signal<StudioState>>();
    let status = state.read().status.clone();
    let mode = state.read().mode;
    let words = state.read().word_count();
    let book_words = state.read().compile_book_word_count();
    let book_title = state.read().compile_book_title();
    let goal = state.read().goal();
    let daily_goal = state.read().daily_goal();
    let today_words = state.read().current_book_words_today();
    let session_words = state.read().current_book_words_this_session();
    let session_time = format_elapsed(state.read().session_shown_secs);
    let unsaved = state.read().has_unsaved();
    let book_text = if goal > 0 {
        format!("{book_title}: {book_words} of {goal} words")
    } else {
        format!("{book_title}: {book_words} words")
    };
    let today_text = if daily_goal > 0 {
        format!("Today {today_words} of {daily_goal}")
    } else {
        format!("Today {today_words}")
    };
    let save_text = if unsaved { format!("Unsaved · {status}") } else { status };
    rsx! {
        footer { class: "status",
            span { class: "status-book", title: "{book_text}", "{book_text}" }
            div { class: "status-stats",
                if mode == WorkspaceMode::Writing {
                    span { "Scene {words}" }
                }
                span { "{today_text}" }
                span { "This sitting {session_words} · {session_time}" }
            }
            span { class: "status-save", title: "{save_text}", "{save_text}" }
        }
    }
}

#[component]
fn GoalsPanel() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let goal = state.read().goal();
    let daily_goal = state.read().daily_goal();
    let book_title = state.read().compile_book_title();
    rsx! {
        div { class: "menu popover", role: "dialog", aria_label: "Word targets",
            label { r#for: "goal-daily", "Words a day" }
            input {
                id: "goal-daily",
                r#type: "number",
                min: "0",
                value: "{daily_goal}",
                oninput: move |event| apply_goal(state, event.value(), true),
                onblur: move |_| state.write().save_all(),
            }
            label { r#for: "goal-book", "Words for {book_title}" }
            input {
                id: "goal-book",
                r#type: "number",
                min: "0",
                value: "{goal}",
                oninput: move |event| apply_goal(state, event.value(), false),
                onblur: move |_| state.write().save_all(),
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

