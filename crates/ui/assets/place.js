// Keeping one's place on the page: fixed scrolling, a line marker, and
// fading all but the current sentence or paragraph. Nothing here changes the
// manuscript's own elements, so nothing of it reaches the saved text: the
// fade is a CSS highlight (as the spelling underlines are) and the marker is
// an element on the desk, outside the manuscript.
//
// A manuscript drawn again installs this again; the earlier listeners are
// taken off first. Each selection change does only what the choices need:
// find the caret's line, and its paragraph or sentence when fading.

function wsKeepPlace(options) {
    if (document.wsPlaceOff) document.wsPlaceOff();
    const el = document.querySelector('[data-manuscript]');
    const desk = el ? el.closest('.desk') : null;
    if (!el || !desk) return false;

    let marker = desk.querySelector('.line-marker');
    if (options.marker && !marker) {
        marker = document.createElement('div');
        marker.className = 'line-marker';
        marker.setAttribute('aria-hidden', 'true');
        desk.appendChild(marker);
    }
    if (!options.marker && marker) {
        marker.remove();
        marker = null;
    }
    const highlights = window.CSS && CSS.highlights;
    if (highlights) highlights.delete('ws-current');

    const caretRect = (range) => {
        const rects = range.getClientRects();
        if (rects.length) return rects[rects.length - 1];
        // An empty line has no rectangle of its own; use its block's.
        let node = range.startContainer;
        if (node.nodeType !== 1) node = node.parentElement;
        return node ? node.getBoundingClientRect() : null;
    };

    const blockOf = (node) => {
        let at = node.nodeType === 1 ? node : node.parentElement;
        while (at && at.parentElement !== el) at = at.parentElement;
        return at && at.parentElement === el ? at : null;
    };

    // The sentence around `offset` in `text`: from just after the last
    // sentence end before it to just after the next one.
    const sentenceAround = (text, offset) => {
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
    };

    // A range over characters `start` to `end` of `block`'s text.
    const rangeIn = (block, start, end) => {
        const walker = document.createTreeWalker(block, NodeFilter.SHOW_TEXT);
        const range = document.createRange();
        let seen = 0;
        let started = false;
        while (walker.nextNode()) {
            const node = walker.currentNode;
            const length = node.nodeValue.length;
            if (!started && start <= seen + length) {
                range.setStart(node, start - seen);
                started = true;
            }
            if (started && end <= seen + length) {
                range.setEnd(node, end - seen);
                return range;
            }
            seen += length;
        }
        if (!started) range.selectNodeContents(block);
        else range.setEndAfter(block.lastChild);
        return range;
    };

    const update = () => {
        const sel = window.getSelection();
        if (!sel || !sel.rangeCount || !el.contains(sel.anchorNode)) {
            if (marker) marker.style.display = 'none';
            if (highlights) highlights.delete('ws-current');
            return;
        }
        const range = sel.getRangeAt(0).cloneRange();
        range.collapse(false);
        const rect = caretRect(range);
        if (!rect) return;
        const deskRect = desk.getBoundingClientRect();

        if (options.scroll !== 'off') {
            const share = options.scroll === 'top' ? 0.25 : options.scroll === 'bottom' ? 0.7 : 0.45;
            const want = deskRect.top + desk.clientHeight * share;
            const by = rect.top - want;
            if (Math.abs(by) > 2) desk.scrollTop += by;
        }

        if (marker) {
            const now = caretRect(range) || rect;
            const elRect = el.getBoundingClientRect();
            marker.style.display = 'block';
            marker.style.top = (now.top - deskRect.top + desk.scrollTop - 2) + 'px';
            marker.style.height = (now.height + 4) + 'px';
            marker.style.left = (elRect.left - deskRect.left + desk.scrollLeft) + 'px';
            marker.style.width = elRect.width + 'px';
        }

        if (highlights && options.fade !== 'off') {
            const block = blockOf(range.startContainer);
            if (!block) {
                highlights.delete('ws-current');
                return;
            }
            let current;
            if (options.fade === 'paragraph') {
                current = document.createRange();
                current.selectNodeContents(block);
            } else {
                const before = document.createRange();
                before.selectNodeContents(block);
                before.setEnd(range.startContainer, range.startOffset);
                const [start, end] = sentenceAround(block.textContent, before.toString().length);
                current = rangeIn(block, start, end);
            }
            highlights.set('ws-current', new Highlight(current));
        }
    };

    let queued = false;
    const soon = () => {
        if (queued) return;
        queued = true;
        requestAnimationFrame(() => {
            queued = false;
            update();
        });
    };
    document.addEventListener('selectionchange', soon);
    window.addEventListener('resize', soon);
    document.wsPlaceOff = () => {
        document.removeEventListener('selectionchange', soon);
        window.removeEventListener('resize', soon);
        if (highlights) highlights.delete('ws-current');
    };
    update();
    return true;
}
