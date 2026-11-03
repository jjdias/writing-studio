use serde::{Deserialize, Serialize};
use writing_studio_core::Id;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Location {
    pub id: Id,
    pub name: String,
    pub summary: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimelineEvent {
    pub id: Id,
    pub title: String,
    pub when_label: String,
}
