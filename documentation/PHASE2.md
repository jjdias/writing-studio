# Phase 2 — what shipped

Phase 2 is the **local writing environment** around the Phase 1 studio: search, targets, layout, themes, and a richer manuscript. Nothing in this phase needs an account, a server, or a network connection.

The program stays **open source** (MIT OR Apache-2.0) and **free for the community**.

This work landed on `phase-2`. How to use it: [USAGE.md](USAGE.md). On-disk layout: [PROJECT_FORMAT.md](PROJECT_FORMAT.md).

---

## Goals that Phase 2 met

1. **Stay in the manuscript.** Links, headings, lists, code, scene breaks, and a comment on a selection are stored in the same document JSON.
2. **Find text.** Search covers binder titles and scene text. The index lives in memory and is rebuilt from the project when it opens.
3. **See the work.** The outliner edits metadata in place. The corkboard reorders scenes and groups cards. Labels and colors mark binder rows.
4. **Remember the desk.** Daily progress, sittings, theme, and panel layout are saved in the project.

---

## Features delivered

### Manuscript

- Toolbar: Bold, Italic, Underline, Strike, Link, Unlink, Comment, Uncomment, H1, H2, H3, Body, Quote, List, Numbered, Code, and Break.
- A comment is a note on the selected text. It is stored on that run (`marks.comment`) and listed above the manuscript. Uncomment removes it. The note is saved with the scene.
- These notes are part of the document. Review annotations, highlights, and track changes are still Phase 4. See [ROADMAP.md](../ROADMAP.md).

### Binder, corkboard, and outliner

- Inspector label and color on items that are not roots. Colors are None, Gold, Green, Red, Blue, and Violet. The label shows on the binder row and on the corkboard card.
- Corkboard: drag a card onto another scene to place it just after that scene. Filter the cards. Group them by order, chapter, label, color, or status.
- Outliner: edit chapter title, scene title, status, and synopsis in the row. Each row shows its word count. **Open** returns to Writing.

### Search

- The binder field searches titles and manuscript text.
- `writing-studio-search` keeps a Tantivy index in memory. The project folder stays the source of truth. The index is rebuilt when a project opens and updated when a document is saved.

### Targets, layout, and theme

- Daily word target and project word target, edited from the status bar and stored in the project.
- Today and the current sitting. **History** lists completed sittings. A sitting is stored the next time the project opens, when words were added. Today starts again on the next local day. Phase 3 later gave each book its own baselines. See [PHASE3.md](PHASE3.md).
- Panel layout in the project: binder on the left or right, drag widths, and showing or hiding the binder and inspector. Double-click a sash to reset that width.
- Theme in the project: Dark, Light, or Paper.

### Shortcuts

Chords beyond save, undo, and redo: search, zen mode, workspace modes, new scene, new chapter, open project, snapshot, and the binder and inspector toggles. The list is in [USAGE.md](USAGE.md).

---

## Explicitly not in Phase 2

- One project for a series, the bible, and the research library (Phase 3; see [PHASE3.md](PHASE3.md))
- Review annotations, highlights, snapshot comparison, track changes, and spellcheck (Phase 4). Inline comments from this phase stay as they are.
- Cloud sync, accounts, or real-time collaboration (Phase 5)
