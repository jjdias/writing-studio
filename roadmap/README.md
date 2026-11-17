# Writing Studio roadmap

Writing Studio is **open source** (MIT OR Apache-2.0) and **free for the community**. Anyone may use, study, modify, and share it. No account is required to write.

**Local-first, now and later.** Saves, snapshots, backups, and compile live on the writer's machine. Keeping a project on a second machine or with a hosting provider, planned through git, stays **optional**. It must never become a requirement to open a project, write, or keep copies of work.

## Where things are

- [`documentation/`](../documentation/) describes the program as it is today: how to use it, how it is built, and how a project is stored.
- This folder holds the plan. [`done/`](done/) records each phase that has shipped. [`todo/`](todo/) holds the work that is left, written so that someone new to the code can pick it up.

| Phase | What it is | State | File |
| --- | --- | --- | --- |
| 1 | Local foundation: projects, binder, editor, compile | Done | [done/phase-1.md](done/phase-1.md) |
| 2 | Writing environment | Done | [done/phase-2.md](done/phase-2.md) |
| 3 | Story development: series, plot, characters, world, research | Done | [done/phase-3.md](done/phase-3.md) |
| 4 | Professional editing: comparison, notes, track changes, spellcheck, diagnostics | Done | [done/phase-4.md](done/phase-4.md) |
| 5 | Plain-text projects, and sync through git | Done | [done/phase-5.md](done/phase-5.md) |
| 5.1 | What format 2 still needs to be safe to rely on | Items 1, 2, 5, and 7 done; 3 and 4 dropped; 6, 8, 9, and 10 to do | [todo/phase-5.1.md](todo/phase-5.1.md) |
| 6 | Publishing: export profiles, PDF, EPUB | To do | [todo/phase-6.md](todo/phase-6.md) |
| 7 | Cross-platform release | To do | [todo/phase-7.md](todo/phase-7.md) |
| 8 | Writing surface | To do | [todo/phase-8.md](todo/phase-8.md) |
| Bonus | Work with no phase yet | Not started | [todo/bonus.md](todo/bonus.md) |

Phases 1 to 5 are on `master`.

## How the work is done now

The project is young and has no users yet. Until that changes:

- **The focus is adding function, keeping the code short, and not taking on technical debt.** Prefer removing code to adding it. When a feature replaces something, remove what it replaced in the same change. Do not leave code that only a test uses.
- **The tests in this repository are the only testing there is.** There is no manual testing, no testing in the desktop window, and no other test suite. If behavior is not covered by a test here, it is not tested. Every change that adds or changes behavior adds or changes a test for it.
- **No performance testing for now.** Keep to the performance rules in `.claude/rules/`, and measure only when something is seen to be slow.
- **No compatibility with earlier versions.** Earlier test versions had no downloads. When a format changes, do not write a conversion; refuse the old format with a clear message.

### Known technical debt

None listed. The last three items were cleared at the end of Phase 5: a check used only by its own test, the unused `collaboration` and `security` crates, and the old format 1 document, folded into `documentation/PROJECT_FORMAT.md`.

## What to do next

1. The remaining items of Phase 5.1, each when the code it touches is next changed.
2. Then any of Phases 6, 7, and 8; they do not depend on each other.

## Product principles

1. **Open and free for the community.** The full application stays open source. Optional future services must not lock manuscripts away from people who never connect.
2. **Local storage first.** Projects are folders (`*.writingstudio`) of HTML and JSON files, with a SQLite cache for this machine. Backups are timestamped copies of that cache under `.local/backups/`.
3. **Never lose writing.** Files written whole and renamed into place, a crash-safe cache, and local backups come before sync.
4. **Shared Rust core.** UI, storage, compile, and later optional sync all consume the same document JSON and binder model.
5. **Offline writing is complete.** If a network, account, or server is unavailable, the studio still works.
6. **Easy to stay, easy to leave.** Every change should make writing a book here easier. None should make it harder to take the book somewhere else. A writer who stops using this program keeps files that another program, or a plain text editor, can read.



## Keep local writing unblocked

Do **not** require any of these to write or compile:

- User accounts or login
- A network connection
- Git, a git host, or any hosting provider
- A copy on a server as the only copy
- Simultaneous editing across machines
- Server-side search or storage as the source of truth

With or without sync, the source of truth is the `.writingstudio` folder on the writer's disk. A git host holds a copy of it.
