// Spelling underlines for the manuscript editor.
//
// Rust decides which words are unknown and sends the list. This script finds
// those words in the manuscript and underlines them with the CSS Custom
// Highlight API, which draws on ranges and leaves the manuscript's HTML
// alone, so an underline is never saved into the scene. A webview without
// that API shows no underlines; the spelling list above the manuscript still
// works.

// The text of each paragraph, with the text nodes it is made of. A line break
// starts a new piece so words on two lines are not read as one.
function wsSpellSegments(el) {
    const segments = [];
    let current = null;
    let lastBlock = null;
    const blockOf = (node) => {
        let at = node.parentElement;
        while (at && at !== el && !/^(P|DIV|H[1-6]|LI|BLOCKQUOTE|PRE)$/.test(at.tagName)) {
            at = at.parentElement;
        }
        return at;
    };
    const walker = document.createTreeWalker(el, NodeFilter.SHOW_TEXT | NodeFilter.SHOW_ELEMENT);
    while (walker.nextNode()) {
        const node = walker.currentNode;
        if (node.nodeType === 1) {
            if (node.tagName === 'BR') current = null;
            continue;
        }
        if (!node.nodeValue.length) continue;
        const block = blockOf(node);
        if (!current || block !== lastBlock) {
            current = { text: '', pieces: [] };
            segments.push(current);
            lastBlock = block;
        }
        current.pieces.push({ node, start: current.text.length });
        current.text += node.nodeValue;
    }
    return segments;
}

// Ranges over every word in `wanted`. Words are split the way the Rust side
// splits them: letters and digits, with apostrophes allowed inside.
function wsSpellRanges(el, wanted) {
    const ranges = [];
    if (!el || !wanted.size) return ranges;
    const place = (segment, offset, end) => {
        let piece = segment.pieces[0];
        for (const next of segment.pieces) {
            if (next.start > offset || (end && next.start >= offset)) break;
            piece = next;
        }
        return [piece.node, offset - piece.start];
    };
    wsSpellSegments(el).forEach((segment) => {
        const pattern = /[\p{Alphabetic}\p{N}]+(?:['’][\p{Alphabetic}\p{N}]+)*/gu;
        let match;
        while ((match = pattern.exec(segment.text))) {
            if (!wanted.has(match[0].replace(/’/g, "'"))) continue;
            const range = document.createRange();
            range.setStart(...place(segment, match.index, false));
            range.setEnd(...place(segment, match.index + match[0].length, true));
            ranges.push(range);
        }
    });
    return ranges;
}

// Underline the unknown words, except the one the caret is in: a word is not
// wrong while it is still being typed.
function wsSpellPaint(el) {
    if (typeof Highlight === 'undefined' || !window.CSS || !CSS.highlights) return false;
    const state = window.__wsSpell;
    if (!el || !state || !state.words.size) {
        CSS.highlights.delete('ws-spell');
        return true;
    }
    const sel = window.getSelection();
    const caret =
        sel && sel.rangeCount && sel.isCollapsed && el.contains(sel.anchorNode) ? sel : null;
    const ranges = wsSpellRanges(el, state.words).filter(
        (range) => !caret || range.comparePoint(caret.anchorNode, caret.anchorOffset) !== 0
    );
    CSS.highlights.set('ws-spell', new Highlight(...ranges));
    return true;
}

// Take a new list of unknown words and repaint. The first call also starts
// repainting whenever the caret moves, which covers typing.
function wsSpellApply(el, words) {
    const state = window.__wsSpell || (window.__wsSpell = { words: new Set(), timer: 0 });
    state.words = new Set(words);
    if (!state.listening) {
        state.listening = true;
        document.addEventListener('selectionchange', () => {
            clearTimeout(state.timer);
            state.timer = setTimeout(
                () => wsSpellPaint(document.querySelector('[data-manuscript]')),
                200
            );
        });
    }
    return wsSpellPaint(el);
}

// Select the next place `word` appears after the selection, going round to
// the top again, and scroll to it.
function wsSpellFind(el, word) {
    const ranges = wsSpellRanges(el, new Set([word]));
    if (!ranges.length) return false;
    const sel = window.getSelection();
    let next = ranges[0];
    if (sel && sel.rangeCount && el.contains(sel.anchorNode)) {
        const here = sel.getRangeAt(0);
        next =
            ranges.find((range) => range.compareBoundaryPoints(Range.START_TO_END, here) > 0 &&
                range.compareBoundaryPoints(Range.START_TO_START, here) > 0) || ranges[0];
    }
    sel.removeAllRanges();
    sel.addRange(next);
    const holder = next.startContainer.parentElement;
    if (holder) holder.scrollIntoView({ block: 'center' });
    return true;
}
