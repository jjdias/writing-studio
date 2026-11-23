// The page's Markdown editor: CodeMirror 6, bundled into one file that the
// program carries (crates/ui/assets/codemirror.js), so nothing is loaded from
// the network. To rebuild it after changing a file here or a version in
// package.json: `npm ci` then `npm run build`, here. Commit the bundle and
// LICENSES.txt with the change.
//
// The program talks to it through `window.wsEditor`, below. It mounts one
// editor for the open scene, and the editor sends it messages through
// `send`: `{ kind: "text", text }` after every change, `{ kind: "blur" }`
// when the page loses the focus, `{ kind: "marks", marks }` when the marks
// at the caret change, and `{ kind: "paste", html, from, to }` for rich
// text to be turned into Markdown and given back through `pasted`.

import { EditorState, Compartment, Annotation, Transaction } from "@codemirror/state";
import { EditorView, keymap, drawSelection } from "@codemirror/view";
import { defaultKeymap, history, undo, redo } from "@codemirror/commands";
import { sceneLanguage } from "./syntax.js";
import { preview, sourceMode } from "./preview.js";
import { writingKeys, smartTypography, pairing, pasteHandler, insertPasted, runCommand, marksAt } from "./writing.js";

let view = null;
let token = null;
let extensions = [];
const editable = new Compartment();
const undoHistory = new Compartment();
const showSource = new Compartment();
const smart = new Compartment();
const pairs = new Compartment();
// The selection a link or note is for while its field is open.
let held = null;
// Marks a change the program made, which is not sent back as the writer's.
const fromProgram = Annotation.define();

// A key the editor acted on is not also a shortcut of the program's: the
// program's own keys (Ctrl+S, Ctrl+1, Ctrl+F) pass through untouched.
function keepHandledKeys(element) {
  element.addEventListener("keydown", (event) => {
    if (event.defaultPrevented && (event.ctrlKey || event.metaKey || event.altKey)) {
      event.stopPropagation();
    }
  });
}

// `options`: `{ source, smart, pair }`: every Markdown mark shown as typed,
// smart typography, and pairing of marks and brackets.
function mount(parent, text, send, id, options = {}) {
  unmount();
  token = id;
  let lastMarks = null;
  extensions = [
    undoHistory.of(history()),
    drawSelection(),
    EditorView.lineWrapping,
    EditorView.contentAttributes.of({ "aria-label": "Manuscript", spellcheck: "false" }),
    editable.of(EditorView.editable.of(true)),
    sceneLanguage,
    showSource.of(sourceMode.of(!!options.source)),
    preview,
    writingKeys,
    smart.of(options.smart ? smartTypography : []),
    pairs.of(options.pair ? pairing : []),
    pasteHandler(send),
    // Ctrl+Z and Ctrl+Y go to the editor's own history while it has
    // something to undo. When it has nothing, as just after a scene was
    // split or a note resolved, the key goes on to the program, whose undo
    // takes back those.
    keymap.of([
      { key: "Mod-z", run: undo, preventDefault: false },
      { key: "Mod-y", run: redo, preventDefault: false },
      { key: "Mod-Shift-z", run: redo, preventDefault: false },
      ...defaultKeymap,
    ]),
    EditorView.updateListener.of((update) => {
      if (update.docChanged && update.transactions.some((tr) => !tr.annotation(fromProgram))) {
        send({ kind: "text", text: update.state.doc.toString() });
      }
      // The marks at the caret, for the toolbar, when they change.
      if (update.docChanged || update.selectionSet || lastMarks === null) {
        const marks = marksAt(update.state);
        const key = marks.join(",");
        if (key !== lastMarks) {
          lastMarks = key;
          send({ kind: "marks", marks });
        }
      }
    }),
    EditorView.domEventHandlers({
      blur: () => {
        send({ kind: "blur" });
        return false;
      },
    }),
  ];
  view = new EditorView({ state: EditorState.create({ doc: text, extensions }), parent });
  keepHandledKeys(view.dom);
  return true;
}

// Only the editor that `id` mounted: a scene's page going away must not
// close the next scene's editor, if that one was mounted first.
function unmount(id) {
  if (!view || (id !== undefined && id !== token)) return false;
  view.destroy();
  view = null;
  token = null;
  extensions = [];
  return true;
}

function getText() {
  return view ? view.state.doc.toString() : "";
}

// Show text that changed outside the page: an undo, a resolved note, a
// split. Only the part that differs is replaced, so the caret, the
// selection, and the scroll stay where they were, and it is not reported
// back as the writer's change. The editor's history starts again, so the
// next Ctrl+Z goes to the program and takes back what it did.
function setText(text) {
  if (!view) return false;
  const current = view.state.doc.toString();
  if (current === text) return true;
  let start = 0;
  while (start < current.length && start < text.length && current[start] === text[start]) start++;
  let endOld = current.length;
  let endNew = text.length;
  while (endOld > start && endNew > start && current[endOld - 1] === text[endNew - 1]) {
    endOld--;
    endNew--;
  }
  view.dispatch({
    changes: { from: start, to: endOld, insert: text.slice(start, endNew) },
    annotations: [fromProgram.of(true), Transaction.addToHistory.of(false)],
  });
  view.dispatch({ effects: undoHistory.reconfigure([]) });
  view.dispatch({ effects: undoHistory.reconfigure(history()) });
  return true;
}

function setEditable(on) {
  if (!view) return false;
  view.dispatch({ effects: editable.reconfigure(EditorView.editable.of(!!on)) });
  return true;
}

// The selection, as offsets into the text.
function getSelection() {
  if (!view) return [];
  const { from, to } = view.state.selection.main;
  return [from, to];
}

// Select `from` to `to` (clamped to the text), and show it.
function setSelection(from, to = from) {
  if (!view) return false;
  const length = view.state.doc.length;
  const anchor = Math.max(0, Math.min(from, length));
  const head = Math.max(0, Math.min(to, length));
  view.dispatch({ selection: { anchor, head }, scrollIntoView: true });
  view.focus();
  return true;
}

function setOptions(options) {
  if (!view) return false;
  view.dispatch({
    effects: [
      showSource.reconfigure(sourceMode.of(!!options.source)),
      smart.reconfigure(options.smart ? smartTypography : []),
      pairs.reconfigure(options.pair ? pairing : []),
    ],
  });
  return true;
}

// Run one of the format toolbar's commands on the selection, or on the
// held one when a link or note field had it.
function command(name, argument) {
  if (!view) return false;
  const range = held;
  held = null;
  view.focus();
  if (range) view.dispatch({ selection: { anchor: range.anchor, head: range.head } });
  return runCommand(view, name, argument, range);
}

// Keep the selection for a link or note field, and say where the field
// goes, relative to the element `box` names. Nothing when nothing is
// selected.
function hold(box) {
  if (!view) return [];
  const range = view.state.selection.main;
  const frame = document.querySelector(box);
  if (range.empty || !frame) return [];
  held = range;
  const at = view.coordsAtPos(range.from);
  const end = view.coordsAtPos(range.to) || at;
  if (!at) return [];
  const outer = frame.getBoundingClientRect();
  return [at.left - outer.left, end.bottom - outer.top];
}

// The field closed without acting: the selection comes back.
function release() {
  if (!view) return false;
  const range = held;
  held = null;
  view.focus();
  if (range) view.dispatch({ selection: { anchor: range.anchor, head: range.head } });
  return true;
}

function pasted(from, to, text) {
  return view ? insertPasted(view, from, to, text) : false;
}

window.wsEditor = {
  mount,
  unmount,
  getText,
  setText,
  setEditable,
  setOptions,
  getSelection,
  setSelection,
  command,
  hold,
  release,
  pasted,
};
