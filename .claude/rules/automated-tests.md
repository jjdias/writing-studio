---
paths:
  - "apps/**"
  - "crates/**"
  - "**/tests/**"
---

# Automated tests only

Agents do not test by hand. Every check that a change works is a test that runs from the command line, here and in CI. A check done by hand costs tokens, is gone when the session ends, and never runs again when the code changes.

## Prohibited

- Launching the program to look at a change, driving its window, or taking screenshots of it to decide whether it works. That covers the browser pane, computer use, and the `run` skill.
- Probes that stand in for a test and are thrown away: `println!`, `console.log`, a scratch script, a REPL call.
- Walking through a scenario in your head or in the code and calling it checked.
- Saying a change works because it builds.

Building and starting the program when the user asks to try it is allowed. Start it and stop there: the user does the looking.

## What to do instead

- Write the test with the fix, or before it. See it fail without the change once: break the change, run the test, restore the change.
- Put the test where the behavior lives:
  - a Rust unit test in the crate's module;
  - an integration test in the crate's `tests/`;
  - the page's JavaScript in `crates/ui/codemirror/tests/`, with `npm test`, using the stand-in view in `tests/view.js`.
- When a decision is made in the UI, move it into a plain function the test can call, as the preview and the review list do. Do not test the window to reach it.
- When something can only be seen in a window (layout, scrolling, animation, focus), do not try it by hand. Name it in the summary as not covered, say what the user should look at, and stop.
- A change to `rsx!` gets `cargo check --workspace --release --locked`. Dioxus writes different code in a release build, and a moved value can compile in debug and fail in release.

## What a good test is

- It asserts the behavior the task is about, and it fails for the bug. A test that only runs the code, or repeats the implementation, tests nothing.
- It is deterministic and offline: no network, no wall clock, no sleeps, no unseeded randomness, no reliance on `HashMap` order. Files go in a temporary folder.
- It checks one behavior, and its name says which.
- A whole-output snapshot is only for output that is itself the contract, such as an export.
- A flaky test is a bug to fix. Do not rerun it until it passes, and do not weaken or delete it to get a green run.

## Running and reporting

- Run the tests that cover the change: `cargo test -p <crate> <filter>`, or `npm test` in `crates/ui/codemirror`. The full workspace runs before a push, or when asked. CI runs everything on each push.
- Report what ran and the counts, what failed, and what is not covered by any test.
