// Writing in Markdown: the commands the keys and the format toolbar run,
// lists and quotes that continue, pairing, smart typography, paste, and
// the marks at the caret that the toolbar shows. Each command edits the
// Markdown text; the program reads the result back like any typing.

import { EditorSelection, EditorState, Prec } from "@codemirror/state";
import { EditorView, keymap } from "@codemirror/view";
import { syntaxTree } from "@codemirror/language";
import { closeBrackets, closeBracketsKeymap } from "@codemirror/autocomplete";
import { insertNewlineContinueMarkup, deleteMarkupBackward } from "@codemirror/lang-markdown";

// --- Finding what the caret is in -------------------------------------------

// The innermost node named `name` around `from`..`to`, if any.
function around(state, from, to, name) {
  for (let node = syntaxTree(state).resolveInner(from, 1); node; node = node.parent) {
    if (node.name === name && node.from <= from && node.to >= to) return node;
  }
  for (let node = syntaxTree(state).resolveInner(from, -1); node; node = node.parent) {
    if (node.name === name && node.from <= from && node.to >= to) return node;
  }
  return null;
}

// A `{==text==}` and what follows it: `{.color}` and `{>>note<<}`.
export function criticAt(state, from, to) {
  const node = around(state, from, to, "CriticHighlight");
  return node ? criticParts(state, node) : null;
}

export function criticParts(state, node) {
  const doc = state.doc;
  const part = { from: node.from, innerFrom: node.from + 3, innerTo: node.to - 3, to: node.to, color: null, note: null };
  let end = node.to;
  for (let next = node.nextSibling; next && next.from === end; next = next.nextSibling) {
    if (next.name === "CriticColor" && !part.color && !part.note) {
      part.color = { from: next.from, to: next.to, value: doc.sliceString(next.from + 2, next.to - 1) };
    } else if (next.name === "CriticNote" && !part.note) {
      part.note = { from: next.from, to: next.to, raw: doc.sliceString(next.from + 3, next.to - 3) };
    } else {
      break;
    }
    end = next.to;
  }
  part.end = end;
  return part;
}

// The selection without the spaces at its edges: `** word**` is not bold.
function trimmed(state, range) {
  let { from, to } = range;
  while (from < to && /\s/.test(state.doc.sliceString(from, from + 1))) from++;
  while (to > from && /\s/.test(state.doc.sliceString(to - 1, to))) to--;
  return { from, to };
}

// --- Inline marks ------------------------------------------------------------

const INLINE = {
  bold: { node: "StrongEmphasis", open: "**", close: "**" },
  italic: { node: "Emphasis", open: "*", close: "*" },
  strike: { node: "Strikethrough", open: "~~", close: "~~" },
  underline: { node: "Underline", open: "<u>", close: "</u>" },
};

// Wrap the selection in a mark, or take the mark off what the caret is in.
// With nothing selected, the marks go in with the caret between them.
function toggleInline(view, name) {
  const spec = INLINE[name];
  const { state } = view;
  const range = state.selection.main;
  const inside = around(state, range.from, range.to, spec.node);
  if (inside) {
    // The selection follows the text as the marks come off.
    view.dispatch({
      changes: [
        { from: inside.from, to: inside.from + spec.open.length },
        { from: inside.to - spec.close.length, to: inside.to },
      ],
      userEvent: "input.format",
    });
    return true;
  }
  if (range.empty) {
    view.dispatch({
      changes: { from: range.from, insert: spec.open + spec.close },
      selection: EditorSelection.cursor(range.from + spec.open.length),
      userEvent: "input.format",
    });
    return true;
  }
  const { from, to } = trimmed(state, range);
  view.dispatch({
    changes: [
      { from, insert: spec.open },
      { from: to, insert: spec.close },
    ],
    selection: EditorSelection.range(from + spec.open.length, to + spec.open.length),
    userEvent: "input.format",
  });
  return true;
}

// --- Highlights and notes ----------------------------------------------------

const HIGHLIGHTS = ["yellow", "green", "blue", "pink"];

// The note's text as written: its kind first unless it is a comment, and
// the characters CriticMarkup uses escaped, as crates/format writes them.
export function noteBody(kind, text) {
  const prefix = { question: "Question: ", revise: "Revise: ", check: "Check: " }[kind] || "";
  return prefix + text.replace(/[\\<>{}]/g, (ch) => "\\" + ch);
}

// The suffix a group gets: its color (yellow is left out unless there is a
// note) and its note.
function suffix(color, note) {
  let out = "";
  if (color && (color !== "yellow" || note !== null)) out += `{.${color}}`;
  if (note !== null) out += `{>>${note}<<}`;
  return out;
}

// Change what follows a `{==text==}`, or take the group off when it would
// be neither a highlight nor a note.
function rewriteGroup(view, part, color, note) {
  const changes =
    color === null && note === null
      ? [
          { from: part.from, to: part.innerFrom },
          { from: part.innerTo, to: part.end },
        ]
      : [{ from: part.to, to: part.end, insert: suffix(color, note) }];
  view.dispatch({ changes, userEvent: "input.format" });
  return true;
}

function plainHighlightAt(state, from, to) {
  return around(state, from, to, "Highlight");
}

function highlight(view, color) {
  if (!HIGHLIGHTS.includes(color)) return false;
  const { state } = view;
  const range = state.selection.main;
  const part = criticAt(state, range.from, range.to);
  if (part) return rewriteGroup(view, part, color, part.note ? part.note.raw : null);
  const plain = plainHighlightAt(state, range.from, range.to);
  if (plain) {
    view.dispatch({
      changes: [
        { from: plain.from, to: plain.from + 2, insert: "{==" },
        { from: plain.to - 2, to: plain.to, insert: "==}" + suffix(color, null) },
      ],
      userEvent: "input.format",
    });
    return true;
  }
  if (range.empty) return false;
  const { from, to } = trimmed(state, range);
  view.dispatch({
    changes: [
      { from, insert: "{==" },
      { from: to, insert: "==}" + suffix(color, null) },
    ],
    selection: EditorSelection.cursor(to + 3 + 3 + suffix(color, null).length),
    userEvent: "input.format",
  });
  return true;
}

function clearHighlight(view) {
  const { state } = view;
  const range = state.selection.main;
  const part = criticAt(state, range.from, range.to);
  if (part) {
    // With a note, the text stays noted; without one, the marks go.
    const noted = part.note !== null;
    if (!noted || part.color) return rewriteGroup(view, part, null, noted ? part.note.raw : null);
    return false;
  }
  const plain = plainHighlightAt(state, range.from, range.to);
  if (!plain) return false;
  view.dispatch({
    changes: [
      { from: plain.from, to: plain.from + 2 },
      { from: plain.to - 2, to: plain.to },
    ],
    userEvent: "input.format",
  });
  return true;
}

function addNote(view, kind, text, range) {
  const { state } = view;
  const note = noteBody(kind, text.trim());
  if (!text.trim()) return false;
  const part = criticAt(state, range.from, range.to);
  if (part) {
    const color = part.color ? part.color.value : part.note ? null : "yellow";
    return rewriteGroup(view, part, color, note);
  }
  if (range.empty) return false;
  const { from, to } = trimmed(state, range);
  const after = `==}{>>${note}<<}`;
  view.dispatch({
    changes: [
      { from, insert: "{==" },
      { from: to, insert: after },
    ],
    selection: EditorSelection.cursor(to + 3 + after.length),
    userEvent: "input.format",
  });
  return true;
}

function removeNote(view) {
  const { state } = view;
  const range = state.selection.main;
  const part = criticAt(state, range.from, range.to);
  if (!part || !part.note) return false;
  // A highlight keeps its color; a note alone goes with its marks.
  return rewriteGroup(view, part, part.color ? part.color.value : null, null);
}

// --- Links -------------------------------------------------------------------

// What a link may go to, as the rich-text page took it: a web or mail
// address, with `https://` added when no scheme is given.
export function linkTarget(raw) {
  let url = (raw || "").trim();
  if (!url) return null;
  if (!/^[a-z][a-z0-9+.-]*:/i.test(url)) url = "https://" + url;
  if (!/^(https?:|mailto:)/i.test(url)) return null;
  return /[\s()<>]/.test(url) ? `<${url.replace(/[<>]/g, encodeURIComponent)}>` : url;
}

function addLink(view, raw, range) {
  const target = linkTarget(raw);
  if (!target) return false;
  const { state } = view;
  const link = around(state, range.from, range.to, "Link");
  if (link) {
    const url = link.getChild("URL");
    if (url) {
      view.dispatch({ changes: { from: url.from, to: url.to, insert: target }, userEvent: "input.format" });
      return true;
    }
  }
  if (range.empty) return false;
  const { from, to } = trimmed(state, range);
  const after = `](${target})`;
  view.dispatch({
    changes: [
      { from, insert: "[" },
      { from: to, insert: after },
    ],
    selection: EditorSelection.cursor(to + 1 + after.length),
    userEvent: "input.format",
  });
  return true;
}

function removeLink(view) {
  const { state } = view;
  const range = state.selection.main;
  const link = around(state, range.from, range.to, "Link");
  if (!link) return false;
  const marks = link.getChildren("LinkMark");
  if (marks.length < 2) return false;
  view.dispatch({
    changes: [
      { from: link.from, to: marks[0].to },
      { from: marks[1].from, to: link.to },
    ],
    userEvent: "input.format",
  });
  return true;
}

// Ctrl+K: the selection becomes a link's text with the caret where its
// address goes; with nothing selected, `[]()` with the caret for the text.
function linkKey(view) {
  const { state } = view;
  view.dispatch(
    state.changeByRange((range) => {
      if (range.empty) {
        return { changes: { from: range.from, insert: "[]()" }, range: EditorSelection.cursor(range.from + 1) };
      }
      const { from, to } = trimmed(state, range);
      return {
        changes: [
          { from, insert: "[" },
          { from: to, insert: "]()" },
        ],
        range: EditorSelection.cursor(to + 3),
      };
    }),
    { userEvent: "input.format" }
  );
  return true;
}

// --- Blocks ------------------------------------------------------------------

const LINE_PREFIX = /^(#{1,6}[ \t]+|>[ \t]?|[-+*][ \t]+|\d{1,9}[.)][ \t]+)/;

// Make every line the selection touches a heading, quote, list item, or
// body text. A quote, list, or numbered list already there is taken off.
function setBlock(view, kind) {
  const { state } = view;
  const doc = state.doc;
  const range = state.selection.main;
  if (kind === "code") return toggleCodeBlock(view);
  const first = doc.lineAt(range.from).number;
  const last = doc.lineAt(range.to).number;
  const lines = [];
  for (let n = first; n <= last; n++) lines.push(doc.line(n));
  const has = {
    quote: (text) => /^>/.test(text),
    list: (text) => /^[-+*][ \t]/.test(text),
    numbered: (text) => /^\d{1,9}[.)][ \t]/.test(text),
  };
  const written = lines.filter((line) => line.length > 0);
  const toggleOff = has[kind] && written.length > 0 && written.every((line) => has[kind](line.text));
  let number = 0;
  const changes = [];
  for (const line of lines) {
    if (line.length === 0 && lines.length > 1) continue;
    const old = LINE_PREFIX.exec(line.text);
    let prefix = "";
    if (!toggleOff) {
      if (/^h[1-6]$/.test(kind)) prefix = "#".repeat(Number(kind[1])) + " ";
      else if (kind === "quote") prefix = "> ";
      else if (kind === "list") prefix = "- ";
      else if (kind === "numbered") prefix = `${++number}. `;
    }
    changes.push({ from: line.from, to: line.from + (old ? old[0].length : 0), insert: prefix });
  }
  view.dispatch({ changes, userEvent: "input.format" });
  return true;
}

function toggleCodeBlock(view) {
  const { state } = view;
  const doc = state.doc;
  const range = state.selection.main;
  const fenced = around(state, range.from, range.to, "FencedCode");
  if (fenced) {
    const open = doc.lineAt(fenced.from);
    const close = doc.lineAt(fenced.to);
    const changes = [{ from: open.from, to: Math.min(open.to + 1, doc.length) }];
    if (close.number !== open.number) changes.push({ from: Math.max(close.from - 1, 0), to: close.to });
    view.dispatch({ changes, userEvent: "input.format" });
    return true;
  }
  const first = doc.lineAt(range.from);
  const last = doc.lineAt(range.to);
  view.dispatch({
    changes: [
      { from: first.from, insert: "```\n" },
      { from: last.to, insert: "\n```" },
    ],
    userEvent: "input.format",
  });
  return true;
}

// A scene break after the caret's paragraph, as its own block.
function sceneBreak(view) {
  const { state } = view;
  const line = state.doc.lineAt(state.selection.main.head);
  const insert = line.length === 0 ? "* * *\n\n" : "\n\n* * *\n\n";
  const at = line.to;
  view.dispatch({
    changes: { from: at, insert },
    selection: EditorSelection.cursor(at + insert.length),
    scrollIntoView: true,
    userEvent: "input.format",
  });
  return true;
}

// --- What the toolbar and the review panel ask for ---------------------------

// The program's commands, by the names the rich-text page used for them, so
// the format toolbar is the same for both pages.
export function runCommand(view, name, argument, held) {
  const range = held || view.state.selection.main;
  switch (name) {
    case "bold":
      return toggleInline(view, "bold");
    case "italic":
      return toggleInline(view, "italic");
    case "underline":
      return toggleInline(view, "underline");
    case "strikeThrough":
      return toggleInline(view, "strike");
    case "insertUnorderedList":
      return setBlock(view, "list");
    case "insertOrderedList":
      return setBlock(view, "numbered");
    case "insertHorizontalRule":
      return sceneBreak(view);
    case "formatBlock": {
      const kinds = { P: "body", H1: "h1", H2: "h2", H3: "h3", H4: "h4", H5: "h5", H6: "h6", PRE: "code", BLOCKQUOTE: "quote" };
      return kinds[argument] ? setBlock(view, kinds[argument]) : false;
    }
    case "unlink":
      return removeLink(view);
    case "link":
      return addLink(view, argument, range);
    case "highlight":
      return highlight(view, argument);
    case "clearHighlight":
      return clearHighlight(view);
    case "note":
      return addNote(view, argument.kind, argument.text, range);
    case "uncomment":
      return removeNote(view);
    default:
      return false;
  }
}

// --- The marks at the caret, for the toolbar ----------------------------------

export function marksAt(state) {
  const { head } = state.selection.main;
  const marks = new Set();
  let block = "p";
  for (const side of [-1, 1]) {
    for (let node = syntaxTree(state).resolveInner(head, side); node; node = node.parent) {
      switch (node.name) {
        case "StrongEmphasis":
          marks.add("bold");
          break;
        case "Emphasis":
          marks.add("italic");
          break;
        case "Underline":
          marks.add("underline");
          break;
        case "Strikethrough":
          marks.add("strikeThrough");
          break;
        case "Link":
          marks.add("link");
          break;
        case "Highlight":
          marks.add("highlight");
          break;
        case "CriticHighlight": {
          const part = criticParts(state, node);
          if (part.note) marks.add("note");
          if (part.color || !part.note) marks.add("highlight");
          break;
        }
        case "Blockquote":
          marks.add("quote");
          break;
        case "BulletList":
          marks.add("list");
          break;
        case "OrderedList":
          marks.add("numbered");
          break;
        case "FencedCode":
        case "CodeBlock":
          block = "pre";
          break;
        default: {
          const heading = /^(?:ATX|Setext)Heading(\d)$/.exec(node.name);
          if (heading && block === "p") block = "h" + heading[1];
        }
      }
    }
  }
  return [...marks, "block:" + block];
}

// --- Smart typography --------------------------------------------------------

const VERBATIM = new Set(["FencedCode", "CodeText", "HTMLTag", "HTMLBlock", "URL", "Autolink"]);

function inVerbatim(state, pos) {
  for (let node = syntaxTree(state).resolveInner(pos, -1); node; node = node.parent) {
    if (VERBATIM.has(node.name)) return true;
  }
  return false;
}

// Curly quotes, an em dash for `--`, and an ellipsis for `...`, as typed.
// Not in code, and not where a line of dashes would make a scene break.
export const smartTypography = EditorView.inputHandler.of((view, from, to, text) => {
  if (text.length !== 1 || !/["'\-.]/.test(text)) return false;
  const { state } = view;
  if (inVerbatim(state, from)) return false;
  const before = state.doc.sliceString(Math.max(0, from - 2), from);
  const previous = before.slice(-1);
  const opening = previous === "" || /[\s([{—–-]/.test(previous);
  let insert = null;
  let start = from;
  if (text === '"') insert = opening ? "“" : "”";
  else if (text === "'") insert = opening ? "‘" : "’";
  else if (text === "-" && previous === "-") {
    const line = state.doc.lineAt(from);
    const lineBefore = state.doc.sliceString(line.from, from);
    if (/^[\s-]*$/.test(lineBefore)) return false;
    insert = "—";
    start = from - 1;
  } else if (text === "." && before === "..") {
    insert = "…";
    start = from - 2;
  }
  if (insert === null) return false;
  view.dispatch({
    changes: { from: start, to, insert },
    selection: EditorSelection.cursor(start + insert.length),
    userEvent: "input.type",
  });
  return true;
});

// --- Pairing -----------------------------------------------------------------

// A typed bracket, quote, or Markdown mark gets its closing pair, and
// typing the closing one steps over it.
export const pairing = [
  EditorState.languageData.of(() => [{ closeBrackets: { brackets: ["(", "[", '"', "*", "_", "“"] } }]),
  closeBrackets(),
  keymap.of(closeBracketsKeymap),
];

// --- Paste -------------------------------------------------------------------

// A web address pasted over plain selected text makes it a link. Rich text
// (from a browser or a word processor) is sent to the program, which reads
// it as the rich-text page did and gives back Markdown (`insertPasted`).
export function pasteHandler(send) {
  return EditorView.domEventHandlers({
    paste: (event, view) => {
      const data = event.clipboardData;
      if (!data) return false;
      const plain = data.getData("text/plain");
      const range = view.state.selection.main;
      if (!range.empty && plain && /^(https?:\/\/|mailto:|www\.)\S+$/.test(plain.trim()) && !around(view.state, range.from, range.to, "Link")) {
        event.preventDefault();
        return addLink(view, plain.trim(), range);
      }
      const html = data.getData("text/html");
      if (!html || inVerbatim(view.state, range.from)) return false;
      event.preventDefault();
      send({ kind: "paste", html, from: range.from, to: range.to });
      return true;
    },
  });
}

// Put the program's Markdown for a paste where the paste was asked for.
export function insertPasted(view, from, to, text) {
  const length = view.state.doc.length;
  const start = Math.min(from, length);
  const end = Math.min(Math.max(to, start), length);
  view.dispatch({
    changes: { from: start, to: end, insert: text },
    selection: EditorSelection.cursor(start + text.length),
    scrollIntoView: true,
    userEvent: "input.paste",
  });
  return true;
}

// --- Paragraphs --------------------------------------------------------------

// Enter: in a list or a quote, the next item or quoted line (and on an empty
// one, the end of it); anywhere else, a new paragraph, which in Markdown is a
// blank line. One line end alone would be the same paragraph, wrapped.
function enter(view) {
  const { state } = view;
  const range = state.selection.main;
  for (let node = syntaxTree(state).resolveInner(range.head, -1); node; node = node.parent) {
    if (node.name === "ListItem" || node.name === "Blockquote") return insertNewlineContinueMarkup(view);
    if (node.name === "FencedCode") return false;
  }
  const line = state.doc.lineAt(range.head);
  const before = state.doc.sliceString(line.from, range.from);
  // On a blank line, or before a blank line, one line end is enough.
  const blankAfter = range.to === line.to && line.number < state.doc.lines && state.doc.line(line.number + 1).length === 0;
  const insert = before.trim() === "" || blankAfter ? "\n" : "\n\n";
  view.dispatch({
    changes: { from: range.from, to: range.to, insert },
    selection: EditorSelection.cursor(range.from + insert.length),
    scrollIntoView: true,
    userEvent: "input",
  });
  return true;
}

// Shift+Enter: a line break inside the paragraph, as a backslash at the
// line's end, which is how crates/format writes one.
function lineBreak(view) {
  const { state } = view;
  const range = state.selection.main;
  for (let node = syntaxTree(state).resolveInner(range.head, -1); node; node = node.parent) {
    if (node.name === "FencedCode") return false;
  }
  const line = state.doc.lineAt(range.head);
  const quote = /^>[ \t]?/.exec(line.text);
  const insert = "\\\n" + (quote ? "> " : "");
  view.dispatch({
    changes: { from: range.from, to: range.to, insert },
    selection: EditorSelection.cursor(range.from + insert.length),
    scrollIntoView: true,
    userEvent: "input",
  });
  return true;
}

// --- Keys --------------------------------------------------------------------

export const writingKeys = Prec.high(
  keymap.of([
    { key: "Mod-b", run: (view) => toggleInline(view, "bold") },
    { key: "Mod-i", run: (view) => toggleInline(view, "italic") },
    { key: "Mod-u", run: (view) => toggleInline(view, "underline") },
    { key: "Mod-Shift-x", run: (view) => toggleInline(view, "strike") },
    { key: "Mod-k", run: linkKey },
    { key: "Mod-Alt-0", run: (view) => setBlock(view, "body") },
    { key: "Mod-Alt-1", run: (view) => setBlock(view, "h1") },
    { key: "Mod-Alt-2", run: (view) => setBlock(view, "h2") },
    { key: "Mod-Alt-3", run: (view) => setBlock(view, "h3") },
    { key: "Mod-Alt-4", run: (view) => setBlock(view, "h4") },
    { key: "Mod-Alt-5", run: (view) => setBlock(view, "h5") },
    { key: "Mod-Alt-6", run: (view) => setBlock(view, "h6") },
    { key: "Enter", run: enter },
    { key: "Shift-Enter", run: lineBreak },
    { key: "Backspace", run: deleteMarkupBackward },
  ])
);
