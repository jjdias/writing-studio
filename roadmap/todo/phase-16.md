# Phase 16 — Tests for the page

In progress: stages 1 and 2 are done. Branch: `phase-16`.

The Markdown page is the program's one piece of JavaScript: about 1,900 lines in `crates/ui/codemirror` (`entry.js`, `syntax.js`, `preview.js`, `writing.js`, `places.js`) around the CodeMirror library, built into `crates/ui/assets/codemirror.js`. It is what the writer types into, and it has no tests: the Rust tests drive the window's state, not the page. This phase gives the page's own logic tests that run with `npm test` and in CI.

The JavaScript stays JavaScript. Moving it to Rust would mean leaving the web view for a native interface and an editor as capable as CodeMirror, which is a rewrite of the whole window; tests are what the page lacks.

## Where things are now

- `crates/ui/codemirror/package.json` has one script, `build` (esbuild, then `licenses.js`), and one dev dependency, esbuild. Nothing tests the modules.
- The modules are ES modules, but the package does not say so, so Node reads each one twice and warns. `licenses.js` is CommonJS (`require`).
- CI (`.github/workflows/ci.yml`) builds and tests the Rust workspace and checks the release build. It does not install Node, so it never builds the page or checks that the committed `codemirror.js` is the one its sources make.
- CodeMirror's editor state, syntax tree, and our parser load and run in Node without a window (checked with Node 26).

## Decisions

Made by the maintainer:

- **The JavaScript stays**, and gets tests.

Recommended in this plan (the builder's calls, for the maintainer to correct):

- **Node's own test runner** (`node --test`), with no test library: Node 20 and later have it, and `npm test` runs it.
- **No browser in the tests.** What is tested works on CodeMirror's editor state. Where a function takes an editor view, a test passes a small stand-in (`tests/view.js`): the state, `dispatch` that applies a transaction to it, and the visible range as the whole text. jsdom, which would give the page a document, is a new dependency with its own packages to fetch and vet, for the same tests; it is added only if a stage finds something that cannot be tested without it, and then with the maintainer's yes.
- **Small changes to the modules to make them testable**, nothing more: export a function a test needs (the input handler behind smart typography, `enter`, `lineBreak`, the completion source), and keep the one call to `document` in the preview (`decodeEntity`) where a test does not reach it. No module is reorganised.
- **CI builds the page and compares it** with the committed `codemirror.js`, so a change to the sources that was not built fails there, as a stale file otherwise ships.

## What this phase does not take

- **Tests that drive the window**: clicking, typing into the real page, the panels, the CSS. They need a WebDriver for WebView2 and WebKitGTK, and are a phase of their own.
- **The parts of `places.js` that measure the page**: fixed scrolling, the line marker, and hiding everything while typing read the page's layout, so only a test with a window can check them. What they decide without the layout (which sentence or paragraph is current) is tested.
- **Rendering the window's components in Rust tests** (Dioxus to HTML): also its own piece of work.

## The work

### 1. Tests run

- `"type": "module"` in `package.json`, and `licenses.js` renamed `licenses.cjs` (the build script follows), so Node reads each module once and does not warn.
- `npm test` runs `node --test tests/`. `tests/view.js` holds the stand-in view and a helper that makes an editor state with the page's extensions for some text.
- A first test that the scene's language parses a line, so the runner, the imports, and the stand-in are known to work before anything else is written.
- `crates/ui/codemirror/README.md` (one paragraph): how to build the page and run its tests.

**Done.** `package.json` has `"type": "module"` and a `test` script, `node --test tests/`; `licenses.js` is `licenses.cjs`, and the build runs it under that name. `tests/view.js` holds `pageState` (a state with the page's language and known notes for a text, `|` marking the caret or a selection), `standIn` (the stand-in view: the state, `dispatch`, the visible range, a `focus` that does nothing), `nodeNames`, and `withSelection`. The known notes are made by `noteMap` in `preview.js`, which `entry.js` now uses too, since `entry.js` attaches itself to `window` when it loads and so cannot be loaded in Node. `tests/setup.test.js` checks that a line with a note and a link parses and that the helpers work; renaming the parser's `WikiLink` node on purpose made it fail. The rebuilt bundle differs only by `noteMap`, its licenses are the same 21 packages, and the window's tests pass with it. `README.md` says how to build the page and run its tests.

### 2. The scene's language

`syntax.js` and the small readers built on it.

- The parser finds what the page shows: `**bold**`, `*italic*`, `<u>underline</u>`, `~~strike~~`, `==highlight==`, `{==text==}{.green}`, `{==text==}{>>note<<}`, `[[Title]]` and `[[Title|words]]`, `[^1]` footnotes, a scene break, headings, quotes, lists, code; and leaves alone what is not one (a lone `[[`, `{==` without its end, brackets inside code).
- `readNote` reads a note's kind (`Question: `, `Revise: `, `Check: `) and `(resolved)`; `footnoteTexts` pairs a footnote's mark with its text; `wikiParts` splits a link's title from its words; `linkTarget` and `noteBody` write what the commands insert.
- Each case is one table row (text in, what is found out), so a new case is one line.

**Done.** `tests/syntax.test.js`: a table of 21 texts and what the page finds in each (bold, italic both ways, underline, strikethrough, `==` and `{==…==}` highlights, a color, a note, a link to a note with and without its own words, a footnote mark and its text, headings, a quote, both lists, a code block, a scene break, a Markdown link), and 11 it must leave as text (a lone `[[`, `[[]]`, a link across two lines, a highlight or a note with no end, `===`, an unclosed `**`, inline code, a setext heading, indented code). `readNote` reads each kind and `(resolved)`; a note written by `noteBody` is one note in the text, ending at its own `<<}`, and reads back as it was; `footnoteTexts`, `wikiParts`, and `linkTarget` (which refuses `javascript:` and `file:`) have their cases. 42 tests in all. Breaking `noteBody`'s escaping on purpose made the round trip fail; escaping only `<` or only `}` is enough, which the first try at breaking it showed.

Found on the way, left as they are: the page shows `![image](x.png)` as a link and `<b>…</b>` as raw tags, where the format crate reads both as text on import. Which is right is the maintainer's call, so neither is in the table.

### 3. Writing

`writing.js`, through the stand-in view.

- `marksAt`: what the format toolbar lights for the caret and for a selection (bold, italic, a heading level, a note, a link).
- `runCommand`, each command once on and once off: bold, italic, underline, strike, a heading, body text, a quote, a list, a code block, a scene break, a highlight of each color and clearing it, adding and removing a note of each kind, adding and removing a link.
- Smart typography and pairing: straight quotes become curly ones in prose and stay straight in code and links; `--` becomes a dash and `...` an ellipsis; a bracket or mark gets its pair only when pairing is on.
- Enter continues a list or a quote and ends an empty item; Shift+Enter is a line break in the paragraph.
- Pasting (`insertPasted`) puts Markdown at the selection.
- `[[` offers the notes' titles, narrowed by what is typed after it, and nothing outside a link.

### 4. The preview and the review

`preview.js` and the parts of `places.js` that need no layout.

- The preview hides each mark where the caret is not, shows it where it is, and hides none in Source mode; a link to a title no note has is marked faint, and one to a known note carries its tip; a footnote mark shows its number.
- `reviewItems` lists a scene's notes and highlights in order, and `selectReview` selects the one asked for, as the review panel does.
- Spelling underlines the words it is given and none inside code, links, or a note's markup; `findWord` finds each copy.
- Fading marks the sentence or the paragraph the caret is in.

### 5. In CI

- A job, beside the Rust ones on Linux, that sets up Node, runs `npm ci` and `npm test` in `crates/ui/codemirror`, then `npm run build` and fails if `crates/ui/assets/codemirror.js` or `LICENSES.txt` is not what the sources make.
- `documentation/ARCHITECTURE.md` says the page has tests and how they run; `.claude/rules/testing.md` says to run `npm test` for a change to the page.

## How changes are checked

- `npm test` passes, on this computer and in CI, and fails when a module's behaviour is broken on purpose (one case per stage, tried and put back).
- CI fails on a source change whose bundle was not rebuilt, and passes when it was.
- The page in the program behaves as before: building the bundle after stage 1's changes gives the same program, and the maintainer tries the page, as in earlier phases.
