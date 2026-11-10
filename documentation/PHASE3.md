# Phase 3 — what shipped

Phase 3 is **local story development** for a series. One project folder holds every related book and one bible: plot, characters, world notes, and research. Nothing in this phase needs an account, a server, or a network connection.

The program stays **open source** (MIT OR Apache-2.0) and **free for the community**.

This work landed from `phase-3` onto `master`. How to use it: [USAGE.md](USAGE.md). On-disk layout: [PROJECT_FORMAT.md](PROJECT_FORMAT.md).

---

## Goals that Phase 3 met

1. **One series project.** The bible exists in one place, beside the books, and is never copied into each book.
2. **One current book.** Compile, the word target, today, and the sitting follow that book. The binder, corkboard, outliner, and search can show it or the whole series.
3. **The bible can point at scenes.** Plot points, characters, world records, and research notes can name manuscript scenes. The records stay with the project.
4. **One research library.** Research is the only editor for research notes and attached files.

---

## Features delivered

### Series

- A blank novel starts as one book. **Book** adds another novel under the same manuscript.
- **Whole series** shows every book. Choosing a book makes it the one on screen, the one compiled, and the one in the status bar. The project remembers that choice.
- Selecting a scene points compile and the status bar at that scene's book without leaving whole-series view.
- New chapters, parts, scenes, and manuscript folders land inside the book on screen.
- Scene lists use book / chapter / scene, so the same chapter title in two books stays distinct.
- Markdown and DOCX compile the chosen book. The title page is that book's title. Other books are left out.
- The status bar total matches the outliner for that book: each scene, and a chapter only when it has no scenes. Today and the sitting use baselines stored for that book.
- Each book can keep its own word target. Permanently deleting a book removes that target. If it was the compile book, compile and the status bar move to the first book that remains.
- The daily target stays one number for the project.

### Bible

- **Plot** is a grid of plotlines and plot points. A point can link a scene.
- **Characters** keep profiles, motivations, and relationships.
- **World** keeps locations, cultures, lore, and a timeline.
- **Research** keeps notes and files copied into `attachments/`.
- Characters, locations, cultures, lore, timeline events, and research notes can link scenes. Emptying trash drops links to scenes that were permanently deleted.
- Opening a project copies notes that still sit under the binder Research section into the research library, then removes those notes and research folders from the binder. The binder does not edit research.

---

## Explicitly not in Phase 3

- A separate bible project beside each book (bonus)
- Backups that include `attachments/` (bonus). A SQLite backup still leaves research files behind; copy the project folder to keep both.
- Front matter richer than the book title (Phase 6)
- Snapshot comparison, comments, track changes, and spellcheck (Phase 4)
- Cloud sync, accounts, or real-time collaboration (Phase 5)

Those stay optional. They must not become required to write.
