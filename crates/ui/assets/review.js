// Review helpers for the manuscript editor. This file is put in front of the
// short scripts the toolbar and the review list run, so everything here is a
// plain function and nothing is kept on `window`.

// Wrap each piece of text inside the range in its own element. Wrapping text
// pieces, not the range, keeps paragraphs out of the wrapper when a selection
// crosses more than one paragraph.
function wsWrapRange(el, range, make) {
    const nodes = [];
    const walker = document.createTreeWalker(el, NodeFilter.SHOW_TEXT);
    while (walker.nextNode()) {
        const node = walker.currentNode;
        if (node.nodeValue.length && range.intersectsNode(node)) nodes.push(node);
    }
    const startNode = range.startContainer;
    const startOffset = range.startOffset;
    const endNode = range.endContainer;
    const endOffset = range.endOffset;
    let wrapped = 0;
    nodes.forEach((node) => {
        const start = node === startNode ? startOffset : 0;
        const end = node === endNode ? endOffset : node.nodeValue.length;
        if (end <= start) return;
        let piece = node;
        if (end < piece.nodeValue.length) piece.splitText(end);
        if (start > 0) piece = piece.splitText(start);
        const wrap = make();
        piece.parentNode.insertBefore(wrap, piece);
        wrap.appendChild(piece);
        wrapped += 1;
    });
    return wrapped;
}

function wsSelectedRange(el) {
    const sel = window.getSelection();
    if (!el || !sel || !sel.rangeCount || sel.isCollapsed) return null;
    const range = sel.getRangeAt(0);
    return el.contains(range.commonAncestorContainer) ? range.cloneRange() : null;
}

function wsUnwrap(node) {
    const parent = node.parentNode;
    while (node.firstChild) parent.insertBefore(node.firstChild, node);
    parent.removeChild(node);
    parent.normalize();
}

// Elements matching `selector` that the selection touches, or that hold the caret.
function wsTouched(el, selector) {
    const sel = window.getSelection();
    if (!el || !sel || !sel.rangeCount) return [];
    const range = sel.getRangeAt(0);
    return Array.from(el.querySelectorAll(selector)).filter((node) =>
        range.collapsed ? node.contains(range.startContainer) : range.intersectsNode(node)
    );
}

function wsComment(el, kind) {
    const range = wsSelectedRange(el);
    if (!range) return;
    const raw = window.prompt('Note on the selection');
    const text = raw ? raw.trim() : '';
    if (!text) return;
    wsWrapRange(el, range, () => {
        const span = document.createElement('span');
        span.className = 'ws-comment';
        span.setAttribute('data-comment', text);
        if (kind && kind !== 'comment') span.setAttribute('data-kind', kind);
        span.title = text;
        return span;
    });
    window.getSelection().removeAllRanges();
}

function wsUncomment(el) {
    wsTouched(el, 'span[data-comment]').forEach(wsUnwrap);
}

function wsHighlight(el, color) {
    const range = wsSelectedRange(el);
    if (!range) return;
    wsWrapRange(el, range, () => {
        const mark = document.createElement('mark');
        mark.className = 'ws-hl';
        mark.setAttribute('data-highlight', color);
        return mark;
    });
    window.getSelection().removeAllRanges();
}

function wsClearHighlight(el) {
    wsTouched(el, 'mark[data-highlight]').forEach(wsUnwrap);
}

// Select the nth note or highlight and scroll to it. Text pieces are grouped
// the way the document groups runs: neighbors in one paragraph that carry the
// same note (or the same color) are one item.
function wsSelectReview(el, type, index) {
    if (!el) return false;
    const note = type === 'note';
    const selector = note ? 'span[data-comment]' : 'mark[data-highlight]';
    const blockOf = (node) => {
        let at = node.parentElement;
        while (at && at !== el && !/^(P|DIV|H[1-6]|LI|BLOCKQUOTE|PRE)$/.test(at.tagName)) {
            at = at.parentElement;
        }
        return at;
    };
    const keyOf = (node) => {
        const holder = node.parentElement.closest(selector);
        if (!holder || !el.contains(holder)) return null;
        if (!note) return holder.getAttribute('data-highlight');
        const text = (holder.getAttribute('data-comment') || '').trim();
        if (!text) return null;
        const kind = holder.getAttribute('data-kind') || 'comment';
        const resolved = holder.getAttribute('data-resolved') === 'true';
        return [text, kind, resolved].join('\u0001');
    };
    const walker = document.createTreeWalker(el, NodeFilter.SHOW_TEXT);
    let count = -1;
    let lastKey = null;
    let lastBlock = null;
    let first = null;
    let last = null;
    while (walker.nextNode()) {
        const node = walker.currentNode;
        if (!node.nodeValue.length) continue;
        const key = keyOf(node);
        const block = blockOf(node);
        if (key !== null && (key !== lastKey || block !== lastBlock)) count += 1;
        lastKey = key;
        lastBlock = block;
        if (key === null) continue;
        if (count === index) {
            first = first || node;
            last = node;
        } else if (count > index) {
            break;
        }
    }
    if (!first) return false;
    const range = document.createRange();
    range.setStart(first, 0);
    range.setEnd(last, last.nodeValue.length);
    const sel = window.getSelection();
    sel.removeAllRanges();
    sel.addRange(range);
    first.parentElement.scrollIntoView({ block: 'center' });
    return true;
}

// Select the nth top-level paragraph and scroll to it. A paragraph that was
// removed has no place of its own, so the caller passes where it used to be
// and the nearest paragraph is shown.
function wsShowBlock(el, index) {
    if (!el || !el.children.length) return false;
    const block = el.children[Math.min(index, el.children.length - 1)];
    const range = document.createRange();
    range.selectNodeContents(block);
    const sel = window.getSelection();
    sel.removeAllRanges();
    sel.addRange(range);
    block.scrollIntoView({ block: 'center' });
    return true;
}
