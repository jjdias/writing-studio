//! Compiling a book with a profile, and keeping the project's profiles.

use writing_studio_core::Id;
use writing_studio_export::{compile_docx, compile_epub, compile_html, compile_markdown, CompileInput, EpubExtras};
use writing_studio_document::{tracked_changes, ReviewKind};
use writing_studio_project::{BinderNode, CompileFormat, CompileProfile, HeadingStyle, NodeKind, ProjectSettings, SceneSeparator};

use super::{sanitize, ExportFile, StudioState};

/// Something in the book that may not be meant to go out. None of it stops
/// a compile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckFinding {
    LeftOut,
    NoWords,
    /// A part, chapter, or scene with no title where the profile prints it.
    NoTitle,
    OpenNotes(usize),
    WaitingChanges(usize),
    UnknownWords(usize),
}

impl CheckFinding {
    /// The kinds in the order the dialog lists them.
    pub const GROUPS: [&'static str; 6] =
        ["Left out of the compile", "No words yet", "No title", "Open notes", "Tracked changes waiting", "Unknown words"];

    pub fn group(self) -> &'static str {
        let at = match self {
            CheckFinding::LeftOut => 0,
            CheckFinding::NoWords => 1,
            CheckFinding::NoTitle => 2,
            CheckFinding::OpenNotes(_) => 3,
            CheckFinding::WaitingChanges(_) => 4,
            CheckFinding::UnknownWords(_) => 5,
        };
        Self::GROUPS[at]
    }

    /// What the line says after the item's title, when a count says more
    /// than the group does.
    pub fn count(self) -> Option<usize> {
        match self {
            CheckFinding::OpenNotes(n) | CheckFinding::WaitingChanges(n) | CheckFinding::UnknownWords(n) => Some(n),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckLine {
    pub id: Id,
    pub title: String,
    pub finding: CheckFinding,
}

/// What the check before compiling found, item by item in reading order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CompileCheck {
    pub lines: Vec<CheckLine>,
}

impl CompileCheck {
    /// The findings in a sentence for the status line: "2 open notes and 1
    /// scene with no words".
    pub fn summary(&self) -> String {
        let items = |pick: fn(CheckFinding) -> bool| self.lines.iter().filter(|line| pick(line.finding)).count();
        let sum = |pick: fn(CheckFinding) -> Option<usize>| self.lines.iter().filter_map(|line| pick(line.finding)).sum::<usize>();
        let parts: Vec<String> = [
            (items(|f| f == CheckFinding::LeftOut), "item left out", "items left out"),
            (items(|f| f == CheckFinding::NoWords), "scene with no words", "scenes with no words"),
            (items(|f| f == CheckFinding::NoTitle), "item with no title", "items with no title"),
            (sum(|f| matches!(f, CheckFinding::OpenNotes(_)).then(|| f.count().unwrap_or(0))), "open note", "open notes"),
            (
                sum(|f| matches!(f, CheckFinding::WaitingChanges(_)).then(|| f.count().unwrap_or(0))),
                "tracked change waiting",
                "tracked changes waiting",
            ),
            (sum(|f| matches!(f, CheckFinding::UnknownWords(_)).then(|| f.count().unwrap_or(0))), "unknown word", "unknown words"),
        ]
        .into_iter()
        .filter(|(n, _, _)| *n > 0)
        .map(|(n, one, many)| format!("{n} {}", if n == 1 { one } else { many }))
        .collect();
        match parts.as_slice() {
            [] => String::new(),
            [one] => one.clone(),
            [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
        }
    }
}

impl StudioState {
    /// Look over the current book before it is compiled: what compile
    /// leaves out, scenes with no words, titles the profile would print
    /// empty, open notes, tracked changes waiting, and unknown words when
    /// spellcheck is on. The text is saved first, so the check sees what
    /// the compile will.
    pub fn check_compile(&mut self) -> CompileCheck {
        self.save_all();
        let mut check = CompileCheck::default();
        let (Some(store), Some(book), Some(profile)) = (self.store.clone(), self.compile_book_id(), self.compile_profile())
        else {
            return check;
        };
        let store = store.borrow();
        let tracked: std::collections::HashSet<Id> = store.tracked_document_ids().unwrap_or_default().into_iter().collect();
        let accepted = self.accepted_words();
        let prints_title = |style: HeadingStyle| matches!(style, HeadingStyle::Title | HeadingStyle::NumberAndTitle);
        let mut push = |node: &BinderNode, finding| {
            check.lines.push(CheckLine { id: node.id, title: node.title.clone(), finding });
        };
        for node in self.book_in_order(book) {
            if node.left_out {
                push(node, CheckFinding::LeftOut);
                continue;
            }
            // Inside something left out: not compiled, so not checked.
            if self.binder.is_left_out(node.id) {
                continue;
            }
            let untitled = node.title.trim().is_empty()
                && match node.kind {
                    NodeKind::Chapter => prints_title(profile.chapters),
                    NodeKind::Part => prints_title(profile.parts),
                    NodeKind::Scene => profile.scenes == SceneSeparator::Title,
                    _ => false,
                };
            if untitled {
                push(node, CheckFinding::NoTitle);
            }
            let Some(document) = node.document_id.and_then(|id| store.load_document(id).ok()) else {
                continue;
            };
            if node.kind == NodeKind::Scene && document.word_count() == 0 {
                push(node, CheckFinding::NoWords);
            }
            let notes = document
                .review_items()
                .iter()
                .filter(|item| matches!(item.kind, ReviewKind::Note { resolved: false, .. }))
                .count();
            if notes > 0 {
                push(node, CheckFinding::OpenNotes(notes));
            }
            if tracked.contains(&document.id) {
                if let Ok(Some((baseline, _))) = store.load_tracking_baseline(document.id) {
                    let waiting = tracked_changes(&baseline, &document).len();
                    if waiting > 0 {
                        push(node, CheckFinding::WaitingChanges(waiting));
                    }
                }
            }
            if let Some(speller) = &self.speller {
                let unknown = speller.unknown_words(&document.to_plain_text(), &accepted).len();
                if unknown > 0 {
                    push(node, CheckFinding::UnknownWords(unknown));
                }
            }
        }
        check
    }

    /// Close the Compile dialog and forget what its check found.
    pub fn close_compile(&mut self) {
        self.compile_open = false;
        self.compile_check = None;
        self.compile_preview = None;
    }

    /// The book and everything in it, in reading order, without the trash.
    fn book_in_order(&self, book: Id) -> Vec<&BinderNode> {
        let mut out = Vec::new();
        let Some(root) = self.binder.get(book) else {
            return out;
        };
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            if self.binder.is_in_trash(node.id) {
                continue;
            }
            out.push(node);
            let mut children = self.binder.children(Some(node.id));
            children.reverse();
            stack.extend(children);
        }
        out
    }

    /// The compiled book in a few words: "12 chapters, 84210 words".
    fn compiled_summary(&self, book: Id, documents: &[writing_studio_document::Document]) -> String {
        let compiled: Vec<&BinderNode> =
            self.book_in_order(book).into_iter().filter(|node| !self.binder.is_left_out(node.id)).collect();
        let chapters = compiled.iter().filter(|node| node.kind == NodeKind::Chapter).count();
        let words: u32 = compiled
            .iter()
            .filter_map(|node| node.document_id)
            .filter_map(|id| documents.iter().find(|document| document.id == id))
            .map(|document| document.word_count())
            .sum();
        let chapters = if chapters == 1 { "1 chapter".to_string() } else { format!("{chapters} chapters") };
        format!("{chapters}, {words} words")
    }
    /// The project's compile profiles, or the built-in ones.
    pub fn compile_profiles(&self) -> Vec<CompileProfile> {
        self.meta.as_ref().map(|meta| meta.settings.profiles()).unwrap_or_default()
    }

    /// The profile the current book compiles with: the one last chosen for
    /// it on this computer, or the first.
    pub fn compile_profile(&self) -> Option<CompileProfile> {
        let settings = &self.meta.as_ref()?.settings;
        match self.compile_book_id() {
            Some(book) => settings.profile_for_book(book),
            None => settings.profiles().into_iter().next(),
        }
    }

    /// Leave an item out of compile, with what it holds, or take it back in.
    pub fn set_left_out(&mut self, id: Id, left_out: bool) {
        let Some(node) = self.binder.get(id) else {
            return;
        };
        if node.kind.is_root() || node.left_out == left_out {
            return;
        }
        let title = node.title.clone();
        self.edit_node(id, |node| node.left_out = left_out);
        self.save_all();
        self.status = if left_out {
            format!("“{title}” is left out of compile.")
        } else {
            format!("“{title}” is compiled again.")
        };
    }

    /// Make `id` the book that is compiled, without changing what the
    /// library shows.
    pub fn remember_book_for_compile(&mut self, id: Id) {
        self.remember_compile_book(id);
    }

    /// Compile the current book with `id` from now on, on this computer.
    pub fn choose_compile_profile(&mut self, id: Id) {
        let Some(book) = self.compile_book_id() else {
            return;
        };
        self.edit_settings(|settings| {
            if settings.profile(id).is_some() {
                settings.choose_profile_for_book(book, id);
            }
        });
    }

    /// Keep a change to a profile.
    pub fn change_compile_profile(&mut self, profile: CompileProfile) {
        self.edit_settings(|settings| settings.save_profile(profile));
    }

    /// A copy of the chosen profile, chosen in its place.
    pub fn copy_compile_profile(&mut self) {
        let Some(chosen) = self.compile_profile() else {
            return;
        };
        let mut copy = None;
        self.edit_settings(|settings| copy = settings.copy_profile(chosen.id));
        match copy {
            Some(id) => {
                self.choose_compile_profile(id);
                self.status = format!("Copied “{}”. Change the copy as you like.", chosen.name);
            }
            None => self.status = "There are as many profiles as a project can keep.".into(),
        }
    }

    /// Remove the chosen profile. The book compiles with the first one left.
    pub fn remove_compile_profile(&mut self) {
        let Some(chosen) = self.compile_profile() else {
            return;
        };
        self.edit_settings(|settings| settings.remove_profile(chosen.id));
        self.status = if chosen.is_built_in() {
            format!("Removed “{}”. Put it back from the profile list.", chosen.name)
        } else {
            format!("Removed “{}”.", chosen.name)
        };
    }

    /// Put a built-in profile back as it ships, and choose it.
    pub fn restore_compile_profile(&mut self, id: Id) {
        self.edit_settings(|settings| settings.restore_profile(id));
        self.choose_compile_profile(id);
    }

    /// Built-in profiles that were removed, so they can be put back.
    pub fn removed_built_in_profiles(&self) -> Vec<CompileProfile> {
        let kept = self.compile_profiles();
        writing_studio_project::built_in_profiles()
            .into_iter()
            .filter(|profile| !kept.iter().any(|mine| mine.id == profile.id))
            .collect()
    }

    /// The project's author and the current book's subtitle, for the title
    /// page.
    pub fn compile_author_and_subtitle(&self) -> (String, String) {
        let Some(meta) = self.meta.as_ref() else {
            return Default::default();
        };
        let subtitle = self.compile_book_id().map(|book| meta.settings.subtitle_for_book(book)).unwrap_or_default();
        (meta.settings.author_name(), subtitle)
    }

    /// Whether the current book has a cover for its ebook.
    pub fn compile_has_cover(&self) -> bool {
        match (&self.store, self.compile_book_id()) {
            (Some(store), Some(book)) => store.borrow().has_book_cover(book),
            _ => false,
        }
    }

    /// Copy a picture into the project as the current book's cover.
    pub fn set_compile_cover(&mut self, source: &std::path::Path) {
        let (Some(store), Some(book)) = (self.store.clone(), self.compile_book_id()) else {
            return;
        };
        let result = store.borrow().set_book_cover(book, source);
        self.status = match result {
            Ok(_) => "The cover is kept with the project for this book's ebook.".into(),
            Err(err) => format!("Could not use that picture as the cover: {err}."),
        };
    }

    pub fn remove_compile_cover(&mut self) {
        let (Some(store), Some(book)) = (self.store.clone(), self.compile_book_id()) else {
            return;
        };
        let result = store.borrow().remove_book_cover(book);
        self.status = match result {
            Ok(()) => "Removed the cover.".into(),
            Err(err) => format!("Could not remove the cover: {err}."),
        };
    }

    /// The writer's name, shared by every book in the project.
    pub fn set_author(&mut self, author: &str) {
        self.edit_settings(|settings| settings.set_author(author));
    }

    /// The current book's subtitle.
    pub fn set_compile_subtitle(&mut self, subtitle: &str) {
        let Some(book) = self.compile_book_id() else {
            return;
        };
        self.edit_settings(|settings| settings.set_subtitle_for_book(book, subtitle));
    }

    fn edit_settings(&mut self, change: impl FnOnce(&mut ProjectSettings)) {
        let Some(meta) = self.meta.as_mut() else {
            return;
        };
        change(&mut meta.settings);
        meta.touch();
        self.settings_dirty = true;
        self.note_edit();
    }

    /// The current book compiled with its profile, ready to be written where
    /// the writer chooses.
    pub fn prepare_compile(&mut self) -> Option<ExportFile> {
        let gathered = self.gather_compile()?;
        let store = self.store.clone()?;
        let (profile, documents, book_id) = (&gathered.profile, gathered.documents.as_slice(), gathered.book_id);
        let title = gathered.title.clone();
        let input = gathered.input(&self.binder);
        let bytes = match profile.format {
            CompileFormat::Markdown => compile_markdown(&input).into_bytes(),
            CompileFormat::Html => compile_html(&input).into_bytes(),
            CompileFormat::Epub => {
                // A cover that cannot be read leaves the ebook without one
                // rather than without the ebook.
                let cover = store.borrow().book_cover(book_id).ok().flatten();
                let modified = writing_studio_core::Timestamp::now().to_utc_seconds();
                match compile_epub(&input, &EpubExtras { modified: &modified, cover: cover.as_deref() }) {
                    Ok(bytes) => bytes,
                    Err(err) => {
                        self.status = format!("Could not compile the ebook: {err}");
                        return None;
                    }
                }
            }
            CompileFormat::Docx => match compile_docx(&input) {
                Ok(bytes) => bytes,
                Err(err) => {
                    self.status = format!("Could not compile DOCX: {err}");
                    return None;
                }
            },
        };
        let extension = profile.format.extension();
        Some(ExportFile {
            bytes,
            file_name: format!("{}.{extension}", sanitize(&title)),
            filter_name: profile.format.label(),
            extension,
            kind_label: profile.format.label(),
            summary: self.compiled_summary(book_id, documents),
        })
    }

    /// The current book as the profile sets it, as one web page for the
    /// Compile dialog to show. Nothing is written. Whatever the profile's
    /// format, the preview is the web page with the profile's type, so a
    /// Word or ebook profile shows its text and matter, not its pages.
    pub fn preview_compile(&mut self) -> Option<String> {
        let gathered = self.gather_compile()?;
        Some(compile_html(&gathered.input(&self.binder)))
    }

    /// What a compile of the current book reads, after saving the text.
    fn gather_compile(&mut self) -> Option<Gathered> {
        self.save_all();
        let Some(store) = self.store.clone() else {
            self.status = "Open a project to compile.".into();
            return None;
        };
        let profile = self.compile_profile()?;
        let documents = match store.borrow().load_all_documents() {
            Ok(documents) => documents,
            Err(err) => {
                self.status = format!("Could not load manuscript for compile: {err}");
                return None;
            }
        };
        let Some(book_id) = self.compile_book_id() else {
            self.status = "Add a book before compiling.".into();
            return None;
        };
        let title = self.binder.get(book_id).map(|node| node.title.clone()).unwrap_or_else(|| self.project_title());
        let (author, subtitle) = self.compile_author_and_subtitle();
        Some(Gathered { profile, documents, book_id, title, author, subtitle })
    }
}

/// The book, its profile, and the names for its title page, read once for
/// a compile or a preview.
struct Gathered {
    profile: CompileProfile,
    documents: Vec<writing_studio_document::Document>,
    book_id: Id,
    title: String,
    author: String,
    subtitle: String,
}

impl Gathered {
    fn input<'a>(&'a self, tree: &'a writing_studio_project::BinderTree) -> CompileInput<'a> {
        CompileInput {
            title: &self.title,
            book_id: Some(self.book_id),
            tree,
            documents: &self.documents,
            profile: &self.profile,
            author: &self.author,
            subtitle: &self.subtitle,
            year: writing_studio_core::local_year(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn open_state(tag: &str) -> (StudioState, PathBuf) {
        let dir = std::env::temp_dir().join(format!("ws-ui-{tag}-{}", Id::new()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut state = StudioState::default();
        state.create_project_in(dir.clone());
        assert!(state.session.is_some(), "{}", state.status);
        (state, dir)
    }

    #[test]
    fn a_folder_has_text_of_its_own_that_is_context_and_never_compiled() {
        let (mut state, dir) = open_state("folder-text");
        state.apply_editor_text("The harbor was quiet.".into());
        let words = state.compile_book_word_count();

        // A folder under Notes is a page: its text is the first context of
        // what it holds.
        let notes = state.binder.notes_root().unwrap().id;
        let research = state.binder.children(Some(notes))[1].id;
        state.select_node(research);
        assert!(state.session.is_some(), "a folder opens in the page");
        state.apply_editor_markdown("Sources on the **harbor** town.");
        state.flush_editor();
        let context = state.level_context(research).unwrap();
        assert_eq!(context.about, "Sources on the harbor town.");
        assert!(context.to_markdown(16_000).contains("## About\n\nSources on the harbor town."));

        // A folder in the manuscript can say what it holds, too; the book
        // does not print or count it.
        let book = state.compile_book_id().unwrap();
        let chapter = state.binder.children(Some(book))[0].id;
        state.select_node(chapter);
        state.add_node(NodeKind::Folder);
        let folder = state.selected.unwrap();
        state.select_node(folder);
        state.apply_editor_markdown("Scenes cut from the draft, kept for later.");
        state.flush_editor();
        assert_eq!(state.compile_book_word_count(), words);
        let page = state.preview_compile().unwrap();
        assert!(!page.contains("kept for later") && page.contains("The harbor was quiet."), "{page}");

        // A file added to a note is copied into the project and linked.
        state.select_node(research);
        state.add_node(NodeKind::Note);
        let note = state.selected.unwrap();
        let source = dir.join("map.png");
        std::fs::write(&source, b"not really a picture").unwrap();
        let link = state.attach_file_to_note(note, source).unwrap();
        assert!(link.starts_with("[map.png](../attachments/") && link.ends_with(".png)"), "{link}");
        let stored = link.trim_start_matches("[map.png](../").trim_end_matches(')');
        assert_eq!(std::fs::read(state.project_root().unwrap().join(stored)).unwrap(), b"not really a picture");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn notes_start_from_templates_and_a_note_can_become_one() {
        let (mut state, dir) = open_state("templates");
        let titles: Vec<String> = state.note_templates().into_iter().map(|(_, title)| title).collect();
        assert_eq!(titles, ["New character", "New place", "New research note", "New source"]);

        let notes = state.binder.notes_root().unwrap().id;
        let research = state.binder.children(Some(notes))[1].id;
        let note = state.new_note_from_template(research, "character.md").unwrap();
        let node = state.binder.get(note).unwrap().clone();
        assert_eq!((node.kind, node.parent_id, node.title.as_str()), (NodeKind::Note, Some(research), "New character"));
        assert_eq!(node.keywords, ["character"]);
        assert_eq!(state.selected, Some(note));
        assert!(state.editor_markdown().contains("## What they want"), "{}", state.editor_markdown());

        // A note kept as a template is offered for the next note.
        state.update_node_title(note, "Mara".into());
        state.apply_editor_markdown("A pilot.\n\n## Fears\n\nThe open sea.");
        state.save_note_as_template(note);
        assert!(state.status.contains("templates/mara.md"), "{}", state.status);
        let (name, _) = state.note_templates().into_iter().find(|(_, title)| title == "Mara").unwrap();
        let copy = state.new_note_from_template(research, &name).unwrap();
        assert_eq!(state.binder.get(copy).unwrap().keywords, ["character"]);
        assert!(state.editor_markdown().contains("## Fears"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn notes_are_linked_by_title_and_from_context_and_renaming_follows() {
        let (mut state, dir) = open_state("links");
        let scene = state.selected.unwrap();
        let notes = state.binder.notes_root().unwrap().id;
        let research = state.binder.children(Some(notes))[1].id;
        let mara = state.new_note_from_template(research, "character.md").unwrap();
        state.rename_item(mara, "Mara");

        // The scene mentions her; she is in its backlinks, found by title.
        state.select_node(scene);
        state.apply_editor_markdown("Ask [[mara]], or [[Mara|the pilot's daughter]].");
        state.flush_editor();
        assert_eq!(state.note_titled("MARA"), Some(mara));
        assert_eq!(state.mentions_of(mara), [scene]);
        assert!(state.note_link_targets().iter().any(|(title, _)| title == "Mara"));

        // A link from Context is kept in the note's file.
        let chapter = state.binder.get(scene).unwrap().parent_id.unwrap();
        state.set_note_link(mara, chapter, true);
        let file = state.project_root().unwrap().join(state.store.as_ref().unwrap().borrow().file_of(mara).unwrap());
        assert!(std::fs::read_to_string(&file).unwrap().contains(&format!("links:\n  - {chapter}\n")));

        // Renaming her changes the links to her, keeping the words they show.
        state.select_node(mara);
        state.rename_item(mara, "Mara Vance");
        assert!(state.status.contains("in 1 place"), "{}", state.status);
        state.select_node(scene);
        assert_eq!(state.editor_markdown(), "Ask [[Mara Vance]], or [[Mara Vance|the pilot's daughter]].");
        assert_eq!(state.mentions_of(mara), [scene]);

        // A link opens its note.
        state.open_note_titled("mara vance");
        assert_eq!(state.selected, Some(mara));
        state.open_note_titled("Nobody");
        assert!(state.status.contains("No note is called “Nobody”"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn the_preview_is_the_web_page_for_any_profile_and_writes_nothing() {
        let (mut state, dir) = open_state("preview");
        state.apply_editor_text("The harbor was quiet.".into());
        let manuscript = state.compile_profiles()[0].clone();
        assert_eq!(manuscript.format, CompileFormat::Docx);
        state.choose_compile_profile(manuscript.id);
        let before: Vec<_> = std::fs::read_dir(&dir).unwrap().flatten().map(|entry| entry.path()).collect();
        // The text not yet saved is in it: the preview saves first, as a compile does.
        state.apply_editor_text("The harbor was quiet. A gull called.".into());
        let page = state.preview_compile().unwrap();
        assert!(page.starts_with("<!DOCTYPE html>"));
        assert!(page.contains("The harbor was quiet. A gull called."), "{page}");
        assert!(page.contains("font-family: \"Times New Roman\""), "the profile's own type");
        let after: Vec<_> = std::fs::read_dir(&dir).unwrap().flatten().map(|entry| entry.path()).collect();
        assert_eq!(before, after, "nothing is written beside the project");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn the_check_before_compiling_finds_each_kind_once_and_skips_what_is_left_out() {
        use writing_studio_document::{Block, BlockKind, Document, Inline, Marks};
        let (mut state, dir) = open_state("check");
        let first = state.selected.unwrap();
        state.apply_editor_text("The harbr was quiet.".into());
        state.add_node(NodeKind::Scene);
        let empty = state.selected.unwrap();
        state.add_node(NodeKind::Scene);
        let noted = state.selected.unwrap();
        state.add_node(NodeKind::Scene);
        let cut = state.selected.unwrap();
        state.add_node(NodeKind::Chapter);
        let untitled = state.selected.unwrap();
        state.update_node_title(untitled, String::new());
        state.select_node(first);
        state.set_left_out(cut, true);

        // A scene with an open note, a resolved one, and a change waiting.
        let doc = state.binder.get(noted).unwrap().document_id.unwrap();
        let run = |text: &str, marks: Marks| Inline { id: Id::new(), text: text.into(), marks };
        let note = |text: &str, resolved| Marks { comment: Some(text.into()), comment_resolved: resolved, ..Marks::default() };
        let mut text = Document::new(doc);
        text.blocks = vec![Block {
            id: Id::new(),
            kind: BlockKind::Paragraph {
                inlines: vec![run("Salt ", note("check", false)), run("on the pier.", note("done", true))],
            },
        }];
        {
            let store = state.store.as_ref().unwrap().borrow();
            store.save_document(&text).unwrap();
            // The baseline had one more paragraph: one change, as the
            // Changes tab would list it.
            store.save_tracking_baseline(&Document::from_plain_text(doc, "Salt on the pier.\nA gull.")).unwrap();
            // Words in the left-out scene, which neither the check nor the count may see.
            let cut_doc = state.binder.get(cut).unwrap().document_id.unwrap();
            store.save_document(&Document::from_plain_text(cut_doc, "Never compiled harbr words.")).unwrap();
        }

        let check = state.check_compile();
        let found = |id| check.lines.iter().filter(|line| line.id == id).map(|line| line.finding).collect::<Vec<_>>();
        assert_eq!(found(first), [CheckFinding::UnknownWords(1)], "{:?}", check.lines);
        assert_eq!(found(empty), [CheckFinding::NoWords]);
        assert_eq!(found(noted), [CheckFinding::OpenNotes(1), CheckFinding::WaitingChanges(1)]);
        assert_eq!(found(cut), [CheckFinding::LeftOut], "a left-out scene is not checked for words");
        assert_eq!(found(untitled), [CheckFinding::NoTitle]);
        assert_eq!(
            check.summary(),
            "1 item left out, 1 scene with no words, 1 item with no title, 1 open note, 1 tracked change waiting and 1 unknown word"
        );

        // After the compile the status says what was made; with nothing
        // found, the check is clean.
        let file = state.prepare_compile().unwrap();
        assert_eq!(file.summary, "2 chapters, 8 words", "the left-out scene's words are not counted");
        assert!(CompileCheck::default().summary().is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn the_author_and_subtitle_are_shared_and_printed_with_the_matter() {
        let (mut state, dir) = open_state("matter");
        state.apply_editor_text("The harbor was quiet.".into());
        let plain = state.compile_profiles()[1].id;
        state.choose_compile_profile(plain);
        state.set_author("  Maya Reis\n");
        state.set_compile_subtitle("A Novel");
        let mut profile = state.compile_profile().unwrap();
        profile.matter.copyright = true;
        profile.matter.about_author = true;
        profile.matter.about_author_text = "Maya lives by the sea.".into();
        state.change_compile_profile(profile);
        let text = String::from_utf8(state.prepare_compile().unwrap().bytes).unwrap();
        let year = writing_studio_core::local_year();
        assert!(text.contains("*A Novel*") && text.contains("\nMaya Reis\n"), "{text}");
        assert!(text.contains(&format!("Copyright © {year} Maya Reis.")), "{text}");
        assert!(text.contains("Maya lives by the sea."), "{text}");

        state.save_all();
        let root = state.project_root().unwrap();
        let shared = std::fs::read_to_string(root.join("project.json")).unwrap();
        assert!(shared.contains("\"author\": \"Maya Reis\"") && shared.contains("book_subtitles"), "{shared}");
        let mut reopened = StudioState::default();
        reopened.open_project_at(root);
        assert_eq!(reopened.compile_author_and_subtitle(), ("Maya Reis".into(), "A Novel".into()));
        reopened.set_compile_subtitle("");
        assert_eq!(reopened.compile_author_and_subtitle().1, "", "an empty subtitle removes it");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_book_compiles_with_its_profile_and_profiles_are_shared_with_the_project() {
        let (mut state, dir) = open_state("profiles");
        state.apply_editor_text("The harbor was quiet.".into());
        let profiles = state.compile_profiles();
        assert_eq!(profiles.len(), 4, "the built-in ones");
        assert_eq!(state.compile_profile().unwrap().id, profiles[0].id);
        let file = state.prepare_compile().unwrap();
        assert_eq!(file.extension, "docx");
        assert!(file.bytes.starts_with(b"PK"));

        state.choose_compile_profile(profiles[1].id);
        let file = state.prepare_compile().unwrap();
        assert_eq!(file.extension, "md");
        assert!(String::from_utf8(file.bytes).unwrap().contains("The harbor was quiet."));
        state.choose_compile_profile(profiles[2].id);
        let file = state.prepare_compile().unwrap();
        assert_eq!(file.extension, "html");
        let page = String::from_utf8(file.bytes).unwrap();
        assert!(page.starts_with("<!DOCTYPE html>") && page.contains("<p>The harbor was quiet.</p>"), "{page}");

        // An ebook, with the cover kept in the project for this book.
        state.choose_compile_profile(profiles[3].id);
        assert!(!state.compile_has_cover());
        let picture = dir.join("cover-source.png");
        std::fs::write(&picture, b"\x89PNG\r\n\x1a\npicture").unwrap();
        state.set_compile_cover(&picture);
        assert!(state.compile_has_cover(), "{}", state.status);
        let file = state.prepare_compile().unwrap();
        assert_eq!(file.extension, "epub");
        let names: Vec<String> =
            zip::ZipArchive::new(std::io::Cursor::new(file.bytes)).unwrap().file_names().map(String::from).collect();
        assert_eq!(names[0], "mimetype");
        assert!(names.iter().any(|name| name == "OEBPS/cover.png"), "{names:?}");
        state.remove_compile_cover();
        assert!(!state.compile_has_cover());
        state.choose_compile_profile(profiles[1].id);

        // A scene left out stays in the binder and out of the file.
        let scene = state.selected.unwrap();
        state.set_left_out(scene, true);
        assert!(state.binder_rows().iter().any(|row| row.id == scene && row.left_out));
        let file = state.prepare_compile().unwrap();
        assert!(!String::from_utf8(file.bytes).unwrap().contains("The harbor was quiet."));
        state.set_left_out(scene, false);
        assert!(!state.binder.get(scene).unwrap().left_out);

        state.copy_compile_profile();
        let mut copy = state.compile_profile().unwrap();
        assert_eq!(copy.name, "Plain text copy");
        copy.name = "Reading copy".into();
        copy.title_page = false;
        state.change_compile_profile(copy.clone());
        state.remove_compile_profile();
        assert!(state.compile_profiles().iter().all(|profile| profile.id != copy.id));
        let manuscript = profiles[0].id;
        state.choose_compile_profile(manuscript);
        state.remove_compile_profile();
        assert_eq!(state.removed_built_in_profiles().len(), 1);
        state.restore_compile_profile(manuscript);
        assert!(state.removed_built_in_profiles().is_empty());
        assert_eq!(state.compile_profile().unwrap().id, manuscript);

        state.copy_compile_profile();
        let kept = state.compile_profiles();
        state.save_all();
        let root = state.project_root().unwrap();
        let mut reopened = StudioState::default();
        reopened.open_project_at(root.clone());
        assert_eq!(reopened.compile_profiles(), kept);
        let shared = std::fs::read_to_string(root.join("project.json")).unwrap();
        assert!(shared.contains("compile_profiles"), "profiles go with the project");
        assert!(!shared.contains("compile_choices"), "the choice of profile stays on this computer");
        let _ = std::fs::remove_dir_all(dir);
    }
}
