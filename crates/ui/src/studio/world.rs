use super::*;

#[derive(Clone, PartialEq)]
pub(super) enum WorldForm {
    Location {
        id: Id,
        name: String,
        summary: String,
        culture_id: String,
    },
    Culture {
        id: Id,
        name: String,
        summary: String,
    },
    Lore {
        id: Id,
        title: String,
        body: String,
        location_id: String,
        culture_id: String,
    },
    Event {
        id: Id,
        title: String,
        when_label: String,
        summary: String,
        location_id: String,
    },
}

#[component]
pub(super) fn WorldPane() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let snapshot = state.read().clone();
    let section = snapshot.world_section;
    let items = world_items(&snapshot.world, section);
    let shown = snapshot
        .selected_world
        .filter(|id| items.iter().any(|item| item.id == *id))
        .or_else(|| items.first().map(|item| item.id));
    let form = shown.and_then(|id| world_form(&snapshot.world, section, id));
    let places = named_places(&snapshot.world);
    let cultures = named_cultures(&snapshot.world);
    let confirm = snapshot.confirm_world;
    let add_label = section.add_label();
    rsx! {
        div { class: "cast-pane",
            aside { class: "cast-list",
                for candidate in WorldSection::ALL {
                    button {
                        class: if section == candidate { "mode active" } else { "mode" },
                        onclick: move |_| state.write().set_world_section(candidate),
                        "{candidate.label()}"
                    }
                }
                button {
                    class: "primary",
                    onclick: move |_| state.write().add_world_item(),
                    "{add_label}"
                }
                if items.is_empty() {
                    p { class: "hint", "Nothing in this section yet." }
                }
                for item in items {
                    {
                        let id = item.id;
                        let active = shown == Some(id);
                        let class = if active { "cast-name active" } else { "cast-name" };
                        rsx! {
                            button {
                                class: "{class}",
                                key: "{id}",
                                onclick: move |_| state.write().select_world_item(id),
                                "{item.name}"
                            }
                        }
                    }
                }
            }
            div { class: "cast-profile",
                if let Some(form) = form {
                    WorldEditor { form, places, cultures, confirm }
                } else {
                    p { class: "hint", "Locations, cultures, lore, and the timeline stay in this project. Pick a section and add an entry." }
                }
            }
        }
    }
}

pub(super) fn world_items(world: &writing_studio_worldbuilding::World, section: WorldSection) -> Vec<NamedId> {
    match section {
        WorldSection::Locations => world
            .locations_in_order()
            .into_iter()
            .map(|item| NamedId { id: item.id, name: item.name.clone() })
            .collect(),
        WorldSection::Cultures => world
            .cultures_in_order()
            .into_iter()
            .map(|item| NamedId { id: item.id, name: item.name.clone() })
            .collect(),
        WorldSection::Lore => world
            .lore_in_order()
            .into_iter()
            .map(|item| NamedId {
                id: item.id,
                name: item.title.clone(),
            })
            .collect(),
        WorldSection::Timeline => world
            .events_in_order()
            .into_iter()
            .map(|item| {
                let name = if item.when_label.is_empty() {
                    item.title.clone()
                } else {
                    format!("{} · {}", item.when_label, item.title)
                };
                NamedId { id: item.id, name }
            })
            .collect(),
    }
}

pub(super) fn world_form(world: &writing_studio_worldbuilding::World, section: WorldSection, id: Id) -> Option<WorldForm> {
    match section {
        WorldSection::Locations => {
            let place = world.locations.iter().find(|item| item.id == id)?;
            Some(WorldForm::Location {
                id: place.id,
                name: place.name.clone(),
                summary: place.summary.clone(),
                culture_id: optional_text(place.culture_id),
            })
        }
        WorldSection::Cultures => {
            let culture = world.cultures.iter().find(|item| item.id == id)?;
            Some(WorldForm::Culture {
                id: culture.id,
                name: culture.name.clone(),
                summary: culture.summary.clone(),
            })
        }
        WorldSection::Lore => {
            let entry = world.lore.iter().find(|item| item.id == id)?;
            Some(WorldForm::Lore {
                id: entry.id,
                title: entry.title.clone(),
                body: entry.body.clone(),
                location_id: optional_text(entry.location_id),
                culture_id: optional_text(entry.culture_id),
            })
        }
        WorldSection::Timeline => {
            let event = world.events.iter().find(|item| item.id == id)?;
            Some(WorldForm::Event {
                id: event.id,
                title: event.title.clone(),
                when_label: event.when_label.clone(),
                summary: event.summary.clone(),
                location_id: optional_text(event.location_id),
            })
        }
    }
}

pub(super) fn named_places(world: &writing_studio_worldbuilding::World) -> Vec<NamedId> {
    world
        .locations_in_order()
        .into_iter()
        .map(|item| NamedId { id: item.id, name: item.name.clone() })
        .collect()
}

pub(super) fn named_cultures(world: &writing_studio_worldbuilding::World) -> Vec<NamedId> {
    world
        .cultures_in_order()
        .into_iter()
        .map(|item| NamedId { id: item.id, name: item.name.clone() })
        .collect()
}

pub(super) fn optional_text(id: Option<Id>) -> String {
    id.map(|id| id.to_string()).unwrap_or_default()
}

pub(super) fn parse_optional_id(value: &str) -> Option<Id> {
    let value = value.trim();
    if value.is_empty() { None } else { value.parse().ok() }
}

#[component]
pub(super) fn WorldEditor(form: WorldForm, places: Vec<NamedId>, cultures: Vec<NamedId>, confirm: Option<Id>) -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let id = match &form {
        WorldForm::Location { id, .. }
        | WorldForm::Culture { id, .. }
        | WorldForm::Lore { id, .. }
        | WorldForm::Event { id, .. } => *id,
    };
    let delete_label = if confirm == Some(id) { "Delete entry" } else { "Delete" };
    let link = match &form {
        WorldForm::Location { .. } => BibleLink::Location,
        WorldForm::Culture { .. } => BibleLink::Culture,
        WorldForm::Lore { .. } => BibleLink::Lore,
        WorldForm::Event { .. } => BibleLink::Timeline,
    };
    rsx! {
        match form {
            WorldForm::Location { id, name, summary, culture_id } => rsx! {
                label { "Name" }
                input {
                    value: "{name}",
                    oninput: move |event| state.write().set_location_name(id, event.value()),
                }
                label { "Description" }
                textarea {
                    value: "{summary}",
                    oninput: move |event| state.write().set_location_summary(id, event.value()),
                }
                label { "Culture" }
                select {
                    value: "{culture_id}",
                    onchange: move |event| state.write().set_location_culture(id, parse_optional_id(&event.value())),
                    option { value: "", "No culture" }
                    for culture in cultures {
                        option { value: "{culture.id}", "{culture.name}" }
                    }
                }
            },
            WorldForm::Culture { id, name, summary } => rsx! {
                label { "Name" }
                input {
                    value: "{name}",
                    oninput: move |event| state.write().set_culture_name(id, event.value()),
                }
                label { "Description" }
                textarea {
                    value: "{summary}",
                    oninput: move |event| state.write().set_culture_summary(id, event.value()),
                }
            },
            WorldForm::Lore { id, title, body, location_id, culture_id } => rsx! {
                label { "Title" }
                input {
                    value: "{title}",
                    oninput: move |event| state.write().set_lore_title(id, event.value()),
                }
                label { "Lore" }
                textarea {
                    value: "{body}",
                    oninput: move |event| state.write().set_lore_body(id, event.value()),
                }
                label { "Location" }
                select {
                    value: "{location_id}",
                    onchange: move |event| state.write().set_lore_location(id, parse_optional_id(&event.value())),
                    option { value: "", "No location" }
                    for place in places.clone() {
                        option { value: "{place.id}", "{place.name}" }
                    }
                }
                label { "Culture" }
                select {
                    value: "{culture_id}",
                    onchange: move |event| state.write().set_lore_culture(id, parse_optional_id(&event.value())),
                    option { value: "", "No culture" }
                    for culture in cultures.clone() {
                        option { value: "{culture.id}", "{culture.name}" }
                    }
                }
            },
            WorldForm::Event { id, title, when_label, summary, location_id } => rsx! {
                label { "When" }
                input {
                    title: "A label such as Year 12 or the spring before the war",
                    value: "{when_label}",
                    oninput: move |event| state.write().set_event_when(id, event.value()),
                }
                label { "Title" }
                input {
                    value: "{title}",
                    oninput: move |event| state.write().set_event_title(id, event.value()),
                }
                label { "What happened" }
                textarea {
                    value: "{summary}",
                    oninput: move |event| state.write().set_event_summary(id, event.value()),
                }
                label { "Location" }
                select {
                    value: "{location_id}",
                    onchange: move |event| state.write().set_event_location(id, parse_optional_id(&event.value())),
                    option { value: "", "No location" }
                    for place in places.clone() {
                        option { value: "{place.id}", "{place.name}" }
                    }
                }
                div { class: "cast-actions",
                    button {
                        onclick: move |_| state.write().shift_world_event(id, true),
                        "Earlier"
                    }
                    button {
                        onclick: move |_| state.write().shift_world_event(id, false),
                        "Later"
                    }
                }
            },
        }
        SceneLinks { owner: id, kind: link }
        div { class: "cast-actions",
            button {
                onclick: move |_| state.write().remove_world_item(id),
                "{delete_label}"
            }
        }
    }
}
