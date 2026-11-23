// The page's Markdown editor: CodeMirror 6, bundled into one file that the
// program carries (crates/ui/assets/codemirror.js), so nothing is loaded from
// the network. To rebuild it after changing this file or a version in
// package.json: `npm ci` then `npm run build`, here. Commit the bundle and
// LICENSES.txt with the change.
//
// The program talks to it through `window.wsEditor`, below: it mounts the
// editor with a scene's text, and reads or replaces the text. What the page
// does on each change is the program's, through `onChange`.

import { EditorState } from "@codemirror/state";
import { EditorView, keymap, drawSelection } from "@codemirror/view";
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { searchKeymap } from "@codemirror/search";
import { LanguageSupport } from "@codemirror/language";
import { markdownLanguage } from "@codemirror/lang-markdown";

let view = null;
let extensions = [];

function mount(parent, text, onChange) {
  unmount();
  extensions = [
    history(),
    drawSelection(),
    EditorView.lineWrapping,
    // Markdown alone: an HTML tag such as <u> is one node, not parsed as HTML,
    // which keeps the HTML, CSS, and JavaScript parsers out of the bundle.
    new LanguageSupport(markdownLanguage),
    keymap.of([...defaultKeymap, ...historyKeymap, ...searchKeymap]),
    EditorView.updateListener.of((update) => {
      if (update.docChanged && onChange) onChange(update.state.doc.toString());
    }),
  ];
  view = new EditorView({ state: EditorState.create({ doc: text, extensions }), parent });
  return view;
}

function unmount() {
  if (view) {
    view.destroy();
    view = null;
    extensions = [];
  }
}

function getText() {
  return view ? view.state.doc.toString() : "";
}

// Replace the whole text, as when another scene opens. Not undoable: the
// history starts again with the new scene.
function setText(text) {
  if (!view) return false;
  view.setState(EditorState.create({ doc: text, extensions }));
  return true;
}

window.wsEditor = { mount, unmount, getText, setText };
