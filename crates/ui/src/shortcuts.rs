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
    History,
    NewScene,
    NewChapter,
    OpenProject,
    ClearOrExitFocus,
    ToggleSidebar,
    ToggleInspector,
}

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
        ("4", false) => AppShortcut::History,
        ("5", false) => AppShortcut::Plot,
        ("6", false) => AppShortcut::Characters,
        ("n", false) => AppShortcut::NewScene,
        ("n", true) => AppShortcut::NewChapter,
        ("o", false) => AppShortcut::OpenProject,
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
        assert_eq!(app_shortcut("4", true, false, false), Some(AppShortcut::History));
        assert_eq!(app_shortcut("5", true, false, false), Some(AppShortcut::Plot));
        assert_eq!(app_shortcut("6", true, false, false), Some(AppShortcut::Characters));
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
}
