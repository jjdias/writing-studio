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
    History,
    NewScene,
    NewChapter,
    OpenProject,
    ClearOrExitFocus,
}

pub fn app_shortcut(key: &str, ctrl: bool, shift: bool) -> Option<AppShortcut> {
    let key = key.to_ascii_lowercase();
    if key == "escape" && !ctrl && !shift {
        return Some(AppShortcut::ClearOrExitFocus);
    }
    if !ctrl {
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
        assert_eq!(app_shortcut("s", true, false), Some(AppShortcut::Save));
        assert_eq!(app_shortcut("S", true, false), Some(AppShortcut::Save));
        assert_eq!(app_shortcut("z", true, false), Some(AppShortcut::Undo));
        assert_eq!(app_shortcut("y", true, false), Some(AppShortcut::Redo));
        assert_eq!(app_shortcut("z", true, true), Some(AppShortcut::Redo));
        assert_eq!(app_shortcut("s", false, false), None);
    }

    #[test]
    fn navigation_and_editing_chords() {
        assert_eq!(app_shortcut("f", true, false), Some(AppShortcut::Find));
        assert_eq!(app_shortcut("f", true, true), Some(AppShortcut::ToggleFocus));
        assert_eq!(app_shortcut("\\", true, false), Some(AppShortcut::ToggleFocus));
        assert_eq!(app_shortcut("1", true, false), Some(AppShortcut::Writing));
        assert_eq!(app_shortcut("2", true, false), Some(AppShortcut::Corkboard));
        assert_eq!(app_shortcut("3", true, false), Some(AppShortcut::Outliner));
        assert_eq!(app_shortcut("4", true, false), Some(AppShortcut::History));
        assert_eq!(app_shortcut("n", true, false), Some(AppShortcut::NewScene));
        assert_eq!(app_shortcut("n", true, true), Some(AppShortcut::NewChapter));
        assert_eq!(app_shortcut("o", true, false), Some(AppShortcut::OpenProject));
        assert_eq!(app_shortcut("s", true, true), Some(AppShortcut::Snapshot));
        assert_eq!(app_shortcut("escape", false, false), Some(AppShortcut::ClearOrExitFocus));
        assert_eq!(app_shortcut("b", true, false), None);
        assert_eq!(app_shortcut("escape", true, false), None);
    }
}
