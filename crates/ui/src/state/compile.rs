//! Compiling a book with a profile, and keeping the project's profiles.

use writing_studio_core::Id;
use writing_studio_export::{compile_docx, compile_markdown, CompileInput};
use writing_studio_project::{CompileFormat, CompileProfile, ProjectSettings};

use super::{sanitize, ExportFile, StudioState};

impl StudioState {
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
        self.save_all();
        let Some(store) = self.store.clone() else {
            self.status = "Open a project to compile.".into();
            return None;
        };
        let Some(profile) = self.compile_profile() else {
            return None;
        };
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
        let input = CompileInput {
            title: &title,
            book_id: Some(book_id),
            tree: &self.binder,
            documents: &documents,
            profile: &profile,
            author: &author,
            subtitle: &subtitle,
            year: writing_studio_core::local_year(),
        };
        let bytes = match profile.format {
            CompileFormat::Markdown => compile_markdown(&input).into_bytes(),
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
        })
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
        assert_eq!(profiles.len(), 2, "the built-in ones");
        assert_eq!(state.compile_profile().unwrap().id, profiles[0].id);
        let file = state.prepare_compile().unwrap();
        assert_eq!(file.extension, "docx");
        assert!(file.bytes.starts_with(b"PK"));

        state.choose_compile_profile(profiles[1].id);
        let file = state.prepare_compile().unwrap();
        assert_eq!(file.extension, "md");
        assert!(String::from_utf8(file.bytes).unwrap().contains("The harbor was quiet."));

        // A scene left out stays in the binder and out of the file.
        let scene = state.selected.unwrap();
        state.set_left_out(scene, true);
        assert!(state.scene_groups().iter().flat_map(|group| &group.scenes).any(|card| card.id == scene && card.left_out));
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
