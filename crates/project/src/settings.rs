use serde::{Deserialize, Serialize};

use crate::WritingLog;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectSettings {
    pub daily_word_goal: u32,
    pub project_word_goal: u32,
    pub compile_title_page: bool,
    pub autosave_seconds: u32,
    /// Daily baseline and completed writing sessions. Missing on older projects.
    #[serde(default)]
    pub writing: WritingLog,
}

impl Default for ProjectSettings {
    fn default() -> Self {
        Self {
            daily_word_goal: 500,
            project_word_goal: 80_000,
            compile_title_page: true,
            autosave_seconds: 2,
            writing: WritingLog::default(),
        }
    }
}
