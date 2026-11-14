---
paths:
  - "**/.gitignore"
  - "**/.gitattributes"
---

# Git workflow

Always-on commit and safety limits are in `AGENTS.md`. This file covers repository files and history hygiene.

- Inspect `git status` before editing and the final diff before finishing. Drop scratch files and accidental edits you introduced.
- Leave unrelated user changes unstaged and untouched.
- One task belongs in one focused diff. Do not mix a format migration, a dependency bump, and a feature.
- Do not amend, rebase, or push unless that was requested and the commit is still local and unpushed.
- Do not commit `.env`, credentials, `target/`, or local database journals. `.gitignore` already excludes those build and database sidecars.
