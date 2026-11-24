# Writing Studio

A free, open-source writing app for novels and series. Plan the book, write it scene by scene, and compile a draft, all on your own computer. No account is needed, and nothing leaves your machine unless you choose to sync.

## What you can do

- **Organize the book.** Books, parts, chapters, and scenes in a library you rearrange by dragging. See the scenes as a list, as a mind map with the characters, places, and notes linked to them, or as an outline.
- **Write.** A quiet page with your choice of font, line length, and spacing, focus mode, and the bars fading while you type.
- **Keep track.** Daily and book word targets, a deadline with the pace that meets it, keywords and saved filters, and a status for each scene.
- **Develop the story.** Characters, world notes, a plot grid, and research, shared by every book in a series.
- **Revise.** Notes, highlights, tracked changes, snapshots to compare, spellcheck, and checks on each scene's goal, conflict, and outcome.
- **Never lose work.** Autosave, snapshots, and backups. Each scene is a plain Markdown file that any Markdown or text editor can open.
- **Compile.** Export a book as Word (DOCX), an ebook (EPUB), a web page (HTML), or Markdown, with a title page, contents, and the rest of the front and back matter.
- **Sync if you want to.** Keep a project on several computers through git, with any host you choose ([how](documentation/SYNC.md)). It is never needed to write.

## Get started

To install it, download the Windows installer or the Debian and Ubuntu package from the [releases page](https://github.com/jjdias/writing-studio/releases); the notes there say how. To build it from source instead, you need:

- [Rust](https://www.rust-lang.org/tools/install), stable, 1.80 or later
- On Windows, the [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) and WebView2 (already installed on Windows 10 and 11)
- On Linux, WebKitGTK and GTK 3 with their headers. On Debian or Ubuntu: `sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libxdo-dev libayatana-appindicator3-dev`

Then run:

```bash
cargo run -p writing-studio-desktop
```

On the welcome screen, create a project and choose where its folder goes, outside any other project. Writing Studio makes a `YourTitle.writingstudio` folder there. Add a chapter and a scene, and start writing. Press **Ctrl+/** at any time to see every shortcut.

## Learn more

- [Using Writing Studio](documentation/USAGE.md): every feature, step by step
- [Sync between computers](documentation/SYNC.md)
- [How a project is stored](documentation/PROJECT_FORMAT.md)
- For contributors: [architecture](documentation/ARCHITECTURE.md), [releasing](documentation/RELEASING.md), and the [roadmap](roadmap/README.md)

## License

MIT OR Apache-2.0, at your choice: [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE). Use, study, change, and share it. The packages carry these, the dictionary's license, the Markdown editor's (CodeMirror 6), and a notice of every library the program is built from, in a `licenses` folder: beside the program on Windows, and in `/usr/lib/WritingStudio/licenses` on Linux.

Spellcheck uses the `spellbook` crate (MPL-2.0) and bundles the Hunspell English (US) dictionary, which has its own permissive license: see [`crates/spellcheck/dictionaries/`](crates/spellcheck/dictionaries/README.md).
