# Phase 8 — UI and UX overhaul

Not started.

The window works, but it is hard to use. It looks and behaves like a code editor: a monospace font for prose, an "Explorer", a bright blue status bar, buttons that look like plain text, and three columns in every mode, whether the mode needs them or not. This phase rebuilds the window around writing. It changes how the window looks and behaves, not what a project holds: no file format, storage, or sync change belongs here.

It does not wait for Phase 6 or 7. Phase 7's packaged builds and accessibility pass are easier once this is done.

## What is wrong now

Found by running the window under a virtual display (Xvfb) at 1400 × 860 and at the default 800 × 600, with a small project open, in every mode.

**The manuscript**

- Prose is set in a monospace font (`--font: Consolas, …` in `crates/ui/assets/style.css`). The text has no page: it is the same color as the panels and runs the full width of its column.
- The format toolbar is 25 text buttons in three rows (Bold … Break, Comment, Note, four highlight colors, Track changes, H1–H3, Quote, List). At 800 × 600 it becomes 14 rows, and the text column is about 170 pixels wide, below the fold.
- The toolbar does not show which marks are on. Link and Note ask in a browser `window.prompt` (`studio.rs`, `review.js`).
- The note list, tracked changes, and spelling list each stack above the manuscript and push it down when more than one is in use.

**Layout**

- Every mode shows the binder and its tools, including Plot, Characters, World, and Research, where the binder is not used. Writing and History also show the inspector, which in History repeats the Writing inspector for no reason.
- The tab strip holds the scene tab and also Save, Backup, a book menu, Markdown, DOCX, and Markdown folder. At 800 wide these are cut off.
- Nothing adapts to a small window: columns keep their width and the content clips. There is no sensible minimum window size.

**Controls**

- Most buttons have no border or background (Save, Part, Chapter, Scene, Import, Empty trash, Open, Add motivation, Delete, Move to trash), so they read as labels. Primary buttons come in several sizes (full-width Add character, small Connect).
- Native `select` elements are not styled. On the dark theme they are white boxes: the binder's Book menu is an empty white box, the status bar's spelling and theme menus are white with unreadable text, and Characters has a full-width white "Link a scene" bar.
- Dangerous actions sit beside ordinary ones with the same look: Delete under a character, Move to trash under the inspector, Empty trash next to Trash.

**Navigation and status**

- The mode icons have no names. Two more icons at the bottom (show the inspector, focus) look like modes but are not. The shortcuts do not follow the icons: History is the last icon but Ctrl+4. Choosing the mode that is already open hides the binder.
- The binder spends a column on the kind word (Scene, Chapter, Book) and repeats names ("Manuscript Manuscript", "Trash Trash").
- The status bar is the full blue strip of a code editor. It holds editable goal fields, two menus, and the save state. At 800 wide, "All changes saved locally" wraps onto two lines and the rest clips.

**The other modes**

- Corkboard cards are grey on grey with grey titles: they do not read as cards, and binder colors barely show. The sort and group choices are plain text.
- The outliner repeats the chapter name on every row, and "Open" is a text link at the far right.
- Empty modes are one line of grey text ("No plotlines yet.") with no picture of what goes there.
- History is one long column of sections (snapshots, earlier syncs, kept files, problems, sync, sessions, backups), mostly hint text.

**The code**

- `crates/ui/src/studio.rs` is one file of about 3,800 lines with some 50 components. `style.css` styles each pane's inputs on its own (`.plot-pane input`, `.cast-profile input`, …). There is no shared button, field, menu, panel, or empty state, so each pane looks a little different.

## What the window should be

- **The page comes first.** In Writing, the manuscript is the largest thing on screen at any window size. Everything else can be folded away.
- **A writing app, not a code editor.** A reading font for prose, a page with a limited line length, quiet chrome, and words and labels a writer uses.
- **Each mode shows what it needs.** The binder where the manuscript is being arranged or written; the cast list in Characters; nothing extra in History.
- **One set of controls.** Every button, field, menu, and panel comes from a small set of components and theme tokens, so a change to one changes all of them.
- **Light and dark are both finished.** Every control is readable in both.
- **Usable without a mouse.** Every control has a name and a visible focus, and every command can be reached from the keyboard.

## The work

In this order: each stage gives the next one something to build on. Stage 1 changes no behavior.

### 1. Split the window code, and make the shared pieces

- Split `studio.rs` by area: the frame (activity bar, top bar, status bar), the binder, the editor and its toolbar, the inspector, and one module for each mode. No change in behavior.
- Make the shared components every pane uses: button (default, primary, danger, icon with a name), text field and text area, menu (a styled `select`, or a custom list where a native one cannot be themed), panel with a heading, list row, empty state, and a confirm-by-second-click button that names its item and is cancelled by Escape.
- Rewrite `style.css` around theme tokens: color, type scale, spacing, radius. Remove the per-pane input rules once each pane uses the shared field.

**Done when** no pane styles its own inputs or buttons, and each component looks right in Light and Dark.

### 2. The frame

- The activity bar shows each mode's name, or a name on hover and focus where there is no room. Ctrl+1 to Ctrl+8 follow its order: Writing, Corkboard, Outliner, Plot, Characters, World, Research, History. Choosing the mode that is already open does nothing; hiding the binder has its own control.
- One top bar: the project and book name, the mode's own actions, and a project menu holding Save, Backup, Import, and Export (Markdown, DOCX, Markdown folder). Nothing sits in the tab strip but the scene name.
- The status bar becomes quiet text: this book's word count first, then today and this sitting, then the save state. Goals are set from a menu or in the outliner, not typed into the bar. The theme and spelling language move to a settings menu. The bar never wraps or clips.
- Below about 1,000 pixels wide the inspector folds away first, then the binder. Both come back as an overlay from their button. The window has a minimum size at which every mode still works.

### 3. The writing surface

- A reading font for prose (a serif and a sans to choose from), with the monospace font kept for code blocks only. A page: a limited line length of about 70 characters, a surface lighter or darker than the panels, and margins around it, in both themes.
- One toolbar row. The common marks (bold, italic, a heading menu, quote, list, link, note) are icons with names, and show when they are on at the caret. Highlights, strikethrough, track changes, and the rest go into a "More" menu. The manuscript keeps the caret and the active marks after a toolbar click.
- Link and Note ask in a small field in the window, next to the selection, in place of `window.prompt`.
- Notes, tracked changes, and spelling move into one review panel with a tab for each, which can be folded away or put at the side (moved here from the bonus list). Acting on an item from the panel leaves the caret where it was.
- Focus mode hides everything but the page and a word count. Escape still leaves it.

### 4. The binder and the inspector

- The binder shows each item's kind as a small icon, so the title has the room. No repeated names. Chapter and Scene stay as buttons above it; Book, Part, Folder, and Import go into an "Add" menu. Trash and Empty trash go to the Trash row's own menu, away from adding.
- The book filter is a styled menu with the current book shown, not an empty box.
- The inspector shows only Writing's needs: title, synopsis, status, label, and color at the top; scene beats and diagnostics in a section that can be folded. Move to trash is a danger button at the bottom, set apart.

### 5. The other modes

- **Corkboard.** Cards that look like index cards: readable titles, the binder color as a band, synopsis, status, and word count. Sort and group as a styled menu.
- **Outliner.** Chapter as a group row, not repeated. The title opens the scene; no separate Open link. Columns size to their content.
- **Plot, Characters, World, Research.** Each is a list beside an editor, without the binder. Delete is a danger button set apart from the fields. Scene links use the shared menu.
- **History.** Sections that fold, in order of use: this scene (snapshots, earlier syncs, versions kept from outside), then the project (backups, sync, problems, writing sessions). No inspector.
- **Empty states.** Every mode with nothing in it says what goes there and has the button that adds the first one.

### 6. Keyboard and accessibility

- Every icon button has a name, not only a tooltip, and a visible focus ring.
- A window that lists the shortcuts, opened from Help and by a shortcut.
- Contrast that meets WCAG AA for text and controls in both themes.
- The order of Tab through the window follows the layout.

## How changes are checked

The tests in this repository are the only testing for now (see "How the work is done now" in [../README.md](../README.md)). State tests in `crates/ui/src/state.rs` cover any behavior this phase moves, such as the mode shortcuts, Escape cancelling a confirmation, and what each mode shows.

How the window looks is not tested. Screenshots cost too much to take and read, so there is no screenshot script and no stage needs one. The research above ran the real window once under Xvfb, with a short-lived change that opened a sample project; that change was not kept. Keep layout simple enough to reason about from the code: the shared components and theme tokens of stage 1 are what keep the panes consistent.

## Not in this phase

- Changes to the project format, storage, or sync.
- New writing features. A feature that only becomes reachable through a new layout is fine; a new kind of thing to write or track is not.
- Packaged builds and the platform accessibility pass (Phase 7). Export profiles and compile templates (Phase 6).
