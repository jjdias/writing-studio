//! Plotlines and plot points. Persistence and UI land in Phase 3.

use serde::{Deserialize, Serialize};
use writing_studio_core::Id;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Plotline {
    pub id: Id,
    pub title: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlotPoint {
    pub id: Id,
    pub plotline_id: Id,
    pub title: String,
    pub scene_id: Option<Id>,
}
