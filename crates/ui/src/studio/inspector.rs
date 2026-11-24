use super::*;

/// Context: what is said about the chosen item, on the right. Never
/// the work itself, which is on the page, the map, or the outline.
#[component]
pub(super) fn ContextPane() -> Element {
    let widths = use_context::<Signal<PanelWidths>>();
    let width = widths.read().inspector;
    let state = use_context::<Signal<StudioState>>();
    let trash = state.read().binder.trash_root().map(|node| node.id);
    let trash_chosen = trash.is_some() && state.read().selected == trash;
    rsx! {
        aside {
            class: "secondary dashboard context",
            aria_label: "Context",
            style: "width: {width}px; flex: 0 0 {width}px;",
            if trash_chosen {
                div { class: "dash-heading", "Trash" }
                p { class: "hint", "What was moved here, under Trash in the binder. Choose an item to restore it." }
                TrashActions {}
            } else if let Some(entry) = state.read().entry_shown() {
                EntryContext { entry }
            } else {
                Inspector {}
            }
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
            div { class: "dash-heading", "{node.kind.icon_label()}" }
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
                label { r#for: "item-notes", "Notes" }
                textarea {
                    id: "item-notes",
                    class: "synopsis notes",
                    title: "What you want to remember about this. Kept in its file; never compiled or counted.",
                    value: "{node.notes}",
                    oninput: move |event| state.write().update_node_notes(node.id, event.value()),
                    onblur: move |_| state.write().save_all(),
                }
            }
            if matches!(node.kind, NodeKind::Scene | NodeKind::Chapter | NodeKind::Part | NodeKind::Book)
                && state.read().binder.in_manuscript(node.id)
            {
                NotesHere { id: node.id, kind: node.kind }
            }
            if node.kind == NodeKind::Scene && state.read().binder.in_manuscript(node.id) {
                SceneContextView { id: node.id }
            }
            if matches!(node.kind, NodeKind::Chapter | NodeKind::Part | NodeKind::Book | NodeKind::Folder)
                && (state.read().binder.in_manuscript(node.id) || state.read().binder.is_in_notes(node.id))
            {
                LevelContextView { id: node.id }
            }
            if node.kind == NodeKind::Note && state.read().binder.is_in_notes(node.id) {
                NoteLinks { id: node.id }
            }
            if node.kind == NodeKind::Note && state.read().session.as_ref().is_some_and(|session| Some(session.document_id()) == node.document_id) {
                button {
                    title: "Copy a file into the project and link to it where the caret is in this note",
                    onclick: move |_| {
                        let Some(path) = pick_file("Add a file to this note") else { return };
                        if let Some(link) = state.write().attach_file_to_note(node.id, path) {
                            let link = serde_json::to_string(&link).unwrap_or_else(|_| "\"\"".into());
                            page_call(format!("wsEditor.insertText({link})"));
                        }
                    },
                    "Add a file…"
                }
            }
            if !node.kind.is_root() && !state.read().binder.is_in_notes(node.id) && !state.read().binder.is_in_trash(node.id) {
                label { class: "check-label compile-check",
                    input {
                        r#type: "checkbox",
                        checked: !node.left_out,
                        onchange: move |event| state.write().set_left_out(node.id, !event.checked()),
                    }
                    "Include in compile"
                }
            }
            if node.kind == NodeKind::Folder && state.read().binder.is_in_notes(node.id) && !state.read().binder.is_in_trash(node.id) {
                label {
                    class: "check-label compile-check",
                    title: "A link in the text to a note in this folder becomes a numbered footnote with the note's text when the book is compiled",
                    input {
                        r#type: "checkbox",
                        checked: node.footnotes,
                        onchange: move |event| {
                            state.write().set_folder_footnotes(node.id, event.checked());
                            state.write().save_all();
                        },
                    }
                    "Compile as footnotes"
                }
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
            if node.kind == NodeKind::Scene && !state.read().binder.is_in_trash(node.id) {
                div { class: "scene-actions",
                    // The buttons keep the focus off themselves, so the
                    // caret is still in the manuscript when Split reads it.
                    button {
                        title: "Make a new scene from the caret to the end of this one",
                        onmousedown: move |event| event.prevent_default(),
                        onclick: move |_| split_at_caret(state),
                        "Split at the caret"
                    }
                    button {
                        title: "Join the next scene onto the end of this one",
                        disabled: !state.read().can_merge_with_next(),
                        onmousedown: move |event| event.prevent_default(),
                        onclick: move |_| state.write().merge_with_next(),
                        "Merge with next scene"
                    }
                }
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
        // Enter submits the form. A key handler on the field would also get
        // the event the page sends when a suggestion is picked, which is not
        // a key press and which Dioxus cannot read.
        form {
            class: "keyword-form",
            onsubmit: move |event| {
                event.prevent_default();
                let word = draft();
                state.write().add_keyword(id, &word);
                // A refused keyword stays in the field, to be corrected.
                if word.trim().is_empty() || tidy_keyword(&word).is_some() {
                    draft.set(String::new());
                }
            },
            input {
                class: "keyword-field",
                list: "known-keywords",
                placeholder: "Add a keyword",
                aria_label: "Add a keyword",
                value: "{draft}",
                oninput: move |event| draft.set(event.value()),
            }
        }
        datalist { id: "known-keywords",
            for word in known {
                option { value: "{word}" }
            }
        }
    }
}

/// Split the open scene where the caret is. The page's text is taken in the
/// same read, so the split cuts exactly what is on the page. The place is
/// counted in characters, as the document counts them, not in the page's
/// UTF-16 units.
/// The notes that reach an item, as chips that open them: those linked to
/// it or mentioned in its text, and apart, those linked to what it is in.
/// For the open scene, the notes its text names without linking them, to link
/// with one click. A typed title links another.
#[component]
fn NotesHere(id: Id, kind: NodeKind) -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let (own, through) = state.read().notes_reaching(id);
    let offered: Vec<String> = state
        .read()
        .note_titles()
        .into_iter()
        .filter(|title| !own.iter().any(|(_, kept, _)| kept == title))
        .collect();
    let mut draft = use_signal(String::new);
    let heading = format!("Notes in this {}", kind.icon_label().to_lowercase());
    // Found when the scene was opened or last saved, for the open scene only.
    let named: Vec<(Id, String)> = if state.read().selected == Some(id) {
        state
            .read()
            .named_here
            .iter()
            .filter_map(|note| state.read().binder.get(*note).map(|node| (*note, node.title.clone())))
            .collect()
    } else {
        Vec::new()
    };
    rsx! {
        div { class: "dash-heading", "{heading}" }
        if !own.is_empty() {
            div { class: "keyword-chips",
                for (note, title, linked) in own {
                    span { class: "keyword-chip story-chip", key: "{note}",
                        button {
                            class: "story-open",
                            title: if linked { "Open “{title}”, linked here" } else { "Open “{title}”, mentioned in the text" },
                            onclick: move |_| state.write().select_node(note),
                            "{title}"
                        }
                        if linked {
                            button {
                                class: "keyword-remove",
                                title: "Take the link to “{title}” away",
                                aria_label: "Unlink {title}",
                                onclick: move |_| state.write().set_note_link(note, id, false),
                                "×"
                            }
                        }
                    }
                }
            }
        }
        if !through.is_empty() {
            div { class: "keyword-chips",
                for (note, title, from) in through {
                    span { class: "keyword-chip story-chip from", key: "{note}", title: "Linked to {from}, which holds this",
                        button { class: "story-open", onclick: move |_| state.write().select_node(note), "{title}" }
                        span { class: "story-kind", "from {from}" }
                    }
                }
            }
        }
        if !named.is_empty() {
            div { class: "story-named",
                span { class: "hint", "Named here:" }
                for (note, title) in named {
                    span { class: "keyword-chip story-chip named", key: "{note}",
                        button {
                            class: "story-open",
                            title: "Link “{title}” to this scene",
                            onclick: move |_| state.write().link_named(note),
                            "+ {title}"
                        }
                        button {
                            class: "keyword-remove",
                            title: "Stop suggesting “{title}” for this scene",
                            aria_label: "Hide {title}",
                            onclick: move |_| state.write().hide_named(note),
                            "×"
                        }
                    }
                }
            }
        }
        form {
            class: "keyword-form",
            onsubmit: move |event| {
                event.prevent_default();
                let title = draft();
                if !title.trim().is_empty() {
                    state.write().link_note_titled(id, &title);
                    draft.set(String::new());
                }
            },
            input {
                class: "keyword-field",
                list: "note-titles",
                placeholder: "Link a note",
                aria_label: "Link a note by its title",
                value: "{draft}",
                oninput: move |event| draft.set(event.value()),
            }
        }
        datalist { id: "note-titles",
            for title in offered {
                option { value: "{title}" }
            }
        }
    }
}

#[component]
fn EntryContext(entry: StoryEntry) -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let id = entry.id;
    let labels = state.read().labels_in_use(entry.kind);
    rsx! {
        div { class: "dash-heading", "{entry.name}" }
        label { r#for: "entry-label-{id}", "What it is" }
        input {
            id: "entry-label-{id}",
            list: "entry-labels-{id}",
            placeholder: "Country, pub, father…",
            value: "{entry.label}",
            onchange: move |event| state.write().set_entry_label(id, event.value()),
        }
        datalist { id: "entry-labels-{id}",
            for label in labels {
                option { value: "{label}" }
            }
        }
        Connections { id }
        SceneLinks { owner: entry.id, kind: entry.kind }
    }
}

/// An entry's connections to the others: what it is inside and holds, the
/// fields that name another entry, and the ones the writer named, each way.
/// A named one can be renamed or taken away; a new one is added below.
#[component]
fn Connections(id: Id) -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let all = state.read().story_connections();
    let entries = state.read().story_entries();
    let name_of = |other: Id| entries.iter().find(|entry| entry.id == other).map(|entry| (entry.name.clone(), entry.kind.noun()));
    let mine: Vec<(writing_studio_context::Connection, bool, String, &str)> = all
        .into_iter()
        .filter(|tie| tie.from == id || tie.to == id)
        .filter_map(|tie| {
            let outgoing = tie.from == id;
            let (name, noun) = name_of(if outgoing { tie.to } else { tie.from })?;
            Some((tie, outgoing, name, noun))
        })
        .collect();
    let offered: Vec<String> = entries.iter().filter(|entry| entry.id != id).map(story_choice).collect();
    let names = state.read().connection_names();
    let mut target = use_signal(String::new);
    let mut name = use_signal(String::new);
    rsx! {
        section { class: "cast-block connections",
            h3 { "Connections" }
            if mine.is_empty() {
                p { class: "hint", "Not connected to another entry yet." }
            }
            for (tie, outgoing, other, noun) in mine {
                {
                    let key = format!("{}-{}-{}", tie.from, tie.to, tie.name);
                    let arrow = if outgoing { "→" } else { "←" };
                    let shown = match (tie.tie, outgoing) {
                        (writing_studio_context::Tie::Inside, false) => "holds".to_string(),
                        _ => tie.name.clone(),
                    };
                    rsx! {
                        div { class: "connection-row", key: "{key}",
                            span { class: "connection-name", "{arrow} {shown}" }
                            span { "{other}" }
                            span { class: "story-kind", "{noun}" }
                            if let writing_studio_context::Tie::Named(relation) = tie.tie {
                                button {
                                    class: "keyword-remove",
                                    title: "Take this connection away",
                                    onclick: move |_| state.write().remove_relationship(relation),
                                    "×"
                                }
                            }
                        }
                    }
                }
            }
            form {
                class: "connection-form",
                onsubmit: move |event| {
                    event.prevent_default();
                    state.write().connect_entries(id, &target(), &name());
                    target.set(String::new());
                    name.set(String::new());
                },
                input {
                    list: "connection-names-{id}",
                    placeholder: "lives in, son of…",
                    aria_label: "How they are connected",
                    value: "{name}",
                    oninput: move |event| name.set(event.value()),
                }
                input {
                    list: "connection-targets-{id}",
                    placeholder: "To whom or what",
                    aria_label: "The entry it is connected to",
                    value: "{target}",
                    oninput: move |event| target.set(event.value()),
                }
                button { r#type: "submit", "Connect" }
            }
            datalist { id: "connection-names-{id}",
                for one in names {
                    option { value: "{one}" }
                }
            }
            datalist { id: "connection-targets-{id}",
                for one in offered {
                    option { value: "{one}" }
                }
            }
        }
    }
}

/// The longest context "Copy context" writes, in characters: room for a
/// scene's whole cast without crowding out what an assistant writes back.
const CONTEXT_LIMIT: usize = 16_000;

/// The entries that reach the scene, each opening a short summary of its
/// text in place, and a button that copies the whole context as Markdown.
#[component]
fn SceneContextView(id: Id) -> Element {
    let state = use_context::<Signal<StudioState>>();
    let Some(context) = state.read().scene_context(id) else {
        return rsx! {};
    };
    let markdown = context.to_markdown(CONTEXT_LIMIT);
    rsx! {
        details { class: "inspector-section context",
            summary { "Context" }
            if context.entries.is_empty() {
                p { class: "hint", "No note reaches this scene yet. Write [[its title]] in the text, or link one to the scene or its chapter." }
            }
            for entry in context.entries.clone() {
                details { class: "context-entry", key: "{entry.id}",
                    summary {
                        span { "{entry.name}" }
                        span { class: "story-kind",
                            if let Some(from) = &entry.through { "{entry.folder}, from {from}" } else { "{entry.folder}" }
                        }
                    }
                    if entry.text.is_empty() {
                        p { class: "hint", "Nothing written about it yet." }
                    } else {
                        p { class: "context-text", "{short_summary(&entry.text)}" }
                    }
                }
            }
            button {
                title: "The scene, where it sits, its plot points, and everything linked to it, as Markdown",
                onclick: move |_| copy_to_clipboard(state, markdown.clone(), "Context copied."),
                "Copy context"
            }
        }
    }
}

/// A chapter's, part's, book's, or folder's context: what it holds, each
/// with its synopsis, and the entries in it, with how many of its scenes each
/// is in. **Copy context** gives the whole of it as Markdown.
#[component]
fn LevelContextView(id: Id) -> Element {
    let state = use_context::<Signal<StudioState>>();
    let Some(context) = state.read().level_context(id) else {
        return rsx! {};
    };
    let markdown = context.to_markdown(CONTEXT_LIMIT);
    let noun = context.kind.icon_label().to_lowercase();
    rsx! {
        details { class: "inspector-section context",
            summary { "Context" }
            if !context.about.is_empty() {
                p { class: "context-text", "{short_summary(&context.about)}" }
            }
            if context.outline.is_empty() {
                p { class: "hint", "Nothing in this {noun} yet." }
            } else {
                ul { class: "context-outline",
                    for (index, item) in context.outline.iter().enumerate() {
                        li { key: "{index}", class: if item.depth == 0 { "outline-top" } else { "outline-inner" },
                            span { class: "outline-title", "{item.title}" }
                            if !item.synopsis.is_empty() {
                                span { class: "context-text", " — {short_summary(&item.synopsis)}" }
                            }
                        }
                    }
                }
            }
            for (index, entry) in context.entries.iter().enumerate() {
                details { class: "context-entry", key: "{index}",
                    summary {
                        span { "{entry.name}" }
                        span { class: "story-kind",
                            match entry.scenes {
                                0 => format!("{}, linked here", entry.folder),
                                1 => format!("{}, in 1 scene", entry.folder),
                                many => format!("{}, in {many} scenes", entry.folder),
                            }
                        }
                    }
                    if entry.text.is_empty() {
                        p { class: "hint", "Nothing written about it yet." }
                    } else {
                        p { class: "context-text", "{short_summary(&entry.text)}" }
                    }
                }
            }
            button {
                title: "This {noun}, what it holds with each synopsis, its plot points, and everything in it, as Markdown",
                onclick: move |_| copy_to_clipboard(state, markdown.clone(), "Context copied."),
                "Copy context"
            }
        }
    }
}

/// A note's links: the items it is linked to from here, to take away or add
/// to, and the scenes and notes whose text links to it with `[[…]]`.
#[component]
fn NoteLinks(id: Id) -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let linked: Vec<(Id, String)> = state
        .read()
        .binder
        .get(id)
        .map(|node| node.links.clone())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|item| state.read().binder.get(item).map(|node| (item, format!("{}: {}", node.kind.icon_label(), node.title))))
        .collect();
    let mentions: Vec<(Id, String)> = state
        .read()
        .mentions_of(id)
        .into_iter()
        .filter_map(|item| state.read().binder.get(item).map(|node| (item, format!("{}: {}", node.kind.icon_label(), node.title))))
        .collect();
    // What can be linked: the manuscript's books, parts, chapters, and scenes.
    let choices: Vec<(Id, String)> = state
        .read()
        .binder
        .nodes
        .iter()
        .filter(|node| {
            matches!(node.kind, NodeKind::Book | NodeKind::Part | NodeKind::Chapter | NodeKind::Scene)
                && node.deleted_at.is_none()
                && !linked.iter().any(|(item, _)| *item == node.id)
        })
        .filter(|node| state.read().binder.in_manuscript(node.id))
        .map(|node| (node.id, format!("{}: {}", node.kind.icon_label(), node.title)))
        .collect();
    rsx! {
        label { "Linked to" }
        div { class: "note-links",
            if linked.is_empty() {
                p { class: "hint", "Not linked to a scene or chapter yet. Write [[{state.read().binder.get(id).map(|node| node.title.clone()).unwrap_or_default()}]] in one, or link it here." }
            }
            for (item, label) in linked {
                div { class: "note-link", key: "{item}",
                    button { class: "link-button", onclick: move |_| state.write().select_node(item), "{label}" }
                    button { title: "Take this link away", onclick: move |_| state.write().set_note_link(id, item, false), "Unlink" }
                }
            }
            select {
                aria_label: "Link this note to",
                onchange: move |event| {
                    if let Ok(item) = event.value().parse::<Id>() {
                        state.write().set_note_link(id, item, true);
                    }
                },
                option { value: "", selected: true, "Link to…" }
                for (item, label) in choices {
                    option { value: "{item}", "{label}" }
                }
            }
        }
        label { "Mentioned in" }
        div { class: "note-links",
            if mentions.is_empty() {
                p { class: "hint", "No text links to it yet." }
            }
            for (item, label) in mentions {
                button { class: "link-button note-link", key: "{item}", onclick: move |_| state.write().select_node(item), "{label}" }
            }
        }
    }
}

/// The start of an entry's text, up to a sentence or two.
fn short_summary(text: &str) -> String {
    const LONGEST: usize = 280;
    let first = text.split("\n\n").next().unwrap_or(text).trim();
    if first.chars().count() <= LONGEST {
        return first.to_string();
    }
    let cut: String = first.chars().take(LONGEST).collect();
    let at = cut.rfind(' ').unwrap_or(cut.len());
    format!("{}…", cut[..at].trim_end())
}

/// Put text on the clipboard through the page, and say how it went.
fn copy_to_clipboard(mut state: Signal<StudioState>, text: String, done: &'static str) {
    spawn(async move {
        let script = format!(
            "navigator.clipboard.writeText({}).then(() => dioxus.send(true), () => dioxus.send(false));",
            serde_json::to_string(&text).unwrap_or_else(|_| "\"\"".into())
        );
        let mut eval = document::eval(&script);
        let copied = eval.recv::<bool>().await.unwrap_or(false);
        state.write().status = if copied { done.into() } else { "Could not copy to the clipboard.".into() };
    });
}

pub(super) fn split_at_caret(mut state: Signal<StudioState>) {
    spawn(async move {
        let mut eval = document::eval("dioxus.send(window.wsEditor ? wsEditor.textBeforeCaret() : null);");
        match eval.recv::<Option<String>>().await {
            Ok(Some(before)) => state.write().split_at_markdown(&before),
            _ => state.write().status = "Click in the scene where the new scene should start, then Split.".into(),
        }
    });
}
