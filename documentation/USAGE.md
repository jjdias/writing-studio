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

## Modes

Writing, Corkboard, Outliner, History. **Focus** hides the binder and inspector.
