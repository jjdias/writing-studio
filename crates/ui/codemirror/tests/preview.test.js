// The preview (`preview.js`) and what `places.js` decides without the page's
// layout: what the reader sees where the caret is not, links and notes as
// the page marks them, the review list, where spelling's words are, and
// what fading leaves at full strength.

import { test } from "node:test";
import assert from "node:assert/strict";
import { pageState, standIn } from "./view.js";
import { previewDecorations } from "../preview.js";
import { reviewItems, wordsIn, sentenceAround, currentRange, selectReview, findWord } from "../places.js";

// The text as the preview shows it: what it hides taken out, and what it
// draws in place of something (a bullet, a note's pin, a footnote's number)
// put in.
function shown(state) {
  const { decorations } = previewDecorations(standIn(state));
  const replaced = [];
  decorations.between(0, state.doc.length, (from, to, decoration) => {
    if (decoration.spec.widget || (decoration.point && from < to)) {
      replaced.push({ from, to, text: decoration.spec.widget ? decoration.spec.widget.text : "" });
    }
  });
  replaced.sort((a, b) => a.from - b.from);
  let out = "";
  let at = 0;
  for (const piece of replaced) {
    if (piece.from < at) continue;
    out += state.doc.sliceString(at, piece.from) + piece.text;
    at = piece.to;
  }
  return out + state.doc.sliceString(at);
}

// The text each mark of `className` covers, with its attributes.
function marked(state, className) {
  const { decorations } = previewDecorations(standIn(state));
  const out = [];
  decorations.between(0, state.doc.length, (from, to, decoration) => {
    const classes = (decoration.spec.class || "").split(" ");
    if (!decoration.point && from < to && classes.includes(className)) {
      // Attributes as the page sets them: as text.
      const attributes = Object.fromEntries(Object.entries(decoration.spec.attributes || {}).map(([key, value]) => [key, String(value)]));
      out.push({ text: state.doc.sliceString(from, to), class: decoration.spec.class, ...attributes });
    }
  });
  return out;
}

const notes = [{ title: "Mara", tip: "Characters: A pilot." }];

// Text with the caret (`¦`) on a line of its own, away from the formatting,
// and what the reader sees.
const LIVE = [
  ["a **bold** b\n\n¦", "a bold b\n\n"],
  ["a *italic* b\n\n¦", "a italic b\n\n"],
  ["<u>under</u> b\n\n¦", "under b\n\n"],
  ["~~gone~~ b\n\n¦", "gone b\n\n"],
  ["==marked== b\n\n¦", "marked b\n\n"],
  ["# One\n\n¦", "One\n\n"],
  ["> Said.\n\n¦", "Said.\n\n"],
  ["- one\n- two\n\n¦", "• one\n• two\n\n"],
  ["[the site](https://example.com) now\n\n¦", "the site now\n\n"],
  ["Ask [[Mara]] now\n\n¦", "Ask Mara now\n\n"],
  ["Ask [[Mara|her]] now\n\n¦", "Ask her now\n\n"],
  ["Ask [[Nobody]] now\n\n¦", "Ask Nobody now\n\n"],
  ["the {==harbor==}{>>Question: which?<<} was\n\n¦", "the harbor✎ was\n\n"],
  ["the {==harbor==}{.green} was\n\n¦", "the harbor was\n\n"],
  ["fast[^1] go\n\n[^1]: The source.\n\n¦", "fast1 go\n\n[^1]: The source.\n\n"],
  ["a \\*star\\* b\n\n¦", "a *star* b\n\n"],
];

for (const [text, expected] of LIVE) {
  test(`the preview shows ${JSON.stringify(text)} as ${JSON.stringify(expected)}`, () => {
    assert.equal(shown(pageState(text, { notes, caret: "¦" })), expected);
  });
  test(`Source mode shows ${JSON.stringify(text)} as it is written`, () => {
    const state = pageState(text, { notes, caret: "¦", source: true });
    assert.equal(shown(state), state.doc.toString());
  });
}

test("the marks come back where the caret is", () => {
  assert.equal(shown(pageState("a **bo|ld** b")), "a **bold** b");
  assert.equal(shown(pageState("# On|e")), "# One");
  assert.equal(shown(pageState("Ask [[Ma|ra]] now")), "Ask [[Mara]] now");
  assert.equal(shown(pageState("the {==har|bor==}{>>x<<} was")), "the {==harbor==}{>>x<<} was");
});

test("a link to a known note carries its tip, and one to a title no note has is faint", () => {
  const state = pageState("Ask [[Mara]] and [[Nobody]]\n\n|", { notes });
  assert.deepEqual(marked(state, "ws-wikilink"), [
    { text: "Mara", class: "ws-wikilink", "data-note": "Mara", title: "Characters: A pilot." },
    { text: "Nobody", class: "ws-wikilink ws-wikilink-missing", "data-note": "Nobody", title: "No note is called “Nobody” yet" },
  ]);
});

test("a note's words are marked with its kind and its text, and a link with its address", () => {
  const note = pageState("the {==harbor==}{>>Question: which?<<} was\n\n|");
  assert.deepEqual(marked(note, "ws-note"), [{ text: "harbor", class: "ws-note ws-note-question", title: "which?" }]);
  const link = pageState("[the site](https://example.com)\n\n|");
  assert.deepEqual(marked(link, "cm-link"), [{ text: "the site", class: "cm-link", title: "https://example.com" }]);
});

test("the review list has the notes and the highlights in order, neighbours with one note as one", () => {
  const state = pageState("One {==a==}{>>x<<} and {==b==}{.green}.\n\nTwo ==c== and {==d==}{>>y<<}{==e==}{>>y<<}.");
  const texts = (type) => reviewItems(state, type).map((item) => state.doc.sliceString(item.from, item.to));
  assert.deepEqual(texts("note"), ["a", "d==}{>>y<<}{==e"]);
  assert.deepEqual(texts("highlight"), ["b", "c"]);
  const view = standIn(state);
  assert.equal(selectReview(view, "highlight", 1), true);
  assert.equal(view.state.sliceDoc(view.state.selection.main.from, view.state.selection.main.to), "c");
  assert.equal(selectReview(view, "note", 9), false, "past the end of the list");
});

test("spelling's words are found as text, not in an address, a note, or a mark", () => {
  const state = pageState("Teh harbr was [harbr](https://harbr.com) and {==x==}{>>harbr<<} harbr’s end");
  const hits = wordsIn(state, new Set(["Teh", "harbr", "harbr's"]), 0, state.doc.length).map((hit) => state.doc.sliceString(hit.from, hit.to));
  assert.deepEqual(hits, ["Teh", "harbr", "harbr", "harbr’s"], "the link's words, not its address; not the note's text");
  assert.deepEqual(wordsIn(state, new Set(), 0, state.doc.length), []);
});

test("finding a word goes to the next copy and round to the top", () => {
  const view = standIn(pageState("harbr one harbr two|"));
  assert.equal(findWord(view, "harbr"), true);
  assert.equal(view.state.selection.main.from, 0, "round to the top");
  findWord(view, "harbr");
  assert.equal(view.state.selection.main.from, 10);
  assert.equal(findWord(view, "absent"), false);
});

test("fading keeps the caret's sentence, or its paragraph, at full strength", () => {
  assert.deepEqual(sentenceAround("One. Two here! Three", 7), [5, 14]);
  assert.deepEqual(sentenceAround("One. Two here! Three", 1), [0, 4]);
  assert.deepEqual(sentenceAround("“Go.” She went.", 8), [6, 15], "a closing quote stays with its sentence");
  const range = (text, by) => {
    const state = pageState(text);
    const found = currentRange(state, by);
    return found && state.doc.sliceString(found.from, found.to);
  };
  assert.equal(range("One. Tw|o here. Three.\n\nNext.", "sentence"), "Two here.");
  assert.equal(range("One. Tw|o here. Three.\n\nNext.", "paragraph"), "One. Two here. Three.");
  assert.equal(range("One.\n\n|\n\nTwo", "sentence"), null, "between paragraphs, nothing");
});
