//! App-level keyboard chords. Editor formatting (bold, italic, underline)
//! stays with the manuscript field.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppShortcut {
    Save,
    Undo,
    Redo,
    Snapshot,
    Find,
    ToggleFocus,
    Writing,
    Corkboard,
    Outliner,
    Plot,
    Characters,
    World,
    Research,
    History,
    NewScene,
    NewChapter,
    OpenProject,
    ClearOrExitFocus,
    ToggleSidebar,
    ToggleInspector,
    ShowShortcuts,
}

/// One line of the shortcuts window: what it does and the chord for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Listed {
    pub what: &'static str,
    pub key: &'static str,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

impl Listed {
    const fn ctrl(what: &'static str, key: &'static str) -> Self {
        Self { what, key, ctrl: true, shift: false, alt: false }
    }

    const fn ctrl_shift(what: &'static str, key: &'static str) -> Self {
        Self { what, key, ctrl: true, shift: true, alt: false }
    }

    const fn ctrl_alt(what: &'static str, key: &'static str) -> Self {
        Self { what, key, ctrl: true, shift: false, alt: true }
    }

    /// The chord as it is written: "Ctrl+Shift+S".
    pub fn chord(&self) -> String {
        let mut parts = Vec::new();
        if self.ctrl {
            parts.push("Ctrl".to_string());
        }
        if self.shift {
            parts.push("Shift".to_string());
        }
        if self.alt {
            parts.push("Alt".to_string());
        }
        parts.push(match self.key {
            "escape" => "Escape".to_string(),
            key => key.to_ascii_uppercase(),
        });
        parts.join("+")
    }
}

/// Every app chord, in the order the shortcuts window lists them. A test
/// checks each one against `app_shortcut`, so the window cannot list a
/// chord that does nothing.
pub const LISTED: [Listed; 22] = [
    Listed::ctrl("Save", "s"),
    Listed::ctrl_shift("Snapshot this document", "s"),
    Listed::ctrl("Undo", "z"),
    Listed::ctrl("Redo", "y"),
    Listed::ctrl_shift("Redo", "z"),
    Listed::ctrl("Search the binder, or filter the corkboard", "f"),
    Listed::ctrl_shift("Focus mode", "f"),
    Listed::ctrl("Focus mode", "\\"),
    Listed::ctrl("Writing", "1"),
    Listed::ctrl("Corkboard", "2"),
    Listed::ctrl("Outliner", "3"),
    Listed::ctrl("Plot", "4"),
    Listed::ctrl("Characters", "5"),
    Listed::ctrl("World", "6"),
    Listed::ctrl("Research", "7"),
    Listed::ctrl("History", "8"),
    Listed::ctrl("New scene", "n"),
    Listed::ctrl_shift("New chapter", "n"),
    Listed::ctrl("Open a project", "o"),
    Listed::ctrl_alt("Show or hide the binder", "b"),
    Listed::ctrl_alt("Show or hide the inspector", "i"),
    Listed::ctrl("This list of shortcuts", "/"),
];

/// Chords the manuscript handles itself, listed for the writer.
pub const IN_THE_MANUSCRIPT: [(&str, &str); 3] = [
    ("Bold", "Ctrl+B"),
    ("Italic", "Ctrl+I"),
    ("Underline", "Ctrl+U"),
];

pub fn app_shortcut(key: &str, ctrl: bool, shift: bool, alt: bool) -> Option<AppShortcut> {
    let key = key.to_ascii_lowercase();
    if key == "escape" && !ctrl && !shift && !alt {
        return Some(AppShortcut::ClearOrExitFocus);
    }
    if !ctrl {
        return None;
    }
    if alt && !shift {
        return match key.as_str() {
            "b" => Some(AppShortcut::ToggleSidebar),
            "i" => Some(AppShortcut::ToggleInspector),
            _ => None,
        };
    }
    if alt {
        return None;
    }
    Some(match (key.as_str(), shift) {
        ("s", false) => AppShortcut::Save,
        ("s", true) => AppShortcut::Snapshot,
        ("z", false) => AppShortcut::Undo,
        ("z", true) | ("y", false) => AppShortcut::Redo,
        ("f", false) => AppShortcut::Find,
        ("f", true) | ("\\", false) => AppShortcut::ToggleFocus,
        ("1", false) => AppShortcut::Writing,
        ("2", false) => AppShortcut::Corkboard,
        ("3", false) => AppShortcut::Outliner,
        ("4", false) => AppShortcut::Plot,
        ("5", false) => AppShortcut::Characters,
        ("6", false) => AppShortcut::World,
        ("7", false) => AppShortcut::Research,
        ("8", false) => AppShortcut::History,
        ("n", false) => AppShortcut::NewScene,
        ("n", true) => AppShortcut::NewChapter,
        ("o", false) => AppShortcut::OpenProject,
        ("/", false) => AppShortcut::ShowShortcuts,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_undo_and_redo() {
        assert_eq!(app_shortcut("s", true, false, false), Some(AppShortcut::Save));
        assert_eq!(app_shortcut("S", true, false, false), Some(AppShortcut::Save));
        assert_eq!(app_shortcut("z", true, false, false), Some(AppShortcut::Undo));
        assert_eq!(app_shortcut("y", true, false, false), Some(AppShortcut::Redo));
        assert_eq!(app_shortcut("z", true, true, false), Some(AppShortcut::Redo));
        assert_eq!(app_shortcut("s", false, false, false), None);
    }

    #[test]
    fn navigation_and_editing_chords() {
        assert_eq!(app_shortcut("f", true, false, false), Some(AppShortcut::Find));
        assert_eq!(app_shortcut("f", true, true, false), Some(AppShortcut::ToggleFocus));
        assert_eq!(app_shortcut("\\", true, false, false), Some(AppShortcut::ToggleFocus));
        assert_eq!(app_shortcut("1", true, false, false), Some(AppShortcut::Writing));
        assert_eq!(app_shortcut("2", true, false, false), Some(AppShortcut::Corkboard));
        assert_eq!(app_shortcut("3", true, false, false), Some(AppShortcut::Outliner));
        assert_eq!(app_shortcut("4", true, false, false), Some(AppShortcut::Plot));
        assert_eq!(app_shortcut("5", true, false, false), Some(AppShortcut::Characters));
        assert_eq!(app_shortcut("6", true, false, false), Some(AppShortcut::World));
        assert_eq!(app_shortcut("7", true, false, false), Some(AppShortcut::Research));
        assert_eq!(app_shortcut("8", true, false, false), Some(AppShortcut::History));
        assert_eq!(app_shortcut("n", true, false, false), Some(AppShortcut::NewScene));
        assert_eq!(app_shortcut("n", true, true, false), Some(AppShortcut::NewChapter));
        assert_eq!(app_shortcut("o", true, false, false), Some(AppShortcut::OpenProject));
        assert_eq!(app_shortcut("s", true, true, false), Some(AppShortcut::Snapshot));
        assert_eq!(
            app_shortcut("escape", false, false, false),
            Some(AppShortcut::ClearOrExitFocus)
        );
        assert_eq!(app_shortcut("b", true, false, false), None);
        assert_eq!(app_shortcut("b", true, false, true), Some(AppShortcut::ToggleSidebar));
        assert_eq!(
            app_shortcut("i", true, false, true),
            Some(AppShortcut::ToggleInspector)
        );
        assert_eq!(app_shortcut("b", true, true, true), None);
        assert_eq!(app_shortcut("escape", true, false, false), None);
    }

    #[test]
    fn every_listed_chord_does_something_and_none_is_listed_twice() {
        for item in LISTED {
            assert!(
                app_shortcut(item.key, item.ctrl, item.shift, item.alt).is_some(),
                "{} is listed but does nothing",
                item.chord()
            );
        }
        let mut chords: Vec<String> = LISTED.iter().map(Listed::chord).collect();
        chords.sort();
        chords.dedup();
        assert_eq!(chords.len(), LISTED.len());
        assert_eq!(LISTED[1].chord(), "Ctrl+Shift+S");
        assert_eq!(
            app_shortcut("/", true, false, false),
            Some(AppShortcut::ShowShortcuts)
        );
    }
}
