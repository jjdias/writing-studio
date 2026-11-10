//! Character profiles, motivations, and relationships.
//!
//! The cast is local project data. A relationship points at two characters in
//! the same project. It does not copy manuscript text.

use serde::{Deserialize, Serialize};
use writing_studio_core::{Id, Timestamp};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Character {
    pub id: Id,
    pub name: String,
    pub role: String,
    pub summary: String,
    pub sort_index: i64,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl Character {
    pub fn new(name: impl Into<String>, sort_index: i64) -> Self {
        let now = Timestamp::now();
        Self {
            id: Id::new(),
            name: name.into(),
            role: String::new(),
            summary: String::new(),
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
pub struct Motivation {
    pub id: Id,
    pub character_id: Id,
    pub body: String,
    pub sort_index: i64,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl Motivation {
    pub fn new(character_id: Id, sort_index: i64) -> Self {
        let now = Timestamp::now();
        Self {
            id: Id::new(),
            character_id,
            body: String::new(),
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
pub struct CharacterRelationship {
    pub id: Id,
    pub from_id: Id,
    pub to_id: Id,
    pub kind: String,
    pub sort_index: i64,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl CharacterRelationship {
    pub fn new(from_id: Id, to_id: Id, sort_index: i64) -> Self {
        let now = Timestamp::now();
        Self {
            id: Id::new(),
            from_id,
            to_id,
            kind: String::new(),
            sort_index,
            created_at: now,
            updated_at: now,
        }
    }

    fn touch(&mut self) {
        self.updated_at = Timestamp::now();
    }
}

/// Every character, motivation, and relationship in one project.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Cast {
    pub characters: Vec<Character>,
    pub motivations: Vec<Motivation>,
    pub relationships: Vec<CharacterRelationship>,
}

impl Cast {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn characters_in_order(&self) -> Vec<&Character> {
        let mut characters: Vec<&Character> = self.characters.iter().collect();
        characters.sort_by_key(|character| (character.sort_index, character.id));
        characters
    }

    pub fn motivations_for(&self, character_id: Id) -> Vec<&Motivation> {
        let mut items: Vec<&Motivation> = self
            .motivations
            .iter()
            .filter(|item| item.character_id == character_id)
            .collect();
        items.sort_by_key(|item| (item.sort_index, item.id));
        items
    }

    pub fn relationships_from(&self, character_id: Id) -> Vec<&CharacterRelationship> {
        let mut items: Vec<&CharacterRelationship> = self
            .relationships
            .iter()
            .filter(|item| item.from_id == character_id)
            .collect();
        items.sort_by_key(|item| (item.sort_index, item.id));
        items
    }

    pub fn add_character(&mut self) -> Id {
        let sort_index = self.next_character_index();
        let name = format!("Character {}", self.characters.len() + 1);
        let character = Character::new(name, sort_index);
        let id = character.id;
        self.characters.push(character);
        id
    }

    pub fn set_name(&mut self, id: Id, name: String) -> bool {
        self.edit_character(id, |character| {
            if character.name == name {
                return false;
            }
            character.name = name;
            character.touch();
            true
        })
    }

    pub fn set_role(&mut self, id: Id, role: String) -> bool {
        self.edit_character(id, |character| {
            if character.role == role {
                return false;
            }
            character.role = role;
            character.touch();
            true
        })
    }

    pub fn set_summary(&mut self, id: Id, summary: String) -> bool {
        self.edit_character(id, |character| {
            if character.summary == summary {
                return false;
            }
            character.summary = summary;
            character.touch();
            true
        })
    }

    pub fn remove_character(&mut self, id: Id) -> bool {
        let before = self.characters.len();
        self.characters.retain(|character| character.id != id);
        if self.characters.len() == before {
            return false;
        }
        self.motivations.retain(|item| item.character_id != id);
        self.relationships
            .retain(|item| item.from_id != id && item.to_id != id);
        true
    }

    pub fn add_motivation(&mut self, character_id: Id) -> Option<Id> {
        if !self.has_character(character_id) {
            return None;
        }
        let sort_index = self
            .motivations
            .iter()
            .filter(|item| item.character_id == character_id)
            .map(|item| item.sort_index)
            .max()
            .map(|index| index + 1)
            .unwrap_or(0);
        let item = Motivation::new(character_id, sort_index);
        let id = item.id;
        self.motivations.push(item);
        Some(id)
    }

    pub fn set_motivation(&mut self, id: Id, body: String) -> bool {
        let Some(item) = self.motivations.iter_mut().find(|item| item.id == id) else {
            return false;
        };
        if item.body == body {
            return false;
        }
        item.body = body;
        item.touch();
        true
    }

    pub fn remove_motivation(&mut self, id: Id) -> bool {
        let Some(position) = self.motivations.iter().position(|item| item.id == id) else {
            return false;
        };
        let character_id = self.motivations[position].character_id;
        self.motivations.remove(position);
        let order = self.motivation_order(character_id);
        self.write_motivation_order(&order);
        true
    }

    pub fn shift_motivation(&mut self, id: Id, earlier: bool) -> bool {
        let Some(item) = self.motivations.iter().find(|item| item.id == id) else {
            return false;
        };
        let character_id = item.character_id;
        let mut order = self.motivation_order(character_id);
        if !shift_id(&mut order, id, earlier) {
            return false;
        }
        self.write_motivation_order(&order);
        true
    }

    pub fn add_relationship(&mut self, from_id: Id, to_id: Id) -> Option<Id> {
        if from_id == to_id || !self.has_character(from_id) || !self.has_character(to_id) {
            return None;
        }
        let sort_index = self
            .relationships
            .iter()
            .filter(|item| item.from_id == from_id)
            .map(|item| item.sort_index)
            .max()
            .map(|index| index + 1)
            .unwrap_or(0);
        let item = CharacterRelationship::new(from_id, to_id, sort_index);
        let id = item.id;
        self.relationships.push(item);
        Some(id)
    }

    pub fn set_relationship_kind(&mut self, id: Id, kind: String) -> bool {
        let Some(item) = self.relationships.iter_mut().find(|item| item.id == id) else {
            return false;
        };
        if item.kind == kind {
            return false;
        }
        item.kind = kind;
        item.touch();
        true
    }

    pub fn set_relationship_target(&mut self, id: Id, to_id: Id) -> bool {
        if !self.has_character(to_id) {
            return false;
        }
        let Some(item) = self.relationships.iter_mut().find(|item| item.id == id) else {
            return false;
        };
        if item.from_id == to_id || item.to_id == to_id {
            return false;
        }
        item.to_id = to_id;
        item.touch();
        true
    }

    pub fn remove_relationship(&mut self, id: Id) -> bool {
        let before = self.relationships.len();
        self.relationships.retain(|item| item.id != id);
        self.relationships.len() != before
    }

    fn has_character(&self, id: Id) -> bool {
        self.characters.iter().any(|character| character.id == id)
    }

    fn edit_character(&mut self, id: Id, edit: impl FnOnce(&mut Character) -> bool) -> bool {
        let Some(character) = self.characters.iter_mut().find(|character| character.id == id) else {
            return false;
        };
        edit(character)
    }

    fn next_character_index(&self) -> i64 {
        self.characters
            .iter()
            .map(|character| character.sort_index)
            .max()
            .map(|index| index + 1)
            .unwrap_or(0)
    }

    fn motivation_order(&self, character_id: Id) -> Vec<Id> {
        self.motivations_for(character_id)
            .into_iter()
            .map(|item| item.id)
            .collect()
    }

    fn write_motivation_order(&mut self, order: &[Id]) {
        for (index, id) in order.iter().enumerate() {
            if let Some(item) = self.motivations.iter_mut().find(|item| item.id == *id) {
                let sort_index = index as i64;
                if item.sort_index != sort_index {
                    item.sort_index = sort_index;
                    item.touch();
                }
            }
        }
    }
}

fn shift_id(order: &mut [Id], id: Id, earlier: bool) -> bool {
    let Some(index) = order.iter().position(|item| *item == id) else {
        return false;
    };
    let neighbor = if earlier {
        index.checked_sub(1)
    } else if index + 1 < order.len() {
        Some(index + 1)
    } else {
        None
    };
    let Some(neighbor) = neighbor else {
        return false;
    };
    order.swap(index, neighbor);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_keeps_motivations_and_a_relationship() {
        let mut cast = Cast::new();
        let hero = cast.add_character();
        let rival = cast.add_character();
        assert_eq!(cast.characters[0].name, "Character 1");
        assert!(cast.set_name(hero, "Ada".into()));
        assert!(cast.set_role(hero, "Protagonist".into()));
        assert!(cast.set_summary(hero, "Keeps the door locked.".into()));

        let first = cast.add_motivation(hero).unwrap();
        let second = cast.add_motivation(hero).unwrap();
        assert!(cast.set_motivation(first, "Protect the house".into()));
        assert!(cast.set_motivation(second, "Learn who left the note".into()));
        assert!(cast.shift_motivation(second, true));
        let motives = cast.motivations_for(hero);
        assert_eq!(motives[0].id, second);
        assert_eq!(motives[1].body, "Protect the house");

        let link = cast.add_relationship(hero, rival).unwrap();
        assert!(cast.set_relationship_kind(link, "rival".into()));
        assert_eq!(cast.relationships_from(hero)[0].to_id, rival);
        assert!(cast.add_relationship(hero, hero).is_none());
    }

    #[test]
    fn removing_a_character_drops_their_motivations_and_links() {
        let mut cast = Cast::new();
        let hero = cast.add_character();
        let other = cast.add_character();
        cast.add_motivation(hero).unwrap();
        cast.add_relationship(hero, other).unwrap();
        cast.add_relationship(other, hero).unwrap();
        assert!(cast.remove_character(hero));
        assert!(cast.motivations.is_empty());
        assert!(cast.relationships.is_empty());
        assert_eq!(cast.characters.len(), 1);
    }
}
