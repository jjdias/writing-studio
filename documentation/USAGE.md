# Using Writing Studio

Free, open-source, local-first. Your writing stays in a folder on your machine.

This guide describes the program as it is, Phases 1 to 5. What comes next is in [the roadmap](../roadmap/README.md).

## Start

1. `cargo run -p writing-studio-desktop`
2. **New project** — pick an empty parent folder. The app creates `Title.writingstudio`.
3. **Open project** — select an existing `.writingstudio` folder.

## The project folder

A `.writingstudio` folder is the whole project, kept as plain files you can open without this program:

- Each scene is an HTML file in `manuscript/`, named from its title when it was made (`the-harbor-3f9a.html`). Open one in a browser to read it, styled by `scene.css`. Characters, places, cultures, lore, timeline, and research notes are HTML files in folders of those names.
- `project.json`, `binder.json`, `bible.json`, and `plot.json` hold the title, the binder order, the links between entries, and the plot grid. `README.md` in the folder says what each file is.
- `.local/` belongs to this computer: the search cache, snapshots, backups, writing sessions, and window layout. It is never shared, and it can be deleted; the program fills it again from the files when the project opens.
- A file edited outside the program while the project is open is not lost. Before the program writes that file, the edited version is kept beside it as `name.conflict-<time>.html`, and the status line says so. The open scene is read again when the window comes back to the front, if nothing in it is unsaved.
- A project made by a newer version says so and is not opened. A project from the first test versions, kept in one database, is refused with a message; it is not converted, and nothing in its folder is written.

[PROJECT_FORMAT.md](PROJECT_FORMAT.md) describes every file.

## Write

- Select a scene in the binder. Type in the manuscript.
- Formatting buttons: Bold, Italic, Underline, Strike, Link, Unlink, H1, H2, H3, Body, Quote, List, Numbered, Code, and Break. The review buttons are Note, Remove note, the four highlight colors, and No highlight.
- **Note** asks for a note on the selection. The menu beside it sets what the note is for: Comment, Question, Revise, or Check. The note is saved with the scene, on that text. **Remove note** takes it off; the text stays.
- **Yellow**, **Green**, **Blue**, and **Pink** highlight the selection. **No highlight** takes the color off. Text can carry a highlight and a note together.
- Notes and highlights are listed above the manuscript. Click the quoted text to select it in the manuscript. In a note's row, change its kind, edit its text, **Resolve** it, or **Remove** it. A resolved note stays on the text, dimmed, and leaves the list until **Show resolved** is on. **Remove resolved** takes every resolved note off the scene. Each of these is one undo step.
- Compile writes open notes: footnotes in Markdown, bracketed text in DOCX. A kind other than Comment is named in front of the note. Resolved notes and highlights are left out of the compiled file.
- **Track changes** starts tracking the open scene from the text it has now. After that, each paragraph you rewrite, add, or remove is listed above the manuscript. **Accept** keeps a change and takes it off the list. **Reject** puts that paragraph back as it was; **Ctrl+Z** undoes a reject. **Accept all** and **Reject all** do the whole list. **Stop tracking** keeps the text and drops the list. Tracked changes stay in the project when you close it. Only text is tracked, one scene at a time, and compile uses the text as it stands. See [Phase 4](../roadmap/done/phase-4.md).
- Changes save a few seconds after you stop typing, when you leave the field, or with **Ctrl+S**.
- **Ctrl+Z** undoes. **Ctrl+Y** or **Ctrl+Shift+Z** redoes in the document session.

## Search

The binder field searches titles and manuscript text. **Ctrl+F** focuses it. On the corkboard, **Ctrl+F** focuses the scene filter instead.

The index is built in memory from the project when it opens, and updated when a document is saved. It is not a file in the project folder. The `.writingstudio` folder stays the source of truth.

**Escape** closes an open menu, or a binder or inspector shown over the page. Otherwise it cancels a delete or restore that is waiting for its second click, or leaves focus mode when it is on, or clears the corkboard filter when that field has text, and otherwise clears the binder search.

## Organize

- **Book**, **Part**, **Chapter**, **Scene**, and **Folder** add manuscript items. **Book** adds another novel under the same manuscript. **Ctrl+N** adds a scene when a project is open. **Ctrl+Shift+N** adds a chapter. Characters, world notes, plot, and research stay with the project. Research notes are edited in **Research**, not in the binder.
- **Whole series** in the binder, corkboard, and outliner shows every book. Pick a book there, or in the compile menu, to make that book the one on screen, the one compiled, and the one in the status bar. The project remembers that choice. **Whole series** shows every book again and leaves compile and the status bar on the last book. Selecting a scene points compile and the status bar at that scene's book. New chapters, parts, scenes, and manuscript folders are added inside the book on screen. A scene with nothing selected goes in its last chapter. New folders go in the book on screen.
- The inspector edits title, synopsis, status, label, and color. Label and color are for items that are not roots. Colors are None, Gold, Green, Red, Blue, and Violet. The label shows on the binder row. The inspector is on screen in Writing and History. Other modes hide it. **Ctrl+Alt+I** still shows or hides it for when you return to Writing or History.
- Drag items to reorder or nest. Drop on **Trash** to discard.
- **Restore** returns a trashed item to its previous parent when that parent still exists.
- **Empty trash** asks for a second click, then deletes those items from the local database. Permanently deleting a book drops its word target. If it was the compile book, compile and the status bar move to the first book that remains.

## Corkboard

**Corkboard** shows a card per scene: title, synopsis, status, word count, and label. Drag a card onto another scene to place it just after that scene. Filter the cards. Group them by order, chapter, label, color, or status. Card color comes from the binder color. Click a card to open that scene in Writing.

## Outliner

**Outliner** is a table of chapters and scenes. Edit the chapter title, scene title, status, and synopsis in the row. The words column is that row's count. **Open** returns to Writing on that item.

**Diagnostics**, beside **Outline** at the top, changes the table. Each scene shows its **Goal**, **Conflict**, and **Outcome**, which you type in the row, then its words, its share of dialogue, the characters from the bible it names, and the checks: a missing goal, conflict, or outcome; a scene with no text; a scene much longer or shorter than the book's usual one; no character named; long sentences. The same three fields and checks are in the inspector for the selected scene. The checks count what is in the text. They do not judge it. See [Phase 4](../roadmap/done/phase-4.md) for exactly what is counted.

## Recover

- Inspector → **Snapshot this document**, or **Ctrl+Shift+S**. Restore from the inspector list or **History**.
- **Compare** on a snapshot shows, in **History**, what changed between that snapshot and the current text. Added text is marked green and removed text is struck through. **Compare with** picks a second snapshot instead of the current text. **Changed paragraphs only** hides the rest. Only text is compared, not formatting, links, or inline notes. Comparing changes nothing.
- A project made by an earlier test version, which kept everything in one database, cannot be opened by this version.
- If a project's files were changed outside the program in ways it had to repair, the status line says so when it opens and **History** lists each file and what was done. Files that came in are in **Recovered** at the top of the manuscript. A scene marked **missing** in the binder has lost its file: restore the file and open the project again.
- A file can be changed outside the program, in a text editor or by git. When you come back to the window, it is read again if you have nothing unsaved: the open scene, or the whole project when the binder, the bible, or a bible entry changed. If a save would write over a file that was changed outside, the other version is kept beside it as `….conflict-<time>.html`, and the status line says so.
- **History** → **Kept from outside** lists those kept versions. For the selected scene, **Compare** shows what differs from the current text, and **Restore** puts the kept text back, which Ctrl+Z undoes. **Delete** (second click) removes a kept file once you no longer need it.
- **Backup** writes `.local/backups/project-….sqlite` in the project folder. In **History**, **Restore this scene** puts back only the selected scene as it was in that backup; the text it replaces is kept as a pre-restore snapshot. **Restore all** replaces the whole project (second click): a safety copy is written first, and the scene files are written again from what was restored. Backups and snapshots belong to this computer; they are not part of what git shares.

## Sync

Optional. Keep a project on more than one computer through git, with any git host. **History** → **Sync** connects the project to a repository; then **Sync** in the top bar sends this computer's changes and brings in the other's. Paragraphs both computers changed are both kept, as tracked changes to accept or reject. Setting it up: [SYNC.md](SYNC.md).

Once a project is synced, **History** → **Earlier syncs** lists the selected scene as it was at each sync. **Compare** shows what changed since; **Restore** puts that text back, and Ctrl+Z undoes it.

## Compile

The top bar shows the project's name and the current book, then the **Project** menu: **Save**, **Backup**, **Import Markdown files…**, and the exports.

- **Project** → **Markdown…** or **DOCX…** exports the current book. Choose where to save. In a Markdown compile, open notes are footnotes numbered across the book.
- **Project** → **Markdown folder…** writes the whole project as a folder of Markdown files: every scene in reading order, the bible, and a `README.md` that lists them. Use it to read the book anywhere, or to take it to another writing program.
- **Project** → **Import Markdown files…** brings Markdown files in as new scenes, where a new scene would go. A title in the file's front matter is used; otherwise the file's name.
- The book menu in the top bar chooses which novel is compiled. Other books stay out of the file. The title page uses that book's title.
- Research and trash are not included.

## Targets

The status bar shows the current book's words and target first, then in Writing the open scene, then words added today, this sitting, and the save state. A count that does not fit in a narrow window is left out. The book's total matches the outliner for the book chosen for compile: each scene, and a chapter only when that chapter has no scenes. Research notes are not included. Today and the sitting use that book's own baselines, so another book's count does not change them. The daily target is still one number for the project. Click the book's count to set both targets; they are saved in the project. **History** lists completed sittings. A sitting is stored the next time you open the project, when any book added words. Today's count for each book starts again on the next local day.

## Modes

Writing, Corkboard, Outliner, Plot, Characters, World, Research, History. The activity bar lists them by name in that order, and **Ctrl+1** to **Ctrl+8** follow it. In a short window the names show on hover and focus. Choosing the mode that is open does nothing. Below the modes, **Binder** and **Inspector** show or hide those panels, and **Focus** hides the binder and inspector.

## Plot

**Plot** is a local grid of plotlines. Add a plotline, then add plot points along that line. Each point has a title, a short summary, and an optional link to a manuscript scene. The scene list is labeled book / chapter / scene, so the same chapter in two books stays distinct. **Open** jumps to that scene. **Earlier** and **Later** change the order on the line. Deleting a plotline asks for a second click and removes its points. The grid is saved in the project folder. Emptying trash drops links to scenes that were permanently deleted.

## Characters

**Characters** keeps a local cast. Each profile has a name, a role, and a longer description. Motivations are a list on that character. A relationship names another character and how they are connected. Deleting a character asks for a second click and also removes their motivations and relationships. The cast is saved in the project folder. Each profile can link one or more manuscript scenes, labeled book / chapter / scene. Emptying trash drops those links.

## World

**World** keeps locations, cultures, lore, and a timeline in the project. A location can name a culture. A lore entry can name a location and a culture. A timeline event has a when-label, a title, what happened, and an optional location. **Earlier** and **Later** change event order. Deleting an entry asks for a second click and clears links that pointed at it. Each entry can also link manuscript scenes, labeled book / chapter / scene. Emptying trash drops those scene links.

## Research

**Research** is the only editor for research notes and their files. Each note has a title, notes, and files. **Attach file** copies the file into `attachments/` inside the project folder. **Open** launches it with the usual app for that file type. Removing a file or a note asks for a second click and deletes the copy in `attachments/`. A sqlite backup does not include those files; copy the whole project folder to keep them. A new project seeds one binder research note. Opening a project copies notes that still sit under the binder Research section into this library, then removes those notes and research folders from the binder. Characters, world notes, plot, and the manuscript stay where they are. A note can link manuscript scenes, labeled book / chapter / scene. Emptying trash drops those links.

## Layout

The arrangement is saved in the project. Drag the edge of the binder or the inspector to resize it. The binder width stays between 180 and 480 pixels. The inspector width stays between 220 and 560. Double-click that edge to reset the width. The arrow in the binder header moves the binder to the other side; the inspector stays on the opposite side.

**Binder** at the bottom of the activity bar, or **Ctrl+Alt+B**, shows or hides the binder. **Inspector**, or **Ctrl+Alt+I**, shows or hides the inspector. **Ctrl+B** still bolds the selection in the manuscript.

In a narrow window the inspector folds away first, then the binder, so the page keeps its room: with the default widths, below about 1,000 and 720 pixels wide. A folded panel comes back over the page from its button; a click beside it or **Escape** closes it. The window is never smaller than 720 × 520.

## Spelling

- Words that are not in the dictionary are underlined in red in the manuscript and listed above it. The word you are still typing is not underlined until the caret leaves it.
- Click a word in the list to select it in the manuscript and see suggestions. A suggestion replaces **every** copy of that word in the open scene, as one undo step. **Find next** moves to the next copy. **Add to project dictionary** accepts the word everywhere in this project.
- **Project dictionary** lists the accepted words. Click one to take it out.
- Words in character, place, and culture names are accepted without adding them. Numbers, single letters, and words in capitals are not checked.
- **Settings** → **Spelling** in the top bar chooses the dictionary or turns spellcheck off. English (US) is built in. For another language, put its Hunspell files (`name.aff` and `name.dic`) in a `dictionaries` folder inside the project folder and reopen the project. Dictionaries installed on the computer in the usual Hunspell folders are listed too.
- Spellcheck runs on this computer. No text leaves it.
- Underlines need a recent webview. Where they do not show, the list still works.

## Themes

**Settings** → **Theme** in the top bar saves **Dark**, **Light**, or **Paper** in the project. Dark matches VS Code Dark+. Light matches VS Code Light+. Paper keeps the warm chrome and sets the manuscript on a cream page.

## Keyboard

| Chord | Action |
| --- | --- |
| **Ctrl+S** | Save |
| **Ctrl+Shift+S** | Snapshot this document |
| **Ctrl+Z** | Undo |
| **Ctrl+Y**, **Ctrl+Shift+Z** | Redo |
| **Ctrl+B** | Bold the selection in the manuscript |
| **Ctrl+F** | Search. On the corkboard, focuses the scene filter |
| **Ctrl+Shift+F**, **Ctrl+\\** | Focus mode |
| **Escape** | Close a menu or a panel over the page, or cancel a waiting delete, or leave focus mode, or clear the corkboard filter, or clear binder search |
| **Ctrl+1** | Writing |
| **Ctrl+2** | Corkboard |
| **Ctrl+3** | Outliner |
| **Ctrl+4** | Plot |
| **Ctrl+5** | Characters |
| **Ctrl+6** | World |
| **Ctrl+7** | Research |
| **Ctrl+8** | History |
| **Ctrl+N** | New scene, when a project is open |
| **Ctrl+Shift+N** | New chapter |
| **Ctrl+O** | Open project |
| **Ctrl+Alt+B** | Show or hide the binder |
| **Ctrl+Alt+I** | Show or hide the inspector |
