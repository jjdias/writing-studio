# Phase 5 — what is left

Phase 5 is **plain-text projects, and sync through git**. What has shipped is in [../done/phase-5.md](../done/phase-5.md). This file is the rest, in the order to do it.

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
| `crates/format/src/write.rs`, `read.rs`, `canonical.rs`, `header.rs` | The Markdown export and import, what Markdown cannot hold (`canonical_blocks`), and front matter. |
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

## Stage 2, the one item left — Export a whole book as Markdown, and bring Markdown in

One scene already converts both ways (`SceneFile::to_markdown`, `SceneFile::parse_markdown`), and what Markdown cannot hold is listed in the format. What is missing is a command.

**Export**

- Add to `crates/storage`: `export_markdown(store: &ProjectStore, target: &Path) -> Result<usize>`, returning how many files were written. Build a `ProjectContent` with `ProjectContent::from_store` and write from that.
- The target is a new folder. Refuse a folder that exists and is not empty.
- Write the manuscript so that it reads in order without this program: one file for each scene, named with its place in the book, such as `manuscript/01-chapter-one/003-the-harbor.md`. Use `in_tree_order` in `files.rs` for the order. Leave out the trash.
- Write the bible the same way, one file for each entry, in the folders the project uses. `entry_file` in `files.rs` builds the stored file for an entry; split it so the same `SceneFile` can be written with `to_markdown`.
- Write a `README.md` at the top that lists the books, chapters, and scenes in order, each as a link to its file.
- Do not write `.local/`, the ids of plot points, or settings. This is for reading and for other programs, not for coming back.
- In the window, put the command beside Markdown and DOCX compile. `prepare_export` and `CompiledFormat` in `state.rs` show how compile is wired; `pick_folder` in `studio.rs` chooses the folder.

**Import**

- Bring one or more `.md` files in as new scenes, in the chapter that is selected, in the order they were chosen.
- `SceneFile::parse_markdown` gives the header and the blocks. The title is the header's `title`, or the file's name when there is none.
- Always give the scene a new id. An `id` in the file is ignored, so that importing a file twice, or a file from another project, cannot replace a scene.
- Make the item with `BinderNode::new`, save it with `ProjectStore::upsert_node`, then `save_document`, as `add_node` in `state.rs` does.
- A header that cannot be read is an error that names the file and the line (`HeaderError`). The other files are still brought in.

**Tests**

- Export a project with two books, a trashed scene, and a bible, then check the folder: the order, that the trash is not there, and that a scene's text is what `to_markdown` gives.
- Import the exported scenes into an empty chapter and compare each with `canonical_blocks` of the original.
- Import a file with no header, and a file with a broken header.

**Done when** a book can be exported to a folder a person can read top to bottom, the files open in another Markdown tool, and Markdown files can be brought in as scenes. Then mark stage 2 complete in [../done/phase-5.md](../done/phase-5.md) and add the commands to `documentation/USAGE.md`.

## Stage 3 — Files changed outside the program

- A writer can edit a scene in any text editor. The program reads the change the next time the project opens, and while it is open.
- Text the program does not understand is kept as written, never dropped.
- A file that cannot be read is reported by name, and the rest of the project still opens.
- After any outside change the binder is checked: a file the binder does not list is added to a Recovered folder, and a binder entry whose file is gone is shown as missing, not deleted.

### What is already true

Opening a project fills the cache from the files, so a scene edited in a text editor, or changed by git, is what opens. The reader keeps the text of tags it does not know.

### What is not, and how to do it

**a. One bad file must not stop the project from opening.** Today `read_project_files` returns the first error and the project does not open.

- Give `ProjectContent` a list of problems, each a path and a message, and return the content with the list.
- A scene file that is missing or cannot be read: keep its binder item, mark it as missing, and show it in the binder with a mark. Do not make an empty text for it, and never write an empty file over it. `Mirror` needs a set of missing items, and `save_document`, `write_changed_scenes`, and `fill_cache` must skip them.
- A JSON file that cannot be read is different. Without `binder.json` or `project.json` there is no project: keep that an error. `bible.json` and `plot.json` can be reported and skipped, but then they must not be written either until the writer has seen the problem, or the save would replace them with empty ones.
- In the window, list the problems after opening. The status line holds one message; this needs its own place, such as a section in History.

**b. Recovered.** A `.html` file in `manuscript/` or a bible folder that no list mentions is not part of the project today.

- After reading, list the files in those folders and compare with `content.files`.
- For each one not listed, read it and add it: a scene goes into a folder named Recovered at the top of the manuscript, made when first needed; a bible entry goes into its own kind.
- If the file has no `id`, or an id that is already in use, give it a new one and write the file with it.
- Skip temporary files (`.name.id.tmp`, which `write_atomic` leaves only if it was interrupted) and remove those older than a day.
- Skip `*.conflict-*` files; see c.

**c. Changes while the project is open.** Done in Phase 5.1 item 2: a file changed outside is set aside as `name.conflict-<time>.ext` rather than written over, and the open scene is read again when the window comes back to the front. For item b below, do not take `.conflict-` files in as new scenes without asking: list them for the writer, who decides.

**d. Check the binder after any outside change.** Write `check_binder(nodes) -> (nodes, problems)` and run it in `read_project_files`:

- An id that appears twice: keep the first.
- A parent that is not in the file: move the item to the top level of its section.
- A loop of parents: move one item of the loop into Recovered.
- More or fewer than one of each root (`draft_root`, `research_root`, `trash_root`): this is not a project the program can open; say which.

**e. Names the program does not know.** `SceneHeader.other` keeps them, but the store drops them on the next save. This is item 1 of [phase-5.1.md](phase-5.1.md).

**Tests**, in `mirror.rs`, each on a project made by `ProjectStore::create` and then edited with `fs::write`: a scene file deleted; a scene file with a broken head; a file added to `manuscript/` by hand, with and without an id; a duplicated line in `binder.json`; a parent that does not exist; a loop.

**Done when** each of those opens the project, with everything else intact and the problem named.

## Stage 4 — Merge two copies of a project

One person usually writes a book, sometimes on two machines. Two people in the same scene at the same moment is not what this is for.

- The program merges two copies of a project itself, from the last version they shared. Git is not left to write conflict markers into a file.
- Scenes merge by paragraph. Paragraphs changed on only one side are taken. A paragraph changed on both sides is a conflict.
- A conflict is shown with the Phase 4 review tools: both versions, and a choice of one, the other, or both. Until the writer chooses, both are kept.
- Binder rules: an item moved on one side and edited on the other keeps both changes. An item deleted on one side and edited on the other is kept. A merge that would leave a loop or an item with no parent puts that item in Recovered.
- Attachments are whole files: the newer one is used and the other is kept beside it.
- No merge discards text without showing it.

### How to build it

**Where.** The `writing-studio-synchronization` crate is a placeholder kept for this. It may depend on `writing-studio-storage`, `writing-studio-format`, and `writing-studio-document`. Do not put merge code in the storage crate.

**The function.**

```rust
pub fn merge(base: &ProjectContent, ours: &ProjectContent, theirs: &ProjectContent) -> Merged

pub struct Merged {
    pub content: ProjectContent,
    pub conflicts: Vec<Conflict>,   // what the writer has to look at
}
```

It works on three `ProjectContent` values and touches no files. `read_project_files` makes one from a folder, so stage 5 can read three checkouts and call this.

**Scenes.** Match scenes by id. For a scene changed on both sides, merge by paragraph:

- `compare_units` in `crates/document/src/diff.rs` compares two lists of paragraphs and says where each one sits on both sides. It is `pub(crate)`; make what you need public rather than copying it.
- Compare base with ours, and base with theirs. Walk the base paragraphs in order. A stretch changed on one side only takes that side. A stretch changed the same way on both takes it once. A stretch changed differently on both is a conflict.
- Compare paragraphs with `stored_blocks`, so that formatting counts, not only words.

**A conflict keeps both versions, and uses the track-changes panel to settle it.** Put our paragraphs in the merged scene, followed by theirs. Then set the scene's track-changes baseline to the scene with only our paragraphs (`ProjectStore::save_tracking_baseline`). The panel from Phase 4 then lists their paragraphs as added, and the writer accepts or rejects each one with the buttons that exist. The baseline is in `.local/`, which is right: settling a conflict is this machine's business until it is synced. Add a line to the panel that says the changes came from a merge.

**The head of a scene** (title, synopsis, status, label, color, goal, conflict, outcome): merge each field on its own. One side changed it: take that. Both changed it differently: take ours and report theirs in `conflicts`.

**The binder.** Match items by id.

- Added on one side: add it, after the item it followed on that side.
- Moved on one side and edited on the other: both.
- Moved on both sides to different places: take ours, report it.
- Deleted on one side (gone from `binder.json`, or `deleted` set) and edited on the other: keep it.
- Then run `check_binder` from stage 3. Anything it has to move goes to Recovered.

**Lists in `bible.json` and `plot.json`:** by id, with the same rules. **`project.json`:** each setting on its own; the spelling words are the union of both lists.

**Attachments** are whole files. If both sides changed the same one, keep ours under its name and theirs beside it with `-theirs` before the extension, and report it.

**Tests.** Each rule above, as three small `ProjectContent` values and an expected result. Then properties: merging a side with itself gives that side; merging when only one side changed gives that side; and no paragraph of either side is missing from the result or its conflicts.

**Done when** two copies of a project that were edited apart can be merged with no text lost, and a writer can settle every conflict in the window.

## Stage 5 — Sync through git

- **Sync** saves the project as a commit, fetches, merges with stage 4, and pushes. One button, and a line that says when the project was last synced and whether there is anything to send or receive.
- The writer does not see branches, staging, or rebase. Commit messages are written by the program from what changed.
- Autosave does not commit. Sync, closing the project, and a pause in writing do.
- The first version uses the `git` program installed on the computer, and the keys or tokens the writer has already set up for it. The program stores no passwords.
- A guide in `documentation/` explains how to create a private repository with a common host and connect a project to it.
- Connecting a project warns that a public repository makes the manuscript public.
- A new project gets a `.gitignore` for the cache, the local settings, snapshots, and backups, and a short `README` in the folder that says what the files are and how to read them without this program.
- Large research files are stored as they are. The guide explains Git LFS for writers who need it.
- If git is not installed, or there is no network, everything except Sync works, and Sync says why it cannot run.

### How to build it

**Where.** In `writing-studio-synchronization`, on top of stage 4. Run the `git` program with `std::process::Command`. Do not add a git library in this stage; that is under "Later" below.

**Never store a password, a token, or a key.** The security rule in `.claude/rules/security.md` requires an explicit request for any credential handling. This stage has none: `git` uses what the writer set up for it.

**Running git**

- Set `GIT_TERMINAL_PROMPT=0` so that git fails instead of waiting for a password the window cannot show.
- Give every network command a time limit, and run it off the window's thread.
- Run every command with the project folder as its working directory, and check that the folder is the top of its repository (`git rev-parse --show-toplevel`), so that Sync never commits files outside the project.
- Read the exit status and the error text. Show the writer a short sentence, and keep git's own words where they can be read.

**Connecting a project**

1. If the folder is not a repository: `git init`, and write `.gitattributes` (item 5 of [phase-5.1.md](phase-5.1.md)).
2. Ask for the address of the remote, and add it as `origin`.
3. Before the first push, warn that anyone who can read the repository can read the manuscript, and that a public repository makes it public. The program cannot tell whether a repository is private, so the warning is shown every time a project is connected.

**Sync**, in this order:

1. Save everything (`StudioState::save_all`).
2. Stop if git is in the middle of something: a `MERGE_HEAD`, `rebase-merge`, or `rebase-apply` in `.git/`. Tell the writer to finish it with git.
3. `git add -A`, then commit if anything is staged. `.gitignore` already leaves `.local/` out. Write the message from what changed: scenes added, changed, and removed, by title.
4. `git fetch origin`.
5. Compare with the remote branch:
   - The remote has nothing new: go to 6.
   - We have nothing new: `git merge --ff-only`, then open the project again so the cache is filled from the new files.
   - Both have new commits: read three copies of the project (the merge base, ours, theirs) into temporary folders with `git worktree add --detach`, read each with `read_project_files`, and call `merge` from stage 4. Record the merge without letting git touch the files (`git merge -s ours --no-commit`), write the merged project with `write_project_files`, `git add -A`, and commit. Remove the temporary worktrees. Then open the project again.
6. `git push`. If it is refused because the remote moved, start again from 4, once.

**In the window**

- A Sync button, and a line that says when the project was last synced and whether there is anything to send or receive (`git status --porcelain --branch` after a fetch).
- Autosave does not commit. Sync does, and so does closing the project. A commit after a pause in writing is optional; if it is added, make the pause long (minutes) so the history is not one commit a sentence.
- If git is not installed, or there is no network, everything else works and Sync says which it is.

**Documents to write**

- `documentation/SYNC.md`: how to make a private repository with a common host, set up a key or token for `git`, and connect a project. How to use Git LFS for large research files.
- A section in `documentation/USAGE.md`.

**Tests**

- Use a bare repository in a temporary folder as the remote; no network is needed. Two working copies of one project stand in for two machines.
- Sync with nothing to do; sync that only sends; sync that only receives; sync where both changed different scenes; sync where both changed the same paragraph, which must end with both versions in the scene and a baseline set.
- A folder that is not a repository; a repository with a merge in progress; `git` not found (run with an empty `PATH`).

**Done when** a writer can keep one project on two machines with the Sync button alone, and no sync loses a word.

## Later in this phase

- A git engine built into the program, so that nothing has to be installed and Sync can work on phones and tablets.
- Guided setup: create or pick a repository and sign in from inside the program.
- History from git in the History view: open or compare a scene as it was at an earlier sync.

## Not in this phase

- An account, a server run by this project, or shared-project permissions. Who can read or write a project is decided by the git host.
- Two people editing at the same time. This is bonus work.
- Encrypting a project. A private repository is as private as its host makes it.
