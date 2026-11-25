// That the tests run at all: the scene's language parses a line, and the
// helpers every other test uses do what they say.

import { test } from "node:test";
import assert from "node:assert/strict";
import { nodeNames, pageState, standIn, withSelection } from "./view.js";
import { knownNotes } from "../preview.js";

test("the scene's language parses a line with a note and a link", () => {
  const names = nodeNames("Ask {==the harbor==}{>>Question: which?<<} and [[Mara]].");
  assert.ok(names.includes("CriticHighlight"), names.join(" "));
  assert.ok(names.includes("WikiLink"), names.join(" "));
});

test("a page state takes the caret, a selection, and the known notes", () => {
  assert.equal(withSelection(pageState("One |two")), "One |two");
  assert.equal(withSelection(pageState("One |two| three")), "One |two| three");
  const state = pageState("Text", { notes: [{ title: "Mara", tip: "Characters: A pilot." }] });
  const mara = state.field(knownNotes).get("mara");
  assert.equal(String(mara), "Characters: A pilot.");
  assert.equal(mara.title, "Mara");
});

test("the stand-in view applies what is dispatched to its state", () => {
  const view = standIn(pageState("One|"));
  view.dispatch({ changes: { from: 3, insert: " two" }, selection: { anchor: 7 } });
  assert.equal(withSelection(view.state), "One two|");
  assert.deepEqual(view.visibleRanges, [{ from: 0, to: 7 }]);
});
