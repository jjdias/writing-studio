# Writing Studio project format

Version: **4**, written in `FORMAT` as `writing-studio-project 4`. A project made by an earlier test version, in format 1, 2, or 3, is not opened; see the end of this page.

The files in the project folder are the project. The code that writes a scene is in `crates/format`; the code that writes the folder and keeps it in step with every save is in `crates/storage` (`files.rs`, `mirror.rs`).

## What the format is for

1. **The book can be read without this program.** A scene is a Markdown file: its details at the top, then its text. Any Markdown editor (Obsidian, Typora, iA Writer) or any text editor opens it, and the words are there, in order, with their formatting.
2. **The book can be kept with git.** Text files can be compared and merged. One database file cannot.
3. **What the page shows is what is stored.** The manuscript page edits a scene in this Markdown, and each file is its level's context (title, synopsis, status, beats) above its text.

### Why Markdown is stored

Format 2 stored HTML, because Markdown cannot hold a few things a rich-text page could make: formatting on the spaces at a run's edges, formatting inside a code block, an empty list. Since Phase 12 the page itself is Markdown and never makes them, so storing the same Markdown loses nothing the writer can see, and the files are readable as they are by every Markdown tool. See [Phase 12](../roadmap/done/phase-12.md), stage 8.

## The folder

```text
MyNovel.writingstudio/
  FORMAT                  writing-studio-project 4
  README.md               what these files are, for someone without this program
  project.json            the project's id and title, and settings every machine shares
  binder.json             every binder item: its kind, parent, order, and file
  manuscript/             one Markdown file for each item of the manuscript: scene, chapter, part, book, folder
  notes/                  one Markdown file for each note and folder of notes, under the binder's Notes
    the-harbor-3f9a.md
  characters/             one Markdown file for each character
  places/  cultures/  lore/  timeline/  research/
  bible.json              the order of those entries, which scenes and chapters they point at, who is related to whom
  plot.json               plotlines and plot points
  mindmap.json            the mind map's ideas, lines, and hand-placed nodes, once it has any
  attachments/            research files, as now
    covers/               each book's ebook cover, `<book id>.png` or `.jpg`
  dictionaries/           optional Hunspell dictionaries, as now
  .gitignore              ignores .local/
  .gitattributes          keeps line endings as they are, and research files as files
  .local/                 this machine only; never shared
    cache.sqlite          search, word counts, snapshots, track-changes baselines
    settings.json         theme, panel layout, book on screen, writing sessions
    backups/
```

The Markdown and JSON files are the project. `.local/` can be deleted; the program rebuilds what it can from the files, and what it cannot rebuild (snapshots, backups, writing sessions) was only ever on this machine.

**Markdown folder** in the window exports the whole project as Markdown in the order of the book, for reading it or for another program; see "Markdown export and import".

## Scene files

A scene is written as "One scene" under "Markdown export and import" shows: front matter between two `---` lines, then the text.

### The front matter

- `title` is the scene's title, and `id` the binder item the file is the text of.
- Each other detail is a `key: value` line. Text of several lines is written under `|-`, indented.
- A key with nothing to say is left out. Missing and empty mean the same.
- Keys the program does not know are kept, after the ones it does, and written back on every save; a list stays a list. A key the program knows always comes from the program.
- An item with no text, such as a book, has a file of only front matter. A folder's text, after its front matter, is what the writer says it holds: never compiled or counted. A file with no front matter, only text, is read as a scene with no title; the program gives it an id when it takes the file in. A file whose front matter cannot be read is reported and left out, never written over.

| Key | Meaning |
| --- | --- |
| `id` | The binder item this file is the text of |
| `title` | Its title |
| `synopsis`, `status`, `label`, `color` | As in Context |
| `goal`, `conflict`, `outcome` | The scene's three beats |
| `tags` | Its keywords, a list, nested with `/`, spelled exactly as they are |
| `notes` | What the writer wants to remember about it, on any level; never compiled or counted |
| `footnotes` | `"true"` on a folder of notes compiled as footnotes (Phase 14) |

### The text

The text is Markdown as "Markdown export and import" describes: blocks apart by a blank line, notes and highlights in CriticMarkup, underline as `<u>`, a scene break as `* * *`, an empty paragraph as `<br>` on a line, and what the writer typed never read as formatting. A change to one paragraph changes that paragraph of the file.

### What a scene file does not hold

- The ids of paragraphs and runs. Nothing uses them.
- The time of the last save. Git has it.
- What Markdown cannot say, which the page never makes: see "What the export, and a scene file, leave out".
- A link whose address is not `http`, `https`, `mailto`, or a plain path: the link is taken off and its text is kept.

### File names

`the-harbor-3f9a.md`: the title in lowercase, with accented Latin letters made plain (`été` → `ete`, `ß` → `ss`) and everything else but ASCII letters and digits turned into `-`, at most 40 characters (`untitled` when nothing is left, as for a title in Greek or Japanese), then the first four characters of the id. A file gets its name when it is made. Changing the title, or moving the scene, does not rename the file; the `title` in the front matter is the title.

### What has been checked

- Every kind of block and every piece of formatting is written and read back, with their awkward cases.
- Randomly built documents, made of the characters most likely to break Markdown, are written, read back, and compared, 2,000 with every test run.
- A project on disk is written and read back scene by scene (`crates/format/tests/project.rs`). Set `WS_KEEP_PROJECT` to a folder to keep that project and look at its files.

## Markdown export and import

**The whole project.** **Markdown folder** writes a new folder:

```text
Tide (Markdown)/
  README.md                       the book's title, then every scene and entry, in order, as links
  manuscript/
    01-tide/                      a book
      01-chapter-1/               a chapter that holds scenes
        01-scene-1.md
        02-the-quay.md
  characters/01-mara.md           the bible, one file for each entry
  places/  cultures/  lore/  timeline/  research/
```

The numbers keep files in the order of the book in any file list. An item with text is a file; an item with children is a folder beside it with the same number and name; a chapter with no text of its own is only the folder. The trash is left out. Ids, the plot grid, settings, and `.local/` are not written: this is for reading and for other programs, not for coming back.

**Bringing scenes in.** **Import** in the binder brings `.md` files in as new scenes where a new scene would go, in the order chosen, each with a new id. The front matter gives the title and details; with none, the file's name is the title.

**One scene**, as its file in the project holds it, and as the export and the import write and read it:

```markdown
---
id: 3f9a2c1e-0b7d-4e55-9a10-6c2f8d4b1a77
title: The Harbor
synopsis: |-
  Mara is late.
  The boat is leaving.
status: draft
goal: Reach the boat before the tide
tags:
  - plot/harbor
  - act-two
---

Mara ran for the **north** quay. *Not again*, she thought.

The {==harbor==}{>>Question: which one?<<} was quiet.

“Wait,” said the pilot.\
Nobody waited.
```

- The header is the `---` front matter that other writing tools read: `key: value` lines, quoted only where a YAML reader could misread the value, with text of several lines under `|-`.
- An item's keywords are `tags:`, a list, as Obsidian reads them, nested with `/` as they are; a space in a keyword is written `-`, since a tag cannot hold one. Import reads `tags:` or `keywords:`, a list or one line of words apart by spaces or commas, each without a leading `#`, and gives each a keyword: one in use keeps its spelling, a new one takes the next color. A chapter's keywords are on the chapter's own file, not repeated on its scenes.
- Notes and highlights are CriticMarkup: `{==text==}` is a highlight, `{==text==}{.green}` a colored one, `{==text==}{>>the note<<}` a note, with `Question: `, `Revise: `, `Check: `, and `(resolved) ` in front of the note where they apply.
- Underline is `<u>`. A scene break is `* * *`. An empty paragraph is `<br>` on a line.
- What the writer typed is never read as formatting: characters that mean something to Markdown are written with a backslash.
- Where another Markdown reader would misread `**`, `*`, or `~~`, the file uses `<strong>`, `<em>`, or `<s>`.

**What the export, and a scene file, leave out**, because Markdown cannot say it (the page never makes these):

- Bold, italic, and strikethrough on the spaces at the start or end of the formatted stretch.
- A line break at the very end of a paragraph, heading, quote, or list item.
- Formatting, links, notes, and highlights inside a code block.
- A list with no items.
- A note that crosses formatting or a highlight is written once for each piece.

**Import.** The reader takes the export back, and reads Markdown written elsewhere: wrapped paragraphs, `***` or `---` as a scene break, `*` list items, `<b>` and `<i>`. It reads `_italic_` and `__bold__` (not inside a word, as in `snake_case`), `==text==` as a yellow highlight, and footnotes: `word[^1]` with a line `[^1]: text` becomes a note on the word before the mark, and that line goes. It does not read lists inside lists, quotes inside quotes, inline code, or images; those come in as text.

**The page.** The manuscript page edits a scene in this Markdown: it shows the scene as its file holds it, and reads each change back with this reader.

**Checked against another reader.** The export is read with an independent Markdown reader (`pulldown-cmark`):

- Plain text, 20,000 paragraphs of awkward characters: the same text every time, and nothing read as formatting.
- Formatted prose, 20,000 paragraphs: the same words with the same bold, italic, and links every time.
- Formatting pressed against awkward characters, 20,000 paragraphs: no letter or digit is ever lost. About one paragraph in three hundred shows different bold or italic.

## Other Markdown files

Characters, places, cultures, lore, timeline events, and research notes are files of the same kind: front matter, then the entry's text, written and read by the same code as a scene. The name or title is `title`.

| Folder | Keys in the front matter | Text |
| --- | --- | --- |
| `characters/` | `id`, `role`, `motivation` (a list, in order) | Summary |
| `places/` | `id`, `culture` (an id) | Summary |
| `cultures/` | `id` | Summary |
| `lore/` | `id`, `place`, `culture` (ids) | Body |
| `timeline/` | `id`, `when`, `place` (an id) | Summary |
| `research/` | `id` | Notes |

The text of an entry is plain paragraphs. Its files under `attachments/` are listed in `bible.json`. A book's cover is `attachments/covers/<book id>.png` or `.jpg`: one to a book, used only when the file is a PNG or JPEG picture (checked by its first bytes) of at most 20 MB.

## JSON files

JSON is for structure that is not read as prose. Where a file is a list, each item is on one line, so that adding, moving, or changing an item changes one line.

- **`project.json`** — `format`, `id`, `title`, `created`, and `settings`: the daily and project word targets, each book's word target, the compile profiles, the author and each book's subtitle, and spelling (on or off, the dictionary, and the project's accepted words).
- **`binder.json`** — `items`, one to a line. Each has `id` and `kind`, and `parent` unless it is at the top. An item left out of compile has `"left_out": true`; an item without it is compiled. An item's keywords are in its own file, as `tags`; only a root's would be on its line, as `keywords`. A line with `keywords` and a file whose front matter has no `tags`, as written before keywords moved into the files, is read with the line's keywords, and the next save moves them. Once a project uses them, two more lists follow `items`: `keywords`, one `{ "name": "…", "color": "gold" }` for each keyword in use (the color is a binder color), and `filters`, one `{ "id", "name", "keywords", "any", "without", "status", "text" }` for each saved filter: `keywords` and `without` are lists, a scene must carry all of `keywords` (any one with `"any": true`) and none of `without`, on itself or the chapter, part, or book it is in, each taking in the keywords nested under it with `/`; `entries`, a list of bible entry ids, must each reach the scene through `scene_links` (an entry no longer in the bible matches nothing); `status` is the scene's own; an empty field matches anything. A project without them has neither list, and reading checks what comes in: empty or overlong keywords are left out and a color that is not a binder color is dropped. A `label`, on a binder line or in a scene file's header, is still read from older files and becomes a keyword on that item, which is not written again. A filter with `keyword` (one word) or `label`, as written before filters took several keywords, is refused: the project does not open, and the message says what to change. Every item but the three roots has `file`: its title, synopsis, status, label, color, and keywords are in that file, and the line holds only where the item is. A scene, a chapter, or a note has its text in its file; a book, a part, or a folder has none, and text a person writes under its front matter is kept as it is. A root has its details on its line. A book, a part, or a folder with no `file`, as written before they had files, is read from its line, and the next save gives it a file. A trashed item also has `deleted`, the time it was trashed, and `restore_parent`.
- **`bible.json`** — a list for each kind of entry (`characters`, `places`, `cultures`, `lore`, `timeline`, `research`), each item an `id`, its `file`, `label` when it has one (what it is: "Country", "Father"), and `parent` when it is inside another entry of its kind (a city in a country, a son under his father; an entry without one is at the top, in the list's order); then `research_files`, `relationships` (each with its own `id`, and `from`, `to`, and `kind`: a named connection between any two entries, of any kinds, such as a character who "lives in" a place), and `scene_links` (`from` an entry, to an `item`: a scene, or a chapter, part, or book, whose link reaches every scene in it). A link with `scene` in place of `item`, as written before links could point at a chapter, is refused: the project does not open, and the message says to change `scene` to `item`.
- **`plot.json`** — `plotlines` in order, each with its `points` in order; a point has `title`, `summary`, and an optional `scene`.

- **`mindmap.json`** — `ideas`, `lines`, and `positions`, one to a line. An idea has `id`, `title`, `note`, and `attached` when it hangs from a node (a binder item, another idea); it is never compiled or counted. A line has `id`, `from`, `to`, and `label` when it has a word; its ends are binder items, ideas, or bible entries, and it is neither a binder move nor a story link. A position has `id` (a binder item, an idea, or an entry) and `x` and `y`, the node's center in map units. The file is written once the map has something of its own; a project without it has no ideas, lines, or positions. The cache keeps a copy in `binder_lists`, so a backup holds the map and a restore writes it back. Emptying the trash takes away the lines and positions of what it deleted, and loosens ideas that hung from it. Sync merges each list by `id`.

**Keys the program does not know** are kept and written back: on any line of `binder.json`, `bible.json`, or `plot.json`, on any idea, line, or position of `mindmap.json` and at its top, and in `project.json` and its `settings`. A `relationships` line written before relationships had ids gets one the next time the bible is saved.

**Order is the order of the lines.** Nothing is numbered. In `binder.json` a parent comes before its children, and children are in reading order, so the file reads from the top as the book does. Moving a scene moves one line and changes nothing else. An earlier draft of this format gave each item an order key; line order does the same job with nothing to keep in step.

**Ids.** An item with text has one id, the one in its file. It is also the id of its binder item.

**Paths.** A `file` is a path inside the project folder, with `/` between its parts. A path that is empty, absolute, or holds `..` is refused, and the project does not open. So is a path with a symbolic link on the way that leads out of the project folder, or to nothing; a link that stays inside the project is fine.

**Times.** Apart from `created` and `deleted`, no file holds a time. Records read from files have the time they were read.

## This machine only

`.local/settings.json` holds every setting that is not in `project.json`: the theme, the manuscript font, panel layout, the book on screen, whether the whole series is shown, the autosave delay, and writing sessions. A project opened where there is no `.local/` uses the usual ones.

`.local/cache.sqlite` is a database the program fills from the files every time the project opens. While the program runs it answers searches and word counts. It also holds what is this machine's alone: snapshots and track-changes baselines. `.local/backups/` holds copies of that database.

## What is written when

- **A save writes the file first, then the cache.** The cache never holds writing that is not in a file.
- Every file is written whole under a temporary name and then renamed over the old one. A crash during a save leaves the old file or the new one.
- Saving a scene writes that scene's file. Changing a title, synopsis, status, label, color, or beat writes the file of that item. Other scene files are not read or written.
- `binder.json`, `bible.json`, `plot.json`, `project.json`, and the bible's files are written only when what they hold has changed.
- `README.md`, `.gitignore`, and `.gitattributes` are written when they are missing and then left alone, so they can be edited.
- **A file changed outside the program is never written over.** Before writing or deleting a file, the program checks that it still holds what the program last read or wrote. If not, what it holds is kept beside it as `name.conflict-<time>.ext`. When the window comes back to the front, the files are checked and read again if one changed and nothing is unsaved: the open scene's file alone, or the whole project when a list or a bible entry changed.
- **Opening copes with files changed outside.** A scene whose file is missing, or holds another item's id, stays in the binder marked missing, and its file is never written over. A bible entry whose file is missing leaves the bible. A Markdown file in `manuscript/` or a bible folder that no list names comes in, a scene into a Recovered folder at the top of the manuscript. An item whose parent is missing, or that is in a loop, moves to Recovered. What was repaired is written back at once, and History lists it. A JSON file that cannot be read, a path that leaves the project, or a binder without its three roots stops the project from opening, and the message names the file.
- **Opening reads the files.** The cache is filled from them each time, so a scene edited in a text editor, or brought in by git, is what opens. A file that cannot be read stops the project from opening, and the message names the file.

## What deletes a file

- Emptying the trash deletes the files of the scenes in it. This is the step that already asks twice.
- Deleting a character, a place, a culture, a lore entry, a timeline event, or a research note deletes its file.
- Nothing else does. A file in the folder that no list mentions is left alone and is not part of the project.

## The cache

`.local/cache.sqlite` is filled from the files every time the project opens, and every save writes it after the file. It is never the only copy of anything in the files; deleting it loses only what is this machine's alone: snapshots and track-changes baselines. Its tables:

- `project_meta`: id, format version, title, times, and `settings_json`, every setting (see below).
- `binder_nodes`: id, parent, kind, title, position among its siblings, document id (the item's own id, for an item with text), synopsis, status, `label`, `color`, `deleted_at`, `restore_parent_id`, `keywords`, `left_out`, `notes`.
- `documents`: `content_json` (the document JSON below), word count, character count.
- `scene_beats`: `node_id`, `goal`, `conflict`, `outcome`. A scene with none written has no row.
- `plotlines`, `plot_points`; `characters`, `character_motivations`, `character_relationships`; `locations`, `cultures`, `lore_entries`, `timeline_events`; `research_items`, `research_files`; `bible_parents`, which entry each entry is inside; `bible_labels`, each entry's label; `bible_scene_links`, which bible entries point at which binder items (its `scene_id` column holds a chapter's id as well as a scene's).
- This machine's only: `snapshots`, copies of a document with a label (`manual`, `pre-restore`, …); `tracking_baselines`, the document as it was when tracking began or at the last accept.

Missing tables are added when a project opens (`CREATE TABLE IF NOT EXISTS`).

## Settings

`project.json` holds the settings that go with the book:

- `daily_word_goal`, `project_word_goal`, and `book_word_goals`, one `{ "book_id": "…", "goal": 80000, "deadline": "2027-01-31" }` for each book with its own target. `deadline` is optional, a `YYYY-MM-DD` day; a book with none leaves it out, and an older file without it reads the same. The first book with no entry uses `project_word_goal`; later ones use 80000.
- `compile_profiles`, the compile profiles: each `{ "id": "…", "name": "Manuscript", "format": "docx", "epub", "html", or "markdown", "title_page": true, "page": { "font": "Times New Roman", "font_size": 12, "line_spacing": "single", "one_and_half", or "double", "size": "letter" or "a4", "margin": 10, "indent": true, "chapter_new_page": true }, "chapters" and "parts": "number_and_title", "number", "number_words", or "title", "chapter_word": "Chapter", "part_word": "Part", "scenes": "break", "blank", "title", or "nothing", "matter": { "copyright": false, "copyright_text": "Copyright © {year} {author}. All rights reserved.", "dedication": false, "dedication_text": "", "contents": false, "about_author": false, "about_author_text": "", "also_by": false, "also_by_text": "", "contents_heading": "", "about_author_heading": "", "also_by_heading": "" }, "language": "english", "portuguese", "spanish", or "french" }`. An empty heading means the language's own; a missing language means English. The copyright line and the dedication are up to 400 characters, about the author and the list of other books up to 4000; the copyright is one line, the others keep their line breaks. The words before a number are up to 24 characters, in the book's language. `font_size` is 8 to 24 points and `margin` 2 to 30 tenths of an inch; a value outside is brought inside, and a missing one takes the default. Missing or empty means the built-in profiles, Manuscript, Plain text, Web page, and Ebook, which keep fixed ids so every computer agrees on them. An older file's `compile_title_page` is no longer read.
- `author`, the writer's name as the books print it, up to 120 characters, and `book_subtitles`, one `{ "book_id": "…", "subtitle": "A Novel" }` for each book with a subtitle, up to 200 characters. Both are missing in an older file, which means none.
- `spelling`: `enabled`, `language` (a dictionary name such as `en_US`), and `words`, the project's accepted words, lowercase and sorted.

`.local/settings.json` holds the rest, this machine's:

- `theme` (`dark`, `light`, or `paper`; missing means `light`), `prose_font` (`serif` or `sans`, the manuscript's typeface; missing means `serif`), `keep_place` (`scroll`: `off`, `top`, `middle`, or `bottom`; `line_marker`: true or false; `fade`: `off`, `paragraph`, or `sentence`; `quiet_bars`: true or false; missing means fixed scrolling, the marker, and fading are off and the bars fade), `page` (`line_length` in characters, 45 to 100; `line_height` in hundredths, 120 to 220; `paragraph_spacing` in tenths of a line, 0 to 20; `first_line_indent` in tenths of an em, 0 to 40; each missing value keeps the default look), `autosave_seconds`, `compile_book_id` (the book on screen, compiled, and counted in the status bar), `compile_choices` (the profile last used for each book, `{ "book_id": "…", "profile_id": "…" }`), and `show_whole_series`.
- `layout`: `sidebar_side` (`left` or `right`; kept for older projects, but the binder is always on the left now), `sidebar_width` (180 to 480 pixels), `inspector_width` (220 to 560), and whether each panel is visible.
- `writing`: today's date and each book's word count at the start of today and of the open sitting, and the last 40 completed sittings (`day`, `started_at`, `ended_at`, `words`, `seconds`). A sitting is stored the next time the project opens.

## Document JSON

A scene's text, as the program holds it and as the cache stores it, is a list of blocks; `schema_version` is 1. Block types are `paragraph`, `heading` (with `level`), `quote`, `list` (with `ordered` and `items`, each item a list of runs), `code`, and `scene_break`. A block holds runs of text, each with marks: `bold`, `italic`, `underline`, `strikethrough`, an optional `link`, an optional `comment` with `comment_kind` (`comment`, `question`, `revise`, or `check`) and `comment_resolved`, and an optional `highlight` (`yellow`, `green`, `blue`, or `pink`). Tracked changes are not in the document; they are the difference from a baseline.

```json
{
  "id": "…",
  "schema_version": 1,
  "blocks": [
    { "id": "…", "kind": { "type": "heading", "level": 1, "inlines": [
      { "id": "…", "text": "Dawn", "marks": { "bold": false, "comment": "Check the tide", "comment_kind": "check" } }
    ] } },
    { "id": "…", "kind": { "type": "scene_break" } }
  ]
}
```

The scene file holds the same document as Markdown; see "Scene files".

## Projects from earlier versions

Test versions kept a project in one SQLite file (format 1), then each scene as an HTML file (format 2), then characters, places, and research apart from notes (format 3). There were no downloads of either, so this version does not convert such a project. Opening a format 2 project says it was made by an earlier test version that kept each scene as an HTML file, and writes nothing in the folder. For format 1: Opening one says it was made by an earlier test version, and writes nothing in the folder. Making a new project in that folder is refused too, so the old database is never written over.

Within format 2, a project made before saved filters took several keywords has filters with `keyword`, a single word, in `binder.json`. Those are not converted either: the project is refused with a message naming the change (`"keyword": "Flashback"` becomes `"keywords": ["Flashback"]`). The same goes for a link in `bible.json`'s `scene_links` with `scene`, from before links could point at a chapter: it becomes `item`, with the same value.

## Questions still open

- Whether the whole book is also offered as one Markdown file, beside one file for each scene.
- Whether a scene's snapshots should be offered to another machine. They are this machine's only.
- Lists inside lists. The editor cannot make them today, so the format does not hold them.
