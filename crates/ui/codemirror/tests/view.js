// What the page's tests share: an editor state with the page's language for
// some text, and a stand-in for the editor view, so that what works on the
// state can be tested in Node without a window.
//
// The stand-in has what the page's logic reads and does: the state, the
// visible range (the whole text), and `dispatch`, which applies a
// transaction to the state as the view would. It draws nothing, measures
// nothing, and ignores focus; what needs those is tested in the window.

import { EditorSelection, EditorState, Transaction } from "@codemirror/state";
import { sceneLanguage } from "../syntax.js";
import { knownNotes, noteMap, setNotes, sourceMode } from "../preview.js";

/// A state of `text` with the page's language and its list of known notes.
/// A `|` in the text marks the caret, and two mark a selection between
/// them; they are taken out of the text. `caret` is that mark, for a text
/// that needs a `|` of its own. `source` is Source mode; `notes` are the
/// known notes as the program sends them, `{ title, tip }` each.
export function pageState(text, { source = false, notes = [], extensions = [], caret = "|" } = {}) {
  const marks = [];
  let plain = "";
  for (const ch of text) {
    if (ch === caret) marks.push(plain.length);
    else plain += ch;
  }
  const selection =
    marks.length === 0 ? undefined
    : marks.length === 1 ? EditorSelection.cursor(marks[0])
    : EditorSelection.range(marks[0], marks[1]);
  let state = EditorState.create({
    doc: plain,
    selection,
    extensions: [sceneLanguage, knownNotes, sourceMode.of(source), ...extensions],
  });
  if (notes.length > 0) {
    state = state.update({ effects: setNotes.of(noteMap(notes)) }).state;
  }
  return state;
}

/// A stand-in for the editor view over `state`.
export function standIn(state) {
  const view = {
    state,
    dispatch(...specs) {
      const transaction =
        specs.length === 1 && specs[0] instanceof Transaction ? specs[0] : view.state.update(...specs);
      view.state = transaction.state;
    },
    get visibleRanges() {
      return [{ from: 0, to: view.state.doc.length }];
    },
    get viewport() {
      return { from: 0, to: view.state.doc.length };
    },
    focus() {},
  };
  return view;
}

/// The names of the syntax nodes `text` parses into, in order, without the
/// document and paragraph that hold them all.
export function nodeNames(text) {
  const names = [];
  sceneLanguage.parser.parse(text).iterate({
    enter(node) {
      if (node.name !== "Document" && node.name !== "Paragraph") names.push(node.name);
    },
  });
  return names;
}

/// The text with a `|` where the caret is, or two around the selection, to
/// compare with what a test expects.
export function withSelection(state) {
  const { from, to } = state.selection.main;
  const text = state.doc.toString();
  return from === to ? text.slice(0, from) + "|" + text.slice(from) : text.slice(0, from) + "|" + text.slice(from, to) + "|" + text.slice(to);
}
