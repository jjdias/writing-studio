# Phase 14 — Notes

In progress: stages 1 to 6 are done.

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

**Done.** The binder's Research root is the Notes root (`NodeKind::NotesRoot`, `notes_root` in `binder.json`, "Notes" in the binder); it holds notes and folders, as it did. A folder has `footnotes` (`BinderNode::footnotes`, `footnotes: "true"` in its front matter, a cache column, merged by sync), set by **Compile as footnotes** in Context for a folder under Notes. Files of what is under Notes are in `notes/` (`folder_of`), and a note file that no line names comes into the Notes root when the project opens. A new project's Notes hold Footnotes (compiled) and Research, and no longer the Story notes note. Opening a project no longer moves the binder's notes into the research library (`absorb_binder_research` is gone), which would have taken every note out of the binder. An item with no text, such as a folder or a book, now has a file of front matter only, without the `<br>` of an empty paragraph. `FORMAT` is 4, and a format 3 project is refused with its own message. The bible's entries are still there until stage 7. Test: `notes_are_files_in_notes_and_a_new_project_has_two_folders`.

### 2. A note is a page

- Choosing a note opens it in the Markdown page, with everything the page does: live preview, spelling, notes and highlights, keeping one's place.
- **A folder is a page too**, decided by the maintainer: a folder (of notes, or in the manuscript) is a Markdown file with text of its own, where the writer says what the folder is about and what is in it, as a first context for what is nested inside. It is never compiled or counted, and it is part of the folder's context (Phase 13): **Copy context** gives it first, under **About**.
- Context shows its title, keywords, notes, and links, and what links to it (below).
- A file can be added to a note: it is copied into `attachments/` and a link to it is put in the text, as research files are kept now.

**Done.** A note, and now a folder, opens in the page when chosen, with everything the page does. A folder holds text (`NodeKind::can_hold_document`), in the manuscript or under Notes: compile walks into a folder without printing its text, and the word counts read chapters and scenes only, so it is never compiled or counted. A folder's context (Phase 13) shows its text, and **Copy context** gives it first, under **About** (`LevelContext::about`, filled by `StudioState::level_context`); Context shows a level's context for folders under Notes as well as in the manuscript. **Add a file…** in Context, for the open note, copies a file into `attachments/<note id>/` (`ProjectStore::attach_file`) and puts a Markdown link to it at the caret (`wsEditor.insertText`). A note's links and backlinks in Context come with stage 4. Test: `a_folder_has_text_of_its_own_that_is_context_and_never_compiled` (a folder's page and About, a manuscript folder left out of compile and the count, and a file added to a note).

### 3. Templates

- `templates/` in the project folder: one `.md` file for each template, synced with the project, edited like any note.
- A new project gets a few: Character, Place, Source, and Research note, each a title and headings with a line under each saying what goes there.
- Right-click a notes folder for **New note from template**, with the templates listed; right-click a note for **Save as template**. **New note** with no template is an empty page.

**Done.** `templates/` in the project holds the note templates, one `.md` file each, written once when the project is made (`NOTE_TEMPLATES` in `crates/templates`: Character, Place, Source, and Research note, each a title, a keyword, an opening line, and headings with a line under each), so a template the writer deletes stays deleted. Right-click a folder of notes for **New note from template**, with the templates by title, or **Empty note**: the new note takes the template's title, keywords, and text, and opens in the page (`StudioState::new_note_from_template`). Right-click a note for **Save as template** (`save_note_as_template`), which writes `templates/<title>.md` and never over another template. `ProjectStore::note_templates` reads the folder when the menu opens, leaving out a file whose front matter cannot be read. Test: `notes_start_from_templates_and_a_note_can_become_one`.

### 4. Links between anything

- `[[Title]]` anywhere (a scene, a note, a chapter's text) links to the note of that title. On the page it reads as a link, with the note's first lines as its title; clicking it, or Ctrl+Enter on it, opens the note; typing `[[` offers the notes' titles. A link to a title no note has is shown as such.
- Context's links (to a scene, chapter, part, or book, as story links are now) are kept in the note's front matter.
- Context shows a note's **backlinks**: every scene and note that mentions it, and every item it is linked to.
- Renaming a note offers to change every `[[old title]]` to the new one, as one undo step.

**Done.** `crates/format/src/links.rs` finds `[[Title]]` and `[[Title|words]]` (`wikilinks`), matches titles whatever their case (`same_title`), and renames them (`rename_wikilinks`); the writer writes a link as typed and the reader keeps it as text. On the page (`syntax.js`, `preview.js`, `writing.js`) a link reads as a link, its brackets hidden except where the caret is, with the note's folder and first line as its tip, and a title no note has is shown faint; typing `[[` offers the notes' titles (`@codemirror/autocomplete`), a click opens the note (Ctrl+click in Source mode), and so does Ctrl+Enter. A note's links from Context are `BinderNode::links`, kept as `links:` in its front matter (a cache column, merged by sync). Context for a note shows **Linked to**, with **Link to…** and **Unlink**, and **Mentioned in**, from `StudioState::link_index`: which titles each text links to, filled when the project opens and kept up as the open text changes, so it never reads every file. Renaming a note changes every link to it at once, as Obsidian does by default, rather than offering to; the other files are saved with the change, so it is not one undo step. Tests: the format crate's link tests, and `notes_are_linked_by_title_and_from_context_and_renaming_follows`; the page was checked in a browser.

### 5. Footnotes in the compiled book

- A `[[Title]]` whose note is in a folder compiled as footnotes is a footnote mark. Compile writes a number there and the note's opening text, up to its first heading, as the footnote, so what is under a heading stays the writer's (the Source template is laid out so): a Word footnote in DOCX, an endnote in the ebook and the web page, `[^n]` in Markdown. Each mark is its own footnote.
- Other `[[Title]]` links compile as their text.
- The check before compiling lists a mark whose note is missing.

**Done.** `resolve_note_links` (`crates/export`) resolves every `[[link]]` in the documents compile reads: a link to a note under a folder compiled as footnotes (the folder or one it is in) sets `Marks::footnote`, compile-only and never saved, on the link's words, or with no words of its own on the text before it, so the number follows that word; its text is the note's opening, up to its first heading, as one paragraph with its own links read as their words. Any other link reads as its words. `Marks::reader_note` gives a reader a run's footnote and then its open note, so the Markdown compile writes `[^n]` and the web page and the ebook their numbered notes as before; DOCX writes a real Word footnote (`docx_rs::Footnote`), and open notes stay bracketed. Each mark is its own footnote. The check before compiling lists **Links to notes that are not there** (`CheckFinding::MissingNotes`). A footnote is plain text: italics in a citation are not kept. Test: `a_link_to_a_footnote_note_is_a_footnote_and_other_links_are_their_words` (Markdown, the web page, Word, and the check).

### 6. Everything that read the bible reads notes

- A scene's and a level's context (Phases 11 and 13) list the notes linked to them or mentioned in them.
- "Named here", the diagnostics, and spelling's accepted names use every note's title.
- The mind map's worldbuilding view becomes a notes view: notes, their folders, and the links between them.
- Search finds notes, as it finds scenes.

**Done.** `crates/context` reads notes: `Sources` gives a note's text and the titles an item's text links to, and a scene's or a level's context lists the notes linked to it, mentioned in it, or linked to what it is in, grouped by their folder (`ContextEntry::folder`), so a folder named Characters is the Characters of the context. The spelling's accepted names, the diagnostics' names, and "Named here" are every note's title (`StudioState::note_titles`); "Named here" finds a title by any capitalized part of it, as the diagnostics find a character, so a note called Rose is not named by "rose". Context on a scene or chapter is **Notes in this …** (`NotesHere`, `notes_reaching`, `link_note_titled`): the story links' place, for notes. The mind map's other view is **Notes**: the book's map with the Notes root at its middle, where Tab adds a note. Found on the way: the library never showed Notes, since the old Research root's notes used to be moved into the research library on opening; it now has a **Notes** section (`note_rows`, `add_in_notes`), narrowed by the search, and a click opens a note or a folder in the page. Tests: `a_note_mentioned_in_a_scene_is_in_its_context` (`crates/context`, whose fixture is now notes), and in the window's state the spelling, diagnostics, "Named here", scene context, notes map, and `the_library_lists_notes_and_a_search_narrows_them`.

### 7. What goes

- `crates/characters`, `crates/worldbuilding`, and `crates/research`; `bible.json`; the Characters, World, and Research views and their forms (Ctrl+5 to Ctrl+7, with the shortcuts after them renumbered); the Phase 11 worldbuilding trees in the binder; the binder's Research root and its story notes.
- `documentation/USAGE.md`, `PROJECT_FORMAT.md`, and `ARCHITECTURE.md` describe notes, folders, templates, links, and footnotes.

**Done.** `crates/characters`, `crates/worldbuilding`, and `crates/research` are gone, with `bible.json`, its tables in the cache, and the Characters, World, and Research views; History is **Ctrl+5**. Gone with them: the library's Story and Worldbuilding sections, Context's entry page and connections, the mind map's story entries and its worldbuilding map (its other middle is Notes), and narrowing the views to an entry. What stays, on notes: a saved filter's **With** lists notes, and a scene matches when the note reaches it (`StudioState::note_reaches`: linked to it or to what it is in, or mentioned in its text); merging two scenes gives the kept scene the joined one's note links, as it did characters' and places'; emptying the trash takes notes' links to what it deleted away. A file added to a note is named by the store (`attachments/<note id>/<id>.<ext>`). The diagnostics' "No character named" is "No note is named". `USAGE.md`, `PROJECT_FORMAT.md`, and `ARCHITECTURE.md` describe notes, folders, templates, links, and footnotes, and the bonus entry "Keywords on the story's entries" is gone, as notes carry keywords. Tests: `a_note_linked_to_a_chapter_reaches_its_scenes`, and the merge, map, and sync tests on notes.

## How changes are checked

- A note, a folder, and a template are written and read back with their front matter, and keys the program does not know are kept.
- `[[Title]]` is found in text by the format crate, and a rename changes every mention and nothing else.
- Compile is tested for footnotes in each format: the numbers, the note's text, a note cited twice, a missing note, and links to notes not compiled.
- Context, "Named here", and the diagnostics are tested on a small project of notes.
- A format 3 project is refused and left alone.
- How the Notes view and the page look is the maintainer's to check, as in earlier phases.
