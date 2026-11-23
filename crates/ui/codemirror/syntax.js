// The Markdown the page reads: what crates/format reads, so the page never
// shows formatting the scene would not keep. That is CommonMark with
// strikethrough, without what the scene has no place for (inline code,
// images, setext headings, indented code, and link definitions, which read
// as the text they are), plus what a scene holds that Markdown has no
// common way to say:
//
// - `<u>text</u>` underline, as one node rather than two HTML tags;
// - `==text==` and `{==text==}` highlights, `{.green}` after one for its
//   color, and `{>>note<<}` after one for a note (CriticMarkup);
// - `[^1]` footnote marks, and `[^1]: text` lines that hold their text.

import { tags as t } from "@lezer/highlight";
import { Strikethrough } from "@lezer/markdown";
import { Language } from "@codemirror/language";
import { commonmarkLanguage } from "@codemirror/lang-markdown";

const CriticDelim = { resolve: "CriticHighlight", mark: "CriticMark" };
const HighlightDelim = { resolve: "Highlight", mark: "HighlightMark" };
const UnderlineDelim = { resolve: "Underline", mark: "UnderlineMark" };

// The character code at `at`, or -1 outside the text being read.
function code(cx, at) {
  return at < cx.offset || at >= cx.end ? -1 : cx.char(at);
}

function isSpace(ch) {
  return ch === -1 || ch === 32 || ch === 9 || ch === 10 || ch === 13 || ch === 0xa0;
}

const Scene = {
  defineNodes: [
    { name: "CriticHighlight" },
    { name: "CriticMark", style: t.processingInstruction },
    { name: "CriticColor", style: t.processingInstruction },
    { name: "CriticNote" },
    { name: "CriticNoteText", style: t.comment },
    { name: "Highlight" },
    { name: "HighlightMark", style: t.processingInstruction },
    { name: "Underline" },
    { name: "UnderlineMark", style: t.processingInstruction },
    { name: "FootnoteRef", style: t.labelName },
    { name: "FootnoteDefinition", block: true },
    { name: "FootnoteLabel", style: t.labelName },
  ],
  parseInline: [
    {
      name: "Critic",
      before: "Emphasis",
      parse(cx, next, pos) {
        if (next !== 123 /* { */) return -1;
        const head = cx.slice(pos, Math.min(cx.end, pos + 3));
        if (head === "{==") return cx.addDelimiter(CriticDelim, pos, pos + 3, true, false);
        if (head === "{>>") {
          for (let at = pos + 3; at < cx.end; at++) {
            if (code(cx, at) === 92 /* \ */) {
              at++;
              continue;
            }
            if (cx.slice(at, Math.min(cx.end, at + 3)) === "<<}") {
              return cx.addElement(
                cx.elt("CriticNote", pos, at + 3, [
                  cx.elt("CriticMark", pos, pos + 3),
                  cx.elt("CriticNoteText", pos + 3, at),
                  cx.elt("CriticMark", at, at + 3),
                ])
              );
            }
          }
          return -1;
        }
        const color = /^\{\.(yellow|green|blue|pink)\}/.exec(cx.slice(pos, Math.min(cx.end, pos + 9)));
        if (color) return cx.addElement(cx.elt("CriticColor", pos, pos + color[0].length));
        return -1;
      },
    },
    {
      name: "Highlight",
      before: "Emphasis",
      parse(cx, next, pos) {
        if (next !== 61 /* = */ || code(cx, pos + 1) !== 61) return -1;
        if (code(cx, pos + 2) === 125 /* } */) return cx.addDelimiter(CriticDelim, pos, pos + 3, false, true);
        // Three or more in a row are text.
        if (code(cx, pos - 1) === 61 || code(cx, pos + 2) === 61) return -1;
        return cx.addDelimiter(HighlightDelim, pos, pos + 2, !isSpace(code(cx, pos + 2)), !isSpace(code(cx, pos - 1)));
      },
    },
    {
      name: "Underline",
      before: "HTMLTag",
      parse(cx, next, pos) {
        if (next !== 60 /* < */) return -1;
        const tag = cx.slice(pos, Math.min(cx.end, pos + 4)).toLowerCase();
        if (tag.startsWith("<u>")) return cx.addDelimiter(UnderlineDelim, pos, pos + 3, true, false);
        if (tag === "</u>") return cx.addDelimiter(UnderlineDelim, pos, pos + 4, false, true);
        return -1;
      },
    },
    {
      name: "FootnoteRef",
      before: "Link",
      parse(cx, next, pos) {
        if (next !== 91 /* [ */ || code(cx, pos + 1) !== 94 /* ^ */) return -1;
        const match = /^\[\^([^\]\s]+)\]/.exec(cx.slice(pos, Math.min(cx.end, pos + 64)));
        return match ? cx.addElement(cx.elt("FootnoteRef", pos, pos + match[0].length)) : -1;
      },
    },
  ],
  parseBlock: [
    {
      name: "FootnoteDefinition",
      before: "LinkReference",
      parse(cx, line) {
        const match = /^\[\^([^\]\s]+)\]:[ \t]?/.exec(line.text.slice(line.pos));
        if (!match) return false;
        const from = cx.lineStart + line.pos;
        const textFrom = from + match[0].length;
        const to = cx.lineStart + line.text.length;
        cx.addElement(
          cx.elt("FootnoteDefinition", from, to, [
            cx.elt("FootnoteLabel", from, textFrom),
            ...cx.parser.parseInline(line.text.slice(line.pos + match[0].length), textFrom),
          ])
        );
        cx.nextLine();
        return true;
      },
    },
  ],
};

// The same language data as CodeMirror's own Markdown, so its list and
// quote commands (Enter, Backspace) know this is Markdown.
const parser = commonmarkLanguage.parser.configure([
  Strikethrough,
  Scene,
  { remove: ["InlineCode", "Image", "IndentedCode", "SetextHeading", "LinkReference"] },
]);
export const sceneLanguage = new Language(commonmarkLanguage.data, parser, [], "markdown");

// The text of each footnote, by its label, from the `[^1]: text` lines.
export function footnoteTexts(text) {
  const notes = new Map();
  const pattern = /^\[\^([^\]\s]+)\]:[ \t]?(.*)$/gm;
  let match;
  while ((match = pattern.exec(text))) notes.set(match[1], match[2]);
  return notes;
}

// A note's text as written after `{>>`: its kind and whether it is
// resolved come first, as crates/format writes them.
export function readNote(raw) {
  let text = raw.replace(/\\(.)/g, "$1");
  let resolved = false;
  if (text.startsWith("(resolved) ")) {
    resolved = true;
    text = text.slice(11);
  }
  let kind = "comment";
  const prefix = /^(Comment|Question|Revise|Check): /.exec(text);
  if (prefix) {
    kind = prefix[1].toLowerCase();
    text = text.slice(prefix[0].length);
  }
  return { kind, resolved, text };
}
