# Writing Studio roadmap

Writing Studio is **open source** (MIT OR Apache-2.0) and **free for the community**. Anyone may use, study, modify, and share it. No account is required to write.

**Local-first, now and later.** Saves, snapshots, backups, and compile live on the writer’s machine. Cloud connections and collaboration, if added, stay **optional**. They must never become a requirement to open a project, write, or keep copies of work.

Status: Phases 1, 2, and 3 are complete. Phase 4 is in progress on the `phase-4` branch; snapshot comparison and review notes and highlights are done. Details: [`documentation/PHASE1.md`](documentation/PHASE1.md), [`documentation/PHASE2.md`](documentation/PHASE2.md), [`documentation/PHASE3.md`](documentation/PHASE3.md), and [`documentation/PHASE4.md`](documentation/PHASE4.md).

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
- Notes under `documentation/` (Phase 1 through Phase 4, usage, architecture, project format)
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

## Crates in the desktop app

These crates run in the current app. Phase 1 did not include plotting, characters, worldbuilding, or the research library. Search was a binder title filter in Phase 1 and is a full-text index now.

| Crate | Role |
| --- | --- |
| `writing-studio-core` | IDs, time, errors, events, format version, autosave policy |
| `writing-studio-document` | Block document model, JSON, HTML interchange, text comparison |
| `writing-studio-editor` | Session, undo/redo, counts |
| `writing-studio-project` | Binder tree and settings |
| `writing-studio-manuscript` | Book / chapter / scene outline |
| `writing-studio-templates` | Blank novel seed |
| `writing-studio-storage` | Local SQLite, save, snapshots, backups |
| `writing-studio-corkboard` | Scene cards from the binder |
| `writing-studio-export` | Local Markdown and DOCX compile |
| `writing-studio-plotting` | Plotlines, plot points, and scene links |
| `writing-studio-characters` | Profiles, motivations, and relationships |
| `writing-studio-worldbuilding` | Locations, cultures, lore, and timelines |
| `writing-studio-research` | Research notes and files in `attachments/` |
| `writing-studio-search` | In-memory full-text index of manuscript text, rebuilt when a project opens |
| `writing-studio-ui` | Desktop workspace |
| `writing-studio-desktop` | Native window |

---

## Scaffolded but not built

These crates exist so module boundaries stay stable. They are **types or stubs**, not product features.

| Crate | Current state | Intended later work |
| --- | --- | --- |
| `writing-studio-collaboration` | Yrs placeholder constant | Optional CRDT editing |
| `writing-studio-synchronization` | “optional = true” stub | Optional cloud sync |
| `writing-studio-security` | Notes that local folders are unencrypted | Optional encryption when sync exists |

---

## Done (Phase 2 and Phase 3)

### Phase 2 — Writing environment (local)

Done. See [`documentation/PHASE2.md`](documentation/PHASE2.md).

- Inline links from the toolbar, and more block types (headings, code, scene break)
- Inline comments: a note on a text run, stored in the document JSON. Phase 4 added note kinds, resolving, and highlights. Track changes stays in Phase 4
- Outliner: inline metadata edit and word counts per row
- Binder labels, colors, and deeper nesting polish
- Corkboard: reorder scenes, filter/group, card colors
- Keyboard shortcuts beyond save / undo / redo
- Full-text search of manuscript content (Tantivy, in memory), not only binder titles
- Daily writing targets and session statistics stored in the local project
- Customizable panel layout, stored in the project: binder side, drag widths, and showing or hiding the binder and inspector
- Themes stored in the project: Dark, Light, and Paper

Phase 2 is complete.

### Phase 3 — Story development (local)

Done. See [`documentation/PHASE3.md`](documentation/PHASE3.md).

One project holds the bible and every related book. Compile, the word target, today, and the sitting follow one book. The binder, corkboard, outliner, and search can show that book or the whole series. Research notes are edited only in Research.

Phase 3 is complete.

## Still to do

Later phases keep **local writing complete**. Anything involving accounts, servers, or the internet is deferred and optional.

### Phase 4 — Professional editing (local)

In progress on the `phase-4` branch. See [`documentation/PHASE4.md`](documentation/PHASE4.md).

Done:

- Snapshot comparison. **History** shows what changed between a snapshot and the current text, or between two snapshots of the same document: paragraphs added and removed, and words changed inside a rewritten paragraph. It compares text only, and it writes nothing.

- Review notes and highlights. A note on a passage has a kind (Comment, Question, Revise, Check) and can be resolved, reopened, edited, or removed from a list above the manuscript. Text can be highlighted in four colors. Both are stored on the text in the scene. Compile leaves out resolved notes and highlights.

Still to do:

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
- Accessibility and performance passes for the packaged apps. The desktop writing surface is Phase 8
- Automated community builds

### Phase 8 — Writing surface (local)

The desktop window already runs. This phase changes that window. It is not Phase 4, and it does not wait for the packaged builds.

- Dark and Light give the manuscript a page: a reading font, a limited line length, and a surface that is not the same color as the panels. Paper is the model.
- The activity bar shows the mode names. Ctrl+1 through Ctrl+8 follow the icon order: Writing, Corkboard, Outliner, Plot, Characters, World, Research, History.
- Choosing the mode that is already open does not hide the binder. Hiding the binder stays on its own control.
- Save, Backup, and Compile leave the tab strip and sit on a toolbar. The tab is the scene name.
- The format toolbar shows which marks are on. Link and Comment ask in a small field in the window. The manuscript keeps the caret and the active formatting.
- The status bar leads with the compile book's count, with today and the sitting beside it. The theme moves to a menu. The bar grows instead of clipping. Autosave is its own line.
- Chapter and Scene stay above the binder. Book, Part, and Folder move into a menu. The kind mark becomes an icon so the title has the room.
- A second click that deletes or restores names the item. Escape cancels that confirmation.
- Icon buttons show focus and have a name, not only a tooltip. The window can list its shortcuts.

### Bonus — One project per book, and a bible project beside it

Not current work. The series layout in use is one project that holds the bible and every related book. This bonus keeps the other layout available later. The bible would still exist in one place, as files on disk, and would not be copied into each book.

The bible is its own `.writingstudio` folder. Each novel stores a path to that folder. The book project keeps the manuscript, snapshots, and backups. Characters, world notes, plot, and research are read from the bible and written back there.

- Store a local path from a book project to one bible folder. The path is a folder, not an account.
- Read and edit characters, world notes, plot, and research in that folder.
- If the bible folder is missing or moved, the book still opens and the manuscript stays writable. The bible panel says the folder is unavailable.
- Open the bible next to the manuscript. A scene can point at a character, place, or note in the bible.

### Bonus — Keep research files with a backup

Not current work. **Backup** copies the SQLite file and leaves `attachments/` behind. Restoring an older backup can leave research rows pointing at files that are gone, or leave files on disk that the restored library no longer lists.

- Include `attachments/` in a project backup, beside the SQLite copy.
- Restore the database and those files together.
- After restore, every research file row has its file on disk, and files that belonged only to the replaced library are not left behind as the current library.

### Bonus — Tags

Not current work. The binder label stays one short label for the corkboard. Tags are a separate list of keywords on an item, stored in the project database. They are not typed into the scene, so compile does not write them into the draft.

The model is the same idea as a note app’s tags: many tags on one item, nested with `/`, and a click that filters to the items that carry that tag.

- Add and remove tags from the inspector. Suggestions come from tags already used in the project.
- A tag allows letters, numbers, `_`, `-`, `/`, and common Unicode. Spaces are not allowed. A tag that is only digits is not allowed. `/` nests a tag, as in `plot/subplot`.
- Matching ignores case. The list shows the spelling from the first time that tag was used.
- Choosing a parent tag also shows items tagged with its children. `plot` includes `plot/subplot`.
- A tag list in the binder column shows each tag, nested, with a count of items that are not in the trash. Choosing one filters the binder.
- Compile, word counts, and the outliner ignore tags.
- Emptying trash drops tags on the deleted items. A tag that nothing uses anymore leaves the list.
- Start with binder items. Research notes can use the same list. Characters and world records can follow later.

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

Current work is Phase 4, local professional editing, on the `phase-4` branch. Snapshot comparison and review notes and highlights are done. Next is track changes; then spellcheck and scene diagnostics. The writing-surface UI is Phase 8. Richer front matter than the book title stays in Phase 6. A separate bible project beside each book, backups that include research files, and tags stay bonus work.

Optional cloud stays later.
