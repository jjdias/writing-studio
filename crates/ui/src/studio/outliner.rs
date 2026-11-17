use super::*;

#[component]
pub(super) fn OutlinerPane() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let snapshot = state.read().clone();
    let mut books: Vec<OwnedOutlineBook> = Vec::new();
    let scope = snapshot.series_scope();
    for line in outline_rows(&snapshot.binder) {
        if scope.is_some_and(|book| line.book_id != book) {
            continue;
        }
        let Some(chapter) = snapshot.binder.get(line.chapter_id) else {
            continue;
        };
        let scene = line.scene_id.and_then(|id| snapshot.binder.get(id));
        let meta = scene.unwrap_or(chapter);
        let row = OwnedOutlineRow {
            chapter_id: line.chapter_id,
            scene_id: line.scene_id,
            open_id: meta.id,
            chapter_title: chapter.title.clone(),
            scene_title: scene.map(|node| node.title.clone()).unwrap_or_default(),
            status: meta.status.clone(),
            synopsis: meta.synopsis.clone(),
            words: row_word_count(line.document_id, |id| words_for_document(&snapshot, id)),
        };
        if books.last().map(|book| book.id) != Some(line.book_id) {
            let title = snapshot
                .binder
                .get(line.book_id)
                .map(|node| node.title.clone())
                .unwrap_or_default();
            books.push(OwnedOutlineBook {
                id: line.book_id,
                title,
                rows: Vec::new(),
            });
        }
        if let Some(book) = books.last_mut() {
            book.rows.push(row);
        }
    }
    let diagnostics = snapshot.outliner_diagnostics;
    rsx! {
        div { class: "outliner",
            div { class: "cork-tools",
                SeriesScopeSelect {}
                button {
                    class: if diagnostics { "" } else { "primary" },
                    onclick: move |_| state.write().outliner_diagnostics = false,
                    "Outline"
                }
                button {
                    class: if diagnostics { "primary" } else { "" },
                    title: "Goal, conflict, and outcome for each scene, with checks on its text",
                    onclick: move |_| state.write().outliner_diagnostics = true,
                    "Diagnostics"
                }
            }
            if books.is_empty() {
                p { class: "hint", "Add a chapter in the binder to outline it here." }
            }
            if diagnostics {
                DiagnosticsTables { books: books.clone() }
            }
            for book in books.into_iter().filter(|_| !diagnostics) {
                h3 { "{book.title}" }
                table {
                    thead {
                        tr {
                            th { "Chapter" }
                            th { "Scene" }
                            th { "Status" }
                            th { "Synopsis" }
                            th { "Words" }
                            th {}
                        }
                    }
                    tbody {
                        for row in book.rows {
                            tr {
                                td {
                                    input {
                                        class: "cell-input",
                                        value: "{row.chapter_title}",
                                        oninput: move |event| {
                                            state.write().update_node_title(row.chapter_id, event.value());
                                        },
                                    }
                                }
                                td {
                                    if let Some(scene_id) = row.scene_id {
                                        input {
                                            class: "cell-input",
                                            value: "{row.scene_title}",
                                            oninput: move |event| {
                                                state.write().update_node_title(scene_id, event.value());
                                            },
                                        }
                                    } else {
                                        span { class: "hint", "—" }
                                    }
                                }
                                td {
                                    input {
                                        class: "cell-input",
                                        value: "{row.status}",
                                        oninput: move |event| {
                                            state.write().update_node_status(row.open_id, event.value());
                                        },
                                    }
                                }
                                td {
                                    input {
                                        class: "cell-input",
                                        value: "{row.synopsis}",
                                        oninput: move |event| {
                                            state.write().update_node_synopsis(row.open_id, event.value());
                                        },
                                    }
                                }
                                td { class: "words", "{row.words}" }
                                td {
                                    button {
                                        onclick: move |_| {
                                            state.write().select_node(row.open_id);
                                            state.write().mode = WorkspaceMode::Writing;
                                        },
                                        "Open"
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

/// Every scene of each book on screen, with its beats and what the checks found.
#[component]
pub(super) fn DiagnosticsTables(books: Vec<OwnedOutlineBook>) -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    // Read the scenes once when the table opens. Text cannot change while it
    // is on screen, because the manuscript is not.
    use_hook(move || {
        spawn(async move { state.write().load_diagnostic_texts() });
    });
    let snapshot = state.read().clone();
    rsx! {
        p { class: "hint",
            "Goal, conflict, and outcome are yours to write. The checks count what is in the text on this computer; they do not read meaning."
        }
        for book in books {
            DiagnosticsBook { context: snapshot.book_context(book.id), book }
        }
    }
}

#[component]
pub(super) fn DiagnosticsBook(book: OwnedOutlineBook, context: BookContext) -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let snapshot = state.read().clone();
    let rows: Vec<_> = book
        .rows
        .iter()
        .filter_map(|row| Some((row.clone(), snapshot.scene_report_in(row.open_id, &context)?)))
        .collect();
    let total = rows.len();
    let complete = rows.iter().filter(|(_, report)| report.beats.is_complete()).count();
    let typical = if context.typical_words > 0 {
        format!(" · a scene is usually about {} words", context.typical_words)
    } else {
        String::new()
    };
    rsx! {
        h3 { "{book.title}" }
        p { class: "hint", "{complete} of {total} scenes have a goal, a conflict, and an outcome{typical}" }
        table { class: "diag-table",
            thead {
                tr {
                    th { "Scene" }
                    th { "Goal" }
                    th { "Conflict" }
                    th { "Outcome" }
                    th { "Words" }
                    th { "Dialogue" }
                    th { "Named" }
                    th { "Checks" }
                    th {}
                }
            }
            tbody {
                for (row, report) in rows {
                    tr { key: "{row.open_id}",
                        td { class: "diag-scene",
                            span { class: "card-meta", "{row.chapter_title}" }
                            if !row.scene_title.is_empty() {
                                div { "{row.scene_title}" }
                            }
                        }
                        for beat in [Beat::Goal, Beat::Conflict, Beat::Outcome] {
                            td {
                                input {
                                    class: "cell-input",
                                    title: "{beat_hint(beat)}",
                                    value: match beat {
                                        Beat::Goal => report.beats.goal.clone(),
                                        Beat::Conflict => report.beats.conflict.clone(),
                                        Beat::Outcome => report.beats.outcome.clone(),
                                    },
                                    oninput: move |event| state.write().set_scene_beat(row.open_id, beat, event.value()),
                                    onblur: move |_| state.write().save_all(),
                                }
                            }
                        }
                        td { class: "words", "{report.facts.words}" }
                        td { class: "words",
                            if report.facts.words > 0 { "{report.facts.dialogue_percent}%" }
                        }
                        td { class: "diag-cast", "{cast_summary(&report.facts)}" }
                        td {
                            if report.findings.is_empty() {
                                span { class: "hint", "—" }
                            } else {
                                ul { class: "diag-list",
                                    for finding in report.findings.iter() {
                                        li { class: if finding.is_beat() { "diag-beat" } else { "" }, "{finding.message()}" }
                                    }
                                }
                            }
                        }
                        td {
                            button {
                                onclick: move |_| {
                                    state.write().select_node(row.open_id);
                                    state.write().mode = WorkspaceMode::Writing;
                                },
                                "Open"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Clone, PartialEq)]
pub(super) struct OwnedOutlineBook {
    pub(super) id: Id,
    pub(super) title: String,
    pub(super) rows: Vec<OwnedOutlineRow>,
}

#[derive(Clone, PartialEq)]
pub(super) struct OwnedOutlineRow {
    pub(super) chapter_id: Id,
    pub(super) scene_id: Option<Id>,
    pub(super) open_id: Id,
    pub(super) chapter_title: String,
    pub(super) scene_title: String,
    pub(super) status: String,
    pub(super) synopsis: String,
    pub(super) words: u32,
}
