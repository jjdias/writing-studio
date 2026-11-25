// The scene's language (`syntax.js`) and the small readers built on it:
// what the page finds in a scene's text, and what it leaves as text, so the
// page never shows formatting the scene's file would not keep.

import { test } from "node:test";
import assert from "node:assert/strict";
import { sceneLanguage, footnoteTexts, readNote } from "../syntax.js";
import { wikiParts } from "../preview.js";
import { linkTarget, noteBody } from "../writing.js";

// What a test looks at: the nodes the page draws as formatting.
const SHOWN = new Set([
  "StrongEmphasis", "Emphasis", "Underline", "Strikethrough", "Highlight", "CriticHighlight", "CriticColor",
  "CriticNote", "WikiLink", "FootnoteRef", "FootnoteDefinition", "ATXHeading1", "ATXHeading2", "ATXHeading3",
  "Blockquote", "BulletList", "OrderedList", "FencedCode", "HorizontalRule", "Link",
]);

// Each shown node of `text`, as `Name "its text"`, in order.
function found(text) {
  const out = [];
  sceneLanguage.parser.parse(text).iterate({
    enter(node) {
      if (SHOWN.has(node.name)) out.push(`${node.name} ${JSON.stringify(text.slice(node.from, node.to))}`);
    },
  });
  return out;
}

// Text in, what the page finds in it.
const FINDS = [
  ["**bold**", ['StrongEmphasis "**bold**"']],
  ["*italic*", ['Emphasis "*italic*"']],
  ["_italic_", ['Emphasis "_italic_"']],
  ["<u>under</u>", ['Underline "<u>under</u>"']],
  ["~~gone~~", ['Strikethrough "~~gone~~"']],
  ["==marked==", ['Highlight "==marked=="']],
  ["{==text==}", ['CriticHighlight "{==text==}"']],
  ["{==text==}{.green}", ['CriticHighlight "{==text==}"', 'CriticColor "{.green}"']],
  ["{==text==}{>>Question: which?<<}", ['CriticHighlight "{==text==}"', 'CriticNote "{>>Question: which?<<}"']],
  ["[[Mara]]", ['WikiLink "[[Mara]]"']],
  ["[[Mara|the pilot's daughter]]", ['WikiLink "[[Mara|the pilot\'s daughter]]"']],
  ["fast[^1] as that", ['FootnoteRef "[^1]"']],
  ["[^1]: The source.", ['FootnoteDefinition "[^1]: The source."']],
  ["# One", ['ATXHeading1 "# One"']],
  ["### Three", ['ATXHeading3 "### Three"']],
  ["> Said.", ['Blockquote "> Said."']],
  ["- one\n- two", ['BulletList "- one\\n- two"']],
  ["1. one", ['OrderedList "1. one"']],
  ["```\ncode\n```", ['FencedCode "```\\ncode\\n```"']],
  ["* * *", ['HorizontalRule "* * *"']],
  ["[a link](https://example.com)", ['Link "[a link](https://example.com)"']],
];

for (const [text, expected] of FINDS) {
  test(`finds ${JSON.stringify(text)}`, () => assert.deepEqual(found(text), expected));
}

// Text the page leaves as text: what is half written, and what a scene has
// no place for. None of the scene's own formatting may be found in it.
const OWN = ["StrongEmphasis", "Emphasis", "Underline", "Strikethrough", "Highlight", "CriticHighlight", "CriticNote", "WikiLink", "ATXHeading1", "FencedCode"];
const LEAVES = [
  "a [[ alone",
  "[[]]",
  "[[Mara\nVance]]",
  "{== no end",
  "==text}",
  "=== three",
  "**not closed",
  "a `code` word",
  "Title\n===",
  "    indented",
  "a {>> note with no end",
];

for (const text of LEAVES) {
  test(`leaves ${JSON.stringify(text)} as text`, () => {
    const own = found(text).filter((item) => OWN.some((name) => item.startsWith(name + " ")));
    assert.deepEqual(own, []);
  });
}

test("a Markdown link after a [[ is a link, not a note link", () => {
  assert.deepEqual(found("[[Mara](x)"), ['Link "[Mara](x)"']);
});

test("a note's end can be written in it, escaped", () => {
  const text = "{==text==}{>>a \\<<} b<<}";
  assert.deepEqual(found(text), ['CriticHighlight "{==text==}"', 'CriticNote "{>>a \\\\<<} b<<}"']);
});

test("readNote reads a note's kind, whether it is resolved, and its text", () => {
  assert.deepEqual(readNote("Question: which harbor?"), { kind: "question", resolved: false, text: "which harbor?" });
  assert.deepEqual(readNote("(resolved) Revise: tighten"), { kind: "revise", resolved: true, text: "tighten" });
  assert.deepEqual(readNote("Check: the date"), { kind: "check", resolved: false, text: "the date" });
  assert.deepEqual(readNote("Comment: fine"), { kind: "comment", resolved: false, text: "fine" });
  assert.deepEqual(readNote("Just a thought"), { kind: "comment", resolved: false, text: "Just a thought" });
  assert.deepEqual(readNote("Check:no space"), { kind: "comment", resolved: false, text: "Check:no space" });
  assert.deepEqual(readNote("a \\<<\\} b"), { kind: "comment", resolved: false, text: "a <<} b" }, "escapes are read");
});

test("a note written by noteBody is one note in the text, and reads back as it was", () => {
  for (const kind of ["comment", "question", "revise", "check"]) {
    for (const text of ["which one?", "a <<} in it", "braces {like} these", "back\\slash"]) {
      const line = `{==harbor==}{>>${noteBody(kind, text)}<<} after`;
      const notes = [];
      sceneLanguage.parser.parse(line).iterate({
        enter(node) {
          if (node.name === "CriticNote") notes.push([node.from, node.to]);
        },
      });
      // One note, from its `{>>` to the `<<}` before " after".
      assert.deepEqual(notes, [[line.indexOf("{>>"), line.length - " after".length]], `${kind}: ${text}`);
      const inside = line.slice(notes[0][0] + 3, notes[0][1] - 3);
      assert.deepEqual(readNote(inside), { kind, resolved: false, text }, `${kind}: ${text}`);
    }
  }
});

test("footnoteTexts pairs each mark with its text", () => {
  const notes = footnoteTexts("Fast[^1] and far[^n2].\n\n[^1]: The source.\n[^n2]:Another\n[^not a label]: no");
  assert.deepEqual([...notes], [["1", "The source."], ["n2", "Another"]]);
});

test("wikiParts splits a link's title from the words it shows", () => {
  assert.deepEqual(wikiParts("Mara"), { title: "Mara", words: "Mara", bar: -1 });
  assert.deepEqual(wikiParts(" Mara | the pilot's daughter "), { title: "Mara", words: "the pilot's daughter", bar: 6 });
});

test("linkTarget makes a link's address, and refuses one that is not a web or mail address", () => {
  assert.equal(linkTarget("example.com/a"), "https://example.com/a");
  assert.equal(linkTarget("https://example.com"), "https://example.com");
  assert.equal(linkTarget("mailto:mara@example.com"), "mailto:mara@example.com");
  assert.equal(linkTarget("https://example.com/a b"), "<https://example.com/a b>");
  assert.equal(linkTarget("https://example.com/<x>"), "<https://example.com/%3Cx%3E>");
  assert.equal(linkTarget("javascript:alert(1)"), null);
  assert.equal(linkTarget("file:///etc/passwd"), null);
  assert.equal(linkTarget("   "), null);
});
