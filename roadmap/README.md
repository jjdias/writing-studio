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
| 5 | Plain-text projects, and sync through git | Stages 1 and 2 done; 3 to 5 to do | [done/phase-5.md](done/phase-5.md), [todo/phase-5.md](todo/phase-5.md) |
| 5.1 | What format 2 still needs to be safe to rely on | Items 1 and 2 done; items 3 to 10 to do | [todo/phase-5.1.md](todo/phase-5.1.md) |
| 6 | Publishing: export profiles, PDF, EPUB | To do | [todo/phase-6.md](todo/phase-6.md) |
| 7 | Cross-platform release | To do | [todo/phase-7.md](todo/phase-7.md) |
| 8 | Writing surface | To do | [todo/phase-8.md](todo/phase-8.md) |
| Bonus | Work with no phase yet | Not started | [todo/bonus.md](todo/bonus.md) |

Phases 1 to 4 are on `master`. Phase 5 is on the `alternative-phase-5` branch.

## What to do next

1. Phase 5.1, items 3 and 4 (items 1 and 2 are done). They are about not losing writing in a format 2 project, and they should come before more is built on it.
2. The rest of Phase 5, in order: the Markdown export of a whole book, then stages 3, 4, and 5.
3. The rest of Phase 5.1.

The other phases do not depend on Phase 5 and can be taken in any order after it.

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


## Scaffolded but not built

These crates exist so module boundaries stay stable. They are **types or stubs**, not product features.

| Crate | Current state | Intended later work |
| --- | --- | --- |
| `writing-studio-collaboration` | Yrs placeholder constant | None planned. Editing at the same time is bonus work; Phase 5 does not use this crate |
| `writing-studio-synchronization` | “optional = true” stub | Phase 5: merging two copies of a project, and sync through git |
| `writing-studio-security` | Notes that local folders are unencrypted | None planned. A project is as private as the folder and the git host it is kept on |
