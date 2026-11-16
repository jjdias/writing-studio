# Phase 5.1 — What format 2 still needs to be safe to rely on

Not started. These are gaps found while building Phase 5 stages 1 and 2. None of them is in the Phase 5 plan, and each one matters before writers keep real books in format 2.

Items 1, 2, and 5 are done. Items 3 and 4 were dropped. Items 6 to 10 are small; do each when the code it touches is next changed.

Read "Before you start" in [phase-5.md](phase-5.md) first: it lists the files, the rules the code keeps, and how to test.

## 1. Keep what the program does not understand in a file

**Done.** See [../done/phase-5.md](../done/phase-5.md), "Phase 5.1". What is left of it:

- Unknown keys in `project.json`, and unknown settings inside it, are still dropped on save.
- Unknown keys on a line of `relationships` or `scene_links` in `bible.json` are still dropped. Those lines have no id of their own to keep them by; give them one if this is needed.

## 2. Do not write over a file that was changed outside while the project is open

**Done.** See [../done/phase-5.md](../done/phase-5.md), "Phase 5.1". What is left of it:

- Set-aside files (`name.conflict-<time>.html`) are only named in the status line. A list of them in History, with a way to open or compare one, would make them easier to deal with. Stage 4's way of settling a conflict can replace this.
- Only the open scene is read again when the window comes back to the front. A bible entry or the binder changed outside is caught when it is next saved (it is set aside, not lost), but not shown until the project is opened again.

## 3. Ask before converting a format 1 project

**Dropped.** Conversion of format 1 projects was removed: format 1 had no downloads, so there are no books to convert. A format 1 folder is refused with a message instead. See [../done/phase-5.md](../done/phase-5.md), "Phase 5.1".

## 4. Try a book of real size in the window

**Dropped.** There is no performance testing and no testing in the window at this point; the tests in this repository are the only testing. See "How the work is done now" in [../README.md](../README.md).

## 5. Write `.gitattributes` with every project

**Done** with Phase 5 stage 5. Every project gets `* text=auto eol=lf` and `attachments/** -text` when the file is missing.

## 6. File names that are the same on every computer

**The problem.** `file_name` keeps letters from any script, so a scene called "Été" gets `été-3f9a.html`. macOS can store such a name in a different Unicode form from Windows and Linux, and git can then see two files where there is one. Very long paths are also a problem on Windows.

**How to fix it.** In `file_name` (`crates/format/src/scene.rs`), turn letters with accents into the plain letter where there is one (é → e), and replace any other character that is not an ASCII letter or digit with `-`. A title in a script with no plain letters (Japanese, Arabic) then gives only the id, which is fine: the title is in the file. Keep names at 40 characters. Files already named keep their names; this only changes new files.

**Tests.** Titles in French, German, Greek, Japanese, and with emoji give ASCII names, and two different titles never give the same name.

## 7. Refuse a path that leaves the project through a link

**The problem.** `inside` in `files.rs` refuses `..` and absolute paths. It does not refuse a folder in the project that is a symbolic link to somewhere else, so a project from someone else could make the program read or write outside its folder.

**How to fix it.** Before reading or writing a path from a project file, resolve it (`fs::canonicalize` on the parent folder) and check that it is still under the project folder, also resolved. For a file that does not exist yet, check its parent.

**Tests.** On Unix: a project whose `manuscript` folder is a link to a folder outside; opening it refuses with a message, and nothing outside is written.

## 8. Restore one scene from a backup

**The problem.** In format 2 a backup is a copy of the cache, and restoring it replaces the whole project, files and all. A writer who wants one scene back has to restore everything.

**How to fix it.** In History, list the scenes of a backup and let one be restored. Open the backup read-only with `rusqlite`, read that scene's row from `documents`, and save it with `save_document`, after a snapshot of the current text, as `restore_snapshot` in `recovery.rs` does.

## 9. Show line breaks typed in a paragraph

**The problem.** The HTML the editor is given (`to_html` in `crates/document/src/html.rs`) writes a line break inside a paragraph as a plain line break in the text, not as `<br>`. The editor's styles do not keep line breaks in text, so a browser shows it as a space. A line break typed in a paragraph is kept in the file, but looks like a space when the scene is opened again in the window. This is older than Phase 5; the stored format is not affected.

**How to fix it.** Write `<br>` for `\n` in `inline_to_html` in `crates/document/src/html.rs`; the reader there already turns `<br>` back into a line break. Add a test next to `empty_browser_paragraph_stays_empty` that a paragraph with a line break gives `<br>` and reads back the same.

## 10. Update the guides for format 2

- `documentation/USAGE.md` still says little about the folder. Add a section on what the folder holds, that it can be read without the program, that `.local/` is this computer's, and what happens when an older project is opened.
- Decide whether the guides should mention `check_project`, which checks that every scene of a project reads back from its file.
