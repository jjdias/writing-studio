# Writing Studio roadmap

Writing Studio is a **free, open-source** writing environment (MIT OR Apache-2.0). Anyone can use, study, modify, and share it. There is no paid edition and no account required to write.

**Local-first, now and later.** Saves, snapshots, and backups live on the writer’s machine. Cloud connections and collaboration will be added only as **optional** features. They must never become a requirement to open a project, write, or keep copies of work.

Status of this file matches the tree as of the Phase 1 desktop app (`v0.1.0`, commit on `master`).

---

## Product principles

1. **Free and open.** The full application stays open source. Future cloud connectors stay optional and must not lock manuscripts behind a service.
2. **Local storage first.** Projects are folders (`*.writingstudio`) with SQLite. Backups are timestamped copies of that database under `backups/`.
3. **Never lose writing.** Transactional saves, WAL, crash-safe SQLite, and local backups come before sync.
4. **Shared Rust core.** UI, storage, and later cloud all consume the same document JSON and binder model.
5. **Cloud is a later add-on.** Offline writing remains complete if a network, account, or server is unavailable.

---

## Done (Phase 1)

These pieces exist and are used by the desktop app.

### Workspace and tooling

- Cargo workspace with a shared core and a Dioxus desktop launcher (`apps/desktop`)
- Rust 1.99 toolchain pin, bundled rusqlite (including the SQLite backup API)
- GitHub repository: [jjdias/writing-studio](https://github.com/jjdias/writing-studio)
- Architecture and project-format notes under `documentation/`
- Library tests for document round-trip, undo, and scene storage

### Project format and local persistence

- Versioned project folder (`FORMAT` + `project.sqlite` + `backups/` + `attachments/`)
- SQLite schema: `project_meta`, `binder_nodes`, `documents`, `snapshots`
- WAL mode and transactional document writes
- Create / open project from the welcome screen (native folder picker)
- Save (Ctrl+S / Save button) writes documents, binder metadata, and project settings to disk
- Folder backups copy SQLite into `backups/` without replacing the live file until the copy succeeds
- Per-document snapshots (inspector “Snapshot this document”)
- Snapshot restore from the inspector and History mode (previous text kept as a `pre-restore` snapshot)
- Backup restore from History mode (second click to confirm; a safety copy of the live database is written first)
- Soft-delete tombstones on binder nodes (`deleted_at`)
- UUIDs and RFC 3339 timestamps

### Manuscript organization

- Binder tree: manuscript, research, trash, book, chapter, scene, note
- Blank novel template seeds a first chapter, scene, and research note
- Add chapter, scene, or note from the binder toolbar
- Drag-and-drop reorder and nesting in the binder (drop on Trash to discard)
- Trash, restore to the previous parent, and empty-trash with a second-click confirm
- Inspector: title, synopsis, status
- Word counts for the current document and the whole project; project word goal is stored in settings (default 80,000)

### Editor

- Structured document model (blocks, inlines, marks) stored as JSON
- Rich-text editor maps HTML in the desktop view onto that block model (bold, italic, underline, strike, headings, quotes, lists)
- Undo / redo (Ctrl+Z / Ctrl+Y), dirty tracking, word and character counts
- Flush on blur, on scene change, and on a local idle timer (`autosave_seconds`, default 2s after the last edit)

### Workspace UI

- Welcome screen and three modes: Writing, Corkboard, Outliner
- Binder filter (title substring)
- Focus mode (hides binder and inspector)
- Corkboard cards from scenes (synopsis, status, word count); click opens writing mode
- Outliner table of chapters and scenes

### Crates that actually run

| Crate | Role |
| --- | --- |
| `writing-studio-core` | IDs, time, errors, events, format version |
| `writing-studio-document` | Block document model and JSON |
| `writing-studio-editor` | Session, undo/redo, counts |
| `writing-studio-project` | Binder tree and settings |
| `writing-studio-manuscript` | Book / chapter / scene outline |
| `writing-studio-templates` | Blank novel seed |
| `writing-studio-storage` | Local SQLite, save, snapshots, backups |
| `writing-studio-corkboard` | Scene cards from the binder |
| `writing-studio-export` | Plain-text manuscript concatenation (library only, no UI) |
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
| `writing-studio-security` | Notes that local folders are unencrypted | Optional encryption / credentials when cloud exists |

Also not in the UI yet: export commands, binder labels/colors.

---

## Still to do

Phases below keep **local writing complete**. Anything involving accounts, servers, or the internet is deferred and marked optional.

### Phase 2 — Writing environment (local)

- Inline links from the toolbar, comments, and more block types
- Binder labels, colors, and deeper nesting polish
- Corkboard: reorder scenes, filter/group, card colors
- Outliner: inline metadata edit and word counts per row
- Keyboard shortcuts beyond save / undo / redo
- Full-text search of manuscript content (Tantivy), not only binder titles
- Daily writing targets and session statistics stored in the local project
- Restore a snapshot; browse and restore local backups
- Customizable panel layout and themes

### Phase 3 — Story development (local)

- Plot Grid: plotlines, plot points, links to scenes
- Character profiles, relationships, motivations
- Locations, cultures, lore, timelines
- Research library: notes plus files in `attachments/`
- Series bible shared across local projects (still files on disk)

### Phase 4 — Professional editing (local)

- Snapshot comparison and restore
- Comments, annotations, highlights
- Track changes / suggestions without a server
- Spellcheck integration
- Scene diagnostics (goal, conflict, outcome) as local analysis
- Optional **local** AI later (on-device only); no cloud AI requirement

### Phase 5 — Optional cloud (must not be required)

Do this only after local save/backup/restore is trustworthy.

- Optional account and Axum backend
- Opt-in sync of an already-local project
- Conflict rules for binder structure, deletions, and attachments (CRDTs for prose are not enough)
- Real-time collaboration via Yrs / WebSockets
- Permissions for shared projects
- Encrypt traffic; treat credentials as optional, never as a gate to the manuscript
- Writers who never connect remain on the same project format

### Phase 6 — Publishing (local)

- Export profiles independent of the editor widget
- DOCX, PDF, EPUB, HTML, Markdown
- Front matter, compile order, formatting templates
- Manuscript validation before compile

### Phase 7 — Cross-platform release

- Packaged Windows, macOS, and Linux builds
- Later: Android, iOS, and Dioxus Web/WASM
- Accessibility and performance passes
- Automated builds; still no license wall

---

## Explicitly out of scope until later (cloud)

Do **not** block local writing on any of these:

- User accounts, login, or license checks
- Mandatory cloud backup
- Simultaneous editing across machines
- Server-side search or storage as the source of truth
- Paid sync or “pro” feature gates

When cloud work starts, the source of truth for a disconnected writer remains the `.writingstudio` folder on disk.

---

## Immediate next steps

Highest value after Phase 1, still fully local:

1. Compile to Markdown / DOCX from the current export crate

Cloud sync and collaboration stay off this near-term list on purpose.
