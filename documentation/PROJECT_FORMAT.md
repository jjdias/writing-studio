# Writing Studio project format

Version: **1**

A project is a directory you can copy, back up, and keep offline—not a single opaque file and not a cloud object.

```text
MyNovel.writingstudio/
  FORMAT                 # `writing-studio-project 1`
  project.sqlite         # metadata, binder, document JSON, snapshots, plot grid
  backups/               # timestamped SQLite copies
  attachments/           # reserved for research files (unused in Phase 1)
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

Existing databases gain `plotlines` and `plot_points` on open (`CREATE TABLE IF NOT EXISTS`). Databases created before `restore_parent_id` gain that column on open (`ALTER TABLE`). Format version stays **1** because these tables are additive.

## Settings JSON

```json
{
  "daily_word_goal": 500,
  "project_word_goal": 80000,
  "compile_title_page": true,
  "autosave_seconds": 2,
  "theme": "dark",
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

`writing`, `layout`, and `theme` are optional for older projects. `theme` is `dark`, `light`, or `paper`. `sidebar_side` is `left` or `right`. Widths are pixels and are clamped when the project opens. `day_baseline` is the manuscript word count at the start of that local day. The open sitting records its start count and last activity. Completed sittings that added words are appended to `sessions` (newest at the end, capped) the next time the project is opened. A new local day sets `day_baseline` to the current count.

## Document JSON

Each document is a list of blocks (paragraph, heading, quote, list) with inline runs and marks (bold, italic, underline, strikethrough, optional link). Schema version is stored on the document (`schema_version`, currently 1). The desktop editor may use a restricted HTML view; what is saved is this JSON, not the HTML.

## Compile

Markdown and DOCX compile walk the **draft/manuscript** tree only. Research and Trash are omitted. Output is a normal file on disk chosen by the writer.
