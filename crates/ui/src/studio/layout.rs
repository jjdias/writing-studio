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

/// The window's width in logical pixels, kept current as it is resized.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct WindowWidth(pub(super) u32);

/// Matches `.activity` in `style.css`.
const ACTIVITY_WIDTH: u32 = 64;
/// The narrowest the page may get before a side panel folds away.
const PAGE_MIN_WIDTH: u32 = 400;

/// Which side panels have folded away. A folded panel is not laid out
/// beside the page; its button shows it over the page instead.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Fold {
    pub(super) binder: bool,
    pub(super) inspector: bool,
}

/// The inspector folds first, once the page would be narrower than
/// `PAGE_MIN_WIDTH` beside both panels, then the binder. With the default
/// widths that is about 1,000 and 720 pixels.
pub(super) fn fold_for(window: u32, widths: PanelWidths) -> Fold {
    let beside_binder = ACTIVITY_WIDTH + widths.sidebar + PAGE_MIN_WIDTH;
    Fold {
        binder: window < beside_binder,
        inspector: window < beside_binder + widths.inspector,
    }
}

pub(super) fn current_fold() -> Fold {
    let window = use_context::<Signal<WindowWidth>>();
    let widths = use_context::<Signal<PanelWidths>>();
    let width = window.read().0;
    let widths = *widths.read();
    fold_for(width, widths)
}

/// The binder's own control: it hides and shows the binder beside the
/// page, or over the page once it has folded away.
pub(super) fn toggle_binder(mut state: Signal<StudioState>, chrome: Signal<PanelChrome>, fold: Fold) {
    if state.peek().focus_mode {
        state.write().focus_mode = false;
    }
    if fold.binder {
        state.write().toggle_overlay(FoldedPanel::Binder);
    } else {
        toggle_sidebar(state, chrome);
    }
}

pub(super) fn toggle_inspector_panel(mut state: Signal<StudioState>, chrome: Signal<PanelChrome>, fold: Fold) {
    if state.peek().focus_mode {
        state.write().focus_mode = false;
    }
    if fold.inspector {
        state.write().toggle_overlay(FoldedPanel::Inspector);
    } else {
        toggle_inspector(state, chrome);
    }
}

/// Keeps `WindowWidth` current. Called from the window's event handler.
pub(super) fn track_window_width(mut window: Signal<WindowWidth>, size: dioxus::desktop::tao::dpi::PhysicalSize<u32>) {
    let scale = dioxus::desktop::window().scale_factor();
    let width = size.to_logical::<f64>(scale).width.round() as u32;
    if window.peek().0 != width {
        window.set(WindowWidth(width));
    }
}

pub(super) fn initial_window_width() -> WindowWidth {
    let desktop = dioxus::desktop::window();
    let size = desktop.inner_size().to_logical::<f64>(desktop.scale_factor());
    WindowWidth(size.width.round() as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_inspector_folds_before_the_binder() {
        let widths = PanelWidths::default();
        assert_eq!(fold_for(1400, widths), Fold::default());
        let below_inspector = fold_for(1000, widths);
        assert!(below_inspector.inspector && !below_inspector.binder);
        assert_eq!(fold_for(700, widths), Fold { binder: true, inspector: true });
        // A wider binder needs more room before it stays beside the page.
        let wide = PanelWidths { sidebar: SIDEBAR_WIDTH_MAX, ..widths };
        assert!(fold_for(900, wide).binder);
    }
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
            // Resizing is by mouse; Tab passes over it and the width stays as set.
            aria_hidden: "true",
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
