//! The font families installed on this computer, for a compile profile's
//! font list.

use std::sync::OnceLock;

/// The installed families, sorted and each listed once. The computer's
/// fonts are read the first time they are asked for and kept while the
/// program runs; a font installed after that shows on the next start.
pub(crate) fn installed_families() -> &'static [String] {
    static FAMILIES: OnceLock<Vec<String>> = OnceLock::new();
    FAMILIES.get_or_init(|| {
        let mut fonts = fontdb::Database::new();
        fonts.load_system_fonts();
        // A face lists its family first in English, which is the name Word
        // and other programs know it by.
        family_list(fonts.faces().filter_map(|face| face.families.first().map(|(name, _)| name.clone())))
    })
}

/// Names sorted without regard to case, each once. Names Windows keeps for
/// vertical writing ("@MS Gothic") and hidden system faces (".SF NS") are
/// not fonts a writer chooses.
fn family_list(names: impl Iterator<Item = String>) -> Vec<String> {
    let mut list: Vec<String> = names
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty() && !name.starts_with('@') && !name.starts_with('.'))
        .collect();
    list.sort_by_key(|name| name.to_lowercase());
    list.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    list
}

/// The choices for a font list: the installed families, and the profile's
/// own font first when this computer does not have it, as a profile made on
/// another computer can name one. Each is the font and its label.
pub(crate) fn font_choices(current: &str, installed: &[String]) -> Vec<(String, String)> {
    let mut choices: Vec<(String, String)> = Vec::with_capacity(installed.len() + 1);
    if !current.is_empty() && !installed.iter().any(|name| name.eq_ignore_ascii_case(current)) {
        choices.push((current.to_string(), format!("{current} (not on this computer)")));
    }
    choices.extend(installed.iter().map(|name| (name.clone(), name.clone())));
    choices
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn families_are_sorted_once_each_and_a_missing_font_is_still_a_choice() {
        let names = ["Times New Roman", "arial", "@MS Gothic", "Arial", " ", ".SF NS", "Garamond", "Times New Roman"];
        let list = family_list(names.into_iter().map(String::from));
        assert_eq!(list, ["arial", "Garamond", "Times New Roman"]);

        let choices = font_choices("Courier Prime", &list);
        assert_eq!(choices[0], ("Courier Prime".into(), "Courier Prime (not on this computer)".into()));
        assert_eq!(choices.len(), 4);
        assert_eq!(font_choices("Garamond", &list).len(), 3, "an installed font is listed once");
        assert_eq!(font_choices("ARIAL", &list).len(), 3, "whatever its case");
    }

    #[test]
    fn this_computer_has_fonts_to_choose_from() {
        let started = std::time::Instant::now();
        let families = installed_families();
        eprintln!("{} families in {:?}", families.len(), started.elapsed());
        assert!(!families.is_empty());
    }
}
