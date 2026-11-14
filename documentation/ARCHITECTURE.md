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

## Crates that run

| Crate | Responsibility |
| --- | --- |
| `writing-studio-core` | IDs, timestamps, errors, events, autosave debounce, format version |
| `writing-studio-document` | Block document model, JSON, restricted HTML interchange, text comparison between two versions |
| `writing-studio-editor` | Session, undo/redo, HTML/plain apply, counts |
| `writing-studio-project` | Binder tree, settings, drag-and-drop, trash/restore |
| `writing-studio-manuscript` | Book/chapter/scene outline |
| `writing-studio-templates` | Blank novel seed |
| `writing-studio-storage` | SQLite, snapshots, folder backups, restore, load-all documents |
| `writing-studio-corkboard` | Scene cards from the binder |
| `writing-studio-plotting` | Plotlines, plot points, and scene links |
| `writing-studio-characters` | Profiles, motivations, and relationships |
| `writing-studio-worldbuilding` | Locations, cultures, lore, and timelines |
| `writing-studio-research` | Research notes and files in `attachments/` |
| `writing-studio-search` | In-memory full-text index of manuscript text, rebuilt from SQLite when a project opens |
| `writing-studio-export` | Manuscript compile to Markdown and DOCX |
| `writing-studio-ui` | Workspace UI |
| `writing-studio-desktop` | Native window |

The plot grid, character cast, world notes, and research library are stored in the same project database as the binder. Research files are copied into `attachments/`. Search is not stored there: the Tantivy index is memory only. Remaining crates are typed placeholders (collaboration, synchronization, security) so module boundaries stay stable.

Inline comments, links, code blocks, and scene breaks are fields on the document JSON. An inline comment is a note on one text run (`marks.comment`). Review annotations, highlights, and track changes are still Phase 4. See [PHASE2.md](PHASE2.md) and [ROADMAP.md](../ROADMAP.md).

## Data flow

1. The UI holds a `StudioState` with an optional `ProjectStore` (SQLite connection in the project folder).
2. Opening a scene loads JSON into an `EditorSession` wrapping a `Document`. Saved JSON is the block model, including links, code blocks, scene breaks, and inline comments. The desktop view is restricted HTML mapped back onto that JSON.
3. Typing updates the session; idle autosave, blur, scene change, and Ctrl+S write JSON back through `ProjectStore`. A save also updates the in-memory search index.
4. Binder moves and trash update `binder_nodes` (parent, `sort_index`, `label`, `color`, `deleted_at`, `restore_parent_id`). Plot, characters, world notes, research, and scene links use the tables in [PROJECT_FORMAT.md](PROJECT_FORMAT.md). Opening a project copies leftover binder research notes into the research library and removes them from the binder. Emptying trash clears scene links to deleted scenes and drops a deleted book's word target.
5. History restore either replaces one document from `snapshots` or copies a `backups/*.sqlite` into the live connection. That SQLite copy does not include `attachments/`.
6. Compile loads documents, walks the chosen book, and writes a file the user chooses. Which book is compiled, and how today and the sitting follow that book, is in [PHASE3.md](PHASE3.md) and [PROJECT_FORMAT.md](PROJECT_FORMAT.md).

## Non-negotiables

- The UI never owns the document schema.
- Saving is transactional. Backups copy SQLite via the backup API.
- Cloud, collaboration, and export engines must consume the same document JSON.
- Offline writing stays complete if a future optional sync service is unused or unavailable.
- There is no account gate and no separate “community vs restricted” feature split in the core.

See [PHASE1.md](PHASE1.md), [PHASE2.md](PHASE2.md), and [PHASE3.md](PHASE3.md) for the feature lists and [PROJECT_FORMAT.md](PROJECT_FORMAT.md) for on-disk layout. Current work is Phase 4: [PHASE4.md](PHASE4.md) lists what has shipped, and [ROADMAP.md](../ROADMAP.md) lists what is left.
