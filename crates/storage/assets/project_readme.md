# What these files are

This folder is a book project made with Writing Studio. You do not need that program to read it.

- `manuscript/` holds the book: one HTML file for each scene. Open any of them in a web browser.
- `characters/`, `places/`, `cultures/`, `lore/`, `timeline/`, and `research/` hold the notes about the book, one HTML file for each entry.
- `binder.json` says what order the scenes come in, and which chapter and book each belongs to. The lines are in reading order.
- `bible.json` lists the notes in order, and which scenes each one points at.
- `plot.json` holds the plot lines and their points.
- `project.json` holds the title and the settings that go with the book.
- `attachments/` holds files added to research notes.
- `scene.css` only changes how the HTML files look in a browser.
- `.local/` belongs to one computer: window layout, writing sessions, snapshots, and backups. It is not part of the book and is not shared.

In a scene file, a note on a passage is `<span class="note" data-note="…">` around that passage, and a highlight is `<mark>`.

To move the book to another writing program, export it from Writing Studio as Markdown, or convert the HTML files with a tool such as pandoc.
