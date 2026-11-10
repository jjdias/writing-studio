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
| `writing-studio-document` | Block document model, JSON, restricted HTML interchange |
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
| `writing-studio-search` | Full-text manuscript search |
| `writing-studio-export` | Manuscript compile to Markdown and DOCX |
| `writing-studio-ui` | Workspace UI |
| `writing-studio-desktop` | Native window |

The plot grid, character cast, world notes, and research library are stored in the same project database as the binder. Research files are copied into `attachments/`. Remaining crates are typed placeholders (collaboration, synchronization, security) so module boundaries stay stable.

## Data flow

1. The UI holds a `StudioState` with an optional `ProjectStore` (SQLite connection in the project folder).
2. Opening a scene loads JSON into an `EditorSession` wrapping a `Document`.
3. Typing updates the session; idle autosave, blur, scene change, and Ctrl+S write JSON back through `ProjectStore`.
4. Binder moves and trash update `binder_nodes` (parent, `sort_index`, `deleted_at`, `restore_parent_id`). Plotlines and plot points live in `plotlines` and `plot_points`. Characters, motivations, and relationships live in `characters`, `character_motivations`, and `character_relationships`. Locations, cultures, lore, and timeline events live in `locations`, `cultures`, `lore_entries`, and `timeline_events`. Research notes live in `research_items`. Each attached file is a row in `research_files` and a copy under `attachments/`. Characters, world records, and research notes can point at manuscript scenes through `bible_scene_links`. Those records stay project-level. Opening a project copies any notes still under the binder Research root into the research library and removes them from the binder. Emptying trash clears scene links to deleted scenes and, for a deleted book, removes that book's word target.
5. History restore either replaces one document from `snapshots` or copies a `backups/*.sqlite` into the live connection. That SQLite copy does not include `attachments/`.
6. Compile loads documents, walks the chosen book, and writes a file the user chooses. The title page uses that book's title. Other books stay out of the file. The status bar counts that book the way the outliner does, and today and the sitting use baselines stored for that book.

## Non-negotiables

- The UI never owns the document schema.
- Saving is transactional. Backups copy SQLite via the backup API.
- Cloud, collaboration, and export engines must consume the same document JSON.
- Offline writing stays complete if a future optional sync service is unused or unavailable.
- There is no account gate and no separate “community vs restricted” feature split in the core.

See [PHASE1.md](PHASE1.md) and [PHASE3.md](PHASE3.md) for the feature lists and [PROJECT_FORMAT.md](PROJECT_FORMAT.md) for on-disk layout.
