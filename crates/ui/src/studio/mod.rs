//! The window. Each area is its own module; they share names through
//! `use super::*`, so an item one area uses from another is `pub(super)`.


use dioxus::prelude::*;
use writing_studio_core::Id;
use writing_studio_document::{
    DiffOp, Highlight, NavItem, NavKind, NoteKind, ParagraphChange, ReviewItem, ReviewKind, ReviewTarget,
    TrackedChange,
};
use writing_studio_editor::EditorSession;
use writing_studio_manuscript::{outline_rows, row_word_count};
use writing_studio_project::{
    binder_color_hex, drag_width, format_elapsed, same_keyword, tidy_keyword, BinderNode, KeywordQuery, KeywordSort, DropPlace, NodeKind, PanelChrome, PanelLayout,
    Fade, FixedScroll, KeepPlace, MarkdownOptions, Pace, PageStyle, SavedFilter, PanelWidths, ProseFont, Theme, BINDER_COLORS, FIRST_LINE_INDENT_RANGE, LINE_HEIGHT_RANGE,
    LINE_LENGTH_RANGE, PARAGRAPH_SPACING_RANGE, INSPECTOR_WIDTH_DEFAULT, INSPECTOR_WIDTH_MAX,
    INSPECTOR_WIDTH_MIN, SIDEBAR_WIDTH_DEFAULT, SIDEBAR_WIDTH_MAX, SIDEBAR_WIDTH_MIN,
};
use writing_studio_diagnostics::{BookContext, SceneFacts};
use writing_studio_spellcheck::UnknownWord;

use crate::shortcuts::{app_shortcut, AppShortcut, IN_THE_MANUSCRIPT, LISTED};
use crate::state::{binder_matches, pasted_markdown,
    Beat, CheckFinding, CompileCheck, Earlier, FoldedPanel, FrameMenu, QuickMatch, ReviewTab, BinderRow,
    StudioState, WorkspaceMode,
};

mod layout;
mod binder;
mod inspector;
mod editor;
mod commands;
mod history;
mod dialogs;
mod shared;
mod plot;
mod mindmap;
mod outliner;
mod library;
mod compile;

use layout::*;
use binder::*;
use inspector::*;
use editor::*;
use commands::*;
use history::*;
use dialogs::*;
use shared::*;
use plot::*;
use mindmap::*;
use outliner::*;
use library::*;
use compile::*;

const STYLES: &str = include_str!("../../assets/style.css");
/// The Markdown editor, CodeMirror 6 bundled into one file
/// (`crates/ui/codemirror` builds it). Loaded once, as `window.wsEditor`.
const CODEMIRROR_JS: &str = include_str!("../../assets/codemirror.js");
/// Unknown words shown as buttons before the rest are counted as "more".
const SPELL_WORDS_SHOWN: usize = 60;

#[component]
pub fn App() -> Element {
    use_context_provider(|| Signal::new(StudioState::default()));
    use_context_provider(|| Signal::new(PanelChrome::default()));
    use_context_provider(|| Signal::new(PanelWidths::default()));
    use_context_provider(|| Signal::new(None::<ResizeDrag>));
    use_context_provider(|| Signal::new(initial_window_width()));
    // After the first render, so the page is there to run it.
    use_effect(|| {
        let _ = document::eval(&format!("if (!window.wsEditor) {{\n{CODEMIRROR_JS}\n}}"));
    });
    rsx! {
        // The page's own title, which assistive tools read, is the window's.
        document::Title { "Writing Studio" }
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
    let page = page_variables(state.read().page_style());

    rsx! {
        div {
            class: "shell theme-{theme.as_str()} prose-{font.as_str()}",
            style: "{page}",
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
            if open && state.read().quick_open { QuickOpenDialog {} }
            if open && state.read().compile_open { CompileDialog {} }
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
    let overlay = state.read().overlay;
    let menu_open = state.read().open_menu.is_some();
    let chrome = use_context::<Signal<PanelChrome>>();
    let arrangement = *chrome.read();
    let fold = current_fold();
    let inspector_mode = state.read().context_shown();
    // The binder is on screen in every mode, and Context on the right where
    // there is something to say about what is chosen. In a narrow window
    // Context folds away first, then the binder.
    let show_library = !focus && arrangement.sidebar_visible && !fold.binder;
    let show_inspector =
        !focus && arrangement.inspector_visible && inspector_mode && !fold.inspector;
    let overlay_library = !focus && fold.binder && overlay == Some(FoldedPanel::Binder);
    let overlay_inspector =
        !focus && fold.inspector && inspector_mode && overlay == Some(FoldedPanel::Inspector);
    let library_on = if fold.binder { overlay_library } else { arrangement.sidebar_visible };
    let inspector_on = inspector_mode && if fold.inspector { overlay_inspector } else { arrangement.inspector_visible };
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
                }
            }
            if show_library {
                Library {}
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
                                name: "Context (Ctrl+Alt+I)",
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
                        WorkspaceMode::MindMap => rsx! { MindMapPane {} },
                        WorkspaceMode::Outliner => rsx! { OutlinerPane {} },
                        WorkspaceMode::Plot => rsx! { PlotPane {} },
                        WorkspaceMode::History => rsx! { HistoryPane {} },
                    }}
                }
            }
            // Context is on the right of the page, as the inspector was.
            if show_inspector {
                Sash { kind: ResizeKind::Inspector, positive: false }
                ContextPane {}
            }
            if overlay_inspector {
                div { class: "overlay-backdrop", onclick: move |_| state.write().overlay = None }
                div { class: "overlay overlay-right", ContextPane {} }
            }
        }
        if focus {
            // Focus mode keeps only the page and how many words are on it.
            div { class: "focus-count", aria_live: "polite", "{state.read().word_count()} words" }
        } else {
            StatusBar {}
        }
        ItemMenuView {}
    }
}

/// Every shortcut, and what the program is, from Help or Ctrl+/. Escape or
/// Close puts it away.
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
                        td { "Close a menu or a panel, cancel a waiting delete, leave focus mode, clear a search, or let go of a keyword" }
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
            About {}
        }
    }
}

/// The program's version, its license, and what it carries from others.
/// Text only: a link would open inside the window, not in a browser.
#[component]
fn About() -> Element {
    rsx! {
        section { class: "about", aria_labelledby: "about-title",
            h3 { id: "about-title", "About Writing Studio" }
            p { "Version {VERSION}. Free and open source under the MIT or Apache-2.0 license, at your choice." }
            p { "Source and releases: github.com/jjdias/writing-studio" }
            p {
                "Spelling uses the spellbook library (MPL-2.0) and the Hunspell English (US) dictionary, "
                "which keeps its own license. The Markdown editor is CodeMirror 6 (MIT). The licenses of every library the program is built from "
                "are in THIRD-PARTY-NOTICES.html, in {LICENSES_FOLDER}."
            }
        }
    }
}

/// Where the packages install the licenses: the Linux package puts the
/// program in /usr/bin and its files in /usr/lib/WritingStudio.
const LICENSES_FOLDER: &str = if cfg!(target_os = "linux") {
    "/usr/lib/WritingStudio/licenses"
} else {
    "the licenses folder beside the program"
};

/// The version of this build, from the workspace's `Cargo.toml`.
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[component]
fn ProjectMenu() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let book_title = state.read().compile_book_title();
    // Each item closes the menu before it acts, so a file dialog does not
    // open with the menu still showing behind it.
    rsx! {
        div { class: "menu", role: "menu", aria_label: "Project",
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
            button {
                class: "menu-item",
                role: "menuitem",
                title: "Compile {book_title} with a profile",
                onclick: move |_| {
                    let mut current = state.write();
                    current.open_menu = None;
                    current.compile_open = true;
                },
                "Compile…"
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
        div { class: "menu popover", role: "dialog", aria_label: "Settings",
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
            MarkdownSettings {}
            PageSettings {}
            KeepPlaceSettings {}
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
    let book_text = if goal > 0 {
        format!("{book_title}: {book_words} of {goal} words")
    } else {
        format!("{book_title}: {book_words} words")
    };
    // With a deadline, the book's count says the pace that meets it.
    let book_text = match state.read().compile_book_pace() {
        Some(pace) => format!("{book_text} · {}", pace_text(pace)),
        None => book_text,
    };
    let today_text = if daily_goal > 0 {
        format!("Today {today_words} of {daily_goal}")
    } else {
        format!("Today {today_words}")
    };
    let save_text = if unsaved { format!("Unsaved · {status}") } else { status };
    let undo_offered = state.read().undo_offered();
    rsx! {
        footer { class: "status",
            span { class: "status-book", title: "{book_text}", "{book_text}" }
            div { class: "status-stats",
                if mode == WorkspaceMode::Writing {
                    span { if words == 1 { "1 word in this scene" } else { "{words} words in this scene" } }
                }
                span { "{today_text}" }
                span { "This sitting {session_words} · {session_time}" }
            }
            span { class: "status-save", title: "{save_text}", "{save_text}" }
            if undo_offered {
                button {
                    class: "status-undo",
                    title: "Undo what the status line says",
                    onmousedown: move |event| event.prevent_default(),
                    onclick: move |_| state.write().take_undo_offer(),
                    "Undo"
                }
            }
        }
    }
}

#[component]
fn GoalsPanel() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let goal = state.read().goal();
    let daily_goal = state.read().daily_goal();
    let book_title = state.read().compile_book_title();
    let deadline = state.read().compile_book_deadline().unwrap_or_default();
    let pace = state.read().compile_book_pace();
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
            label { r#for: "goal-deadline", "Finish by" }
            div { class: "deadline-row",
                input {
                    id: "goal-deadline",
                    r#type: "date",
                    value: "{deadline}",
                    onchange: move |event| {
                        let day = event.value();
                        state.write().set_compile_book_deadline((!day.is_empty()).then_some(day));
                        state.write().save_all();
                    },
                }
                if !deadline.is_empty() {
                    button {
                        title: "Take the deadline off",
                        onclick: move |_| {
                            state.write().set_compile_book_deadline(None);
                            state.write().save_all();
                        },
                        "Clear"
                    }
                }
            }
            if let Some(pace) = pace {
                p { class: "hint deadline-pace", "{pace_text(pace)}" }
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


/// The page style as CSS variables on the shell, which the page's rules
/// read. Line length is in characters, spacing in lines, indent in em.
fn page_variables(style: PageStyle) -> String {
    format!(
        "--line-length: {}ch; --line-height: {}; --para-space: {}; --indent: {}em;",
        style.line_length,
        style.line_height as f64 / 100.0,
        style.paragraph_spacing as f64 / 10.0,
        style.first_line_indent as f64 / 10.0,
    )
}

/// One slider of the page settings.
#[derive(Clone, Copy, PartialEq)]
enum PageKnob {
    LineLength,
    LineHeight,
    ParagraphSpacing,
    FirstLineIndent,
}

impl PageKnob {
    const ALL: [PageKnob; 4] = [
        PageKnob::LineLength,
        PageKnob::LineHeight,
        PageKnob::ParagraphSpacing,
        PageKnob::FirstLineIndent,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::LineLength => "Line length",
            Self::LineHeight => "Line height",
            Self::ParagraphSpacing => "Space between paragraphs",
            Self::FirstLineIndent => "First-line indent",
        }
    }

    fn range(self) -> (u32, u32) {
        match self {
            Self::LineLength => LINE_LENGTH_RANGE,
            Self::LineHeight => LINE_HEIGHT_RANGE,
            Self::ParagraphSpacing => PARAGRAPH_SPACING_RANGE,
            Self::FirstLineIndent => FIRST_LINE_INDENT_RANGE,
        }
    }

    fn get(self, style: PageStyle) -> u32 {
        match self {
            Self::LineLength => style.line_length,
            Self::LineHeight => style.line_height,
            Self::ParagraphSpacing => style.paragraph_spacing,
            Self::FirstLineIndent => style.first_line_indent,
        }
    }

    fn set(self, mut style: PageStyle, value: u32) -> PageStyle {
        match self {
            Self::LineLength => style.line_length = value,
            Self::LineHeight => style.line_height = value,
            Self::ParagraphSpacing => style.paragraph_spacing = value,
            Self::FirstLineIndent => style.first_line_indent = value,
        }
        style
    }

    /// The value as it reads: "70 characters", "1.65", "0.7 lines", "1.5 em".
    fn shown(self, value: u32) -> String {
        match self {
            Self::LineLength => format!("{value} characters"),
            Self::LineHeight => format!("{:.2}", value as f64 / 100.0),
            Self::ParagraphSpacing if value == 0 => "None".to_string(),
            Self::ParagraphSpacing => format!("{:.1} lines", value as f64 / 10.0),
            Self::FirstLineIndent if value == 0 => "None".to_string(),
            Self::FirstLineIndent => format!("{:.1} em", value as f64 / 10.0),
        }
    }
}

/// Line length, line height, paragraph spacing, and indent, set while
/// looking at the page. They do not change compile.
#[component]
fn PageSettings() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let style = state.read().page_style();
    rsx! {
        for knob in PageKnob::ALL {
            {
                let (low, high) = knob.range();
                let value = knob.get(style);
                let id = format!("setting-page-{}", knob as u8);
                rsx! {
                    label { class: "range-label", r#for: "{id}",
                        span { "{knob.label()}" }
                        span { class: "range-value", "{knob.shown(value)}" }
                    }
                    input {
                        id: "{id}",
                        r#type: "range",
                        min: "{low}",
                        max: "{high}",
                        value: "{value}",
                        oninput: move |event| {
                            if let Ok(next) = event.value().parse::<u32>() {
                                let current = state.read().page_style();
                                state.write().set_page_style(knob.set(current, next));
                            }
                        },
                    }
                }
            }
        }
        if style != PageStyle::default() {
            button {
                class: "range-reset",
                onclick: move |_| state.write().set_page_style(PageStyle::default()),
                "Reset the page"
            }
        }
    }
}

/// Fixed scrolling, the line marker, and fading, under the page settings.
/// How the Markdown page shows and takes text, saved in the project.
#[component]
fn MarkdownSettings() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let options = state.read().markdown_options();
    let mut set = move |change: fn(&mut MarkdownOptions, bool), on: bool| {
        let mut next = state.read().markdown_options();
        change(&mut next, on);
        state.write().set_markdown_options(next);
    };
    rsx! {
        label { class: "check-label", title: "Ctrl+E. Off, the marks show only where the caret is.",
            input {
                r#type: "checkbox",
                checked: options.source_mode,
                onchange: move |event| set(|options, on| options.source_mode = on, event.checked()),
            }
            "Show every Markdown mark"
        }
        label { class: "check-label", title: "Curly quotes, an em dash for --, and an ellipsis for ..., as you type",
            input {
                r#type: "checkbox",
                checked: options.smart_typography,
                onchange: move |event| set(|options, on| options.smart_typography = on, event.checked()),
            }
            "Smart quotes and dashes"
        }
        label { class: "check-label", title: "A typed bracket, quotation mark, *, _, or ` gets its closing pair",
            input {
                r#type: "checkbox",
                checked: options.pair_marks,
                onchange: move |event| set(|options, on| options.pair_marks = on, event.checked()),
            }
            "Close brackets and marks"
        }
    }
}

#[component]
fn KeepPlaceSettings() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let keep = state.read().keep_place();
    let mut set = move |next: KeepPlace| state.write().set_keep_place(next);
    rsx! {
        label { r#for: "setting-scroll", "Keep the line I type" }
        select {
            id: "setting-scroll",
            onchange: move |event| {
                if let Some(scroll) = FixedScroll::from_id(&event.value()) {
                    let current = state.read().keep_place();
                    set(KeepPlace { scroll, ..current });
                }
            },
            for item in FixedScroll::ALL {
                option { value: "{item.as_str()}", selected: item == keep.scroll, "{item.label()}" }
            }
        }
        label { r#for: "setting-fade", "Fade the rest of the page" }
        select {
            id: "setting-fade",
            onchange: move |event| {
                if let Some(fade) = Fade::from_id(&event.value()) {
                    let current = state.read().keep_place();
                    set(KeepPlace { fade, ..current });
                }
            },
            for item in Fade::ALL {
                option { value: "{item.as_str()}", selected: item == keep.fade, "{item.label()}" }
            }
        }
        label { class: "check-label",
            input {
                r#type: "checkbox",
                checked: keep.line_marker,
                onchange: move |event| {
                    let current = state.read().keep_place();
                    set(KeepPlace { line_marker: event.checked(), ..current });
                },
            }
            "Mark the line I am on"
        }
        label { class: "check-label",
            input {
                r#type: "checkbox",
                checked: keep.hide_while_typing,
                onchange: move |event| {
                    let current = state.read().keep_place();
                    set(KeepPlace { hide_while_typing: event.checked(), ..current });
                },
            }
            "Hide everything but the page while I type"
        }
    }
}

/// A pace in words, as the targets panel and the status bar say it.
pub(super) fn pace_text(pace: Pace) -> String {
    match pace {
        Pace::Reached => "Target reached".to_string(),
        Pace::Due { days: 1, per_day } => format!("{per_day} words today to finish"),
        Pace::Due { days, per_day } => format!("{per_day} words a day for {days} days"),
        Pace::Passed { days_late, left } => {
            let days = if days_late == 1 { "1 day".to_string() } else { format!("{days_late} days") };
            format!("{left} words left, {days} past the deadline")
        }
    }
}

/// Time to a deadline the short way, as the library shows it: "1y 4mo",
/// "5mo", "12d", "today".
pub(super) fn time_left(days: u32) -> String {
    match days {
        0 | 1 => "today".to_string(),
        days if days >= 365 => {
            let (years, months) = (days / 365, (days % 365) / 30);
            if months == 0 { format!("{years}y") } else { format!("{years}y {months}mo") }
        }
        days if days >= 30 => format!("{}mo", days / 30),
        days => format!("{}d", days - 1),
    }
}

#[cfg(test)]
mod pace_tests {
    use super::*;

    #[test]
    fn a_pace_and_the_time_left_read_plainly() {
        assert_eq!(pace_text(Pace::Due { days: 3, per_day: 200 }), "200 words a day for 3 days");
        assert_eq!(pace_text(Pace::Due { days: 1, per_day: 600 }), "600 words today to finish");
        assert_eq!(pace_text(Pace::Passed { days_late: 1, left: 600 }), "600 words left, 1 day past the deadline");
        assert_eq!(time_left(1), "today");
        assert_eq!(time_left(13), "12d");
        assert_eq!(time_left(150), "5mo");
        assert_eq!(time_left(500), "1y 4mo");
    }
}
