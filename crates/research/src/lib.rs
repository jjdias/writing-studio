use serde::{Deserialize, Serialize};
use writing_studio_core::Id;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResearchItem {
    pub id: Id,
    pub title: String,
    pub notes: String,
}
