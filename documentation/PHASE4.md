# Phase 4 — what has shipped so far

Phase 4 is **local professional editing**: comparing versions, reviewing, tracking changes, spellcheck, and scene diagnostics. Nothing in this phase needs an account, a server, or a network connection.

The program stays **open source** (MIT OR Apache-2.0) and **free for the community**.

Phase 4 is in progress on the `phase-4` branch. How to use it: [USAGE.md](USAGE.md).

---

## Features delivered

### Snapshot comparison

- **History** → **Compare** on a snapshot shows what changed between that snapshot and the current text. The inspector's snapshot list has the same button; it opens History.
- **Compare with** switches the other side to a second snapshot of the same document. The older version is always on the left of the arrow.
- Paragraphs that only one side has are marked as added or removed. A paragraph that was rewritten shows the words that were added and removed inside it. A paragraph that kept less than 40% of its words is listed as one removed and one added paragraph.
- The summary line counts words added, words removed, and paragraphs changed.
- **Changed paragraphs only** hides the paragraphs that are the same on both sides.
- The current text is the text in the editor, including typing that is not saved yet.
- Comparing reads the snapshot. It does not change the document, and it does not write a snapshot.
- Choosing another binder item closes the comparison.

Limits:

- Only text is compared. A change that is only bold, italic, a link, an inline note, or a block type (paragraph to heading) does not show.
- A paragraph that moved shows as removed in one place and added in another.

Where it lives: the comparison is in `writing-studio-document` (`compare_documents`, `compare_text`). `ProjectStore::load_snapshot` reads a snapshot without restoring it. No new tables, and the project format version is unchanged.

---

## Still to do in Phase 4

- Review comments, annotations, and highlights beyond the inline note on a text run (Phase 2)
- Track changes / suggestions without a server
- Spellcheck integration
- Scene diagnostics (goal, conflict, outcome) as local analysis
- Optional local assistance later (on-device only); never required
