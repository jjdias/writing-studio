use super::*;

#[component]
pub(super) fn InspectorPane() -> Element {
    let widths = use_context::<Signal<PanelWidths>>();
    let width = widths.read().inspector;
    rsx! {
        aside {
            class: "secondary",
            style: "width: {width}px; flex: 0 0 {width}px;",
            div { class: "view-title", "Inspector" }
            Inspector {}
        }
    }
}

#[component]
pub(super) fn Inspector() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let node: Option<BinderNode> = state.read().selected_node().cloned();
    rsx! {
        if let Some(node) = node {
            label { "Title" }
            input {
                value: "{node.title}",
                oninput: move |event| state.write().update_selected_title(event.value()),
                onblur: move |_| state.write().save_all(),
            }
            label { "Synopsis" }
            textarea {
                class: "synopsis",
                value: "{node.synopsis}",
                oninput: move |event| state.write().update_selected_synopsis(event.value()),
                onblur: move |_| state.write().save_all(),
            }
            label { "Status" }
            input {
                value: "{node.status}",
                oninput: move |event| state.write().update_selected_status(event.value()),
                onblur: move |_| state.write().save_all(),
            }
            if state.read().has_scene_diagnostics(node.id) {
                SceneDiagnostics { node_id: node.id }
            }
            if !node.kind.is_root() {
                label { "Label" }
                input {
                    value: "{node.label}",
                    placeholder: "Short tag, such as subplot",
                    oninput: move |event| state.write().update_selected_label(event.value()),
                    onblur: move |_| state.write().save_all(),
                }
                label { "Color" }
                div { class: "swatch-row",
                    for swatch in BINDER_COLORS {
                        button {
                            class: if node.color == swatch.id { "swatch-btn active" } else { "swatch-btn" },
                            style: if swatch.hex.is_empty() { String::new() } else { format!("background: {}; color: #1c1915", swatch.hex) },
                            onclick: move |_| {
                                state.write().update_selected_color(swatch.id.to_string());
                                state.write().save_all();
                            },
                            "{swatch.name}"
                        }
                    }
                }
            }
            if state.read().binder.is_in_trash(node.id) && node.kind != NodeKind::TrashRoot {
                button { class: "primary", onclick: move |_| state.write().restore_selected(), "Restore from trash" }
            } else if !node.kind.is_root() {
                button { class: "danger", onclick: move |_| state.write().trash_selected(), "Move to trash" }
            }
            button { onclick: move |_| state.write().snapshot(), "Snapshot this document" }
            SnapshotList {}
        } else {
            p { class: "hint", "Select a binder item to edit metadata." }
        }
    }
}

pub(super) fn beat_label(beat: Beat) -> &'static str {
    match beat {
        Beat::Goal => "Goal",
        Beat::Conflict => "Conflict",
        Beat::Outcome => "Outcome",
    }
}

pub(super) fn beat_hint(beat: Beat) -> &'static str {
    match beat {
        Beat::Goal => "What the viewpoint character wants in this scene",
        Beat::Conflict => "What stands in the way",
        Beat::Outcome => "How the scene ends: won, lost, or changed",
    }
}

/// "Mara ×5, Rose ×2", for at most three characters.
pub(super) fn cast_summary(facts: &SceneFacts) -> String {
    let mut parts: Vec<String> = facts
        .characters
        .iter()
        .take(3)
        .map(|mention| format!("{} ×{}", mention.name, mention.count))
        .collect();
    if facts.characters.len() > 3 {
        parts.push(format!("and {} more", facts.characters.len() - 3));
    }
    parts.join(", ")
}

/// Goal, conflict, and outcome for the selected scene, with what the checks found.
#[component]
pub(super) fn SceneDiagnostics(node_id: Id) -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let Some(report) = state.read().scene_report(node_id) else {
        return rsx! {};
    };
    let facts = report.facts.clone();
    let cast = cast_summary(&facts);
    rsx! {
        div { class: "pane-title history-title", "Scene diagnostics" }
        for beat in [Beat::Goal, Beat::Conflict, Beat::Outcome] {
            label { "{beat_label(beat)}" }
            textarea {
                class: "synopsis beat",
                placeholder: "{beat_hint(beat)}",
                value: match beat {
                    Beat::Goal => report.beats.goal.clone(),
                    Beat::Conflict => report.beats.conflict.clone(),
                    Beat::Outcome => report.beats.outcome.clone(),
                },
                oninput: move |event| state.write().set_scene_beat(node_id, beat, event.value()),
                onblur: move |_| state.write().save_all(),
            }
        }
        if facts.words > 0 {
            p { class: "hint diag-facts",
                "{facts.words} words · {facts.dialogue_percent}% dialogue · {facts.average_sentence} words in an average sentence"
                if !cast.is_empty() {
                    " · {cast}"
                }
            }
        }
        if report.findings.is_empty() {
            p { class: "hint", "Nothing to flag in this scene." }
        } else {
            ul { class: "diag-list",
                for finding in report.findings.iter() {
                    li { class: if finding.is_beat() { "diag-beat" } else { "" }, "{finding.message()}" }
                }
            }
        }
    }
}

#[component]
pub(super) fn SnapshotList() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let snapshots = state.read().snapshots_for_selected();
    rsx! {
        div { class: "pane-title history-title", "Snapshots" }
        if snapshots.is_empty() {
            p { class: "hint", "No snapshots yet. Capture one to keep a local copy of this document." }
        } else {
            ul { class: "history-list",
                for item in snapshots {
                    li { class: "history-item",
                        div {
                            strong { "{item.label}" }
                            span { class: "card-meta", "{item.created_label()} · {item.word_count} words" }
                            p { class: "preview", "{item.preview}" }
                        }
                        div { class: "history-actions",
                            button {
                                title: "Show what changed since this snapshot, in History",
                                onclick: move |_| {
                                    let mut state = state.write();
                                    if state.compare_snapshot != Some(item.id) {
                                        state.compare_snapshot(item.id);
                                    }
                                    state.mode = WorkspaceMode::History;
                                },
                                "Compare"
                            }
                            button {
                                onclick: move |_| state.write().restore_snapshot(item.id),
                                "Restore"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub(super) fn SnapshotCompare() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let Some(result) = state.read().snapshot_comparison() else {
        return rsx! {};
    };
    let chosen = state.read().compare_snapshot;
    let against = state.read().compare_against;
    let changes_only = state.read().compare_changes_only;
    let others: Vec<_> = state
        .read()
        .snapshots_for_selected()
        .into_iter()
        .filter(|item| Some(item.id) != chosen)
        .collect();
    let against_value = against.map(|id| id.to_string()).unwrap_or_default();
    rsx! {
        div { class: "compare",
            div { class: "compare-bar",
                label { "Compare with" }
                select {
                    class: "compile-select",
                    title: "The other side of the comparison",
                    value: "{against_value}",
                    onchange: move |event| {
                        state.write().set_compare_against(event.value().parse::<Id>().ok());
                    },
                    option { value: "", "Current text" }
                    for item in others {
                        option { value: "{item.id}", "{item.label} · {item.created_label()}" }
                    }
                }
                label { class: "compare-toggle",
                    input {
                        r#type: "checkbox",
                        checked: changes_only,
                        onchange: move |event| state.write().compare_changes_only = event.checked(),
                    }
                    "Changed paragraphs only"
                }
                button { onclick: move |_| state.write().close_comparison(), "Close" }
            }
            match result {
                Err(err) => rsx! { p { class: "hint", "Could not compare: {err}" } },
                Ok(comparison) => rsx! {
                    p { class: "compare-summary",
                        strong { "{comparison.older}" }
                        " → "
                        strong { "{comparison.newer}" }
                        if comparison.diff.is_unchanged() {
                            " · The text is the same."
                        } else {
                            " · "
                            span { class: "diff-added", "+{comparison.diff.words_added} words" }
                            " "
                            span { class: "diff-removed", "−{comparison.diff.words_removed} words" }
                            " · {comparison.diff.changed_paragraphs()} paragraphs changed"
                        }
                    }
                    if !(changes_only && comparison.diff.is_unchanged()) {
                        div { class: "compare-body",
                            for paragraph in comparison.diff.paragraphs.iter().filter(|p| !changes_only || p.change != ParagraphChange::Same) {
                                p {
                                    class: match paragraph.change {
                                        ParagraphChange::Same => "diff-para",
                                        ParagraphChange::Changed => "diff-para changed",
                                        ParagraphChange::Added => "diff-para added",
                                        ParagraphChange::Removed => "diff-para removed",
                                    },
                                    for span in paragraph.spans.iter() {
                                        match span.op {
                                            DiffOp::Same => rsx! { "{span.text}" },
                                            DiffOp::Added => rsx! { ins { class: "diff-added", "{span.text}" } },
                                            DiffOp::Removed => rsx! { del { class: "diff-removed", "{span.text}" } },
                                        }
                                    }
                                }
                            }
                        }
                    }
                    p { class: "hint", "Text only. Formatting, links, and inline notes are not compared." }
                },
            }
        }
    }
}
