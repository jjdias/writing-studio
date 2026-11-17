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
    dioxus::LaunchBuilder::desktop()
        .with_cfg(Config::new().with_window(window))
        .launch(writing_studio_ui::App);
}
