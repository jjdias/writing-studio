use super::*;

#[component]
pub(super) fn HistoryPane() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let chrome = use_context::<Signal<PanelChrome>>();
    let widths = use_context::<Signal<PanelWidths>>();
    let snapshots = state.read().snapshots_for_selected();
    let comparing = state.read().compare_snapshot;
    let backups = state.read().backups();
    let confirm = state.read().confirm_backup.clone();
    let has_text = state.read().selected_node().is_some_and(|node| node.document_id.is_some());
    let heading = state
        .read()
        .selected_node()
        .map(|node| node.title.clone())
        .unwrap_or_else(|| "No document selected".into());
    rsx! {
        div { class: "history-pane",
            section {
                h2 { "Document snapshots" }
                p { class: "hint", "Local copies of “{heading}”. Compare shows what changed since a snapshot. Restoring writes the previous text as a pre-restore snapshot." }
                if snapshots.is_empty() {
                    p { class: "hint", "Select a scene or note, then snapshot it from the inspector." }
                } else {
                    ul { class: "history-list wide",
                        for item in snapshots {
                            li { class: "history-item",
                                div {
                                    strong { "{item.label}" }
                                    span { class: "card-meta", "{item.created_label()} · {item.word_count} words" }
                                    p { class: "preview", "{item.preview}" }
                                }
                                div { class: "history-actions",
                                    button {
                                        class: if comparing == Some(item.id) { "primary" } else { "" },
                                        title: "Show what changed since this snapshot",
                                        onclick: move |_| state.write().compare_snapshot(item.id),
                                        if comparing == Some(item.id) { "Comparing" } else { "Compare" }
                                    }
                                    button {
                                        onclick: move |_| state.write().restore_snapshot(item.id),
                                        "Restore"
                                    }
                                }
                            }
                        }
                    }
                    if comparing.is_some() {
                        SnapshotCompare {}
                    }
                }
            }
            EarlierSyncs {}
            KeptFromOutside {}
            OpenProblems {}
            SyncSection {}
            section {
                h2 { "Writing sessions" }
                p { class: "hint", "Sittings saved in this project. Today’s count starts again each local day." }
                WritingSessions {}
            }
            section {
                h2 { "Project backups" }
                p { class: "hint", "Copies of this project in its backups folder. Restore this scene puts back the selected scene only. Restore all replaces the whole project and asks for a second click." }
                div { class: "binder-tools",
                    button { class: "primary", onclick: move |_| state.write().backup(), "Write backup now" }
                }
                if backups.is_empty() {
                    p { class: "hint", "No backups yet. Use Backup in the toolbar or the button above." }
                } else {
                    ul { class: "history-list wide",
                        for item in backups {
                            li { class: "history-item",
                                div {
                                    strong { "{item.file_name}" }
                                    span { class: "card-meta", "{item.modified_label} · {item.size_label()}" }
                                }
                                div { class: "history-actions",
                                    if has_text {
                                        button {
                                            title: "Put back only “{heading}” as it was in this backup",
                                            onclick: {
                                                let path = item.path.clone();
                                                move |_| state.write().restore_scene_from_backup(path.clone())
                                            },
                                            "Restore this scene"
                                        }
                                    }
                                    button {
                                        class: danger(confirm.as_ref() == Some(&item.path)),
                                        title: "Replace the whole project with this backup",
                                        onclick: move |_| {
                                            state.write().request_backup_restore(item.path.clone());
                                            publish_layout(state, chrome, widths);
                                        },
                                        if confirm.as_ref() == Some(&item.path) { "Confirm restore" } else { "Restore all" }
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

/// The selected scene's versions at earlier syncs, from git.
#[component]
pub(super) fn EarlierSyncs() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let revisions = state.read().scene_revisions();
    if revisions.is_empty() {
        return rsx! {};
    }
    let comparing = state.read().compare_revision.clone();
    rsx! {
        section {
            h2 { "Earlier syncs" }
            p { class: "hint", "This scene as it was each time it was synced. Restore is one step that Ctrl+Z undoes." }
            ul { class: "history-list wide",
                for revision in revisions {
                    li { class: "history-item",
                        div {
                            strong { "{revision.date}" }
                            p { class: "preview", "{revision.message}" }
                        }
                        div { class: "history-actions",
                            button {
                                class: if comparing.as_ref() == Some(&Earlier::Sync(revision.commit.clone())) { "primary" } else { "" },
                                onclick: {
                                    let earlier = Earlier::Sync(revision.commit.clone());
                                    move |_| state.write().compare_with_revision(earlier.clone())
                                },
                                if comparing.as_ref() == Some(&Earlier::Sync(revision.commit.clone())) { "Comparing" } else { "Compare" }
                            }
                            button {
                                onclick: {
                                    let earlier = Earlier::Sync(revision.commit.clone());
                                    move |_| state.write().restore_revision(&earlier)
                                },
                                "Restore"
                            }
                        }
                    }
                }
            }
            if matches!(comparing, Some(Earlier::Sync(_))) {
                SnapshotCompare {}
            }
        }
    }
}

/// Files changed outside the program and kept beside the one the program
/// wrote. The selected scene's can be compared and restored.
#[component]
pub(super) fn KeptFromOutside() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let kept = state.read().kept_files();
    if kept.is_empty() {
        return rsx! {};
    }
    let comparing = state.read().compare_revision.clone();
    let confirm = state.read().confirm_kept.clone();
    let showing = matches!(comparing, Some(Earlier::Kept(_)));
    rsx! {
        section {
            h2 { "Kept from outside" }
            p { class: "hint", "A file changed outside the program while it was open, kept beside the one the program wrote. Select a scene to compare or restore its kept versions." }
            ul { class: "history-list wide",
                for (item, mine) in kept {
                    li { class: "history-item",
                        div {
                            strong { "{item.kept_as}" }
                            span { class: "card-meta", "Kept beside {item.file}" }
                        }
                        div { class: "history-actions",
                            if mine {
                                button {
                                    class: if comparing == Some(Earlier::Kept(item.kept_as.clone())) { "primary" } else { "" },
                                    onclick: {
                                        let earlier = Earlier::Kept(item.kept_as.clone());
                                        move |_| state.write().compare_with_revision(earlier.clone())
                                    },
                                    if comparing == Some(Earlier::Kept(item.kept_as.clone())) { "Comparing" } else { "Compare" }
                                }
                                button {
                                    onclick: {
                                        let earlier = Earlier::Kept(item.kept_as.clone());
                                        move |_| state.write().restore_revision(&earlier)
                                    },
                                    "Restore"
                                }
                            }
                            button {
                                class: danger(confirm.as_ref() == Some(&item.kept_as)),
                                onclick: {
                                    let kept_as = item.kept_as.clone();
                                    move |_| state.write().request_kept_removal(kept_as.clone())
                                },
                                if confirm.as_ref() == Some(&item.kept_as) { "Confirm delete" } else { "Delete" }
                            }
                        }
                    }
                }
            }
            if showing {
                SnapshotCompare {}
            }
        }
    }
}

/// Run a sync off the window's thread, then take in what it did.
pub(super) fn run_sync(mut state: Signal<StudioState>) {
    let Some(root) = state.write().begin_sync() else {
        return;
    };
    spawn(async move {
        let result = tokio::task::spawn_blocking(move || writing_studio_synchronization::git::sync(&root))
            .await
            .unwrap_or_else(|err| Err(writing_studio_core::Error::message(err.to_string())));
        state.write().finish_sync(result);
    });
}

/// Connecting the project to a git repository, and where it stands.
#[component]
pub(super) fn SyncSection() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let status = state.read().sync_status();
    let conflicts = state.read().sync_conflicts.clone();
    let address = state.read().sync_address.clone();
    let syncing = state.read().syncing;
    rsx! {
        section {
            h2 { "Sync" }
            if !writing_studio_synchronization::git::available() {
                p { class: "hint", "Sync uses git, which is not installed on this computer. Everything else works without it." }
            } else if let Some(status) = status {
                p { class: "hint",
                    "Connected to {status.remote}. "
                    {match status.last_synced.and_then(|time| time.elapsed().ok()) {
                        Some(ago) => format!("Last synced {} ago. ", format_elapsed(ago.as_secs())),
                        None => "Not synced yet. ".to_string(),
                    }}
                    {match (status.to_send, status.to_receive) {
                        (0, 0) => "Nothing to send or receive.".to_string(),
                        (send, 0) => format!("{send} to send."),
                        (0, receive) => format!("{receive} to receive."),
                        (send, receive) => format!("{send} to send, {receive} to receive."),
                    }}
                }
                div { class: "binder-tools",
                    button { class: "primary", disabled: syncing, onclick: move |_| run_sync(state), if syncing { "Syncing…" } else { "Sync now" } }
                }
            } else {
                p { class: "hint",
                    "Keep this project on another computer, or with a git host, by connecting it to a git repository. Use a private repository: anyone who can read the repository can read the book. The guide in documentation/SYNC.md shows how to make one."
                }
                input {
                    class: "search",
                    placeholder: "Repository address, such as git@host:you/book.git",
                    value: "{address}",
                    oninput: move |event| state.write().sync_address = event.value(),
                }
                div { class: "binder-tools",
                    button { class: "primary", onclick: move |_| state.write().connect_sync(), "Connect" }
                }
            }
            if !conflicts.is_empty() {
                p { class: "hint", "Changed on both machines in the last sync. Paragraphs kept in both versions are in the track-changes panel of their scene, to accept or reject." }
                ul { class: "history-list wide",
                    for conflict in conflicts {
                        li { class: "history-item", "{conflict}" }
                    }
                }
            }
        }
    }
}

/// What was wrong with the files when the project opened, and what was done.
#[component]
pub(super) fn OpenProblems() -> Element {
    let problems = use_context::<Signal<StudioState>>().read().open_problems();
    if problems.is_empty() {
        return rsx! {};
    }
    rsx! {
        section {
            h2 { "Found when the project opened" }
            p { class: "hint",
                "The project's files were changed outside the program. Items that came in are in Recovered at the top of the manuscript. An item marked missing keeps its place, and its file is never written over: restore the file, then open the project again."
            }
            ul { class: "history-list wide",
                for problem in problems {
                    li { class: "history-item", "{problem}" }
                }
            }
        }
    }
}

#[component]
pub(super) fn WritingSessions() -> Element {
    let sessions = use_context::<Signal<StudioState>>().read().stored_sessions();
    rsx! {
        if sessions.is_empty() {
            p { class: "hint", "None yet. This sitting and today’s words stay on the status bar. A sitting is stored here the next time you open the project, if it added words." }
        } else {
            ul { class: "history-list wide",
                for item in sessions {
                    li { class: "history-item",
                        div {
                            strong { "{item.day}" }
                            span { class: "card-meta", "{item.words} words · {format_elapsed(item.seconds)}" }
                        }
                    }
                }
            }
        }
    }
}
