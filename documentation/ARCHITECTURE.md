# Architecture

Writing Studio is a Cargo workspace. Application logic lives in crates under `crates/`. The Dioxus desktop shell in `apps/desktop` is a thin launcher.

The program is open source and free for community use. The architecture is local-first: a usable writing studio never depends on a network.

```text
Dioxus UI
    → editor session, binder, compile input
        → SQLite project folder (*.writingstudio)
```

Compile (Markdown / DOCX) reads the binder and `documents` table. It does not go through the rich-text widget.

```text
Binder + stored documents → writing-studio-export → .md / .docx on disk
```

## Phase 1 crates that actually run

| Crate | Responsibility |
| --- | --- |
| `writing-studio-core` | IDs, timestamps, errors, events, autosave debounce, format version |
| `writing-studio-document` | Block document model, JSON, restricted HTML interchange |
| `writing-studio-editor` | Session, undo/redo, HTML/plain apply, counts |
| `writing-studio-project` | Binder tree, settings, drag-and-drop, trash/restore |
| `writing-studio-manuscript` | Book/chapter/scene outline |
| `writing-studio-templates` | Blank novel seed |
| `writing-studio-storage` | SQLite, snapshots, folder backups, restore, load-all documents |
| `writing-studio-corkboard` | Scene cards from the binder |
| `writing-studio-search` | Binder title filter |
| `writing-studio-export` | Manuscript compile to Markdown and DOCX |
| `writing-studio-ui` | Workspace UI |
| `writing-studio-desktop` | Native window |

Remaining crates are typed placeholders (plotting, characters, worldbuilding, research, collaboration, synchronization, security) so module boundaries stay stable.

## Data flow

1. The UI holds a `StudioState` with an optional `ProjectStore` (SQLite connection in the project folder).
2. Opening a scene loads JSON into an `EditorSession` wrapping a `Document`.
3. Typing updates the session; idle autosave, blur, scene change, and Ctrl+S write JSON back through `ProjectStore`.
4. Binder moves and trash update `binder_nodes` (parent, `sort_index`, `deleted_at`, `restore_parent_id`).
5. History restore either replaces one document from `snapshots` or copies a `backups/*.sqlite` into the live connection.
6. Compile loads all documents, walks the draft root, and writes a file the user chooses.

## Non-negotiables

- The UI never owns the document schema.
- Saving is transactional. Backups copy SQLite via the backup API.
- Cloud, collaboration, and export engines must consume the same document JSON.
- Offline writing stays complete if a future optional sync service is unused or unavailable.
- There is no account gate and no separate “community vs restricted” feature split in the core.

See [PHASE1.md](PHASE1.md) for the feature list and [PROJECT_FORMAT.md](PROJECT_FORMAT.md) for on-disk layout.
