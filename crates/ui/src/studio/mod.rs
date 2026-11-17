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
    PanelWidths, ProseFont, SidebarSide, Theme, BINDER_COLORS, INSPECTOR_WIDTH_DEFAULT, INSPECTOR_WIDTH_MAX,
    INSPECTOR_WIDTH_MIN, SIDEBAR_WIDTH_DEFAULT, SIDEBAR_WIDTH_MAX, SIDEBAR_WIDTH_MIN,
};
use writing_studio_diagnostics::{BookContext, SceneFacts};
use writing_studio_search::matches_query;
use writing_studio_spellcheck::UnknownWord;

use crate::shortcuts::{app_shortcut, AppShortcut};
use crate::state::{
    Beat, BibleLink, CompiledFormat, Earlier, FoldedPanel, FrameMenu, ReviewTab, StudioState, WorkspaceMode,
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
    let theme = state.read().theme();
    let font = state.read().prose_font();

    rsx! {
        div {
            class: "shell theme-{theme.as_str()} prose-{font.as_str()}",
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
                let fold = fold_for(window_width.peek().0, *widths.peek());
                if apply_shortcut(state, chrome, widths, fold, action) {
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
    let mode = state.read().mode;
    let focus = state.read().focus_mode;
    let unsaved = state.read().has_unsaved();
    let overlay = state.read().overlay;
    let menu_open = state.read().open_menu.is_some();
    let chrome = use_context::<Signal<PanelChrome>>();
    let arrangement = *chrome.read();
    let fold = current_fold();
    let sidebar_left = arrangement.sidebar_side == SidebarSide::Left;
    let inspector_mode = matches!(mode, WorkspaceMode::Writing | WorkspaceMode::History);
    let show_sidebar = !focus && arrangement.sidebar_visible && !fold.binder;
    let show_inspector =
        !focus && arrangement.inspector_visible && inspector_mode && !fold.inspector;
    let overlay_binder = !focus && fold.binder && overlay == Some(FoldedPanel::Binder);
    let overlay_inspector =
        !focus && fold.inspector && inspector_mode && overlay == Some(FoldedPanel::Inspector);
    let binder_on = if fold.binder { overlay_binder } else { arrangement.sidebar_visible };
    let inspector_on = if fold.inspector { overlay_inspector } else { arrangement.inspector_visible };
    // The binder keeps its side over the page; the inspector takes the other.
    let overlay_left = (overlay_binder && sidebar_left) || (overlay_inspector && !sidebar_left);
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
        if menu_open {
            div { class: "menu-backdrop", onclick: move |_| state.write().open_menu = None }
        }
        if !focus { TopBar {} }
        div { class: if focus { "workbench focus" } else { "workbench" },
            nav { class: "activity", aria_label: "Modes",
                for candidate in WorkspaceMode::ALL {
                    button {
                        class: if mode == candidate { "activity-btn active" } else { "activity-btn" },
                        title: "{candidate.label()} ({mode_shortcut(candidate)})",
                        aria_label: "{candidate.label()}",
                        aria_current: if mode == candidate { "page" } else { "false" },
                        onclick: move |_| {
                            state.write().choose_mode(candidate);
                        },
                        ModeGlyph { mode: candidate }
                        span { class: "activity-name", "{candidate.label()}" }
                    }
                }
                div { class: "activity-spacer" }
                div { class: "activity-toggles",
                    button {
                        class: if binder_on { "activity-toggle on" } else { "activity-toggle" },
                        title: "Binder (Ctrl+Alt+B)",
                        aria_label: "Binder",
                        aria_pressed: "{binder_on}",
                        onclick: move |_| toggle_binder(state, chrome, fold),
                        svg { class: "glyph", view_box: "0 0 16 16",
                            path { d: "M1.5 2.5h13v11h-13zM6 2.5v11", fill: "none", stroke: "currentColor", stroke_width: "1.2" }
                        }
                        span { class: "activity-name", "Binder" }
                    }
                    button {
                        class: if inspector_on { "activity-toggle on" } else { "activity-toggle" },
                        title: "Inspector (Ctrl+Alt+I)",
                        aria_label: "Inspector",
                        aria_pressed: "{inspector_on}",
                        onclick: move |_| toggle_inspector_panel(state, chrome, fold),
                        svg { class: "glyph", view_box: "0 0 16 16",
                            path { d: "M1.5 2.5h13v11h-13zM10 2.5v11", fill: "none", stroke: "currentColor", stroke_width: "1.2" }
                        }
                        span { class: "activity-name", "Inspector" }
                    }
                    button {
                        class: if focus { "activity-toggle on" } else { "activity-toggle" },
                        title: "Focus (Ctrl+Shift+F)",
                        aria_label: "Focus",
                        aria_pressed: "{focus}",
                        onclick: move |_| {
                            let next = !state.read().focus_mode;
                            state.write().focus_mode = next;
                        },
                        svg { class: "glyph", view_box: "0 0 16 16",
                            path { d: "M2 6V2h4M10 2h4v4M14 10v4h-4M6 14H2v-4", fill: "none", stroke: "currentColor", stroke_width: "1.2" }
                        }
                        span { class: "activity-name", "Focus" }
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
            if overlay_binder || overlay_inspector {
                div { class: "overlay-backdrop", onclick: move |_| state.write().overlay = None }
                div { class: if overlay_left { "overlay overlay-left" } else { "overlay overlay-right" },
                    if overlay_binder { Explorer {} } else { InspectorPane {} }
                }
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

/// The project and the book, and the menus for the whole project.
#[component]
fn TopBar() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let title = state.read().project_title();
    let menu = state.read().open_menu;
    let syncing = state.read().syncing;
    let connected = state.read().sync_connected();
    let project_open = menu == Some(FrameMenu::Project);
    let settings_open = menu == Some(FrameMenu::Settings);
    rsx! {
        header { class: "topbar",
            span { class: "topbar-project", title: "{title}", "{title}" }
            CompileBookSelect {}
            div { class: "topbar-spacer" }
            if connected {
                button {
                    title: "Send this machine's changes and bring in the others', through git",
                    disabled: syncing,
                    onclick: move |_| run_sync(state),
                    if syncing { "Syncing…" } else { "Sync" }
                }
            }
            div { class: "menu-anchor",
                button {
                    class: if project_open { "menu-btn open" } else { "menu-btn" },
                    aria_haspopup: "menu",
                    aria_expanded: "{project_open}",
                    onclick: move |_| state.write().toggle_menu(FrameMenu::Project),
                    "Project"
                }
                if project_open { ProjectMenu {} }
            }
            div { class: "menu-anchor",
                button {
                    class: if settings_open { "menu-btn open" } else { "menu-btn" },
                    aria_haspopup: "dialog",
                    aria_expanded: "{settings_open}",
                    onclick: move |_| state.write().toggle_menu(FrameMenu::Settings),
                    "Settings"
                }
                if settings_open { SettingsPanel {} }
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
    let mut state = use_context::<Signal<StudioState>>();
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
    let goals_open = state.read().open_menu == Some(FrameMenu::Goals);
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
            div { class: "menu-anchor",
                button {
                    class: "status-goals",
                    title: "Word targets",
                    aria_haspopup: "dialog",
                    aria_expanded: "{goals_open}",
                    onclick: move |_| state.write().toggle_menu(FrameMenu::Goals),
                    "{book_text}"
                }
                if goals_open { GoalsPanel {} }
            }
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
        div { class: "menu popover menu-up", role: "dialog", aria_label: "Word targets",
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
            title: "Current book. Export, the word target, and the binder follow this book.",
            aria_label: "Current book",
            value: "{value}",
            onchange: move |event| {
                if let Ok(id) = event.value().parse::<Id>() {
                    state.write().set_compile_book(id);
                }
            },
            for (id, title) in books {
                option { value: "{id}", selected: current == Some(id), "{title}" }
            }
        }
    }
}
