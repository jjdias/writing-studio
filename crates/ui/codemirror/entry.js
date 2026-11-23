// The page's Markdown editor: CodeMirror 6, bundled into one file that the
// program carries (crates/ui/assets/codemirror.js), so nothing is loaded from
// the network. To rebuild it after changing a file here or a version in
// package.json: `npm ci` then `npm run build`, here. Commit the bundle and
// LICENSES.txt with the change.
//
// The program talks to it through `window.wsEditor`, below. It mounts one
// editor for the open scene, and the editor sends it messages through
// `send`: `{ kind: "text", text }` after every change, and
// `{ kind: "blur" }` when the page loses the focus.

import { EditorState, Compartment, Annotation, Transaction } from "@codemirror/state";
import { EditorView, keymap, drawSelection } from "@codemirror/view";
import { defaultKeymap, history, undo, redo } from "@codemirror/commands";
import { LanguageSupport } from "@codemirror/language";
import { markdownLanguage } from "@codemirror/lang-markdown";

let view = null;
let token = null;
let extensions = [];
const editable = new Compartment();
const undoHistory = new Compartment();
// Marks a change the program made, which is not sent back as the writer's.
const fromProgram = Annotation.define();

// Ctrl+Z and Ctrl+Y go to the editor's own history while it has something
// to undo. When it has nothing, as just after a scene was split or a note
// resolved, the key goes on to the program, whose undo takes back those.
function historyKey(command) {
  return (target) => command(target);
}

// A key the editor acted on is not also a shortcut of the program's: the
// program's own keys (Ctrl+S, Ctrl+1, Ctrl+F) pass through untouched.
function keepHandledKeys(element) {
  element.addEventListener("keydown", (event) => {
    if (event.defaultPrevented && (event.ctrlKey || event.metaKey || event.altKey)) {
      event.stopPropagation();
    }
  });
}

function mount(parent, text, send, id) {
  unmount();
  token = id;
  extensions = [
    undoHistory.of(history()),
    drawSelection(),
    EditorView.lineWrapping,
    EditorView.contentAttributes.of({ "aria-label": "Manuscript", spellcheck: "false" }),
    editable.of(EditorView.editable.of(true)),
    // Markdown alone: an HTML tag such as <u> is one node, not parsed as HTML,
    // which keeps the HTML, CSS, and JavaScript parsers out of the bundle.
    new LanguageSupport(markdownLanguage),
    keymap.of([
      { key: "Mod-z", run: historyKey(undo), preventDefault: false },
      { key: "Mod-y", run: historyKey(redo), preventDefault: false },
      { key: "Mod-Shift-z", run: historyKey(redo), preventDefault: false },
      ...defaultKeymap,
    ]),
    EditorView.updateListener.of((update) => {
      if (update.docChanged && update.transactions.some((tr) => !tr.annotation(fromProgram))) {
        send({ kind: "text", text: update.state.doc.toString() });
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

window.wsEditor = { mount, unmount, getText, setText, setEditable };
