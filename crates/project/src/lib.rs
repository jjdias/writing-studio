//! Project structure: binder tree, settings, and node kinds.
//!
//! The binder is the source of truth for manuscript organization. Documents
//! are referenced by id and stored separately.

mod binder;
mod settings;

pub use binder::{BinderNode, BinderTree, NodeKind};
pub use settings::ProjectSettings;
pub use writing_studio_core::PROJECT_FORMAT_VERSION;

use serde::{Deserialize, Serialize};
use writing_studio_core::{Id, Timestamp};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectMeta {
    pub id: Id,
    pub format_version: u32,
    pub title: String,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    pub settings: ProjectSettings,
}

impl ProjectMeta {
    pub fn new(title: impl Into<String>) -> Self {
        let now = Timestamp::now();
        Self {
            id: Id::new(),
            format_version: PROJECT_FORMAT_VERSION,
            title: title.into(),
            created_at: now,
            updated_at: now,
            settings: ProjectSettings::default(),
        }
    }

    pub fn touch(&mut self) {
        self.updated_at = Timestamp::now();
    }
}
