use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ResizeKind {
    Sidebar,
    Inspector,
}

#[derive(Clone, Copy)]
pub(super) struct ResizeDrag {
    pub(super) kind: ResizeKind,
    pub(super) origin_x: f64,
    pub(super) origin_width: u32,
    pub(super) positive: bool,
}

pub(super) fn resize_limits(kind: ResizeKind) -> (u32, u32) {
    match kind {
        ResizeKind::Sidebar => (SIDEBAR_WIDTH_MIN, SIDEBAR_WIDTH_MAX),
        ResizeKind::Inspector => (INSPECTOR_WIDTH_MIN, INSPECTOR_WIDTH_MAX),
    }
}

pub(super) fn publish_layout(
    state: Signal<StudioState>,
    mut chrome: Signal<PanelChrome>,
    mut widths: Signal<PanelWidths>,
) {
    let layout = state.peek().panel_layout();
    chrome.set(layout.chrome());
    widths.set(layout.widths());
}

pub(super) fn persist_layout(mut state: Signal<StudioState>, layout: PanelLayout) {
    state.write().store_layout(layout);
}

pub(super) fn persist_chrome(state: Signal<StudioState>, chrome: PanelChrome) {
    let next = state.peek().panel_layout().with_chrome(chrome);
    persist_layout(state, next);
}

pub(super) fn toggle_sidebar(state: Signal<StudioState>, mut chrome: Signal<PanelChrome>) {
    let mut next = *chrome.peek();
    next.sidebar_visible = !next.sidebar_visible;
    chrome.set(next);
    persist_chrome(state, next);
}

pub(super) fn toggle_inspector(state: Signal<StudioState>, mut chrome: Signal<PanelChrome>) {
    let mut next = *chrome.peek();
    next.inspector_visible = !next.inspector_visible;
    chrome.set(next);
    persist_chrome(state, next);
}

pub(super) fn reveal_sidebar(state: Signal<StudioState>, mut chrome: Signal<PanelChrome>) -> bool {
    if chrome.peek().sidebar_visible {
        return false;
    }
    let mut next = *chrome.peek();
    next.sidebar_visible = true;
    chrome.set(next);
    persist_chrome(state, next);
    true
}

pub(super) fn select_activity(mut state: Signal<StudioState>, chrome: Signal<PanelChrome>, mode: WorkspaceMode) {
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

pub(super) fn flip_sidebar(state: Signal<StudioState>, mut chrome: Signal<PanelChrome>) {
    let mut next = *chrome.peek();
    next.sidebar_side = match next.sidebar_side {
        SidebarSide::Left => SidebarSide::Right,
        SidebarSide::Right => SidebarSide::Left,
    };
    chrome.set(next);
    persist_chrome(state, next);
}

pub(super) fn finish_resize(
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
pub(super) fn Sash(kind: ResizeKind, positive: bool) -> Element {
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
