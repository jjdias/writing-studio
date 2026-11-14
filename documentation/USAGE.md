# Using Writing Studio

Free, open-source, local-first. Your writing stays in a folder on your machine.

Phases 1, 2, and 3 are the app described here. Current work is Phase 4 in [ROADMAP.md](../ROADMAP.md).

## Start

1. `cargo run -p writing-studio-desktop`
2. **New project** — pick an empty parent folder. The app creates `Title.writingstudio`.
3. **Open project** — select an existing `.writingstudio` folder.

## Write

- Select a scene in the binder. Type in the manuscript.
- Formatting buttons: Bold, Italic, Underline, Strike, Link, Unlink, H1, H2, H3, Body, Quote, List, Numbered, Code, and Break. The review buttons are Note, Remove note, the four highlight colors, and No highlight.
- **Note** asks for a note on the selection. The menu beside it sets what the note is for: Comment, Question, Revise, or Check. The note is saved with the scene, on that text. **Remove note** takes it off; the text stays.
- **Yellow**, **Green**, **Blue**, and **Pink** highlight the selection. **No highlight** takes the color off. Text can carry a highlight and a note together.
- Notes and highlights are listed above the manuscript. Click the quoted text to select it in the manuscript. In a note's row, change its kind, edit its text, **Resolve** it, or **Remove** it. A resolved note stays on the text, dimmed, and leaves the list until **Show resolved** is on. **Remove resolved** takes every resolved note off the scene. Each of these is one undo step.
- Compile writes open notes: footnotes in Markdown, bracketed text in DOCX. A kind other than Comment is named in front of the note. Resolved notes and highlights are left out of the compiled file. Track changes is not built yet. See [PHASE4.md](PHASE4.md).
- Changes save a few seconds after you stop typing, when you leave the field, or with **Ctrl+S**.
- **Ctrl+Z** undoes. **Ctrl+Y** or **Ctrl+Shift+Z** redoes in the document session.

## Search

The binder field searches titles and manuscript text. **Ctrl+F** focuses it. On the corkboard, **Ctrl+F** focuses the scene filter instead.

The index is built in memory from the project when it opens, and updated when a document is saved. It is not a file in the project folder. The `.writingstudio` folder stays the source of truth.

**Escape** leaves zen mode when it is on. Otherwise it clears the corkboard filter when that field has text, and otherwise clears the binder search.

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

## Recover

- Inspector → **Snapshot this document**, or **Ctrl+Shift+S**. Restore from the inspector list or **History**.
- **Compare** on a snapshot shows, in **History**, what changed between that snapshot and the current text. Added text is marked green and removed text is struck through. **Compare with** picks a second snapshot instead of the current text. **Changed paragraphs only** hides the rest. Only text is compared, not formatting, links, or inline notes. Comparing changes nothing.
- **Backup** writes `backups/project-….sqlite`. Restore from **History** (second click). A safety copy of the live file is written first.

## Compile

- **Markdown** or **DOCX** in the top bar. Choose where to save.
- The book menu next to those buttons chooses which novel is compiled. Other books stay out of the file. The title page uses that book's title.
- Research and trash are not included.

## Targets

The status bar shows this document, this sitting, and words added today. The labeled total matches the outliner for the book chosen for compile: each scene, and a chapter only when that chapter has no scenes. Research notes are not included. Today and the sitting use that book's own baselines, so another book's count does not change them. The daily target is still one number for the project. Both targets are editable there and saved in the project. **History** lists completed sittings. A sitting is stored the next time you open the project, when any book added words. Today's count for each book starts again on the next local day.

## Modes

Writing, Corkboard, Outliner, Plot, Characters, World, Research, History. The activity bar lists them in that order. Click the active icon again to hide the binder. **Zen mode** on that bar hides the binder and inspector.

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

Click the active activity-bar icon again to hide the binder, or press **Ctrl+Alt+B**. **Ctrl+Alt+I** shows or hides the inspector. **Ctrl+B** still bolds the selection in the manuscript.

## Themes

The status bar theme list saves **Dark**, **Light**, or **Paper** in the project. Dark matches VS Code Dark+. Light matches VS Code Light+. Paper keeps the warm chrome and sets the manuscript on a cream page.

## Keyboard

| Chord | Action |
| --- | --- |
| **Ctrl+S** | Save |
| **Ctrl+Shift+S** | Snapshot this document |
| **Ctrl+Z** | Undo |
| **Ctrl+Y**, **Ctrl+Shift+Z** | Redo |
| **Ctrl+B** | Bold the selection in the manuscript |
| **Ctrl+F** | Search. On the corkboard, focuses the scene filter |
| **Ctrl+Shift+F**, **Ctrl+\\** | Zen mode |
| **Escape** | Leave zen mode, or clear the corkboard filter, or clear binder search |
| **Ctrl+1** | Writing |
| **Ctrl+2** | Corkboard |
| **Ctrl+3** | Outliner |
| **Ctrl+4** | History |
| **Ctrl+5** | Plot |
| **Ctrl+6** | Characters |
| **Ctrl+7** | World |
| **Ctrl+8** | Research |
| **Ctrl+N** | New scene, when a project is open |
| **Ctrl+Shift+N** | New chapter |
| **Ctrl+O** | Open project |
| **Ctrl+Alt+B** | Show or hide the binder |
| **Ctrl+Alt+I** | Show or hide the inspector |
