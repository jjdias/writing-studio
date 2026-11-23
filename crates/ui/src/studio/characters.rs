use super::*;

#[derive(Clone, PartialEq)]
pub(super) struct CastListItem {
    pub(super) id: Id,
    pub(super) name: String,
}

#[derive(Clone, PartialEq)]
pub(super) struct MotiveView {
    pub(super) id: Id,
    pub(super) body: String,
}

#[derive(Clone, PartialEq)]
pub(super) struct ProfileView {
    pub(super) id: Id,
    pub(super) name: String,
    pub(super) role: String,
    pub(super) summary: String,
    pub(super) motivations: Vec<MotiveView>,
}

#[component]
pub(super) fn CharactersPane() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let snapshot = state.read().clone();
    let people: Vec<CastListItem> = snapshot
        .cast
        .characters_in_order()
        .into_iter()
        .map(|character| CastListItem {
            id: character.id,
            name: character.name.clone(),
        })
        .collect();
    let shown = snapshot.selected_character.filter(|id| people.iter().any(|person| person.id == *id))
        .or_else(|| people.first().map(|person| person.id));
    let profile = shown.and_then(|id| profile_view(&snapshot.cast, id));
    let others: Vec<CastListItem> = people
        .iter()
        .filter(|person| Some(person.id) != shown)
        .cloned()
        .collect();
    let confirm = snapshot.confirm_character;
    rsx! {
        div { class: "cast-pane",
            div { class: "cast-profile",
                if let Some(profile) = profile {
                    CharacterProfile { profile, others, confirm }
                } else {
                    EmptyState {
                        mode: WorkspaceMode::Characters,
                        title: "No characters yet",
                        body: "Each character has a profile and motivations; Context beside it holds their connections and the scenes they appear in. The cast stays in this project.",
                        action: "Add character",
                        onclick: move |_| state.write().add_character(),
                    }
                }
            }
        }
    }
}

pub(super) fn profile_view(cast: &writing_studio_characters::Cast, id: Id) -> Option<ProfileView> {
    let character = cast.characters.iter().find(|character| character.id == id)?;
    Some(ProfileView {
        id: character.id,
        name: character.name.clone(),
        role: character.role.clone(),
        summary: character.summary.clone(),
        motivations: cast
            .motivations_for(id)
            .into_iter()
            .map(|item| MotiveView {
                id: item.id,
                body: item.body.clone(),
            })
            .collect(),
    })
}

#[component]
pub(super) fn CharacterProfile(profile: ProfileView, others: Vec<CastListItem>, confirm: Option<Id>) -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let id = profile.id;
    let name = profile.name.clone();
    let role = profile.role.clone();
    let summary = profile.summary.clone();
    let motivations = profile.motivations.clone();
    let delete_label = confirm_label(confirm == Some(id), "Delete", &name);
    rsx! {
        label { "Name" }
        input {
            title: "Character name",
            value: "{name}",
            oninput: move |event| state.write().set_character_name(id, event.value()),
        }
        label { "Role" }
        input {
            title: "Role in the story",
            placeholder: "Protagonist, rival, mentor…",
            value: "{role}",
            oninput: move |event| state.write().set_character_role(id, event.value()),
        }
        label { "Profile" }
        textarea {
            title: "Who they are",
            value: "{summary}",
            oninput: move |event| state.write().set_character_summary(id, event.value()),
        }
        section { class: "cast-block",
            h3 { "Motivations" }
            p { class: "hint", "What they want, fear, or need. Order is the order you keep them in." }
            button {
                onclick: move |_| state.write().add_motivation(id),
                "Add motivation"
            }
            if motivations.is_empty() {
                p { class: "hint", "No motivations yet." }
            }
            for motive in motivations {
                {
                    let motive_id = motive.id;
                    let body = motive.body.clone();
                    rsx! {
                        div { class: "cast-row", key: "{motive_id}",
                            input {
                                title: "Motivation",
                                value: "{body}",
                                oninput: move |event| state.write().set_motivation_body(motive_id, event.value()),
                            }
                            button {
                                title: "Move earlier",
                                onclick: move |_| state.write().shift_motivation(motive_id, true),
                                "Earlier"
                            }
                            button {
                                title: "Move later",
                                onclick: move |_| state.write().shift_motivation(motive_id, false),
                                "Later"
                            }
                            button {
                                onclick: move |_| state.write().remove_motivation(motive_id),
                                "Remove"
                            }
                        }
                    }
                }
            }
        }
        div { class: "cast-actions",
            button {
                class: danger(confirm == Some(id)),
                onclick: move |_| state.write().remove_character(id),
                "{delete_label}"
            }
        }
    }
}
