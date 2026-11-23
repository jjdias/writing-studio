# Architecture

Writing Studio is a Cargo workspace. Application logic lives in crates under `crates/`. The Dioxus desktop shell in `apps/desktop` is a thin launcher.

The program is open source and free for community use. The architecture is local-first: a usable writing studio never depends on a network.

```text
Dioxus UI
    → editor session, binder, compile input
        → project folder (*.writingstudio): HTML and JSON files, with a SQLite cache in .local/
```

Compile (Markdown / DOCX) reads the binder and `documents` table. It does not go through the rich-text widget.

```text
Binder + stored documents → writing-studio-export → .md / .docx on disk
```

## Crates that run

| Crate | Responsibility |
| --- | --- |
| `writing-studio-core` | IDs, timestamps, errors, events, autosave debounce, format version |
| `writing-studio-document` | Block document model, JSON, restricted HTML interchange, text comparison between two versions, tracked changes |
| `writing-studio-editor` | Session, undo/redo, HTML/plain apply, counts |
| `writing-studio-project` | Binder tree, settings, drag-and-drop, trash/restore |
| `writing-studio-manuscript` | Book/chapter/scene outline |
| `writing-studio-templates` | Blank novel seed |
| `writing-studio-storage` | The project folder: HTML and JSON files as the project, a SQLite cache beside them, snapshots, backups, and restore; the project exported as a folder of Markdown, and Markdown brought in |
| `writing-studio-plotting` | Plotlines, plot points, and scene links |
| `writing-studio-characters` | Profiles, motivations, and relationships |
| `writing-studio-worldbuilding` | Locations, cultures, lore, and timelines |
| `writing-studio-research` | Research notes and files in `attachments/` |
| `writing-studio-search` | In-memory full-text index of manuscript text, rebuilt from SQLite when a project opens |
| `writing-studio-diagnostics` | Scene goal, conflict, and outcome, and the counts and checks made on a scene's text. No dependencies |
| `writing-studio-context` | A scene's context: the scene, where it sits, the scenes around it, its plot points, and the characters, places, and notes linked to it or to what it is in; as Markdown, capped to a length. Reads the model crates only, not the window or the files |
| `writing-studio-format` | A scene as an HTML file, and back, exactly; and as Markdown for export and import. See [PROJECT_FORMAT.md](PROJECT_FORMAT.md) |
| `writing-studio-spellcheck` | Checks words against a Hunspell dictionary (English US is compiled in) and the project's own words; suggests replacements. No network |
| `writing-studio-synchronization` | Merging two copies of a project by paragraph and by field, and sync through the `git` program. See [SYNC.md](SYNC.md) |
| `writing-studio-export` | Manuscript compile to Markdown (with the format crate's writer) and DOCX |
| `writing-studio-ui` | Workspace UI |
| `writing-studio-desktop` | Native window |

The plot grid, character cast, world notes, and research library are files in the project folder beside the manuscript, and rows in the cache. Research files are copied into `attachments/`. Search is not stored: the Tantivy index is memory only.

Inline comments, links, code blocks, and scene breaks are fields on the document JSON. A note sits on text runs (`marks.comment`, with `comment_kind` and `comment_resolved`), and so does a highlight (`marks.highlight`). `Document::review_items` lists them, joining neighboring runs that carry the same note. Tracked changes are the difference between the document and a baseline copy kept in `tracking_baselines`; nothing about them is stored in the document. See [Phase 4](../roadmap/done/phase-4.md) and [the roadmap](../roadmap/README.md).

## Data flow

1. The UI holds a `StudioState` with an optional `ProjectStore`. The files in the project folder are the project. The store also holds a SQLite cache in `.local/`, filled from the files when the project opens. Every save writes the file first and the cache second.
2. Opening a scene loads JSON into an `EditorSession` wrapping a `Document`. Saved JSON is the block model, including links, code blocks, scene breaks, and inline comments. The desktop view is restricted HTML mapped back onto that JSON.
3. Typing updates the session; idle autosave, blur, scene change, and Ctrl+S write JSON back through `ProjectStore`. A save also updates the in-memory search index.
4. Binder moves and trash update `binder_nodes` (parent, `sort_index`, `label`, `color`, `deleted_at`, `restore_parent_id`). Plot, characters, world notes, research, and scene links use the tables in [PROJECT_FORMAT.md](PROJECT_FORMAT.md). Opening a project copies leftover binder research notes into the research library and removes them from the binder. Emptying trash clears scene links to deleted scenes and drops a deleted book's word target.
5. History restore either replaces one document from `snapshots` or copies a `.local/backups/*.sqlite` into the cache and writes the files again from it. That SQLite copy does not include `attachments/`.
6. Compile loads documents, walks the chosen book, and writes a file the user chooses. Which book is compiled, and how today and the sitting follow that book, is in [Phase 3](../roadmap/done/phase-3.md) and [PROJECT_FORMAT.md](PROJECT_FORMAT.md).

## Non-negotiables

- The UI never owns the document schema.
- Saving is safe against a crash: each file is written whole and renamed into place, and cache writes are transactional. Backups copy SQLite via the backup API.
- Sync, merge, and export work on the same document model as the editor.
- Offline writing stays complete if a future optional sync service is unused or unavailable.
- There is no account gate and no separate “community vs restricted” feature split in the core.

See [Phase 1](../roadmap/done/phase-1.md), [Phase 2](../roadmap/done/phase-2.md), [Phase 3](../roadmap/done/phase-3.md), and [Phase 4](../roadmap/done/phase-4.md) for what each phase added, and [PROJECT_FORMAT.md](PROJECT_FORMAT.md) for on-disk layout. What comes next is in [the roadmap](../roadmap/README.md).
