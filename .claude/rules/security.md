---
paths:
  - "crates/storage/**"
  - "crates/synchronization/**"
  - "**/.env"
  - "**/.env.*"
---

# Security

Always-on secret and safety limits are in `AGENTS.md`.

- This studio is local-first. Offline writing must stay complete if sync or an account service is absent. Do not add a network or account gate to a path that currently works offline.
- Validate untrusted input at the boundary that accepts it: file paths, imported project data, HTML interchange, and SQL parameters.
- Keep database access parameterized. Keep filesystem access inside the project folder the user opened.
- Do not log document text, paths that identify a user's files, or anything from `.env`.
- The synchronization crate is a boundary: what it brings in through git is another machine's files, as untrusted as any imported project. Do not bypass the checks in storage or the document model for it.
- New crypto, credential storage, or sync authentication needs an explicit request. Do not invent a scheme inside an unrelated change.
