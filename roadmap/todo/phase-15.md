# Phase 15 — Free context

In progress: stage 1 is done. Branch: `phase-15`.

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

### 3. The first paragraph as the synopsis

- `BinderNode::summary()` (in `crates/project`): the context's first paragraph, trimmed, or nothing.
- What showed the synopsis shows the summary: the outliner's rows, the mind map under a chosen scene, the scenes before and after in a scene's context, and the outline of a chapter, part, or book in its context.
- The outliner's cell edits the first paragraph and leaves the rest of the context as it is; an empty cell takes the first paragraph away.
- An idea made into a scene gives its note as the new scene's context.
- Copy context writes the item's whole context under `## Context`, in place of `## Synopsis` and `## Notes`; when the text is cut to fit, the context is kept before the notes' text, as the synopsis was.
- The Markdown export writes `context` in each file's front matter; the import reads `context`, and a file's `synopsis` as its context when it has no `context`, as other programs write.

### 4. The connected list

- Only the item's own: not the keywords or notes of the chapter, part, or book it is in, which are in their own files. (The builder's call: an inherited list would have to be written again in every file under a chapter each time the chapter changed.)
- `connected` in the front matter, after `context`: the item's keywords, in their order, then each note it is tied to, in the order first met: `[[links]]` in its context, `[[links]]` in its text, then notes linked to it from Context. A link in the text is written as `"[[Title]]"`, quoted, as Obsidian reads a link in front matter, as it is written there; a note linked from Context, by its own title in the same form. Each appears once.
- For a note, the list also has the items it is linked to from Context, after its links, each by its caption as the program shows it elsewhere ("Tide / Chapter 1 / Harbor"), quoted, since a scene's title alone is not unique and a scene is not a `[[link]]`.
- The program writes it on every save of the item and never reads it back: the next save writes it again from the item. A hand edit is written over; sync does not merge it.
- `[[links]]` in a context count wherever links count: in `link_index`, so a note mentioned in a scene's context reaches that scene and is in its **Notes in this scene**. A link in a context is never a footnote, as the context is never compiled.

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

### 6. Docs

- `documentation/USAGE.md`, `PROJECT_FORMAT.md` (the keys, the cache, format 5), and `ARCHITECTURE.md` describe the context and the connected list; the roadmap's table lists this phase.

## How changes are checked

- An item's context is written as `context: |-` and read back, with Markdown and `[[links]]` in it, and the old keys are kept as unknown keys.
- The connected list is written from keywords, links in the context and the text, and notes linked from Context, in that order, once each; a hand-edited list is written over on the next save.
- Each change in "Keeping the lists current" brings every list it touches up to date, and no other file is written; a keystroke writes only its own scene.
- A list changed outside the program is written again when the project opens, and by **Update connections**, which reports how many files it changed; a project whose lists are all current writes nothing.
- The summary is the first paragraph; the outliner's cell changes only that paragraph.
- Sync keeps both machines' new paragraphs of a context.
- A format 4 project is refused and left alone.
- The context box, the chips, and the outliner without the beats are the maintainer's to try in the window, as in earlier phases.
