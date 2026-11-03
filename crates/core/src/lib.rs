//! Shared primitives used by every Writing Studio crate.
//!
//! The UI must never own these types. Persistence, sync, and export all speak
//! this language so manuscript data stays independent of the interface.

mod error;
mod event;
mod id;
mod time;

pub use error::{Error, Result};
pub use event::StudioEvent;
pub use id::{Id, NodeId};
pub use time::Timestamp;

/// Current on-disk project format. Bump only with a documented migration.
pub const PROJECT_FORMAT_VERSION: u32 = 1;

/// Marker filename stored beside the SQLite database.
pub const PROJECT_FORMAT_FILE: &str = "FORMAT";

/// SQLite database filename inside a project folder.
pub const PROJECT_DB_FILE: &str = "project.sqlite";

/// Human-readable product name.
pub const PRODUCT_NAME: &str = "Writing Studio";
