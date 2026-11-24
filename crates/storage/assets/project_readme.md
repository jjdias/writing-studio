# What these files are

This folder is a book project made with Writing Studio. You do not need that program to read it.

- `manuscript/` holds the book: one Markdown file for each scene, and for each chapter with text of its own. Open any of them in a Markdown editor such as Obsidian, Typora, or iA Writer, or in any text editor.
- `characters/`, `places/`, `cultures/`, `lore/`, `timeline/`, and `research/` hold the notes about the book, one Markdown file for each entry.
- `binder.json` says what order the scenes come in, and which chapter and book each belongs to. The lines are in reading order.
- `bible.json` lists the notes in order, and which scenes each one points at.
- `plot.json` holds the plot lines and their points.
- `project.json` holds the title and the settings that go with the book.
- `attachments/` holds files added to research notes.
- `.local/` belongs to one computer: window layout, writing sessions, snapshots, and backups. It is not part of the book and is not shared.

Each file starts with its details between two `---` lines (its title, synopsis, status), then its text.

In the text, a note on a passage is `{==the passage==}{>>the note<<}`, and a highlight is `{==the passage==}`, with its color after it as `{.green}` when it is not yellow (CriticMarkup).
