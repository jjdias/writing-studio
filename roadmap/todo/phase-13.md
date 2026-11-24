# Phase 13 — Context at every level

In progress: stages 1 to 4 are done. Stage 5 waits for the maintainer's choice. It comes after [Phase 12](../done/phase-12.md), whose stage 8 makes every text file of the project Markdown with front matter.

A scene, a chapter, a part, and a book each have a **context** and a **body**. The context is what the writer knows about that level: its synopsis, keywords, status, goal, and notes, and what it is linked to. The body is its text: a scene's prose, a chapter's own text if it has any. For a chapter, part, or book, what it holds (its scenes or chapters, each with its synopsis) is part of what is read about it, in order.

The point is that anyone reading the project, a person or an AI, can understand each level from its context without reading everything under it: the book from its chapters' synopses, a chapter from its scenes', and a scene from its own context and its text. That is how a long book fits in what an AI can read at once.

## Where things are now

- A scene's file has its context in its header (synopsis, status, goal, conflict, outcome) and its text in its body; after Phase 12 stage 8 it is a `.md` file with front matter.
- A chapter has a file only when it holds text of its own. A book, a part, a folder, and a chapter without text have their title, synopsis, and status in `binder.json`.
- Every item's keywords are in `binder.json`. Links to characters, places, and notes are in `bible.json`, plot points in `plot.json`.
- `crates/context` builds a scene's context (Phase 11): where it sits, its keywords with those of what it is in, the scenes around it, its plot points, and the entries linked to it, as Markdown, capped to a length. Context shows it, and **Copy context** puts it on the clipboard.

## The rules this phase keeps

From the maintainer's decision on this idea:

- **What a level holds is not written in its file.** The order and nesting of the book are in `binder.json` alone. A chapter's list of scenes is made from the binder when it is read, never stored: a file that listed its children would have to be rewritten on every move in the binder, and two machines would disagree about it when they sync.
- **Links are named, not copied.** A scene's context names the characters and plot points linked to it; their text is read from `bible.json`, `plot.json`, and the entries' own files when the context is made, so it is never out of date.
- **The files stay flat.** Scenes stay in `manuscript/`, not in a folder for each chapter: moving a scene does not move its file, so git history stays readable and a move on one machine does not fight an edit on another.

## The work

### 1. A file for every level

- A book, a part, and a chapter without text get a `.md` file in `manuscript/`, as a chapter with text has: its context as front matter, and an empty body. A folder in the binder gets one too, for its notes.
- `binder.json` keeps only the structure: each item's `id`, `kind`, `parent`, order, `file`, whether it is left out of compile, and whether it is in the trash. Title, synopsis, status, and color move to the files of the items that did not have one.
- **Keywords move into the files**, as `tags:`, as the Markdown export already writes them and Obsidian reads them. The keyword list with its colors stays in `binder.json`.
- Sync merges these as it merges a scene's header today, key by key.

**Done.** Every binder item but the three roots has a file in `manuscript/` (`has_own_file`): a book, a part, or a folder has its title, synopsis, status, label, color, and keywords there, and no text; what a person writes under its front matter is kept and written back (`Extras::body`). Keywords are `tags:` in each item's front matter, spelled exactly as they are, and `binder.json` keeps only where each item is. A project written by Phase 12 (books without files, keywords on the binder lines) still opens, and its next save moves them; so this stays format 3. Sync needed nothing: it merges the items, not the files. Tests: `a_book_has_a_file_of_its_details_and_a_move_changes_only_the_binder` (a book's file, its kept text, and a move that writes only `binder.json`), and the keyword test, now on the files' `tags`.

### 2. Notes for the writer at every level

- `notes:` in the front matter of any file: what the writer wants to remember about this scene, chapter, part, or book ("the reader should not trust Mara yet"). Several lines.
- Edited in Context, for whatever is chosen. Never compiled, never counted.

**Done.** `BinderNode::notes`, kept as `notes:` in the item's front matter (several lines under `|-`), in the cache's new `binder_nodes.notes` column (added to an older cache when it opens), and merged by sync with the item's other details. Context has a **Notes** field under the keywords for any item but a root. Notes are not in the document, so they are never compiled or counted. Test: `notes_on_any_level_are_kept_in_its_file`.

### 3. The context of a chapter, a part, and a book

In `crates/context`, beside `scene_context`:

- Its own context: title, synopsis, status, goal, keywords (with those of what it is in), notes, and word count and target.
- Where it sits: the book, the part.
- What it holds, in order: each scene or chapter with its title, status, and synopsis. For a book, its chapters, and under each its scenes' titles; a chapter's scenes with their synopses. This is the rolling summary: each level is read from the level below's synopses, not its text.
- The entries linked to it or to anything in it, named, with how often they appear; and the plotlines and plot points that run through its scenes.
- As Markdown, capped to a length like the scene's: when it is too long, the deepest synopses go first, then the entries' text, and what was left out is said.

**Done.** `level_context` in `crates/context`, for a chapter, a part, a book, or a folder: its kind, title, status, synopsis, notes, keywords (with those of what it is in), and place; an outline of what it holds and what that holds, each with its status and synopsis; the plot points of its scenes; and every entry linked to it or reaching one of its scenes, with how many scenes it is in. `LevelContext::to_markdown` caps it like a scene's: the deepest synopses go first, then the others, then the entries' text from the end, and what was left out is said. A scene's context now has its notes too. Word counts are not in it: the crate reads the models, not the text, and the window shows them. Test: `a_book_and_a_chapter_are_read_from_the_synopses_below_them`.

### 4. Context in the window

- Context shows the context of a chosen chapter, part, or book, as it shows a scene's, with its notes to edit.
- **Copy context** for any level, as for a scene.
- The mind map's chosen node shows the same.

**Done.** Context, for a chosen chapter, part, book, or folder in the manuscript, shows its outline with each synopsis and the entries in it with how many of its scenes each is in, and **Copy context** puts `LevelContext::to_markdown` on the clipboard (`LevelContextView`, `StudioState::level_context`). The mind map's chosen node is the chosen item, so it shows the same. Context's heading names the item's kind. Its notes are edited above (stage 2). Test: the scene context's state test also reads its chapter's.

### 5. Context files for other tools (to decide first)

- The project's files are made for reading, but a level's whole context (what it holds, its links) is made, not stored. A tool that reads folders, such as an AI working in the project, would need the program to give it that.
- The choice: a setting, off until chosen, that keeps `.local/context/` up to date, one `.md` file of context for each level, rebuilt when something it reads is saved. `.local/` is this machine's only and never synced, so these files never conflict and never go stale on another machine.
- The other way is to leave it to the optional local assistance in [bonus.md](bonus.md), which asks `crates/context` directly.

## Format

This changes the project's files again: books, parts, and folders have files, and keywords move out of `binder.json`. If it is merged before a release with Phase 12's format 3, it stays format 3, and the two changes are one; otherwise it is format 4, and the rule on older formats in [../README.md](../README.md) applies, with the caution in Phase 12 stage 8 about projects people already have.

## How changes are checked

- Every level's file is written and read back with its front matter, keywords, and notes, and keys this program does not know are kept.
- A move in the binder changes `binder.json` and no other file.
- Each level's context, and its Markdown at several caps, is tested in `crates/context` against a small book, as the scene's context is.
- Sync merges two machines' edits to one file's front matter, key by key.
