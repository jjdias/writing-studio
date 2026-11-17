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
        .filter(|card| card_matches(card, &query))
        .collect();
    let sections = group_cards(&cards, group);
    rsx! {
        div { class: "cork-pane",
            div { class: "cork-tools",
                SeriesScopeSelect {}
                input {
                    id: "cork-filter",
                    class: "cork-filter",
                    placeholder: "Filter scenes…",
                    title: "Ctrl+F",
                    value: "{query}",
                    oninput: move |event| state.write().cork_query = event.value(),
                }
                for candidate in CorkGroup::ALL {
                    button {
                        class: if group == candidate { "mode active" } else { "mode" },
                        onclick: move |_| state.write().cork_group = candidate,
                        "{candidate.label()}"
                    }
                }
            }
            p { class: "hint", "Drag a card onto another scene to place it just after that scene. Colors come from the binder." }
            if sections.is_empty() {
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
    let stripe = if hex.is_empty() {
        String::new()
    } else {
        format!("border-top: 6px solid {hex}")
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
            h3 { "{card.title}" }
            p {
                if card.synopsis.is_empty() {
                    "No synopsis yet."
                } else {
                    "{card.synopsis}"
                }
            }
            span { class: "card-meta", "{card.status} · {card.word_count} words" }
            if !card.label.is_empty() {
                span { class: "card-label", "{card.label}" }
            }
        }
    }
}
