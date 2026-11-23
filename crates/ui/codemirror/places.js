// Places on the Markdown page: the nth note or highlight, as the review
// panel counts them, and the nth block, as Navigate and tracked changes
// count them; the unknown words underlined; and keeping one's place.

import { EditorSelection, StateEffect, StateField } from "@codemirror/state";
import { Decoration, EditorView, ViewPlugin } from "@codemirror/view";
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

// --- Spelling ------------------------------------------------------------------

// The program decides which words are unknown and sends the list; the page
// underlines them. Not in Markdown marks, addresses, or a note's text, and
// not the word the caret is in: a word is not wrong while it is typed.
const setWords = StateEffect.define();
const spellWords = StateField.define({
  create: () => new Set(),
  update(words, tr) {
    for (const effect of tr.effects) if (effect.is(setWords)) return effect.value;
    return words;
  },
});
const NOT_TEXT = new Set([
  "URL", "LinkMark", "HTMLTag", "HTMLBlock", "CriticMark", "CriticColor", "CriticNoteText",
  "FootnoteRef", "FootnoteLabel", "Entity", "Escape", "CodeMark", "CodeInfo", "HeaderMark",
  "QuoteMark", "ListMark", "EmphasisMark", "StrikethroughMark", "UnderlineMark", "HighlightMark",
]);
// Words as the program splits them: letters and digits, with apostrophes
// allowed inside.
const WORD = /[\p{Alphabetic}\p{N}]+(?:['’][\p{Alphabetic}\p{N}]+)*/gu;
const misspelled = Decoration.mark({ class: "ws-spell" });

function isText(state, pos) {
  for (let node = syntaxTree(state).resolveInner(pos, 1); node; node = node.parent) {
    if (NOT_TEXT.has(node.name)) return false;
  }
  return true;
}

// Every place one of `words` is written as text, in `from`..`to`.
function wordsIn(state, words, from, to) {
  const found = [];
  if (!words.size) return found;
  for (let pos = from; pos <= to; ) {
    const line = state.doc.lineAt(pos);
    WORD.lastIndex = 0;
    let match;
    while ((match = WORD.exec(line.text))) {
      const start = line.from + match.index;
      if (words.has(match[0].replace(/’/g, "'")) && isText(state, start)) {
        found.push({ from: start, to: start + match[0].length });
      }
    }
    pos = line.to + 1;
  }
  return found;
}

const spellPlugin = ViewPlugin.fromClass(
  class {
    constructor(view) {
      this.decorations = this.build(view);
    }
    update(update) {
      const wordsChanged = update.startState.field(spellWords) !== update.state.field(spellWords);
      if (update.docChanged || update.viewportChanged || update.selectionSet || wordsChanged) {
        this.decorations = this.build(update.view);
      }
    }
    build(view) {
      const { state } = view;
      const caret = state.selection.main;
      const { from, to } = view.viewport;
      const marks = wordsIn(state, state.field(spellWords), from, to)
        .filter((hit) => !(caret.empty && caret.head >= hit.from && caret.head <= hit.to))
        .map((hit) => misspelled.range(hit.from, hit.to));
      return Decoration.set(marks, true);
    }
  },
  { decorations: (plugin) => plugin.decorations }
);

export const spelling = [spellWords, spellPlugin];

export function setSpelling(view, words) {
  view.dispatch({ effects: setWords.of(new Set(words)) });
  return true;
}

// Select the next place `word` is written after the selection, going round
// to the top, and show it.
export function findWord(view, word) {
  const hits = wordsIn(view.state, new Set([word]), 0, view.state.doc.length);
  if (!hits.length) return false;
  const after = view.state.selection.main.to;
  const next = hits.find((hit) => hit.from >= after) || hits[0];
  return show(view, next.from, next.to);
}

// --- Keeping one's place ---------------------------------------------------------

// The sentence around `offset` in `text`: from just after the last sentence
// end before it to just after the next one, as the rich-text page found it.
function sentenceAround(text, offset) {
  const ends = /[.!?…]+["'”’)\]]*\s+/g;
  let start = 0;
  let end = text.length;
  let match;
  while ((match = ends.exec(text)) !== null) {
    const after = match.index + match[0].length;
    if (after <= offset) {
      start = after;
    } else {
      end = match.index + match[0].trimEnd().length;
      break;
    }
  }
  return [start, end];
}

const current = Decoration.mark({ class: "ws-current" });

// Fading: everything but the caret's sentence or paragraph is faint.
function fadePlugin(by) {
  return ViewPlugin.fromClass(
    class {
      constructor(view) {
        this.decorations = this.build(view);
      }
      update(update) {
        if (update.docChanged || update.selectionSet || update.focusChanged) this.decorations = this.build(update.view);
      }
      build(view) {
        const { state } = view;
        if (!view.hasFocus) return Decoration.none;
        const head = state.selection.main.head;
        const block = blocks(state).find((node) => node.from <= head && node.to >= head);
        if (!block || block.from === block.to) return Decoration.none;
        if (by === "paragraph") return Decoration.set([current.range(block.from, block.to)]);
        const text = state.doc.sliceString(block.from, block.to);
        const [start, end] = sentenceAround(text, head - block.from);
        return start < end ? Decoration.set([current.range(block.from + start, block.from + end)]) : Decoration.none;
      }
    },
    { decorations: (plugin) => plugin.decorations }
  );
}

// Fixed scrolling, the line marker, and the bars stepping back while one
// types. The desk scrolls, as it did for the rich-text page; the marker is an
// element on the desk, outside the editor.
function placePlugin(options) {
  return ViewPlugin.fromClass(
    class {
      constructor(view) {
        this.view = view;
        this.desk = view.dom.closest(".desk");
        this.marker = null;
        if (options.marker && this.desk) {
          this.marker = document.createElement("div");
          this.marker.className = "line-marker";
          this.marker.setAttribute("aria-hidden", "true");
          this.marker.style.display = "none";
          this.desk.appendChild(this.marker);
        }
        const body = document.body;
        let moved = 0;
        this.typed = (event) => {
          if (event.ctrlKey || event.altKey || event.metaKey) {
            delete body.dataset.typing;
            return;
          }
          if (event.key.length === 1 || ["Enter", "Backspace", "Delete"].includes(event.key)) {
            body.dataset.typing = "1";
            moved = 0;
          }
        };
        this.pointer = (event) => {
          if (!body.dataset.typing) return;
          // A hand resting on the mouse jitters; only a real move brings the bars back.
          moved += Math.abs(event.movementX) + Math.abs(event.movementY);
          if (moved > 8) delete body.dataset.typing;
        };
        this.chord = (event) => {
          if (event.ctrlKey || event.altKey || event.metaKey) delete body.dataset.typing;
        };
        this.left = () => delete body.dataset.typing;
        if (options.bars) {
          view.contentDOM.addEventListener("keydown", this.typed);
          view.contentDOM.addEventListener("blur", this.left);
          document.addEventListener("mousemove", this.pointer);
          document.addEventListener("keydown", this.chord);
        }
        this.place(view);
      }
      update(update) {
        if (update.docChanged || update.selectionSet || update.geometryChanged || update.focusChanged) this.place(update.view);
      }
      place(view) {
        view.requestMeasure({
          read: () => {
            if (!this.desk || !view.hasFocus) return null;
            const head = view.state.selection.main.head;
            const caret = view.coordsAtPos(head, -1) || view.coordsAtPos(head, 1);
            const page = view.dom.closest(".manuscript");
            if (!caret || !page) return null;
            return { caret, desk: this.desk.getBoundingClientRect(), page: page.getBoundingClientRect() };
          },
          write: (at) => {
            if (!at) {
              if (this.marker) this.marker.style.display = "none";
              return;
            }
            const { caret, desk, page } = at;
            let shift = 0;
            if (options.scroll && options.scroll !== "off") {
              const share = options.scroll === "top" ? 0.25 : options.scroll === "bottom" ? 0.7 : 0.45;
              const by = caret.top - (desk.top + this.desk.clientHeight * share);
              if (Math.abs(by) > 2) {
                const before = this.desk.scrollTop;
                this.desk.scrollTop += by;
                shift = this.desk.scrollTop - before;
              }
            }
            if (this.marker) {
              this.marker.style.display = "block";
              this.marker.style.top = caret.top - shift - desk.top + this.desk.scrollTop - 2 + "px";
              this.marker.style.height = caret.bottom - caret.top + 4 + "px";
              this.marker.style.left = page.left - desk.left + this.desk.scrollLeft + "px";
              this.marker.style.width = page.width + "px";
            }
          },
        });
      }
      destroy() {
        if (this.marker) this.marker.remove();
        this.view.contentDOM.removeEventListener("keydown", this.typed);
        this.view.contentDOM.removeEventListener("blur", this.left);
        document.removeEventListener("mousemove", this.pointer);
        document.removeEventListener("keydown", this.chord);
        delete document.body.dataset.typing;
      }
    }
  );
}

// The extensions for these choices: `{ scroll, marker, fade, bars }`, as the
// program's Settings give them.
export function keepingPlace(options) {
  const chosen = options || {};
  const out = [placePlugin(chosen)];
  if (chosen.fade && chosen.fade !== "off") out.push(fadePlugin(chosen.fade));
  return out;
}
