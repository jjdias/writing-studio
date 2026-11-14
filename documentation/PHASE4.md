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

### Review notes and highlights

Phase 2 could put one plain note on a text run. This builds on that field.

- **Note** puts a note on the selected text. A note has a kind: Comment, Question, Revise, or Check. The kind changes the underline in the manuscript.
- A note can be **resolved** and reopened. A resolved note stays on the text, dimmed.
- **Yellow**, **Green**, **Blue**, and **Pink** highlight the selected text. A highlight needs no note, and text can carry both.
- A selection that crosses paragraphs is marked in each paragraph. Paragraphs are never moved inside a note or a highlight.
- A list above the manuscript shows every note and highlight in the open scene with the text it sits on. Click that text to select it in the manuscript. From the list: change a note's kind, edit its text, resolve or reopen it, or remove it. **Show resolved** lists resolved notes; **Remove resolved** takes them all off the scene.
- Text that is partly bold or linked under one note is one entry in the list, not one per run.
- Removing a note or highlight leaves the text. Each change from the list is one undo step.
- Compile writes open notes as before (footnotes in Markdown, bracketed text in DOCX), with the kind in front when it is not Comment. Resolved notes and highlights are left out.

Limits:

- The list covers the open scene. There is no list of open notes across a whole book yet.
- A note has no author or date. This is one writer's project on one machine.
- A note that crosses paragraphs is one entry per paragraph.
- Changing a note from the list redraws the manuscript, so the caret returns to the start.
- **Note** still asks in a browser prompt. The in-window field is Phase 8.

Where it lives: `marks.comment_kind`, `marks.comment_resolved`, and `marks.highlight` in the document JSON, beside the Phase 2 `marks.comment`. Older documents load unchanged and `schema_version` is still 1. See [PROJECT_FORMAT.md](PROJECT_FORMAT.md). `Document::review_items` lists them. The editor's selection handling is in `crates/ui/assets/review.js`.

---

## Still to do in Phase 4

- Track changes / suggestions without a server
- Spellcheck integration
- Scene diagnostics (goal, conflict, outcome) as local analysis
- Optional local assistance later (on-device only); never required
