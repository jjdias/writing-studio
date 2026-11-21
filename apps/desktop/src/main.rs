// A release build on Windows opens no console window beside the program;
// a debug build keeps one for its log.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use dioxus::desktop::tao::window::Icon;
use dioxus::desktop::{Config, LogicalSize, WindowBuilder};

/// The window's icon as raw RGBA pixels, 64 by 64, made from
/// `icons/icon.svg`: raw pixels need no image decoder in the program.
const WINDOW_ICON: &[u8] = include_bytes!("../icons/window-64.rgba");

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
    let mut config = Config::new()
        .with_window(window)
        .with_disable_drag_drop_handler(true);
    if let Ok(icon) = Icon::from_rgba(WINDOW_ICON.to_vec(), 64, 64) {
        config = config.with_icon(icon);
    }
    dioxus::LaunchBuilder::desktop()
        .with_cfg(config)
        .launch(writing_studio_ui::App);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A file of the wrong size would leave the window with no icon, and
    /// nothing would say so.
    #[test]
    fn the_window_icon_is_64_pixels_square() {
        assert_eq!(WINDOW_ICON.len(), 64 * 64 * 4);
        assert!(Icon::from_rgba(WINDOW_ICON.to_vec(), 64, 64).is_ok());
        assert_eq!(WINDOW_ICON[3], 0, "the corners are clear");
    }
}
