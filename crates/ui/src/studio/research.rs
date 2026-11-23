use super::*;

#[derive(Clone, PartialEq)]
pub(super) struct ResearchNoteView {
    pub(super) id: Id,
    pub(super) title: String,
    pub(super) notes: String,
    pub(super) files: Vec<ResearchFileView>,
}

#[derive(Clone, PartialEq)]
pub(super) struct ResearchFileView {
    pub(super) id: Id,
    pub(super) file_name: String,
}

#[component]
pub(super) fn ResearchPane() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let snapshot = state.read().clone();
    let notes: Vec<NamedId> = snapshot
        .library
        .items_in_order()
        .into_iter()
        .map(|item| NamedId {
            id: item.id,
            name: item.title.clone(),
        })
        .collect();
    let shown = snapshot
        .selected_research
        .filter(|id| notes.iter().any(|note| note.id == *id))
        .or_else(|| notes.first().map(|note| note.id));
    let note = shown.and_then(|id| research_note(&snapshot.library, id));
    let confirm = snapshot.confirm_research;
    rsx! {
        div { class: "cast-pane",
            div { class: "cast-profile",
                if let Some(note) = note {
                    ResearchNote { note, confirm }
                } else {
                    EmptyState {
                        mode: WorkspaceMode::Research,
                        title: "No research notes yet",
                        body: "A research note keeps what you found and the files that go with it. Files are copied into this project's attachments folder.",
                        action: "Add note",
                        onclick: move |_| state.write().add_research_item(),
                    }
                }
            }
        }
    }
}

pub(super) fn research_note(library: &writing_studio_research::ResearchLibrary, id: Id) -> Option<ResearchNoteView> {
    let item = library.items.iter().find(|item| item.id == id)?;
    Some(ResearchNoteView {
        id: item.id,
        title: item.title.clone(),
        notes: item.notes.clone(),
        files: library
            .files_for(id)
            .into_iter()
            .map(|file| ResearchFileView {
                id: file.id,
                file_name: file.file_name.clone(),
            })
            .collect(),
    })
}

#[component]
pub(super) fn ResearchNote(note: ResearchNoteView, confirm: Option<Id>) -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let id = note.id;
    let title = note.title.clone();
    let notes = note.notes.clone();
    let files = note.files.clone();
    let delete_label = confirm_label(confirm == Some(id), "Delete", &title);
    rsx! {
        label { "Title" }
        input {
            value: "{title}",
            oninput: move |event| state.write().set_research_title(id, event.value()),
        }
        label { "Notes" }
        textarea {
            value: "{notes}",
            oninput: move |event| state.write().set_research_notes(id, event.value()),
        }
        section { class: "cast-block",
            h3 { "Files" }
            p { class: "hint", "A copy is stored in this project's attachments folder." }
            button {
                onclick: move |_| {
                    let Some(path) = pick_file("Attach a research file") else { return };
                    state.write().attach_research_file(id, path);
                },
                "Attach file"
            }
            if files.is_empty() {
                p { class: "hint", "No files attached." }
            }
            for file in files {
                {
                    let file_id = file.id;
                    let name = file.file_name.clone();
                    let remove_label = confirm_label(confirm == Some(file_id), "Remove", &name);
                    rsx! {
                        div { class: "cast-row", key: "{file_id}",
                            span { "{name}" }
                            button {
                                onclick: move |_| open_research_file(state, file_id),
                                "Open"
                            }
                            button {
                                class: danger(confirm == Some(file_id)),
                                onclick: move |_| state.write().remove_research_file(file_id),
                                "{remove_label}"
                            }
                        }
                    }
                }
            }
        }
        div { class: "cast-actions",
            button {
                class: danger(confirm == Some(id)),
                onclick: move |_| state.write().remove_research_item(id),
                "{delete_label}"
            }
        }
    }
}

pub(super) fn open_research_file(mut state: Signal<StudioState>, file_id: Id) {
    let Some(path) = state.read().research_file_path(file_id) else {
        state.write().status = "That attachment is missing.".into();
        return;
    };
    if !path.is_file() {
        state.write().status = format!("File is not in attachments/: {}", path.display());
        return;
    }
    match open_with_default_app(&path) {
        Ok(()) => state.write().status = format!("Opened {}", path.file_name().and_then(|name| name.to_str()).unwrap_or("file")),
        Err(err) => state.write().status = format!("Could not open file: {err}"),
    }
}

pub(super) fn open_with_default_app(path: &std::path::Path) -> std::io::Result<()> {
    #[cfg(target_os = "windows")]
    {
        return windows_shell::open(path);
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(path).spawn()?;
        return Ok(());
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open").arg(path).spawn()?;
        return Ok(());
    }
    #[allow(unreachable_code)]
    Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "cannot open files on this system"))
}

/// Opening a file as Explorer does, with `ShellExecuteW`. The path goes to
/// Windows as one value, so `&`, `%`, `^` and the rest stay part of the name;
/// through `cmd /C start` they were commands and variables, and a synced
/// project could name an attachment that ran one.
#[cfg(target_os = "windows")]
mod windows_shell {
    use std::ffi::{c_void, OsStr};
    use std::io;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    const SW_SHOWNORMAL: i32 = 1;

    #[link(name = "shell32")]
    extern "system" {
        fn ShellExecuteW(
            hwnd: *mut c_void,
            operation: *const u16,
            file: *const u16,
            parameters: *const u16,
            directory: *const u16,
            show: i32,
        ) -> *mut c_void;
    }

    fn wide(text: &OsStr) -> Vec<u16> {
        text.encode_wide().chain(std::iter::once(0)).collect()
    }

    pub(super) fn open(path: &Path) -> io::Result<()> {
        let operation = wide(OsStr::new("open"));
        let file = wide(path.as_os_str());
        // SAFETY: both strings are NUL-terminated and live until the call
        // returns; the null pointers are the documented "none" values.
        let result = unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                operation.as_ptr(),
                file.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                SW_SHOWNORMAL,
            )
        };
        // Above 32 is success; anything else is one of the codes below.
        match result as usize {
            code if code > 32 => Ok(()),
            code => Err(error(code)),
        }
    }

    fn error(code: usize) -> io::Error {
        match code {
            2 | 3 => io::Error::new(io::ErrorKind::NotFound, "the file is not there"),
            5 => io::Error::new(io::ErrorKind::PermissionDenied, "Windows refused access to the file"),
            31 => io::Error::new(io::ErrorKind::Unsupported, "no program is set to open this kind of file"),
            code => io::Error::other(format!("Windows could not open it (code {code})")),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        /// `cmd /C start` reported success whatever happened, and cut a path
        /// at `&`. A missing file under a folder named with `&` and `%` must
        /// come back as not found, which shows Windows looked up the whole
        /// path. Nothing opens, as the file does not exist.
        #[test]
        fn a_path_with_ampersand_and_percent_is_one_path() {
            let folder = std::env::temp_dir().join("Tom&Jerry%USERNAME%^draft");
            let missing = folder.join("missing&calc.pdf");
            let err = open(&missing).expect_err("a missing file cannot open");
            assert_eq!(err.kind(), io::ErrorKind::NotFound);
        }

        #[test]
        fn codes_become_readable_errors() {
            assert_eq!(error(2).kind(), io::ErrorKind::NotFound);
            assert_eq!(error(31).kind(), io::ErrorKind::Unsupported);
            assert!(error(8).to_string().contains("code 8"));
        }
    }
}
