# Phase 1 — what shipped

This file records **what Phase 1 shipped**. Later phases changed the window and some of these crates. For the current app, use [USAGE.md](USAGE.md). Phase 2 is [PHASE2.md](PHASE2.md). Phase 3 is [PHASE3.md](PHASE3.md). Current work is Phase 4 in [ROADMAP.md](../ROADMAP.md).

Phase 1 is a **usable local writing studio**: create a project on disk, organize a manuscript, write scenes, recover work, and compile a draft. Nothing in this phase needs an account, a server, or a network connection.

The program is **open source** (MIT OR Apache-2.0) and **free for the community** to use, study, change, and share.

Branch work for this phase lives on `phase-1` (see the GitHub repository). The desktop crate is `writing-studio-desktop`.

---

## Goals that Phase 1 met

1. **Local-first.** A project is a folder you own (`*.writingstudio`). SQLite holds structure and prose.
2. **Shared Rust core.** The Dioxus UI does not own the document schema. Storage, the editor session, and compile all use the same JSON documents and binder tree.
3. **Do not lose writing.** WAL, transactional document saves, idle autosave, snapshots, and folder backups.
4. **Compile without the editor widget.** Markdown and DOCX are built from binder order plus stored documents.

---

## How to run it

Requirements: Rust stable (1.80+), Microsoft C++ Build Tools on Windows (bundled SQLite), WebView2.

```bash
cargo run -p writing-studio-desktop
```

On the welcome screen, set a title and choose a parent folder. The app creates `YourTitle.writingstudio/`. Open that folder later with **Open project**.

Library tests (no window):

```bash
cargo test --workspace --exclude writing-studio-desktop --exclude writing-studio-ui
```

---

## Workspace

| Mode | What it is |
| --- | --- |
| **Writing** | Binder, rich-text manuscript, inspector |
| **Corkboard** | Scene cards (title, synopsis, status, word count) |
| **Outliner** | Chapter/scene table |
| **History** | Document snapshots and project backups |

Also, in this phase: a binder title filter, focus mode (hides binder and inspector), and a status bar with word counts and unsaved/autosave state.

Top bar in this phase: Save, Backup, Markdown, DOCX, Focus.

`writing-studio-search` was a binder title filter in Phase 1. It now keeps an in-memory full-text index of manuscript text. See [PHASE2.md](PHASE2.md). The current tab bar is Save, Backup, the book being compiled, Markdown, and DOCX. Zen mode is on the activity bar. See [USAGE.md](USAGE.md).

---

## Features delivered

### Projects

- Create and open a versioned project folder (`FORMAT` + `project.sqlite` + `backups/` + `attachments/`).
- Settings stored in `project_meta.settings_json`: daily/project word goals, `compile_title_page`, `autosave_seconds` (default 2).
- Blank novel template: Manuscript, Research, Trash, a book, Chapter 1, Scene 1, and a research note. The seed still creates that note. Opening a project now copies it into the research library and removes it from the binder. See [PHASE3.md](PHASE3.md).

### Binder

- Hierarchy: manuscript, research, trash, book, part, chapter, scene, note, folder.
- Add chapter, scene, or note from the toolbar. The binder tools are now Book, Part, Chapter, Scene, and Folder. See [USAGE.md](USAGE.md).
- Inspector: title, synopsis, status. Label and color came in Phase 2.
- Drag-and-drop: reorder siblings, nest into a valid parent, drop onto Trash.
- Trash: move, restore to the previous parent when it still exists, empty trash (second click; deletes those rows and documents on disk).
- Soft-delete tombstones (`deleted_at`) plus `restore_parent_id`.
- Corkboard and word counts ignore trashed items.

### Editor

- Structured blocks: paragraph, heading, quote, list; inlines with bold, italic, underline, strikethrough, optional link.
- Desktop rich-text view (HTML interchange) mapped back onto that model. Toolbar in this phase: Bold, Italic, Underline, Strike, Heading, Body, Quote, List, Numbered.
- Links, inline comments, code blocks, and scene breaks came in Phase 2 and are stored on the same document JSON. An inline comment is a note on a text run. Review annotations, highlights, and track changes remain Phase 4. See [PHASE2.md](PHASE2.md).
- Undo/redo (Ctrl+Z / Ctrl+Y) against the document session; Ctrl+S and blur still flush.
- Timed autosave after idle (`autosave_seconds`). Status shows **Unsaved** then **Autosaved locally.**
- Per-scene word and character counts; project total vs goal.

### Recovery

- **Snapshot this document** in the inspector; list and restore in the inspector and History. Restore writes a `pre-restore` snapshot of the current text first.
- **Backup** copies SQLite into `backups/`. History lists those files. Restore requires a second click and writes a safety copy of the live database first.

### Compile

- **Markdown** and **DOCX** in the top bar. Save dialog; files go wherever you choose.
- Compile walks the **draft/manuscript** binder only (not Research, not Trash).
- Formatting comes from the stored block model, not from the live editor widget.
- Optional title page from `compile_title_page`.

---

## Data on disk

See [PROJECT_FORMAT.md](PROJECT_FORMAT.md). Each scene or note is its own document row (JSON). Never one giant manuscript field.

---

## Crates that run in Phase 1

| Crate | Role |
| --- | --- |
| `writing-studio-core` | IDs, time, errors, events, autosave policy, format constants |
| `writing-studio-document` | Block model, JSON, HTML interchange |
| `writing-studio-editor` | Session, undo/redo, HTML/plain apply, counts |
| `writing-studio-project` | Binder tree, move/trash/restore, settings |
| `writing-studio-manuscript` | Outline for the outliner |
| `writing-studio-templates` | Blank novel seed |
| `writing-studio-storage` | SQLite, snapshots, backups, restore, load-all for compile |
| `writing-studio-corkboard` | Scene cards |
| `writing-studio-export` | Markdown and DOCX compile |
| `writing-studio-search` | Binder title filter in this phase. The crate now indexes manuscript text in memory |
| `writing-studio-ui` | Dioxus workspace |
| `writing-studio-desktop` | Native window |

At the end of Phase 1, plotting, characters, worldbuilding, and research were still placeholders. Phase 2 extended search, the editor, the corkboard, and the outliner. Phase 3 built the bible. Collaboration, synchronization, and security remain stubs.

---

## Tests that cover Phase 1

- Document plain-text and HTML round-trip, including marks and lists
- Editor undo and HTML bold
- Binder reorder, trash/restore, illegal drops, purge
- Scene save/reload, snapshot restore, backup restore
- Autosave idle policy
- Markdown compile (marks, skip research), DOCX zip package, draft-only walk

---

## Explicitly not in Phase 1

- Full-text search, labels and colors, corkboard grouping, an editable outliner, daily sittings, themes, panel layout, links, code blocks, scene breaks, and inline comments (shipped in Phase 2; see [PHASE2.md](PHASE2.md))
- Plot Grid, character/worldbuilding UIs, and the research file library (shipped in Phase 3; see [PHASE3.md](PHASE3.md))
- Review annotations, highlights, and track changes (Phase 4). Phase 2 inline comments are notes on a text run, already stored with the scene
- Cloud sync, accounts, or real-time collaboration
- PDF, EPUB, HTML compile (Markdown and DOCX are enough for this phase)
- Packaged installers for macOS/Linux/mobile
- On-device AI

Phase 2 and Phase 3 have since shipped. Cloud sync, packaged installers, PDF/EPUB/HTML compile, on-device AI, and the Phase 4 review tools remain later work. They must not become required to write.
