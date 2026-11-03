# Architecture

Writing Studio is a Cargo workspace. Application logic lives in crates under `crates/`. The Dioxus desktop shell in `apps/desktop` is a thin launcher.

```text
Dioxus UI  →  shared crates  →  SQLite project folder
```

## Phase 1 crates that actually run

| Crate | Responsibility |
| --- | --- |
| `writing-studio-core` | IDs, timestamps, errors, events, format version |
| `writing-studio-document` | Block document model and JSON schema |
| `writing-studio-editor` | Session, undo/redo, counts |
| `writing-studio-project` | Binder tree and settings |
| `writing-studio-manuscript` | Book/chapter/scene outline |
| `writing-studio-templates` | Blank novel seed |
| `writing-studio-storage` | SQLite + backups |
| `writing-studio-ui` | Workspace UI |
| `writing-studio-desktop` | Native window |

Remaining crates are typed placeholders for later phases so the module boundaries stay stable.

## Non-negotiables

- The UI never owns the document schema.
- Saving is transactional. Backups copy SQLite via the backup API.
- Cloud, CRDTs, and export engines must consume the same document JSON.
