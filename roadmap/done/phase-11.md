# Phase 11 — Tags, story links, and a mind map

Done. Every stage is on `master`, 13 to 16 added after trying the window, and the whole workspace's tests pass. Looking at it in the window, by hand, is left for a later phase, as the maintainer decided; so is the Phase 7 check of the packages.

Phase 9 gave binder items colored keywords and saved filters. This phase does four things with what is there:

- It takes what Obsidian does well with tags (nesting, a tag list with counts, searches that combine tags, tags in a Markdown file's front matter) and fits it to those keywords. The "Tags" entry of the bonus list moves here.
- It lets a writer say, from the scene, which characters, places, and notes it needs, and from a chapter for all its scenes; and it gathers all of that into one **scene context**, shown beside the text and ready for any assistance added later.
- It turns the corkboard into a mind map of the manuscript, with room for ideas and the story's entries around it.
- It moves the choice of view to the library, so the middle of the window only shows the page, the mind map, or the outline.

## What Obsidian does

Obsidian is a Markdown note app. From its help pages (sources at the end):

- **Two ways to tag.** `#tag` typed in a note's text, or a `tags` list in its front matter (properties). Both count as the note's tags.
- **What a tag may hold.** Letters, numbers, `_`, `-`, `/`, and most Unicode, emoji included. No spaces, and not only digits: `#1984` is not a tag, `#y1984` is. Both rules come from finding tags inside text.
- **Case.** `#Tag` and `#TAG` are one tag, shown as it was first spelled.
- **Nesting.** `#inbox/to-read` sits under `inbox`. A search for `inbox` finds its children too; a search for `work` does not find `#myjob/work`.
- **The Tags view.** Every tag with the number of notes that carry it, as a tree or a flat list, sorted by name or by count. A click searches for the tag; Ctrl-click adds it to the search.
- **Search.** `tag:` combines with other terms: a space is AND, `OR` is either, `-` leaves out, and parentheses group.
- **What it does not have.** Its help pages describe no tag colors and no renaming a tag everywhere.

## What Writing Studio has already

Phase 9's keywords already work like Obsidian's tags in most ways:

| Obsidian | Writing Studio today |
| --- | --- |
| Many tags on a note | Many keywords on a binder item |
| Case does not matter; the first spelling shows | The same (`same_word` in `crates/project/src/binder.rs`) |
| Suggestions when adding | The inspector suggests keywords already used |
| A tag nothing uses disappears | The same, with its color |
| Click a tag to filter | The corkboard and outliner keyword menu, and saved filters |
| No tag colors | Each keyword has a binder color |
| Nothing like it | A chapter's, part's, or book's keyword covers its scenes |

Missing: nesting, a keyword list with counts, filters that combine keywords, renaming and merging, and keywords in Markdown front matter.

The story's entries are half connected already. Every character, place, culture, lore entry, timeline event, and research note can be linked to scenes, by id, from the entry's own page (`scene_links` in `bible.json`). A plot point can point at a scene. Scene diagnostics already find which characters a scene names (`crates/diagnostics`). What is missing: making those links from the scene, linking a chapter for all its scenes, seeing them together while writing, and the links on any map.

## Story links, not a keyword for each entry

The first idea was to give each entry a keyword of its own, so that tagging a chapter with `Mara` connects it to the character Mara. The result is right; the way to get there is the links the project already has, by id, made to feel like keywords:

- **A keyword is a name, a link is an id.** Renaming Mara would have to rewrite her keyword on every item, two entries with the same name would share one, and a keyword typed by hand that matches no entry would point at nothing. A link follows the entry whatever it is called.
- **One record of who is where.** `scene_links` already says it. A keyword for each entry would be a second record that can disagree with the first.
- **The keyword list stays readable.** A book can have a hundred characters and places; mixed with `flashback` and `plot/subplot` they would bury them. The story's entries get their own group in the list instead (stage 7).

So a writer gets what was asked for: chips in the scene's inspector, with suggestions as they type, on a chapter for all its scenes, listed with counts, filtering the binder, drawn on the map, and gathered into the scene's context. Underneath, each chip is a link.

## What this phase does not take

- **`#tag` typed in a scene's text.** Scene text is the book: compile would have to find and remove every tag, and "#1 bestseller" would read as one. Keywords stay outside the text, as the bonus entry already said.
- **No spaces, and not only digits.** Obsidian needs those rules because it finds tags in text. Keywords are never looked for in text, and many already have spaces (labels became keywords in Phase 9). Only the Markdown export has to write a space differently (stage 6).
- **Linking by itself.** A character named in a scene is offered as a link (stage 8); the writer decides.
- **Assistance.** The scene context is built so that assistance can read it later ("Optional local assistance" in [bonus.md](../todo/bonus.md)); nothing in this phase sends text anywhere or runs a model.

## The work

In this order: the layout change first, because every later stage adds to the library. Then keywords, which change one field of the format. Then story links and the scene context, which the mind map draws. The mind map last, as its third stage adds a project file. The stages stand on their own, so the phase can be split at the mind map if it runs long.

### 1. Views in the library

**Done.** The work bar has no view switch. The library has a **Views** heading after the books, with Mind map and Outline (`WorkspaceMode::LIBRARY_VIEWS`), before the saved filters and Extras (`LIBRARY_EXTRAS`); both lists are drawn by one `ModeRow`. A test checks that every mode but the page has exactly one library row. The modes are named "Mind map" and "Outline" in the library, their tooltips, and the shortcut list; the corkboard keeps its code and its "filter the corkboard" wording until stage 10.

- The Page / Cards / Outline switch leaves the top of the editor. The library lists **Mind map** and **Outline** above "Extras", under the books they show. Choosing a scene in the library, the scene list, or the outline opens it on the page, as now.
- The middle of the window shows only the page, the mind map, or the outline. Characters, World, Plot, Research, and History keep their own places, as now.
- Ctrl+1 to Ctrl+3 keep their order (page, mind map, outline), and the library row of the view that is showing is marked, as a book or a filter is.
- Until stage 10 the "Mind map" row opens the corkboard as it is.

### 2. Nested keywords

**Done.** `crates/project/src/binder.rs` has `keyword_covers` (a keyword or one nested under it), `same_keyword`, and `tidy_keyword`, all ignoring case in any alphabet. Saved filters and the outline's keyword menu (`keyword_allows`) match with `keyword_covers`; the binder search already matched keywords by any part of the name, so it finds nested ones as it is. `add_keyword` tidies what the writer types and the inspector says why a keyword is refused, keeping it in the field; `carry_keyword` keeps an older label or a merged scene's keyword as it is. The keyword menus offer `BinderTree::keyword_choices`: every keyword in use, each after any parent no item carries itself, so `plot` can be chosen while only `plot/subplot` exists.

- A keyword with `/` is nested: `plot/subplot` is under `plot`. Choosing `plot` anywhere also matches `plot/…`: saved filters (`BinderTree::scenes_matching`), the outline's and the map's keyword menu (`StudioState::keyword_allows`), and the binder search. A keyword is never matched by a part of its name: `subplot` does not match `plot/subplot`.
- A keyword is tidied when it is entered: a leading `#` is dropped, spaces around `/` go, and empty parts (`a//b`, a `/` at either end) are not allowed. Keywords already in a project are read as they are.
- `keyword_allows` compares with `eq_ignore_ascii_case`, so `Ação` and `AÇÃO` differ there and match everywhere else. One shared comparison, Unicode-aware like `same_word`, fixes it.
- No file changes: a nested keyword is a name with `/` in it.

### 3. A keyword list in the library

**Done.** `BinderTree::keyword_tree` gives the rows (`KeywordRow`: the whole name, the last part shown, depth, count, children), sorted by `KeywordSort`; a parent no item carries counts its children's items, and an item carrying `plot` and `plot/subplot` counts once. The library's **Keywords** section, after the saved filters, shows only once a keyword exists, with a "By name" / "By use" switch. A click sets `keyword_filter`, which the scene list (`scene_groups`) now honors as the corkboard and outliner already did; a second click or Escape clears it, and from Characters or another mode it opens the page beside the list. Folding is kept for the session (`folded_keywords`).

- A **Keywords** section in the library, after the saved filters: every keyword as a tree, with its color and the number of items that carry it, not counting the trash. A parent that no item carries itself still shows, holding its children.
- Sorted by name or by count; the tree folds like the binder.
- A click filters the binder to what carries the keyword or one of its children, as a search does. A second click, or Escape, clears it. The mind map and the outline follow the same filter.

### 4. Rename and merge

**Done.** `BinderTree::rename_keyword` renames on every item (the trash included), its children with it, in the keyword list with its color, and in saved filters; `items_with_keyword` and `keyword_merges` say what it will do. A name in use merges: an item carrying both keeps one, and the kept keyword's spelling and color win, for the children too. A new name is tidied like a typed one, and a rename that would make a child longer than the keyword limit is refused rather than cut. In the library, a pencil button or F2 on a keyword row opens the field; Enter renames, Escape leaves it. A rename happens at once and names how many items changed; a merge asks first, in the status line, and waits for a second Enter. Either is undone with the status line's Undo (`UndoOffer::Keywords`, from a snapshot of every item's keywords, the list, and the filters), not with Ctrl+Z, which stays with the text. The chosen keyword follows a rename.

- A keyword is renamed from its row in the list, on every item that carries it, its children included: `plot` to `story` makes `plot/subplot` into `story/subplot`.
- Renaming to a keyword that already exists merges the two; the color of the one kept stays. It asks first, naming how many items change.
- One undo step. Saved filters that name the keyword follow it.

### 5. Filters that combine keywords

**Done.** `SavedFilter` has `keywords`, `any`, and `without`, and no `keyword` or `label`. `BinderTree::keywords_match` checks an item's keywords with those of what it is in (`keywords_through`), each taking in its children; saved filters and the binder search both use it. The filter form adds keywords from a menu as chips, offers "Any of them, not all" once there are two, and has a "Leave out" list the same way; `filter_parts` names a new filter and describes it on its row ("flashback, not draft"). `KeywordQuery::parse` splits the search into `#keyword`, `-#keyword`, and the words, which alone go to the text index. Reading `binder.json` refuses a filter with `keyword` or `label`, saying what to change; `documentation/PROJECT_FORMAT.md` describes both. A filter's status is now the scene's own: before, it matched through the chapter, so "Draft" took every scene in a draft chapter.

- A saved filter takes several keywords, matching all of them or any of them, and keywords to leave out. Status and text stay as they are.
- In the binder search, `#plot` is a keyword (with its children), and `-#plot` leaves it out; other words are matched as now. Several are all required, as a space is AND in Obsidian. `OR` and parentheses are not added: a saved filter is where "any" is chosen.
- In `binder.json`, a filter's `keyword` becomes `keywords`, a list, beside `any` and `without`; `documentation/PROJECT_FORMAT.md` and its version notes say so. Following "No compatibility with earlier versions" in [../README.md](../README.md), nothing converts an old filter: a project whose filters still have `keyword` is refused with a message that says why.

### 6. Keywords in Markdown front matter

**Done.** The Markdown folder export writes an item's keywords as `tags:` (`tags_header` in `crates/storage/src/markdown.rs`), a space as `-`; an item without keywords has no `tags:`. `import_markdown_scene` reads `tags:` and `keywords:`, as a list or one line, through `tidy_keyword`, so `#` goes; the import then adds them with `add_keyword`, which gives a new keyword its color, and saves the binder. Only an item's own keywords are written: Obsidian has no chapters for a scene to take them from. A keyword that is only digits (`1984`) is written as it is, though Obsidian does not take it as a tag.

- The Markdown folder export writes each scene's keywords as `tags:`, a list in its front matter, as Obsidian reads it. A space in a keyword becomes `-`, as a tag cannot hold one.
- Markdown import reads `tags:` (and `keywords:`) into the new scene's keywords, after a leading `#`; `label:` is still read as now.
- A book exported this way opens in Obsidian with its tags, and its Tags view lists them.

### 7. Story links from the scene

**Done.** No model changed: an entry's `scene_ids` already held binder ids, and now holds a chapter's, part's, or book's too. `bible.json` writes a link's target as `item` and refuses one with `scene`, saying what to change. In the ui, `StoryEntry` and `story_entries` give every entry of every kind with its links; `story_reaches` follows a scene up to what it is in; `story_links_of` splits an item's own entries from those that reach it through its chapter, part, or book. The inspector of a scene, chapter, part, or book in the manuscript has **In this scene** (or chapter, part, book): chips that open their entry, × to unlink, the inherited ones apart and dashed as "from Chapter 1", and a field suggesting every entry as "Mara · character" (`link_story_named`, which also takes a name only one entry has, and says when none or several match). An entry's page now lists and links chapters, parts, and books as well as scenes (`link_choices`, `item_caption`). The library's **Story** group, under Keywords, lists by kind the entries with links outside the trash and how many scenes each reaches; a click narrows the scene list, corkboard, and outliner as a keyword does (`story_filter`, checked with the keyword by `chosen_allows`, which replaces `keyword_allows`), and Escape clears both. A saved filter has `entries` too, picked in the form under "With", and is named for them ("with Mara"). Links to an item in the trash hide, in the Story group and on the entry's page, and go when the trash is emptied, as scene links did.

- The inspector of a scene, chapter, part, or book has **In this scene** (or chapter, part, book): the characters, places, cultures, lore entries, timeline events, and research notes it is linked to, as chips with the entry's kind, like keywords. Typing suggests entries by name, of every kind; a chip opens its entry.
- A link from a chapter, part, or book covers every scene in it, as a keyword does; the scene shows those chips apart, as "from Chapter 3", and does not repeat them.
- The entry's page lists everything it is linked to, chapters as well as scenes, as it lists scenes now, and links from there as now. It is one record: a link made on either side shows on both.
- The library's Keywords section gets a **Story** group under it: the entries that have links, by kind, each with the number of scenes it reaches. A click filters the binder to them, as a keyword does; a saved filter can name entries too.
- In `bible.json`, a link's `scene` becomes `item`, as it can point at any binder item. Nothing converts an old file (the README's rule again): one with `scene` is refused with a message that says why. The SQLite cache's `bible_scene_links` follows. Sync merges links as it does now. A link to an item in the trash hides, and goes when the trash is emptied, as a scene link does now.

### 8. Named in the text, offered as a link

**Done.** `StudioState::named_in` finds, in a scene's text, the entries not linked to it (directly or through what it is in) and not hidden for it: characters with diagnostics' `mentions` (any capitalized part of the name, so Rose is not found in "the sun rose"), now public; places, cultures, and lore with the new `phrases_in` in `crates/diagnostics`, the whole name as words in a row, ignoring case. Timeline events and research notes are not looked for: their titles are phrases, not names. `refresh_named_here` runs when a scene is opened, when its text is saved (`flush_editor`), and after a link or unlink, and keeps the result in `named_here`, so drawing the inspector searches nothing. The inspector shows "Named here:" under the scene's links, each with + to link and × to hide (`link_named`, `hide_named`). A hidden suggestion stays hidden until the program closes.

- The scene's inspector lists the entries its text names that it is not linked to: "Named here: Mara, Port Ely". One click links each; one more hides the suggestion for that scene.
- Names come from each entry's name and, for a character, the names diagnostics already look for (`crates/diagnostics`, `mentions`); places, cultures, and lore by their names. Matched as whole words, ignoring case, on the scene's text when it is saved, not on every key press (`.claude/rules/performance.md`).
- Nothing is linked without the writer's click.

### 9. The scene context

**Done.** `crates/context` has `scene_context` (`SceneContext`: title, synopsis, status, keywords with the chapter's, place from the book down, the scene before and after in the book's order, plot points by plotline, and every entry that reaches the scene, characters first, each saying which chapter, part, or book brings it) and `SceneContext::to_markdown(limit)`. It also holds the story's entries (`StoryEntry`, `story_entries`, `reaches`, and `EntryKind`, which the ui uses as `BibleLink`), so the ui and the context read the same list. The inspector of a scene shows **Context**, folded: each entry's name opens the start of its text in place, and "Copy context" puts the Markdown, at most 16,000 characters, on the clipboard through the page (`navigator.clipboard`), saying whether it worked. `documentation/ARCHITECTURE.md` lists the crate. The `document` crate is not read: the context holds no scene text.

- One function gathers everything a scene needs, in this order: the scene's title, synopsis, status, and keywords; where it is (book, part, chapter, and the synopsis of the scene before and after); its plot points; and every entry it is linked to, directly or through its chapter, part, or book, each with its kind, name, and text, characters first.
- It lives in a crate of its own, `crates/context`, that reads the model crates (`project`, `characters`, `worldbuilding`, `research`, `plotting`, `document`) and not the UI or the storage, so anything added later (assistance, an export) can call it with a project's data.
- The inspector shows it as **Context**: the entries' names, each opening a short summary of its text in place, so a writer starting a scene sees who and what is in it without leaving the page.
- It also writes the context as plain Markdown ("Copy context", to the clipboard), which is what an assistant would read. A context is capped at a length the function is given, dropping the entries' text from the end of the list first and saying so.

### 10. The mind map

**Done.** `crates/ui/src/mindmap.rs` lays the map out (`layout`, tested: every node placed in binder order, none overlapping, folded branches closed) and moves between nodes (`step`, tested); `studio/mindmap.rs` draws it as SVG. The middle is the book the series menu chose, the only book, or the manuscript when there are several (`map_root`). Nodes show title, status or how many scenes they hold, and up to five keyword dots; a click chooses a node and the inspector, now in the mind map too, shows it, while a chosen scene's synopsis and word count show under the map; a double click or Enter opens a scene. Branches fold from their edge or with Space, for the session (`folded_map`). Zoom: "Fit" draws the map to the window, − and + or Ctrl+wheel set a size the pane scrolls around (`map_zoom`), which is how it pans. The filter (with `#keyword` as in the binder search), the keyword menu, and a chosen keyword or story entry dim what does not match. `WorkspaceMode::Corkboard` is `MindMap`; the corkboard pane, its card styles, and the `writing-studio-corkboard` crate are gone, card grouping with them, and the outline's toolbar class is `view-tools`.

The corkboard becomes a mind map of the manuscript.

- The book in the middle; its parts, chapters, and scenes as branches around it, read from the binder in binder order. With a series, the map shows the book chosen in the series menu, as the corkboard does.
- A node shows the title, the status, and the keyword colors as dots; a scene's synopsis and word count show when it is chosen. Choosing a node shows it in the inspector; opening a scene takes it to the page.
- A branch folds and unfolds. The map pans and zooms, and fits the window when it opens.
- What the corkboard did stays: the text filter and the keyword menu dim what does not match instead of hiding it, so the shape of the book stays in view.
- It replaces the corkboard: the corkboard pane and the `corkboard` crate go in the same change, card grouping included, and the "Mind map" row of stage 1 opens the map.
- Laid out automatically, left and right of the middle, with no positions saved yet. Drawn as SVG in the page, with no new crate; the layout is computed in Rust and tested there.
- The keyboard moves from node to node with the arrows, opens with Enter, and folds with Space. The outline stays the view a screen reader uses.

### 11. Editing the book from the map

**Done.** A node is dragged with the mouse (SVG takes no HTML drag-and-drop): pressing on it starts the drag, three strips over each other node choose before, inside, or after, as a library row's thirds do, and letting go there drops it through `drop_in_binder`, which the library uses; a line or an outline shows where, red where it cannot go. Tab adds a child and Enter the next one beside it (`add_from_map`, which the library's `add_node` now shares `create_node` with), choosing it and opening its name under the map (the library's `RenameField`); F2 renames, Delete moves to the trash. Stage 10's Enter, which opened a scene, is Ctrl+Enter now, as Enter adds. Undo: `UndoOffer::Added` moves an addition back to the trash, and a drag, in the library as on the map, offers `UndoOffer::PutBack` (the trash's undo, renamed), which puts the item back where it was. Moving to the trash from the map does as the library's ×: at once, with Undo, not a question. The corkboard's own drag (`StudioState::set_drop_target` and `drop_on`) and `BinderTree::drop_on` and `drop_allowed`, which only tests used after that, are gone; the tests use `drop_at`.

- Dragging a scene onto another chapter moves it there in the binder, as dragging in the library does; dropped between two scenes, it goes between them. Parts and chapters move the same way.
- From a chosen node, Tab adds a child (a scene under a chapter, a chapter under a part or book) and Enter adds the next one beside it; F2 renames. Each is one undo step, and goes through the same binder functions as the library, so sync and the files see an ordinary binder change.
- Moving an item to the trash from the map asks first, as in the library.

### 12. Ideas, the story's entries, and lines

**Done.** `MindMap` in `crates/project/src/mindmap.rs` (ideas, lines, positions, each keeping unknown keys with `#[serde(flatten)]`, for which the crate now uses `serde_json`) is read and written by storage as `mindmap.json` (`read_mindmap`, `mindmap_json`, `ProjectStore::load_mindmap` and `persist_mindmap`, with a copy in the cache's `binder_lists` so backups hold it), carried in `ProjectContent`, and merged by sync by id. `layout_all` puts ideas in a row under the tree and shown entries in a row over it (`Extra`, `MapKind`), and moves hand-placed nodes, a binder node with its branch; tested. On the map: **+ Idea**, an idea's title and note under the map with **Make it a scene** and **Remove idea**; the **Story** menu (`MapStory`); **Join**, then the lines' words under the map; **Tidy**. Dragging decides by what moves (`drop_on_map`, `map_drop_allowed`): a binder node moves in the binder, an idea becomes a scene in a chapter or folder or hangs from anything else, an entry is linked to a book, part, chapter, or scene; let go on an empty part of the map, a node stays there, its place found by the page from the pointer (`place_at_pointer`, `getScreenCTM`). An idea or entry is chosen on the map without the inspector (`map_focus`). Emptying the trash forgets the deleted items' lines and positions (`MindMap::forget`).

- **Ideas** are nodes that are not in the book: a title and a short note, attached to the book or to any node, or loose. They are never compiled and never counted.
- **The story's entries** show on the map beside what they are linked to, from stage 7's links, one kind at a time or all, from a menu on the map: a character linked to three scenes is one node with three lines. Dragging an entry onto a scene or chapter links it there; the lines are the links, so the map, the inspector, and the entry's page always agree.
- **Lines** join any two nodes, scenes and ideas alike, with an optional word on the line ("echoes", "pays off"). A line is not a binder move and not a story link.
- **Positions.** A node the writer drags keeps where it was put; the others stay laid out automatically around it. "Tidy" puts everything back.
- **An idea becomes a scene**: dropped on a chapter, or with a menu item, it becomes a new scene there, its note the synopsis, and its lines go with it.
- Stored in a new file, `mindmap.json`: `ideas`, `lines`, and `positions` by item id (entries included). Story links stay in `bible.json`. Sync merges `mindmap.json` by id, as `bible.json` and `plot.json` are, and keeps keys the program does not know. Lines to an item in the trash hide, and go when the trash is emptied. A project without the file has no ideas or lines. `documentation/PROJECT_FORMAT.md` describes it.

### Added after trying the window

The window had four columns: the binder, the scene list, the page, and the inspector on the right, which crowded the page. These stages make it three: the **binder**, with scenes nested in their chapters and the story's worldbuilding as trees of its own; **Context**, where the scene list was, holding what the inspector held; and the page, mind map, or outline. Context is for what is said about the work, never the work itself.

### 13. Scenes in the binder, and the Context panel

**Done.** `StudioState::binder_rows` gives the tree (`BinderRow`: depth, kind, title, status, keyword colors, left out, scenes held and finished), with scenes in their chapters; `shown_scenes` is the set a search, saved filter, keyword, or story entry narrows it to, keeping what holds a match and opening what is folded; `trash_rows` lists the trash's contents under its row. One `LibraryRow` draws every row: a scene opens on the page (Ctrl+click and Shift+click pick), anything else is chosen and shows in Context. The search box is over the tree; **+ Add** adds a scene too. `ContextPane` (the inspector's `Inspector`, moved) sits between the binder and the page, with Restore and Empty trash when the trash is chosen, in Writing, the mind map, and the outline. The binder takes the old scene list's width setting and is resizable; Context takes the inspector's; Context folds below about 960 pixels, the binder below 660. The scene list, its cards, `scene_groups`, `SceneCard`, `SceneGroup`, `library_rows`, and the search index's `opening` (only the cards used it) are gone. `binder_matches` moved to the state.

- The scene list goes. The binder lists scenes in their chapters (and in the parts, books, and folders they sit in), each row with its title, status, and keyword colors, and the chapter's ring of finished scenes. Choosing a scene opens it on the page; choosing a book, part, chapter, or folder shows it in Context.
- Where the scene list was, **Context** held (it moved back to the right of the page at the end of the phase) what the right-hand inspector held: progress, synopsis, status, keywords, In this scene, Named here, the scene context, beats and diagnostics, snapshots, split and merge, include in compile, color, and Move to trash. The right-hand inspector goes; its button and Ctrl+Alt+I show and hide Context.
- What the scene list did comes to the binder: the search box over the tree, which keeps the books, parts, and chapters that hold a match; Ctrl+click and Shift+click picking; dragging scenes as chapters are dragged; a saved filter, keyword, or story entry chosen narrows the tree to what matches; the trash's contents under Trash, with Restore and Empty trash in Context when the trash is chosen.
- On the mind map and the outline, Context shows the chosen node. Characters, World, and Research keep the main area until stage 14.

### 14. Worldbuilding in the binder

**Done.** Every kind of entry has `parent_id`, read and written as `parent` in `bible.json` and kept in the cache's new `bible_parents` table. `crates/context` has the trees (`Nested`, `children_of`, `tree_rows`, `is_within`, `move_entry`, tested), and `StoryEntry` its `parent`. The binder's **Worldbuilding** lists each kind with + and its tree (`WorldbuildingSection`, `EntryRow`): a row opens the entry's page, folds with the binder's folding (kept with the project), renames in place (`rename_entry`, through the binder's rename field), drags before, after, or inside another of its kind (`drop_entry`, `nest_entry`), adds inside (`add_entry`), and deletes after a second click, what it held moving up (`remove_entry`). The Characters, World, and Research views lost their lists and the World section tabs, and are each the open entry's page (`ENTRY_PAGES`); Context beside them shows the entry's links into the manuscript (`EntryContext`). Plot and History moved under Views; `LIBRARY_EXTRAS` is gone.

- "Extras" becomes **Worldbuilding**: Characters, Places, Cultures, Lore, Events, and Research, each a tree in the binder that nests freely within its kind: Country › City › Neighbourhood › Pub; Father › Son › Grandson. Plot and History move under Views.
- Entries are added (+ on a kind's heading, or inside a chosen entry), renamed in place (F2, double-click), dragged to nest or reorder within their kind, folded like binder rows, and deleted as now, with a second click.
- Choosing an entry opens its page in the main area: its name, its text (a character's summary, a place's or culture's summary, lore's body, an event's time and summary, a research note's notes and files), and what its kind has besides (a character's role and motivations). Context shows its connections and where it is in the manuscript.
- The Characters, World, and Research views go, with their code.
- In `bible.json`, an entry has `parent` when it is inside another of its kind. An entry without one is at the top, so this adds to the format and refuses nothing. Sync merges it as a field. `documentation/PROJECT_FORMAT.md` says so.

### 15. Labels and named connections

**Done.** Every kind of entry has `label`, in `bible.json` and the cache's new `bible_labels`; `StoryEntry` carries it. `Cast::connect` joins any two entries with a name (a `relationships` line, now between any kinds), `disconnect_all` drops an entry's, and removing a place or a note drops its connections. `crates/context` lists every connection (`connections`, `Connection`, `Tie`: named, inside, or a field), tested. Context for an entry has **What it is** (`set_entry_label`, suggestions from `labels_in_use`) and **Connections** (each way, × on named ones, and a form that connects to an entry typed or picked, `connect_entries` through `entry_named`, with names suggested from those in use). The binder shows a label beside its entry's name. The character page's relationship editor went, with `Cast::add_relationship`, `set_relationship_target`, and `relationships_from`, which only it and tests used.

- Every entry has a **label**: what it is ("Country", "Pub", "Father"), typed with suggestions from the labels its kind already uses, and shown beside its name in the binder, on its page, and on the map.
- **Connections** join any two entries, of any kinds, with a name ("lives in", "son of", "rules"), suggested from the names in use. They are added and removed in Context, and each side shows the other. Being inside another entry is a connection too ("inside"), without being added.
- A character's relationships become connections: `relationships` in `bible.json` may join any two entries, its `kind` the connection's name. A place's culture, lore's place and culture, and an event's place stay their own fields and show as connections.
- In `bible.json`, an entry has `label`. Both are additions to the format.

### 16. Worldbuilding on the mind map

**Done.** The map's layout takes any tree (`layout_tree`, `Tree`, `place_by_hand`, tested), and `StudioState::world_layout` builds the worldbuilding one: a middle and a node for each shown kind (`MapKind::Group`, fixed ids for the session), each kind's entries nested, or, with `map_center`, one entry with its parent, what it holds, and what it is connected to. `world_lines` draws named connections and fields as labeled lines, filtered by `TieFilter`; `world_kinds` says which kinds show. `current_map` gives whichever map is chosen. On it, Tab and Enter add entries (`add_on_world_map`), F2 renames, Delete deletes, Join connects two entries (named in Context), and dropping an entry on another of its kind nests it, on its kind brings it to the top. An entry chosen on the map shows in Context. Its state test passed, and so does the whole workspace's.

- A menu over the map chooses its middle: the book, as now, or **Worldbuilding**. Worldbuilding shows the chosen kinds' trees, nesting as branches, and the named connections as labeled lines between them, so how places and characters relate shows at a glance.
- Filters: which kinds (Places, Characters, and so on), which connection names, and one entry to center on, showing what it is inside, what it holds, and what it is connected to.
- Editing as on the book's map where it fits: drag to nest within a kind, Tab and Enter to add, F2 to rename, Delete, and Join, which draws a named connection (stored as a connection, not a map line).
- The book's map keeps its story entries (stage 12).

## Noticed along the way

What came up while this phase was planned and built, and while the Phase 7 packages were tried, that was not part of the plan. Each says whether it is fixed.

**Fixed**

- Compile wrote an empty paragraph under every chapter heading in Word, EPUB, and HTML, from a chapter's empty text, and the starter scene held an empty paragraph between its two lines (found trying the portable package; fixed on `phase-7` before this phase).
- The outline's keyword menu ignored case only for ASCII, so `Ação` and `AÇÃO` were two keywords there and one everywhere else (stage 2).
- Renaming `plot` to `Story` while `story` was in use spelled the children `Story/subplot`; they now take the kept spelling (stage 4, caught by its test).
- A saved filter's status matched through the chapter, part, or book, which are "draft" when made, so a "Draft" filter took every scene in a draft chapter whatever its own status. Status is the scene's own now (stage 5).
- `SavedFilter::label`, left over from before keywords, is gone with the filter format change (stage 5).
- `WorkspaceMode::view_title`, never called since Phase 8, is gone (stage 10).
- The Markdown folder export lost every keyword: it wrote a scene's `label`, which Phase 9 emptied by folding labels into keywords, and never `keywords`. It writes them as `tags:` now (stage 6).
- The status bar's "Scene 21", the open scene's word count that read like a scene number, says "21 words in this scene".
- The page inside the window was titled "Dioxus app"; it is "Writing Studio", as the window is (`document::Title`).
- A keyword longer than 40 characters was cut to 40 when a project was read, so two long ones could become one. Reading keeps it whole up to 200 characters, a limit for mistakes; typing still stops at 40.
- The Package workflow did not run on code changes, so its packages could be older than the code. It runs on changes to `crates/**` and `Cargo.lock` too.
- Each entry's `scene_ids` held chapters, parts, and books after stage 7; the field is `linked_ids`, and the ui's `bible_scene_ids` is `bible_linked_ids`. Nothing stored changes: the bible is written field by field, and the cache's column keeps its name.
- An entry page's Open said "That scene is no longer in the binder" for a chapter too; it says "That is no longer in the binder."
- A character was found by any capitalized part of the name, so "The Pilot" was named by every "The". A name's articles, prepositions, and titles ("The", "Of", "Dr", "La", "De", and the like) no longer count, in diagnostics and in "Named here".
- A backup held no `mindmap.json`, as the map had no place in the cache. The cache keeps a copy now, so a backup holds the map and a restore brings it back.

**Open**

- **Two kinds of undo.** Ctrl+Z undoes text (and a merge or split through it). Trash, a drag, an addition on the map, and keyword renames and merges are undone from the status line's Undo button, which goes as soon as another message replaces it. A new keyword, a status, or a rename has no undo. One undo for every binder change is a piece of work of its own: the binder is saved as each change is made, so it needs a history of binder states beside the editor's.
- **The start screen says "Organize a manuscript the way Scrivener does"**, naming another product in the program's own words. A question for the maintainer, not a bug.
- **Mind map positions come from the page.** Where a node is let go is turned into map units by the page (`getScreenCTM`), as Rust cannot measure the drawing; nothing tests that step but the maintainer's look.
- **What lasts only until the program closes**, as the plan asked: a hidden "Named here" suggestion, keyword folding and the keyword list's order, and folded mind map branches. Keeping them would put them in `.local/settings.json`.
- **Not yet checked by hand:** everything in this phase in the window, opening a real research file through `ShellExecuteW`, the MSI install and removal, and everything on Linux (stage 6 of Phase 7, the maintainer's).

## How changes are checked

The tests in this repository are the only testing for now (see "How the work is done now" in [../README.md](../README.md)).

- Keyword matching is a unit test in `crates/project`: a parent matches its children and not the other way, the tidying of a new keyword, and case outside ASCII.
- Rename and merge, the counts, and combined filters are unit tests; a filter with the old `keyword` field is refused with its message.
- The Markdown export writes `tags:` and the import reads it back, with a space and a leading `#`.
- Story links: a link on a chapter reaches its scenes, a link made on either side shows on both, a renamed entry keeps its links, links to the trash go when it is emptied, and a `bible.json` with the old `scene` field is refused. Suggestions from the text find whole words only and never link by themselves.
- The scene context is a unit test of `crates/context`: what is in it, its order, a chapter's links included once, and the cap dropping text from the end.
- The map's layout is a unit test: every node placed, none overlapping, binder order kept. Moves, additions, and links made from the map are state tests that check the binder and the bible afterwards.
- `mindmap.json` round-trips, merges two machines' ideas and lines by id, keeps unknown keys, and a project without it opens with none.
- How the window looks is checked by the maintainer, as in Phases 8 and 9. The contrast test in `crates/ui/src/contrast.rs` covers any new colors.

## Not in this phase

- Keywords on the story's entries themselves (a character tagged `antagonist`). Entries are linked, not tagged; tagging them is in [bonus.md](../todo/bonus.md).
- Any assistance, local or remote, and anything that sends text from the computer.
- `OR` and parentheses in the binder search.
- A mind map for anything but the manuscript and what is linked to it.

## Sources

- [Obsidian: Tags](https://obsidian.md/help/tags)
- [Obsidian: Tags view](https://obsidian.md/help/plugins/tags)
- [Obsidian: Search](https://obsidian.md/help/plugins/search)
- [Obsidian: Properties](https://obsidian.md/help/properties)
