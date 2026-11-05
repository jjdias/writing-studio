# Writing Studio roadmap

Writing Studio is **open source** (MIT OR Apache-2.0) and **free for the community**. Anyone may use, study, modify, and share it. No account is required to write.

**Local-first, now and later.** Saves, snapshots, backups, and compile live on the writer’s machine. Cloud connections and collaboration, if added, stay **optional**. They must never become a requirement to open a project, write, or keep copies of work.

Status: Phase 1 desktop app is implemented on the `phase-1` branch (`v0.1.0`). Details: [`documentation/PHASE1.md`](documentation/PHASE1.md).

---

## Product principles

1. **Open and free for the community.** The full application stays open source. Optional future services must not lock manuscripts away from people who never connect.
2. **Local storage first.** Projects are folders (`*.writingstudio`) with SQLite. Backups are timestamped copies of that database under `backups/`.
3. **Never lose writing.** Transactional saves, WAL, crash-safe SQLite, and local backups come before sync.
4. **Shared Rust core.** UI, storage, compile, and later optional sync all consume the same document JSON and binder model.
5. **Offline writing is complete.** If a network, account, or server is unavailable, the studio still works.

---

## Done (Phase 1)

These pieces exist and are used by the desktop app. See [`documentation/PHASE1.md`](documentation/PHASE1.md) for the narrative list and [`documentation/USAGE.md`](documentation/USAGE.md) for how to use them.

### Workspace and tooling

- Cargo workspace with a shared core and a Dioxus desktop launcher (`apps/desktop`)
- Rust toolchain pin, bundled rusqlite (including the SQLite backup API)
- GitHub repository: [jjdias/writing-studio](https://github.com/jjdias/writing-studio)
- Notes under `documentation/` (Phase 1, usage, architecture, project format)
- Library tests for documents, editor, binder, storage, autosave, and compile

### Project format and local persistence

- Versioned project folder (`FORMAT` + `project.sqlite` + `backups/` + `attachments/`)
- SQLite schema: `project_meta`, `binder_nodes`, `documents`, `snapshots`
- WAL mode and transactional document writes
- Create / open project from the welcome screen (native folder picker)
- Save (Ctrl+S / Save) writes documents, binder metadata, and settings to disk
- Folder backups copy SQLite into `backups/`
- Per-document snapshots; restore from the inspector and History (`pre-restore` snapshot first)
- Backup restore from History (second click; safety copy of the live database first)
- Soft-delete tombstones and `restore_parent_id`
- UUIDs and RFC 3339 timestamps
- Timed local autosave (`autosave_seconds`, default 2s after the last edit)

### Manuscript organization

- Binder tree: manuscript, research, trash, book, chapter, scene, note
- Blank novel template
- Add chapter, scene, or note
- Drag-and-drop reorder and nesting; drop on Trash
- Trash, restore, empty trash (second-click confirm)
- Inspector: title, synopsis, status
- Word counts and project word goal
- Compile draft binder to local Markdown or DOCX

### Editor

- Structured document model (blocks, inlines, marks) stored as JSON
- Rich-text desktop editor mapped onto that model
- Undo / redo, dirty tracking, word and character counts
- Flush on blur, scene change, Ctrl+S, and idle autosave

### Workspace UI

- Welcome screen
- Writing, Corkboard, Outliner, History modes
- Binder filter, focus mode
- Corkboard scene cards; outliner table

### Crates that actually run

| Crate | Role |
| --- | --- |
| `writing-studio-core` | IDs, time, errors, events, format version, autosave policy |
| `writing-studio-document` | Block document model, JSON, HTML interchange |
| `writing-studio-editor` | Session, undo/redo, counts |
| `writing-studio-project` | Binder tree and settings |
| `writing-studio-manuscript` | Book / chapter / scene outline |
| `writing-studio-templates` | Blank novel seed |
| `writing-studio-storage` | Local SQLite, save, snapshots, backups |
| `writing-studio-corkboard` | Scene cards from the binder |
| `writing-studio-export` | Local Markdown and DOCX compile |
| `writing-studio-search` | Binder title filter |
| `writing-studio-ui` | Desktop workspace |
| `writing-studio-desktop` | Native window |

---

## Scaffolded but not built

These crates exist so module boundaries stay stable. They are **types or stubs**, not product features.

| Crate | Current state | Intended later work |
| --- | --- | --- |
| `writing-studio-plotting` | `Plotline` / `PlotPoint` structs | Plot Grid UI and persistence |
| `writing-studio-characters` | `Character` / relationship structs | Profiles, relationships, inspector |
| `writing-studio-worldbuilding` | `Location` / `TimelineEvent` structs | Locations, lore, timelines |
| `writing-studio-research` | `ResearchItem` struct | Attachments, PDFs, library UI |
| `writing-studio-collaboration` | Yrs placeholder constant | Optional CRDT editing |
| `writing-studio-synchronization` | “optional = true” stub | Optional cloud sync |
| `writing-studio-security` | Notes that local folders are unencrypted | Optional encryption when sync exists |

Binder labels and colors are editable in the inspector and shown on binder rows.

---

## Still to do

Later phases keep **local writing complete**. Anything involving accounts, servers, or the internet is deferred and optional.

### Phase 2 — Writing environment (local)

Done on `phase-2`:

- Inline links from the toolbar, comments, and more block types
- Outliner: inline metadata edit and word counts per row
- Binder labels, colors, and deeper nesting polish

Still in this phase:

- Corkboard: reorder scenes, filter/group, card colors
- Keyboard shortcuts beyond save / undo / redo
- Full-text search of manuscript content (Tantivy), not only binder titles
- Daily writing targets and session statistics stored in the local project
- Customizable panel layout and themes

### Phase 3 — Story development (local)

- Plot Grid: plotlines, plot points, links to scenes
- Character profiles, relationships, motivations
- Locations, cultures, lore, timelines
- Research library: notes plus files in `attachments/`
- Series bible shared across local projects (still files on disk)

### Phase 4 — Professional editing (local)

- Snapshot comparison
- Comments, annotations, highlights
- Track changes / suggestions without a server
- Spellcheck integration
- Scene diagnostics (goal, conflict, outcome) as local analysis
- Optional **local** assistance later (on-device only); never required

### Phase 5 — Optional cloud

Do this only after local save/backup/restore is trustworthy. Sync remains opt-in.

- Optional account and Axum backend
- Opt-in sync of an already-local project
- Conflict rules for binder structure, deletions, and attachments
- Real-time collaboration via Yrs / WebSockets
- Shared-project permissions
- Encrypt traffic; credentials never gate the manuscript
- Writers who never connect remain on the same project format

### Phase 6 — Publishing (local)

- Export profiles independent of the editor widget
- PDF, EPUB, HTML, and richer compile templates (Markdown and DOCX already ship)
- Front matter, compile order, formatting templates
- Manuscript validation before compile

### Phase 7 — Cross-platform release

- Packaged Windows, macOS, and Linux builds
- Later: Android, iOS, and Dioxus Web/WASM
- Accessibility and performance passes
- Automated community builds

---

## Keep local writing unblocked

Do **not** require any of these to write or compile:

- User accounts or login
- A network connection
- Cloud backup as the only copy
- Simultaneous editing across machines
- Server-side search or storage as the source of truth

When optional cloud work starts, the source of truth for a disconnected writer remains the `.writingstudio` folder on disk.

---

## Immediate next steps

Phase 2 is in progress on `phase-2`. Links, comments, extra block types, the outliner, and binder labels, colors, and deeper nesting are in. Still to implement, all local:

- Corkboard: reorder scenes, filter/group, card colors
- Keyboard shortcuts beyond save / undo / redo
- Full-text search of manuscript content (Tantivy), not only binder titles
- Daily writing targets and session statistics stored in the local project
- Customizable panel layout and themes

Optional cloud stays later.
