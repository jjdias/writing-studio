use super::*;

#[component]
pub(super) fn CorkboardPane() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let snapshot = state.read().clone();
    let query = snapshot.cork_query.clone();
    let group = snapshot.cork_group;
    let scope = snapshot.series_scope();
    let cards: Vec<_> = cards_from_binder(&snapshot.binder, |id| words_for_document(&snapshot, id))
        .into_iter()
        .filter(|card| {
            scope
                .map(|book| snapshot.binder.book_of(card.node_id) == Some(book))
                .unwrap_or(true)
        })
        .collect();
    let has_scenes = !cards.is_empty();
    let cards: Vec<_> = cards
        .into_iter()
        .filter(|card| card_matches(card, &query) && snapshot.keyword_allows(card.node_id))
        .collect();
    let sections = group_cards(&cards, group);
    rsx! {
        div { class: "cork-pane",
            div { class: "cork-tools",
                SeriesScopeSelect {}
                KeywordSelect {}
                input {
                    id: "cork-filter",
                    class: "cork-filter",
                    placeholder: "Filter scenes…",
                    title: "Ctrl+F",
                    value: "{query}",
                    oninput: move |event| state.write().cork_query = event.value(),
                }
                label { class: "inline-label", r#for: "cork-group", "Group by" }
                select {
                    id: "cork-group",
                    title: "How the cards are grouped",
                    onchange: move |event| {
                        if let Some(next) = CorkGroup::ALL.into_iter().find(|item| item.label() == event.value()) {
                            state.write().cork_group = next;
                        }
                    },
                    for candidate in CorkGroup::ALL {
                        option { value: "{candidate.label()}", selected: group == candidate, "{candidate.label()}" }
                    }
                }
            }
            if has_scenes {
                p { class: "hint", "Drag a card onto another scene to place it just after that scene. Colors come from the binder." }
            }
            if !has_scenes {
                EmptyState {
                    mode: WorkspaceMode::Corkboard,
                    title: "No scenes yet",
                    body: "Each scene in the manuscript is a card here, with its synopsis, status, and word count.",
                    action: "Add a scene",
                    onclick: move |_| {
                        state.write().add_node(NodeKind::Scene);
                    },
                }
            } else if sections.is_empty() {
                p { class: "hint", "No scenes match this filter." }
            }
            for section in sections {
                section { class: "cork-section",
                    h3 { "{section.title}" }
                    div { class: "corkboard",
                        for card in section.cards {
                            CorkCardView { card }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub(super) fn CorkCardView(card: writing_studio_corkboard::CorkCard) -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let id = card.node_id;
    let hex = binder_color_hex(&card.color);
    let dragging_id = state.read().dragging;
    let dragging = dragging_id == Some(id);
    let drop_target = state.read().drop_target == Some(id);
    let drop_ok = dragging_id.is_some_and(|dragged| state.read().binder.drop_allowed(dragged, id));
    let mut class = String::from("card");
    if dragging {
        class.push_str(" dragging");
    }
    if drop_target && drop_ok {
        class.push_str(" drop-target");
    }
    if drop_target && !drop_ok {
        class.push_str(" drop-invalid");
    }
    // The binder color is the card's band. A card with no color keeps a
    // plain band, so every card has the same shape.
    let keywords = card.keywords.join(", ");
    let stripe = if hex.is_empty() {
        String::new()
    } else {
        format!("--band: {hex}")
    };
    rsx! {
        button {
            class: "{class}",
            style: "{stripe}",
            draggable: true,
            onclick: move |_| {
                let suppress = state.write().ignore_next_click;
                state.write().ignore_next_click = false;
                if suppress {
                    return;
                }
                state.write().select_node(id);
                state.write().mode = WorkspaceMode::Writing;
            },
            oncontextmenu: move |event: MouseEvent| {
                event.prevent_default();
                let point = event.client_coordinates();
                state.write().open_item_menu(id, point.x, point.y);
            },
            ondragstart: move |event| {
                event.stop_propagation();
                state.write().begin_drag(id);
                state.write().ignore_next_click = true;
            },
            ondragover: move |event| {
                event.prevent_default();
                event.stop_propagation();
                state.write().set_drop_target(id);
            },
            ondrop: move |event| {
                event.prevent_default();
                event.stop_propagation();
                state.write().drop_on(id);
            },
            ondragend: move |_| state.write().end_drag(),
            h3 { class: "card-title", "{card.title}" }
            p { class: if card.synopsis.is_empty() { "card-synopsis empty" } else { "card-synopsis" },
                if card.synopsis.is_empty() {
                    "No synopsis yet."
                } else {
                    "{card.synopsis}"
                }
            }
            span { class: "card-meta",
                if card.status.is_empty() {
                    "{card.word_count} words"
                } else {
                    "{status_name(&card.status)} · {card.word_count} words"
                }
            }
            if !card.keywords.is_empty() {
                span { class: "card-label", "{keywords}" }
            }
        }
    }
}
