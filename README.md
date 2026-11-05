# Writing Studio

Open-source, local-first writing studio in Rust. Organize a manuscript, write scenes, recover work, and compile a draft. **Free for the community** under MIT OR Apache-2.0. No account is required. Cloud sync may exist later as an optional extra; it will never be required to write.

## Phase 1

A working desktop app:

- Cargo workspace with a shared core and a Dioxus desktop shell
- Versioned project folder (`*.writingstudio`) backed by SQLite
- Binder: manuscript, research, chapters, scenes, notes, drag-and-drop, trash/restore
- Structured documents per scene, edited as rich text mapped onto the block model
- Undo/redo, word counts, snapshots, folder backups, timed local autosave
- Writing, corkboard, outliner, and history workspace modes
- Compile the manuscript to local Markdown or DOCX

Full list: [`documentation/PHASE1.md`](documentation/PHASE1.md).

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
- [Usage](documentation/USAGE.md)
- [Architecture](documentation/ARCHITECTURE.md)
- [Project format](documentation/PROJECT_FORMAT.md)
- [Roadmap](ROADMAP.md)

## License

MIT OR Apache-2.0. Use, study, modify, and share it with the community.
