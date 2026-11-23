// Finding places on the Markdown page for the program: the nth note or
// highlight, as the review panel counts them, and the nth block, as
// Navigate and tracked changes count them.

import { EditorSelection } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import { syntaxTree } from "@codemirror/language";
import { criticParts } from "./writing.js";

// The top-level blocks, in the order the scene's blocks are in. A footnote's
// line is not a block of the scene: it becomes a note.
function blocks(state) {
  const out = [];
  const top = syntaxTree(state).topNode;
  for (let node = top.firstChild; node; node = node.nextSibling) {
    if (node.name !== "FootnoteDefinition") out.push(node);
  }
  return out;
}

// Notes and highlights in the order the review panel lists them. Pieces in
// one block next to each other with the same note, or the same color, are
// one item, as the document groups them.
function reviewItems(state, type) {
  const items = [];
  let last = null;
  blocks(state).forEach((block, blockIndex) => {
    const cursor = block.cursor();
    const end = block.to;
    do {
      const node = cursor.node;
      if (node.from > end) break;
      let key = null;
      let from = 0;
      let to = 0;
      let after = 0;
      if (node.name === "CriticHighlight") {
        const part = criticParts(state, node);
        from = part.innerFrom;
        to = part.innerTo;
        after = part.end;
        if (type === "note") key = part.note ? part.note.raw : null;
        else key = part.color ? part.color.value : part.note ? null : "yellow";
      } else if (node.name === "Highlight" && type === "highlight") {
        key = "yellow";
        from = node.from + 2;
        to = node.to - 2;
        after = node.to;
      } else {
        continue;
      }
      if (key === null) continue;
      if (last && last.key === key && last.block === blockIndex && last.after === node.from) {
        last.to = to;
        last.after = after;
      } else {
        last = { key, block: blockIndex, from, to, after };
        items.push(last);
      }
    } while (cursor.next());
  });
  return items;
}

function show(view, from, to) {
  view.dispatch({
    selection: EditorSelection.range(from, to),
    effects: EditorView.scrollIntoView(from, { y: "center" }),
  });
  view.focus();
  return true;
}

export function selectReview(view, type, index) {
  const item = reviewItems(view.state, type)[index];
  return item ? show(view, item.from, item.to) : false;
}

// The nth block, or the last one when a removed block's place is past the
// end.
export function showBlock(view, index) {
  const list = blocks(view.state);
  if (!list.length) return false;
  const block = list[Math.min(index, list.length - 1)];
  return show(view, block.from, block.to);
}
