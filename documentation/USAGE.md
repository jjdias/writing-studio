# Using Writing Studio (Phase 1)

Free, open-source, local-first. Your writing stays in a folder on your machine.

## Start

1. `cargo run -p writing-studio-desktop`
2. **New project** — pick an empty parent folder. The app creates `Title.writingstudio`.
3. **Open project** — select an existing `.writingstudio` folder.

## Write

- Select a scene in the binder. Type in the manuscript. Formatting buttons apply bold, italic, underline, strike, heading, quote, and lists.
- Changes save a few seconds after you stop typing, when you leave the field, or with **Ctrl+S**.
- **Ctrl+Z** / **Ctrl+Y** undo and redo in the document session.

## Organize

- **Chapter**, **Scene**, **Note** add items.
- Drag items to reorder or nest. Drop on **Trash** to discard.
- **Restore** returns a trashed item to its previous parent when that parent still exists.
- **Empty trash** asks for a second click, then deletes those items from the local database.

## Recover

- Inspector → **Snapshot this document**. Restore from the inspector list or **History**.
- **Backup** writes `backups/project-….sqlite`. Restore from **History** (second click). A safety copy of the live file is written first.

## Compile

- **Markdown** or **DOCX** in the top bar. Choose where to save.
- Only the manuscript (not research, not trash) is included.

## Targets

The status bar shows this document, this sitting, words added today, and the manuscript total. The daily and project targets are editable there and saved in the project. **History** lists completed sittings. A sitting is stored the next time you open the project, when it added words. Today’s count starts again on the next local day.

## Modes

Writing, Corkboard, Outliner, Plot, Characters, World, Research, History. **Ctrl+5** opens Plot. **Ctrl+6** opens Characters. **Ctrl+7** opens World. **Ctrl+8** opens Research. **Focus** hides the binder and inspector.

## Plot

**Plot** is a local grid of plotlines. Add a plotline, then add plot points along that line. Each point has a title, a short summary, and an optional link to a manuscript scene. **Open** jumps to that scene. **Earlier** and **Later** change the order on the line. Deleting a plotline asks for a second click and removes its points. The grid is saved in the project folder. Emptying trash drops links to scenes that were permanently deleted.

## Characters

**Characters** keeps a local cast. Each profile has a name, a role, and a longer description. Motivations are a list on that character. A relationship names another character and how they are connected. Deleting a character asks for a second click and also removes their motivations and relationships. The cast is saved in the project folder.

## World

**World** keeps locations, cultures, lore, and a timeline in the project. A location can name a culture. A lore entry can name a location and a culture. A timeline event has a when-label, a title, what happened, and an optional location. **Earlier** and **Later** change event order. Deleting an entry asks for a second click and clears links that pointed at it.

## Research

**Research** is a local library of notes. Each note has a title, notes, and files. **Attach file** copies the file into `attachments/` inside the project folder. **Open** launches it with the usual app for that file type. Removing a file or a note asks for a second click and deletes the copy in `attachments/`. A sqlite backup does not include those files; copy the whole project folder to keep them.

## Layout

The arrangement is saved in the project. Drag the edge of the binder or the inspector to resize it. Double-click that edge to reset the width. The arrow in the binder header moves the binder to the other side; the inspector stays on the opposite side.

Click the active activity-bar icon again to hide the binder, or press **Ctrl+Alt+B**. **Ctrl+Alt+I** shows or hides the inspector. **Ctrl+B** still bolds the selection in the manuscript.

## Themes

The status bar theme list saves **Dark**, **Light**, or **Paper** in the project. Dark matches VS Code Dark+. Light matches VS Code Light+. Paper keeps the warm chrome and sets the manuscript on a cream page.
