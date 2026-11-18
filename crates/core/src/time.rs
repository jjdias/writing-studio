use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// UTC timestamp stored as RFC 3339 in SQLite and JSON.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Timestamp(DateTime<Utc>);

impl Timestamp {
    pub fn now() -> Self {
        Self(Utc::now())
    }

    pub fn inner(self) -> DateTime<Utc> {
        self.0
    }

    pub fn to_rfc3339(self) -> String {
        self.0.to_rfc3339()
    }

    pub fn parse(value: &str) -> Result<Self, chrono::ParseError> {
        Ok(Self(DateTime::parse_from_rfc3339(value)?.with_timezone(&Utc)))
    }
}

impl Default for Timestamp {
    fn default() -> Self {
        Self::now()
    }
}

/// Local calendar day as `YYYY-MM-DD`.
pub fn local_day() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

/// Days from one `YYYY-MM-DD` day to another: 1 for tomorrow, negative for a
/// day already past. `None` when either is not a date.
pub fn days_between(from: &str, to: &str) -> Option<i64> {
    let parse = |day: &str| chrono::NaiveDate::parse_from_str(day.trim(), "%Y-%m-%d").ok();
    Some((parse(to)? - parse(from)?).num_days())
}

#[cfg(test)]
mod day_tests {
    use super::*;

    #[test]
    fn days_between_counts_calendar_days() {
        assert_eq!(days_between("2026-11-17", "2026-11-17"), Some(0));
        assert_eq!(days_between("2026-11-17", "2026-12-31"), Some(44));
        assert_eq!(days_between("2026-11-17", "2026-11-10"), Some(-7));
        assert_eq!(days_between("2026-11-17", "someday"), None);
    }
}
