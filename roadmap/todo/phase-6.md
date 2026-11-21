# Phase 6 — Publishing (local)

In progress: stages 1 to 3 are done.

Writing Studio compiles one book to Markdown or DOCX today, with a title page and nothing else to choose. This phase turns compile into publishing: the writer says what the book should look like once, as a profile, and gets a manuscript to send out, an ebook, a print-ready PDF, or a web page, from the same text. All of it happens on the writer's computer.

## Where compile is now

- `crates/export` reads the binder and the documents, never the editor (`compile::collect_sections`): each book, part, chapter, and scene becomes a section with a heading level, folders and notes are skipped, and the trash is left out. So the first item of the old plan, export independent of the editor, is already true. This phase keeps it that way.
- **Project → Markdown…** and **DOCX…** compile the current book. The only choice is `compile_title_page`, a project setting. Open notes become footnotes in Markdown and bracketed text in DOCX; resolved notes and highlights are left out.
- Every part, chapter, and scene is compiled, with its title as a heading. There is no way to leave a scene out, to number chapters, or to put a break between scenes instead of a heading.

## The work

In this order: profiles first, because every format after them reads one. Then what goes into the book, then the new formats, from the simplest to the hardest.

### 1. Compile profiles

**Done.** `CompileProfile` and `PageSetup` (`crates/project/src/compile.rs`) are kept in `project.json` as `compile_profiles`; the profile last used for each book is local (`compile_choices`). The built-in profiles are Manuscript and Plain text for now: Paperback, Ebook, and Web page come with the stages that make their formats. Word takes its font, size, line spacing, page size, margins, indent, and chapter page breaks from the profile (`crates/export/src/docx.rs`). **Project → Compile…** (`studio/compile.rs`) replaces the Markdown and DOCX items.

- A profile is a named set of choices: the format, what goes in front of and behind the text (stage 3), how chapters and scenes are set (stage 2), and the page (font, size, margins, line spacing) for the formats that have pages.
- Profiles are part of the project, in `project.json` (a shared setting), so every computer that syncs the project compiles the same book. A new project has the built-in ones; the writer can copy one, rename it, change it, and remove it. A built-in profile can be put back as it was.
- **Built-in profiles:** *Manuscript* (DOCX, standard manuscript format: a 12 pt serif or Courier, double spaced, 1 inch margins, the writer's name and word count on the first page), *Paperback* (PDF, 6 × 9 in, a serif, justified), *Ebook* (EPUB), *Web page* (HTML), and *Plain text* (Markdown).
- **Project → Compile…** opens one dialog: choose the book (the current one is chosen), the profile, and Compile. It replaces the separate Markdown and DOCX items; **Markdown folder…** and **Import** stay where they are. The last profile used is remembered for each book.
- `compile_title_page` becomes part of each profile. The old setting is no longer read: a project opens with the built-in profiles, which have a title page.

### 2. What is compiled, and how it is set

**Done.** `BinderNode::left_out` (`"left_out": true` in `binder.json`, a column in the cache, merged by sync like any field) and `BinderTree::is_left_out`, which also counts what an item is in. The profile has `chapters` and `parts` (`HeadingStyle`), the words before their numbers, and `scenes` (`SceneSeparator`). `collect_sections` numbers the compiled chapters and parts across the book, sets scenes apart as the profile says, and skips what is left out. The dashboard and the right-click menu have **Include in compile**; a left-out row is dimmed in the library and the scene list. The outliner dims left-out rows too. Number words follow the profile's `BookLanguage` (English, Portuguese, Spanish, French).

- **Include in compile**, on each book, part, chapter, and scene: on by default, a checkbox in the dashboard and in the right-click menu. A part or chapter left out leaves out what it holds. The scene list and the outliner show a left-out item dimmed.
- **Chapter headings**, per profile: the chapter's title, a number ("Chapter 1", or "Chapter One"), or both ("Chapter 1: The Harbor"). Parts the same way.
- **Between scenes**, per profile: the scene's title as a heading, a scene break (a centered `* * *`, or a blank line), or nothing. The default is a break: most books do not print scene titles.
- **Compile order** is the binder's order, in reading order. A compile lists the order it used in its summary (stage 7), so the writer sees what went in.

### 3. Front and back matter

**Done.** `Matter` (`crates/project/src/compile.rs`) is part of each profile: the copyright line, dedication, contents, about the author, and also by, each on or off, with its text kept while it is off. The author (`author`) and each book's subtitle (`book_subtitles`) are shared settings. `collect_sections` puts the matter around the text as `MatterPage` sections, leaves a page with no text out, and builds the contents from the compiled parts and chapters. Word starts each page of matter on a new page; Markdown sets them one after another. The subtitle is per book rather than per project, so each book of a series has its own. The matter headings come from the book language and can each be changed in the profile. A Word table of contents lists the headings without page numbers.

- **Front matter**, per profile, each one on or off: a title page (title, subtitle, author), a copyright page (one line of text the writer edits, with the year filled in), a dedication, and a table of contents (from the parts and chapters that are compiled).
- **Back matter:** "About the author" and "Also by", as text the writer edits.
- The author's name and the subtitle are new fields of the project, shared like the title. The texts above are kept with the profile, so a manuscript and an ebook can carry different ones.
- Matter is part of the compile only: it is not a scene and does not count toward the book's words or targets.

### 4. HTML

- One self-contained `.html` file: the book's text with its formatting, the front and back matter, and the page settings of Phase 9 as its stylesheet. No scripts and no outside files, so it opens anywhere and can be put on a website as it is.
- Notes as footnotes at the end, linked both ways, as in the Markdown compile. Highlights left out.
- The text is written by the document's own HTML writer (`Document::to_html`), so a scene compiles as it was stored, escaped the same way.

### 5. EPUB

- An EPUB 3 file: one XHTML file per chapter, the navigation document from the table of contents, and the package metadata (title, author, language, a stable identifier kept in the project, the date).
- An optional cover image chosen from the computer and kept in the project folder.
- Built with the `zip` crate, which `docx-rs` already brings in: no new crate in the build, only a new direct use of one already there.
- Checked by tests that read the file back: the entries in the right order (`mimetype` first and stored), every file in the manifest present, and every XHTML file well formed.

### 6. PDF

- A paged PDF for print and for reading on screen: the page size, margins, font, and line spacing of the profile, page numbers, chapters starting on a new page (on a right-hand page for print), and justified text with hyphenation.
- **This needs a decision before the stage starts.** A good PDF needs real typesetting: line breaking, hyphenation, fonts, page layout. The choices are:
  - **Typst as a library** (Apache-2.0): a full typesetting engine in Rust, offline, with good line breaking and hyphenation. It is a large dependency, and the program has to carry at least one font family with a license that allows it (for example Libertinus or Source Serif, both OFL).
  - **A small PDF crate** such as `printpdf`: much smaller, but the program would do its own line breaking and page layout, which is a lot of code to own and will look worse.
  - **Printing the HTML compile through the window's own engine** (WebView2 on Windows): no new dependency, but each platform's web view does it differently or not at all, and page control (right-hand chapter starts, running heads) is limited.
- The recommendation is Typst: it is the only one that gives a book-quality PDF without this project writing a typesetter. Its weight is a cost to weigh with the maintainer before it is added (see `.claude/rules/architecture.md`).

### 7. Check the manuscript before compiling

- Before a compile, a short list of what may not be meant to go out: scenes with no words, open notes, tracked changes not yet accepted or rejected, chapters with no title when the profile prints titles, scenes left out of the compile, and a count of unknown words when spellcheck is on.
- Each line names the scene, and a click opens it. **Compile anyway** compiles as it is; nothing in the list blocks a compile.
- After a compile, the status line says what was made: the file, the profile, how many chapters and words, and what the check found.

### 8. Preview

- **Preview** in the Compile dialog shows the book as the profile sets it, in the window, from the HTML compile with the profile's page settings, before any file is written. For PDF it shows the text and matter, not the exact pages.

## How changes are checked

The tests in this repository are the only testing for now (see "How the work is done now" in [../README.md](../README.md)).

- Profiles round-trip through `project.json`, the built-in ones are there in a new project, and a project with the old `compile_title_page` opens with it in its profiles.
- `collect_sections` tests for include in compile, chapter numbering, scene breaks, and front and back matter.
- Each format's output is read back in its test: HTML is well formed and escaped, EPUB as above, DOCX keeps its current tests, PDF has the right page count and size and the text in order.
- The check before compile is a state test with a scene of each kind it reports.
- How the dialog and the preview look is not tested, as in Phases 8 and 9.

## Not in this phase

- Print-on-demand or store uploads, or anything that needs the network.
- Exact PDF pages in the preview.
- Fonts other than the ones the program carries and the ones on the computer.
- Packaged builds (Phase 7).
