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
- What it reads about a scene is Phase 11's scene context (`crates/context`): the scene, where it sits, its plot points, and the entries linked to it, capped to a given length.

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

## Keywords on the story's entries

What is left of the "Tags" entry, which moved to [Phase 11](phase-11.md). There, binder items get Obsidian-style keywords, and characters, places, and notes are linked to scenes and chapters, not tagged.

- A character, place, or note carries keywords from the same list (`antagonist`, `setting/coast`), and the keyword list counts them.
- It changes `bible.json`, so it is a format change of its own.

## PDF compile

Moved here from Phase 6, where it was stage 6, before it started. Today a book compiles to Word, an ebook, a web page, or Markdown; a PDF means printing the web page from a browser or saving the Word file as PDF.

- A paged PDF for print and for reading on screen: the page size, margins, font, and line spacing of the profile, page numbers, chapters starting on a new page (on a right-hand page for print), and justified text with hyphenation. A built-in *Paperback* profile (6 × 9 in, a serif, justified) comes with it.
- **It needs a decision before it starts.** A good PDF needs real typesetting: line breaking, hyphenation, fonts, page layout. The choices are:
  - **Typst as a library** (Apache-2.0): a full typesetting engine in Rust, offline, with good line breaking and hyphenation. It is a large dependency, and the program has to carry at least one font family with a license that allows it (for example Libertinus or Source Serif, both OFL).
  - **A small PDF crate** such as `printpdf`: much smaller, but the program would do its own line breaking and page layout, which is a lot of code to own and will look worse.
  - **Printing the HTML compile through the window's own engine** (WebView2 on Windows): no new dependency, but each platform's web view does it differently or not at all, and page control (right-hand chapter starts, running heads) is limited.
- The recommendation is Typst: it is the only one that gives a book-quality PDF without this project writing a typesetter. Its weight is a cost to weigh with the maintainer before it is added (see `.claude/rules/architecture.md`).
- Checked by a test that reads the file back: the right page count and size, and the text in order.

## Sync without installed git

Moved here from Phase 7. It was there for phones, which were dropped. Today sync runs the `git` installed on the computer; without it the program writes and compiles, and sync says git is missing.

- A git engine built into the program, so sync needs nothing installed.
- It needs a decision first on how the program keeps SSH keys or tokens, which it does not do today (`.claude/rules/security.md`).

## Linux AppImage

Moved here from Phase 7. Linux ships as a `.deb` for Debian and Ubuntu; a writer on another distribution builds from source.

- An AppImage: one file that runs on most Linux distributions, carrying WebKitGTK with it.
- `dx bundle --package-types appimage` failed on the GitHub Ubuntu runner (Ubuntu 26.04) and said only that `build_appimage.sh` failed, as `dx` does not pass on the script's output. What was tried: running the AppImage tools without FUSE (`APPIMAGE_EXTRACT_AND_RUN=1`) and skipping their `strip` (`NO_STRIP=true`). Not tried: adding `librsvg2-dev`, which the GTK plugin's image loaders need, and running the script again with `bash -x` to see where it stops.
