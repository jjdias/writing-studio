# Phase 5.1 — What format 2 still needs to be safe to rely on

Not started. These are gaps found while building Phase 5 stages 1 and 2. None of them is in the Phase 5 plan, and each one matters before writers keep real books in format 2.

Items 1 to 4 are about not losing writing. Do them first, in order, before the rest of Phase 5. Items 1 and 2 are done. Items 5 to 10 can follow in any order.

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

**The problem.** Opening a format 1 project converts it at once. The conversion is one-way: a version of the program from before format 2 will not open the project afterwards. A writer who shares a project with someone on an older version, or who wants to try the new version first, has no choice.

**How to fix it.**

- `ProjectStore::needs_conversion` already says whether opening would convert. In `open_project` (`crates/ui/src/studio.rs`), call it first. If it is true, ask, with three choices:
  - **Convert this project.** As now.
  - **Convert a copy.** Choose a new folder, write the converted copy there (`to_format_2` in `crates/storage/examples/` does exactly this; move its code into the storage crate), and open the copy. The original is not touched.
  - **Cancel.**
- Say in the question that older versions cannot open a converted project, and that a backup of the database is written first.
- A confirmation is two clicks elsewhere in the program (backup restore, empty trash). Follow that pattern rather than adding a dialog library.

**Tests.** In `state.rs`: open a format 1 project with "copy" and check that the original folder is unchanged and the copy is format 2. With "cancel", nothing is written.

## 4. Convert a real book before relying on the conversion

**The problem.** Conversion has been tested on projects the tests build. No book written in the program has been converted. A project that has been used for months has things the tests do not make: research files, every kind of block, trashed books, very long scenes, titles in many scripts.

**How to do it.**

- With a copy of real projects (ask the people who own them; never commit them), run `to_format_2` and `check_project`. Both only read the project.
- Open the converted copy in the window and look at the binder, a few scenes, the bible, the plot grid, and History.
- Measure how long opening takes. Opening reads every scene file to fill the cache. A book of 300 scenes should open in well under a second; if it does not, the next step is to skip scenes whose file has not changed since the cache was filled (keep each file's fingerprint in the cache), not to stop reading the files.
- Write down what was found in [../done/phase-5.md](../done/phase-5.md).

**Done when** at least one real book of more than 50 scenes converts, opens, and shows the same text, notes, and bible as before.

## 5. Write `.gitattributes` with every project

**The problem.** Git on Windows can be set to change line endings to `\r\n` when it checks a file out. The program reads such files, but it writes `\n`, so every file it saves would show as changed in git. Research files under `attachments/` are images and PDFs, which git must not touch.

**How to fix it.** Write `.gitattributes` with the other files a new project gets (`README.md`, `scene.css`, `.gitignore`, in `write_project_files`), and only when it is missing:

```text
* text=auto eol=lf
attachments/** -text
```

**Tests.** A new project has the file. A project with its own `.gitattributes` keeps it.

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

**How to fix it.** In History, list the scenes of a backup and let one be restored. Open the backup read-only with `rusqlite`, read that scene's row from `documents`, and save it with `save_document`, after a snapshot of the current text, as `restore_snapshot` in `recovery.rs` does. A backup made before conversion has its own ids for texts: look the scene up by its binder item.

## 9. Check that the editor keeps line breaks

**The problem.** Noticed while writing the stored format and not confirmed: the HTML the editor is given (`to_html` in `crates/document/src/html.rs`) writes a line break inside a paragraph as a plain line break in the text, not as `<br>`. A browser shows that as a space. If so, a line break typed in a paragraph is kept in the file but looks like a space when the scene is opened again in the window. This is older than Phase 5, and the stored format is not affected.

**How to do it.** In the window, type two lines in one paragraph with Shift+Enter, change scene, and come back. If the break shows as a space, write `<br>` for `\n` in `inline_to_html` in `crates/document/src/html.rs`; the reader there already turns `<br>` back into a line break. Add a test next to `empty_browser_paragraph_stays_empty`.

## 10. Update the guides for format 2

- `documentation/USAGE.md` still says little about the folder. Add a section on what the folder holds, that it can be read without the program, that `.local/` is this computer's, and what happens when an older project is opened.
- Describe the two commands writers can run on a project (`to_format_2` and `check_project`) or remove them from the guides once item 3 has put conversion of a copy in the window.
