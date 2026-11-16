# Writing Studio

Open-source, local-first writing studio in Rust. Organize a manuscript, write scenes, recover work, and compile a draft. **Free for the community** under MIT OR Apache-2.0. No account is required. Sync between machines is planned through git, with any host the writer chooses; it will never be required to write.

## Phase 1

A working desktop app:

- Cargo workspace with a shared core and a Dioxus desktop shell
- Versioned project folder (`*.writingstudio`): each scene is an HTML file, with a SQLite cache beside them
- Binder: manuscript, chapters, scenes, drag-and-drop, trash/restore
- Structured documents per scene, edited as rich text mapped onto the block model
- Undo/redo, word counts, snapshots, folder backups, timed local autosave
- Writing, corkboard, outliner, and history workspace modes
- Compile one book to local Markdown or DOCX

Full list: [`roadmap/done/phase-1.md`](roadmap/done/phase-1.md).

## Phase 2

The writing environment around that studio:

- Links, notes, highlights, headings, code blocks, and scene breaks stored on the scene
- Full-text search, binder labels and colors, corkboard grouping, and an editable outliner
- Daily targets, sittings, themes, and a panel layout saved in the project

Full list: [`roadmap/done/phase-2.md`](roadmap/done/phase-2.md).

## Phase 3

Story development in the same local project:

- One project holds every book in a series, plus one shared bible
- Plot grid, character profiles, world notes, and a research library of notes and files
- Binder, corkboard, outliner, and search can show one book or the whole series
- Compile, the word target, today, and the sitting follow the current book
- Bible entries can point at scenes. Research is edited only in Research

Full list: [`roadmap/done/phase-3.md`](roadmap/done/phase-3.md).

## Phase 4

Professional editing, still local:

- Compare a snapshot with the current text or with another snapshot
- Notes with a kind that can be resolved, and highlights in four colors
- Track changes to a scene, then accept or reject them
- Spellcheck on this computer, with a project dictionary
- Scene diagnostics: goal, conflict, and outcome, with checks on the text

Full list: [`roadmap/done/phase-4.md`](roadmap/done/phase-4.md).

## Phase 5

Projects as plain files:

- Each scene is an HTML file that any browser opens, with the book's structure in JSON beside it
- A SQLite cache on each computer, filled from the files when a project opens
- Scenes can be written as Markdown for other writing tools
- Optional sync between computers through git, with any git host ([how](documentation/SYNC.md))

Full list: [`roadmap/done/phase-5.md`](roadmap/done/phase-5.md).

Phases 1 through 5 are done. See the [roadmap](roadmap/README.md) for what is next.

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

- [Phase 1 features](roadmap/done/phase-1.md)
- [Phase 2 features](roadmap/done/phase-2.md)
- [Phase 3 features](roadmap/done/phase-3.md)
- [Phase 4 features](roadmap/done/phase-4.md)
- [Phase 5 features](roadmap/done/phase-5.md)
- [Usage](documentation/USAGE.md)
- [Architecture](documentation/ARCHITECTURE.md)
- [Project format](documentation/PROJECT_FORMAT.md)
- [Roadmap](roadmap/README.md)

## License

MIT OR Apache-2.0. Use, study, modify, and share it with the community.

Spellcheck uses the `spellbook` crate (MPL-2.0) and bundles the Hunspell English (US) dictionary, which has its own permissive license: see [`crates/spellcheck/dictionaries/`](crates/spellcheck/dictionaries/README.md).
