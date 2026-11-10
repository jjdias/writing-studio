//! Research notes and the files attached to them.
//!
//! Notes live in the project database. File bytes stay in the project folder
//! under `attachments/`. This crate only tracks those records.

use serde::{Deserialize, Serialize};
use writing_studio_core::{Id, Timestamp};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResearchItem {
    pub id: Id,
    pub title: String,
    pub notes: String,
    pub sort_index: i64,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl ResearchItem {
    fn new(title: impl Into<String>, sort_index: i64) -> Self {
        let now = Timestamp::now();
        Self {
            id: Id::new(),
            title: title.into(),
            notes: String::new(),
            sort_index,
            created_at: now,
            updated_at: now,
        }
    }

    fn touch(&mut self) {
        self.updated_at = Timestamp::now();
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResearchFile {
    pub id: Id,
    pub item_id: Id,
    pub file_name: String,
    /// Path relative to `attachments/`, always `item-id/file-id` plus an optional extension.
    pub stored_name: String,
    pub created_at: Timestamp,
}

impl ResearchFile {
    pub fn new(item_id: Id, file_name: impl Into<String>, stored_name: impl Into<String>) -> Self {
        Self {
            id: Id::new(),
            item_id,
            file_name: file_name.into(),
            stored_name: stored_name.into(),
            created_at: Timestamp::now(),
        }
    }
}

/// Every research note and attached file in one project.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ResearchLibrary {
    pub items: Vec<ResearchItem>,
    pub files: Vec<ResearchFile>,
}

impl ResearchLibrary {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn items_in_order(&self) -> Vec<&ResearchItem> {
        let mut items: Vec<&ResearchItem> = self.items.iter().collect();
        items.sort_by_key(|item| (item.sort_index, item.id));
        items
    }

    pub fn files_for(&self, item_id: Id) -> Vec<&ResearchFile> {
        let mut files: Vec<&ResearchFile> = self
            .files
            .iter()
            .filter(|file| file.item_id == item_id)
            .collect();
        files.sort_by_key(|file| (file.created_at, file.id));
        files
    }

    pub fn add_item(&mut self) -> Id {
        let sort_index = self
            .items
            .iter()
            .map(|item| item.sort_index)
            .max()
            .map(|index| index + 1)
            .unwrap_or(0);
        let item = ResearchItem::new(format!("Research {}", self.items.len() + 1), sort_index);
        let id = item.id;
        self.items.push(item);
        id
    }

    pub fn set_title(&mut self, id: Id, title: String) -> bool {
        let Some(item) = self.items.iter_mut().find(|item| item.id == id) else {
            return false;
        };
        if item.title == title {
            return false;
        }
        item.title = title;
        item.touch();
        true
    }

    pub fn set_notes(&mut self, id: Id, notes: String) -> bool {
        let Some(item) = self.items.iter_mut().find(|item| item.id == id) else {
            return false;
        };
        if item.notes == notes {
            return false;
        }
        item.notes = notes;
        item.touch();
        true
    }

    /// Removes the note. Returns the stored attachment paths that should be deleted.
    pub fn remove_item(&mut self, id: Id) -> Option<Vec<String>> {
        if !self.items.iter().any(|item| item.id == id) {
            return None;
        }
        let stored = self
            .files
            .iter()
            .filter(|file| file.item_id == id)
            .map(|file| file.stored_name.clone())
            .collect();
        self.items.retain(|item| item.id != id);
        self.files.retain(|file| file.item_id != id);
        Some(stored)
    }

    pub fn add_file(&mut self, file: ResearchFile) -> bool {
        if !self.items.iter().any(|item| item.id == file.item_id) {
            return false;
        }
        if self.files.iter().any(|existing| existing.id == file.id) {
            return false;
        }
        self.files.push(file);
        true
    }

    /// Removes one attachment record. Returns its stored path when it existed.
    pub fn remove_file(&mut self, id: Id) -> Option<String> {
        let position = self.files.iter().position(|file| file.id == id)?;
        Some(self.files.remove(position).stored_name)
    }
}

/// `attachments/{item}/{file}` plus an optional short alphanumeric extension.
pub fn stored_attachment_name(item_id: Id, file_id: Id, original_name: &str) -> String {
    let extension = std::path::Path::new(original_name)
        .extension()
        .and_then(|ext| ext.to_str())
        .filter(|ext| {
            !ext.is_empty()
                && ext.len() <= 8
                && ext.chars().all(|ch| ch.is_ascii_alphanumeric())
        })
        .map(|ext| format!(".{ext}"))
        .unwrap_or_default();
    format!("{item_id}/{file_id}{extension}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_keeps_files_and_removing_it_lists_them() {
        let mut library = ResearchLibrary::new();
        let item = library.add_item();
        assert!(library.set_title(item, "Maps".into()));
        assert!(library.set_notes(item, "Coastal charts.".into()));
        let file_id = Id::new();
        let stored = stored_attachment_name(item, file_id, "coast.pdf");
        assert!(stored.ends_with(".pdf"));
        assert!(!stored.contains(".."));
        let file = ResearchFile {
            id: file_id,
            item_id: item,
            file_name: "coast.pdf".into(),
            stored_name: stored.clone(),
            created_at: Timestamp::now(),
        };
        assert!(library.add_file(file));
        assert_eq!(library.files_for(item).len(), 1);
        assert_eq!(library.remove_item(item), Some(vec![stored]));
        assert!(library.files.is_empty());
        assert!(!library.add_file(ResearchFile::new(Id::new(), "x", "x")));
    }

    #[test]
    fn stored_names_drop_unsafe_extensions() {
        let item = Id::new();
        let file = Id::new();
        let stored = stored_attachment_name(item, file, "../../secret.txt");
        assert!(!stored.contains(".."));
        assert!(stored.ends_with(".txt"));
        assert!(stored.starts_with(&format!("{item}/")));
        let bare = stored_attachment_name(item, file, "notes.toolongext");
        assert!(!bare.contains('.'));
    }
}
