use super::*;

#[component]
pub(super) fn EditorPane() -> Element {
    let state = use_context::<Signal<StudioState>>();
    let snapshot = state.read().clone();
    let node = snapshot.selected_node().cloned();
    let words = snapshot.word_count();
    let focus = snapshot.focus_mode;
    let can_edit = snapshot.session.is_some() && !snapshot.syncing;
    let side = snapshot.review_at_side && !snapshot.review_folded;
    let scene = snapshot.session.as_ref().map(|session| session.document_id());
    let marks = use_signal(Vec::<String>::new);
    let prompt = use_signal(|| None::<InlinePrompt>);
    // The words to underline. An empty list clears the underlines, which is
    // also what a project with spellcheck off sends.
    let unknown: Vec<UnknownWord> = snapshot.unknown_words();
    let unknown_json = serde_json::to_string(
        &unknown.iter().map(|item| item.word.as_str()).collect::<Vec<_>>(),
    )
    .unwrap_or_else(|_| "[]".into());
    // The notes a `[[link]]` can name, with each one's tip.
    let notes_json = serde_json::to_string(
        &snapshot
            .note_link_targets()
            .into_iter()
            .map(|(title, tip)| serde_json::json!({ "title": title, "tip": tip }))
            .collect::<Vec<_>>(),
    )
    .unwrap_or_else(|_| "[]".into());
    let keep = snapshot.keep_place();
    let keep_json = format!(
        r#"{{"scroll":"{}","marker":{},"fade":"{}","bars":{}}}"#,
        keep.scroll.as_str(),
        keep.line_marker,
        keep.fade.as_str(),
        keep.hide_while_typing
    );

    rsx! {
        div { class: "editor-pane",
            if !focus {
                FormatToolbar { enabled: can_edit, marks, prompt }
            }
            div { class: if side { "editor-body side" } else { "editor-body" },
                div { class: if keep.scroll == FixedScroll::Off { "desk" } else { "desk fixed-scroll" },
                    // The scene's title is the page's heading, renamed in
                    // place, with the word count small at its right.
                    div { class: "page-head",
                        if let Some(node) = node.clone() {
                            PageTitle { key: "{node.id}-{node.title}", id: node.id, title: node.title.clone(), editable: !node.kind.is_root() }
                        } else {
                            h1 { class: "page-title", "Choose a scene" }
                        }
                        span { class: "page-words", "{words} words" }
                    }
                    if let Some(scene) = scene {
                        MarkdownPage { key: "{scene}", scene, can_edit, marks, spelling: unknown_json.clone(), keep: keep_json.clone(), fading: keep.fade != Fade::Off, notes: notes_json.clone() }
                    } else {
                        div { class: "manuscript markdown-page", aria_label: "Manuscript" }
                    }
                }
                if !focus {
                    ReviewPanel { unknown }
                }
                if let Some(at) = prompt() {
                    PromptField { key: "{at.x}-{at.y}", at, prompt }
                }
            }
        }
    }
}

/// The scene as Markdown, in CodeMirror (`window.wsEditor`, loaded by
/// `App`). One editor for each scene: its text comes from the session, and
/// every change goes back through `crates/format` into the document.
#[component]
fn MarkdownPage(
    scene: Id,
    can_edit: bool,
    marks: Signal<Vec<String>>,
    spelling: String,
    keep: String,
    fading: bool,
    notes: String,
) -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let mut marks = marks;
    let epoch = state.read().editor_epoch;
    let token = serde_json::to_string(&scene.to_string()).unwrap_or_else(|_| "\"\"".into());
    let mount_token = token.clone();
    let mount_keep = keep.clone();
    use_hook(move || {
        spawn(async move {
            // Wait for the element to be on the page.
            tokio::time::sleep(std::time::Duration::from_millis(16)).await;
            let mut eval = document::eval(&format!(
                r#"
                const [text, options] = await dioxus.recv();
                const el = document.getElementById('markdown-page');
                if (window.wsEditor && el) {{
                    wsEditor.mount(el, text, (message) => dioxus.send(message), {mount_token}, options);
                }}
                "#
            ));
            let current = state.peek();
            let mut options = page_options(&current.markdown_options());
            options["keep"] = serde_json::from_str(&mount_keep).unwrap_or_default();
            let _ = eval.send((current.editor_markdown(), options));
            drop(current);
            // What the page sends: see `crates/ui/codemirror/entry.js`.
            while let Ok(message) = eval.recv::<serde_json::Value>().await {
                match message["kind"].as_str() {
                    Some("text") => state.write().apply_editor_markdown(message["text"].as_str().unwrap_or_default()),
                    Some("blur") => state.write().flush_editor(),
                    Some("open-note") => state.write().open_note_titled(message["title"].as_str().unwrap_or_default()),
                    Some("marks") => {
                        let next: Vec<String> = message["marks"]
                            .as_array()
                            .map(|list| list.iter().filter_map(|mark| mark.as_str().map(str::to_owned)).collect())
                            .unwrap_or_default();
                        if *marks.peek() != next {
                            marks.set(next);
                        }
                    }
                    Some("paste") => {
                        let text = pasted_markdown(message["html"].as_str().unwrap_or_default());
                        let text = serde_json::to_string(&text).unwrap_or_else(|_| "\"\"".into());
                        let (from, to) = (message["from"].as_u64().unwrap_or(0), message["to"].as_u64().unwrap_or(0));
                        let _ = document::eval(&format!("window.wsEditor && wsEditor.pasted({from}, {to}, {text});"));
                    }
                    _ => {}
                }
            }
        })
    });
    // Text changed from outside the page (undo, the review panel, a split):
    // show it. The page's own changes do not move the epoch.
    use_effect(use_reactive((&epoch,), move |_| {
        let text = serde_json::to_string(&state.peek().editor_markdown()).unwrap_or_else(|_| "\"\"".into());
        let _ = document::eval(&format!("window.wsEditor && wsEditor.setText({text});"));
    }));
    let options = page_options(&state.read().markdown_options());
    use_effect(use_reactive((&options,), move |(options,)| {
        let _ = document::eval(&format!("window.wsEditor && wsEditor.setOptions({options});"));
    }));
    // The unknown words to underline, and keeping one's place, as the
    // rich-text page had them.
    use_effect(use_reactive((&spelling,), move |(words,)| {
        let _ = document::eval(&format!("window.wsEditor && wsEditor.setSpelling({words});"));
    }));
    use_effect(use_reactive((&notes,), move |(notes,)| {
        let _ = document::eval(&format!("window.wsEditor && wsEditor.setNotes({notes});"));
    }));
    use_effect(use_reactive((&keep,), move |(keep,)| {
        let _ = document::eval(&format!("window.wsEditor && wsEditor.setKeepPlace({keep});"));
    }));
    let editable = if can_edit { "true" } else { "false" };
    use_effect(use_reactive((&can_edit,), move |(can_edit,)| {
        let _ = document::eval(&format!("window.wsEditor && wsEditor.setEditable({can_edit});"));
    }));
    use_drop(move || {
        let _ = document::eval(&format!("window.wsEditor && wsEditor.unmount({token});"));
    });
    rsx! {
        div {
            id: "markdown-page",
            class: if fading { "manuscript markdown-page fading" } else { "manuscript markdown-page" },
            "data-editable": "{editable}",
            aria_label: "Manuscript",
        }
    }
}

/// The Markdown page's choices, as the editor takes them.
fn page_options(options: &MarkdownOptions) -> serde_json::Value {
    serde_json::json!({
        "source": options.source_mode,
        "smart": options.smart_typography,
        "pair": options.pair_marks,
    })
}

/// Run a call on the Markdown page's editor, when there is one.
pub(super) fn page_call(call: impl AsRef<str>) {
    let _ = document::eval(&format!("window.wsEditor && {};", call.as_ref()));
}

/// One of the format toolbar's commands, on the selection.
pub(super) fn format_command(command: &str, argument: &str) {
    let argument = serde_json::to_string(argument).unwrap_or_else(|_| "\"\"".into());
    page_call(format!("wsEditor.command('{command}', {argument})"));
}

/// Select the next place a word appears in the manuscript.
pub(super) fn find_spelling(word: &str) {
    let word_json = serde_json::to_string(word).unwrap_or_else(|_| "\"\"".into());
    page_call(format!("wsEditor.findWord({word_json})"));
}

fn keep_focus(event: MouseEvent) {
    event.prevent_default();
}

pub(super) fn review_anchor(text: &str) -> String {
    let trimmed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if trimmed.chars().count() <= 42 {
        trimmed
    } else {
        format!("{}…", trimmed.chars().take(42).collect::<String>())
    }
}

/// Notes, tracked changes, and spelling, one tab each. It shows only the
/// tabs that have something in them, and nothing when none do.
#[component]
pub(super) fn ReviewPanel(unknown: Vec<UnknownWord>) -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let items: Vec<ReviewItem> = state.read().review_items();
    let navigation: Vec<NavItem> = state.read().navigation();
    let started = state
        .read()
        .tracking
        .as_ref()
        .filter(|_| state.read().is_tracking())
        .map(|tracking| tracking.started_label());
    let changes: Vec<TrackedChange> = if started.is_some() {
        state.read().tracked_changes()
    } else {
        Vec::new()
    };
    let spelling_on = state.read().speller.is_some() && state.read().session.is_some();
    let project_words = if spelling_on { state.read().project_words() } else { Vec::new() };
    let open_items = items
        .iter()
        .filter(|item| !matches!(item.kind, ReviewKind::Note { resolved: true, .. }))
        .count();
    let mut tabs = Vec::new();
    if !items.is_empty() {
        tabs.push((ReviewTab::Notes, open_items));
    }
    if started.is_some() {
        tabs.push((ReviewTab::Changes, changes.len()));
    }
    if spelling_on && (!unknown.is_empty() || !project_words.is_empty()) {
        tabs.push((ReviewTab::Spelling, unknown.len()));
    }
    if !navigation.is_empty() {
        tabs.push((ReviewTab::Navigate, navigation.len()));
    }
    let available: Vec<ReviewTab> = tabs.iter().map(|(tab, _)| *tab).collect();
    let Some(shown) = ReviewTab::shown(state.read().review_tab, &available) else {
        return rsx! {};
    };
    let folded = state.read().review_folded;
    let side = state.read().review_at_side;
    rsx! {
        section {
            class: if folded { "review-panel folded" } else { "review-panel" },
            aria_label: "Review",
            div { class: "review-tabs", role: "tablist",
                for (tab, count) in tabs {
                    button {
                        class: if tab == shown && !folded { "review-tab active" } else { "review-tab" },
                        role: "tab",
                        aria_selected: "{tab == shown}",
                        onmousedown: keep_focus,
                        onclick: move |_| {
                            let mut current = state.write();
                            current.review_tab = Some(tab);
                            current.review_folded = false;
                        },
                        "{tab.label()}"
                        span { class: "review-count", "{count}" }
                    }
                }
                div { class: "review-spacer" }
                if !folded {
                    button {
                        class: "view-action",
                        title: if side { "Put the review panel under the manuscript" } else { "Put the review panel beside the manuscript" },
                        onmousedown: keep_focus,
                        onclick: move |_| {
                            let next = !state.read().review_at_side;
                            state.write().review_at_side = next;
                        },
                        if side { "Below" } else { "Beside" }
                    }
                }
                button {
                    class: "view-action",
                    title: if folded { "Show the review panel" } else { "Fold the review panel to its tabs" },
                    aria_expanded: "{!folded}",
                    onmousedown: keep_focus,
                    onclick: move |_| {
                        let next = !state.read().review_folded;
                        state.write().review_folded = next;
                    },
                    if folded { "Show" } else { "Hide" }
                }
            }
            if !folded {
                div { class: "review-body", role: "tabpanel",
                    {match shown {
                        ReviewTab::Notes => rsx! { ReviewList { items } },
                        ReviewTab::Changes => rsx! {
                            TrackPanel { started: started.unwrap_or_default(), changes }
                        },
                        ReviewTab::Spelling => rsx! { SpellPanel { unknown, project_words } },
                        ReviewTab::Navigate => rsx! { NavList { items: navigation } },
                    }}
                }
            }
        }
    }
}

/// Words in the open document that the dictionary does not know, and the
/// project's own word list.
#[component]
pub(super) fn SpellPanel(unknown: Vec<UnknownWord>, project_words: Vec<String>) -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let show_project = state.read().show_project_words;
    let focus = state
        .read()
        .spell_focus
        .clone()
        .filter(|word| unknown.iter().any(|item| &item.word == word));
    let suggestions = focus
        .as_deref()
        .map(|word| state.read().spelling_suggestions(word))
        .unwrap_or_default();
    let total = unknown.len();
    let hidden = total.saturating_sub(SPELL_WORDS_SHOWN);
    let project_count = project_words.len();
    rsx! {
        div { class: "review spell",
            div { class: "review-head",
                if total == 0 {
                    span { "Every word is known" }
                } else {
                    span { "{total} words not in the dictionary" }
                }
                if project_count > 0 {
                    button {
                        class: if show_project { "primary" } else { "" },
                        title: "Words this project accepts. Click one to take it out.",
                        onmousedown: keep_focus,
                        onclick: move |_| {
                            let mut state = state.write();
                            state.show_project_words = !state.show_project_words;
                        },
                        "Project dictionary ({project_count})"
                    }
                }
            }
            if total > 0 {
                div { class: "spell-words",
                    for item in unknown.into_iter().take(SPELL_WORDS_SHOWN) {
                        SpellWord { item: item.clone(), active: focus.as_deref() == Some(item.word.as_str()) }
                    }
                    if hidden > 0 {
                        span { class: "hint", "and {hidden} more" }
                    }
                }
            }
            if let Some(word) = focus {
                SpellFocus { word, suggestions }
            }
            if show_project && project_count > 0 {
                div { class: "spell-words",
                    for word in project_words {
                        button {
                            key: "{word}",
                            class: "spell-chip",
                            title: "Take “{word}” out of this project's dictionary",
                            onmousedown: keep_focus,
                            // The handler has its own copy: a release build
                            // writes the text below after making it.
                            onclick: {
                                let word = word.clone();
                                move |_| {
                                    let word = word.clone();
                                    state.write().remove_project_word(&word);
                                }
                            },
                            "{word} ×"
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub(super) fn SpellWord(item: UnknownWord, active: bool) -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let word = item.word.clone();
    rsx! {
        button {
            class: if active { "spell-chip active" } else { "spell-chip" },
            title: "Show “{item.word}” in the manuscript, with suggestions",
            onclick: move |_| {
                state.write().spell_focus = Some(word.clone());
                find_spelling(&word);
            },
            "{item.word}"
            if item.count > 1 {
                span { class: "spell-count", " ×{item.count}" }
            }
        }
    }
}

#[component]
pub(super) fn SpellFocus(word: String, suggestions: Vec<String>) -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let (find_word, add_word) = (word.clone(), word.clone());
    rsx! {
        div { class: "spell-focus",
            strong { "{word}" }
            if suggestions.is_empty() {
                span { class: "hint", "No suggestions" }
            }
            for suggestion in suggestions {
                button {
                    key: "{suggestion}",
                    title: "Replace every “{word}” in this document with “{suggestion}”",
                    onmousedown: keep_focus,
                    onclick: {
                        let (word, suggestion) = (word.clone(), suggestion.clone());
                        move |_| {
                            let (word, suggestion) = (word.clone(), suggestion.clone());
                            state.write().replace_misspelling(&word, &suggestion);
                        }
                    },
                    "{suggestion}"
                }
            }
            button {
                title: "Show the next place this word appears",
                onclick: move |_| find_spelling(&find_word),
                "Find next"
            }
            button {
                title: "Accept this word everywhere in this project",
                onmousedown: keep_focus,
                onclick: move |_| {
                    let word = add_word.clone();
                    state.write().add_project_word(&word);
                },
                "Add to project dictionary"
            }
            button {
                onmousedown: keep_focus,
                onclick: move |_| state.write().spell_focus = None,
                "Close"
            }
        }
    }
}

/// Paragraphs changed since tracking was turned on, with accept and reject.
#[component]
pub(super) fn TrackPanel(started: String, changes: Vec<TrackedChange>) -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let count = changes.len();
    rsx! {
        div { class: "review track",
            div { class: "review-head",
                if count == 0 {
                    span { "Tracking since {started} · nothing changed yet" }
                } else {
                    span { "Tracking since {started} · {count} changed paragraphs" }
                    button {
                        title: "Keep every change. Tracking goes on from the text as it is now.",
                        onmousedown: keep_focus,
                        onclick: move |_| state.write().accept_all_tracked_changes(),
                        "Accept all"
                    }
                    button {
                        title: "Put the text back as it was when tracking began. Undo brings the changes back.",
                        onmousedown: keep_focus,
                        onclick: move |_| state.write().reject_all_tracked_changes(),
                        "Reject all"
                    }
                }
            }
            ul { class: "review-list track-list",
                for (index, change) in changes.into_iter().enumerate() {
                    TrackRow { index, change }
                }
            }
        }
    }
}

#[component]
pub(super) fn TrackRow(index: usize, change: TrackedChange) -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let (label, row_class) = match change.change {
        ParagraphChange::Added => ("Added", "review-item track-item added"),
        ParagraphChange::Removed => ("Removed", "review-item track-item removed"),
        _ => ("Changed", "review-item track-item"),
    };
    let block = change.current_block.unwrap_or(change.current_at);
    let blank = change.spans.iter().all(|span| span.text.trim().is_empty());
    rsx! {
        li { class: row_class,
            button {
                class: "review-anchor track-kind",
                title: "Show where this is in the manuscript",
                onclick: move |_| page_call(format!("wsEditor.showBlock({block})")),
                "{label}"
            }
            p { class: "track-text",
                if blank {
                    span { class: "hint", "Blank paragraph" }
                }
                for span in change.spans.iter() {
                    match span.op {
                        DiffOp::Same => rsx! { "{span.text}" },
                        DiffOp::Added => rsx! { ins { class: "diff-added", "{span.text}" } },
                        DiffOp::Removed => rsx! { del { class: "diff-removed", "{span.text}" } },
                    }
                }
            }
            button {
                title: "Keep this change",
                onmousedown: keep_focus,
                onclick: move |_| state.write().accept_tracked_change(index),
                "Accept"
            }
            button {
                title: "Put this paragraph back as it was. Undo brings the change back.",
                onmousedown: keep_focus,
                onclick: move |_| state.write().reject_tracked_change(index),
                "Reject"
            }
        }
    }
}

/// Notes and highlights in the open document.
#[component]
pub(super) fn ReviewList(items: Vec<ReviewItem>) -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let show_resolved = state.read().show_resolved_notes;
    let is_resolved = |item: &ReviewItem| matches!(item.kind, ReviewKind::Note { resolved: true, .. });
    let resolved = items.iter().filter(|item| is_resolved(item)).count();
    let highlights = items
        .iter()
        .filter(|item| matches!(item.kind, ReviewKind::Highlight(_)))
        .count();
    let open = items.len() - resolved - highlights;
    let shown: Vec<ReviewItem> = items
        .into_iter()
        .filter(|item| show_resolved || !is_resolved(item))
        .collect();
    rsx! {
        div { class: "review",
            div { class: "review-head",
                span { "{open} open · {resolved} resolved · {highlights} highlighted" }
                if resolved > 0 {
                    label { class: "compare-toggle",
                        input {
                            r#type: "checkbox",
                            checked: show_resolved,
                            onchange: move |event| state.write().show_resolved_notes = event.checked(),
                        }
                        "Show resolved"
                    }
                    button {
                        title: "Take every resolved note off this document. The text stays.",
                        onmousedown: keep_focus,
                        onclick: move |_| state.write().remove_resolved_notes(),
                        "Remove resolved"
                    }
                }
            }
            ul { class: "review-list",
                for item in shown {
                    ReviewRow { item }
                }
            }
        }
    }
}

#[component]
pub(super) fn ReviewRow(item: ReviewItem) -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let target = item.target;
    let anchor = review_anchor(&item.text);
    let row_class = if matches!(item.kind, ReviewKind::Note { resolved: true, .. }) {
        "review-item resolved"
    } else {
        "review-item"
    };
    let jump = match target {
        ReviewTarget::Note(index) => format!("wsEditor.selectReview('note', {index})"),
        ReviewTarget::Highlight(index) => format!("wsEditor.selectReview('highlight', {index})"),
    };
    rsx! {
        li { class: row_class,
            button {
                class: "review-anchor",
                title: "Show this text in the manuscript",
                onclick: move |_| page_call(&jump),
                "{anchor}"
            }
            match item.kind {
                ReviewKind::Note { comment, kind, resolved } => {
                    let ReviewTarget::Note(index) = target else { return rsx! {} };
                    rsx! {
                        select {
                            class: "compile-select",
                            title: "What this note is for",
                            value: "{kind.as_str()}",
                            onchange: move |event| {
                                if let Some(next) = NoteKind::from_id(&event.value()) {
                                    state.write().set_note_kind(index, next);
                                }
                            },
                            for choice in NoteKind::ALL {
                                option { value: "{choice.as_str()}", selected: choice == kind, "{choice.label()}" }
                            }
                        }
                        input {
                            class: "review-text",
                            title: "Note text. Enter or leaving the field keeps the change.",
                            value: "{comment}",
                            onchange: move |event| state.write().set_note_text(index, &event.value()),
                        }
                        button {
                            onmousedown: keep_focus,
                            onclick: move |_| state.write().resolve_note(index, !resolved),
                            if resolved { "Reopen" } else { "Resolve" }
                        }
                    }
                }
                ReviewKind::Highlight(color) => rsx! {
                    span { class: "review-swatch hl-{color.as_str()}", "{color.label()} highlight" }
                },
            }
            button {
                title: "Remove it. The text stays.",
                onmousedown: keep_focus,
                onclick: move |_| state.write().remove_review(target),
                "Remove"
            }
        }
    }
}

/// The paragraph styles, as `formatBlock` takes them, and the name the
/// mark watcher reports for each.
const BLOCK_STYLES: [(&str, &str, &str); 5] = [
    ("P", "p", "Body"),
    ("H1", "h1", "Heading 1"),
    ("H2", "h2", "Heading 2"),
    ("H3", "h3", "Heading 3"),
    ("PRE", "pre", "Code"),
];

/// One row. Every button keeps the focus in the manuscript, so the caret
/// and the selection stay where they were.
#[component]
pub(super) fn FormatToolbar(
    enabled: bool,
    marks: Signal<Vec<String>>,
    prompt: Signal<Option<InlinePrompt>>,
) -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let menu = state.read().open_menu;
    let tracking = state.read().is_tracking();
    let on = move |name: &str| marks.read().iter().any(|mark| mark == name);
    let block = marks
        .read()
        .iter()
        .find_map(|mark| mark.strip_prefix("block:").map(str::to_owned))
        .unwrap_or_else(|| "p".into());
    let block_label = BLOCK_STYLES
        .iter()
        .find(|(_, id, _)| *id == block)
        .map(|(_, _, label)| *label)
        .unwrap_or("Body");
    let heading_open = menu == Some(FrameMenu::Heading);
    let more_open = menu == Some(FrameMenu::More);
    rsx! {
        div { class: "format-toolbar", role: "toolbar", aria_label: "Format",
            ToolButton {
                name: "Bold", title: "Bold (Ctrl+B)", on: on("bold"), enabled,
                onclick: move |_| format_command("bold", ""),
                span { class: "tool-letter bold", "B" }
            }
            ToolButton {
                name: "Italic", title: "Italic (Ctrl+I)", on: on("italic"), enabled,
                onclick: move |_| format_command("italic", ""),
                span { class: "tool-letter italic", "I" }
            }
            span { class: "tool-rule" }
            div { class: "menu-anchor",
                button {
                    class: if heading_open { "tool tool-menu open" } else { "tool tool-menu" },
                    title: "Paragraph style",
                    aria_haspopup: "menu",
                    aria_expanded: "{heading_open}",
                    disabled: !enabled,
                    onmousedown: keep_focus,
                    onclick: move |_| state.write().toggle_menu(FrameMenu::Heading),
                    "{block_label}"
                }
                if heading_open {
                    div { class: "menu", role: "menu", aria_label: "Paragraph style",
                        for (command, id, label) in BLOCK_STYLES {
                            button {
                                class: if id == block { "menu-item current" } else { "menu-item" },
                                role: "menuitemradio",
                                aria_checked: "{id == block}",
                                onmousedown: keep_focus,
                                onclick: move |_| {
                                    state.write().open_menu = None;
                                    format_command("formatBlock", command);
                                },
                                "{label}"
                            }
                        }
                    }
                }
            }
            ToolButton {
                name: "Quote", title: "Quote", on: on("quote"), enabled,
                onclick: move |_| {
                    let target = if on("quote") { "P" } else { "BLOCKQUOTE" };
                    format_command("formatBlock", target);
                },
                svg { class: "glyph", view_box: "0 0 16 16",
                    path { d: "M3 4.5h4v4H5.2L4.2 12H3zM9 4.5h4v4h-1.8l-1 3.5H9z", fill: "currentColor" }
                }
            }
            ToolButton {
                name: "List", title: "Bulleted list", on: on("list"), enabled,
                onclick: move |_| format_command("insertUnorderedList", ""),
                svg { class: "glyph", view_box: "0 0 16 16",
                    circle { cx: "3", cy: "4", r: "1.1", fill: "currentColor" }
                    circle { cx: "3", cy: "8", r: "1.1", fill: "currentColor" }
                    circle { cx: "3", cy: "12", r: "1.1", fill: "currentColor" }
                    path { d: "M6 4h8M6 8h8M6 12h8", fill: "none", stroke: "currentColor", stroke_width: "1.2" }
                }
            }
            span { class: "tool-rule" }
            ToolButton {
                name: "Link", title: "Link the selected text", on: on("link"), enabled,
                onclick: move |_| open_prompt(state, prompt, PromptKind::Link),
                svg { class: "glyph", view_box: "0 0 16 16",
                    path { d: "M6.5 9.5l3-3M7.2 4.8l1.1-1.1a2.6 2.6 0 013.7 3.7l-1.1 1.1M8.8 11.2l-1.1 1.1a2.6 2.6 0 01-3.7-3.7l1.1-1.1", fill: "none", stroke: "currentColor", stroke_width: "1.3", stroke_linecap: "round" }
                }
            }
            ToolButton {
                name: "Note", title: "Put a note on the selected text", on: on("note"), enabled,
                onclick: move |_| open_prompt(state, prompt, PromptKind::Note),
                svg { class: "glyph", view_box: "0 0 16 16",
                    path { d: "M2.5 3h11v7.5h-6l-3 2.5v-2.5h-2z", fill: "none", stroke: "currentColor", stroke_width: "1.2", stroke_linejoin: "round" }
                }
            }
            span { class: "tool-rule" }
            div { class: "menu-anchor",
                button {
                    class: if more_open { "tool open" } else { "tool" },
                    title: "More formatting",
                    aria_label: "More",
                    aria_haspopup: "menu",
                    aria_expanded: "{more_open}",
                    disabled: !enabled,
                    onmousedown: keep_focus,
                    onclick: move |_| state.write().toggle_menu(FrameMenu::More),
                    svg { class: "glyph", view_box: "0 0 16 16",
                        circle { cx: "3.5", cy: "8", r: "1.2", fill: "currentColor" }
                        circle { cx: "8", cy: "8", r: "1.2", fill: "currentColor" }
                        circle { cx: "12.5", cy: "8", r: "1.2", fill: "currentColor" }
                    }
                }
                if more_open {
                    MoreMenu { marks, tracking }
                }
            }
            if tracking {
                span { class: "tool-status", title: "Changes to this document are being tracked", "Tracking changes" }
            }
        }
    }
}

#[component]
fn ToolButton(
    name: &'static str,
    title: &'static str,
    on: bool,
    enabled: bool,
    onclick: EventHandler<MouseEvent>,
    children: Element,
) -> Element {
    rsx! {
        button {
            class: if on { "tool on" } else { "tool" },
            title: "{title}",
            aria_label: "{name}",
            aria_pressed: "{on}",
            disabled: !enabled,
            onmousedown: keep_focus,
            onclick: move |event| onclick.call(event),
            {children}
        }
    }
}

#[component]
fn MoreMenu(marks: Signal<Vec<String>>, tracking: bool) -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let on = move |name: &str| marks.read().iter().any(|mark| mark == name);
    let command = move |name: &'static str, argument: &'static str| {
        move |_| {
            state.write().open_menu = None;
            format_command(name, argument);
        }
    };
    let script = move |call: String| {
        move |_| {
            state.write().open_menu = None;
            page_call(&call);
        }
    };
    rsx! {
        div { class: "menu", role: "menu", aria_label: "More formatting",
            button { class: "menu-item", role: "menuitemcheckbox", aria_checked: "{on(\"underline\")}", onmousedown: keep_focus, onclick: command("underline", ""),
                span { "Underline" } span { class: "menu-hint", "Ctrl+U" }
            }
            button { class: "menu-item", role: "menuitemcheckbox", aria_checked: "{on(\"strikeThrough\")}", onmousedown: keep_focus, onclick: command("strikeThrough", ""), "Strikethrough" }
            button { class: "menu-item", role: "menuitemcheckbox", aria_checked: "{on(\"numbered\")}", onmousedown: keep_focus, onclick: command("insertOrderedList", ""), "Numbered list" }
            button { class: "menu-item", role: "menuitem", onmousedown: keep_focus, onclick: command("insertHorizontalRule", ""), "Scene break" }
            div { class: "menu-rule" }
            div { class: "menu-label", "Highlight" }
            div { class: "menu-swatches",
                for color in Highlight::ALL {
                    button {
                        class: "hl-btn hl-{color.as_str()}",
                        role: "menuitem",
                        title: "Highlight the selected text in {color.as_str()}",
                        onmousedown: keep_focus,
                        onclick: script(format!("wsEditor.command('highlight', '{}')", color.as_str())),
                        "{color.label()}"
                    }
                }
            }
            button { class: "menu-item", role: "menuitem", onmousedown: keep_focus, onclick: script("wsEditor.command('clearHighlight')".into()), "Remove highlight" }
            div { class: "menu-rule" }
            button { class: "menu-item", role: "menuitem", disabled: !on("link"), onmousedown: keep_focus, onclick: command("unlink", ""), "Remove link" }
            button { class: "menu-item", role: "menuitem", disabled: !on("note"), onmousedown: keep_focus, onclick: script("wsEditor.command('uncomment')".into()), "Remove note" }
            div { class: "menu-rule" }
            button {
                class: "menu-item",
                role: "menuitemcheckbox",
                aria_checked: "{tracking}",
                title: if tracking {
                    "Stop tracking. The text stays as it is and the list of changes is dropped."
                } else {
                    "Track changes to this document from now, to accept or reject them later"
                },
                onmousedown: keep_focus,
                onclick: move |_| {
                    state.write().open_menu = None;
                    if tracking {
                        state.write().stop_tracking();
                    } else {
                        state.write().start_tracking();
                    }
                },
                if tracking { "Stop tracking changes" } else { "Track changes" }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum PromptKind {
    Link,
    Note,
}

/// The field a link or note asks in, placed under the selection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct InlinePrompt {
    kind: PromptKind,
    x: f64,
    y: f64,
}

/// Hold the selection and open the field for it. With nothing selected the
/// status line says what to do instead.
fn open_prompt(mut state: Signal<StudioState>, mut prompt: Signal<Option<InlinePrompt>>, kind: PromptKind) {
    spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(16)).await;
        let mut eval = document::eval("dioxus.send(window.wsEditor ? wsEditor.hold('.editor-body') : []);");
        match eval.recv::<Vec<f64>>().await.unwrap_or_default()[..] {
            [x, y] => prompt.set(Some(InlinePrompt { kind, x, y })),
            _ => {
                state.write().status = match kind {
                    PromptKind::Link => "Select the text to link first.".into(),
                    PromptKind::Note => "Select the text for the note first.".into(),
                }
            }
        }
    });
}

#[component]
fn PromptField(at: InlinePrompt, prompt: Signal<Option<InlinePrompt>>) -> Element {
    let mut text = use_signal(String::new);
    let mut note_kind = use_signal(|| NoteKind::Comment);
    let mut apply = move || {
        let value = serde_json::to_string(&text()).unwrap_or_else(|_| "\"\"".into());
        let call = match at.kind {
            PromptKind::Link => format!("wsEditor.command('link', {value})"),
            PromptKind::Note => format!("wsEditor.command('note', {{ kind: '{}', text: {value} }})", note_kind().as_str()),
        };
        prompt.set(None);
        page_call(call);
    };
    let mut cancel = move || {
        prompt.set(None);
        page_call("wsEditor.release()");
    };
    let (label, placeholder, action) = match at.kind {
        PromptKind::Link => ("Link address", "example.com", "Link"),
        PromptKind::Note => ("Note on the selection", "What to remember here", "Add note"),
    };
    let left = at.x.max(8.0);
    let top = at.y + 6.0;
    rsx! {
        div {
            class: "inline-prompt",
            role: "dialog",
            aria_label: "{label}",
            style: "left: {left}px; top: {top}px;",
            if at.kind == PromptKind::Note {
                select {
                    class: "compile-select",
                    title: "What this note is for",
                    aria_label: "Kind of note",
                    value: "{note_kind().as_str()}",
                    onchange: move |event| {
                        if let Some(next) = NoteKind::from_id(&event.value()) {
                            note_kind.set(next);
                        }
                    },
                    for choice in NoteKind::ALL {
                        option { value: "{choice.as_str()}", selected: choice == note_kind(), "{choice.label()}" }
                    }
                }
            }
            input {
                aria_label: "{label}",
                placeholder: "{placeholder}",
                value: "{text}",
                oninput: move |event| text.set(event.value()),
                onmounted: move |event| async move {
                    let _ = event.set_focus(true).await;
                },
                onkeydown: move |event| match event.key() {
                    Key::Enter => {
                        event.prevent_default();
                        apply();
                    }
                    Key::Escape => {
                        event.stop_propagation();
                        cancel();
                    }
                    _ => {}
                },
            }
            button { class: "primary", disabled: text().trim().is_empty(), onclick: move |_| apply(), "{action}" }
            button { onclick: move |_| cancel(), "Cancel" }
        }
    }
}

/// The scene's title at the top of the page. It is a field, so the title is
/// renamed where it is read; Enter or leaving it keeps the change.
#[component]
fn PageTitle(id: Id, title: String, editable: bool) -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let mut draft = use_signal(|| title.clone());
    if !editable {
        return rsx! { h1 { class: "page-title", "{title}" } };
    }
    rsx! {
        input {
            class: "page-title",
            aria_label: "Scene title",
            placeholder: "Untitled scene",
            value: "{draft}",
            oninput: move |event| draft.set(event.value()),
            onkeydown: move |event| match event.key() {
                Key::Enter => {
                    event.prevent_default();
                    state.write().rename_item(id, &draft());
                    focus_dom_now(".markdown-page .cm-content");
                }
                Key::Escape => {
                    event.stop_propagation();
                    draft.set(title.clone());
                }
                _ => {}
            },
            onblur: move |_| {
                if draft().trim().is_empty() {
                    draft.set(state.read().binder.get(id).map(|node| node.title.clone()).unwrap_or_default());
                } else {
                    state.write().rename_item(id, &draft());
                }
            },
        }
    }
}

/// Move the focus to the first element matching a selector.
fn focus_dom_now(selector: &'static str) {
    spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(16)).await;
        let script = format!("document.querySelector('{selector}')?.focus(); dioxus.send(true);");
        let mut eval = document::eval(&script);
        let _ = eval.recv::<bool>().await;
    });
}

/// The scene's headings, links, and notes in order. Choosing one selects
/// it on the page, the way the Notes tab does.
#[component]
pub(super) fn NavList(items: Vec<NavItem>) -> Element {
    rsx! {
        ul { class: "review-list nav-list",
            for item in items {
                {
                    let (kind, class) = match &item.kind {
                        NavKind::Heading(level) => (format!("Heading {level}"), format!("nav-item nav-heading level-{level}")),
                        NavKind::Link(_) => ("Link".to_string(), "nav-item nav-link".to_string()),
                        NavKind::Note(_) => ("Note".to_string(), "nav-item nav-note".to_string()),
                    };
                    let jump = match &item.kind {
                        NavKind::Note(index) => format!("wsEditor.selectReview('note', {index})"),
                        _ => format!("wsEditor.showBlock({})", item.block),
                    };
                    let detail = match &item.kind {
                        NavKind::Link(url) => url.clone(),
                        _ => String::new(),
                    };
                    let text = review_anchor(&item.text);
                    rsx! {
                        li { class: "{class}",
                            button {
                                class: "review-anchor",
                                title: if detail.is_empty() { "Show this in the manuscript".to_string() } else { detail.clone() },
                                onclick: move |_| page_call(&jump),
                                span { class: "nav-kind", "{kind}" }
                                span { class: "nav-text", "{text}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
