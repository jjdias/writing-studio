# Phase 5 — what has shipped so far

Phase 5 is **plain-text projects, and sync through git**. Two of its five stages are done, the second with its Markdown export and import. The rest is in [../todo/phase-5.md](../todo/phase-5.md), and what the done part still needs is in [../todo/phase-5.1.md](../todo/phase-5.1.md).

This work is on the `alternative-phase-5` branch. The format itself is described in [PROJECT_FORMAT_2.md](../../documentation/PROJECT_FORMAT_2.md).

## Why the phase changed

Phase 5 was first planned as an optional account and a server. It was replaced by this plan: a writer who wants the book on two machines, or with a hosting provider, uses git. Any git host works, so the program needs no integration for each storage service and runs no server of its own.

Git works on text files, and a project was one SQLite file. So the first half of the phase changes how a project is stored, and that half stands on its own: the book becomes files that can be read without this program.

## Stage 1 — The format, and a scene as a file

- The format is written down, with example files. Scene files are specified in full. The bible's files and the JSON files are specified more briefly, to be settled as stage 2 builds them.
- **A scene is stored as an HTML file**, one file each, with every paragraph on its own line. Its title is the `<title>`; its synopsis, status, label, color, goal, conflict, and outcome are `<meta>` lines. Names the program does not know are kept. Any browser opens the file.
- HTML holds a scene exactly, with one spelling for each thing and nothing escaped but `&`, `<`, and `>`. Markdown was built first and could not: it drops a few things the editor produces, and other Markdown readers disagree about `**` next to punctuation.
- **Markdown is the export**, for taking a book to another writing tool, and an import. It uses `---` front matter and CriticMarkup for notes and highlights. What it cannot say is listed in the format.
- Nothing in a scene file is ever run: scripts are skipped, and a link that is not a safe address is not written.
- A new crate, `writing-studio-format`, does all of this.
- File names are a readable name and a short id, such as `the-harbor-3f9a.html`. A file is named once and not renamed.
- Files hold no time of last save and no per-paragraph ids.
- The round trip is tested: every kind of block and formatting, randomly built documents, and a project on disk. The Markdown export is also read with an independent Markdown reader. `check_project` runs the check on any project folder without changing it.

Order keys were dropped from the plan. The order of the lines in a file is the order of the items, which needs nothing kept in step.

### Decisions made in stage 1

- **Markdown was tried first as the stored format, and replaced by HTML.** Markdown cannot hold a scene exactly: it drops bold on a space at the edge of a stretch, a line break at the end of a paragraph, and formatting inside a code block, and other Markdown readers disagree about `**` beside punctuation. The Markdown writer and reader were kept as the export and the import.
- **The header of the Markdown export is `---` front matter**, not JSON, because other writing tools read it.
- **Order keys were dropped.** The order of the lines in a file is the order of the items.

## Stage 2 — The files are the project

- The HTML and JSON files are the project: the manuscript, the binder, scene beats, characters, the world, research, the plot grid, and settings. `FORMAT` says 2.
- Every save writes the file first and the cache second. Each file is written whole and renamed into place, so a crash leaves the old file or the new one.
- A save writes only what changed: the scene being written, or the one item whose title or synopsis changed.
- SQLite is a cache on this machine, in `.local/`. It is filled from the files every time a project opens, so a scene edited outside the program is what opens, and a project copied to another machine without `.local/` is complete.
- What belongs to one machine stays in `.local/`, which git ignores: theme, panel layout, the book on screen, writing sessions, snapshots, backups, and track-changes baselines.
- A folder made by an earlier test version, in format 1, is not opened or converted. Opening one says so and writes nothing.
- Emptying the trash deletes the files of the scenes in it. Deleting a bible entry deletes its file. Nothing else deletes a file.
- A path in a project file that points outside the project folder is refused.
- A new project folder gets a `README.md` that explains the files, a stylesheet so a browser shows a scene as a page, and a `.gitignore`.
- Restoring a backup writes the files again from what was restored.
- Compile, search, snapshots, track changes, spellcheck, and diagnostics work on the new store. Their tests pass on it.


### Decisions made in stage 2

- **A scene's text has the id of its binder item.** Format 1 gave them separate ids.
- **The cache is filled from the files on every open**, not kept in step by comparing dates. It is simpler, and it is always right.
- **The book on screen is this machine's setting**, with the theme and layout. Word targets and the spelling list go with the book.
- **One `bible.json`** orders the bible's entries and holds their links, in place of a separate links file.
- **Emptying the trash deletes scene files**, because that step already asks twice. Deleting a bible entry deletes its file.
- **Format 1 projects are not converted.** Conversion was built, then removed: format 1 had no downloads, so there were no books to convert, and an in-place one-way rewrite of a project was the riskiest code in the phase.

## Stage 2, last item — The whole book as Markdown, and Markdown brought in

- **Markdown folder**, beside Markdown and DOCX compile, writes the whole project into a new folder: the manuscript in reading order (`manuscript/01-tide/01-chapter-1/01-scene-1.md`), the bible one file for each entry, and a `README.md` that lists everything in order with links. The trash is left out. A chapter that only holds scenes is a folder, not an empty file. Scenes keep their notes and highlights as CriticMarkup, and their details as front matter. It is for reading the book or taking it to another program, so ids and settings are not written.
- **Import** in the binder brings Markdown files in as new scenes, where a new scene would go, in the order chosen. The front matter gives the title and details; with none, the file's name is the title. Every scene gets a new id, so a file brought in twice is two scenes. A file that cannot be read is named in the status line, and the others still come in.
- **One Markdown writer.** Compile had its own writer, which escaped only four characters, so a `#` or `1.` typed at the start of a line came out as a heading or a list; and it numbered footnotes from 1 in every scene, so a compiled book had several `[^1]`. Compile now uses the format crate's writer, in a style for readers: open notes as footnotes numbered across the book, without resolved notes or highlights. Its own writer, about 90 lines, is gone.

Checked by tests of compile (footnotes across a book, a title that needs escaping), of the footnote style, of the export (order, the trash left out, the README, a full folder refused), of import (text and details come back, no header, a broken header), and of the window's state for both.

## Phase 5.1, item 1 — Entries the program does not know are kept

A file can hold entries the program does not use: a person adds `<meta name="pov" content="Mara">` to a scene, a key to a line of `binder.json`, or a later version of the program writes a new one. Before this, the next save of that item removed them.

- When a project opens, each item's unknown entries are read with it: names in the head of a scene or bible file, and keys on its line in `binder.json`, `bible.json` (entries and research files), and `plot.json` (plotlines and points).
- They are kept in memory while the project is open and written back after the entries the program knows, on every save of that item, and when a backup is restored.
- An unknown entry never stands in for one the program knows: a name or key the program writes is never taken from the extras.
- They are not in the cache. The files are read every time a project opens, so the cache does not need them. This replaces the database column the plan proposed.
- Still dropped: unknown keys in `project.json`, and on the `relationships` and `scene_links` lines of `bible.json`, which have no id of their own.

Checked by a test that adds an entry to a scene file, its binder line, a character file, and a plot line, then saves each through the store, and restores a backup.

## Phase 5.1, item 2 — A file changed outside is never written over

A project's files can change while it is open: an edit in a text editor, a `git pull` in a terminal, a sync tool. Before this, the program's next save of that item wrote over the change.

- The store remembers what each file held when it last read or wrote it. Before writing a file it reads it. If it holds something else, what it holds is kept beside it as `name.conflict-<time>.ext`, and then the program's version is written. This is for every file the program writes: scenes, bible entries, and the JSON files.
- Deleting a file works the same way: a scene file changed outside is kept under the conflict name, not deleted, when the trash is emptied.
- The status line names the file and where the other version was kept.
- When the window comes back to the front, the open scene's file is checked. If it changed and there is no unsaved typing in the editor, the scene is read again: its text, title, synopsis, status, label, color, and beats. With unsaved typing nothing is replaced, and the status line says the save will keep both. Only that one file is read; there is no watcher and no timer.
- **One change from the plan.** The plan said the outside version should become the scene and the program's be set aside. It is the other way round: the program's version is the scene, because that is what the writer sees in the window. Either way nothing is lost.

Checked by tests that change a scene file, `binder.json`, and a scene about to be deleted behind the store's back, and by a test of the window's state that reads a changed scene again, and keeps both when the writer is typing. **The focus event itself has not been tried in the window.**

## Phase 5.1, item 3 — Dropped, with conversion

Item 3 asked the writer before converting a format 1 project. It was built, and then conversion itself was removed: the format 1 versions had no downloads, so there were no books to convert. Opening a format 1 folder now says it was made by an earlier test version, and writes nothing. The question, `convert_copy`, the `to_format_2` example, and the conversion code are gone; they are in the history before this change.

## What was checked

- 148 tests passed across the workspace at the end of stage 2. The storage crate's 24 tests, the UI crate's 10, the format crate's 25, and the export crate's 5 pass after the Markdown export.
- Scene files: every kind of block and formatting, with their awkward cases; randomly built documents (2,000 with every run, and more than a million in one-off runs); a project on disk.
- The Markdown export, read with an independent Markdown reader (`pulldown-cmark`).
- The store: making a project, saving, reopening after a file was changed outside, opening with no `.local/`, refusing a format 1 folder, deleting a bible entry, and restoring a backup.
- **Not checked: the desktop window.** Nothing in this phase, or in Phase 4, has been run by hand in the window. No project larger than a few scenes has been opened.
