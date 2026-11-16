//! Keeping one project on more than one machine. Optional: a project that
//! is never synced works the same.
//!
//! [`merge`] combines two copies of a project that were changed apart.
//! [`git`] carries a project between machines with the `git` program.

pub mod git;
mod merge;

pub use merge::{merge, Conflict, Merged};
