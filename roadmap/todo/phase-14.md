# Phase 14 — Notes

Not started.

Characters, places, cultures, lore, timeline events, research notes, and the binder's story notes become one thing: a **note**. A note is a Markdown file the writer writes as they like, in the same page as a scene, in folders the writer makes and names. Nothing about it is a form. The program knows a note's title, its keywords, and what it links to; everything else is the writer's text.

A new project's notes start with two folders, which the writer can rename or delete:

- **Footnotes**, compiled into the book: a footnote mark in the text becomes a numbered footnote with the note's text, as a non-fiction book has.
- **Research**, never compiled, but linked like any other note.

The aim is a simpler program to build and to use: one kind of note instead of six kinds of entry in three crates, each with its own fields, files, view, and rules. A book-type preset (non-fiction with footnotes and research; fantasy with characters, places, and lore) is then only a set of folders and templates, for a later phase.

## Where things are now

- `crates/characters`, `crates/worldbuilding`, and `crates/research` each hold their own kinds of entry, with fixed fields (a character's role and motivations, a place's culture, a timeline event's date, a research item's files), and the window has a view for each (Characters, World, Research) and forms to fill in.
- `bible.json` orders the entries, and holds their links to scenes and chapters (`scene_links`), their nesting, labels, and named connections. Each entry has a `.md` file in its own folder.
- The binder has a Research root with **Story notes**, a binder note opened in the page.
- The Phase 11 worldbuilding trees in the binder, the mind map's worldbuilding view, a scene's context, "Named here", the diagnostics' names, and spelling's accepted names all read the entries.

## Decisions

Made by the maintainer:

- **One kind of note, in folders the writer controls.** Two folders to start, Footnotes (compiled) and Research (not compiled), both the writer's to change or delete.
- **No forms.** A note is opened in the Markdown page and written freely. A new note can start from a **template**: a Markdown file whose headings guide a good shape, for the writer and for an AI reading the notes, and which the writer is free to change.
- **No conversion.** The program is pre-alpha with no users. A project from before this phase is refused with a message, as the roadmap's rule asks; test projects are made again. v0.1.0 waits for this phase.

Recommended in this plan:

- **Notes are binder items**, under a **Notes** root that replaces the Research root, as scenes are under the Manuscript. They get what binder items already have: a file, keywords, notes in Context, nesting, the trash, drag and drop, the page, search, and sync.
- **A note's name is its link.** `[[Mara]]` in a scene or another note links to the note titled Mara, as in Obsidian. Links that are not a mention in the text are made from Context, as story links are now.
- **A footnote has a place in the text.** A link to a note in a folder that compiles as footnotes is a footnote mark: compile puts a number there and the note's text at the foot of the page, or at the end in an ebook. The same note can be cited in many places.

## What this phase does not take

- **Book-type presets.** Folders and templates for non-fiction, fantasy, and others come later; this phase gives a new project its two folders and a few templates.
- **Fields the program reads.** A character's role or an event's date is text in the note, not data the program sorts by. The timeline's dated list goes with it; a writer who wants one writes it as a note.
- **The plot grid.** Plotlines and plot points stay as they are: they are a different tool, and they already link to scenes.

## The work

### 1. Notes in the binder, and format 4

- `NodeKind::Notes` (a root, replacing `ResearchRoot`), `NodeKind::NoteFolder`, and `NodeKind::Note`. A folder holds notes and folders; a folder has one setting, **compiled as footnotes**.
- Each note is `notes/<name>-<id>.md`: front matter (`id`, `title`, `tags`, `notes`, and `links`, the binder items it is linked to from Context), then its text. A folder's file holds its title and setting, as a book's does since Phase 13.
- A new project's Notes root holds Footnotes (compiled) and Research (not compiled).
- `FORMAT` is 4. A format 3 project is refused with a message, and nothing in it is written.

### 2. A note is a page

- Choosing a note opens it in the Markdown page, with everything the page does: live preview, spelling, notes and highlights, keeping one's place.
- Context shows its title, keywords, notes, and links, and what links to it (below).
- A file can be added to a note: it is copied into `attachments/` and a link to it is put in the text, as research files are kept now.

### 3. Templates

- `templates/` in the project folder: one `.md` file for each template, synced with the project, edited like any note.
- A new project gets a few: Character, Place, Source, and Research note, each a title and headings with a line under each saying what goes there.
- Right-click a notes folder for **New note from template**, with the templates listed; right-click a note for **Save as template**. **New note** with no template is an empty page.

### 4. Links between anything

- `[[Title]]` anywhere (a scene, a note, a chapter's text) links to the note of that title. On the page it reads as a link, with the note's first lines as its title; clicking it, or Ctrl+Enter on it, opens the note; typing `[[` offers the notes' titles. A link to a title no note has is shown as such.
- Context's links (to a scene, chapter, part, or book, as story links are now) are kept in the note's front matter.
- Context shows a note's **backlinks**: every scene and note that mentions it, and every item it is linked to.
- Renaming a note offers to change every `[[old title]]` to the new one, as one undo step.

### 5. Footnotes in the compiled book

- A `[[Title]]` whose note is in a folder compiled as footnotes is a footnote mark. Compile writes a number there and the note's text as the footnote: a Word footnote in DOCX, an endnote in the ebook and the web page, `[^n]` in Markdown. Each mark is its own footnote.
- Other `[[Title]]` links compile as their text.
- The check before compiling lists a mark whose note is missing.

### 6. Everything that read the bible reads notes

- A scene's and a level's context (Phases 11 and 13) list the notes linked to them or mentioned in them.
- "Named here", the diagnostics, and spelling's accepted names use every note's title.
- The mind map's worldbuilding view becomes a notes view: notes, their folders, and the links between them.
- Search finds notes, as it finds scenes.

### 7. What goes

- `crates/characters`, `crates/worldbuilding`, and `crates/research`; `bible.json`; the Characters, World, and Research views and their forms (Ctrl+5 to Ctrl+7, with the shortcuts after them renumbered); the Phase 11 worldbuilding trees in the binder; the binder's Research root and its story notes.
- `documentation/USAGE.md`, `PROJECT_FORMAT.md`, and `ARCHITECTURE.md` describe notes, folders, templates, links, and footnotes.

## How changes are checked

- A note, a folder, and a template are written and read back with their front matter, and keys the program does not know are kept.
- `[[Title]]` is found in text by the format crate, and a rename changes every mention and nothing else.
- Compile is tested for footnotes in each format: the numbers, the note's text, a note cited twice, a missing note, and links to notes not compiled.
- Context, "Named here", and the diagnostics are tested on a small project of notes.
- A format 3 project is refused and left alone.
- How the Notes view and the page look is the maintainer's to check, as in earlier phases.
