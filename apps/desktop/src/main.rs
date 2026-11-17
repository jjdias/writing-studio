use dioxus::desktop::{Config, LogicalSize, WindowBuilder};

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // The smallest window in which every mode still works, with both side
    // panels folded away.
    let window = WindowBuilder::new()
        .with_title("Writing Studio")
        .with_min_inner_size(LogicalSize::new(720.0, 520.0));
    // On Windows the webview's own file-drop handler takes every drag, and
    // the binder's drag and drop never sees one. The program takes no files
    // dropped from outside (import uses a file dialog), so it is turned off.
    let config = Config::new()
        .with_window(window)
        .with_disable_drag_drop_handler(true);
    dioxus::LaunchBuilder::desktop()
        .with_cfg(config)
        .launch(writing_studio_ui::App);
}
