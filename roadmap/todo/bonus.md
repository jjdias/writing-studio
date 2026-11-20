# Bonus work

None of this is started, and none of it is promised. Each entry says what is missing today and what would fix it. An entry moves into a phase when it is picked up.

## Guided sync setup

Moved here from Phase 5. Today a writer makes the repository with their git host and sets up git's keys or token themselves, following `documentation/SYNC.md`.

- Create or pick a repository, and sign in, from inside the program.
- It needs an integration with each git host, which goes against syncing with any git host, and the program storing tokens, which needs an explicit decision (`.claude/rules/security.md`).

## Optional local assistance

Not current work. This was the last item of Phase 4 and moved here so that Phase 4 could close on the editing tools.

- Assistance runs on the writer's device only. No text is sent to a server.
- It is optional and off unless the writer turns it on. Writing, saving, compiling, and every Phase 4 tool work without it.
- It never becomes required to open a project or to keep working in one.

## Review a whole book at once

Not current work. The Phase 4 review tools work on the scene that is open. Going through a finished draft means opening each scene to see what is left.

- One list of open notes across the current book or the whole series, grouped by scene. Clicking a note opens its scene at that text.
- The same for unknown words: a book-wide spelling list, and a replacement that can run across the book.
- A switch that tracks changes in every scene of a book from now, and a list of the scenes that have changes waiting.
- Counts of open notes and waiting changes on the binder row and the corkboard card.
- A date on each note, shown in the list.

## Finer track changes

Not current work. Tracked changes are per paragraph, and the manuscript does not show them.

- Mark changed, added, and removed paragraphs in the manuscript itself, not only in the list above it.
- Accept or reject one edit inside a paragraph that has several.
- Track a change that is only formatting, a link, or a note.
- Show a paragraph that moved as moved, not as removed in one place and added in another.
- A date on each change, from when it was first seen.

## Richer snapshot comparison

Not current work. Comparison is text only, in one column, between a snapshot and the current text or another snapshot.

- Show changes to formatting, links, notes, and highlights.
- Show a moved paragraph as moved.
- Old and new side by side, as a choice beside the single column.
- Compare the scene as it is in a project backup, without restoring that backup.
- Restore one paragraph from a snapshot, instead of the whole scene.

## Spelling at the word

Not current work. A suggestion replaces every copy of a word in the scene, and all of spelling is worked from the list above the manuscript.

- A right-click menu on an underlined word: suggestions, add to the project dictionary, ignore.
- Replace one copy of a word, and ignore a word in this scene only.
- Two dictionaries at once, for a book that mixes languages.
- Replace a word that is partly bold or partly linked.
- Check spelling in the synopsis, the scene beats, and the bible's text fields.
- Read dictionary files in encodings other than UTF-8 and Latin-1.

## Diagnostics that fit the book

Not current work. The checks use fixed numbers and count dialogue and characters in one way.

- Settings for the checks, saved in the project: what counts as a long or short scene and a long sentence, and which checks run.
- Count dialogue written in single quotation marks or after a dash.
- Other names for a character (a nickname, a title), so a scene that uses them is found.
- A viewpoint character for each scene, so a first-person scene is not reported as naming nobody, and a count of scenes per viewpoint.
- A small chart of scene length and dialogue share across the book.
- The missing-beat marks on the corkboard card.

## Review marks in the compiled file

Not current work. Compile writes open notes as footnotes or bracketed text, and leaves out highlights and tracked changes. This belongs with the export profiles of Phase 6.

- DOCX: notes as Word comments, highlights as Word highlighting, and tracked changes as Word revisions that an editor can accept or reject in Word.
- A compile choice between a clean draft and a draft with review marks.

## Bring an older project up to date on open

Not current work. New tables are added when a project opens, but columns added to `binder_nodes` after its first version are not: a database from before `label` and `color` does not gain them.

- Add any missing column on open, and after a backup is restored.
- A test that opens a database from each earlier layout.

## Tests that drive the window

Not current work. The tests cover the document model, storage, the state behind the window, and the editor scripts in a browser. Nothing opens the desktop window and clicks through it, so a control wired to the wrong action is only found by hand.

- A test run that starts the app on a temporary project and drives the toolbar, the review panels, History, and the outliner.
- Run it in the automated builds of Phase 7.

## Keep research files with a backup

Not current work. **Backup** copies the SQLite file and leaves `attachments/` behind. Restoring an older backup can leave research rows pointing at files that are gone, or leave files on disk that the restored library no longer lists.

- Include `attachments/` in a project backup, beside the SQLite copy.
- Restore the database and those files together.
- After restore, every research file row has its file on disk, and files that belonged only to the replaced library are not left behind as the current library.

## Tags

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
