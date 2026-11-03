use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("project not found at {0}")]
    ProjectNotFound(PathBuf),

    #[error("unsupported project format version {found} (expected {expected})")]
    UnsupportedFormat { found: u32, expected: u32 },

    #[error("invalid project: {0}")]
    InvalidProject(String),

    #[error("document {0} was not found")]
    DocumentNotFound(String),

    #[error("binder node {0} was not found")]
    NodeNotFound(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Message(String),
}

impl Error {
    pub fn message(msg: impl Into<String>) -> Self {
        Self::Message(msg.into())
    }
}
