//! Contrast of the theme colors against WCAG AA, read from `style.css`.
//! How the window looks is not tested, but whether its text and controls
//! can be read is a sum over these colors, so it is checked here.

use std::collections::HashMap;

const STYLES: &str = include_str!("../assets/style.css");

/// The `--name: #rrggbb` colors of one block, such as `:root {`.
fn tokens(selector: &str) -> HashMap<String, String> {
    let start = STYLES
        .find(&format!("{selector} {{"))
        .unwrap_or_else(|| panic!("no {selector} block in style.css"));
    let body = &STYLES[start..];
    let end = body.find('}').expect("block is closed");
    body[..end]
        .lines()
        .filter_map(|line| {
            let (name, value) = line.trim().strip_prefix("--")?.split_once(':')?;
            let value = value.trim().trim_end_matches(';').trim();
            value.starts_with('#').then(|| (name.trim().to_string(), value.to_string()))
        })
        .collect()
}

/// Each theme's colors: its own block over the dark defaults in `:root`.
fn theme(name: &str) -> HashMap<String, String> {
    let mut colors = tokens(":root");
    if name != "dark" {
        colors.extend(tokens(&format!(".shell.theme-{name}")));
    }
    colors
}

fn luminance(hex: &str) -> f64 {
    let hex = hex.trim_start_matches('#');
    assert_eq!(hex.len(), 6, "#{hex} is not #rrggbb");
    let channel = |at: usize| {
        let value = u8::from_str_radix(&hex[at..at + 2], 16).unwrap() as f64 / 255.0;
        if value <= 0.03928 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(0) + 0.7152 * channel(2) + 0.0722 * channel(4)
}

fn ratio(one: &str, other: &str) -> f64 {
    let (a, b) = (luminance(one), luminance(other));
    let (light, dark) = if a > b { (a, b) } else { (b, a) };
    (light + 0.05) / (dark + 0.05)
}

/// Text needs 4.5:1. A control's edge and the focus ring need 3:1.
const TEXT: [(&str, &str); 14] = [
    ("text", "panel"),
    ("text", "panel-2"),
    ("text", "control"),
    ("muted", "panel"),
    ("muted", "panel-2"),
    ("editor-text", "editor"),
    ("link", "panel"),
    ("link", "panel-2"),
    ("page-link", "editor"),
    ("danger", "panel"),
    ("danger", "panel-2"),
    ("selection-fg", "selection"),
    ("on-fill", "primary"),
    ("on-fill", "danger-fill"),
];

const EDGES: [(&str, &str); 5] = [
    ("control-border", "panel"),
    ("control-border", "panel-2"),
    ("input-border", "panel"),
    ("input-border", "panel-2"),
    ("gold", "panel"),
];

#[test]
fn every_theme_meets_wcag_aa_for_text_and_controls() {
    let mut failures = Vec::new();
    for name in ["dark", "light", "paper"] {
        let colors = theme(name);
        let color = |token: &str| {
            colors
                .get(token)
                .unwrap_or_else(|| panic!("--{token} is not set for {name}"))
                .clone()
        };
        for (pairs, needed) in [(&TEXT[..], 4.5), (&EDGES[..], 3.0)] {
            for (front, back) in pairs {
                let found = ratio(&color(front), &color(back));
                if found < needed {
                    failures.push(format!("{name}: --{front} on --{back} is {found:.2}:1, needs {needed}:1"));
                }
            }
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

#[test]
fn contrast_is_measured_the_wcag_way() {
    assert!((ratio("#000000", "#ffffff") - 21.0).abs() < 0.01);
    assert!((ratio("#777777", "#ffffff") - 4.48).abs() < 0.01);
}
