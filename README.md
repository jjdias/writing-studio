# Writing Studio

Open-source, local-first writing studio in Rust. Organize a manuscript, write scenes, recover work, and compile a draft. **Free for the community** under MIT OR Apache-2.0. No account is required. Cloud sync may exist later as an optional extra; it will never be required to write.

## Phase 1

A working desktop app:

- Cargo workspace with a shared core and a Dioxus desktop shell
- Versioned project folder (`*.writingstudio`) backed by SQLite
- Binder: manuscript, chapters, scenes, drag-and-drop, trash/restore
- Structured documents per scene, edited as rich text mapped onto the block model
- Undo/redo, word counts, snapshots, folder backups, timed local autosave
- Writing, corkboard, outliner, and history workspace modes
- Compile one book to local Markdown or DOCX

Full list: [`documentation/PHASE1.md`](documentation/PHASE1.md).

## Phase 2

The writing environment around that studio:

- Links, notes, highlights, headings, code blocks, and scene breaks stored on the scene
- Full-text search, binder labels and colors, corkboard grouping, and an editable outliner
- Daily targets, sittings, themes, and a panel layout saved in the project

Full list: [`documentation/PHASE2.md`](documentation/PHASE2.md).

## Phase 3

Story development in the same local project:

- One project holds every book in a series, plus one shared bible
- Plot grid, character profiles, world notes, and a research library of notes and files
- Binder, corkboard, outliner, and search can show one book or the whole series
- Compile, the word target, today, and the sitting follow the current book
- Bible entries can point at scenes. Research is edited only in Research

Full list: [`documentation/PHASE3.md`](documentation/PHASE3.md).

Phases 1, 2, and 3 are done. Current work is Phase 4, local professional editing. See the [roadmap](ROADMAP.md).

## Requirements

- Rust stable (1.80+)
- Windows: [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) so bundled SQLite can compile
- WebView2 (usually already installed on Windows 10/11)

## Run

```bash
cargo run -p writing-studio-desktop
```

Create a project from the welcome screen. Choose an empty parent folder; Writing Studio writes `YourTitle.writingstudio/`.

## Tests

```bash
cargo test --workspace --exclude writing-studio-desktop --exclude writing-studio-ui
```

Desktop/UI crates need a windowing runtime and are not part of the default library test sweep.

## Documentation

- [Phase 1 features](documentation/PHASE1.md)
- [Phase 2 features](documentation/PHASE2.md)
- [Phase 3 features](documentation/PHASE3.md)
- [Phase 4 so far](documentation/PHASE4.md)
- [Usage](documentation/USAGE.md)
- [Architecture](documentation/ARCHITECTURE.md)
- [Project format](documentation/PROJECT_FORMAT.md)
- [Roadmap](ROADMAP.md)

## License

MIT OR Apache-2.0. Use, study, modify, and share it with the community.
