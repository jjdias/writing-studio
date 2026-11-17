use super::*;

#[component]
pub(super) fn ModeGlyph(mode: WorkspaceMode) -> Element {
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

pub(super) fn mode_shortcut(mode: WorkspaceMode) -> &'static str {
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

pub(super) fn apply_shortcut(
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

pub(super) fn set_mode(mut state: Signal<StudioState>, mode: WorkspaceMode) -> bool {
    if !state.read().is_open() {
        return false;
    }
    state.write().mode = mode;
    true
}

pub(super) fn clear_or_exit_focus(mut state: Signal<StudioState>) -> bool {
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

pub(super) fn focus_find(mut state: Signal<StudioState>, chrome: Signal<PanelChrome>) {
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

pub(super) fn focus_dom(id: &'static str, wait_for_layout: bool) {
    spawn(async move {
        if wait_for_layout {
            tokio::time::sleep(std::time::Duration::from_millis(40)).await;
        }
        let script = format!("document.getElementById('{id}')?.focus(); dioxus.send(true);");
        let mut eval = document::eval(&script);
        let _ = eval.recv::<bool>().await;
    });
}
