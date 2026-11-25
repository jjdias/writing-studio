# Phase 15 — Free context

Done: stages 1 to 6, and the changes asked for after the build. The maintainer tried it in the window.

Context stops being a form. A scene, a chapter, a part, and a book each get one **context**: free text, written as the writer likes, about what matters there. It replaces the synopsis, the notes, and a scene's goal, conflict, and outcome. The program does not need these as separate fields: an AI reading the project reads them as plain text anyway, as it reads notes since [Phase 14](../done/phase-14.md).

Beside it, each file gets a **connected** list the program writes itself: the item's keywords and every note its context and its text link to. An AI reading the top of a `.md` then knows what the item is about and what it is tied to before it reads a word of the text.

## Where things are now

- Context, for a scene, chapter, part, book, or manuscript folder, has **Synopsis** (one box), **Status**, **Keywords**, **Notes** (one box, Phase 13), **Color**, and **Include in compile**; a scene also has **Goal**, **Conflict**, and **Outcome** under "Beats and diagnostics". Context is not beside a note or a folder of notes (Phase 14): their own text is their context.
- In the file these are `synopsis`, `notes`, `goal`, `conflict`, and `outcome` in the front matter (`crates/format`, `crates/storage/src/files.rs`), and the cache has `binder_nodes.synopsis`, `binder_nodes.notes`, and a `scene_beats` table. Sync merges each key on its own.
- The synopsis is read elsewhere: the outliner's rows edit it; the mind map shows it under a chosen scene; a chapter's, part's, or book's context is built from the synopses below it (`crates/context`); Copy context gives the scenes before and after by their synopses; an idea made into a scene gives its note as the synopsis; the Markdown export and import carry it.
- The beats are read by the diagnostics (`crates/diagnostics`: "No goal", "No conflict", "No outcome", and the scene's beats in `SceneBeats`), by the outliner's Diagnostics table, which has a column for each, and by merging two scenes, which joins them.
- `[[links]]` are found in a scene's or note's text only (`StudioState::link_index`); renaming a note rewrites them in every text.

## Decisions

Made by the maintainer:

- **One free text replaces all of it.** Synopsis, notes, and the three beats become one `context`. Where a synopsis was shown, the first paragraph of the context is shown. The diagnostics' beat checks go.
- **The connected list holds keywords and `[[links]]`.** Not every note title the text happens to name: a common word that is also a note's title would get in.
- **It lives in the front matter**, at the top of the item's own `.md`, not in the text (which compile prints) and not in a file of its own.
- **Notes linked from Context count as connected too**, so the list is as complete for an AI as it can be: a note linked to a scene with **Link a note** is as deliberate a tie as `[[Mara]]` in its text.
- **The lists are kept current three ways**: the few files a change touches are saved as it happens; opening a project writes again every file whose list is out of date; and **Update connections**, in the Project menu, does the same pass on demand. One file at a time: a parallel save would gain nothing, as every write also goes through the one cache.

Recommended in this plan (the builder's calls, for the maintainer to correct):

- **Context is edited in a plain box**, saved as it is typed, not in the Markdown page: a side panel does not need live preview, and the box can grow with what is written.
- **A folder keeps its page text as its context** and gets no box, in the manuscript as under Notes: Phase 14 made a folder's text "what it is about and what is in it", which is what context is. A folder's Context keeps Status, Keywords, Color, and Include in compile. (This differs from what was said in the conversation, where manuscript folders got the box: the builder found the folder's text already is one.)
- **No conversion.** As in Phase 14, the program is pre-alpha: `FORMAT` is 5, and a format 4 project is refused with a message and left alone.

## What this phase does not take

- **A rich editor for context.** No live preview, Markdown shortcuts, or spelling in the box. Links in it work as text (`[[Mara]]`), not as clickable links.
- **Any field the program reads in the context.** It is the writer's text; nothing parses "Goal:" out of it.
- **The plot grid**, which stays as it is.
- **Note titles named without a link** in the connected list ("Named here" still offers them as links, as now).

## The work

### 1. One context, and format 5

- `BinderNode::context: String` replaces `synopsis` and `notes`. `SceneBeats`, the `scene_beats` table, `load_scene_beats` and `persist_scene_beats`, and the beats' part of the Markdown export and import go.
- In the file, `context` is the key after `title`, several lines under `|-`. `synopsis`, `notes`, `goal`, `conflict`, and `outcome` are no longer written; a format 5 file that has one is read as an unknown key and kept, as any other.
- The cache's `binder_nodes.context` column replaces `synopsis` and `notes` (a new column on an older cache, as before).
- Sync merges `context` by paragraph, as it merges a scene's text, so two machines that each add a paragraph both keep theirs. A paragraph both changed keeps both versions, this machine's first, and sync reports the conflict as it reports others; a context has no tracked changes, so nothing else marks it. (The builder's call: nothing is lost without a word.)
- **A paragraph** of a context is text between blank lines, as in Markdown, for the summary, the outliner's cell, and sync alike. (The builder's call.)
- `FORMAT` is 5; a format 4 project is refused with its own message.

**Done.** `BinderNode::context` replaces `synopsis` and `notes`, and `SceneHeader::context` replaces `synopsis` and the three beats; `context` is written right after `title`. `SceneBeats`, the cache's `scene_beats` table, `load_scene_beats` and `persist_scene_beats`, and the beats in the Markdown export and import, in sync, and in merging scenes are gone; storage, sync, and the format crate's tests no longer use `crates/diagnostics`. An older file's `synopsis`, `notes`, `goal`, `conflict`, and `outcome` are kept as unknown keys, and the Markdown import takes a file's `synopsis` as its context when it has none. The cache has `binder_nodes.context`, and a root's line in `binder.json` has `context` in place of `synopsis`. Sync merges a context by paragraph (`merge_context`, with the scene text's three-way merge made general as `merge_runs`): both machines' new paragraphs are kept, and a paragraph both changed keeps both, this machine's first, with a conflict that says so. `FORMAT` is 5; a format 4 project is refused with its own message and left alone.

So that the window kept building, parts of stages 2 and 3 came with this stage: Context has one **Context** box for a scene, chapter, part, or book in place of Synopsis and Notes, and no beats; "Beats and diagnostics" is **Diagnostics**, the outliner's Diagnostics table has no Goal, Conflict, or Outcome, and the diagnostics crate has no beat findings. `BinderNode::summary()` (`first_paragraph`) is shown where the synopsis was: the outliner, whose **Summary** cell changes only the first paragraph (`with_summary`), the mind map, the scenes before and after, and a level's outline. Copy context writes `## Context`. An idea made into a scene gives its note as the context, and merging scenes adds each joined scene's context after the open one's. Stage 2 still has the box's place, the chips, and folders; stage 3 has what is left of the summary. Tests: `the_summary_is_the_first_paragraph_and_is_changed_alone`, `a_context_merges_by_paragraph`, `a_context_on_any_level_is_kept_first_in_its_file`, the format 4 refusal, and the format, storage, context, and window tests on contexts.

### 2. Context in the window

- Context, for a scene, chapter, part, or book, is one box, **Context**, under the progress and the heading, then Status, Keywords, Color, and Include in compile. It grows with its text and saves as it is typed, as the synopsis did.
- Under it, the connected list as chips (keywords, then links), each opening what it names: a keyword narrows the binder to it, a link opens its note. It is the list as it will be written, so the writer sees what an AI will read.
- "Beats and diagnostics" becomes **Diagnostics**: the word, dialogue, and sentence checks stay; Goal, Conflict, and Outcome go. The outliner's Diagnostics table loses those three columns, and the diagnostics crate its three findings.
- A folder's Context, in the manuscript and under Notes, has no context box; a note's Context stays hidden, as Phase 14 left it.

**Done.** The **Context** box (from stage 1) sits under the progress and the heading, for a scene, chapter, part, or book; it grows with its text up to most of the panel (`field-sizing: content`) and saves as it is typed. Under it, **Connected** shows the list as chips: keywords, then notes as `[[Title]]`, a keyword narrowing the binder to it and a note opening. The list comes from one function, `writing_studio_storage::connected` (`crates/storage/src/connected.rs`), which stage 4 will write into each file, so Context and the file cannot differ. The window reads it without reading any text: the titles the text links to, as written, are kept beside the link index as the text is typed (`StudioState::connected_of`). A folder has no box, and a note's Context stays hidden. Diagnostics without the beats came in stage 1. Tests: `an_item_lists_its_keywords_then_its_notes_once_each` (`crates/storage`) and `context_lists_what_an_item_is_connected_to`.

### 3. The first paragraph as the synopsis

- `BinderNode::summary()` (in `crates/project`): the context's first paragraph, trimmed, or nothing.
- What showed the synopsis shows the summary: the outliner's rows, the mind map under a chosen scene, the scenes before and after in a scene's context, and the outline of a chapter, part, or book in its context.
- The outliner's cell edits the first paragraph and leaves the rest of the context as it is; an empty cell takes the first paragraph away.
- An idea made into a scene gives its note as the new scene's context.
- Copy context writes the item's whole context under `## Context`, in place of `## Synopsis` and `## Notes`; when the text is cut to fit, the context is kept before the notes' text, as the synopsis was.
- The Markdown export writes `context` in each file's front matter; the import reads `context`, and a file's `synopsis` as its context when it has no `context`, as other programs write.

**Done.** Most of this came with stage 1. The summary (`BinderNode::summary`, the context's first paragraph) shows in the four places the plan names: the outliner, the mind map under a chosen scene, the scenes before and after in Copy context, and a level's outline. The outliner's **Summary** cell is a box one line high that grows, not a one-line field, so a summary written on two lines keeps its line break when it is changed there; it changes only the first paragraph (`with_summary`). An idea made into a scene gives its note as the context. Copy context writes the item's whole context under `## Context`, which stays when the notes' text is left out to fit. The Markdown export writes `context`, and the import reads it, or another program's `synopsis` when there is none. Tests: `exported_scenes_come_back_as_new_scenes` and `the_markdown_drops_entry_text_from_the_end_first_and_says_so`, with the stage 1 tests.

### 4. The connected list

- Only the item's own: not the keywords or notes of the chapter, part, or book it is in, which are in their own files. (The builder's call: an inherited list would have to be written again in every file under a chapter each time the chapter changed.)
- `connected` in the front matter, after `context`: the item's keywords, in their order, then each note it is tied to, in the order first met: `[[links]]` in its context, `[[links]]` in its text, then notes linked to it from Context. A link in the text is written as `"[[Title]]"`, quoted, as Obsidian reads a link in front matter, as it is written there; a note linked from Context, by its own title in the same form. Each appears once.
- For a note, the list also has the items it is linked to from Context, after its links, each by its caption as the program shows it elsewhere ("Tide / Chapter 1 / Harbor"), quoted, since a scene's title alone is not unique and a scene is not a `[[link]]`.
- The program writes it on every save of the item and never reads it back: the next save writes it again from the item. A hand edit is written over; sync does not merge it.
- `[[links]]` in a context count wherever links count: in `link_index`, so a note mentioned in a scene's context reaches that scene and is in its **Notes in this scene**. A link in a context is never a footnote, as the context is never compiled.

**Done.** Every item's file has `connected` after `context` (`SceneHeader::connected`, written from `connected_written` in `crates/storage/src/connected.rs`, the same list Context shows); the Markdown export writes it too. The reader takes it out of the head and nothing uses it, so a hand edit is written over on the next save and is never kept as an unknown key. The text's links are read from the text being written. What comes from the binder (keywords, the context's links, notes linked from Context, a note's captions) is part of each file's head fingerprint, so when a change to one item changes another's list (a note linked from Context, a scene renamed, a note trashed) the next save of the binder writes that other file again, and only its head; saving a scene's text uses the binder as last written (`Mirror::tree`), so it reads no binder. `[[links]]` in a context are in the window's link index with the text's (`StudioState::relink`, on any change to a context), so a note mentioned in a context reaches the item and counts as a mention; renaming a note rewrites `[[old]]` in contexts as well as texts, and saves them. Tests: `a_file_lists_what_its_item_is_connected_to` (order, a note linked from Context, a note's captions, a hand edit written over) and `notes_are_linked_by_title_and_from_context_and_renaming_follows` (a book's context links and their rename).

### 5. Keeping the lists current

A list is only written when its own file is saved, so three things keep it true:

- **As a change happens**, the files it touches are saved with it, and only those:
  - linking or unlinking a note from Context saves the item;
  - renaming a note already rewrites and saves every file whose text links to it, and now rewrites `[[old]]` in contexts too; it also saves the items it is linked to from Context;
  - trashing, restoring, or emptying from the trash a note saves the items it was linked to from Context; emptying the trash of a scene or chapter saves the notes that were linked to it, as Phase 14 already takes those links away;
  - renaming a scene, chapter, part, or book saves the notes linked to it, whose captions change;
  - renaming or merging a keyword already saves every item that carries it.

  Typing in a scene saves that scene only, as now: a keystroke never writes another file.
- **When a project opens**, it already reads every file to fill the cache. It works out each list from what it read and writes again, one at a time, only the files whose list differs: what sync, git, or a hand edit changed outside the program. History lists how many were brought up to date, as it lists other repairs.
- **Update connections**, in the Project menu, does the same pass at any time. Its tooltip says what it does ("Writes the top of every file whose context and keyword list are out of date, so an AI reading the project sees every connection"), and the status line says how many files changed. With nothing unsaved it runs at once; with something unsaved it saves first.
- The pass holds one file at a time. It is not run in parallel: every write also updates the cache, which takes one writer at a time, and a project of a few hundred files is written in well under a second. (Measured when it is built, as the performance rule asks.)

**Done.** As a change happens: stage 4 put the binder's part of each list in the file's head fingerprint, so linking or unlinking a note, renaming, trashing, or restoring a note or a scene, and renaming or merging a keyword write again, on the next save of the binder, the head of each file whose list changed and no other; typing in a scene writes that scene only. When a project opens, `ProjectStore::open` compares what each file listed (`ProjectContent::listed`) with what its item has, from the texts it just read, and writes again only the files that differ; History lists how many, as it lists other repairs, and a file that cannot be written does not stop the project from opening. **Update connections**, in the Project menu (`StudioState::update_connections`), saves what is unsaved, reads in what changed outside as when the window comes back to the front, and then runs the same pass against what is on disk (`ProjectStore::update_connections`); the status line says how many files changed. A file changed outside that the program has not read in is left alone, never written over. The pass works out each file's list from its head alone (`listed_in`) and writes a file out in full only when its list is wrong; the text's links come from its plain text, as Context's do.

Measured on 300 scenes of about 500 words and 50 notes (351 files), the build that runs and a release build: opening, every list up to date, 0.69 s and 0.31 s; **Update connections** with nothing to write, 0.13 s and 0.07 s; with every file out of date, 1.58 s and 1.38 s. That last is the safe write of each file (about 4 ms each), not the pass, so the plan's "well under a second" holds only when few files are out of date; the pass is still one file at a time, as the cache takes one writer. Tests: `a_file_lists_what_its_item_is_connected_to` (opening rewrites a list changed by hand and says so, leaves the others alone; Update connections writes what the cache has and the files missed, then nothing, and never writes over a file changed outside) and `update_connections_writes_the_lists_that_are_out_of_date`.

### 6. Docs

- `documentation/USAGE.md`, `PROJECT_FORMAT.md` (the keys, the cache, format 5), and `ARCHITECTURE.md` describe the context and the connected list; the roadmap's table lists this phase.

**Done.** `USAGE.md` describes the Context box and its summary, **Connected** and `connected:`, **Update connections**, Diagnostics without beats, the outliner's Summary, and merging scenes' contexts; `PROJECT_FORMAT.md` the `context` and `connected` keys with an example, format 5 and the format 4 refusal, what writes a file's head, opening's pass, and the cache's `context` column; `ARCHITECTURE.md` the crates' parts in it. Found on the way and fixed with them, as they described what this phase changed: `SYNC.md` (a context merges by paragraph; `connected:` is not merged; an attached file, not a research file, gets a `-theirs` copy), `README.md` (notes, not characters and world notes; checks on a scene's text, not its beats), and USAGE's mind map, which still described the Story menu Phase 14 took away.

### After the build

Asked for by the maintainer after trying the phase:

- **The side panels slide** in and out: each is in a slot whose width moves (`.panel-slot`), and what is in it stays a moment after it closes (`use_lingering`), so it slides shut whole; not while a sash is dragged.
- **The page stays in the middle of the window** while the panels move: it is shifted by half the difference of their widths (`--shift`), as far as the room beside it allows, and moves with them.
- **The page's scroll bar fades** when it is not used: it shows while the page scrolls or under the pointer (`.desk.scrolling`), then fades.
- **Notes in this scene** has a list of every note not yet linked, with its folder, narrowed as it is typed (`NotePicker`), in place of a field whose suggestions the window did not show.
- **No status bar.** What the program has to say shows for a few seconds in a notice at the foot of the window (`StatusNotice`), with **Undo** while it applies; saving on its own says nothing. Today's words and this sitting's, which only the bar showed, are under the book's count in the library, and a deadline's pace is that line's tooltip.
- **A scene has a ring** in the binder, as a chapter and a book have, filled by its status (`status_fraction`), whose tooltip gives its status, words, characters, and reading time; Context's **Progress** is gone. A scene's size is kept as texts are read and saved (`StudioState::text_sizes`), and the open scene's is its session's, so the binder reads no text for it.

- Then, after trying it: the page followed a closing panel late, because its offset was a transition of its own, recalculated from the desk's width on every frame. The two panels' widths are now animated values (`--left-slot`, `--right-slot`, registered with `@property`), the slots' widths and the page's offset are worked out from them on each frame, and nothing else animates.
- Changing the font changed the page's width, since the line length was in `ch`, the width of a "0" in the chosen font. It is in pixels now, at 9px for each character of the page's 17px type (`CHARACTER_PX`), so the page keeps its width whatever the font.
- A scene's ring is at the row's end, after its keyword dots, and shows its tooltip: every child of a library row ignored the pointer, so the row's own "press F2 to rename" showed instead.

Tests: `a_scenes_ring_fills_with_its_status` and `a_scene_row_carries_the_scenes_size`; how the panels, the page, the scroll bar, and the notice move is the maintainer's to try.

## How changes are checked

- An item's context is written as `context: |-` and read back, with Markdown and `[[links]]` in it, and the old keys are kept as unknown keys.
- The connected list is written from keywords, links in the context and the text, and notes linked from Context, in that order, once each; a hand-edited list is written over on the next save.
- Each change in "Keeping the lists current" brings every list it touches up to date, and no other file is written; a keystroke writes only its own scene.
- A list changed outside the program is written again when the project opens, and by **Update connections**, which reports how many files it changed; a project whose lists are all current writes nothing.
- The summary is the first paragraph; the outliner's cell changes only that paragraph.
- Sync keeps both machines' new paragraphs of a context.
- A format 4 project is refused and left alone.
- The context box, the chips, and the outliner without the beats are the maintainer's to try in the window, as in earlier phases.
