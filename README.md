# Writing Studio

Local-first writing environment in Rust: manuscript binder, scene documents, corkboard, outliner, autosave, and backups. Cloud sync is planned later and will never be required to write.

## Phase 1 (this tree)

- Cargo workspace with a shared core and a Dioxus desktop app
- Versioned project folder (`*.writingstudio`) backed by SQLite
- Binder: manuscript, research, chapters, scenes, notes
- Structured documents stored per scene (plain-text editing in v1)
- Undo/redo, word counts, snapshots, folder backups, timed local autosave
- Writing, corkboard, and outliner workspace modes

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

## Layout

See `documentation/ARCHITECTURE.md` and `documentation/PROJECT_FORMAT.md`.
