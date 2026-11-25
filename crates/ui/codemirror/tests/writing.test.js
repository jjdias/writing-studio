// Writing (`writing.js`): what the format toolbar lights for the caret, what
// each command does to the text, smart typography and pairing, Enter and
// Shift+Enter, a paste put in place, and what `[[` offers. Texts mark the
// caret with `|`, or a selection with two.

import { test } from "node:test";
import assert from "node:assert/strict";
import { insertBracket, CompletionContext } from "@codemirror/autocomplete";
import { pageState, standIn, withSelection } from "./view.js";
import { runCommand, marksAt, typeSmart, enter, lineBreak, insertPasted, noteTitles, pairing } from "../writing.js";

// The text after a command, with the caret or selection, and whether it ran.
function after(text, name, argument) {
  const view = standIn(pageState(text));
  const done = runCommand(view, name, argument);
  return [done, withSelection(view.state)];
}

// Before, the command (with what it takes), and after. Each one on, then off.
const COMMANDS = [
  ["a |word| b", "bold", undefined, "a **|word|** b"],
  ["a **|word|** b", "bold", undefined, "a |word| b"],
  ["a |word| b", "italic", undefined, "a *|word|* b"],
  ["a *|word|* b", "italic", undefined, "a |word| b"],
  ["a |word| b", "underline", undefined, "a <u>|word|</u> b"],
  ["a <u>|word|</u> b", "underline", undefined, "a |word| b"],
  ["a |word| b", "strikeThrough", undefined, "a ~~|word|~~ b"],
  ["a ~~|word|~~ b", "strikeThrough", undefined, "a |word| b"],
  ["a wo|rd b", "bold", undefined, "a wo**|**rd b"],
  ["Tit|le", "formatBlock", "H1", "# Tit|le"],
  ["# Tit|le", "formatBlock", "H2", "## Tit|le"],
  ["# Tit|le", "formatBlock", "P", "Tit|le"],
  ["Sa|id", "formatBlock", "BLOCKQUOTE", "> Sa|id"],
  ["> Sa|id", "formatBlock", "P", "Sa|id"],
  ["co|de", "formatBlock", "PRE", "```\nco|de\n```"],
  ["One|", "insertUnorderedList", undefined, "- One|"],
  ["- One|", "insertUnorderedList", undefined, "One|"],
  ["One|", "insertOrderedList", undefined, "1. One|"],
  ["One|\n\nTwo", "insertHorizontalRule", undefined, "One\n\n* * *\n\n|\n\nTwo"],
  ["the |harbor| was", "highlight", "yellow", "the {==harbor==}| was"],
  ["the |harbor| was", "highlight", "green", "the {==harbor==}{.green}| was"],
  ["the {==|harbor|==}{.green} was", "clearHighlight", undefined, "the |harbor| was"],
  ["the |harbor| was", "note", { kind: "question", text: "which one?" }, "the {==harbor==}{>>Question: which one?<<}| was"],
  ["the {==har|bor==}{>>Question: which?<<} was", "uncomment", undefined, "the har|bor was"],
  ["see |the site| now", "link", "example.com", "see [the site](https://example.com)| now"],
  ["see [the |site|](https://example.com) now", "unlink", undefined, "see the |site| now"],
];

for (const [before, name, argument, expected] of COMMANDS) {
  test(`${name}${argument === undefined ? "" : " " + JSON.stringify(argument)} on ${JSON.stringify(before)}`, () => {
    assert.deepEqual(after(before, name, argument), [true, expected]);
  });
}

test("a command that does not apply changes nothing", () => {
  assert.deepEqual(after("Text|", "formatBlock", "TABLE"), [false, "Text|"]);
  assert.deepEqual(after("Text|", "no such command"), [false, "Text|"]);
  assert.deepEqual(after("the {==harbor==}{>>x<<}| was", "uncomment"), [false, "the {==harbor==}{>>x<<}| was"]);
  assert.deepEqual(after("see |the site| now", "link", "javascript:alert(1)")[1], "see |the site| now", "not a web address");
});

// The caret in a text, and what the toolbar lights for it.
const MARKS = [
  ["a **wo|rd** b", ["bold", "block:p"]],
  ["# He|ad", ["block:h1"]],
  ["> - it|em", ["list", "quote", "block:p"]],
  ["the {==har|bor==}{>>x<<} was", ["note", "block:p"]],
  ["the {==har|bor==} was", ["highlight", "block:p"]],
  ["the {==har|bor==}{.green}{>>x<<} was", ["note", "highlight", "block:p"]],
  ["[li|nk](x)", ["link", "block:p"]],
  ["```\nco|de\n```", ["block:pre"]],
  ["a <u>~~|x|~~</u>", ["strikeThrough", "underline", "block:p"]],
  ["plain te|xt", ["block:p"]],
];

for (const [text, expected] of MARKS) {
  test(`the toolbar for ${JSON.stringify(text)}`, () => assert.deepEqual(marksAt(pageState(text)).sort(), [...expected].sort()));
}

// Typing one character, with smart typography on: the text after, or the
// same text when it is left to be typed as it is.
function typed(text, ch) {
  const view = standIn(pageState(text));
  const { from, to } = view.state.selection.main;
  const done = typeSmart(view, from, to, ch);
  return done ? withSelection(view.state) : null;
}

test("smart typography curls quotes, makes dashes and an ellipsis, and leaves code and addresses alone", () => {
  assert.equal(typed("She said |", '"'), "She said “|");
  assert.equal(typed('"Wait|', '"'), '"Wait”|');
  assert.equal(typed("(|", "'"), "(‘|");
  assert.equal(typed("it|", "'"), "it’|");
  assert.equal(typed("a -|", "-"), "a —|");
  assert.equal(typed("and ..|", "."), "and …|");
  assert.equal(typed("a |", "x"), null, "other characters are typed as they are");
  assert.equal(typed("-|", "-"), null, "a line of dashes may become a scene break");
  assert.equal(typed("- -|", "-"), null);
  assert.equal(typed("```\n|\n```", '"'), null, "not in code");
  assert.equal(typed("[x](http://a|)", "'"), null, "not in an address");
});

test("pairing closes a bracket, a quote, and a Markdown mark", () => {
  const paired = (bracket) => {
    const transaction = insertBracket(pageState("a |", { extensions: [pairing] }), bracket);
    return transaction ? withSelection(transaction.state) : null;
  };
  assert.equal(paired("("), "a (|)");
  assert.equal(paired('"'), 'a "|"');
  assert.equal(paired("*"), "a *|*");
  assert.equal(paired("<"), null, "only what pairing lists");
});

// A key's command on a text: the text after, or null when it leaves the key
// to the editor.
function pressed(command, text) {
  const view = standIn(pageState(text));
  return command(view) ? withSelection(view.state) : null;
}

test("Enter starts a paragraph, continues a list or a quote, and ends an empty item", () => {
  assert.equal(pressed(enter, "One|"), "One\n\n|");
  assert.equal(pressed(enter, "One| two"), "One\n\n| two");
  assert.equal(pressed(enter, "One|\n\nTwo"), "One\n|\n\nTwo", "before a blank line, one line end is enough");
  assert.equal(pressed(enter, "- one|"), "- one\n- |");
  assert.equal(pressed(enter, "1. one|"), "1. one\n2. |");
  assert.equal(pressed(enter, "- |"), "|");
  assert.equal(pressed(enter, "> said|"), "> said\n> |");
  assert.equal(pressed(enter, "```\nco|de\n```"), null, "in code, the editor's own Enter");
});

test("Shift+Enter is a line break in the paragraph, and keeps a quote quoted", () => {
  assert.equal(pressed(lineBreak, "One| two"), "One\\\n| two");
  assert.equal(pressed(lineBreak, "> said|"), "> said\\\n> |");
  assert.equal(pressed(lineBreak, "```\nco|de\n```"), null);
});

test("a paste is put where it was asked for, and never past the end", () => {
  const view = standIn(pageState("a |b| c"));
  insertPasted(view, 2, 3, "**B**");
  assert.equal(withSelection(view.state), "a **B**| c");
  const short = standIn(pageState("short"));
  insertPasted(short, 50, 60, "!");
  assert.equal(withSelection(short.state), "short!|");
});

test("[[ offers the notes' titles, and finishes the link", () => {
  const notes = [
    { title: "Mara", tip: "Characters: A pilot." },
    { title: "Port Ely", tip: "Places: A town." },
  ];
  const offered = (text) => {
    const state = pageState(text, { notes });
    const result = noteTitles(new CompletionContext(state, state.selection.main.head, false));
    return result && { from: result.from, options: result.options.map((option) => [option.label, option.apply, option.detail]) };
  };
  const all = [["Mara", "Mara]]", "Characters: A pilot."], ["Port Ely", "Port Ely]]", "Places: A town."]];
  assert.deepEqual(offered("Ask [[|"), { from: 6, options: all });
  assert.deepEqual(offered("Ask [[Ma|"), { from: 6, options: all }, "the editor narrows them as it is typed");
  assert.deepEqual(offered("Ask [[Ma|]]")?.options[0], ["Mara", "Mara", "Characters: A pilot."], "the ]] already there is kept");
  assert.equal(offered("Ask Ma|"), null, "only after [[");
  const state = pageState("Ask [[|");
  assert.equal(noteTitles(new CompletionContext(state, state.selection.main.head, false)), null, "nothing when there are no notes");
});
