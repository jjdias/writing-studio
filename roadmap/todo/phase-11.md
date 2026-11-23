# Phase 11 — Tags, story links, and a mind map

In progress: stages 1 to 3 are done.

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
- **Assistance.** The scene context is built so that assistance can read it later ("Optional local assistance" in [bonus.md](bonus.md)); nothing in this phase sends text anywhere or runs a model.

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

- A keyword is renamed from its row in the list, on every item that carries it, its children included: `plot` to `story` makes `plot/subplot` into `story/subplot`.
- Renaming to a keyword that already exists merges the two; the color of the one kept stays. It asks first, naming how many items change.
- One undo step. Saved filters that name the keyword follow it.

### 5. Filters that combine keywords

- A saved filter takes several keywords, matching all of them or any of them, and keywords to leave out. Status and text stay as they are.
- In the binder search, `#plot` is a keyword (with its children), and `-#plot` leaves it out; other words are matched as now. Several are all required, as a space is AND in Obsidian. `OR` and parentheses are not added: a saved filter is where "any" is chosen.
- In `binder.json`, a filter's `keyword` becomes `keywords`, a list, beside `any` and `without`; `documentation/PROJECT_FORMAT.md` and its version notes say so. Following "No compatibility with earlier versions" in [../README.md](../README.md), nothing converts an old filter: a project whose filters still have `keyword` is refused with a message that says why.

### 6. Keywords in Markdown front matter

- The Markdown folder export writes each scene's keywords as `tags:`, a list in its front matter, as Obsidian reads it. A space in a keyword becomes `-`, as a tag cannot hold one.
- Markdown import reads `tags:` (and `keywords:`) into the new scene's keywords, after a leading `#`; `label:` is still read as now.
- A book exported this way opens in Obsidian with its tags, and its Tags view lists them.

### 7. Story links from the scene

- The inspector of a scene, chapter, part, or book has **In this scene** (or chapter, part, book): the characters, places, cultures, lore entries, timeline events, and research notes it is linked to, as chips with the entry's kind, like keywords. Typing suggests entries by name, of every kind; a chip opens its entry.
- A link from a chapter, part, or book covers every scene in it, as a keyword does; the scene shows those chips apart, as "from Chapter 3", and does not repeat them.
- The entry's page lists everything it is linked to, chapters as well as scenes, as it lists scenes now, and links from there as now. It is one record: a link made on either side shows on both.
- The library's Keywords section gets a **Story** group under it: the entries that have links, by kind, each with the number of scenes it reaches. A click filters the binder to them, as a keyword does; a saved filter can name entries too.
- In `bible.json`, a link's `scene` becomes `item`, as it can point at any binder item. Nothing converts an old file (the README's rule again): one with `scene` is refused with a message that says why. The SQLite cache's `bible_scene_links` follows. Sync merges links as it does now. A link to an item in the trash hides, and goes when the trash is emptied, as a scene link does now.

### 8. Named in the text, offered as a link

- The scene's inspector lists the entries its text names that it is not linked to: "Named here: Mara, Port Ely". One click links each; one more hides the suggestion for that scene.
- Names come from each entry's name and, for a character, the names diagnostics already look for (`crates/diagnostics`, `mentions`); places, cultures, and lore by their names. Matched as whole words, ignoring case, on the scene's text when it is saved, not on every key press (`.claude/rules/performance.md`).
- Nothing is linked without the writer's click.

### 9. The scene context

- One function gathers everything a scene needs, in this order: the scene's title, synopsis, status, and keywords; where it is (book, part, chapter, and the synopsis of the scene before and after); its plot points; and every entry it is linked to, directly or through its chapter, part, or book, each with its kind, name, and text, characters first.
- It lives in a crate of its own, `crates/context`, that reads the model crates (`project`, `characters`, `worldbuilding`, `research`, `plotting`, `document`) and not the UI or the storage, so anything added later (assistance, an export) can call it with a project's data.
- The inspector shows it as **Context**: the entries' names, each opening a short summary of its text in place, so a writer starting a scene sees who and what is in it without leaving the page.
- It also writes the context as plain Markdown ("Copy context", to the clipboard), which is what an assistant would read. A context is capped at a length the function is given, dropping the entries' text from the end of the list first and saying so.

### 10. The mind map

The corkboard becomes a mind map of the manuscript.

- The book in the middle; its parts, chapters, and scenes as branches around it, read from the binder in binder order. With a series, the map shows the book chosen in the series menu, as the corkboard does.
- A node shows the title, the status, and the keyword colors as dots; a scene's synopsis and word count show when it is chosen. Choosing a node shows it in the inspector; opening a scene takes it to the page.
- A branch folds and unfolds. The map pans and zooms, and fits the window when it opens.
- What the corkboard did stays: the text filter and the keyword menu dim what does not match instead of hiding it, so the shape of the book stays in view.
- It replaces the corkboard: the corkboard pane and the `corkboard` crate go in the same change, card grouping included, and the "Mind map" row of stage 1 opens the map.
- Laid out automatically, left and right of the middle, with no positions saved yet. Drawn as SVG in the page, with no new crate; the layout is computed in Rust and tested there.
- The keyboard moves from node to node with the arrows, opens with Enter, and folds with Space. The outline stays the view a screen reader uses.

### 11. Editing the book from the map

- Dragging a scene onto another chapter moves it there in the binder, as dragging in the library does; dropped between two scenes, it goes between them. Parts and chapters move the same way.
- From a chosen node, Tab adds a child (a scene under a chapter, a chapter under a part or book) and Enter adds the next one beside it; F2 renames. Each is one undo step, and goes through the same binder functions as the library, so sync and the files see an ordinary binder change.
- Moving an item to the trash from the map asks first, as in the library.

### 12. Ideas, the story's entries, and lines

- **Ideas** are nodes that are not in the book: a title and a short note, attached to the book or to any node, or loose. They are never compiled and never counted.
- **The story's entries** show on the map beside what they are linked to, from stage 7's links, one kind at a time or all, from a menu on the map: a character linked to three scenes is one node with three lines. Dragging an entry onto a scene or chapter links it there; the lines are the links, so the map, the inspector, and the entry's page always agree.
- **Lines** join any two nodes, scenes and ideas alike, with an optional word on the line ("echoes", "pays off"). A line is not a binder move and not a story link.
- **Positions.** A node the writer drags keeps where it was put; the others stay laid out automatically around it. "Tidy" puts everything back.
- **An idea becomes a scene**: dropped on a chapter, or with a menu item, it becomes a new scene there, its note the synopsis, and its lines go with it.
- Stored in a new file, `mindmap.json`: `ideas`, `lines`, and `positions` by item id (entries included). Story links stay in `bible.json`. Sync merges `mindmap.json` by id, as `bible.json` and `plot.json` are, and keeps keys the program does not know. Lines to an item in the trash hide, and go when the trash is emptied. A project without the file has no ideas or lines. `documentation/PROJECT_FORMAT.md` describes it.

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

- Keywords on the story's entries themselves (a character tagged `antagonist`). Entries are linked, not tagged; tagging them is in [bonus.md](bonus.md).
- Any assistance, local or remote, and anything that sends text from the computer.
- `OR` and parentheses in the binder search.
- A mind map for anything but the manuscript and what is linked to it.

## Sources

- [Obsidian: Tags](https://obsidian.md/help/tags)
- [Obsidian: Tags view](https://obsidian.md/help/plugins/tags)
- [Obsidian: Search](https://obsidian.md/help/plugins/search)
- [Obsidian: Properties](https://obsidian.md/help/properties)
