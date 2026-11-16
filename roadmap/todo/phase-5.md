# Phase 5 — what is left

Phase 5 is **plain-text projects, and sync through git**. All five stages have shipped: see [../done/phase-5.md](../done/phase-5.md). This file is what is left: History from git.

Read this whole section before starting any item.

## Before you start

**Read first**

- [`AGENTS.md`](../../AGENTS.md) and the rules in `.claude/rules/`. They limit scope, tests, and git use.
- [`documentation/PROJECT_FORMAT_2.md`](../../documentation/PROJECT_FORMAT_2.md): the format, and what is written and deleted when.
- [`documentation/ARCHITECTURE.md`](../../documentation/ARCHITECTURE.md): the crates and the data flow.
- "How the work is done now" in [../README.md](../README.md): the current focus, and what counts as tested.
- [phase-5.1.md](phase-5.1.md): small items to do when the code they touch is next changed.

**Where the code is**

| File | What it does |
| --- | --- |
| `crates/format/src/html.rs` | Blocks to the stored HTML and back, exactly. `blocks_to_html`, `html_to_blocks`, `stored_blocks`. Also the tokenizer used for a scene file's head. |
| `crates/format/src/scene.rs` | `SceneFile` and `SceneHeader`: `to_html` / `parse_html` for the stored file, `to_markdown` / `parse_markdown` for export and import. `file_name`. |
| `crates/format/src/write.rs`, `read.rs`, `canonical.rs`, `header.rs` | Markdown: the one writer (`write_markdown`, with notes as CriticMarkup or as footnotes for compile), the reader, what Markdown cannot hold (`canonical_blocks`), and front matter. |
| `crates/storage/src/markdown.rs` | The whole project exported as a folder of Markdown files, and Markdown files brought in as scenes. |
| `crates/storage/src/files.rs` | The whole folder: `ProjectContent`, `write_project_files`, `read_project_files`, `verify_project_files`, `write_atomic`, and one function for each file (`scene_text`, `binder_json`, `bible_files`, `plot_json`, `settings_json`). `inside` checks that a path stays in the project. |
| `crates/storage/src/mirror.rs` | `ProjectStore::create`, `open` (which refuses a format 1 folder), and every save: each public save writes the file and then calls the `_cached` function in `lib.rs` that writes SQLite. `Mirror` remembers which file each item has. |
| `crates/storage/src/lib.rs`, `recovery.rs`, `schema.rs` | The SQLite cache: the `_cached` writes, the loads, snapshots, backups. |
| `crates/ui/src/state.rs` | `StudioState`: everything the window does goes through here, and it is where the tests for behavior live. |
| `crates/ui/src/studio.rs` | The window. `pick_folder` and the save dialog near it show how a folder or file is chosen. |

**Rules the code keeps, and you must keep**

1. A save writes the file first and the cache second. The cache never holds writing that is not in a file.
2. Every write and delete of a project file goes through `write_file` or `remove_file` in `mirror.rs`, which set aside a file that changed outside. Do not write project files around them.
3. Every file is written with `write_atomic`.
4. Every path that comes from a project file goes through `inside` before it is used.
5. A file is deleted only when the writer deleted the thing it holds: emptying the trash, or deleting a bible entry.
6. A scene's text has the id of its binder item.
7. A save writes only the files that changed. Do not read or write every scene on a save: saves run on a two-second idle timer.
8. Nothing in a scene file is ever run.

**How to test**

- `cargo test -p writing-studio-format`, `cargo test -p writing-studio-storage`, `cargo test -p writing-studio-ui`. Run the crate you changed, not the workspace.
- The UI crate needs GTK and WebKit development packages to compile. On Debian or Ubuntu: `libgtk-3-dev libwebkit2gtk-4.1-dev libxdo-dev libsoup-3.0-dev libjavascriptcoregtk-4.1-dev`.
- State tests in `crates/ui/src/state.rs` build a real project in a temporary folder with `open_state`.
- Nothing drives the desktop window. Say so in your summary when a change is only tested below the window.

## Later in this phase

- History from git in the History view: open or compare a scene as it was at an earlier sync.

Moved out of this phase:

- A git engine built into the program, so that nothing has to be installed and Sync can work on phones and tablets: to [Phase 7](phase-7.md), where phones and tablets are. It means a large new dependency, and the program handling SSH keys or tokens itself, which needs an explicit decision (`.claude/rules/security.md`).
- Guided setup, creating a repository and signing in from inside the program: to [bonus](bonus.md). It needs an integration with each git host, against "any git host", and stored tokens.

## Not in this phase

- An account, a server run by this project, or shared-project permissions. Who can read or write a project is decided by the git host.
- Two people editing at the same time. This is bonus work.
- Encrypting a project. A private repository is as private as its host makes it.
