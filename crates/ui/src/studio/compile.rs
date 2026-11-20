use super::*;
use writing_studio_project::{
    CompileFormat, CompileProfile, HeadingStyle, LineSpacing, PageSize, SceneSeparator, FONT_SIZE_RANGE, MARGIN_RANGE,
};

/// Project → Compile…: choose the book and the profile, change the profile,
/// and compile. Every change to a profile is kept as it is made.
#[component]
pub(super) fn CompileDialog() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let books = state.read().manuscript_book_choices();
    let book = state.read().compile_book_id();
    let profiles = state.read().compile_profiles();
    let removed = state.read().removed_built_in_profiles();
    let Some(profile) = state.read().compile_profile() else {
        return rsx! {};
    };
    let chosen = profile.id;
    let built_in = profile.is_built_in();
    let page = profile.page.clone();
    let margin = format!("{:.1}", page.margin as f64 / 10.0);
    rsx! {
        div { class: "dialog-backdrop", onclick: move |_| state.write().compile_open = false }
        div { class: "dialog compile-dialog", role: "dialog", aria_modal: "true", aria_labelledby: "compile-title",
            div { class: "dialog-head",
                h2 { id: "compile-title", "Compile" }
                button {
                    onmounted: move |event| async move {
                        let _ = event.set_focus(true).await;
                    },
                    onclick: move |_| state.write().compile_open = false,
                    "Close"
                }
            }
            div { class: "compile-grid",
                label { r#for: "compile-book", "Book" }
                select {
                    id: "compile-book",
                    onchange: move |event| {
                        if let Ok(id) = event.value().parse::<Id>() {
                            state.write().remember_book_for_compile(id);
                        }
                    },
                    for (id, title) in books {
                        option { value: "{id}", selected: Some(id) == book, "{title}" }
                    }
                }
                label { r#for: "compile-profile", "Profile" }
                div { class: "compile-row",
                    select {
                        id: "compile-profile",
                        onchange: move |event| {
                            let value = event.value();
                            if let Some(id) = value.strip_prefix("restore:").and_then(|id| id.parse::<Id>().ok()) {
                                state.write().restore_compile_profile(id);
                            } else if let Ok(id) = value.parse::<Id>() {
                                state.write().choose_compile_profile(id);
                            }
                        },
                        for kept in profiles {
                            option { value: "{kept.id}", selected: kept.id == chosen, "{kept.name}" }
                        }
                        for gone in removed {
                            option { value: "restore:{gone.id}", "Put back “{gone.name}”" }
                        }
                    }
                    button { title: "Make a copy of this profile to change", onclick: move |_| state.write().copy_compile_profile(), "Copy" }
                    if built_in {
                        button {
                            title: "Put this profile back as it ships",
                            onclick: move |_| state.write().restore_compile_profile(chosen),
                            "Reset"
                        }
                    }
                    button { class: "danger", onclick: move |_| state.write().remove_compile_profile(), "Remove" }
                }
            }
            fieldset { class: "compile-profile",
                legend { "{profile.name}" }
                div { class: "compile-grid",
                    label { r#for: "profile-name", "Name" }
                    input {
                        id: "profile-name",
                        value: "{profile.name}",
                        onchange: move |event| {
                            let name = event.value();
                            change_profile(state, move |profile| profile.name = name);
                        },
                    }
                    label { r#for: "profile-format", "Format" }
                    select {
                        id: "profile-format",
                        onchange: move |event| {
                            let picked = CompileFormat::ALL.into_iter().find(|format| format.extension() == event.value());
                            if let Some(format) = picked {
                                change_profile(state, move |profile| profile.format = format);
                            }
                        },
                        for format in CompileFormat::ALL {
                            option { value: "{format.extension()}", selected: format == profile.format, "{format.label()}" }
                        }
                    }
                    span {}
                    label { class: "check-label",
                        input {
                            r#type: "checkbox",
                            checked: profile.title_page,
                            onchange: move |event| {
                                let on = event.checked();
                                change_profile(state, move |profile| profile.title_page = on);
                            },
                        }
                        "Title page"
                    }
                }
                div { class: "compile-grid",
                    label { r#for: "profile-chapters", "Chapters" }
                    div { class: "compile-row",
                        select {
                            id: "profile-chapters",
                            onchange: move |event| {
                                if let Some(style) = HeadingStyle::ALL.into_iter().find(|style| style.label() == event.value()) {
                                    change_profile(state, move |profile| profile.chapters = style);
                                }
                            },
                            for style in HeadingStyle::ALL {
                                option { value: "{style.label()}", selected: style == profile.chapters, "{style.label()}" }
                            }
                        }
                        input {
                            class: "compile-word",
                            aria_label: "The word before a chapter's number",
                            title: "The word before a chapter's number, in the book's language",
                            value: "{profile.chapter_word}",
                            onchange: move |event| {
                                let word = event.value();
                                change_profile(state, move |profile| profile.chapter_word = word);
                            },
                        }
                    }
                    label { r#for: "profile-parts", "Parts" }
                    div { class: "compile-row",
                        select {
                            id: "profile-parts",
                            onchange: move |event| {
                                if let Some(style) = HeadingStyle::ALL.into_iter().find(|style| style.label() == event.value()) {
                                    change_profile(state, move |profile| profile.parts = style);
                                }
                            },
                            for style in HeadingStyle::ALL {
                                option { value: "{style.label()}", selected: style == profile.parts, "{style.label()}" }
                            }
                        }
                        input {
                            class: "compile-word",
                            aria_label: "The word before a part's number",
                            title: "The word before a part's number, in the book's language",
                            value: "{profile.part_word}",
                            onchange: move |event| {
                                let word = event.value();
                                change_profile(state, move |profile| profile.part_word = word);
                            },
                        }
                    }
                    label { r#for: "profile-scenes", "Between scenes" }
                    select {
                        id: "profile-scenes",
                        onchange: move |event| {
                            if let Some(separator) = SceneSeparator::ALL.into_iter().find(|separator| separator.label() == event.value()) {
                                change_profile(state, move |profile| profile.scenes = separator);
                            }
                        },
                        for separator in SceneSeparator::ALL {
                            option { value: "{separator.label()}", selected: separator == profile.scenes, "{separator.label()}" }
                        }
                    }
                    span {}
                    span { class: "hint", "{heading_example(&profile)}" }
                }
                if profile.format.has_pages() {
                    div { class: "compile-grid",
                        label { r#for: "page-font", "Font" }
                        input {
                            id: "page-font",
                            value: "{page.font}",
                            onchange: move |event| {
                                let font = event.value();
                                change_profile(state, move |profile| profile.page.font = font);
                            },
                        }
                        label { r#for: "page-size-pt", "Size" }
                        div { class: "compile-row",
                            input {
                                id: "page-size-pt",
                                r#type: "number",
                                min: "{FONT_SIZE_RANGE.0}",
                                max: "{FONT_SIZE_RANGE.1}",
                                value: "{page.font_size}",
                                onchange: move |event| {
                                    if let Ok(size) = event.value().parse::<u32>() {
                                        change_profile(state, move |profile| profile.page.font_size = size);
                                    }
                                },
                            }
                            span { class: "hint", "points" }
                        }
                        label { r#for: "page-spacing", "Line spacing" }
                        select {
                            id: "page-spacing",
                            onchange: move |event| {
                                let picked = LineSpacing::ALL.into_iter().find(|spacing| spacing.label() == event.value());
                                if let Some(spacing) = picked {
                                    change_profile(state, move |profile| profile.page.line_spacing = spacing);
                                }
                            },
                            for spacing in LineSpacing::ALL {
                                option { value: "{spacing.label()}", selected: spacing == page.line_spacing, "{spacing.label()}" }
                            }
                        }
                        label { r#for: "page-paper", "Page" }
                        select {
                            id: "page-paper",
                            onchange: move |event| {
                                let picked = PageSize::ALL.into_iter().find(|size| size.label() == event.value());
                                if let Some(size) = picked {
                                    change_profile(state, move |profile| profile.page.size = size);
                                }
                            },
                            for size in PageSize::ALL {
                                option { value: "{size.label()}", selected: size == page.size, "{size.label()}" }
                            }
                        }
                        label { r#for: "page-margin", "Margins" }
                        div { class: "compile-row",
                            input {
                                id: "page-margin",
                                r#type: "number",
                                step: "0.1",
                                min: "{MARGIN_RANGE.0 as f64 / 10.0}",
                                max: "{MARGIN_RANGE.1 as f64 / 10.0}",
                                value: "{margin}",
                                onchange: move |event| {
                                    if let Ok(inches) = event.value().parse::<f64>() {
                                        let tenths = (inches * 10.0).round().max(0.0) as u32;
                                        change_profile(state, move |profile| profile.page.margin = tenths);
                                    }
                                },
                            }
                            span { class: "hint", "inches" }
                        }
                        span {}
                        label { class: "check-label",
                            input {
                                r#type: "checkbox",
                                checked: page.indent,
                                onchange: move |event| {
                                    let on = event.checked();
                                    change_profile(state, move |profile| profile.page.indent = on);
                                },
                            }
                            "Indent paragraphs"
                        }
                        span {}
                        label { class: "check-label",
                            input {
                                r#type: "checkbox",
                                checked: page.chapter_new_page,
                                onchange: move |event| {
                                    let on = event.checked();
                                    change_profile(state, move |profile| profile.page.chapter_new_page = on);
                                },
                            }
                            "Start each chapter on a new page"
                        }
                    }
                }
            }
            div { class: "dialog-actions",
                button {
                    class: "primary",
                    onclick: move |_| {
                        state.write().compile_open = false;
                        export_project(state);
                    },
                    "Compile…"
                }
            }
        }
    }
}

/// Change the current book's profile and keep it.
fn change_profile(mut state: Signal<StudioState>, edit: impl FnOnce(&mut CompileProfile)) {
    let Some(mut current) = state.read().compile_profile() else {
        return;
    };
    edit(&mut current);
    state.write().change_compile_profile(current);
}

/// How the profile heads a chapter, shown under its choices: "Chapter 1: The
/// Harbor".
fn heading_example(profile: &CompileProfile) -> String {
    format!("A chapter reads “{}”.", profile.chapters.heading(&profile.chapter_word, 1, "The Harbor"))
}
