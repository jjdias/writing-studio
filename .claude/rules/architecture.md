---
paths:
  - "crates/**/*.rs"
  - "apps/**/*.rs"
  - "**/Cargo.toml"
  - "**/Cargo.lock"
---

# Architecture

Read `documentation/ARCHITECTURE.md` before changing crate boundaries, the document model, or how projects are stored.

- The desktop app is a thin launcher. The UI does not own the document schema.
- Reuse an existing crate, type, or workspace dependency before adding a new one. Confirm the dependency is already in the workspace before using it.
- Do not add a crate for something the standard library or an existing dependency already does. Do not swap a library or bump unrelated crates.
- Say why a new dependency is required before adding it. Weigh maintenance, license, and whether it stays optional for offline use.
- Saving stays transactional. Export, search, and any future sync must consume the same document JSON.
- Do not expand a placeholder crate (collaboration, synchronization, security) into a new subsystem unless the task asks for that.
- Follow the style of the surrounding crate. Comments explain a non-obvious decision, not the next line of code.
- Handle `Result` and `Option` explicitly. Do not leave a TODO in place of work the task required.
