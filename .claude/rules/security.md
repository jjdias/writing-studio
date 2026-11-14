---
paths:
  - "crates/security/**"
  - "crates/storage/**"
  - "crates/collaboration/**"
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
- Collaboration, synchronization, and security crates are boundaries. Do not stub them with a bypass that skips checks in storage or the document model.
- New crypto, credential storage, or sync authentication needs an explicit request. Do not invent a scheme inside an unrelated change.
