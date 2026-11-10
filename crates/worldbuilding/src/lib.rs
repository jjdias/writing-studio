//! Locations, cultures, lore, and a timeline.
//!
//! The world is local project data. A location may name a culture. Lore and
//! timeline events may name a location. Those links are references inside this
//! project, not copies of manuscript text.

use serde::{Deserialize, Serialize};
use writing_studio_core::{Id, Timestamp};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Location {
    pub id: Id,
    pub name: String,
    pub summary: String,
    pub culture_id: Option<Id>,
    pub sort_index: i64,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl Location {
    fn new(name: impl Into<String>, sort_index: i64) -> Self {
        let now = Timestamp::now();
        Self {
            id: Id::new(),
            name: name.into(),
            summary: String::new(),
            culture_id: None,
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
pub struct Culture {
    pub id: Id,
    pub name: String,
    pub summary: String,
    pub sort_index: i64,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl Culture {
    fn new(name: impl Into<String>, sort_index: i64) -> Self {
        let now = Timestamp::now();
        Self {
            id: Id::new(),
            name: name.into(),
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
pub struct LoreEntry {
    pub id: Id,
    pub title: String,
    pub body: String,
    pub location_id: Option<Id>,
    pub culture_id: Option<Id>,
    pub sort_index: i64,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl LoreEntry {
    fn new(title: impl Into<String>, sort_index: i64) -> Self {
        let now = Timestamp::now();
        Self {
            id: Id::new(),
            title: title.into(),
            body: String::new(),
            location_id: None,
            culture_id: None,
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
pub struct TimelineEvent {
    pub id: Id,
    pub title: String,
    pub when_label: String,
    pub summary: String,
    pub location_id: Option<Id>,
    pub sort_index: i64,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl TimelineEvent {
    fn new(title: impl Into<String>, sort_index: i64) -> Self {
        let now = Timestamp::now();
        Self {
            id: Id::new(),
            title: title.into(),
            when_label: String::new(),
            summary: String::new(),
            location_id: None,
            sort_index,
            created_at: now,
            updated_at: now,
        }
    }

    fn touch(&mut self) {
        self.updated_at = Timestamp::now();
    }
}

/// Locations, cultures, lore, and timeline events for one project.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct World {
    pub locations: Vec<Location>,
    pub cultures: Vec<Culture>,
    pub lore: Vec<LoreEntry>,
    pub events: Vec<TimelineEvent>,
}

impl World {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn locations_in_order(&self) -> Vec<&Location> {
        sorted(&self.locations, |item| (item.sort_index, item.id))
    }

    pub fn cultures_in_order(&self) -> Vec<&Culture> {
        sorted(&self.cultures, |item| (item.sort_index, item.id))
    }

    pub fn lore_in_order(&self) -> Vec<&LoreEntry> {
        sorted(&self.lore, |item| (item.sort_index, item.id))
    }

    pub fn events_in_order(&self) -> Vec<&TimelineEvent> {
        sorted(&self.events, |item| (item.sort_index, item.id))
    }

    pub fn add_location(&mut self) -> Id {
        let index = next_index(self.locations.iter().map(|item| item.sort_index));
        let place = Location::new(format!("Location {}", self.locations.len() + 1), index);
        let id = place.id;
        self.locations.push(place);
        id
    }

    pub fn set_location_name(&mut self, id: Id, name: String) -> bool {
        edit(&mut self.locations, id, |item| item.id, |item| {
            if item.name == name {
                return false;
            }
            item.name = name;
            item.touch();
            true
        })
    }

    pub fn set_location_summary(&mut self, id: Id, summary: String) -> bool {
        edit(&mut self.locations, id, |item| item.id, |item| {
            if item.summary == summary {
                return false;
            }
            item.summary = summary;
            item.touch();
            true
        })
    }

    pub fn set_location_culture(&mut self, id: Id, culture_id: Option<Id>) -> bool {
        if culture_id.is_some_and(|culture| !self.cultures.iter().any(|item| item.id == culture)) {
            return false;
        }
        edit(&mut self.locations, id, |item| item.id, |item| {
            if item.culture_id == culture_id {
                return false;
            }
            item.culture_id = culture_id;
            item.touch();
            true
        })
    }

    pub fn remove_location(&mut self, id: Id) -> bool {
        let before = self.locations.len();
        self.locations.retain(|item| item.id != id);
        if self.locations.len() == before {
            return false;
        }
        for entry in &mut self.lore {
            if entry.location_id == Some(id) {
                entry.location_id = None;
                entry.touch();
            }
        }
        for event in &mut self.events {
            if event.location_id == Some(id) {
                event.location_id = None;
                event.touch();
            }
        }
        true
    }

    pub fn add_culture(&mut self) -> Id {
        let index = next_index(self.cultures.iter().map(|item| item.sort_index));
        let culture = Culture::new(format!("Culture {}", self.cultures.len() + 1), index);
        let id = culture.id;
        self.cultures.push(culture);
        id
    }

    pub fn set_culture_name(&mut self, id: Id, name: String) -> bool {
        edit(&mut self.cultures, id, |item| item.id, |item| {
            if item.name == name {
                return false;
            }
            item.name = name;
            item.touch();
            true
        })
    }

    pub fn set_culture_summary(&mut self, id: Id, summary: String) -> bool {
        edit(&mut self.cultures, id, |item| item.id, |item| {
            if item.summary == summary {
                return false;
            }
            item.summary = summary;
            item.touch();
            true
        })
    }

    pub fn remove_culture(&mut self, id: Id) -> bool {
        let before = self.cultures.len();
        self.cultures.retain(|item| item.id != id);
        if self.cultures.len() == before {
            return false;
        }
        for place in &mut self.locations {
            if place.culture_id == Some(id) {
                place.culture_id = None;
                place.touch();
            }
        }
        for entry in &mut self.lore {
            if entry.culture_id == Some(id) {
                entry.culture_id = None;
                entry.touch();
            }
        }
        true
    }

    pub fn add_lore(&mut self) -> Id {
        let index = next_index(self.lore.iter().map(|item| item.sort_index));
        let entry = LoreEntry::new(format!("Lore {}", self.lore.len() + 1), index);
        let id = entry.id;
        self.lore.push(entry);
        id
    }

    pub fn set_lore_title(&mut self, id: Id, title: String) -> bool {
        edit(&mut self.lore, id, |item| item.id, |item| {
            if item.title == title {
                return false;
            }
            item.title = title;
            item.touch();
            true
        })
    }

    pub fn set_lore_body(&mut self, id: Id, body: String) -> bool {
        edit(&mut self.lore, id, |item| item.id, |item| {
            if item.body == body {
                return false;
            }
            item.body = body;
            item.touch();
            true
        })
    }

    pub fn set_lore_location(&mut self, id: Id, location_id: Option<Id>) -> bool {
        if location_id.is_some_and(|place| !self.locations.iter().any(|item| item.id == place)) {
            return false;
        }
        edit(&mut self.lore, id, |item| item.id, |item| {
            if item.location_id == location_id {
                return false;
            }
            item.location_id = location_id;
            item.touch();
            true
        })
    }

    pub fn set_lore_culture(&mut self, id: Id, culture_id: Option<Id>) -> bool {
        if culture_id.is_some_and(|culture| !self.cultures.iter().any(|item| item.id == culture)) {
            return false;
        }
        edit(&mut self.lore, id, |item| item.id, |item| {
            if item.culture_id == culture_id {
                return false;
            }
            item.culture_id = culture_id;
            item.touch();
            true
        })
    }

    pub fn remove_lore(&mut self, id: Id) -> bool {
        let before = self.lore.len();
        self.lore.retain(|item| item.id != id);
        self.lore.len() != before
    }

    pub fn add_event(&mut self) -> Id {
        let index = next_index(self.events.iter().map(|item| item.sort_index));
        let event = TimelineEvent::new(format!("Event {}", self.events.len() + 1), index);
        let id = event.id;
        self.events.push(event);
        id
    }

    pub fn set_event_title(&mut self, id: Id, title: String) -> bool {
        edit(&mut self.events, id, |item| item.id, |item| {
            if item.title == title {
                return false;
            }
            item.title = title;
            item.touch();
            true
        })
    }

    pub fn set_event_when(&mut self, id: Id, when_label: String) -> bool {
        edit(&mut self.events, id, |item| item.id, |item| {
            if item.when_label == when_label {
                return false;
            }
            item.when_label = when_label;
            item.touch();
            true
        })
    }

    pub fn set_event_summary(&mut self, id: Id, summary: String) -> bool {
        edit(&mut self.events, id, |item| item.id, |item| {
            if item.summary == summary {
                return false;
            }
            item.summary = summary;
            item.touch();
            true
        })
    }

    pub fn set_event_location(&mut self, id: Id, location_id: Option<Id>) -> bool {
        if location_id.is_some_and(|place| !self.locations.iter().any(|item| item.id == place)) {
            return false;
        }
        edit(&mut self.events, id, |item| item.id, |item| {
            if item.location_id == location_id {
                return false;
            }
            item.location_id = location_id;
            item.touch();
            true
        })
    }

    pub fn shift_event(&mut self, id: Id, earlier: bool) -> bool {
        let mut order: Vec<Id> = self.events_in_order().into_iter().map(|event| event.id).collect();
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
        for (index, id) in order.into_iter().enumerate() {
            if let Some(event) = self.events.iter_mut().find(|event| event.id == id) {
                let sort_index = index as i64;
                if event.sort_index != sort_index {
                    event.sort_index = sort_index;
                    event.touch();
                }
            }
        }
        true
    }

    pub fn remove_event(&mut self, id: Id) -> bool {
        let before = self.events.len();
        self.events.retain(|item| item.id != id);
        self.events.len() != before
    }
}

fn sorted<T>(items: &[T], key: impl Fn(&T) -> (i64, Id)) -> Vec<&T> {
    let mut items: Vec<&T> = items.iter().collect();
    items.sort_by_key(|item| key(item));
    items
}

fn next_index(indexes: impl Iterator<Item = i64>) -> i64 {
    indexes.max().map(|index| index + 1).unwrap_or(0)
}

fn edit<T>(
    items: &mut [T],
    id: Id,
    id_of: impl Fn(&T) -> Id,
    change: impl FnOnce(&mut T) -> bool,
) -> bool {
    let Some(item) = items.iter_mut().find(|item| id_of(item) == id) else {
        return false;
    };
    change(item)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn places_cultures_lore_and_events_link_together() {
        let mut world = World::new();
        let culture = world.add_culture();
        world.set_culture_name(culture, "River folk".into());
        let place = world.add_location();
        world.set_location_name(place, "Mill town".into());
        assert!(world.set_location_culture(place, Some(culture)));
        let note = world.add_lore();
        world.set_lore_title(note, "The flood mark".into());
        world.set_lore_body(note, "A line cut into the mill wall.".into());
        assert!(world.set_lore_location(note, Some(place)));
        assert!(world.set_lore_culture(note, Some(culture)));
        let _first = world.add_event();
        let second = world.add_event();
        world.set_event_when(second, "Year 12".into());
        assert!(world.set_event_location(second, Some(place)));
        assert!(world.shift_event(second, true));
        assert_eq!(world.events_in_order()[0].id, second);
        assert!(!world.set_location_culture(place, Some(Id::new())));
    }

    #[test]
    fn removing_a_place_or_culture_clears_links() {
        let mut world = World::new();
        let culture = world.add_culture();
        let place = world.add_location();
        world.set_location_culture(place, Some(culture));
        let note = world.add_lore();
        world.set_lore_location(note, Some(place));
        world.set_lore_culture(note, Some(culture));
        let event = world.add_event();
        world.set_event_location(event, Some(place));
        assert!(world.remove_culture(culture));
        assert_eq!(world.locations[0].culture_id, None);
        assert_eq!(world.lore[0].culture_id, None);
        assert!(world.remove_location(place));
        assert_eq!(world.lore[0].location_id, None);
        assert_eq!(world.events[0].location_id, None);
    }
}
