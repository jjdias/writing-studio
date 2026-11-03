use crate::{Id, Timestamp};

/// Domain events emitted by the core so save, search, sync, and undo can react
/// without the UI owning business rules.
#[derive(Clone, Debug)]
pub enum StudioEvent {
    ProjectOpened {
        project_id: Id,
        at: Timestamp,
    },
    StructureChanged {
        project_id: Id,
        node_id: Id,
    },
    DocumentChanged {
        document_id: Id,
    },
    DocumentSaved {
        document_id: Id,
        at: Timestamp,
    },
    BackupCreated {
        at: Timestamp,
    },
}
