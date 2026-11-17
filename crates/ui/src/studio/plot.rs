use super::*;

#[derive(Clone, PartialEq)]
pub(super) struct PlotPointView {
    pub(super) id: Id,
    pub(super) title: String,
    pub(super) summary: String,
    pub(super) scene_id: Option<Id>,
}

#[derive(Clone, PartialEq)]
pub(super) struct PlotLineView {
    pub(super) id: Id,
    pub(super) title: String,
    pub(super) color: String,
    pub(super) points: Vec<PlotPointView>,
}

#[component]
pub(super) fn PlotPane() -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let snapshot = state.read().clone();
    let scenes = scene_choices(&snapshot.binder);
    let confirm = snapshot.confirm_plotline;
    let lines: Vec<PlotLineView> = snapshot
        .plot
        .lines_in_order()
        .into_iter()
        .map(|line| PlotLineView {
            id: line.id,
            title: line.title.clone(),
            color: line.color.clone(),
            points: snapshot
                .plot
                .points_on(line.id)
                .into_iter()
                .map(|point| PlotPointView {
                    id: point.id,
                    title: point.title.clone(),
                    summary: point.summary.clone(),
                    scene_id: point.scene_id,
                })
                .collect(),
        })
        .collect();
    let shown = snapshot
        .selected_plotline
        .filter(|id| lines.iter().any(|line| line.id == *id))
        .or_else(|| lines.first().map(|line| line.id));
    let list: Vec<(Id, String, String)> = lines
        .iter()
        .map(|line| (line.id, line.title.clone(), binder_color_hex(&line.color).to_string()))
        .collect();
    let open = lines.into_iter().find(|line| Some(line.id) == shown);
    rsx! {
        div { class: "cast-pane",
            aside { class: "cast-list", aria_label: "Plotlines",
                button {
                    class: "primary",
                    onclick: move |_| state.write().add_plotline(),
                    "Add plotline"
                }
                for (id, title, hex) in list {
                    button {
                        key: "{id}",
                        class: if shown == Some(id) { "cast-name active" } else { "cast-name" },
                        onclick: move |_| state.write().selected_plotline = Some(id),
                        if !hex.is_empty() {
                            span { class: "swatch", style: "background: {hex}" }
                        }
                        if title.trim().is_empty() { "Untitled plotline" } else { "{title}" }
                    }
                }
            }
            div { class: "cast-profile plot-pane",
                if let Some(line) = open {
                    p { class: "hint", "Plot points stay in order on this line and can open a manuscript scene." }
                    PlotLineRow { line, scenes: scenes.clone(), confirm, binder_scenes: linked_scene_labels(&snapshot, &scenes) }
                } else {
                    EmptyState {
                        mode: WorkspaceMode::Plot,
                        title: "No plotlines yet",
                        body: "A plotline is one thread through the book. Its plot points stay in order and can open the scenes they happen in.",
                        action: "Add plotline",
                        onclick: move |_| state.write().add_plotline(),
                    }
                }
            }
        }
    }
}

#[derive(Clone, PartialEq)]
pub(super) struct LinkedScene {
    pub(super) id: Id,
    pub(super) label: String,
}

pub(super) fn linked_scene_labels(snapshot: &StudioState, scenes: &[(Id, String)]) -> Vec<LinkedScene> {
    snapshot
        .plot
        .points
        .iter()
        .filter_map(|point| point.scene_id)
        .filter(|id| !scenes.iter().any(|(scene, _)| scene == id))
        .map(|id| LinkedScene {
            id,
            label: scene_caption(&snapshot.binder, id),
        })
        .collect()
}

#[component]
pub(super) fn PlotLineRow(line: PlotLineView, scenes: Vec<(Id, String)>, confirm: Option<Id>, binder_scenes: Vec<LinkedScene>) -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let line_id = line.id;
    let color = line.color.clone();
    let title = line.title.clone();
    let stripe = binder_color_hex(&color);
    let border = if stripe.is_empty() {
        String::new()
    } else {
        format!("border-top-color: {stripe}")
    };
    let delete_label = confirm_label(confirm == Some(line_id), "Delete", &title);
    let points = line.points.clone();
    rsx! {
        section { class: "plot-row", key: "{line_id}",
            div { class: "plot-row-head",
                input {
                    class: "plot-line-title",
                    title: "Plotline title",
                    value: "{title}",
                    oninput: move |event| {
                        state.write().set_plotline_title(line_id, event.value());
                    },
                }
                select {
                    class: "plot-color",
                    title: "Plotline color",
                    value: "{color}",
                    onchange: move |event| {
                        state.write().set_plotline_color(line_id, event.value());
                    },
                    for swatch in BINDER_COLORS {
                        option { value: "{swatch.id}", selected: color == swatch.id, "{swatch.name}" }
                    }
                }
                button {
                    onclick: move |_| state.write().add_plot_point(line_id),
                    "Add point"
                }
            }
            div { class: "plot-points",
                if points.is_empty() {
                    p { class: "hint", "No plot points on this line." }
                }
                for point in points {
                    PlotPointCard { point, scenes: scenes.clone(), border: border.clone(), extras: binder_scenes.clone() }
                }
            }
            div { class: "cast-actions",
                button {
                    class: danger(confirm == Some(line_id)),
                    onclick: move |_| state.write().remove_plotline(line_id),
                    "{delete_label}"
                }
            }
        }
    }
}

#[component]
pub(super) fn PlotPointCard(point: PlotPointView, scenes: Vec<(Id, String)>, border: String, extras: Vec<LinkedScene>) -> Element {
    let mut state = use_context::<Signal<StudioState>>();
    let point_id = point.id;
    let title = point.title.clone();
    let summary = point.summary.clone();
    let scene_id = point.scene_id;
    let scene_value = scene_id.map(|id| id.to_string()).unwrap_or_default();
    let mut options = scenes;
    if let Some(id) = scene_id {
        if let Some(extra) = extras.into_iter().find(|extra| extra.id == id) {
            options.push((extra.id, extra.label));
        }
    }
    rsx! {
        article { class: "plot-card", style: "{border}", key: "{point_id}",
            input {
                title: "Plot point title",
                value: "{title}",
                oninput: move |event| {
                    state.write().set_plot_point_title(point_id, event.value());
                },
            }
            textarea {
                title: "What happens at this point",
                value: "{summary}",
                oninput: move |event| {
                    state.write().set_plot_point_summary(point_id, event.value());
                },
            }
            select {
                title: "Linked manuscript scene",
                value: "{scene_value}",
                onchange: move |event| {
                    state.write().set_plot_point_scene(point_id, parse_scene_id(&event.value()));
                },
                option { value: "", selected: scene_value.is_empty(), "No scene" }
                for (id, label) in options {
                    option { value: "{id}", selected: id.to_string() == scene_value, "{label}" }
                }
            }
            div { class: "plot-card-actions",
                button {
                    title: "Move earlier on this plotline",
                    onclick: move |_| state.write().shift_plot_point(point_id, true),
                    "Earlier"
                }
                button {
                    title: "Move later on this plotline",
                    onclick: move |_| state.write().shift_plot_point(point_id, false),
                    "Later"
                }
                button {
                    disabled: scene_id.is_none(),
                    onclick: move |_| {
                        if let Some(id) = scene_id {
                            state.write().open_linked_scene(id);
                        }
                    },
                    "Open"
                }
                button {
                    onclick: move |_| state.write().remove_plot_point(point_id),
                    "Remove"
                }
            }
        }
    }
}
