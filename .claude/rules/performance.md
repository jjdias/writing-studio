---
paths:
  - "crates/storage/**"
  - "crates/search/**"
  - "crates/export/**"
  - "crates/ui/**"
  - "crates/editor/**"
  - "crates/document/**"
---

# Performance

- Do not add a query, index rebuild, or compile pass on a path that already runs per keystroke or per autosave.
- Search is an in-memory index rebuilt when a project opens and updated on save. Do not persist a second index or rescan every document for a local edit.
- Load or compile the book, scene, or page the user is working with. Do not pull the whole library into memory for a single-scene operation.
- Keep SQLite work inside the existing transaction and backup patterns. Avoid repeated lookups of the same row in one operation.
- UI updates should follow the current render and state split. Do not introduce a new cache or background worker for a cost that has not been observed.
- When the task is itself a performance fix, measure or point at the hot call before changing the algorithm.
