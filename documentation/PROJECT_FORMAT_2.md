# Writing Studio project format 2 (draft)

Status: **draft**. The program still reads and writes format 1 ([PROJECT_FORMAT.md](PROJECT_FORMAT.md)). This is the format Phase 5 moves to. See [ROADMAP.md](../ROADMAP.md).

Each section says how far it has got:

- **Built and tested**: scene files. The code is in `crates/format`.
- **Specified**: everything else. It is written down here so it can be argued with, and is built in the next stage.

## What the format is for

1. **The book can be read without this program.** A scene is a Markdown file. Open it in any text editor, or in another writing tool, and the words are there, in order, with their formatting.
2. **The book can be kept with git.** Text files can be compared and merged. One database file cannot.
3. **Nothing is lost on the way in or out.** What the program writes, it reads back as the same document.

## The folder

```text
MyNovel.writingstudio/
  FORMAT                  writing-studio-project 2
  README.md               what these files are, for someone without this program
  project.json            the project's id and title, and settings every machine shares
  binder.json             every binder item: its kind, parent, order, and file
  manuscript/             one Markdown file for each scene, and each chapter that holds text
    the-harbor-3f9a.md
  characters/             one Markdown file for each character
  places/  cultures/  lore/  timeline/  research/
  plot.json               plotlines and plot points
  links.json              which bible entries point at which scenes; who is related to whom
  attachments/            research files, as now
  dictionaries/           optional Hunspell dictionaries, as now
  .gitignore              ignores .local/
  .local/                 this machine only; never shared
    cache.sqlite          search, word counts, snapshots, track-changes baselines
    settings.json         theme, panel layout, book on screen, writing sessions
    backups/
```

The Markdown and JSON files are the project. `.local/` can be deleted; the program rebuilds what it can from the files, and what it cannot rebuild (snapshots, backups, writing sessions) was only ever on this machine.

## Scene files — built and tested

A scene file is a header, a blank line, and the scene.

```markdown
---
id: 3f9a2c1e-0b7d-4e55-9a10-6c2f8d4b1a77
title: The Harbor
synopsis: |-
  Mara is late.
  The boat is leaving.
status: draft
goal: Reach the boat before the tide
outcome: The boat has gone
---

Mara ran for the **north** quay. *Not again*, she thought.

“Wait,” said the pilot.\
Nobody waited.
```

### The header

The header is the `---` block that other writing tools call front matter. It uses a small part of YAML.

- Each line is `key: value`. A value is a piece of text, or a list of pieces of text.
- A value is written bare when no YAML reader could take it for anything else. Otherwise it is in double quotes, with `\"`, `\\`, `\n`, and `\t`. `title: "The Harbor: Dawn"`, `label: "1984"`.
- Text of several lines is written under `|-`, indented by two spaces.
- A list is the key alone, then one `  - item` line for each item.
- A key with nothing to say is left out. Missing and empty mean the same.
- Keys the program does not know are kept, in their place after the ones it does, and written back.
- A file with no header is read as a scene with no title. The program gives it an id when it takes the file in.

The reader also accepts single-quoted values, `#` comments, `|` and `|+` blocks, a byte-order mark, and Windows line endings.

Keys of a scene:

| Key | Meaning |
| --- | --- |
| `id` | The binder item this file is the text of |
| `title`, `synopsis`, `status`, `label`, `color` | As in the inspector |
| `goal`, `conflict`, `outcome` | The scene's three beats |

### The text

Blocks are separated by one blank line. The file ends with a line break.

| In the editor | In the file |
| --- | --- |
| Paragraph | The text, on one line |
| Line break inside a paragraph or quote | `\` at the end of the line |
| Line break inside a heading or list item | `<br>` |
| Empty paragraph | `<br>` alone on a line |
| Heading 1 to 6 | `# ` to `###### ` |
| Quote | `> ` at the start of each line |
| List | `- item`; a numbered list is `1. item`, `2. item` |
| A second list straight after a list of the same kind | `+ item`, or `1) item`, so the two stay two |
| Code | Between lines of three or more backticks; more than any run of backticks inside |
| Scene break | `* * *` |
| Bold, italic, strikethrough | `**bold**`, `*italic*`, `~~struck~~` |
| Underline | `<u>underlined</u>` |
| Link | `[text](address)`; `[text](<address with spaces or (brackets)>)` |
| Highlight, yellow | `{==text==}` |
| Highlight, another color | `{==text==}{.green}` — `green`, `blue`, or `pink` |
| Note on text | `{==text==}{>>the note<<}` |
| Note with a kind | `{==text==}{>>Question: the note<<}` — `Question`, `Revise`, or `Check` |
| Resolved note | `{==text==}{>>(resolved) Revise: the note<<}` |
| Note and highlight together | `{==text==}{.yellow}{>>the note<<}` |

Notes and highlights are CriticMarkup, an existing plain-text convention. The color and the words in front of the note are this program's additions to it.

**What the writer typed is never read as formatting.** A character that means something to Markdown is written with a backslash in front: `\*`, `\_`, `` \` ``, `\[`, `\]`, `\<`, `\{`, `\}`, `\~`, `\\`. Some are escaped only where they would matter: `#`, `>`, `-`, `+`, and `=` at the start of a line; the `.` of `1.` at the start of a line; `=` next to another `=`; `&` in front of something that looks like `&amp;`; `!` at the end of a stretch of text; every `#` in a heading. Spaces and tabs at the start or end of a line are written `&#32;` and `&#9;`, because editors remove them and Markdown gives them meaning.

**Where `**` would be misread, tags are used.** Other Markdown readers decide whether `**` opens or closes bold from the characters beside it. Where they would decide wrongly, the file says `<strong>`, `<em>`, or `<s>` instead. This happens when formatting ends exactly where other formatting begins, and when it presses punctuation against a letter, as in `word<strong>“quoted”</strong>`. In ordinary prose it is rare.

### What a file cannot hold

The file holds the document exactly, with these exceptions. They are removed when a scene is written, so the file and the document agree.

- Bold, italic, and strikethrough do not cover the spaces at the start or end of the formatted stretch. `**word **` becomes `**word** `.
- A line break at the very end of a paragraph, heading, quote, or list item is dropped. It shows nothing.
- A code block is plain text. Formatting, links, notes, and highlights inside one are dropped.
- A note that crosses formatting or a highlight is written once for each piece: `{==harbor ==}{>>check<<}{==was==}{.yellow}{>>check<<}`. The program reads it back as one note.
- The ids of paragraphs and runs are not stored. Nothing uses them.
- The time of the last save is not stored. Git has it.

### Reading files changed by hand

The reader accepts more than the writer produces, so that a file edited in another program still opens:

- A paragraph wrapped over several lines is one paragraph.
- `***`, `---`, and `___` are scene breaks. `*` starts a list item as `-` does. An indented line under a list item carries it on.
- `<b>`, `<i>`, and `<del>` are bold, italic, and strikethrough.
- `&amp;`, `&lt;`, `&gt;`, `&quot;`, `&nbsp;`, and numbered references are read as their characters.
- Anything else it does not recognize stays in the text as it was typed: a table, a footnote, an image, a tag it does not know.

It does not read `_italic_` or `__bold__`; use `*`. It does not read lists inside lists or quotes inside quotes; they come in as text.

### What has been checked

- Every kind of block and every piece of formatting is written and read back in the tests, with their awkward cases.
- Randomly built documents, made of the characters most likely to break Markdown, are written, read back, and compared. 2,000 run with every test run. More than a million were run while this was written; they found one fault, in code blocks, which was fixed.
- The files are read with an independent Markdown reader (`pulldown-cmark`), to see what another program makes of them:
  - Plain text, 20,000 paragraphs of awkward characters: the same text every time, and nothing read as formatting.
  - Formatted prose, 20,000 paragraphs: the same words with the same bold, italic, and links every time.
  - Formatting pressed against awkward characters, 20,000 paragraphs: no letter or digit is ever lost. About one paragraph in three hundred shows different bold or italic.

  This reader is where most of the escaping rules above come from. It found that a single `~word~` is strikethrough to some readers, that `!` before a link makes an image, that a paragraph cannot end with a line break, and where `**` is misread.
- A project on disk is converted scene by scene and compared. To check one of your own, which only reads it:

  ```bash
  cargo run -p writing-studio-format --example check_project -- path/to/Novel.writingstudio
  ```

  Add `--write some/folder` to save the Markdown files and look at them.

### File names

`the-harbor-3f9a.md`: the title in lowercase with everything but letters and digits turned into `-`, at most 40 characters, then the first four characters of the id. A file gets its name when it is made. Changing the title, or moving the scene, does not rename the file; the title in the header is the title.

## Other Markdown files — specified

Characters, places, cultures, lore, timeline events, and research notes are files of the same kind: a header, then the entry's text as Markdown, written and read by the same code as a scene.

| Folder | Header keys | Text |
| --- | --- | --- |
| `characters/` | `id`, `name`, `role`, `motivations` (a list) | Summary |
| `places/` | `id`, `name`, `culture` (an id) | Summary |
| `cultures/` | `id`, `name` | Summary |
| `lore/` | `id`, `title`, `place`, `culture` (ids) | Body |
| `timeline/` | `id`, `title`, `when`, `place` (an id) | Summary |
| `research/` | `id`, `title`, `files` (a list of names under `attachments/`) | Notes |

## JSON files — specified

JSON is for structure that is not read as prose. Every file is written with two-space indentation, one item to a line where items are short, and keys in a fixed order, so that a change to one item changes one line.

- **`project.json`** — `format`, `id`, `title`, and the shared settings: word targets, the compile book, spelling language and the project's accepted words, the compile title page.
- **`binder.json`** — a list of items. Each has `id`, `kind`, `parent`, `order`, and either `file` (an item with text, whose title is in that file) or `title` (a folder, part, or book with no text of its own). A trashed item also has `deleted` and `restore_parent`.
- **`plot.json`** — plotlines, each with its points; a point has `title`, `summary`, `order`, and an optional `scene`.
- **`links.json`** — bible entries that point at scenes, and relationships between characters.

**Order.** `order` is a short text key, not a number. A new item between two others gets a key that sorts between theirs, so moving one item changes one line and does not renumber its neighbors.

**Ids.** An item with text has one id, the one in its file. Format 1 gives the binder item and its text separate ids; the conversion keeps the binder item's.

## Questions still open

- Whether `README.md` in the project folder is written once or kept up to date.
- Whether a scene's snapshots should be offered to another machine. They are local in this draft.
- Lists inside lists. The editor cannot make them today, so the format does not hold them.
