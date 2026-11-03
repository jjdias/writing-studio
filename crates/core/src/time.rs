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
