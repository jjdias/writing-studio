use serde::{Deserialize, Serialize};
use writing_studio_core::Id;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Character {
    pub id: Id,
    pub name: String,
    pub role: String,
    pub summary: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CharacterRelationship {
    pub id: Id,
    pub from_id: Id,
    pub to_id: Id,
    pub kind: String,
}
