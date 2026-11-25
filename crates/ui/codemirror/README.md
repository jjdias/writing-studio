# The Markdown page

The page the writer types into: CodeMirror 6 with the scene's language (`syntax.js`), its live preview (`preview.js`), the commands and keys for writing (`writing.js`), and what keeps one's place (`places.js`), started by `entry.js`, which the program reaches as `window.wsEditor`. They are built into one file, `crates/ui/assets/codemirror.js`, which is committed and carried in the program, so nothing is loaded from the network.

With Node 20 or later, in this folder:

- `npm ci` installs the pinned packages.
- `npm run build` writes `../assets/codemirror.js` and `LICENSES.txt`. Commit both with a change to the sources.
- `npm test` runs the tests in `tests/` with Node's own runner. They work on CodeMirror's editor state, with a stand-in for the editor view (`tests/view.js`), so no window or browser is needed; what needs one (fixed scrolling, the line marker, hiding while typing) is tried in the program.
