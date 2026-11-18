use super::*;

#[component]
pub(super) fn InspectorPane() -> Element {
    let widths = use_context::<Signal<PanelWidths>>();
    let width = widths.read().inspector;
    rsx! {
        aside {
            class: "secondary dashboard",
            aria_label: "Dashboard",
            style: "width: {width}px; flex: 0 0 {width}px;",
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
            // The title is the heading on the page, so it is renamed there.
            Progress {}
            div { class: "dash-heading", "Scene" }
            label { "Synopsis" }
            textarea {
                class: "synopsis",
                value: "{node.synopsis}",
                oninput: move |event| state.write().update_selected_synopsis(event.value()),
                onblur: move |_| state.write().save_all(),
            }
            label { r#for: "scene-status", "Status" }
            StatusSelect {
                id: "scene-status",
                current: node.status.clone(),
                any: false,
                onchange: move |status: String| {
                    state.write().update_selected_status(status);
                    state.write().save_all();
                },
            }
            if !node.kind.is_root() {
                Keywords { id: node.id }
            }
            if !node.kind.is_root() {
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
            if state.read().has_scene_diagnostics(node.id) {
                details { class: "inspector-section", open: true,
                    summary { "Beats and diagnostics" }
                    SceneDiagnostics { node_id: node.id }
                }
            }
            details { class: "inspector-section", open: true,
                summary { "Snapshots" }
                button { onclick: move |_| state.write().snapshot(), "Snapshot this document" }
                SnapshotList {}
            }
            // Set apart from the fields, at the bottom.
            if state.read().binder.is_in_trash(node.id) && node.kind != NodeKind::TrashRoot {
                div { class: "inspector-end",
                    button { class: "primary", onclick: move |_| state.write().restore_selected(), "Restore from trash" }
                }
            } else if !node.kind.is_root() {
                div { class: "inspector-end",
                    button { class: "danger", onclick: move |_| state.write().trash_selected(), "Move to trash" }
                }
            }
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
                    option { value: "", selected: against.is_none(), "Current text" }
                    for item in others {
                        option { value: "{item.id}", selected: against == Some(item.id), "{item.label} · {item.created_label()}" }
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

/// Words a reader gets through in a minute, for the reading time.
const READING_WORDS_PER_MINUTE: u32 = 230;

/// "1 min 18 sec", or "18 sec" under a minute.
pub(super) fn reading_time(words: u32) -> String {
    let seconds = words * 60 / READING_WORDS_PER_MINUTE;
    match (seconds / 60, seconds % 60) {
        (0, seconds) => format!("{seconds} sec"),
        (minutes, 0) => format!("{minutes} min"),
        (minutes, seconds) => format!("{minutes} min {seconds} sec"),
    }
}

/// The open scene's size, at the top of the dashboard.
#[component]
fn Progress() -> Element {
    let state = use_context::<Signal<StudioState>>();
    let words = state.read().word_count();
    let characters = state
        .read()
        .session
        .as_ref()
        .map(|session| session.plain_text().chars().count())
        .unwrap_or(0);
    rsx! {
        div { class: "dash-heading", "Progress" }
        dl { class: "dash-facts",
            dt { "Characters" } dd { "{characters}" }
            dt { "Words" } dd { "{words}" }
            dt { "Reading time" } dd { "{reading_time(words)}" }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reading_time_is_in_minutes_and_seconds() {
        assert_eq!(reading_time(0), "0 sec");
        assert_eq!(reading_time(115), "30 sec");
        assert_eq!(reading_time(230), "1 min");
        assert_eq!(reading_time(313), "1 min 21 sec");
    }
}

/// The item's keywords as colored chips. Clicking a chip's dot gives the
/// keyword the next color, everywhere; × takes it off this item. Enter in
/// the field adds one.
#[component]
fn Keywords(id: Id) -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let words: Vec<(String, String)> = {
        let current = state.read();
        current
            .binder
            .get(id)
            .map(|node| current.keyword_colors_of(node))
            .unwrap_or_default()
    };
    let known: Vec<String> = state.read().binder.keywords.iter().map(|keyword| keyword.name.clone()).collect();
    let mut draft = use_signal(String::new);
    rsx! {
        div { class: "dash-heading", "Keywords" }
        div { class: "keyword-chips",
            for (word, hex) in words {
                span { class: "keyword-chip", key: "{word}",
                    button {
                        class: "keyword-dot",
                        style: if hex.is_empty() { String::new() } else { format!("background: {hex}") },
                        title: "Give “{word}” the next color",
                        aria_label: "Change the color of {word}",
                        onclick: {
                            let word = word.clone();
                            move |_| state.write().cycle_keyword_color(&word)
                        },
                    }
                    span { "{word}" }
                    button {
                        class: "keyword-remove",
                        title: "Take “{word}” off this item",
                        aria_label: "Remove {word}",
                        onclick: {
                            let word = word.clone();
                            move |_| state.write().remove_keyword(id, &word)
                        },
                        "×"
                    }
                }
            }
        }
        input {
            class: "keyword-field",
            list: "known-keywords",
            placeholder: "Add a keyword",
            aria_label: "Add a keyword",
            value: "{draft}",
            oninput: move |event| draft.set(event.value()),
            onkeydown: move |event| {
                if event.key() == Key::Enter {
                    event.prevent_default();
                    let word = draft();
                    state.write().add_keyword(id, &word);
                    draft.set(String::new());
                }
            },
        }
        datalist { id: "known-keywords",
            for word in known {
                option { value: "{word}" }
            }
        }
    }
}
