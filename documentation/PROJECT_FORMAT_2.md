# Writing Studio project format 2 (draft)

Status: **draft**. The program still reads and writes format 1 ([PROJECT_FORMAT.md](PROJECT_FORMAT.md)). This is the format Phase 5 moves to. See [ROADMAP.md](../ROADMAP.md).

Each section says how far it has got:

- **Built and tested**: scene files, and their Markdown export and import. The code is in `crates/format`.
- **Specified**: everything else. It is written down here so it can be argued with, and is built in the next stage.

## What the format is for

1. **The book can be read without this program.** A scene is an HTML file. Any browser opens it as a page, and the words are there, in order, with their formatting. For another writing tool, the program also writes each scene as Markdown.
2. **The book can be kept with git.** Text files can be compared and merged. One database file cannot.
3. **Nothing is lost.** A scene file holds the scene exactly. What the program writes, it reads back as the same document.

### Why HTML is stored, and Markdown is exported

Markdown was the first choice and was built and tested. It reads well as plain text, but it cannot hold a scene exactly: it has no way to say a few things the editor produces, and other Markdown readers disagree about what `**` means next to punctuation. HTML can say all of it, with one spelling for each thing, and needs nothing escaped but `&`, `<`, and `>`.

So the stored file is HTML, and Markdown is an export: it has to be good to read and good for other programs, and it does not have to be exact. The Markdown reader stays, for bringing text in.

| | Stored: HTML | Export: Markdown |
| --- | --- | --- |
| Holds the scene exactly | Yes | No; see the list below |
| Opens without this program | In any browser | In any text editor or Markdown tool |
| Read by the program | Always | To bring text in |

## The folder

```text
MyNovel.writingstudio/
  FORMAT                  writing-studio-project 2
  README.md               what these files are, for someone without this program
  scene.css               how a scene file looks in a browser
  project.json            the project's id and title, and settings every machine shares
  binder.json             every binder item: its kind, parent, order, and file
  manuscript/             one HTML file for each scene, and each chapter that holds text
    the-harbor-3f9a.html
  characters/             one HTML file for each character
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

The HTML and JSON files are the project. `.local/` can be deleted; the program rebuilds what it can from the files, and what it cannot rebuild (snapshots, backups, writing sessions) was only ever on this machine.

## Scene files — built and tested

```html
<!doctype html>
<html>
<head>
<meta charset="utf-8">
<title>The Harbor</title>
<meta name="id" content="3f9a2c1e-0b7d-4e55-9a10-6c2f8d4b1a77">
<meta name="synopsis" content="Mara is late.&#10;The boat is leaving.">
<meta name="status" content="draft">
<meta name="goal" content="Reach the boat before the tide">
<link rel="stylesheet" href="../scene.css">
</head>
<body>
<p>Mara ran for the <strong>north</strong> quay. <em>Not again</em>, she thought.</p>
<p>The <span class="note" data-note="which one?" data-kind="question">harbor</span> was quiet.</p>
<p>“Wait,” said the pilot.<br>Nobody waited.</p>
</body>
</html>
```

### The head

- `<title>` is the scene's title.
- Everything else is a `<meta name="…" content="…">` line, one to a line. A line break inside a value is written `&#10;`.
- A name with nothing to say is left out. Missing and empty mean the same.
- Names the program does not know are kept, after the ones it does, and written back. The same name twice is a list.
- The stylesheet line is the same in every file. It only affects how a browser shows the scene.
- A file with no head, only the text, is read as a scene with no title. The program gives it an id when it takes the file in.

| Name | Meaning |
| --- | --- |
| `id` | The binder item this file is the text of |
| `synopsis`, `status`, `label`, `color` | As in the inspector |
| `goal`, `conflict`, `outcome` | The scene's three beats |

### The text

Every block is on its own line, so a change to one paragraph changes one line of the file. A list has a line for each item. A code block has a line for each of its lines.

| In the editor | In the file |
| --- | --- |
| Paragraph | `<p>…</p>`; an empty one is `<p></p>` |
| Line break inside a block | `<br>` |
| Heading 1 to 6 | `<h1>` to `<h6>` |
| Quote | `<blockquote>…</blockquote>` |
| List | `<ul>` or `<ol>` on a line, then one `<li>…</li>` line for each item |
| Code | `<pre>`, a line break, the code as typed, `</pre>` |
| Scene break | `<hr>` |
| Bold, italic, strikethrough, underline | `<strong>`, `<em>`, `<s>`, `<u>` |
| Link | `<a href="address">` |
| Highlight, yellow | `<mark>` |
| Highlight, another color | `<mark class="green">` — `green`, `blue`, or `pink` |
| Note on text | `<span class="note" data-note="the note">` |
| Note with a kind | `data-kind="question"` — `question`, `revise`, or `check` |
| Resolved note | `data-resolved="true"` |

- Formatting nests in one fixed order, outermost first: note, highlight, link, bold, italic, strikethrough, underline. A note over text that is partly bold or partly highlighted is written once, around all of it.
- In the text, only `&`, `<`, and `>` are written differently: `&amp;`, `&lt;`, `&gt;`. A non-breaking space is written `&nbsp;` so that it can be seen.
- Spaces are kept as typed, including several in a row and at the ends of a paragraph. The stylesheet tells a browser to show them.
- There is no line break inside a block other than in code. A browser's view and the file agree.

### What a scene file does not hold

- The ids of paragraphs and runs. Nothing uses them.
- The time of the last save. Git has it.
- A link whose address is not `http`, `https`, `mailto`, or a plain path: the link is taken off and its text is kept. A file that a browser opens must not carry a `javascript:` address.

Everything else is stored exactly, including the things Markdown cannot say.

### Reading files changed by hand

The reader accepts more than the writer produces:

- A file that was tidied or indented: a line break and the spaces around it, inside a block, read as one space, and as nothing at the edge of a block.
- `<b>`, `<i>`, `<del>`, and `<strike>`; `<div>` as a paragraph; text outside any block as a paragraph; tags and attributes in capitals; an attribute with no value.
- A tag it does not know is passed over and its text is kept.
- `&copy;` and other named characters it does not know stay as typed.

Nothing in a scene file is ever run. `<script>` and `<style>` are skipped with everything inside them, and attributes such as `onclick` are ignored.

It does not read lists inside lists, or blocks inside a quote; their text comes in as paragraphs.

### File names

`the-harbor-3f9a.html`: the title in lowercase with everything but letters and digits turned into `-`, at most 40 characters, then the first four characters of the id. A file gets its name when it is made. Changing the title, or moving the scene, does not rename the file; the `<title>` is the title.

### What has been checked

- Every kind of block and every piece of formatting is written and read back, with their awkward cases, including the ones Markdown drops.
- Randomly built documents, made of the characters most likely to break HTML or Markdown, are written, read back, and compared. 2,000 run with every test run; 300,000 were run once when the HTML reader was written.
- A project on disk is converted scene by scene and compared. To check one of your own, which only reads it:

  ```bash
  cargo run -p writing-studio-format --example check_project -- path/to/Novel.writingstudio
  ```

  Add `--write some/folder` to save each scene file, with its Markdown export beside it, and look at them.

## Markdown export and import — built and tested

The same scene, written for another program:

```markdown
---
id: 3f9a2c1e-0b7d-4e55-9a10-6c2f8d4b1a77
title: The Harbor
synopsis: |-
  Mara is late.
  The boat is leaving.
status: draft
goal: Reach the boat before the tide
---

Mara ran for the **north** quay. *Not again*, she thought.

The {==harbor==}{>>Question: which one?<<} was quiet.

“Wait,” said the pilot.\
Nobody waited.
```

- The header is the `---` front matter that other writing tools read: `key: value` lines, quoted only where a YAML reader could misread the value, with text of several lines under `|-`.
- Notes and highlights are CriticMarkup: `{==text==}` is a highlight, `{==text==}{.green}` a colored one, `{==text==}{>>the note<<}` a note, with `Question: `, `Revise: `, `Check: `, and `(resolved) ` in front of the note where they apply.
- Underline is `<u>`. A scene break is `* * *`. An empty paragraph is `<br>` on a line.
- What the writer typed is never read as formatting: characters that mean something to Markdown are written with a backslash.
- Where another Markdown reader would misread `**`, `*`, or `~~`, the file uses `<strong>`, `<em>`, or `<s>`.

**What the export leaves out**, because Markdown cannot say it:

- Bold, italic, and strikethrough on the spaces at the start or end of the formatted stretch.
- A line break at the very end of a paragraph, heading, quote, or list item.
- Formatting, links, notes, and highlights inside a code block.
- A list with no items.
- A note that crosses formatting or a highlight is written once for each piece.

**Import.** The reader takes the export back, and reads Markdown written elsewhere: wrapped paragraphs, `***` or `---` as a scene break, `*` list items, `<b>` and `<i>`. It does not read `_italic_`, lists inside lists, or quotes inside quotes; those come in as text.

**Checked against another reader.** The export is read with an independent Markdown reader (`pulldown-cmark`):

- Plain text, 20,000 paragraphs of awkward characters: the same text every time, and nothing read as formatting.
- Formatted prose, 20,000 paragraphs: the same words with the same bold, italic, and links every time.
- Formatting pressed against awkward characters, 20,000 paragraphs: no letter or digit is ever lost. About one paragraph in three hundred shows different bold or italic.

## Other HTML files — specified

Characters, places, cultures, lore, timeline events, and research notes are files of the same kind: a head, then the entry's text, written and read by the same code as a scene. The name or title is the `<title>`.

| Folder | Names in the head | Text |
| --- | --- | --- |
| `characters/` | `id`, `role`, `motivation` (once for each) | Summary |
| `places/` | `id`, `culture` (an id) | Summary |
| `cultures/` | `id` | Summary |
| `lore/` | `id`, `place`, `culture` (ids) | Body |
| `timeline/` | `id`, `when`, `place` (an id) | Summary |
| `research/` | `id`, `file` (once for each name under `attachments/`) | Notes |

## JSON files — specified

JSON is for structure that is not read as prose. Every file is written with two-space indentation, one item to a line where items are short, and keys in a fixed order, so that a change to one item changes one line.

- **`project.json`** — `format`, `id`, `title`, and the shared settings: word targets, the compile book, spelling language and the project's accepted words, the compile title page.
- **`binder.json`** — a list of items. Each has `id`, `kind`, `parent`, `order`, and either `file` (an item with text, whose title is in that file) or `title` (a folder, part, or book with no text of its own). A trashed item also has `deleted` and `restore_parent`.
- **`plot.json`** — plotlines, each with its points; a point has `title`, `summary`, `order`, and an optional `scene`.
- **`links.json`** — bible entries that point at scenes, and relationships between characters.

**Order.** `order` is a short text key, not a number. A new item between two others gets a key that sorts between theirs, so moving one item changes one line and does not renumber its neighbors.

**Ids.** An item with text has one id, the one in its file. Format 1 gives the binder item and its text separate ids; the conversion keeps the binder item's.

## Questions still open

- Whether `README.md` and `scene.css` in the project folder are written once or kept up to date.
- Whether the whole book is also offered as one Markdown file, beside one file for each scene.
- Whether a scene's snapshots should be offered to another machine. They are local in this draft.
- Lists inside lists. The editor cannot make them today, so the format does not hold them.
