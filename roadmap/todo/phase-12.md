# Phase 12 — A Markdown editor

In progress: stages 1 to 5 are done.

The page is a rich-text editor today: a `contenteditable` element that the program's own scripts (`crates/ui/assets/review.js`, `spell.js`, `place.js`) and `crates/ui/src/studio/editor.rs` keep in step with the document model. This phase makes the page a Markdown editor: the writer types and sees Markdown, shown as the best Markdown editors show it, while everything the page does now keeps working.

## Where things are now

- **The document model is the text.** A scene is a list of blocks (paragraph, heading, quote, list, code, scene break) of runs with marks: bold, italic, underline, strikethrough, a link, a note (with its kind and whether it is resolved), and a highlight in one of four colors (`crates/document`). Saving, search, compile, sync, and diagnostics all read it; the rule in `.claude/rules/architecture.md` is that they keep doing so.
- **Scenes are HTML files** (`documentation/PROJECT_FORMAT.md`), because HTML holds every mark exactly.
- **Markdown already round-trips** in `crates/format`: the export and import write and read headings, lists, quotes, code, `* * *`, `<u>` for underline, `<br>` for an empty paragraph, and notes and highlights as CriticMarkup (`{==text==}{>>note<<}`, `{==text==}{.green}`), with tests that every document survives the trip.
- **The page does a lot besides text.** Spelling marks and their suggestions, notes and highlights with the review list, tracked changes against a baseline, beats and diagnostics, keeping one's place (fixed scrolling, a line marker, fading all but the sentence or paragraph), focus mode, split at the caret, word counts, and "Named here" all reach into the page's DOM.

## What Markdown editors do

From their documentation (sources at the end):

- **Obsidian** has two ways to edit. *Live Preview*, the default, shows formatted text and hides most Markdown syntax, which comes back where the cursor goes into the formatted text. *Source mode* shows every mark as typed. It is built on CodeMirror 6.
- **Typora** is "seamless live preview": spans are rendered as they are typed and open into their Markdown when the cursor enters them; `#` and Return makes a heading, `-` and a space a list, `>` a quote. ⌘1 to ⌘6 set heading levels. It can pair brackets, quotes, and Markdown marks as they are typed. Footnotes are `[^1]` and show on hover; `==text==` highlights (an option). It has a Focus mode (all but the current line blurred), a Typewriter mode (the line in the middle), word, character, and reading-time counts, and an outline.
- **iA Writer** keeps the syntax visible and quiet. Formatting comes from typing, the toolbar, the Format menu, or a command palette. Footnotes are `[^1]` (inline ones on Apple devices), highlights `==text==`, comments `<!-- -->` everywhere. Focus mode dims all but the sentence or the paragraph, with typewriter scrolling apart. Smart dashes are a setting.
- **Ulysses** uses its own markup ("Markdown XL") and never hides it ("WYSIWYM"); quiet themes make it recede. Marks come from the toolbar, a markup bar, or ⌘B, ⌘I, ⌘K; lists continue themselves ("Smart Lists"); a pasted URL becomes a link. Footnotes open a field from `(fn)`, annotations wrap text in `{ }` with a note beside it, comments are `++inline++` or a `%%` paragraph, and comments and annotations do not count as words and are left out of export.
- **CodeMirror 6**, the component Obsidian uses, draws plain text and lets *decorations* style it: mark decorations style a range, replace decorations hide a range (or put a widget there), widget and line decorations add elements or line styles. A live preview is replace decorations over the markup, rebuilt as the selection moves so the active element shows its marks; atomic ranges make the caret step over what is hidden. Its content is a `contenteditable` that it owns: other code must not change its DOM, so spelling, notes, and the rest become decorations too.

## What this phase takes

- **Live preview as the page, and Source mode beside it**, as Obsidian and Typora do: Markdown marks hidden except in the element the caret is in, headings set as headings, quotes and lists as they read, `* * *` as a scene break; one command shows every mark.
- **Typing the marks or using shortcuts**: `**`, `*`, `#`, `-`, `>`, `[text](url)` as typed, and Ctrl+B, Ctrl+I, Ctrl+K, and a key for heading levels. Lists continue on Enter and end on an empty item (Smart Lists). Optional pairing of `*`, `_`, `"`, and brackets.
- **Highlights as `==text==`** and **footnotes as `[^1]`**, as Typora, iA Writer, and Obsidian read them, with a footnote's text shown at its mark.
- **Notes stay CriticMarkup** (`{==text==}{>>note<<}`), as the export already writes them, shown as the highlighted text with the note beside it, as Ulysses shows annotations; like Ulysses' comments, notes do not count as words and are not compiled.
- **Paste**: rich text from a browser or a word processor comes in as Markdown (through `crates/format`), and a URL pasted over a selection makes a link.
- **Smart typography as a setting**: curly quotes and dashes as typed, as iA Writer offers.
- **What the page does now**, all of it: spelling, the review list, tracked changes, keeping one's place, focus mode, split at the caret, word counts, and "Named here".

## What this phase does not take

- **Ulysses' own markup.** Markdown, CommonMark with the common additions (highlights, footnotes) and CriticMarkup for notes, so a scene opens the same in Obsidian, Typora, or iA Writer.
- **Comments as `<!-- -->` or `%%`.** Notes are CriticMarkup already; a second way to write them would split the review list.
- **A Markdown math or diagram engine.** Fiction rarely needs one.

## Decisions

Decided by the maintainer: the component is CodeMirror 6, and scenes stay HTML files for now (stage 8 is decided at the end).

- **The editor component.** CodeMirror 6 is MIT-licensed, works offline, and is what Obsidian proves out; it is JavaScript, so it is vendored as one pinned, prebuilt file in `crates/ui/assets` (like the scripts there), not loaded from the network, and its license goes into the `licenses` folder beside `THIRD-PARTY-NOTICES.html`, which `cargo-about` does not cover. A plain `textarea` with the program's own highlighting is the smaller choice, but gives no live preview and makes spelling marks and notes hard to place. **Decided: CodeMirror 6**, approved by the maintainer as the dependency `.claude/rules/architecture.md` asks.
- **What is stored.** Two ways:
  - *Keep the HTML files* and the document model as they are: the page turns the scene into Markdown when it opens it and back into blocks when it saves (both already in `crates/format`). No format change.
  - *Store scenes as `.md` files* (format 3): the files are what the writer sees, and other Markdown editors open the project directly. Following "No compatibility with earlier versions" in [../README.md](../README.md), format 2 projects would be refused.
  **Decided: the HTML files stay for now.** `.md` storage is the phase's last, separate stage, decided once the editor has proved itself.
- **Underline and colored highlights.** Markdown has neither. Underline stays `<u>`, as the export writes it; highlights keep their color as `{==text==}{.green}`. The toolbar keeps offering both.
- **Shortcut keys.** Ctrl+1 to Ctrl+8 change views today, so heading levels take Ctrl+Alt+1 to Ctrl+Alt+6, or the paragraph style menu.

## The work

In this order: the component and the round trip first, then what the writer sees, then everything the page did before, and only then the old editor goes.

### 1. The component, bundled

- CodeMirror 6 with its Markdown language, history, and search, built once into one file at a pinned version and vendored in `crates/ui/assets`, loaded by the page like `review.js`. Nothing from the network.
- Its license in the packages' `licenses` folder; `about.hbs` or the Package workflow notes it.
- Rust and the page talk through small messages (`document::eval`, as now): the page sends the text when it changes, the program sends a scene's text when one opens.

**Done.** `crates/ui/codemirror` builds the bundle: `package.json` pins CodeMirror's packages and esbuild exactly, `package-lock.json` pins the rest, and `npm ci` then `npm run build` there writes `crates/ui/assets/codemirror.js` (330 KB, minified) and `LICENSES.txt`, from what esbuild read. The bundle is the Markdown language alone, so an HTML tag such as `<u>` is one node and the HTML, CSS, and JavaScript parsers stay out (520 KB with them). All its packages are MIT. The page loads it once when the window opens (`App`, `CODEMIRROR_JS`), as `window.wsEditor`: `mount(parent, text, onChange)`, `getText`, `setText` (a new scene, with a fresh history and no change reported), and `unmount`. The Package workflow copies `LICENSES.txt` into each package's `licenses` folder as `editor-LICENSES.txt`, and the notices and About name it. Checked in a browser: typing reports the text, `setText` replaces it without a change, and `unmount` leaves nothing. Nothing mounts it on the page yet: that is stage 2.

### 2. Markdown on the page, and back

- Opening a scene puts its Markdown (from `crates/format`'s writer) on the page; a change goes back through the reader into the document model, so saving, search, compile, sync, and diagnostics read what they read today.
- Every document the old editor could make survives Markdown and back (the format crate's round-trip tests, extended to every mark and to what the page can type). What Markdown cannot hold is written as the export writes it.
- The editor session's undo (`crates/editor`) keeps the structure steps (merge, split); typing is CodeMirror's history.

**Done.** Settings has **Markdown page (in progress)**, off by default and not saved, so the rich-text page stays the page until stage 7. On, the page is `MarkdownPage` (`studio/editor.rs`): one CodeMirror for each scene, mounted with `StudioState::editor_markdown` (the Markdown export, with nothing for an empty scene), and every change read back by `apply_editor_markdown` through `markdown_to_blocks` and `EditorSession::apply_blocks`, which keeps the blocks' ids where they stand (`Document::replace_blocks`, as `replace_html` does), so tracked changes still line up. Leaving the page saves, as before. A change from outside the page (undo, the review panel, a split) moves the epoch, and `wsEditor.setText` puts in only what differs, keeping the caret, and starts CodeMirror's history again: so Ctrl+Z takes back typing from CodeMirror's history while it has any, and otherwise goes on to the program's undo, which takes back the structure step. A key CodeMirror acted on does not also reach the program's shortcuts. Opening a scene changes nothing; the first change puts it in canonical form, as the Markdown export always has. Tests: `text_half_typed_on_the_page_reads_back_stably` (`crates/format`: marks left open, a list item or quote just started, read without losing a letter and stable once written back) and `the_markdown_page_reads_and_writes_the_scene` (`crates/ui`); the script was checked in a browser.

### 3. Live preview and Source mode

- Marks hidden except in the element the caret is in; headings, quotes, lists, links, emphasis, and `* * *` drawn as they read. A command and a setting switch to Source mode.
- The page's type settings (Phase 9: font, size, line length, spacing, indent) apply as they do now.

**Done.** `crates/ui/codemirror/syntax.js` is the language: CodeMirror's CommonMark with GitHub's additions (not its subscript, superscript, and emoji, which `crates/format` does not read), plus `<u>…</u>` as one node, `==text==` and `{==text==}` highlights with `{.color}` and `{>>note<<}` after them, `[^1]` footnote marks, and `[^1]: text` lines. It shares CodeMirror's Markdown language data, so its list and quote commands work on it. `preview.js` draws the page from the syntax tree: headings, emphasis, strikethrough, underline, links (the address as their title), inline code and code blocks, quotes with their rule, lists, the scene break centred, highlights in their color, notes with their kind's underline and a pin that shows the note, resolved notes faint, footnote marks raised with their text as a title, escapes and character references as the character, and an empty paragraph's `<br>` as nothing. Outside Source mode the marks are hidden except in the inline element the selection touches, or on the caret's line for a heading's `#`, a quote's `>`, and a list's marker; the caret steps over what is hidden. Source mode (**Show every Markdown mark** in Settings, or **Ctrl+E**) shows them all, quiet. It is `MarkdownOptions::source_mode` in the project's settings, with smart typography and pairing for stage 4; a project without them gets live preview. The font, size, line length, and indent come from the manuscript, and the space between paragraphs is the blank line's height. Checked in a browser: what each mark looks like away from the caret, in it, and in Source mode.

### 4. Writing in Markdown

- Ctrl+B, Ctrl+I, Ctrl+K, strikethrough, heading levels, quote, and list, from keys and from the format toolbar, which wraps or unwraps the marks.
- Lists continue on Enter; a quote too. Optional pairing of marks and brackets.
- Pasting rich text converts it to Markdown; a URL over a selection makes a link.
- Smart quotes and dashes, as a setting.

**Done.** `crates/ui/codemirror/writing.js`. Keys: Ctrl+B, Ctrl+I, Ctrl+U, Ctrl+Shift+X (strikethrough), Ctrl+K (the selection becomes a link's text with the caret where its address goes), and Ctrl+Alt+1 to 6 and Ctrl+Alt+0 for headings and body text; each wraps the selection without the spaces at its edges, or takes the mark off what the caret is in. The format toolbar is on the Markdown page too, and its buttons, the link and note fields, and the review panel run the same commands there: `run_format_command`, `run_editor_script`, `open_prompt`, and `keep_caret` (`studio/editor.rs`) call the editor when the page is Markdown, by the rich-text page's names (`MARKDOWN_PAGE_CALLS`), until stage 7 removes the other branch. Highlights, notes, and links are written as `crates/format` writes them: `{==text==}`, `{.green}`, `{>>Question: text<<}` with `\ < > { }` escaped, and `[text](address)` for a web or mail address only. The toolbar shows the marks at the caret from the syntax tree. Enter continues a list or quote and ends it on an empty item; Backspace takes a marker off. **Close brackets and marks** and **Smart quotes and dashes** in Settings (`MarkdownOptions`): curly quotes and apostrophes, `--` as an em dash (not on a line of dashes, which is a scene break), `...` as an ellipsis, and nothing in code. A web address pasted over plain text makes it a link; rich text pasted from a browser or a word processor goes to the program, which takes the fragment that was copied without its styles, scripts, and title, reads it as the rich-text page read a paste (`pasted_markdown`), and gives back Markdown. Tests: `rich_text_pasted_on_the_markdown_page_comes_in_as_markdown`; the commands, keys, lists, typography, pairing, and paste were checked in a browser.

### 5. Notes, highlights, and footnotes

- `==text==` and colored highlights, notes as CriticMarkup shown as highlighted text with the note in the review list and beside it, resolved notes dimmed. Notes do not count as words.
- Footnotes `[^1]` with their text at the end of the scene, shown at the mark, compiled as today's notes are.

**Done.** `crates/format` reads `==text==` as a yellow highlight (written back as `{==text==}`), `_` and `__` as `*` and `**` except inside a word, and footnotes: each `word[^1]` with a `[^1]: text` line becomes a note on the word before the mark, and the lines of the footnotes used go, so a footnote is in the review list, is not counted, and compiles as a note does. A mark with no plain word before it or no text, and anything in a code block, stay as written. Once the scene is drawn again (an undo, a review action, reopening), a footnote shows as the note it became. The page's parser reads only what `crates/format` keeps, so it never shows formatting the scene would lose: inline code, images, setext headings, indented code, and link definitions are text on the page too. Enter starts a new paragraph (a blank line), in a list or quote the next item or line; Shift+Enter is a line break inside the paragraph, the backslash `crates/format` writes. The review panel's notes and highlights and Navigate's blocks are found on the Markdown page as the rich-text page found them (`places.js`: pieces side by side in one block with the same note or color are one item). Tests: `double_equals_is_a_yellow_highlight`, `footnotes_are_read_as_notes_on_the_word_before_them`, `underscores_are_emphasis_except_inside_a_word`; the page was checked in a browser.

### 6. What the page did before

Each as CodeMirror decorations or panels, not changes to its DOM:

- Spelling marks and their suggestions (`spell.js`).
- Tracked changes against the baseline, and accepting or rejecting them.
- Keeping one's place: fixed scrolling, the line marker, fading all but the sentence or paragraph (`place.js`); focus mode.
- Split at the caret, merge, word counts (without Markdown marks or notes), "Named here", and the Navigate list of headings.

### 7. The rich-text editor goes

- The `contenteditable` page, its HTML handling in `editor.rs`, and the scripts or parts of them that only it used are removed, as the roadmap's rules ask.

### 8. Scenes as Markdown files (decided at the end)

- If the maintainer chooses it: scenes are stored as `.md` files with their header as front matter, `FORMAT` says 3, and format 2 projects are refused with a clear message. `documentation/PROJECT_FORMAT.md` is rewritten for it.

## How changes are checked

- The round trip, Markdown to blocks to Markdown, is tested in `crates/format` for every mark, list, note, highlight, and footnote, and for text typed in the editor's own shapes (marks left open, a list item emptied).
- Keys, smart lists, pasting, and smart typography are tested as text in and text out where they are written in Rust; what is written in the page's script is checked by the maintainer in the window.
- Spelling, tracked changes, notes, and keeping one's place keep their state tests; how they look on the page is the maintainer's to check, as in Phases 8, 9, and 11.

## Sources

- [Obsidian: Edit and read](https://obsidian.md/help/edit-and-read)
- [Typora: Markdown reference](https://support.typora.io/Markdown-Reference/) and [typora.io](https://typora.io/)
- [iA Writer: Markdown guide](https://ia.net/writer/support/basics/markdown-guide), [Syntax highlight](https://ia.net/writer/support/editor/syntax-highlight), [Focus mode](https://ia.net/writer/support/editor/focus-mode)
- [Ulysses: Markdown XL](https://help.ulysses.app/en_US/dive-into-editing/markdown-xl), [Comments, notes, and annotations](https://help.ulysses.app/en_US/dive-into-editing/comments-notes-annotations)
- [CodeMirror: System guide](https://codemirror.net/docs/guide/), [Decorations example](https://codemirror.net/examples/decoration/)
