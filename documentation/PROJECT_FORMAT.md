# Writing Studio project format

Version: **1**

A project is a directory you can copy, back up, and keep offline—not a single opaque file and not a cloud object.

```text
MyNovel.writingstudio/
  FORMAT                 # `writing-studio-project 1`
  project.sqlite         # metadata, binder, documents, snapshots, plot, characters, world, research
  backups/               # timestamped SQLite copies
  attachments/           # copies of research files, one folder per note
```

## Rules

1. Manuscript text lives in the `documents` table, one row per scene and per chapter that holds its own text. Research notes live in `research_items`, not as binder documents.
2. Binder hierarchy lives in `binder_nodes` and references `document_id`.
3. Never store the whole book as one text field.
4. Soft deletion uses `deleted_at` tombstones. Trash also sets `parent_id` to the trash root and remembers `restore_parent_id`.
5. Identifiers are UUIDs. Timestamps are RFC 3339 UTC.
6. `FORMAT` is the compatibility gate. Newer apps may read older versions; older apps must refuse newer ones.
7. The format does not depend on an account. Anyone with the folder can open it in this app.

## Tables (v1)

- `project_meta` — id, format_version, title, created_at, updated_at, `settings_json`
- `binder_nodes` — hierarchy, synopsis, status, `label`, `color`, `deleted_at`, `restore_parent_id`. `label` is a short tag. `color` is empty or one of `gold`, `green`, `red`, `blue`, `violet`
- `documents` — `content_json` (block model), word_count, char_count
- `snapshots` — per-document JSON copies with label (`manual`, `pre-restore`, …)
- `plotlines` — id, title, color, sort_index, created_at, updated_at
- `plot_points` — plotline_id, title, summary, optional `scene_id` (a binder scene), sort_index, timestamps
- `characters` — name, role, summary, sort_index, timestamps
- `character_motivations` — character_id, body, sort_index, timestamps
- `character_relationships` — from_id, to_id, kind, sort_index, timestamps
- `cultures` — name, summary, sort_index, timestamps
- `locations` — name, summary, optional `culture_id`, sort_index, timestamps
- `lore_entries` — title, body, optional `location_id` and `culture_id`, sort_index, timestamps
- `timeline_events` — title, `when_label`, summary, optional `location_id`, sort_index, timestamps
- `research_items` — title, notes, sort_index, timestamps
- `research_files` — id, item_id, display `file_name`, `stored_name` relative to `attachments/`, created_at
- `bible_scene_links` — owner_kind, owner_id, scene_id, sort_index. A character, location, culture, lore entry, timeline event, or research note can point at several manuscript scenes. The records stay in the project; the scene text is not copied.

Existing databases gain the plot, character, world, research, and `bible_scene_links` tables on open (`CREATE TABLE IF NOT EXISTS`). Databases created before `restore_parent_id` gain that column on open (`ALTER TABLE`). `label` and `color` are part of the current `binder_nodes` definition. Opening a project does not add those columns to a database created before they existed. Format version stays **1** because the added tables are additive.

A research file is stored as `attachments/{note-id}/{file-id}` plus a short extension. The sqlite backup does not include `attachments/`. Copy the project folder when you want the files and the database together.

## Settings JSON

```json
{
  "daily_word_goal": 500,
  "project_word_goal": 80000,
  "compile_title_page": true,
  "autosave_seconds": 2,
  "theme": "dark",
  "compile_book_id": null,
  "book_word_goals": [],
  "show_whole_series": true,
  "layout": {
    "sidebar_side": "left",
    "sidebar_width": 260,
    "inspector_width": 300,
    "sidebar_visible": true,
    "inspector_visible": true
  },
  "writing": {
    "day": "2026-11-06",
    "day_baseline": 1200,
    "session_started_at": "2026-11-06T18:00:00Z",
    "session_active_at": "2026-11-06T18:20:00Z",
    "session_baseline": 1200,
    "sessions": [],
    "books": []
  }
}
```

`writing`, `layout`, `theme`, `compile_book_id`, `book_word_goals`, and `show_whole_series` are optional for older projects. `show_whole_series` defaults to true. When it is false, the binder, corkboard, outliner, and search show `compile_book_id`. `theme` is `dark`, `light`, or `paper`. `sidebar_side` is `left` or `right`. Widths are pixels. On open, the binder width is clamped to 180–480 and the inspector width to 220–560. `day_baseline` remains the older whole-manuscript count. `writing.books` stores each book's outline count at the start of `day` and at the start of the open sitting:

```json
{ "book_id": "…", "day": "2026-11-06", "day_baseline": 1200, "session_baseline": 1200 }
```

Today and the sitting shown for a book use those baselines. Completed sittings that added words, summed across books, are appended to `sessions` (newest at the end) the next time the project is opened. The list keeps the newest 40. Each sitting is:

```json
{
  "day": "2026-11-06",
  "started_at": "2026-11-06T18:00:00Z",
  "ended_at": "2026-11-06T18:20:00Z",
  "words": 400,
  "seconds": 1200
}
```

A new local day sets each book's baseline to that book's current outline count. `compile_book_id` is the book Markdown and DOCX export, and the book whose word count and target are shown. Each `book_word_goals` entry is `{ "book_id": "…", "goal": 80000 }`. The first book with no entry keeps `project_word_goal`. Later books with no entry use 80000. Permanently deleting a book removes its entry. If that book was `compile_book_id`, the field becomes the first remaining book.

## Document JSON

Each document is a list of blocks. `schema_version` is currently 1. Block types are `paragraph`, `heading` (with `level`), `quote`, `list`, `code`, and `scene_break`. A block holds inline runs. Each run has marks: `bold`, `italic`, `underline`, `strikethrough`, an optional `link`, an optional `comment`, `comment_kind`, `comment_resolved`, and an optional `highlight`.

`comment` is the note from the Note control. `comment_kind` is `comment`, `question`, `revise`, or `check`. `comment_resolved` is true once the note is resolved. `highlight` is `yellow`, `green`, `blue`, or `pink`. The last three fields came with Phase 4. A document saved before them loads with a plain, open comment and no highlight, and `schema_version` is still 1. Track changes is not stored yet. See [PHASE4.md](PHASE4.md).

```json
{
  "id": "…",
  "schema_version": 1,
  "updated_at": "2026-11-06T18:20:00Z",
  "blocks": [
    {
      "id": "…",
      "kind": {
        "type": "heading",
        "level": 1,
        "inlines": [
          {
            "id": "…",
            "text": "Dawn",
            "marks": {
              "bold": false,
              "italic": false,
              "underline": false,
              "strikethrough": false,
              "link": null,
              "comment": "Check the tide"
            }
          }
        ]
      }
    },
    { "id": "…", "kind": { "type": "scene_break" } }
  ]
}
```

A `code` block uses the same inline list as a paragraph. A `list` block has `ordered` and `items`, and each item has its own inline list. The desktop editor may use a restricted HTML view; what is saved is this JSON, not the HTML.

## Compile

Markdown and DOCX compile one book in the **draft/manuscript** tree. The book's title is the front matter. Other books and Trash are omitted. Research notes are not binder items in that tree. Output is a normal file on disk chosen by the writer. A project with no chosen book compiles the first book.
