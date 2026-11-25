---
paths:
  - "**/tests/**"
  - "**/*test*.rs"
  - "**/*tests.rs"
  - "crates/ui/codemirror/**"
---

# Testing

Always-on test and debug limits are in `AGENTS.md`. This file adds rules for test code.

- Prefer a focused unit test over an integration or end-to-end run. Use the broader suites only when the change crosses that boundary, and the full workspace suite only before a release or when asked.
- Keep tests asserting the behavior under change. Do not weaken an assertion so the current code passes.
- Name the crate and the test filter before a long `cargo test` invocation.
- Integration coverage belongs next to the crate that owns the behavior. Do not add a new harness for a case an existing test can express.
- The Markdown page's JavaScript (`crates/ui/codemirror`) has its own tests: run `npm test` there for a change to it, and `npm run build` to commit the bundle with the change, as CI checks it.
