//! Project structure: binder tree, settings, and node kinds.
//!
//! The binder is the source of truth for manuscript organization. Documents
//! are referenced by id and stored separately.

mod binder;
mod settings;
mod writing;

pub use binder::{binder_color_hex, normalize_binder_color, BinderColor, BinderNode, BinderTree, NodeKind, BINDER_COLORS};
pub use settings::{
    drag_width, BookWordGoal, PanelChrome, PanelLayout, PanelWidths, ProjectSettings, SidebarSide,
    Theme,
    INSPECTOR_WIDTH_DEFAULT, INSPECTOR_WIDTH_MAX, INSPECTOR_WIDTH_MIN, SIDEBAR_WIDTH_DEFAULT,
    SIDEBAR_WIDTH_MAX, SIDEBAR_WIDTH_MIN,
};
pub use writing::{format_elapsed, StoredSession, WritingLog};
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
