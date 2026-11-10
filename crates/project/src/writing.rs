//! Daily writing progress and completed sessions stored with the project.

use serde::{Deserialize, Serialize};
use writing_studio_core::{Id, Timestamp};

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

/// Today and the open sitting for one book.
///
/// Baselines are that book's outline word count, not the whole series.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookProgress {
    pub book_id: Id,
    #[serde(default)]
    pub day: String,
    #[serde(default)]
    pub day_baseline: u32,
    /// Outline word count when the open sitting started covering this book.
    #[serde(default)]
    pub session_baseline: Option<u32>,
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
    /// Per-book today and sitting baselines. Missing on older projects.
    #[serde(default)]
    pub books: Vec<BookProgress>,
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
            books: Vec::new(),
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

    /// Start a sitting from each book's outline count.
    ///
    /// The first call after an older project still closes the previous sitting
    /// with the single manuscript baseline. Later calls add up each book's change.
    pub fn open_books(&mut self, today: &str, started_at: &str, counts: &[(Id, u32)]) {
        if self.books.is_empty() {
            let sum = counts.iter().map(|(_, words)| *words).sum();
            self.open_session(today, started_at, sum);
        } else {
            self.finish_books(counts);
            self.session_started_at = started_at.to_string();
            self.session_active_at = started_at.to_string();
        }
        for &(book_id, current) in counts {
            let entry = self.ensure_book(book_id);
            if entry.day != today {
                entry.day = today.to_string();
                entry.day_baseline = current;
            }
            entry.session_baseline = Some(current);
        }
    }

    /// Remember a book that was not in the sitting yet, without resetting one that was.
    pub fn track_book(&mut self, book_id: Id, today: &str, current: u32) {
        let fresh = self.books.iter().all(|book| book.book_id != book_id);
        let sitting = !self.session_started_at.is_empty();
        let entry = self.ensure_book(book_id);
        if fresh || entry.day != today {
            entry.day = today.to_string();
            entry.day_baseline = current;
        }
        if sitting && entry.session_baseline.is_none() {
            entry.session_baseline = Some(current);
        }
    }

    pub fn words_today_for_book(&self, book_id: Id, current: u32) -> u32 {
        self.books
            .iter()
            .find(|book| book.book_id == book_id)
            .map(|book| current.saturating_sub(book.day_baseline))
            .unwrap_or(0)
    }

    pub fn session_words_for_book(&self, book_id: Id, current: u32) -> u32 {
        self.books
            .iter()
            .find(|book| book.book_id == book_id)
            .and_then(|book| book.session_baseline)
            .map(|baseline| current.saturating_sub(baseline))
            .unwrap_or(0)
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

    fn finish_books(&mut self, counts: &[(Id, u32)]) {
        if self.session_started_at.is_empty() {
            return;
        }
        let words = counts.iter().fold(0u32, |total, (book_id, current)| {
            let added = self
                .books
                .iter()
                .find(|book| book.book_id == *book_id)
                .and_then(|book| book.session_baseline)
                .map(|baseline| current.saturating_sub(baseline))
                .unwrap_or(0);
            total.saturating_add(added)
        });
        self.record_session(words);
        for book in &mut self.books {
            book.session_baseline = None;
        }
        self.session_started_at.clear();
        self.session_active_at.clear();
    }

    fn ensure_book(&mut self, book_id: Id) -> &mut BookProgress {
        if let Some(index) = self.books.iter().position(|book| book.book_id == book_id) {
            return &mut self.books[index];
        }
        self.books.push(BookProgress {
            book_id,
            day: String::new(),
            day_baseline: 0,
            session_baseline: None,
        });
        self.books.last_mut().expect("book progress was just added")
    }

    fn record_session(&mut self, words: u32) {
        if self.session_started_at.is_empty() || words == 0 {
            return;
        }
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
    fn each_book_keeps_its_own_today_and_sitting() {
        let mut log = WritingLog::default();
        let harbor = Id::new();
        let inland = Id::new();
        log.open_books(
            "2026-11-06",
            "2026-11-06T18:00:00Z",
            &[(harbor, 100), (inland, 40)],
        );
        assert_eq!(log.words_today_for_book(harbor, 130), 30);
        assert_eq!(log.session_words_for_book(harbor, 130), 30);
        assert_eq!(log.words_today_for_book(inland, 40), 0);
        log.note_activity("2026-11-06T18:20:00Z");
        log.open_books(
            "2026-11-06",
            "2026-11-06T19:00:00Z",
            &[(harbor, 130), (inland, 55)],
        );
        assert_eq!(log.sessions.len(), 1);
        assert_eq!(log.sessions[0].words, 45);
        assert_eq!(log.words_today_for_book(harbor, 130), 30);
        assert_eq!(log.words_today_for_book(inland, 55), 15);
        assert_eq!(log.session_words_for_book(inland, 55), 0);
        let added = Id::new();
        log.track_book(added, "2026-11-06", 0);
        log.track_book(added, "2026-11-06", 12);
        assert_eq!(log.words_today_for_book(added, 12), 12);
        assert_eq!(log.session_words_for_book(added, 12), 12);
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
