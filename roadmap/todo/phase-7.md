# Phase 7 — Cross-platform release

Not started.

- Packaged Windows, macOS, and Linux builds
- Later: Android, iOS, and Dioxus Web/WASM
- A git engine built into the program, so that Sync needs nothing installed and works on phones and tablets. Moved here from Phase 5. It needs a decision on how the program handles SSH keys or tokens, which it does not do today
- Accessibility and performance passes for the packaged apps. The desktop window's overhaul, including keyboard and contrast, is Phase 8
- Automated community builds
- Packaged builds carry the license notice of the bundled spelling dictionary (`crates/spellcheck/dictionaries/`)
