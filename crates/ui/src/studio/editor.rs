use super::*;

#[component]
pub(super) fn EditorPane() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let snapshot = state.read().clone();
    let heading = snapshot
        .selected_node()
        .map(|node| node.title.clone())
        .unwrap_or_else(|| "Select a scene".into());
    let kind = snapshot
        .selected_node()
        .map(|node| node.kind.icon_label())
        .unwrap_or("Document");
    let html = snapshot.editor_html.clone();
    let epoch = snapshot.editor_epoch;
    let can_edit = snapshot.session.is_some() && !snapshot.syncing;
    // The words to underline. An empty list clears the underlines, which is
    // also what a project with spellcheck off sends.
    let unknown: Vec<String> = snapshot
        .unknown_words()
        .into_iter()
        .map(|item| item.word)
        .collect();
    let unknown_json = serde_json::to_string(&unknown).unwrap_or_else(|_| "[]".into());
    use_effect(use_reactive((&epoch, &unknown_json), |(_, words)| paint_spelling(words)));
    let initial_html = if html.trim().is_empty() {
        "<p><br></p>".into()
    } else {
        html
    };

    rsx! {
        div { class: "editor-pane",
            div { class: "breadcrumb", "{kind} › {heading}" }
            FormatToolbar { enabled: can_edit }
            ReviewList {}
            TrackPanel {}
            SpellPanel {}
            div {
                key: "{epoch}",
                class: "manuscript rich",
                contenteditable: can_edit,
                "data-manuscript": "true",
                dangerous_inner_html: "{initial_html}",
                oninput: move |_| read_manuscript_html(state),
                onblur: move |_| {
                    read_manuscript_html(state);
                    state.write().flush_editor();
                },
            }
        }
    }
}

pub(super) fn read_manuscript_html(state: Signal<StudioState>) {
    spawn(async move {
        // Leave the webview event before running script. Evaluating script
        // inside the input callback deadlocks WebView2 and the window stops.
        tokio::time::sleep(std::time::Duration::from_millis(16)).await;
        let mut eval = document::eval(
            r#"
            const el = document.querySelector('[data-manuscript]');
            dioxus.send(el ? el.innerHTML : '');
            "#,
        );
        if let Ok(html) = eval.recv::<String>().await {
            commit_editor_html(state, html);
        }
    });
}

/// Send the unknown words to the manuscript, which underlines them. Nothing
/// comes back into the document.
pub(super) fn paint_spelling(words_json: String) {
    spawn(async move {
        // Wait for the manuscript to be on screen after a redraw.
        tokio::time::sleep(std::time::Duration::from_millis(40)).await;
        let script = format!(
            r#"
            {SPELL_JS}
            dioxus.send(wsSpellApply(document.querySelector('[data-manuscript]'), {words_json}));
            "#
        );
        let mut eval = document::eval(&script);
        let _ = eval.recv::<bool>().await;
    });
}

/// Select the next place a word appears in the manuscript.
pub(super) fn find_spelling(word: &str) {
    let word_json = serde_json::to_string(word).unwrap_or_else(|_| "\"\"".into());
    spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(16)).await;
        let script = format!(
            r#"
            {SPELL_JS}
            const el = document.querySelector('[data-manuscript]');
            if (el) {{ el.focus(); }}
            dioxus.send(wsSpellFind(el, {word_json}));
            "#
        );
        let mut eval = document::eval(&script);
        let _ = eval.recv::<bool>().await;
    });
}

pub(super) fn commit_editor_html(mut state: Signal<StudioState>, html: String) {
    if state.read().same_editor_html(&html) {
        return;
    }
    state.write().apply_editor_html(html);
}

pub(super) fn review_anchor(text: &str) -> String {
    let trimmed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if trimmed.chars().count() <= 42 {
        trimmed
    } else {
        format!("{}…", trimmed.chars().take(42).collect::<String>())
    }
}

/// Words in the open document that the dictionary does not know, and the
/// project's own word list.
#[component]
pub(super) fn SpellPanel() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let snapshot = state.read().clone();
    if snapshot.speller.is_none() || snapshot.session.is_none() {
        return rsx! {};
    }
    let unknown: Vec<UnknownWord> = snapshot.unknown_words();
    let project_words = snapshot.project_words();
    if unknown.is_empty() && project_words.is_empty() {
        return rsx! {};
    }
    let show_project = snapshot.show_project_words;
    let focus = snapshot
        .spell_focus
        .clone()
        .filter(|word| unknown.iter().any(|item| &item.word == word));
    let suggestions = focus
        .as_deref()
        .map(|word| snapshot.spelling_suggestions(word))
        .unwrap_or_default();
    let total = unknown.len();
    let hidden = total.saturating_sub(SPELL_WORDS_SHOWN);
    let project_count = project_words.len();
    rsx! {
        div { class: "review spell",
            div { class: "review-head",
                if total == 0 {
                    span { "Spelling · every word is known" }
                } else {
                    span { "Spelling · {total} words not in the dictionary" }
                }
                if project_count > 0 {
                    button {
                        class: if show_project { "primary" } else { "" },
                        title: "Words this project accepts. Click one to take it out.",
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
                            onclick: move |_| state.write().remove_project_word(&word),
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
                    onclick: {
                        let word = word.clone();
                        move |_| state.write().replace_misspelling(&word, &suggestion)
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
                onclick: move |_| state.write().add_project_word(&add_word),
                "Add to project dictionary"
            }
            button { onclick: move |_| state.write().spell_focus = None, "Close" }
        }
    }
}

/// Paragraphs changed since tracking was turned on, with accept and reject.
#[component]
pub(super) fn TrackPanel() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let Some(started) = state
        .read()
        .tracking
        .as_ref()
        .filter(|_| state.read().is_tracking())
        .map(|tracking| tracking.started_label())
    else {
        return rsx! {};
    };
    let changes: Vec<TrackedChange> = state.read().tracked_changes();
    let count = changes.len();
    rsx! {
        div { class: "review track",
            div { class: "review-head",
                if count == 0 {
                    span { "Tracking changes since {started} · nothing changed yet" }
                } else {
                    span { "Tracking changes since {started} · {count} changed paragraphs" }
                    button {
                        title: "Keep every change. Tracking goes on from the text as it is now.",
                        onclick: move |_| state.write().accept_all_tracked_changes(),
                        "Accept all"
                    }
                    button {
                        title: "Put the text back as it was when tracking began. Undo brings the changes back.",
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
                onclick: move |_| run_editor_script(state, format!("wsShowBlock(el, {block});")),
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
                onclick: move |_| state.write().accept_tracked_change(index),
                "Accept"
            }
            button {
                title: "Put this paragraph back as it was. Undo brings the change back.",
                onclick: move |_| state.write().reject_tracked_change(index),
                "Reject"
            }
        }
    }
}

/// Notes and highlights in the open document, above the manuscript.
#[component]
pub(super) fn ReviewList() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let items: Vec<ReviewItem> = state.read().review_items();
    if items.is_empty() {
        return rsx! {};
    }
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
                span { "Review · {open} open · {resolved} resolved · {highlights} highlighted" }
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
        ReviewTarget::Note(index) => format!("wsSelectReview(el, 'note', {index});"),
        ReviewTarget::Highlight(index) => format!("wsSelectReview(el, 'highlight', {index});"),
    };
    rsx! {
        li { class: row_class,
            button {
                class: "review-anchor",
                title: "Show this text in the manuscript",
                onclick: move |_| run_editor_script(state, jump.clone()),
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
                onclick: move |_| state.write().remove_review(target),
                "Remove"
            }
        }
    }
}

pub(super) fn run_format_command(state: Signal<StudioState>, command: &'static str, argument: &'static str) {
    spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(16)).await;
        let script = format!(
            r#"
            const el = document.querySelector('[data-manuscript]');
            if (el) {{ el.focus(); }}
            document.execCommand('{command}', false, {arg});
            dioxus.send(el ? el.innerHTML : '');
            "#,
            command = command,
            arg = if argument.is_empty() {
                "null".into()
            } else {
                format!("'{argument}'")
            }
        );
        let mut eval = document::eval(&script);
        if let Ok(html) = eval.recv::<String>().await {
            commit_editor_html(state, html);
        }
    });
}

pub(super) fn run_editor_script(state: Signal<StudioState>, body: impl Into<String>) {
    let body = body.into();
    spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(16)).await;
        let script = format!(
            r#"
            {REVIEW_JS}
            const el = document.querySelector('[data-manuscript]');
            if (el) {{ el.focus(); }}
            {body}
            dioxus.send(el ? el.innerHTML : '');
            "#
        );
        let mut eval = document::eval(&script);
        if let Ok(html) = eval.recv::<String>().await {
            commit_editor_html(state, html);
        }
    });
}

#[component]
pub(super) fn FormatToolbar(enabled: bool) -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let tracking = state.read().is_tracking();
    let mut note_kind = use_signal(|| NoteKind::Comment);
    rsx! {
        div { class: "format-toolbar",
            div { class: "format-group",
                button { disabled: !enabled, onclick: move |_| run_format_command(state, "bold", ""), "Bold" }
                button { disabled: !enabled, onclick: move |_| run_format_command(state, "italic", ""), "Italic" }
                button { disabled: !enabled, onclick: move |_| run_format_command(state, "underline", ""), "Underline" }
                button { disabled: !enabled, onclick: move |_| run_format_command(state, "strikeThrough", ""), "Strike" }
                button { disabled: !enabled, onclick: move |_| run_editor_script(state, LINK_SCRIPT), "Link" }
                button { disabled: !enabled, onclick: move |_| run_format_command(state, "unlink", ""), "Unlink" }
            }
            div { class: "format-group",
                select {
                    class: "compile-select",
                    title: "What the next note is for",
                    disabled: !enabled,
                    value: "{note_kind().as_str()}",
                    onchange: move |event| {
                        if let Some(next) = NoteKind::from_id(&event.value()) {
                            note_kind.set(next);
                        }
                    },
                    for choice in NoteKind::ALL {
                        option { value: "{choice.as_str()}", "{choice.label()}" }
                    }
                }
                button {
                    disabled: !enabled,
                    title: "Put a note on the selected text",
                    onclick: move |_| run_editor_script(state, format!("wsComment(el, '{}');", note_kind().as_str())),
                    "Note"
                }
                button {
                    disabled: !enabled,
                    title: "Take the note off the selected text",
                    onclick: move |_| run_editor_script(state, "wsUncomment(el);"),
                    "Remove note"
                }
                for color in Highlight::ALL {
                    button {
                        class: "hl-btn hl-{color.as_str()}",
                        disabled: !enabled,
                        title: "Highlight the selected text in {color.as_str()}",
                        onclick: move |_| run_editor_script(state, format!("wsHighlight(el, '{}');", color.as_str())),
                        "{color.label()}"
                    }
                }
                button {
                    disabled: !enabled,
                    title: "Take the highlight off the selected text",
                    onclick: move |_| run_editor_script(state, "wsClearHighlight(el);"),
                    "No highlight"
                }
            }
            div { class: "format-group",
                button {
                    class: if tracking { "primary" } else { "" },
                    disabled: !enabled,
                    title: if tracking {
                        "Stop tracking. The text stays as it is and the list of changes is dropped."
                    } else {
                        "Track changes to this document from now, to accept or reject them later"
                    },
                    onclick: move |_| {
                        if tracking {
                            state.write().stop_tracking();
                        } else {
                            state.write().start_tracking();
                        }
                    },
                    if tracking { "Stop tracking" } else { "Track changes" }
                }
            }
            div { class: "format-group",
                button { disabled: !enabled, onclick: move |_| run_format_command(state, "formatBlock", "H1"), "H1" }
                button { disabled: !enabled, onclick: move |_| run_format_command(state, "formatBlock", "H2"), "H2" }
                button { disabled: !enabled, onclick: move |_| run_format_command(state, "formatBlock", "H3"), "H3" }
                button { disabled: !enabled, onclick: move |_| run_format_command(state, "formatBlock", "P"), "Body" }
                button { disabled: !enabled, onclick: move |_| run_format_command(state, "formatBlock", "BLOCKQUOTE"), "Quote" }
                button { disabled: !enabled, onclick: move |_| run_format_command(state, "insertUnorderedList", ""), "List" }
                button { disabled: !enabled, onclick: move |_| run_format_command(state, "insertOrderedList", ""), "Numbered" }
                button { disabled: !enabled, onclick: move |_| run_format_command(state, "formatBlock", "PRE"), "Code" }
                button { disabled: !enabled, onclick: move |_| run_format_command(state, "insertHorizontalRule", ""), "Break" }
            }
        }
    }
}

pub(super) const LINK_SCRIPT: &str = r#"
const sel = window.getSelection();
if (sel && sel.rangeCount && !sel.isCollapsed) {
    const saved = sel.getRangeAt(0).cloneRange();
    const raw = window.prompt('Link address');
    if (raw && raw.trim()) {
        let url = raw.trim();
        if (!/^[a-z][a-z0-9+.-]*:/i.test(url)) {
            url = 'https://' + url;
        }
        if (/^(https?:|mailto:)/i.test(url)) {
            sel.removeAllRanges();
            sel.addRange(saved);
            document.execCommand('createLink', false, url);
        }
    }
}
"#;
