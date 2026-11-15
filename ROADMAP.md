# Writing Studio roadmap

Writing Studio is **open source** (MIT OR Apache-2.0) and **free for the community**. Anyone may use, study, modify, and share it. No account is required to write.

**Local-first, now and later.** Saves, snapshots, backups, and compile live on the writer’s machine. Keeping a project on a second machine or with a hosting provider, planned in Phase 5 through git, stays **optional**. It must never become a requirement to open a project, write, or keep copies of work.

Status: Phases 1, 2, 3, and 4 are complete and on `master`. Details: [`documentation/PHASE1.md`](documentation/PHASE1.md), [`documentation/PHASE2.md`](documentation/PHASE2.md), [`documentation/PHASE3.md`](documentation/PHASE3.md), and [`documentation/PHASE4.md`](documentation/PHASE4.md).

---

## Product principles

1. **Open and free for the community.** The full application stays open source. Optional future services must not lock manuscripts away from people who never connect.
2. **Local storage first.** Projects are folders (`*.writingstudio`) of HTML and JSON files, with a SQLite cache for this machine. Backups are timestamped copies of that cache under `.local/backups/`.
3. **Never lose writing.** Files written whole and renamed into place, a crash-safe cache, and local backups come before sync.
4. **Shared Rust core.** UI, storage, compile, and later optional sync all consume the same document JSON and binder model.
5. **Offline writing is complete.** If a network, account, or server is unavailable, the studio still works.
6. **Easy to stay, easy to leave.** Every change should make writing a book here easier. None should make it harder to take the book somewhere else. A writer who stops using this program keeps files that another program, or a plain text editor, can read.


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
- SQLite schema: `project_meta`, `binder_nodes`, `documents`, `snapshots` (later phases added tables; see [`documentation/PROJECT_FORMAT.md`](documentation/PROJECT_FORMAT.md))
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
| `writing-studio-document` | Block document model, JSON, HTML interchange, text comparison, tracked changes |
| `writing-studio-editor` | Session, undo/redo, counts |
| `writing-studio-project` | Binder tree and settings |
| `writing-studio-manuscript` | Book / chapter / scene outline |
| `writing-studio-templates` | Blank novel seed |
| `writing-studio-storage` | The project folder: files as the project, a SQLite cache, snapshots, backups |
| `writing-studio-corkboard` | Scene cards from the binder |
| `writing-studio-export` | Local Markdown and DOCX compile |
| `writing-studio-plotting` | Plotlines, plot points, and scene links |
| `writing-studio-characters` | Profiles, motivations, and relationships |
| `writing-studio-worldbuilding` | Locations, cultures, lore, and timelines |
| `writing-studio-research` | Research notes and files in `attachments/` |
| `writing-studio-search` | In-memory full-text index of manuscript text, rebuilt when a project opens |
| `writing-studio-spellcheck` | Hunspell dictionaries (English US bundled), project word list, suggestions |
| `writing-studio-diagnostics` | Scene goal, conflict, and outcome; counts and checks on a scene's text |
| `writing-studio-format` | A scene as an HTML file and back; Markdown export and import |
| `writing-studio-ui` | Desktop workspace |
| `writing-studio-desktop` | Native window |

---

## Scaffolded but not built

These crates exist so module boundaries stay stable. They are **types or stubs**, not product features.

| Crate | Current state | Intended later work |
| --- | --- | --- |
| `writing-studio-collaboration` | Yrs placeholder constant | None planned. Editing at the same time is bonus work; Phase 5 does not use this crate |
| `writing-studio-synchronization` | “optional = true” stub | Phase 5: merging two copies of a project, and sync through git |
| `writing-studio-security` | Notes that local folders are unencrypted | None planned. A project is as private as the folder and the git host it is kept on |

---

## Done (Phase 2, Phase 3, and Phase 4)

### Phase 2 — Writing environment (local)

Done. See [`documentation/PHASE2.md`](documentation/PHASE2.md).

- Inline links from the toolbar, and more block types (headings, code, scene break)
- Inline comments: a note on a text run, stored in the document JSON. Phase 4 added note kinds, resolving, highlights, and track changes
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

### Phase 4 — Professional editing (local)

Done. See [`documentation/PHASE4.md`](documentation/PHASE4.md).

- Snapshot comparison. **History** shows what changed between a snapshot and the current text, or between two snapshots of the same document: paragraphs added and removed, and words changed inside a rewritten paragraph. It compares text only, and it writes nothing.
- Review notes and highlights. A note on a passage has a kind (Comment, Question, Revise, Check) and can be resolved, reopened, edited, or removed from a list above the manuscript. Text can be highlighted in four colors. Both are stored on the text in the scene. Compile leaves out resolved notes and highlights.
- Track changes without a server. **Track changes** keeps a copy of the scene as it is at that moment, in the project. Every paragraph rewritten, added, or removed after that is listed above the manuscript, to accept or reject one at a time or all at once. It tracks text, per scene.
- Spellcheck, on this computer only. Unknown words are underlined in the manuscript and listed above it with suggestions. English (US) is built in; other Hunspell dictionaries are read from the project's `dictionaries/` folder or from the system. A project keeps its own list of accepted words, and names from the bible are accepted without adding them.
- Scene diagnostics. Each scene has a goal, a conflict, and an outcome, written in the inspector or in the outliner's **Diagnostics** table. The table lists what is missing and what the text shows: length against the book's usual scene, share of dialogue, sentence length, and which characters from the bible are named. It counts on this computer and does not read meaning.

Phase 4 is complete. Optional on-device assistance, which was listed here, is now bonus work.

## Still to do

Later phases keep **local writing complete**. Anything involving accounts, servers, or the internet is deferred and optional.

### Phase 5 — Plain-text projects, and sync through git (optional)

In progress on the `alternative-phase-5` branch: stage 1 is done. This replaces the earlier plan for an account and a server.

A writer who wants the book on two machines, or kept with a hosting provider, uses git. Any git host works, so the program does not need an integration for each storage service, and it runs no server of its own. Git is optional. Nobody needs it, or an account anywhere, to write.

Git works on text files. Before this phase a project was one SQLite file, which git can store but cannot compare or merge. So the first half of this phase changes how a project is stored, and that half is worth doing even for a writer who never uses git: the book becomes files that can be read without this program.

The stages are in order. Each one is finished and useful before the next starts.

#### 1. Write the format down

Done for scenes, on the `alternative-phase-5` branch. See [`documentation/PROJECT_FORMAT_2.md`](documentation/PROJECT_FORMAT_2.md). Nothing the program stores has changed yet: it still reads and writes format 1.

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

#### 2. Project format 2: text files are the project

Done except for the Markdown export of a whole book. The program now makes, opens, and saves format 2 projects. See [`documentation/PROJECT_FORMAT_2.md`](documentation/PROJECT_FORMAT_2.md).

- The HTML and JSON files are the project: the manuscript, the binder, scene beats, characters, the world, research, the plot grid, and settings. `FORMAT` says 2.
- Every save writes the file first and the cache second. Each file is written whole and renamed into place, so a crash leaves the old file or the new one.
- A save writes only what changed: the scene being written, or the one item whose title or synopsis changed.
- SQLite is a cache on this machine, in `.local/`. It is filled from the files every time a project opens, so a scene edited outside the program is what opens, and a project copied to another machine without `.local/` is complete.
- What belongs to one machine stays in `.local/`, which git ignores: theme, panel layout, the book on screen, writing sessions, snapshots, backups, and track-changes baselines.
- Opening a format 1 project converts it: a backup first, then the files, then a check of the files against the database, and only then the change of format. Snapshots and earlier backups are kept. A conversion that is interrupted is finished on the next open. An older version of the program refuses a format 2 project.
- Emptying the trash deletes the files of the scenes in it. Deleting a bible entry deletes its file. Nothing else deletes a file.
- A path in a project file that points outside the project folder is refused.
- A new project folder gets a `README.md` that explains the files, a stylesheet so a browser shows a scene as a page, and a `.gitignore`.
- Restoring a backup writes the files again from what was restored.
- Compile, search, snapshots, track changes, spellcheck, and diagnostics work on the new store. Their tests pass on it.

Still to do:

- The whole book can be exported as Markdown files, one for each scene and bible entry, and Markdown can be brought in. The conversion is written and tested for one scene; there is no command for a book yet.

#### 3. Files changed outside the program

- A writer can edit a scene in any text editor. The program reads the change the next time the project opens, and while it is open.
- Text the program does not understand is kept as written, never dropped.
- A file that cannot be read is reported by name, and the rest of the project still opens.
- After any outside change the binder is checked: a file the binder does not list is added to a Recovered folder, and a binder entry whose file is gone is shown as missing, not deleted.

#### 4. Merge two copies of a project

One person usually writes a book, sometimes on two machines. Two people in the same scene at the same moment is not what this is for.

- The program merges two copies of a project itself, from the last version they shared. Git is not left to write conflict markers into a file.
- Scenes merge by paragraph. Paragraphs changed on only one side are taken. A paragraph changed on both sides is a conflict.
- A conflict is shown with the Phase 4 review tools: both versions, and a choice of one, the other, or both. Until the writer chooses, both are kept.
- Binder rules: an item moved on one side and edited on the other keeps both changes. An item deleted on one side and edited on the other is kept. A merge that would leave a loop or an item with no parent puts that item in Recovered.
- Attachments are whole files: the newer one is used and the other is kept beside it.
- No merge discards text without showing it.

#### 5. Sync through git

- **Sync** saves the project as a commit, fetches, merges with stage 4, and pushes. One button, and a line that says when the project was last synced and whether there is anything to send or receive.
- The writer does not see branches, staging, or rebase. Commit messages are written by the program from what changed.
- Autosave does not commit. Sync, closing the project, and a pause in writing do.
- The first version uses the `git` program installed on the computer, and the keys or tokens the writer has already set up for it. The program stores no passwords.
- A guide in `documentation/` explains how to create a private repository with a common host and connect a project to it.
- Connecting a project warns that a public repository makes the manuscript public.
- A new project gets a `.gitignore` for the cache, the local settings, snapshots, and backups, and a short `README` in the folder that says what the files are and how to read them without this program.
- Large research files are stored as they are. The guide explains Git LFS for writers who need it.
- If git is not installed, or there is no network, everything except Sync works, and Sync says why it cannot run.

#### Later in this phase

- A git engine built into the program, so that nothing has to be installed and Sync can work on phones and tablets.
- Guided setup: create or pick a repository and sign in from inside the program.
- History from git in the History view: open or compare a scene as it was at an earlier sync.

#### Not in this phase

- An account, a server run by this project, or shared-project permissions. Who can read or write a project is decided by the git host.
- Two people editing at the same time. This is bonus work.
- Encrypting a project. A private repository is as private as its host makes it.

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
- Packaged builds carry the license notice of the bundled spelling dictionary (`crates/spellcheck/dictionaries/`)

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

### Bonus — Editing at the same time

Not current work. This was part of the earlier plan for Phase 5, with a server. Most books are written by one person, or by a team in which one person edits, so sync through git covers the need.

- Two writers in the same project at the same moment, each seeing the other's changes as they are made.
- This needs a connection between the two programs. It must stay optional, and a project used this way must stay the same plain-text project.

### Bonus — Optional local assistance

Not current work. This was the last item of Phase 4 and moved here so that Phase 4 could close on the editing tools.

- Assistance runs on the writer's device only. No text is sent to a server.
- It is optional and off unless the writer turns it on. Writing, saving, compiling, and every Phase 4 tool work without it.
- It never becomes required to open a project or to keep working in one.

### Bonus — Review a whole book at once

Not current work. The Phase 4 review tools work on the scene that is open. Going through a finished draft means opening each scene to see what is left.

- One list of open notes across the current book or the whole series, grouped by scene. Clicking a note opens its scene at that text.
- The same for unknown words: a book-wide spelling list, and a replacement that can run across the book.
- A switch that tracks changes in every scene of a book from now, and a list of the scenes that have changes waiting.
- Counts of open notes and waiting changes on the binder row and the corkboard card.
- A date on each note, shown in the list.

### Bonus — Finer track changes

Not current work. Tracked changes are per paragraph, and the manuscript does not show them.

- Mark changed, added, and removed paragraphs in the manuscript itself, not only in the list above it.
- Accept or reject one edit inside a paragraph that has several.
- Track a change that is only formatting, a link, or a note.
- Show a paragraph that moved as moved, not as removed in one place and added in another.
- A date on each change, from when it was first seen.

### Bonus — Richer snapshot comparison

Not current work. Comparison is text only, in one column, between a snapshot and the current text or another snapshot.

- Show changes to formatting, links, notes, and highlights.
- Show a moved paragraph as moved.
- Old and new side by side, as a choice beside the single column.
- Compare the scene as it is in a project backup, without restoring that backup.
- Restore one paragraph from a snapshot, instead of the whole scene.

### Bonus — Spelling at the word

Not current work. A suggestion replaces every copy of a word in the scene, and all of spelling is worked from the list above the manuscript.

- A right-click menu on an underlined word: suggestions, add to the project dictionary, ignore.
- Replace one copy of a word, and ignore a word in this scene only.
- Two dictionaries at once, for a book that mixes languages.
- Replace a word that is partly bold or partly linked.
- Check spelling in the synopsis, the scene beats, and the bible's text fields.
- Read dictionary files in encodings other than UTF-8 and Latin-1.

### Bonus — Diagnostics that fit the book

Not current work. The checks use fixed numbers and count dialogue and characters in one way.

- Settings for the checks, saved in the project: what counts as a long or short scene and a long sentence, and which checks run.
- Count dialogue written in single quotation marks or after a dash.
- Other names for a character (a nickname, a title), so a scene that uses them is found.
- A viewpoint character for each scene, so a first-person scene is not reported as naming nobody, and a count of scenes per viewpoint.
- A small chart of scene length and dialogue share across the book.
- The missing-beat marks on the corkboard card.

### Bonus — Review marks in the compiled file

Not current work. Compile writes open notes as footnotes or bracketed text, and leaves out highlights and tracked changes. This belongs with the export profiles of Phase 6.

- DOCX: notes as Word comments, highlights as Word highlighting, and tracked changes as Word revisions that an editor can accept or reject in Word.
- A compile choice between a clean draft and a draft with review marks.

### Bonus — One place for the review panels

Not current work. The note list, the tracked changes, and the spelling list each sit above the manuscript and take room from it when all three are in use. This belongs with the writing surface of Phase 8.

- One panel for the three, with a tab for each, that can be folded away or moved to the side.
- Changing a note from that panel leaves the caret where it was.

### Bonus — Bring an older project up to date on open

Not current work. New tables are added when a project opens, but columns added to `binder_nodes` after its first version are not: a database from before `label` and `color` does not gain them.

- Add any missing column on open, and after a backup is restored.
- A test that opens a database from each earlier layout.

### Bonus — Tests that drive the window

Not current work. The tests cover the document model, storage, the state behind the window, and the editor scripts in a browser. Nothing opens the desktop window and clicks through it, so a control wired to the wrong action is only found by hand.

- A test run that starts the app on a temporary project and drives the toolbar, the review panels, History, and the outliner.
- Run it in the automated builds of Phase 7.

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
- Git, a git host, or any hosting provider
- A copy on a server as the only copy
- Simultaneous editing across machines
- Server-side search or storage as the source of truth

With or without sync, the source of truth is the `.writingstudio` folder on the writer's disk. A git host holds a copy of it.

---

## Immediate next steps

Phase 4, local professional editing, is complete and merged into `master`: snapshot comparison, review notes and highlights, track changes, spellcheck, and scene diagnostics. Phase 5 is in progress on the `alternative-phase-5` branch: plain-text projects first, then sync through git. Stage 1 is done: the format is written down, and a scene is stored as an HTML file that reads back exactly, with Markdown as the export. Stage 2 is done except for exporting a whole book as Markdown: the program makes, opens, and saves projects as files, with SQLite as a cache, and converts a format 1 project when it is opened. Next is that export, then stage 3. The writing-surface UI is Phase 8. Richer front matter than the book title stays in Phase 6.

Bonus work, none of it started: editing at the same time; optional on-device assistance; reviewing a whole book at once; finer track changes; richer snapshot comparison; spelling at the word; diagnostics that fit the book; review marks in the compiled file; one place for the review panels; bringing an older project up to date on open; tests that drive the window; a separate bible project beside each book; backups that include research files; and tags.

There is no plan for an account or a server run by this project.
