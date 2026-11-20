use super::*;

pub(super) fn create_project(
    mut state: Signal<StudioState>,
    chrome: Signal<PanelChrome>,
    widths: Signal<PanelWidths>,
) {
    let Some(folder) = pick_folder("Choose a folder for the new project") else {
        return;
    };
    state.write().create_project_in(folder);
    publish_layout(state, chrome, widths);
}

pub(super) fn open_project(
    mut state: Signal<StudioState>,
    chrome: Signal<PanelChrome>,
    widths: Signal<PanelWidths>,
) {
    let Some(folder) = pick_folder("Open a Writing Studio project folder") else {
        return;
    };
    state.write().open_project_at(folder);
    publish_layout(state, chrome, widths);
}

/// Compile the current book with its profile and write the file where the
/// writer chooses.
pub(super) fn export_project(mut state: Signal<StudioState>) {
    let Some(export) = state.write().prepare_compile() else {
        return;
    };
    let Some(path) = pick_save_file(export.filter_name, export.extension, export.file_name.clone()) else {
        return;
    };
    match std::fs::write(&path, export.bytes) {
        Ok(()) => {
            state.write().status = format!("Compiled {} to {}", export.kind_label, path.display());
        }
        Err(err) => state.write().status = format!("Could not write {}: {err}", export.kind_label),
    }
}

pub(super) fn pick_file(title: &str) -> Option<std::path::PathBuf> {
    let title = title.to_string();
    std::thread::spawn(move || rfd::FileDialog::new().set_title(title).pick_file())
        .join()
        .ok()
        .flatten()
}

pub(super) fn pick_markdown_files() -> Vec<std::path::PathBuf> {
    std::thread::spawn(|| {
        rfd::FileDialog::new()
            .set_title("Choose Markdown files to bring in as scenes")
            .add_filter("Markdown", &["md", "markdown", "txt"])
            .pick_files()
    })
    .join()
    .ok()
    .flatten()
    .unwrap_or_default()
}

pub(super) fn pick_folder(title: &str) -> Option<std::path::PathBuf> {
    let title = title.to_string();
    // The dialog runs on its own thread so the webview thread is not inside
    // the Windows file-dialog message loop. That loop was freezing the window.
    std::thread::spawn(move || rfd::FileDialog::new().set_title(title).pick_folder())
        .join()
        .ok()
        .flatten()
}

pub(super) fn pick_save_file(filter_name: &str, extension: &str, file_name: String) -> Option<std::path::PathBuf> {
    let filter_name = filter_name.to_string();
    let extension = extension.to_string();
    let title = format!("Compile {filter_name}");
    std::thread::spawn(move || {
        rfd::FileDialog::new()
            .set_title(title)
            .add_filter(filter_name, &[extension])
            .set_file_name(file_name)
            .save_file()
    })
    .join()
    .ok()
    .flatten()
}
