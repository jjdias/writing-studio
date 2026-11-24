// Live preview: the text is styled from the syntax tree (bold is bold, a
// heading is large, a quote has its rule), and outside Source mode the
// Markdown marks are hidden, except in the element the caret is in: an
// inline element while the selection touches it, a heading's `#`, a
// quote's `>`, or a list's marker while the caret is on that line. The
// caret steps over what is hidden. In Source mode every mark shows, quietly.

import { Facet, RangeSet, StateEffect, StateField } from "@codemirror/state";
import { Decoration, EditorView, ViewPlugin, WidgetType } from "@codemirror/view";
import { syntaxTree } from "@codemirror/language";
import { footnoteTexts, readNote } from "./syntax.js";

// True when every mark is shown as typed.
export const sourceMode = Facet.define({ combine: (values) => values.some(Boolean) });

// The notes a `[[link]]` can name: lowercase title to its tip.
export const setNotes = StateEffect.define();
export const knownNotes = StateField.define({
  create: () => new Map(),
  update(notes, tr) {
    for (const effect of tr.effects) if (effect.is(setNotes)) return effect.value;
    return notes;
  },
});

// The title and the words of a `[[…]]`, from its text.
export function wikiParts(inside) {
  const bar = inside.indexOf("|");
  const title = (bar < 0 ? inside : inside.slice(0, bar)).trim();
  const words = bar < 0 ? title : inside.slice(bar + 1).trim();
  return { title, words, bar };
}

class TextWidget extends WidgetType {
  constructor(text, className, title) {
    super();
    this.text = text;
    this.className = className;
    this.title = title || "";
  }
  eq(other) {
    return other.text === this.text && other.className === this.className && other.title === this.title;
  }
  toDOM() {
    const span = document.createElement("span");
    span.className = this.className;
    span.textContent = this.text;
    if (this.title) span.title = this.title;
    return span;
  }
  ignoreEvent() {
    return false;
  }
}

const hidden = Decoration.replace({});
const markClass = (className, attributes) => Decoration.mark({ class: className, attributes });
const lineClass = (className) => Decoration.line({ class: className });
const quietMark = markClass("cm-md-mark");

const INLINE_STYLES = {
  Emphasis: "cm-em",
  StrongEmphasis: "cm-strong",
  Strikethrough: "cm-strike",
  Underline: "cm-underline",
  Highlight: "ws-hl ws-hl-yellow",
};
const INLINE_MARKS = new Set(["EmphasisMark", "StrikethroughMark", "UnderlineMark", "HighlightMark", "CodeMark"]);

function decodeEntity(text) {
  const area = document.createElement("textarea");
  area.innerHTML = text;
  return area.value;
}

// The suffixes that follow a `{==text==}`: its color and its note, when
// they come right after it.
function criticGroup(node, doc) {
  let end = node.to;
  let color = null;
  let note = null;
  const suffixes = [];
  for (let next = node.nextSibling; next && next.from === end; next = next.nextSibling) {
    if (next.name === "CriticColor" && !color && !note) {
      color = doc.sliceString(next.from + 2, next.to - 1);
    } else if (next.name === "CriticNote" && !note) {
      note = readNote(doc.sliceString(next.from + 3, next.to - 3));
    } else {
      break;
    }
    suffixes.push({ from: next.from, to: next.to, name: next.name });
    end = next.to;
  }
  return { color, note, suffixes, end };
}

function build(view) {
  const { state } = view;
  const doc = state.doc;
  const live = !state.facet(sourceMode);
  const ranges = state.selection.ranges;
  const touches = (from, to) => ranges.some((range) => range.from <= to && range.to >= from);
  const activeLines = new Set();
  for (const range of ranges) {
    for (let line = doc.lineAt(range.from).number; line <= doc.lineAt(range.to).number; line++) activeLines.add(line);
  }
  const lineActive = (pos) => activeLines.has(doc.lineAt(pos).number);
  const decorations = [];
  const atomic = [];
  const add = (decoration, from, to) => {
    if (from < to || decoration.spec.widget) decorations.push(decoration.range(from, to));
  };
  const hide = (from, to) => {
    if (from >= to) return;
    decorations.push(hidden.range(from, to));
    atomic.push(hidden.range(from, to));
  };
  const replace = (from, to, widget) => {
    const decoration = Decoration.replace({ widget });
    decorations.push(decoration.range(from, to));
    atomic.push(decoration.range(from, to));
  };
  const onLines = (from, to, className) => {
    const first = doc.lineAt(from).number;
    const last = doc.lineAt(Math.max(from, to - 1)).number;
    for (let line = first; line <= last; line++) decorations.push(lineClass(className).range(doc.line(line).from));
  };
  // The mark and the space after it, as a heading's `# ` or a quote's `> `.
  const withSpace = (to) => (doc.sliceString(to, to + 1) === " " ? to + 1 : to);
  let footnotes = null;

  const { from, to } = view.viewport;
  // A blank line between paragraphs is the space the page settings give.
  for (let pos = from; pos <= to; ) {
    const line = doc.lineAt(pos);
    if (line.length === 0) decorations.push(lineClass("cm-blank").range(line.from));
    pos = line.to + 1;
  }
  syntaxTree(state).iterate({
    from,
    to,
    enter: (node) => {
      const name = node.name;
      const heading = /^(ATX|Setext)Heading(\d)$/.exec(name);
      if (heading) {
        onLines(node.from, node.to, `cm-h cm-h${heading[2]}`);
        return;
      }
      switch (name) {
        case "Paragraph":
          // The first-line indent is for the story's paragraphs, not a
          // list item's or a quote's.
          if (node.node.parent && node.node.parent.name === "Document") {
            decorations.push(lineClass("cm-p").range(doc.lineAt(node.from).from));
          }
          return;
        case "HeaderMark":
          if (live && !lineActive(node.from)) hide(node.from, withSpace(node.to));
          else add(quietMark, node.from, node.to);
          return;
        case "Blockquote":
          onLines(node.from, node.to, "cm-quote");
          return;
        case "QuoteMark":
          if (live && !lineActive(node.from)) hide(node.from, withSpace(node.to));
          else add(quietMark, node.from, node.to);
          return;
        case "ListItem":
          decorations.push(lineClass("cm-li").range(doc.lineAt(node.from).from));
          return;
        case "ListMark": {
          const bullet = /^[-+*]$/.test(doc.sliceString(node.from, node.to));
          if (bullet && live && !lineActive(node.from)) replace(node.from, node.to, new TextWidget("•", "cm-bullet"));
          else add(markClass("cm-list-mark"), node.from, node.to);
          return;
        }
        case "HorizontalRule":
          onLines(node.from, node.to, "cm-scene-break");
          if (live && !lineActive(node.from)) replace(node.from, node.to, new TextWidget("* * *", "cm-scene-break-mark"));
          return;
        case "FencedCode":
        case "CodeBlock":
          onLines(node.from, node.to, "cm-codeblock");
          return;
        case "CodeInfo":
          add(quietMark, node.from, node.to);
          return;
        case "HTMLBlock":
          // `<br>` alone on its line is an empty paragraph.
          if (live && !lineActive(node.from) && /^<br\s*\/?>$/i.test(doc.sliceString(node.from, node.to).trim())) {
            hide(node.from, node.to);
          } else {
            add(quietMark, node.from, node.to);
          }
          return false;
        case "HTMLTag": {
          const tag = doc.sliceString(node.from, node.to);
          if (live && /^<br\s*\/?>$/i.test(tag) && !touches(node.from, node.to)) {
            replace(node.from, node.to, new TextWidget("↵", "cm-br", "Line break"));
          } else {
            add(quietMark, node.from, node.to);
          }
          return;
        }
        case "Escape":
          if (live && !touches(node.from, node.to)) hide(node.from, node.from + 1);
          return;
        case "HardBreak":
          // A line break inside a paragraph: the backslash at the line's end.
          if (live && !lineActive(node.from)) hide(node.from, Math.min(node.to, doc.lineAt(node.from).to));
          else add(quietMark, node.from, Math.min(node.to, doc.lineAt(node.from).to));
          return;
        case "Entity":
          if (live && !touches(node.from, node.to)) {
            replace(node.from, node.to, new TextWidget(decodeEntity(doc.sliceString(node.from, node.to)), "cm-entity"));
          }
          return;
        case "Link": {
          // `[text](address)`: the text is the link, the rest its marks.
          const open = node.node.getChild("LinkMark");
          const marks = node.node.getChildren("LinkMark");
          const url = node.node.getChild("URL");
          const textEnd = marks.length > 1 ? marks[1].from : node.to;
          add(markClass("cm-link", url ? { title: doc.sliceString(url.from, url.to) } : undefined), open ? open.to : node.from, textEnd);
          const active = touches(node.from, node.to);
          if (live && !active && open && marks.length > 1) {
            hide(open.from, open.to);
            hide(marks[1].from, node.to);
          } else {
            for (const mark of marks) add(quietMark, mark.from, mark.to);
            if (url) add(markClass("cm-md-mark cm-url"), url.from, url.to);
          }
          return;
        }
        case "URL":
          // A link's address is taken care of with its link.
          if (node.node.parent && node.node.parent.name === "Link") return;
          add(markClass("cm-link"), node.from, node.to);
          return;
        case "Autolink":
          add(markClass("cm-link"), node.from, node.to);
          return false;
        case "CriticHighlight": {
          const group = criticGroup(node.node, doc);
          const active = touches(node.from, group.end);
          const open = node.node.firstChild;
          const close = node.node.lastChild;
          const inner = [open ? open.to : node.from, close ? close.from : node.to];
          if (group.color || !group.note) add(markClass(`ws-hl ws-hl-${group.color || "yellow"}`), inner[0], inner[1]);
          if (group.note) {
            const { kind, resolved, text } = group.note;
            add(markClass(`ws-note ws-note-${kind}${resolved ? " ws-note-resolved" : ""}`, { title: text }), inner[0], inner[1]);
          }
          const marks = [open, close].filter((mark) => mark && mark.name === "CriticMark");
          for (const mark of marks) {
            if (live && !active) hide(mark.from, mark.to);
            else add(quietMark, mark.from, mark.to);
          }
          for (const suffix of group.suffixes) {
            if (live && !active) {
              if (suffix.name === "CriticNote") {
                const { kind, resolved, text } = group.note;
                const pin = `ws-note-pin ws-note-${kind}${resolved ? " ws-note-resolved" : ""}`;
                replace(suffix.from, suffix.to, new TextWidget("✎", pin, text));
              } else {
                hide(suffix.from, suffix.to);
              }
            } else {
              add(markClass(suffix.name === "CriticNote" ? "cm-md-mark ws-note-text" : "cm-md-mark"), suffix.from, suffix.to);
            }
          }
          return;
        }
        case "CriticNote":
        case "CriticColor":
          // Taken care of with the highlight they follow; alone, they are
          // shown as typed.
          return false;
        case "WikiLink": {
          const inside = doc.sliceString(node.from + 2, node.to - 2);
          const { title, bar } = wikiParts(inside);
          const notes = state.field(knownNotes, false) || new Map();
          const tip = notes.get(title.toLowerCase());
          const known = tip !== undefined;
          const shownFrom = bar < 0 ? node.from + 2 : node.from + 3 + bar;
          const attributes = { "data-note": title, title: known ? tip || title : `No note is called “${title}” yet` };
          const className = known ? "ws-wikilink" : "ws-wikilink ws-wikilink-missing";
          if (live && !touches(node.from, node.to)) {
            hide(node.from, shownFrom);
            add(markClass(className, attributes), shownFrom, node.to - 2);
            hide(node.to - 2, node.to);
          } else {
            add(quietMark, node.from, node.from + 2);
            add(markClass(className, attributes), node.from + 2, node.to - 2);
            add(quietMark, node.to - 2, node.to);
          }
          return false;
        }
        case "FootnoteRef": {
          footnotes = footnotes || footnoteTexts(doc.toString());
          const label = doc.sliceString(node.from + 2, node.to - 1);
          const text = footnotes.get(label) || "No text for this footnote yet";
          if (live && !touches(node.from, node.to)) replace(node.from, node.to, new TextWidget(label, "ws-footnote-ref", text));
          else add(markClass("cm-md-mark ws-footnote-ref-source", { title: text }), node.from, node.to);
          return;
        }
        case "FootnoteDefinition":
          onLines(node.from, node.to, "ws-footnote-def");
          return;
        case "FootnoteLabel":
          add(quietMark, node.from, node.to);
          return;
        default:
          break;
      }
      if (INLINE_STYLES[name]) {
        add(markClass(INLINE_STYLES[name]), node.from, node.to);
        return;
      }
      if (INLINE_MARKS.has(name)) {
        const parent = node.node.parent;
        // A fence's backticks are a block's: always shown, quietly.
        if (name === "CodeMark") {
          add(quietMark, node.from, node.to);
          return;
        }
        if (live && parent && !touches(parent.from, parent.to)) hide(node.from, node.to);
        else add(quietMark, node.from, node.to);
      }
    },
  });
  return {
    decorations: Decoration.set(decorations, true),
    atomic: RangeSet.of(atomic, true),
  };
}

export const preview = ViewPlugin.fromClass(
  class {
    constructor(view) {
      const built = build(view);
      this.decorations = built.decorations;
      this.atomic = built.atomic;
    }
    update(update) {
      const changed =
        update.docChanged ||
        update.viewportChanged ||
        update.selectionSet ||
        update.startState.facet(sourceMode) !== update.state.facet(sourceMode) ||
        syntaxTree(update.startState) !== syntaxTree(update.state);
      if (!changed) return;
      const built = build(update.view);
      this.decorations = built.decorations;
      this.atomic = built.atomic;
    }
  },
  {
    decorations: (plugin) => plugin.decorations,
    provide: (plugin) =>
      EditorView.atomicRanges.of((view) => {
        const value = view.plugin(plugin);
        return value ? value.atomic : RangeSet.empty;
      }),
  }
);
