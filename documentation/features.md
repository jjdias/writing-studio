# Features

What Writing Studio does today, in one place. [USAGE.md](USAGE.md) says how to use each feature step by step, and [PROJECT_FORMAT.md](PROJECT_FORMAT.md) says how a project is stored (format 5).

## Principles

1. **Open and free for the community.** The whole program is open source (MIT OR Apache-2.0). No account is needed to write, and no feature is held back.
2. **Local first.** A project is a folder (`*.writingstudio`) of Markdown and JSON files on the writer's disk, with a SQLite cache in `.local/` for this computer. That folder is the source of truth. A git host, if used, holds a copy.
3. **Never lose writing.** Files are written whole and renamed into place, the cache is crash-safe, and snapshots and backups are local.
4. **One core.** The window, storage, compile, and sync all use the same document model and binder.
5. **Offline writing is complete.** Writing and compiling never need an account, a network, git, a server, or another computer.
6. **Easy to stay, easy to leave.** Every scene and note is a plain Markdown file that another program, or a text editor, can read.

## Projects and files

- **New project** makes `Title.writingstudio` where the writer chooses. A project is never made inside another one.
- **Each item is a Markdown file:**
  - each scene is a file in `manuscript/`;
  - each chapter, part, and book has a file of its own there too;
  - each note and folder of notes is a file in `notes/`;
  - each template is a file in `templates/`.
- **What a file holds.** It starts with front matter between `---` lines: the title, the context, what the item is connected to, its status, and its keywords. The text comes after. It opens in Obsidian, Typora, iA Writer, or any text editor.
- **The JSON files.** `project.json`, `binder.json`, `plot.json`, and `mindmap.json` hold the title, the binder's order, the plot grid, and the mind map. A `README.md` in the project folder says what each file is.
- **`.local/` belongs to this computer.** It holds the search cache, snapshots, backups, writing sessions, and the layout. It is never shared, and the program rebuilds it from the files if it is deleted.
- **File names** come from the title when the item is made (`the-harbor-3f9a.md`), and are the same on every computer.
- **Changes made outside the program** (by a text editor or by git) are read again when the window comes back to the front, if nothing is unsaved. If a save would write over such a file, the other version is kept beside it as `name.conflict-<time>.md`.
- **Entries the program does not know** are kept in a file, never dropped.
- **No link leads out of the project.**
- **Older and newer formats.** A project from a newer version says so and is not opened. A project from an earlier format is refused with a message, and nothing in its folder is written.

## The page

- **A Markdown editor** (CodeMirror 6), carried in the program, so nothing is loaded from the network.
- **Live preview.** Bold reads as bold and a heading is large. The marks show only where the caret is. **Source mode** (Ctrl+E) shows every mark.
- **Keys:** bold, italic, underline, strikethrough, link, headings 1 to 6, and body text. With text selected, a key wraps it; inside a mark, it takes the mark off.
- **Paragraphs and lists.** Enter starts a paragraph and Shift+Enter breaks the line. Lists and quotes continue on Enter and end on an empty item.
- **Smart quotes and dashes**, and **closing brackets and marks**, both off until chosen.
- **Pasting** from a web page or word processor comes in as Markdown. A web address pasted over selected text makes it a link.
- **The format toolbar:** Bold, Italic, paragraph style, Quote, List, Link, Note, and **More** (Underline, Strikethrough, Numbered list, Scene break, highlight colors, and the removes). A button is lit when its mark is on.
- **Link and Note** ask in a small field under the selection, never in a browser prompt.
- **The scene's title** is the heading at the top of the page, renamed in place, with the word count beside it.
- **Saving** happens a few seconds after typing stops, on leaving the field, or with Ctrl+S. Undo and redo cover the document session.

### Page settings and keeping one's place

- **Settings for this computer:** the manuscript font (serif or sans serif; code stays fixed-width), line length (45 to 100 characters, in pixels, so the width does not change with the font), line height, paragraph spacing, first-line indent, and **Reset the page**.
- **Keep the line I type** near the top, middle, or bottom while the page scrolls under it.
- **Fade the rest of the page,** leaving the current sentence or paragraph at full strength.
- **Mark the line I am on** with a faint band.
- **Hide everything but the page while I type.** It is off by default. The bars and panels fade but keep their room, and come back on a pointer move, a Ctrl or Alt key, or leaving the page.
- **Focus mode** (Ctrl+Shift+F) shows only the page and a word count.

## Review

- **Notes on the text:** `{==text==}{>>the note<<}`, each of a kind: Comment, Question, Revise, or Check. A note can be resolved (it stays dimmed on the text) or removed.
- **Highlights:** yellow, green, blue, and pink. `==text==` is yellow too. Text can carry a highlight and a note together.
- **Footnotes** written `word[^1]` become notes on the word before them.
- **Tracked changes,** for one scene at a time, against the text it had when tracking started. Each changed paragraph is listed to accept or reject, one by one or all at once. Ctrl+Z undoes a reject. Tracking lasts until it is stopped, across closing the project.
- **Spelling.** Unknown words are underlined, except the one being typed. Suggestions replace every copy in the scene as one undo step. There is a project dictionary.
  - English (US) is built in. Another language takes its Hunspell files, in the project's `dictionaries/` folder or the computer's usual folders.
  - Notes' titles, numbers, single letters, and words in capitals are accepted.
  - The check runs on this computer.
- **The review panel** under the page has one tab each for Notes, Changes, Spelling, and **Navigate**. Navigate lists the scene's headings, links, and notes in order. A tab shows only when it has something in it. The panel can fold to its tabs or sit beside the page, and acting on it leaves the caret where it was.

## Context

- **Context** is one free text on every scene, chapter, part, and book: what it is for, what the reader should not know yet, what happens. It is saved as it is typed.
  - It is written at the top of the item's file, after the title, where an AI reading the file reads first.
  - It is never compiled or counted.
  - Its first paragraph is the item's summary, used by the outliner, the mind map, and **Copy context**.
  - A folder's own text is its context.
- **Connected** lists what the item is tied to: its keywords, the notes its context and text link to, and the notes linked to it from Context. A note's list has the scenes and chapters it is linked to.
  - The list is written into the file as `connected:` for any reader, and is never read back.
  - It is kept current three ways: as changes are made, when a project opens, and with **Project → Update connections**, which also covers files changed outside the program.
- **Copy context** puts a level's context on the clipboard as Markdown, at most 16,000 characters. When it is longer, it says what was left out. Nothing is sent anywhere.
  - For a scene: where it sits, the scenes before and after, its plot points, and every note that reaches it, with the note's text.
  - For a chapter, part, book, or folder: what it holds two levels down with their summaries, its plot points, and its notes.
- **Context is not shown for notes**, which are context themselves.

## Notes

- **Notes and folders of notes** live in the library under the manuscript. They are written in the page like scenes, and the binder's search finds them. A new project has the folders **Footnotes** and **Research**, both the writer's to rename or delete.
- **Templates** start a note: right-click a folder for **New note from template**, or right-click a note for **Save as template**. A new project has Character, Place, Source, and Research note. Templates are files in `templates/`, and come without a color.
- **`[[Links]]`** go to a note by title, or with other words shown: `[[Mara|the pilot's daughter]]`.
  - Typing `[[` offers the notes' titles.
  - A link's tip shows where the note is and its first line. A link to a title no note has is faint.
  - Clicking a link opens its note (Ctrl+click in Source mode), as does Ctrl+Enter with the caret in it.
  - Renaming a note changes every link to it.
- **Linking from a scene or chapter.** Its Context lists the notes linked to it, those mentioned in its text, and those linked to what it is in.
  - **Named here** offers the notes the text names, each with **+** to link it.
  - **Link a note…** lists every note not yet linked, narrowed as the writer types.
- **The right-click menu** of a note or folder has:
  - **Color**, shown as a dot in the library, with no dot for no color;
  - **Compile as footnotes**, which a folder passes down to what it holds and a new note takes from its folder. A full dot means a note compiles, and an empty ring means it does not.
- **Add a file…** copies a file into `attachments/` and links it at the caret.
- **Footnotes in the compiled book.** A link to a note set to compile as footnotes becomes a footnote holding the note's opening, up to its first heading. It is a real footnote in Word, an endnote in EPUB and HTML, and `[^1]` in Markdown. Other links print as their words.

## Organizing the book

- **The binder** holds books, parts, chapters, folders, and scenes.
  - Each scene shows its status as a colored pill, its keyword dots, and a ring filled by its status. The ring's tooltip gives the scene's words, characters, and reading time.
  - Each chapter shows a ring that fills as its scenes are finished.
- **Statuses:** To do, Draft, Revise, Done, and On hold, each with its own color. Done and Final count as finished.
- **Colors** for items: None, Gold, Green, Red, Blue, and Violet.
- **Adding:** **+ Add**, Ctrl+N for a scene, and Ctrl+Shift+N for a chapter. New items go in the book on screen.
- **Renaming:** double-click a row, or press F2, in the library.
- **Moving:** drag anything wherever it can go, within a book or into another one, with a line or an outline showing where it will land. Moves can be undone.
- **Several scenes at once:** Ctrl+click and Shift+click pick them, to set a status, merge, or move to the trash together.
- **The right-click menu** opens, renames, sets a status, splits, merges, moves to the trash, or restores.
- **Split at the caret** (Ctrl+Shift+K) and **Merge with the next scene** (Ctrl+Shift+M) carry the notes, highlights, tracked changes, plot points, and note links with the text. Each takes a snapshot first and is one undo step.
- **Trash.** Items go to the trash with what they hold and can be restored. **Empty trash** asks for a second click.
- **Include in compile** leaves a book, part, chapter, or scene out of the compiled file. It stays in the binder, dimmed.
- **Keyboard:** Up, Down, Home, End, and Enter move through the binder. **Go to** (Ctrl+P) finds an item by name.
- **Search** covers the scenes' titles, statuses, keywords, and text, and their chapters' names.
  - Every word must match, and the last one can be unfinished.
  - `#plot` asks for a keyword and `-#plot` leaves it out.
  - The index lives in memory, built when the project opens and updated on each save.

### Keywords and filters

- **Keywords** tag any item, as many as the writer likes.
  - A `/` nests them: choosing `plot` also takes in `plot/subplot`.
  - Case is ignored in every alphabet.
  - A keyword has one color everywhere.
- **The keyword list** in the library is a tree, with each keyword's color and how many items carry it, ordered by name or by use.
  - Clicking a keyword narrows the binder, the mind map, and the outliner to it.
  - Renaming one (F2) changes it on every item. Renaming it to a keyword already in use merges the two, and the change can be undone.
- **Saved filters** combine keywords (all or any of them), keywords to leave out, notes that reach a scene, a status, and words in the text, across every book. A filter with no name is named for what it looks for.
- **Keywords and filters belong to the project,** so every synced computer sees them. In Markdown, keywords are `tags:`, as Obsidian reads them.

## Views

- **Modes:** Writing, Mind map, Outliner, Plot, and History, opened with Ctrl+1 to Ctrl+5. A view with nothing in it says what goes there.
- **The mind map** shows the book in the middle, with its parts, chapters, and scenes branching out in binder order. With **Notes** chosen, it shows the notes and their folders instead.
  - **Navigating:** folding branches, the arrow keys, Fit, zoom, and a filter.
  - **Editing:** Tab adds a child and Enter a sibling, F2 renames, Delete moves to the trash, and dragging a node moves it in the binder. These can be undone.
  - **Ideas** hang from a node and are never compiled. An idea becomes a scene with **Make it a scene**, or by being dragged onto a chapter.
  - **Join** draws a labeled line between two nodes. **Tidy** lays the map out again.
  - Ideas, lines, and the places the writer put nodes are saved in `mindmap.json` and sync with the project.
- **The outliner** is a table of chapters and scenes, with each status and summary editable in its row and each word count.
- **Diagnostics,** in the outliner and in Context, are counts and checks on a scene's text, not judgments of it. For each scene they show:
  - its words;
  - its share of dialogue, meaning the text inside “ ”, " ", or « »;
  - the notes whose titles it names. A title is found by its capitalized words, ignoring articles and titles such as "The" and "Dr";
  - its checks: no text; much longer than the book's middle scene length (more than twice it) or much shorter (under a third of it), counted once the book has four scenes with text; no note named; and sentences averaging over 30 words.
- **Plot** is a grid of plotlines, each with plot points. A point has a title, a summary, and an optional link to a scene, and can be moved earlier or later on its line.

## Series, targets, and sessions

- **Several books** share one project and its notes. **Whole series** shows every book. Choosing a book makes it the one on screen, the one counted, and the one compiled, and the project remembers the choice.
- **Targets.** Each book has a word target and the project has a daily one. Today's words and this sitting's show under the book's count in the library.
- **A deadline** (**Finish by**) for each book shows the time left and the words a day needed to meet it.
- **Writing sessions** are kept and listed in History. Today's count starts again on the next local day.

## Recovery

- **Autosave** writes the file first and the cache second.
- **Snapshots** (Ctrl+Shift+S) are restored from Context or History. **Compare** shows the text added and removed between two versions, and can show only the paragraphs that changed.
- **Backups** are SQLite copies in `.local/backups/`. A backup can restore one scene (the text it replaces is kept as a snapshot) or the whole project (a safety copy is written first).
- **Kept from outside** lists the conflict copies: compare them, restore them (which can be undone), or delete them.
- **Repairs when a project opens.** Files changed outside the program are repaired and listed in History. Files that came in from outside go in **Recovered**, and a scene whose file is gone is marked **missing**.
- **History** shows the selected scene (snapshots, earlier syncs, and copies kept from outside), then the project (backups, sync, what was found on opening, and writing sessions).

## Sync

- **Optional,** through the `git` installed on the computer, with any git host. **History → Sync** connects a repository, and **Sync** sends and brings in changes.
- **Merging** works paragraph by paragraph and field by field. Paragraphs both computers changed are kept as tracked changes to accept or reject.
- **Earlier syncs** lists each scene as it was at each sync, to compare or restore.
- Backups, snapshots, and `.local/` are never shared. [SYNC.md](SYNC.md) has the setup.

## Compile

- **Profiles** set the format (Word DOCX, EPUB 3, a single-file HTML page, or Markdown) and the language (English, Português, Español, or Français).
  - The language sets the number words, "Chapter", "Part", and the matter headings.
  - A profile also sets how chapters and parts are headed and what goes between scenes (a scene break, an empty line, the title, or nothing).
  - For Word, it sets the font from this computer's fonts, the size, line spacing, page size, margins, indents, and a new page for each chapter.
- **Built-in profiles:** Manuscript (Word), Ebook (EPUB), Web page (HTML), and Plain text (Markdown). Profiles can be copied, removed, or reset, and travel with the project.
- **Front and back matter:** a title page (title, subtitle, and author), a copyright page (with `{year}` and `{author}` filled in), a dedication, contents, About the author, and Also by.
- **EPUB:** a page for each part, chapter, and page of matter, a table of contents, notes linked both ways, metadata with an id that stays the same across compiles, and an optional cover (PNG or JPEG up to 20 MB, checked by its content).
- **HTML:** one file with its stylesheet inside and no scripts, with linked contents and notes. Printed, each chapter starts a new page.
- **Notes in the compiled file:** open notes are footnotes in Markdown and bracketed text in DOCX. Resolved notes and highlights are left out.
- **Preview** shows the book as the profile sets it, before anything is written.
- **The check before compiling** lists what may not be meant to go out:
  - items left out of the compile;
  - empty scenes;
  - parts, chapters, or scenes with no title where the profile prints one;
  - open notes and tracked changes;
  - unknown words;
  - links to notes that do not exist.

  A click opens each item. Nothing in the list blocks the compile.
- **Markdown folder export** writes the whole project as Markdown: every scene in reading order, every note, and a README.
- **Markdown import** brings files in as scenes, taking their title and their `tags:` or `keywords:`.

## The window

- **Three columns:** the binder, the page (or the mind map or outline), and Context.
  - The side panels slide in and out (Ctrl+Alt+B and Ctrl+Alt+I), and the page stays centered while they move.
  - The panels can be resized within limits, and a double-click resets them.
  - In a narrow window they fold away and come back over the page. The window is never smaller than 720 × 520.
- **Notices** show for a few seconds at the foot of the window, with **Undo** while it still applies. Saving says nothing.
- **Menus:** Project (Save, Backup, Import, the exports, and Update connections), Settings, and Help.
- **Themes:** Dark, Light, and Paper. Text and controls meet WCAG AA contrast in each.
- **Keyboard.** Every shortcut is listed under Ctrl+/. Tab follows the layout, and every control shows a focus ring. Escape closes or cancels the innermost thing.
- **About** shows the version, the license, and the libraries and dictionary the program carries.
- **The scroll bar** shows while the page scrolls and fades after.

## Platforms and releases

- **Windows:** an MSI installer and a portable zip, using WebView2.
- **Linux:** a `.deb` package, using WebKitGTK.
- **One executable.** The stylesheet, the page's scripts, and the dictionary are inside it.
- **Licenses.** Each package has a `licenses` folder with the program's license, the dictionary's, CodeMirror's, and a notice of every library the program is built from.
- **Releases.** Pushing a `v*` tag that matches the version in `Cargo.toml` builds the packages in CI and publishes a GitHub Release with SHA-256 checksums. A tag with a suffix makes a pre-release. [RELEASING.md](RELEASING.md) has the steps.
- **CI** runs on every push and pull request:
  - it builds and tests the workspace on Windows and Linux, and checks the release build;
  - its Page job runs the page's JavaScript tests and checks that the committed bundle matches its sources.

## Left out on purpose

- **Other systems.** macOS, iOS, Android, and the web are dropped. Apple's systems need a paid developer membership, a phone needs a different interface, and a browser page cannot keep a project folder.
- **No network.** There are no accounts, no server, no automatic updates, no app store, and no assistance that sends text off the computer.
- **Sync needs git installed.** Git is not built into the program.
- **Not in the program:** grammar checking, editing at the same time on two computers, PDF output, print-on-demand, a Markdown math or diagram engine, and comments written as `<!-- -->` or `%%`.
- **No book-type presets.** There are no folders and templates for particular genres.
- **No fields the program reads** in notes or context: a role or a date is the writer's text. The program keeps no context files in `.local/`.
- **Parsing the context.** Nothing reads "Goal:" or anything else out of it.

## Known limits

- **Two kinds of undo.** Ctrl+Z undoes text, and merges or splits through it. Trash, drags, additions on the map, and keyword renames are undone from the notice's Undo button, which goes when another notice replaces it. A new keyword, a status change, or a rename has no undo.
- **Settings that last only until the program closes:** a hidden "Named here" suggestion, how keywords are folded and ordered, and folded mind map branches.
- **Diagnostics count, they do not read.**
  - Single quotation marks and dashes are not counted as dialogue.
  - A first-person scene, or one that uses only a nickname, names no note.
  - Abbreviations such as "Mr." end a sentence early.
  - The thresholds are fixed.
- **The context box** has no live preview, Markdown keys, or spelling, and its `[[links]]` are not clickable.
- **Seen while taking the README's screenshots:**
  - At the binder's default width, a scene inside a chapter that has a status and two keywords loses its title: the name has 0 px left.
  - The outliner's Status column is too narrow, and cuts "Done" to "Do".
  - The spelling tab says "1 words".
- **What the page shows differently from the files:** `![image](x.png)` shows as a link, `<b>` shows as raw tags, and spelling underlines words inside code blocks.
- **Not covered by tests,** and tried only in the window:
  - how the panels, the notice, and the scroll bar move;
  - fixed scrolling, the line marker, and hiding the bars while typing;
  - where a dropped mind map node lands;
  - the look of the window.
- **Before `v0.1.0`,** the maintainer still has to check the installer, the Start menu entry and removal, sync, research files, and Linux in WSL2. The Windows installer is not signed, so SmartScreen warns on first run.
- **The start screen names Scrivener** in its own words.
