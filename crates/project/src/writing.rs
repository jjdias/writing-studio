//! Daily writing progress and completed sessions stored with the project.

use serde::{Deserialize, Serialize};
use writing_studio_core::Timestamp;

/// Completed sittings kept in the project. Older entries drop off the front.
pub const SESSION_HISTORY_LIMIT: usize = 40;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredSession {
    pub day: String,
    pub started_at: String,
    pub ended_at: String,
    pub words: u32,
    pub seconds: u64,
}

/// Manuscript baselines for today and the open sitting.
///
/// `day_baseline` is the manuscript word count at the start of `day`.
/// `session_baseline` is the count at the start of the sitting that is still open.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WritingLog {
    #[serde(default)]
    pub day: String,
    #[serde(default)]
    pub day_baseline: u32,
    #[serde(default)]
    pub session_started_at: String,
    #[serde(default)]
    pub session_active_at: String,
    #[serde(default)]
    pub session_baseline: u32,
    #[serde(default)]
    pub sessions: Vec<StoredSession>,
}

impl Default for WritingLog {
    fn default() -> Self {
        Self {
            day: String::new(),
            day_baseline: 0,
            session_started_at: String::new(),
            session_active_at: String::new(),
            session_baseline: 0,
            sessions: Vec::new(),
        }
    }
}

impl WritingLog {
    pub fn words_today(&self, current_words: u32) -> u32 {
        current_words.saturating_sub(self.day_baseline)
    }

    pub fn session_words(&self, current_words: u32) -> u32 {
        current_words.saturating_sub(self.session_baseline)
    }

    /// Close the sitting stored in this log, then start one at `current_words`.
    ///
    /// A change of local day points today's baseline at `current_words`.
    /// A sitting that added no words is not recorded.
    pub fn open_session(&mut self, today: &str, started_at: &str, current_words: u32) {
        self.finish_session(current_words);
        if self.day != today {
            self.day = today.to_string();
            self.day_baseline = current_words;
        }
        self.session_started_at = started_at.to_string();
        self.session_active_at = started_at.to_string();
        self.session_baseline = current_words;
    }

    pub fn note_activity(&mut self, at: &str) {
        if self.session_started_at.is_empty() {
            return;
        }
        self.session_active_at = at.to_string();
    }

    fn finish_session(&mut self, current_words: u32) {
        if self.session_started_at.is_empty() {
            return;
        }
        let words = current_words.saturating_sub(self.session_baseline);
        if words > 0 {
            let ended_at = if self.session_active_at.is_empty() {
                self.session_started_at.clone()
            } else {
                self.session_active_at.clone()
            };
            let seconds = elapsed_seconds(&self.session_started_at, &ended_at);
            self.sessions.push(StoredSession {
                day: self.day.clone(),
                started_at: self.session_started_at.clone(),
                ended_at,
                words,
                seconds,
            });
            if self.sessions.len() > SESSION_HISTORY_LIMIT {
                let extra = self.sessions.len() - SESSION_HISTORY_LIMIT;
                self.sessions.drain(0..extra);
            }
        }
        self.session_started_at.clear();
        self.session_active_at.clear();
        self.session_baseline = current_words;
    }
}

pub fn format_elapsed(seconds: u64) -> String {
    let minutes = seconds / 60;
    if minutes == 0 {
        return "under 1 min".into();
    }
    if minutes < 60 {
        return format!("{minutes} min");
    }
    let hours = minutes / 60;
    let remain = minutes % 60;
    if remain == 0 {
        format!("{hours} h")
    } else {
        format!("{hours} h {remain} min")
    }
}

fn elapsed_seconds(start: &str, end: &str) -> u64 {
    let Ok(start) = Timestamp::parse(start) else {
        return 0;
    };
    let Ok(end) = Timestamp::parse(end) else {
        return 0;
    };
    end.inner()
        .signed_duration_since(start.inner())
        .num_seconds()
        .max(0) as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProjectSettings;

    #[test]
    fn same_day_keeps_the_daily_baseline_and_records_the_sitting() {
        let mut log = WritingLog::default();
        log.open_session("2026-11-06", "2026-11-06T18:00:00Z", 100);
        log.note_activity("2026-11-06T18:05:00Z");
        assert_eq!(log.words_today(150), 50);
        assert_eq!(log.session_words(150), 50);

        log.open_session("2026-11-06", "2026-11-06T19:00:00Z", 150);
        assert_eq!(log.day_baseline, 100);
        assert_eq!(log.session_baseline, 150);
        assert_eq!(log.words_today(150), 50);
        assert_eq!(log.session_words(150), 0);
        assert_eq!(log.sessions.len(), 1);
        assert_eq!(log.sessions[0].day, "2026-11-06");
        assert_eq!(log.sessions[0].words, 50);
        assert_eq!(log.sessions[0].seconds, 300);
        assert_eq!(log.sessions[0].ended_at, "2026-11-06T18:05:00Z");
    }

    #[test]
    fn a_new_day_resets_today_and_keeps_the_previous_sitting() {
        let mut log = WritingLog::default();
        log.open_session("2026-11-06", "2026-11-06T18:00:00Z", 100);
        log.note_activity("2026-11-06T18:10:00Z");
        log.open_session("2026-11-07", "2026-11-07T09:00:00Z", 180);
        assert_eq!(log.day, "2026-11-07");
        assert_eq!(log.day_baseline, 180);
        assert_eq!(log.words_today(180), 0);
        assert_eq!(log.sessions.len(), 1);
        assert_eq!(log.sessions[0].words, 80);
        assert_eq!(log.sessions[0].day, "2026-11-06");
    }

    #[test]
    fn an_empty_sitting_is_not_stored() {
        let mut log = WritingLog::default();
        log.open_session("2026-11-06", "2026-11-06T18:00:00Z", 100);
        log.open_session("2026-11-06", "2026-11-06T19:00:00Z", 100);
        assert!(log.sessions.is_empty());
    }

    #[test]
    fn history_keeps_the_newest_sessions() {
        let mut log = WritingLog::default();
        log.open_session("2026-11-06", "2026-11-06T00:00:00Z", 0);
        let mut words = 0;
        for added in 1..=45 {
            words += added;
            log.note_activity("2026-11-06T01:00:00Z");
            log.open_session("2026-11-06", "2026-11-06T02:00:00Z", words);
        }
        assert_eq!(log.sessions.len(), SESSION_HISTORY_LIMIT);
        assert_eq!(log.sessions[0].words, 6);
        assert_eq!(log.sessions.last().map(|session| session.words), Some(45));
    }

    #[test]
    fn format_elapsed_uses_minutes_then_hours() {
        assert_eq!(format_elapsed(0), "under 1 min");
        assert_eq!(format_elapsed(59), "under 1 min");
        assert_eq!(format_elapsed(60), "1 min");
        assert_eq!(format_elapsed(3600), "1 h");
        assert_eq!(format_elapsed(3660), "1 h 1 min");
    }

    #[test]
    fn older_settings_json_still_loads() {
        let json = r#"{"daily_word_goal":250,"project_word_goal":1000,"compile_title_page":false,"autosave_seconds":4}"#;
        let settings: ProjectSettings = serde_json::from_str(json).unwrap();
        assert_eq!(settings.daily_word_goal, 250);
        assert_eq!(settings.project_word_goal, 1000);
        assert!(!settings.compile_title_page);
        assert!(settings.writing.day.is_empty());
        assert!(settings.writing.sessions.is_empty());
    }
}
