use serde::{Deserialize, Serialize};

use crate::WritingLog;

pub const SIDEBAR_WIDTH_DEFAULT: u32 = 260;
pub const SIDEBAR_WIDTH_MIN: u32 = 180;
pub const SIDEBAR_WIDTH_MAX: u32 = 480;
pub const INSPECTOR_WIDTH_DEFAULT: u32 = 300;
pub const INSPECTOR_WIDTH_MIN: u32 = 220;
pub const INSPECTOR_WIDTH_MAX: u32 = 560;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SidebarSide {
    Left,
    Right,
}

impl Default for SidebarSide {
    fn default() -> Self {
        Self::Left
    }
}

/// Which side the binder sits on, and whether each side panel is open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PanelChrome {
    pub sidebar_side: SidebarSide,
    pub sidebar_visible: bool,
    pub inspector_visible: bool,
}

impl Default for PanelChrome {
    fn default() -> Self {
        PanelLayout::default().chrome()
    }
}

/// Pixel widths for the binder and the inspector.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PanelWidths {
    pub sidebar: u32,
    pub inspector: u32,
}

impl Default for PanelWidths {
    fn default() -> Self {
        PanelLayout::default().widths()
    }
}

/// Workbench arrangement stored with the project. Missing on older projects.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PanelLayout {
    #[serde(default)]
    pub sidebar_side: SidebarSide,
    #[serde(default = "default_sidebar_width")]
    pub sidebar_width: u32,
    #[serde(default = "default_inspector_width")]
    pub inspector_width: u32,
    #[serde(default = "default_visible")]
    pub sidebar_visible: bool,
    #[serde(default = "default_visible")]
    pub inspector_visible: bool,
}

fn default_sidebar_width() -> u32 {
    SIDEBAR_WIDTH_DEFAULT
}

fn default_inspector_width() -> u32 {
    INSPECTOR_WIDTH_DEFAULT
}

fn default_visible() -> bool {
    true
}

impl Default for PanelLayout {
    fn default() -> Self {
        Self {
            sidebar_side: SidebarSide::Left,
            sidebar_width: SIDEBAR_WIDTH_DEFAULT,
            inspector_width: INSPECTOR_WIDTH_DEFAULT,
            sidebar_visible: true,
            inspector_visible: true,
        }
    }
}

impl PanelLayout {
    pub fn chrome(self) -> PanelChrome {
        PanelChrome {
            sidebar_side: self.sidebar_side,
            sidebar_visible: self.sidebar_visible,
            inspector_visible: self.inspector_visible,
        }
    }

    pub fn widths(self) -> PanelWidths {
        PanelWidths {
            sidebar: self.sidebar_width,
            inspector: self.inspector_width,
        }
    }

    pub fn with_chrome(mut self, chrome: PanelChrome) -> Self {
        self.sidebar_side = chrome.sidebar_side;
        self.sidebar_visible = chrome.sidebar_visible;
        self.inspector_visible = chrome.inspector_visible;
        self
    }

    pub fn with_widths(mut self, widths: PanelWidths) -> Self {
        self.sidebar_width = widths.sidebar;
        self.inspector_width = widths.inspector;
        self.clamp()
    }

    pub fn clamp(mut self) -> Self {
        self.sidebar_width = self
            .sidebar_width
            .clamp(SIDEBAR_WIDTH_MIN, SIDEBAR_WIDTH_MAX);
        self.inspector_width = self
            .inspector_width
            .clamp(INSPECTOR_WIDTH_MIN, INSPECTOR_WIDTH_MAX);
        self
    }
}

/// Workbench color theme stored with the project. Missing on older projects.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    Dark,
    Light,
    Paper,
}

impl Default for Theme {
    fn default() -> Self {
        Self::Dark
    }
}

impl Theme {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
            Self::Paper => "paper",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Dark => "Dark",
            Self::Light => "Light",
            Self::Paper => "Paper",
        }
    }

    pub fn from_id(value: &str) -> Option<Self> {
        match value {
            "dark" => Some(Self::Dark),
            "light" => Some(Self::Light),
            "paper" => Some(Self::Paper),
            _ => None,
        }
    }
}

/// Width after a horizontal drag. `positive` means moving the pointer right grows the panel.
pub fn drag_width(origin: u32, delta_px: i32, positive: bool, min: u32, max: u32) -> u32 {
    let next = if positive {
        origin as i32 + delta_px
    } else {
        origin as i32 - delta_px
    };
    next.clamp(min as i32, max as i32) as u32
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectSettings {
    pub daily_word_goal: u32,
    pub project_word_goal: u32,
    pub compile_title_page: bool,
    pub autosave_seconds: u32,
    /// Daily baseline and completed writing sessions. Missing on older projects.
    #[serde(default)]
    pub writing: WritingLog,
    /// Binder side, panel widths, and which panels are open. Missing on older projects.
    #[serde(default)]
    pub layout: PanelLayout,
    /// Dark, light, or paper. Missing on older projects.
    #[serde(default)]
    pub theme: Theme,
}

impl Default for ProjectSettings {
    fn default() -> Self {
        Self {
            daily_word_goal: 500,
            project_word_goal: 80_000,
            compile_title_page: true,
            autosave_seconds: 2,
            writing: WritingLog::default(),
            layout: PanelLayout::default(),
            theme: Theme::Dark,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn older_settings_keep_the_default_layout() {
        let json = r#"{"daily_word_goal":250,"project_word_goal":1000,"compile_title_page":false,"autosave_seconds":4}"#;
        let settings: ProjectSettings = serde_json::from_str(json).unwrap();
        assert_eq!(settings.layout, PanelLayout::default());
    }

    #[test]
    fn partial_layout_fills_the_other_fields() {
        let json = r#"{"daily_word_goal":1,"project_word_goal":2,"compile_title_page":true,"autosave_seconds":2,"layout":{"sidebar_side":"right","sidebar_width":900}}"#;
        let settings: ProjectSettings = serde_json::from_str(json).unwrap();
        let layout = settings.layout.clamp();
        assert_eq!(layout.sidebar_side, SidebarSide::Right);
        assert_eq!(layout.sidebar_width, SIDEBAR_WIDTH_MAX);
        assert_eq!(layout.inspector_width, INSPECTOR_WIDTH_DEFAULT);
        assert!(layout.sidebar_visible);
        assert!(layout.inspector_visible);
    }

    #[test]
    fn theme_defaults_to_dark_and_roundtrips() {
        let json = r#"{"daily_word_goal":1,"project_word_goal":2,"compile_title_page":true,"autosave_seconds":2}"#;
        let settings: ProjectSettings = serde_json::from_str(json).unwrap();
        assert_eq!(settings.theme, Theme::Dark);
        let saved = r#"{"daily_word_goal":1,"project_word_goal":2,"compile_title_page":true,"autosave_seconds":2,"theme":"paper"}"#;
        let settings: ProjectSettings = serde_json::from_str(saved).unwrap();
        assert_eq!(settings.theme, Theme::Paper);
        assert_eq!(Theme::from_id("light"), Some(Theme::Light));
        assert_eq!(Theme::from_id("sepia"), None);
    }

    #[test]
    fn drag_width_follows_the_pointer_and_stays_in_range() {
        assert_eq!(drag_width(260, 40, true, 180, 480), 300);
        assert_eq!(drag_width(260, 40, false, 180, 480), 220);
        assert_eq!(drag_width(260, 500, true, 180, 480), 480);
        assert_eq!(drag_width(260, 500, false, 180, 480), 180);
    }
}
