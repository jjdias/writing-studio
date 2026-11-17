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
    rsx! {
        div { class: "plot-pane",
            div { class: "plot-toolbar",
                button {
                    class: "primary",
                    onclick: move |_| state.write().add_plotline(),
                    "Add plotline"
                }
            }
            p { class: "hint", "Each row is a plotline. Plot points stay in order on that line and can open a manuscript scene. Titles save with the project." }
            if lines.is_empty() {
                p { class: "hint", "No plotlines yet." }
            }
            for line in lines {
                PlotLineRow { line, scenes: scenes.clone(), confirm, binder_scenes: linked_scene_labels(&snapshot, &scenes) }
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
                        option { value: "{swatch.id}", "{swatch.name}" }
                    }
                }
                button {
                    onclick: move |_| state.write().add_plot_point(line_id),
                    "Add point"
                }
                button {
                    class: danger(confirm == Some(line_id)),
                    onclick: move |_| state.write().remove_plotline(line_id),
                    "{delete_label}"
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
                option { value: "", "No scene" }
                for (id, label) in options {
                    option { value: "{id}", "{label}" }
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
