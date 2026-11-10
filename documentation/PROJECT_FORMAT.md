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

1. Manuscript text lives in the `documents` table, one row per scene/note.
2. Binder hierarchy lives in `binder_nodes` and references `document_id`.
3. Never store the whole book as one text field.
4. Soft deletion uses `deleted_at` tombstones. Trash also sets `parent_id` to the trash root and remembers `restore_parent_id`.
5. Identifiers are UUIDs. Timestamps are RFC 3339 UTC.
6. `FORMAT` is the compatibility gate. Newer apps may read older versions; older apps must refuse newer ones.
7. The format does not depend on an account. Anyone with the folder can open it in this app.

## Tables (v1)

- `project_meta` — id, format_version, title, created_at, updated_at, `settings_json`
- `binder_nodes` — hierarchy, synopsis, status, `deleted_at`, `restore_parent_id`
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
- `research_files` — item_id, display `file_name`, `stored_name` relative to `attachments/`
- `bible_scene_links` — owner_kind, owner_id, scene_id, sort_index. A character, location, culture, lore entry, timeline event, or research note can point at several manuscript scenes. The records stay in the project; the scene text is not copied.

Existing databases gain the plot, character, world, research, and `bible_scene_links` tables on open (`CREATE TABLE IF NOT EXISTS`). Databases created before `restore_parent_id` gain that column on open (`ALTER TABLE`). Format version stays **1** because these tables are additive.

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
    "sessions": []
  }
}
```

`writing`, `layout`, `theme`, `compile_book_id`, `book_word_goals`, and `show_whole_series` are optional for older projects. `show_whole_series` defaults to true. When it is false, the binder, corkboard, outliner, and search show `compile_book_id`. `theme` is `dark`, `light`, or `paper`. `sidebar_side` is `left` or `right`. Widths are pixels and are clamped when the project opens. `day_baseline` is the manuscript word count at the start of that local day. The open sitting records its start count and last activity. Completed sittings that added words are appended to `sessions` (newest at the end, capped) the next time the project is opened. A new local day sets `day_baseline` to the current count. `compile_book_id` is the book Markdown and DOCX export, and the book whose word count and target are shown. Each `book_word_goals` entry is a book id and a target. The first book with no entry keeps `project_word_goal`. Later books with no entry use 80000.

## Document JSON

Each document is a list of blocks (paragraph, heading, quote, list) with inline runs and marks (bold, italic, underline, strikethrough, optional link). Schema version is stored on the document (`schema_version`, currently 1). The desktop editor may use a restricted HTML view; what is saved is this JSON, not the HTML.

## Compile

Markdown and DOCX compile one book in the **draft/manuscript** tree. The book's title is the front matter. Other books, Research, and Trash are omitted. Output is a normal file on disk chosen by the writer. A project with no chosen book compiles the first book.
