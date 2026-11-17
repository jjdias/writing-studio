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
            aside { class: "cast-list",
                button {
                    class: "primary",
                    onclick: move |_| state.write().add_research_item(),
                    "Add note"
                }
                if notes.is_empty() {
                    p { class: "hint", "No research notes yet." }
                }
                for note in notes {
                    {
                        let id = note.id;
                        let active = shown == Some(id);
                        let class = if active { "cast-name active" } else { "cast-name" };
                        rsx! {
                            button {
                                class: "{class}",
                                key: "{id}",
                                onclick: move |_| state.write().select_research_item(id),
                                "{note.name}"
                            }
                        }
                    }
                }
            }
            div { class: "cast-profile",
                if let Some(note) = note {
                    ResearchNote { note, confirm }
                } else {
                    p { class: "hint", "Research notes and their files stay in this project. Files are copied into attachments/." }
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
        SceneLinks { owner: id, kind: BibleLink::Research }
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
        std::process::Command::new("cmd")
            .args(["/C", "start", "", &path.display().to_string()])
            .spawn()?;
        return Ok(());
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
