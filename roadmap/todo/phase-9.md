# Phase 9 — Writing comfort, from Ulysses

In progress: stages 1 to 7 are done.

Phase 8 rebuilt the window around writing. This phase takes what the Ulysses writing app does well for the writer at the page, and fits it to how Writing Studio works: rich text on a page, a binder of books, chapters, and scenes, and everything on the writer's computer. It does not wait for Phase 6 or 7.

## What Ulysses does

Ulysses is a Markdown editor for Mac and iOS. Found from its help pages, release notes, and reviews (sources at the end):

- **Three columns that fold away.** A library of groups and saved filters, a list of the documents ("sheets") in the chosen group, and the editor. Either side column can be hidden, leaving only the text, and the toolbar fades while the writer types.
- **An editor the writer tunes.** Font, line length, line height, paragraph spacing, and first-line indent are settings. The current line, sentence, or paragraph can stay bright while the rest fades. Fixed scrolling keeps the line being typed at one height on screen, and a line marker tints it. These replaced an older "typewriter mode".
- **Goals with deadlines.** A goal can be words, characters, or paragraphs, and with a deadline the app says how many words a day are left to reach it.
- **A dashboard beside the text.** Progress, keywords, a navigation list (headings, links, footnotes, notes), attachments, and revision checks.
- **Keywords and filters.** Documents carry colored keywords, and filters work like smart folders over them.
- **Glue and split.** Selected sheets can be merged into one, and a sheet can be split at the caret.
- **Export with a preview** in the chosen style.

## What this phase does not take

- **One library with no projects.** A project folder that is plain files and syncs through git is the point of Phase 5.
- **Markdown as what is typed.** The manuscript is rich text; Markdown stays the import and export.
- **Grammar checks on a server.** Ulysses sends text out for its advanced checks. Nothing here leaves the computer, so checks stay local (Phase 4's diagnostics).
- **Export preview.** It belongs with export profiles in [Phase 6](phase-6.md), which is where to add it.

Ulysses has no corkboard or visual outline; Writing Studio keeps both.

## The work

In this order: the first stages are layout, settings, and display only, and the later ones change what the binder and the project hold.

### 1. A brighter, quieter layout

The window after Phase 8 still reads as a tool: a dark activity bar, panels with edges, a status strip. Ulysses' window (the reference images for this stage are of its Mac app) reads as a place to write: three light columns, separated by tone and space instead of lines, with color kept for what carries meaning. This stage moves the window toward that, clearer, brighter, and more minimal, without making it bland. It changes the layout and the look, not what a project holds.

- **Light by default, and finished.** A new project opens in Light. Surfaces are near-white, the left column a soft grey, the page white. Columns are set apart by tone and space; borders stay only where a control needs an edge. One accent color for selection and focus. Dark and Paper get the same structure with their own tones, and every color still passes the contrast test (`crates/ui/src/contrast.rs`).
- **Color where it means something.** Status shows as a small colored pill (Draft, Revise, and Final have their own colors; any other status the writer types gets a steady color from its name, so nothing new is stored). Goals show as a progress ring. Binder colors and keyword colors (stage 7) stay. That is what keeps the window from being plain: color is information, not decoration.
- **The library column (left).** At the top, the project's name large, then the current book's word target with a ring and its deadline with the time left (stage 5), and three small icon buttons: statistics, export, and more. Below, the manuscript's books, parts, and chapters as rows with line icons, the chosen one as a rounded, tinted row. Then a small "Extras" heading over Characters, World, Plot, Research, and Trash. These replace the dark activity bar: a mode is chosen where its content lives. History moves to the dashboard and the more menu.
- **The scene list (middle).** Headed by what is chosen in the library ("Chapters", a book, a part). Scenes grouped under their chapter, each group's heading with a small progress ring. Each scene is a card without a frame: its title in bold, the first two lines of its text, and its status pill. The chosen card is a rounded grey block. Drag and drop works here as in the binder. Corkboard and Outliner become two other ways to show this column's content, chosen from its header, so they keep everything they do now.
- **The editor (center).** White, with no frame. The scene title as a large heading at the top of the page, the word count small and grey at the top right. A slim top bar with icon buttons only: back and forward through scenes, search, the format marks of Phase 8 stage 3 (bold, italic, link, paragraph style, more), export, and the dashboard toggle.
- **The dashboard (right, optional).** Replaces the inspector and the review panel's place beside the page, as one column of quiet sections with small grey headings: Progress (characters, words, reading time), Keywords, Outline (the scene's headings), Notes and annotations, and the review tabs. Synopsis, status, label, and color are its first section for the chosen scene. History's sections for this scene are a tab here.
- **Folding.** Each side column folds from a button at its top, and the narrow-window rules of Phase 8 still apply: the dashboard folds first, then the scene list, then the library.
- **Chrome type.** The system UI font at a size that reads easily (13 to 14 px), headings in weight rather than capitals, and no uppercase labels except the small section headings.

Checked by the existing state and contrast tests, plus state tests for what each column shows when a library row is chosen. How it looks is not tested, as in Phase 8.

**Where it stands.** Done: Light is the default and its colors are the brighter set above, with library, active-row, and status-pill tokens in every theme (all in the contrast test). The library column (`studio/library.rs`) replaces the activity bar and the top bar: project name, goal ring, Project, Settings, and shortcuts buttons, the manuscript's containers (`BinderTree::manuscript_containers`), Extras, and Trash. The scene list groups the chosen container's scenes by chapter (`StudioState::scene_groups`), with each scene's opening kept by the search index (`ManuscriptIndex::opening`) and its status pill. Library rows and scene cards keep the free drag and drop. A slim bar over the page holds the library toggle, Page / Cards / Outline, focus, and the inspector toggle, and no title: the scene's title is on the page. Then: the scene title is the page's heading, renamed in place, with the word count at its right (`PageTitle`); each chapter heading in the scene list has a ring that fills as its scenes reach a final status; the inspector is the dashboard, with Progress (characters, words, reading time) above the scene's fields; and labels are in sentence case, not capitals. The review panel's tabs stay under or beside the page rather than moving into the dashboard: they need the page's width for tracked changes, and Beside already puts them in a column. **Stage 1 is done.**

### 2. Page settings

**Done.** `PageStyle` in `.local/settings.json` (`page`), set with sliders under the manuscript font in Settings and read by the page as CSS variables (`page_variables`). The defaults keep the look from before; each value is kept within its limits.

- Line length, line height, paragraph spacing, and first-line indent, beside the manuscript font in Settings. They are this computer's settings, kept in `.local/settings.json` next to `prose_font`, each optional with today's look as its default. The page tokens in `style.css` take them.
- Compile does not change: these are how the page looks while writing.

### 3. Keeping the place

**Done.** `KeepPlace` in `.local/settings.json` (`keep_place`), chosen in Settings. `place.js` follows the caret: it scrolls the desk to hold the line at the chosen height, moves a marker element on the desk (outside the manuscript), and sets the `ws-current` CSS highlight over the paragraph or sentence while the page's text color fades. Nothing is written into the manuscript. Updates wait for the next frame, so a burst of typing does the work once.

- Fixed scrolling: the line being typed stays at one height (top, middle, or bottom of the page), or the page scrolls as it does now.
- A line marker that tints the line with the caret.
- Fade the rest: the current sentence or paragraph stays at full strength, and the rest of the page fades. Both are choices in Settings and work in focus mode.
- These run in the page (`review.js` already follows the caret for the toolbar), and must not slow typing: no work per keystroke beyond finding the caret's line or paragraph.

### 4. Chrome that steps back

**Done.** `KeepPlace::quiet_bars`, on unless turned off in Settings. `place.js` marks the body while one types in the manuscript, and the format toolbar, the bar over the page, the status bar, and the side columns fade by CSS, keeping their room so the page does not move; a pointer move of more than a few pixels, a Ctrl or Alt chord, or leaving the page clears the mark.

- While the writer types, the format toolbar, the top bar, and the status bar fade, and they come back when the pointer moves or a key with Ctrl or Alt is pressed. A setting turns this off.
- Focus mode stays as it is: the page and a word count.

### 5. Goals with a deadline

**Done.** `BookWordGoal::deadline` (optional, `YYYY-MM-DD`, shared in `project.json`), set with **Finish by** in the word targets panel. `pace` gives the words a day left, counting today and rounding up, or that the target is reached or the day has passed. The library shows the date and time left under the goal; the status bar adds the pace. The project format document describes the field.

- A book can have a deadline beside its word target. The status bar and the word targets panel show how many words a day are left to meet it, from today's count.
- The deadline is shared with the book, in `project.json` with `book_word_goals`. It is optional: a project without one opens as it does now, and the format document says so.

### 6. Navigating the scene

**Done.** `Document::navigation` lists the headings, links, and notes block by block (within one paragraph, links before notes), from the document the editor holds. The review panel's **Navigate** tab, after the others so the panel still opens where it did, jumps to a heading or link with `wsShowBlock` and to a note with `wsSelectReview`.

- A Navigate tab in the review panel lists the open scene's headings, links, and notes in order. Choosing one moves to it in the page, the way the Notes tab already does.
- It reads the document the editor already holds, not the files.

### 7. Keywords and saved filters

**Done.** `BinderNode::keywords`, and `BinderTree::keywords` (one color each) and `filters`, in `binder.json` (the lists only once used). The dashboard edits keywords; cards show them as dots; the corkboard and outliner narrow to one. A filter is a library row; its matches come from `BinderTree::scenes_matching` and, for text, the search index. Sync merges keywords like any other field of an item and the two lists by name and by id (`merge_by_key`). A scene matches through the chapter, part, or book it is in, so a tagged chapter brings its scenes; the corkboard and outliner keyword menu works the same way. Labels were folded into keywords (`BinderTree::fold_labels_into_keywords`, run when a project is read and after a Markdown import); the corkboard groups by first keyword. The SQLite cache keeps keywords and the two lists too (`binder_nodes.keywords`, `binder_lists`), so saves that rewrite `binder.json` from it keep them. Status is a dropdown (To do, Draft, Revise, Done, On hold, `STATUSES`) in the dashboard and the filter form; a status from an older project stays a choice and shows with a capital. Not done: editing a saved filter, which is removed and made again for now.

- An item can carry several keywords, each with one of the binder colors, alongside the label it has now. The inspector edits them, the binder row shows their colors, and the corkboard and outliner can filter by them.
- A saved filter (keywords, status, label, or text) shows in the binder as its own row and lists what matches across the books.
- Keywords and filters are shared with the project, in the binder's files. This is a format addition: the old label stays, older projects open with no keywords, and the format document and its version notes describe it.

### 8. Merge and split scenes

- Merge joins the selected scenes into the first, in binder order, and moves the rest to the trash, so nothing is lost until the trash is emptied. Notes, highlights, and tracked changes go with their text.
- Split makes a new scene from the caret to the end of the open scene, placed right after it.
- Each is one undo step, takes a snapshot of every scene it changes first, and says what it did in the status line.

## How changes are checked

The tests in this repository are the only testing for now (see "How the work is done now" in [../README.md](../README.md)).

- Settings round-trip through `.local/settings.json`, and an older settings file opens with today's defaults (as `prose_font` does).
- The deadline arithmetic (words a day left, on the day of the deadline, after it) is a unit test.
- Keyword and filter storage round-trips, and a project from before this phase opens with none.
- Merge and split are state tests: the text, notes, and tracked changes end up in the right scene, the snapshots are there, and undo puts it back.
- How the window looks is not tested, as in Phase 8. The contrast test in `crates/ui/src/contrast.rs` covers any new colors.

## Not in this phase

- Export profiles and the export preview (Phase 6). Packaged builds (Phase 7).
- Sync changes, or any feature that needs the network.
- A Markdown editing mode.

## Sources

- [Ulysses: Customize the Editor](https://help.ulysses.app/dive-into-editing/editor-customization-guide)
- [Ulysses release notes](https://ulysses.app/release-notes/)
- [MacStories: Ulysses 20 review](https://www.macstories.net/reviews/ulysses-20-review-new-dashboard-featuring-advanced-grammar-and-style-check-outline-and-much-more/)
- [MacStories: Ulysses 13, goals and keywords](https://www.macstories.net/reviews/ulysses-13-brings-upgrades-to-writing-goals-keywords-and-code-blocks/)
- [TidBITS: Ulysses blends power and simplicity](https://tidbits.com/2016/08/13/writing-app-ulysses-blends-power-and-simplicity/)
- [The Sweet Setup: Ulysses 15](https://thesweetsetup.com/ulysses-15-introduces-split-view-editor-new-image-export-and-previewing-features-and-keyword-management-improvements/)
