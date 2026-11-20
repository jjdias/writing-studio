use super::*;

/// The library row height, as `.library-row` sets it in `style.css`.
const LIBRARY_ROW_HEIGHT: f64 = 30.0;
/// A scene card's height, as `.scene-card` sets it in `style.css`.
const SCENE_CARD_HEIGHT: f64 = 92.0;

/// The left column: the project and its goal, the manuscript's books,
/// parts, chapters, and folders, then the other modes under "Extras".
#[component]
pub(super) fn Library() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let title = state.read().project_title();
    let mode = state.read().mode;
    let menu = state.read().open_menu;
    let book_words = state.read().compile_book_word_count();
    let goal = state.read().goal();
    let connected = state.read().sync_connected();
    let syncing = state.read().syncing;
    let manuscript_mode = matches!(mode, WorkspaceMode::Writing | WorkspaceMode::Corkboard | WorkspaceMode::Outliner);
    let filter_chosen = state.read().shown_filter().is_some();
    let shown = if manuscript_mode && !filter_chosen { state.read().shown_container() } else { None };
    let rows: Vec<(Id, u32, NodeKind, String, bool, bool)> = {
        let current = state.read();
        current
            .library_rows()
            .into_iter()
            .filter_map(|(id, depth, has_children)| {
                let node = current.binder.get(id)?;
                Some((id, depth, node.kind, node.title.clone(), has_children, current.is_collapsed(id)))
            })
            .collect()
    };
    let trash = state.read().binder.trash_root().map(|node| node.id);
    let fraction = if goal > 0 { (book_words as f64 / goal as f64).min(1.0) } else { 0.0 };
    let deadline = state.read().compile_book_deadline();
    let days_left = match state.read().compile_book_pace() {
        Some(Pace::Due { days, .. }) => Some(time_left(days)),
        Some(Pace::Passed { .. }) => Some("past".to_string()),
        _ => None,
    };
    let add_open = menu == Some(FrameMenu::Add);
    let add = move |kind: NodeKind| {
        move |_| {
            state.write().open_menu = None;
            state.write().add_node(kind);
        }
    };
    rsx! {
        aside { class: "library", aria_label: "Library",
            div { class: "library-head",
                h1 { class: "library-title", title: "{title}", "{title}" }
                div { class: "menu-anchor",
                    button {
                        class: "library-goal",
                        title: "Word targets",
                        aria_haspopup: "dialog",
                        aria_expanded: "{menu == Some(FrameMenu::Goals)}",
                        onclick: move |_| state.write().toggle_menu(FrameMenu::Goals),
                        Ring { fraction }
                        if goal > 0 { "{book_words} of {goal} words" } else { "{book_words} words" }
                    }
                    if menu == Some(FrameMenu::Goals) { GoalsPanel {} }
                }
                if let Some(day) = deadline.clone() {
                    div { class: "library-deadline", title: "The day to reach the target by",
                        svg { class: "glyph", view_box: "0 0 16 16", "aria-hidden": "true",
                            path { d: "M2.5 3.5h11v10h-11zM2.5 6.5h11M5.5 2v3M10.5 2v3", fill: "none", stroke: "currentColor", stroke_width: "1.2" }
                        }
                        span { "{day}" }
                        if let Some(left) = days_left.clone() {
                            span { class: "library-deadline-left", "{left}" }
                        }
                    }
                }
                div { class: "library-tools",
                    div { class: "menu-anchor",
                        IconButton {
                            name: "Project: save, backup, import, and export",
                            open: menu == Some(FrameMenu::Project),
                            onclick: move |_| state.write().toggle_menu(FrameMenu::Project),
                            path { d: "M3 9.5v3.5h10V9.5M8 2.5v7M5 5.5l3-3 3 3", fill: "none", stroke: "currentColor", stroke_width: "1.3", stroke_linecap: "round", stroke_linejoin: "round" }
                        }
                        if menu == Some(FrameMenu::Project) { ProjectMenu {} }
                    }
                    div { class: "menu-anchor",
                        IconButton {
                            name: "Settings",
                            open: menu == Some(FrameMenu::Settings),
                            onclick: move |_| state.write().toggle_menu(FrameMenu::Settings),
                            circle { cx: "4", cy: "8", r: "1.2", fill: "currentColor" }
                            circle { cx: "8", cy: "8", r: "1.2", fill: "currentColor" }
                            circle { cx: "12", cy: "8", r: "1.2", fill: "currentColor" }
                        }
                        if menu == Some(FrameMenu::Settings) { SettingsPanel {} }
                    }
                    IconButton {
                        name: "Keyboard shortcuts (Ctrl+/)",
                        open: false,
                        onclick: move |_| {
                            let mut current = state.write();
                            current.open_menu = None;
                            current.show_shortcuts = true;
                        },
                        circle { cx: "8", cy: "8", r: "5.5", fill: "none", stroke: "currentColor", stroke_width: "1.2" }
                        path { d: "M6.4 6.3a1.7 1.7 0 113 1.1c-.6.4-1.4.8-1.4 1.6M8 11v.2", fill: "none", stroke: "currentColor", stroke_width: "1.2", stroke_linecap: "round" }
                    }
                    if connected {
                        button {
                            class: "library-sync",
                            title: "Send this machine's changes and bring in the others', through git",
                            disabled: syncing,
                            onclick: move |_| run_sync(state),
                            if syncing { "Syncing…" } else { "Sync" }
                        }
                    }
                }
            }
            nav { class: "library-list", aria_label: "Manuscript",
                for (id, depth, kind, name, has_children, collapsed) in rows {
                    LibraryRow { key: "{id}", id, depth, kind, name, active: shown == Some(id), has_children, collapsed }
                }
                div { class: "menu-anchor library-add",
                    button {
                        class: if add_open { "library-add-btn open" } else { "library-add-btn" },
                        aria_haspopup: "menu",
                        aria_expanded: "{add_open}",
                        onclick: move |_| state.write().toggle_menu(FrameMenu::Add),
                        "+ Add"
                    }
                    if add_open {
                        div { class: "menu", role: "menu", aria_label: "Add",
                            button { class: "menu-item", role: "menuitem", onclick: add(NodeKind::Chapter), span { "Chapter" } span { class: "menu-hint", "Ctrl+Shift+N" } }
                            button { class: "menu-item", role: "menuitem", onclick: add(NodeKind::Part), "Part" }
                            button { class: "menu-item", role: "menuitem", title: "Another novel under the manuscript", onclick: add(NodeKind::Book), "Book" }
                            button { class: "menu-item", role: "menuitem", onclick: add(NodeKind::Folder), "Folder" }
                        }
                    }
                }
                FilterSection {}
                div { class: "library-heading", "Extras" }
                for candidate in [WorkspaceMode::Characters, WorkspaceMode::World, WorkspaceMode::Plot, WorkspaceMode::Research, WorkspaceMode::History] {
                    button {
                        class: if mode == candidate { "library-row active" } else { "library-row" },
                        title: "{candidate.label()} ({mode_shortcut(candidate)})",
                        aria_current: if mode == candidate { "page" } else { "false" },
                        onclick: move |_| {
                            state.write().choose_mode(candidate);
                        },
                        ModeGlyph { mode: candidate }
                        span { class: "library-name", "{candidate.label()}" }
                    }
                }
                if let Some(trash) = trash {
                    LibraryRow { id: trash, depth: 0, kind: NodeKind::TrashRoot, name: "Trash".to_string(), active: shown == Some(trash), has_children: false, collapsed: false }
                }
            }
        }
    }
}

/// A manuscript container in the library. Chosen, it fills the scene
/// list; it takes chapters, scenes, and folders dropped on it.
#[component]
fn LibraryRow(
    id: Id,
    depth: u32,
    kind: NodeKind,
    name: String,
    active: bool,
    has_children: bool,
    collapsed: bool,
) -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let dragging = state.read().dragging == Some(id);
    let target = state.read().drop_target == Some(id);
    let place = state.read().drop_place;
    let ok = state
        .read()
        .dragging
        .is_some_and(|dragged| state.read().binder.drop_allowed_at(dragged, id, place));
    let mut class = String::from("library-row");
    if active {
        class.push_str(" active");
    }
    if state.read().binder.is_left_out(id) {
        class.push_str(" left-out");
    }
    if dragging {
        class.push_str(" dragging");
    }
    if target {
        class.push_str(match (ok, place) {
            (false, _) => " drop-invalid",
            (true, DropPlace::Before) => " drop-before",
            (true, DropPlace::After) => " drop-after",
            (true, DropPlace::Inside) => " drop-target",
        });
    }
    let indent = depth * 14;
    let fold_label = if collapsed { format!("Show what is in {name}") } else { format!("Fold {name}") };
    let editing = state.read().renaming == Some(id);
    let can_change = !kind.is_root();
    let start_rename = move || {
        if can_change {
            state.write().start_rename(id);
            select_field(format!("rename-{id}"));
        }
    };
    if editing {
        return rsx! {
            div { class: "library-item", style: "padding-left: {indent}px",
                span { class: "fold-spacer" }
                RenameField { id, name: name.clone() }
            }
        };
    }
    rsx! {
        div { class: "library-item", style: "padding-left: {indent}px",
        if has_children {
            button {
                class: if collapsed { "fold-btn folded" } else { "fold-btn" },
                title: "{fold_label}",
                aria_label: "{fold_label}",
                aria_expanded: "{!collapsed}",
                onclick: move |_| state.write().toggle_collapsed(id),
                svg { class: "fold-glyph", view_box: "0 0 16 16", "aria-hidden": "true",
                    path { d: "M5 6l3 3 3-3", fill: "none", stroke: "currentColor", stroke_width: "1.6", stroke_linecap: "round", stroke_linejoin: "round" }
                }
            }
        } else {
            span { class: "fold-spacer" }
        }
        button {
            class: "{class}",
            aria_current: if active { "true" } else { "false" },
            draggable: !kind.is_root(),
            title: if can_change { "Double-click or press F2 to rename" } else { "" },
            onclick: move |_| state.write().choose_library(id),
            oncontextmenu: move |event: MouseEvent| {
                event.prevent_default();
                let point = event.client_coordinates();
                state.write().open_item_menu(id, point.x, point.y);
            },
            ondoubleclick: move |_| {
                let mut start = start_rename;
                start();
            },
            onkeydown: move |event: KeyboardEvent| {
                if event.key() == Key::F2 {
                    event.prevent_default();
                    let mut start = start_rename;
                    start();
                } else if let Some(step) = arrow_step(&event) {
                    event.prevent_default();
                    step_focus(".library button.library-row", step);
                }
            },
            ondragstart: move |event| {
                event.stop_propagation();
                state.write().begin_drag(id);
            },
            ondragover: move |event| {
                event.prevent_default();
                event.stop_propagation();
                let y = event.element_coordinates().y;
                let place = DropPlace::at(y, LIBRARY_ROW_HEIGHT, state.peek().drag_fits_inside(id));
                let current = state.peek();
                let changed = current.drop_target != Some(id) || current.drop_place != place;
                drop(current);
                if changed {
                    state.write().set_binder_drop(id, place);
                }
            },
            ondrop: move |event| {
                event.prevent_default();
                event.stop_propagation();
                state.write().drop_in_binder(id);
            },
            ondragend: move |_| state.write().end_drag(),
            KindGlyph { kind }
            span { class: "library-name", "{name}" }
        }
        if can_change {
            DeleteButton { id, name: name.clone() }
        }
        }
    }
}

/// The × at the right of a row or card: it moves the item to the trash,
/// where it can be restored until the trash is emptied.
#[component]
fn DeleteButton(id: Id, name: String) -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let label = format!("Move “{name}” to the trash");
    rsx! {
        button {
            class: "item-delete",
            title: "{label}",
            aria_label: "{label}",
            onclick: move |event| {
                event.stop_propagation();
                state.write().trash_item(id);
            },
            svg { class: "glyph", view_box: "0 0 16 16", "aria-hidden": "true",
                path { d: "M4.5 4.5l7 7M11.5 4.5l-7 7", fill: "none", stroke: "currentColor", stroke_width: "1.5", stroke_linecap: "round" }
            }
        }
    }
}

/// The name of a book, part, chapter, or folder, edited in its row. Enter
/// or leaving the field keeps it; Escape leaves the name as it was.
#[component]
fn RenameField(id: Id, name: String) -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let mut draft = use_signal(|| name.clone());
    let mut finish = move |keep: bool| {
        if state.peek().renaming != Some(id) {
            return;
        }
        state.write().renaming = None;
        if keep {
            state.write().rename_item(id, &draft());
        }
    };
    rsx! {
        input {
            id: "rename-{id}",
            class: "rename-field",
            aria_label: "Name of “{name}”",
            value: "{draft}",
            oninput: move |event| draft.set(event.value()),
            onkeydown: move |event| match event.key() {
                Key::Enter => {
                    event.prevent_default();
                    finish(true);
                }
                Key::Escape => {
                    event.stop_propagation();
                    finish(false);
                }
                _ => {}
            },
            onblur: move |_| finish(true),
        }
    }
}

/// Focus a field once it is on screen and select its text, so typing
/// replaces the old name.
fn select_field(id: String) {
    spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;
        let script = format!(
            "const field = document.getElementById({}); if (field) {{ field.focus(); field.select(); }} dioxus.send(true);",
            serde_json::to_string(&id).unwrap_or_else(|_| "\"\"".into())
        );
        let mut eval = document::eval(&script);
        let _ = eval.recv::<bool>().await;
    });
}

/// The middle column: the shown container's scenes, grouped by chapter,
/// each with its first lines and its status.
#[component]
pub(super) fn SceneList() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let widths = use_context::<Signal<PanelWidths>>();
    let width = widths.read().sidebar;
    let shown = state.read().shown_container();
    let filter = state.read().shown_filter();
    let heading = match &filter {
        Some(filter) => filter.name.clone(),
        None => shown
            .and_then(|id| state.read().binder.get(id).map(|node| node.title.clone()))
            .unwrap_or_else(|| "Manuscript".into()),
    };
    let in_trash = shown.is_some() && shown == state.read().binder.trash_root().map(|node| node.id);
    let search = state.read().search.clone();
    let hits = state.read().content_hits();
    let groups: Vec<SceneGroup> = {
        let current = state.read();
        current
            .scene_groups()
            .into_iter()
            .map(|mut group| {
                // A chapter whose name matches keeps all its scenes.
                if !matches_query(&group.title, &search) || group.title.is_empty() {
                    group
                        .scenes
                        .retain(|card| binder_matches(&current.binder, card.id, &search, &hits));
                }
                group
            })
            .filter(|group| !group.scenes.is_empty())
            .collect()
    };
    let selected = state.read().selected;
    // The ring for everything shown, filled as its scenes reach a final status.
    let (finished, total) = groups.iter().flat_map(|group| group.scenes.iter()).fold((0, 0), |(done, all), card| {
        (done + usize::from(status_tone(&card.status) == "final"), all + 1)
    });
    rsx! {
        aside {
            class: "scene-list",
            aria_label: "Scenes",
            style: "width: {width}px; flex: 0 0 {width}px;",
            div { class: "scene-list-head",
                h2 { title: "{heading}", "{heading}" }
                if total > 0 {
                    span { class: "scene-list-progress", title: "{finished} of {total} scenes final",
                        Ring { fraction: finished as f64 / total as f64 }
                    }
                }
                if !in_trash && filter.is_none() {
                    IconButton {
                        name: "New scene (Ctrl+N)",
                        open: false,
                        onclick: move |_| state.write().add_node(NodeKind::Scene),
                        path { d: "M9.5 2.5H3.5v10h9V6.5M7 9l1.5-.3 5-5-1.2-1.2-5 5z", fill: "none", stroke: "currentColor", stroke_width: "1.2", stroke_linejoin: "round" }
                    }
                }
            }
            input {
                id: "binder-search",
                class: "search",
                placeholder: "Search titles and text",
                title: "Ctrl+F",
                aria_label: "Search scenes",
                value: "{search}",
                oninput: move |event| state.write().search = event.value(),
            }
            if in_trash {
                div { class: "scene-trash-actions", TrashActions {} }
            }
            div { class: "scene-cards",
                if groups.is_empty() {
                    p { class: "hint scene-empty",
                        if !search.trim().is_empty() {
                            "No scene matches the search."
                        } else if in_trash {
                            "The trash is empty."
                        } else {
                            "No scenes here yet."
                        }
                    }
                }
                for group in groups {
                    {
                        let group_id = group.id;
                        // A group folds with its chapter: one folded set for the library and here.
                        let folded = !group.title.is_empty() && state.read().is_collapsed(group_id);
                        let count = group.scenes.len();
                        // The ring fills as the chapter's scenes reach a final status.
                        let finished = group.scenes.iter().filter(|card| status_tone(&card.status) == "final").count();
                        let fraction = if count == 0 { 0.0 } else { finished as f64 / count as f64 };
                        rsx! {
                            section { class: "scene-group", key: "{group_id}",
                                if !group.title.is_empty() {
                                    h3 { class: "scene-group-title",
                                        button {
                                            class: if folded { "scene-group-toggle folded" } else { "scene-group-toggle" },
                                            aria_expanded: "{!folded}",
                                            title: if folded { "Show these scenes" } else { "Fold these scenes" },
                                            onclick: move |_| state.write().toggle_collapsed(group_id),
                                            svg { class: "fold-glyph", view_box: "0 0 16 16", "aria-hidden": "true",
                                                path { d: "M5 6l3 3 3-3", fill: "none", stroke: "currentColor", stroke_width: "1.6", stroke_linecap: "round", stroke_linejoin: "round" }
                                            }
                                            span { class: "scene-group-name", "{group.title}" }
                                            span { class: "scene-group-progress", title: "{finished} of {count} scenes final",
                                                Ring { fraction }
                                            }
                                        }
                                    }
                                }
                                if !folded {
                                    for card in group.scenes {
                                        SceneCardView { active: selected == Some(card.id), card }
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

#[component]
fn SceneCardView(card: SceneCard, active: bool) -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let id = card.id;
    let dragging = state.read().dragging == Some(id);
    let target = state.read().drop_target == Some(id);
    let place = state.read().drop_place;
    let ok = state
        .read()
        .dragging
        .is_some_and(|dragged| state.read().binder.drop_allowed_at(dragged, id, place));
    let mut class = String::from("scene-card");
    if active {
        class.push_str(" active");
    }
    if card.left_out {
        class.push_str(" left-out");
    }
    if state.read().picked.contains(&id) {
        class.push_str(" picked");
    }
    if dragging {
        class.push_str(" dragging");
    }
    if target {
        class.push_str(match (ok, place) {
            (false, _) => " drop-invalid",
            (true, DropPlace::Before) => " drop-before",
            _ => " drop-after",
        });
    }
    let title = if card.title.trim().is_empty() { "Untitled scene".to_string() } else { card.title.clone() };
    let in_trash = state.read().binder.is_in_trash(id);
    rsx! {
        div { class: "scene-card-wrap",
        button {
            class: "{class}",
            aria_current: if active { "true" } else { "false" },
            draggable: true,
            onkeydown: move |event: KeyboardEvent| {
                if let Some(step) = arrow_step(&event) {
                    event.prevent_default();
                    step_focus(".scene-list button.scene-card", step);
                }
            },
            onclick: move |event: MouseEvent| {
                let keys = event.modifiers();
                if keys.ctrl() || keys.meta() {
                    state.write().toggle_pick(id);
                    return;
                }
                if keys.shift() {
                    state.write().pick_range(id);
                    return;
                }
                let mut current = state.write();
                current.clear_picked();
                current.select_node(id);
                current.mode = WorkspaceMode::Writing;
            },
            oncontextmenu: move |event: MouseEvent| {
                event.prevent_default();
                let point = event.client_coordinates();
                state.write().open_item_menu(id, point.x, point.y);
            },
            ondragstart: move |event| {
                event.stop_propagation();
                state.write().begin_drag(id);
            },
            ondragover: move |event| {
                event.prevent_default();
                event.stop_propagation();
                // A scene card is never a container: its halves are before and after.
                let y = event.element_coordinates().y;
                let place = DropPlace::at(y, SCENE_CARD_HEIGHT, false);
                let current = state.peek();
                let changed = current.drop_target != Some(id) || current.drop_place != place;
                drop(current);
                if changed {
                    state.write().set_binder_drop(id, place);
                }
            },
            ondrop: move |event| {
                event.prevent_default();
                event.stop_propagation();
                state.write().drop_in_binder(id);
            },
            ondragend: move |_| state.write().end_drag(),
            span { class: "scene-card-title", "{title}" }
            span { class: if card.opening.is_empty() { "scene-card-opening empty" } else { "scene-card-opening" },
                if card.opening.is_empty() { "Nothing written yet." } else { "{card.opening}" }
            }
            if !card.status.trim().is_empty() {
                StatusPill { status: card.status.clone() }
            }
            if !card.keywords.is_empty() {
                span { class: "card-keywords",
                    for (word, hex) in card.keywords.iter() {
                        span {
                            class: "keyword-dot small",
                            style: if hex.is_empty() { String::new() } else { format!("background: {hex}") },
                            title: "{word}",
                        }
                    }
                }
            }
        }
        if !in_trash {
            DeleteButton { id, name: title.clone() }
        }
        }
    }
}

/// A status as a small colored pill. Draft, Revise, and Final have their
/// own colors; any other status takes a steady one from its name.
#[component]
pub(super) fn StatusPill(status: String) -> Element {
    let tone = status_tone(&status);
    let status = status_name(&status);
    rsx! {
        span { class: "status-pill pill-{tone}", "{status}" }
    }
}

pub(super) fn status_tone(status: &str) -> &'static str {
    let lower = status.trim().to_lowercase();
    if lower.contains("final") || lower.contains("done") {
        return "final";
    }
    if lower.contains("revis") || lower.contains("edit") {
        return "revise";
    }
    if lower.contains("draft") {
        return "draft";
    }
    if lower.contains("to do") || lower.contains("todo") {
        return "blue";
    }
    if lower.contains("hold") {
        return "grey";
    }
    let sum = lower.bytes().fold(0u32, |sum, byte| sum.wrapping_mul(31).wrapping_add(byte as u32));
    ["blue", "violet", "grey"][(sum % 3) as usize]
}

/// Progress as a small ring, filled from the top clockwise.
#[component]
pub(super) fn Ring(fraction: f64) -> Element {
    let circumference = 2.0 * std::f64::consts::PI * 6.0;
    let filled = circumference * fraction.clamp(0.0, 1.0);
    let rest = circumference - filled;
    rsx! {
        svg { class: "ring", view_box: "0 0 16 16", "aria-hidden": "true",
            circle { class: "ring-track", cx: "8", cy: "8", r: "6", fill: "none", stroke_width: "2" }
            circle {
                cx: "8", cy: "8", r: "6", fill: "none", stroke: "currentColor", stroke_width: "2",
                stroke_dasharray: "{filled} {rest}",
                transform: "rotate(-90 8 8)",
                stroke_linecap: "round",
            }
        }
    }
}

/// An icon-only button with its name, for the quiet bars.
#[component]
pub(super) fn IconButton(name: String, open: bool, onclick: EventHandler<MouseEvent>, children: Element) -> Element {
    rsx! {
        button {
            class: if open { "icon-btn open" } else { "icon-btn" },
            title: "{name}",
            aria_label: "{name}",
            onclick: move |event| onclick.call(event),
            svg { class: "glyph", view_box: "0 0 16 16", "aria-hidden": "true", {children} }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_statuses_have_their_own_colors_and_others_keep_one() {
        assert_eq!(status_tone("Draft"), "draft");
        assert_eq!(status_tone("First draft"), "draft");
        assert_eq!(status_tone("Revise"), "revise");
        assert_eq!(status_tone("Final Draft"), "final");
        assert_eq!(status_tone("Done"), "final");
        assert_eq!(status_tone("To do"), "blue");
        assert_eq!(status_tone("On hold"), "grey");
        assert_eq!(status_name("draft"), "Draft");
        assert_eq!(status_name("Waiting on notes"), "Waiting on notes");
        assert_eq!(status_name("almost"), "Almost");
        let other = status_tone("Waiting on notes");
        assert_eq!(status_tone("waiting on notes"), other);
        assert!(["blue", "violet", "grey"].contains(&other));
    }
}

/// Saved filters as library rows, with the form for a new one.
#[component]
fn FilterSection() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let filters = state.read().binder.filters.clone();
    let chosen = state.read().shown_filter().map(|filter| filter.id);
    let open = state.read().open_menu == Some(FrameMenu::Filter);
    let editing = state.read().editing_filter.and_then(|id| filters.iter().find(|filter| filter.id == id).cloned());
    let form_key = editing.as_ref().map(|filter| filter.id.to_string()).unwrap_or_else(|| "new".into());
    rsx! {
        div { class: "library-heading", "Filters" }
        for filter in filters {
            div { class: "library-item", key: "{filter.id}",
                span { class: "fold-spacer" }
                button {
                    class: if chosen == Some(filter.id) { "library-row active" } else { "library-row" },
                    aria_current: if chosen == Some(filter.id) { "true" } else { "false" },
                    title: "{filter_summary(&filter)}",
                    onclick: move |_| state.write().choose_filter(filter.id),
                    svg { class: "glyph", view_box: "0 0 16 16", "aria-hidden": "true",
                        path { d: "M2.5 3.5h11l-4.2 5v4l-2.6 1.2V8.5z", fill: "none", stroke: "currentColor", stroke_width: "1.2", stroke_linejoin: "round" }
                    }
                    span { class: "library-name", "{filter.name}" }
                }
                button {
                    class: "item-delete item-edit",
                    title: "Change the filter “{filter.name}”",
                    aria_label: "Change the filter {filter.name}",
                    onclick: move |event| {
                        event.stop_propagation();
                        state.write().edit_filter(filter.id);
                    },
                    svg { class: "glyph", view_box: "0 0 16 16", "aria-hidden": "true",
                        path { d: "M3 13l1-3.5 6.5-6.5 2.5 2.5-6.5 6.5z", fill: "none", stroke: "currentColor", stroke_width: "1.2", stroke_linejoin: "round" }
                    }
                }
                button {
                    class: "item-delete",
                    title: "Remove the filter “{filter.name}”. The scenes stay as they are.",
                    aria_label: "Remove the filter {filter.name}",
                    onclick: move |event| {
                        event.stop_propagation();
                        state.write().remove_filter(filter.id);
                    },
                    svg { class: "glyph", view_box: "0 0 16 16", "aria-hidden": "true",
                        path { d: "M4.5 4.5l7 7M11.5 4.5l-7 7", fill: "none", stroke: "currentColor", stroke_width: "1.5", stroke_linecap: "round" }
                    }
                }
            }
        }
        div { class: "menu-anchor library-add",
            button {
                class: if open { "library-add-btn open" } else { "library-add-btn" },
                aria_haspopup: "dialog",
                aria_expanded: "{open}",
                onclick: move |_| state.write().toggle_new_filter(),
                "+ Filter"
            }
            if open {
                FilterForm { key: "{form_key}", editing: editing.clone() }
            }
        }
    }
}

/// What a filter looks for, in a line: "Keyword Flashback · status Revise".
fn filter_summary(filter: &SavedFilter) -> String {
    let parts: Vec<String> = [
        ("keyword", &filter.keyword),
        ("status", &filter.status),
        ("label", &filter.label),
        ("text", &filter.text),
    ]
    .into_iter()
    .filter(|(_, value)| !value.trim().is_empty())
    .map(|(name, value)| format!("{name} “{}”", value.trim()))
    .collect();
    if parts.is_empty() {
        "Every scene".to_string()
    } else {
        format!("Scenes with {}", parts.join(", "))
    }
}

/// A new filter: a name, and what scenes must have. Every field may be left
/// empty; an empty one matches anything.
#[component]
fn FilterForm(editing: Option<SavedFilter>) -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let known: Vec<String> = state.read().binder.keywords.iter().map(|keyword| keyword.name.clone()).collect();
    let start = editing.clone().unwrap_or_else(|| SavedFilter {
        id: Id::new(),
        name: String::new(),
        keyword: String::new(),
        status: String::new(),
        label: String::new(),
        text: String::new(),
    });
    let id = start.id;
    let mut name = use_signal(|| start.name.clone());
    let mut keyword = use_signal(|| start.keyword.clone());
    let mut status = use_signal(|| start.status.clone());
    let mut text = use_signal(|| start.text.clone());
    let title = if editing.is_some() { "Change filter" } else { "New filter" };
    rsx! {
        div { class: "menu popover", role: "dialog", aria_label: "{title}",
            label { r#for: "filter-name", "Name" }
            input { id: "filter-name", placeholder: "Flashbacks to revise", value: "{name}", oninput: move |event| name.set(event.value()) }
            label { r#for: "filter-keyword", "Keyword" }
            select {
                id: "filter-keyword",
                onchange: move |event| keyword.set(event.value()),
                option { value: "", selected: keyword().is_empty(), "Any" }
                for word in known {
                    option { value: "{word}", selected: keyword().eq_ignore_ascii_case(&word), "{word}" }
                }
            }
            label { r#for: "filter-status", "Status" }
            StatusSelect { id: "filter-status", current: status(), any: true, onchange: move |value: String| status.set(value) }
            label { r#for: "filter-text", "Words in the text" }
            input { id: "filter-text", placeholder: "Any", value: "{text}", oninput: move |event| text.set(event.value()) }
            button {
                class: "primary filter-save",
                onclick: move |_| {
                    let filter = SavedFilter {
                        id,
                        name: name().trim().to_string(),
                        keyword: keyword(),
                        status: status().trim().to_string(),
                        label: String::new(),
                        text: text().trim().to_string(),
                    };
                    state.write().add_filter(filter);
                },
                "Save filter"
            }
        }
    }
}

/// The menu a right-click opens on a library row or a scene card: what can
/// be done to that item without going to the dashboard. Its buttons keep the
/// focus off themselves, so Split still finds the caret in the page.
#[component]
pub(super) fn ItemMenuView() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let Some(menu) = state.read().item_menu else {
        return rsx! {};
    };
    let id = menu.id;
    let Some(node) = state.read().binder.get(id).cloned() else {
        return rsx! {};
    };
    let in_trash = state.read().binder.is_in_trash(id);
    let in_research = state.read().binder.is_in_research(id);
    let many = state.read().picked_with(id).len();
    let scene = node.kind == NodeKind::Scene;
    let open_here = scene && state.read().selected == Some(id) && state.read().mode == WorkspaceMode::Writing;
    let has_next = scene && state.read().binder.scene_after(id).is_some();
    let current = status_name(&node.status);
    let close = move |_| state.write().item_menu = None;
    // The menu opens at the pointer and moves back inside the window when it
    // would not fit there.
    let style = format!(
        "left: {x}px; top: {y}px; transform: translate(min(0px, calc(100vw - 100% - {x}px - 8px)), min(0px, calc(100vh - 100% - {y}px - 8px)));",
        x = menu.x,
        y = menu.y
    );
    rsx! {
        div { class: "menu-backdrop", onclick: close, oncontextmenu: move |event: MouseEvent| {
            event.prevent_default();
            state.write().item_menu = None;
        } }
        div { class: "menu item-menu", role: "menu", aria_label: "{node.title}", style: "{style}",
            onmousedown: move |event| event.prevent_default(),
            if many > 1 {
                div { class: "menu-label", "{many} scenes" }
                for status in STATUSES {
                    button {
                        role: "menuitem",
                        class: "menu-item",
                        onclick: move |_| state.write().set_picked_status(status.to_string()),
                        "Mark as {status}"
                    }
                }
                div { class: "menu-rule" }
                button { role: "menuitem", class: "menu-item", onclick: move |_| state.write().merge_picked(), "Merge {many} scenes" }
                div { class: "menu-rule" }
                button {
                    role: "menuitem",
                    class: "menu-item danger-item",
                    onclick: move |_| state.write().trash_picked(),
                    "Move {many} scenes to trash"
                }
            } else if in_trash {
                button { role: "menuitem", class: "menu-item", onclick: move |_| state.write().restore_item(id), "Restore from trash" }
            } else {
                if scene {
                    button {
                        role: "menuitem",
                        class: "menu-item",
                        onclick: move |_| {
                            let mut current = state.write();
                            current.item_menu = None;
                            current.select_node(id);
                            current.mode = WorkspaceMode::Writing;
                        },
                        "Open"
                    }
                } else {
                    button {
                        role: "menuitem",
                        class: "menu-item",
                        onclick: move |_| {
                            let mut current = state.write();
                            current.item_menu = None;
                            current.choose_library(id);
                        },
                        "Show its scenes"
                    }
                    button {
                        role: "menuitem",
                        class: "menu-item",
                        onclick: move |_| {
                            state.write().start_rename(id);
                            select_field(format!("rename-{id}"));
                        },
                        "Rename"
                    }
                }
                if scene {
                    div { class: "menu-rule" }
                    div { class: "menu-label", "Status" }
                    for status in STATUSES {
                        button {
                            role: "menuitemradio",
                            aria_checked: "{current == status}",
                            class: if current == status { "menu-item checked" } else { "menu-item" },
                            onclick: move |_| state.write().set_item_status(id, status.to_string()),
                            "{status}"
                        }
                    }
                    div { class: "menu-rule" }
                    if open_here {
                        button {
                            role: "menuitem",
                            class: "menu-item",
                            onclick: move |_| {
                                state.write().item_menu = None;
                                split_at_caret(state);
                            },
                            "Split at the caret"
                        }
                    }
                    button {
                        role: "menuitem",
                        class: "menu-item",
                        disabled: !has_next,
                        onclick: move |_| state.write().merge_item_with_next(id),
                        "Merge with next scene"
                    }
                }
                if !in_research {
                    div { class: "menu-rule" }
                    button {
                        role: "menuitemcheckbox",
                        aria_checked: "{!node.left_out}",
                        class: if node.left_out { "menu-item" } else { "menu-item checked" },
                        onclick: move |_| {
                            let next = !state.read().binder.get(id).is_some_and(|node| node.left_out);
                            let mut current = state.write();
                            current.item_menu = None;
                            current.set_left_out(id, next);
                        },
                        "Include in compile"
                    }
                }
                div { class: "menu-rule" }
                button {
                    role: "menuitem",
                    class: "menu-item danger-item",
                    onclick: move |_| {
                        state.write().item_menu = None;
                        state.write().trash_item(id);
                    },
                    "Move to trash"
                }
            }
        }
    }
}

/// The Go to box (Ctrl+P): type part of a name, then Enter or a click opens
/// it. The arrow keys move through what was found; Escape closes the box.
#[component]
pub(super) fn QuickOpenDialog() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let mut query = use_signal(String::new);
    let mut chosen = use_signal(|| 0usize);
    let found: Vec<QuickMatch> = state.read().quick_matches(&query()).into_iter().take(30).collect();
    let at = chosen().min(found.len().saturating_sub(1));
    let ids: Vec<Id> = found.iter().map(|item| item.id).collect();
    rsx! {
        div { class: "dialog-backdrop", onclick: move |_| state.write().quick_open = false }
        div { class: "dialog quick-open", role: "dialog", aria_modal: "true", aria_label: "Go to",
            input {
                class: "quick-field",
                placeholder: "Go to a scene or chapter",
                aria_label: "Go to a scene or chapter",
                role: "combobox",
                aria_expanded: "true",
                aria_controls: "quick-results",
                value: "{query}",
                onmounted: move |event| async move {
                    let _ = event.set_focus(true).await;
                },
                oninput: move |event| {
                    query.set(event.value());
                    chosen.set(0);
                },
                onkeydown: move |event| match event.key() {
                    Key::ArrowDown => {
                        event.prevent_default();
                        if at + 1 < ids.len() {
                            chosen.set(at + 1);
                        }
                    }
                    Key::ArrowUp => {
                        event.prevent_default();
                        chosen.set(at.saturating_sub(1));
                    }
                    Key::Enter => {
                        event.prevent_default();
                        if let Some(id) = ids.get(at) {
                            state.write().go_to(*id);
                        }
                    }
                    _ => {}
                },
            }
            ul { id: "quick-results", class: "quick-results", role: "listbox",
                if found.is_empty() {
                    li { class: "quick-empty", "Nothing has that name." }
                }
                for (index, item) in found.into_iter().enumerate() {
                    li {
                        key: "{item.id}",
                        role: "option",
                        aria_selected: "{index == at}",
                        class: if index == at { "quick-item active" } else { "quick-item" },
                        onmouseenter: move |_| chosen.set(index),
                        onclick: move |_| state.write().go_to(item.id),
                        KindGlyph { kind: item.kind }
                        span { class: "quick-title", "{item.title}" }
                        if !item.place.is_empty() {
                            span { class: "quick-place", "{item.place}" }
                        }
                    }
                }
            }
        }
    }
}

/// How far the arrow keys move the focus in a list: one row for Up and Down,
/// to the ends for Home and End.
fn arrow_step(event: &KeyboardEvent) -> Option<i32> {
    match event.key() {
        Key::ArrowDown => Some(1),
        Key::ArrowUp => Some(-1),
        Key::Home => Some(i32::MIN),
        Key::End => Some(i32::MAX),
        _ => None,
    }
}

/// Move the focus `step` rows through the buttons `selector` finds, from the
/// one that has it, and bring that row into view. Enter or Space on a row
/// then opens it, as a click does.
fn step_focus(selector: &'static str, step: i32) {
    spawn(async move {
        let script = format!(
            r#"
            const rows = [...document.querySelectorAll({selector})];
            const at = rows.indexOf(document.activeElement);
            if (at >= 0 && rows.length) {{
                const step = {step};
                const next = step <= -1000000 ? 0 : step >= 1000000 ? rows.length - 1 : Math.min(rows.length - 1, Math.max(0, at + step));
                rows[next].focus();
                rows[next].scrollIntoView({{ block: 'nearest' }});
            }}
            dioxus.send(true);
            "#,
            selector = serde_json::to_string(selector).unwrap_or_else(|_| "\"\"".into()),
        );
        let mut eval = document::eval(&script);
        let _ = eval.recv::<bool>().await;
    });
}
