# Writing Studio project format

Version: **1**

A project is a directory, not a single opaque file:

```text
MyNovel.writingstudio/
  FORMAT                 # `writing-studio-project 1`
  project.sqlite         # metadata, binder, document JSON, snapshots
  backups/               # timestamped SQLite copies
  attachments/           # future binary research files
```

## Rules

1. Manuscript text lives in the `documents` table, one row per scene/note.
2. Binder hierarchy lives in `binder_nodes` and references `document_id`.
3. Never store the whole book as one text field.
4. Soft deletion uses `deleted_at` tombstones.
5. Identifiers are UUIDs. Timestamps are RFC 3339 UTC.
6. `FORMAT` is the compatibility gate. Newer apps may read older versions; older apps must refuse newer ones.

## Tables (v1)

- `project_meta`
- `binder_nodes`
- `documents`
- `snapshots`
