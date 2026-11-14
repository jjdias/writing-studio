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

### Track changes

- **Track changes** on the toolbar starts tracking the open scene. The project keeps a copy of the scene as it is at that moment: the baseline. Nothing else about writing changes.
- Every paragraph whose text now differs from the baseline is listed above the manuscript as Changed, Added, or Removed. A changed paragraph shows the words added and removed inside it.
- **Accept** keeps a change: the baseline takes that paragraph, and the change leaves the list. The text does not move.
- **Reject** undoes a change in the text: the baseline's paragraph comes back, with the formatting, notes, and highlights it had. A rejected addition is removed; a rejected removal is put back where it was. Reject is one undo step, so **Ctrl+Z** brings the change back.
- **Accept all** makes the baseline the text as it is now; tracking goes on from there. **Reject all** puts the whole scene back to the baseline, as one undo step.
- **Stop tracking** keeps the text as it is and discards the baseline.
- Click Changed, Added, or Removed on a row to select that paragraph in the manuscript.
- The baseline is in the project database, so tracked changes are still there after closing and reopening. Each scene is tracked on its own.
- Compile, word counts, search, and snapshots use the current text, with every change in it, whether accepted or not.

Limits:

- Only text is tracked. A paragraph whose words are unchanged is not listed, whatever happened to its bold, links, notes, or highlights.
- Changes are per paragraph. Two edits in one paragraph are accepted or rejected together. A list counts as one paragraph.
- A paragraph that moved is listed as removed in one place and added in another.
- The manuscript itself does not mark changed paragraphs; the list does.
- Tracking is turned on per scene. There is no switch for a whole book.
- A change has no author or date.
- Accept cannot be undone with Ctrl+Z; it does not change the text.

Where it lives: `writing-studio-document` (`tracked_changes`, `accept_change`, `reject_change`), built on the snapshot comparison. Baselines are rows in the new `tracking_baselines` table, added on open; the project format version is unchanged. Permanently deleting a scene deletes its baseline. Restoring a backup now also adds any tables the backup is missing.

### Spellcheck

- Unknown words are underlined in the manuscript and listed above it, in the order they first appear, with a count.
- Clicking a word selects it in the manuscript and shows up to six suggestions. Choosing one replaces every copy of that word in the open scene, as one undo step. With track changes on, the replacement is a tracked change.
- **Add to project dictionary** accepts a word everywhere in the project. The list is saved in the project settings and can be edited from the panel.
- Words in character, place, and culture names are accepted without being added. A possessive of a known word is accepted too.
- Numbers, single letters, and words in capitals are skipped. Curly and straight apostrophes are the same.
- English (US) is compiled into the program, so spellcheck works with no network and no system dictionary. Other Hunspell dictionaries are read from the project's `dictionaries/` folder and from the system's Hunspell folders. The status bar menu chooses one or turns spellcheck off. The choice is saved in the project. A dictionary that has gone missing falls back to English (US).
- The underline is drawn over the text and is not part of the scene. It is never saved or compiled.

Limits:

- One dictionary at a time. A scene in two languages will show the other language's words as unknown.
- A suggestion replaces every copy in the open scene, not one copy. There is no right-click menu on a word.
- The list and the replacement cover the open scene, not the whole book.
- A word that is half bold or half linked is found, but a suggestion cannot replace it; fix it by hand.
- Underlines use the CSS Custom Highlight API. A webview without it shows the list and no underlines.
- Dictionary files that are neither UTF-8 nor Latin-1 need converting to UTF-8 first.
- No grammar checking.

Where it lives: the new `writing-studio-spellcheck` crate, on the `spellbook` crate (a Rust reader for Hunspell dictionaries, MPL-2.0). The bundled dictionary is in `crates/spellcheck/dictionaries/` with its own license file; it is SCOWL-derived and is not under this repository's MIT OR Apache-2.0 terms. The editor side is `crates/ui/assets/spell.js`.

---

## Still to do in Phase 4

- Scene diagnostics (goal, conflict, outcome) as local analysis
- Optional local assistance later (on-device only); never required
