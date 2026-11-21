use super::*;
use writing_studio_project::{
    BookLanguage, CompileFormat, CompileProfile, HeadingStyle, LineSpacing, PageSize, SceneSeparator, FONT_SIZE_RANGE, MARGIN_RANGE,
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
    let (author, subtitle) = state.read().compile_author_and_subtitle();
    let matter = profile.matter.clone();
    let language = profile.language;
    let also_by_example = profile.also_by_heading(&author);
    let copyright_example = matter.copyright_line(writing_studio_core::local_year(), &author);
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
                label { r#for: "compile-subtitle", "Subtitle" }
                input {
                    id: "compile-subtitle",
                    placeholder: "None",
                    title: "This book's subtitle, for its title page",
                    value: "{subtitle}",
                    onchange: move |event| state.write().set_compile_subtitle(&event.value()),
                }
                label { r#for: "compile-author", "Author" }
                input {
                    id: "compile-author",
                    placeholder: "Your name as the book prints it",
                    title: "Shared by every book in the project",
                    value: "{author}",
                    onchange: move |event| state.write().set_author(&event.value()),
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
                    label { r#for: "profile-language", "Language" }
                    select {
                        id: "profile-language",
                        title: "The number words, the word before a chapter's number, and the headings of the matter",
                        onchange: move |event| {
                            if let Some(language) = BookLanguage::ALL.into_iter().find(|language| language.label() == event.value()) {
                                change_profile(state, move |profile| profile.set_language(language));
                            }
                        },
                        for language in BookLanguage::ALL {
                            option { value: "{language.label()}", selected: language == profile.language, "{language.label()}" }
                        }
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
                div { class: "compile-grid",
                    span { class: "compile-group", "Front matter" }
                    div { class: "compile-checks",
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
                        label { class: "check-label",
                            input {
                                r#type: "checkbox",
                                checked: matter.copyright,
                                onchange: move |event| {
                                    let on = event.checked();
                                    change_profile(state, move |profile| profile.matter.copyright = on);
                                },
                            }
                            "Copyright page"
                        }
                        label { class: "check-label",
                            input {
                                r#type: "checkbox",
                                checked: matter.dedication,
                                onchange: move |event| {
                                    let on = event.checked();
                                    change_profile(state, move |profile| profile.matter.dedication = on);
                                },
                            }
                            "Dedication"
                        }
                        label { class: "check-label",
                            input {
                                r#type: "checkbox",
                                checked: matter.contents,
                                onchange: move |event| {
                                    let on = event.checked();
                                    change_profile(state, move |profile| profile.matter.contents = on);
                                },
                            }
                            "Contents"
                        }
                    }
                    if matter.copyright {
                        label { r#for: "matter-copyright", "Copyright" }
                        div {
                            input {
                                id: "matter-copyright",
                                value: "{matter.copyright_text}",
                                onchange: move |event| {
                                    let text = event.value();
                                    change_profile(state, move |profile| profile.matter.copyright_text = text);
                                },
                            }
                            p { class: "hint", "{{year}} and {{author}} are filled in: “{copyright_example}”" }
                        }
                    }
                    if matter.dedication {
                        label { r#for: "matter-dedication", "Dedication" }
                        textarea {
                            id: "matter-dedication",
                            rows: "2",
                            value: "{matter.dedication_text}",
                            onchange: move |event| {
                                let text = event.value();
                                change_profile(state, move |profile| profile.matter.dedication_text = text);
                            },
                        }
                    }
                    if matter.contents {
                        label { r#for: "matter-contents-heading", "Contents heading" }
                        input {
                            id: "matter-contents-heading",
                            placeholder: "{language.contents()}",
                            value: "{matter.contents_heading}",
                            onchange: move |event| {
                                let text = event.value();
                                change_profile(state, move |profile| profile.matter.contents_heading = text);
                            },
                        }
                    }
                    span { class: "compile-group", "Back matter" }
                    div { class: "compile-checks",
                        label { class: "check-label",
                            input {
                                r#type: "checkbox",
                                checked: matter.about_author,
                                onchange: move |event| {
                                    let on = event.checked();
                                    change_profile(state, move |profile| profile.matter.about_author = on);
                                },
                            }
                            "About the author"
                        }
                        label { class: "check-label",
                            input {
                                r#type: "checkbox",
                                checked: matter.also_by,
                                onchange: move |event| {
                                    let on = event.checked();
                                    change_profile(state, move |profile| profile.matter.also_by = on);
                                },
                            }
                            "Also by"
                        }
                    }
                    if matter.about_author {
                        label { r#for: "matter-about", "About the author" }
                        div {
                            input {
                                aria_label: "About the author heading",
                                placeholder: "{language.about_author()}",
                                value: "{matter.about_author_heading}",
                                onchange: move |event| {
                                    let text = event.value();
                                    change_profile(state, move |profile| profile.matter.about_author_heading = text);
                                },
                            }
                            textarea {
                                id: "matter-about",
                                class: "under-heading",
                                rows: "4",
                                value: "{matter.about_author_text}",
                                onchange: move |event| {
                                    let text = event.value();
                                    change_profile(state, move |profile| profile.matter.about_author_text = text);
                                },
                            }
                        }
                    }
                    if matter.also_by {
                        label { r#for: "matter-also-by", "Also by" }
                        div {
                            input {
                                aria_label: "Also by heading",
                                placeholder: "{language.also_by()}",
                                value: "{matter.also_by_heading}",
                                onchange: move |event| {
                                    let text = event.value();
                                    change_profile(state, move |profile| profile.matter.also_by_heading = text);
                                },
                            }
                            textarea {
                                id: "matter-also-by",
                                class: "under-heading",
                                rows: "3",
                                value: "{matter.also_by_text}",
                                onchange: move |event| {
                                    let text = event.value();
                                    change_profile(state, move |profile| profile.matter.also_by_text = text);
                                },
                            }
                            p { class: "hint", "The heading, then one title to a line. {{author}} in the heading is filled in: “{also_by_example}”." }
                        }
                    }
                }
                if profile.format.has_pages() {
                    div { class: "compile-grid",
                        label { r#for: "page-font", "Font" }
                        select {
                            id: "page-font",
                            title: "The fonts on this computer. The file names the font; another computer without it uses one of its own.",
                            onchange: move |event| {
                                let font = event.value();
                                change_profile(state, move |profile| profile.page.font = font);
                            },
                            for (font, label) in crate::fonts::font_choices(&page.font, crate::fonts::installed_families()) {
                                option { value: "{font}", selected: font.eq_ignore_ascii_case(&page.font), "{label}" }
                            }
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
    format!("A chapter reads “{}”.", profile.chapters.heading(profile.language, &profile.chapter_word, 1, "The Harbor"))
}
