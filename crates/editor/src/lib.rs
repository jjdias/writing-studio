//! Editor sessions wrap a [`Document`] with undo, dirty tracking, and counts.
//!
//! Phase 1 uses snapshot undo. Later phases can switch to operational transforms
//! or Yrs CRDTs without changing the document schema.

use writing_studio_core::Id;
use writing_studio_document::Document;

const MAX_UNDO: usize = 100;

#[derive(Clone, Debug)]
pub struct EditorSession {
    document: Document,
    undo: Vec<Document>,
    redo: Vec<Document>,
    dirty: bool,
}

impl EditorSession {
    pub fn new(document: Document) -> Self {
        Self {
            document,
            undo: Vec::new(),
            redo: Vec::new(),
            dirty: false,
        }
    }

    pub fn empty(id: Id) -> Self {
        Self::new(Document::new(id))
    }

    pub fn document(&self) -> &Document {
        &self.document
    }

    pub fn document_id(&self) -> Id {
        self.document.id
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    pub fn plain_text(&self) -> String {
        self.document.to_plain_text()
    }

    pub fn word_count(&self) -> u32 {
        self.document.word_count()
    }

    pub fn char_count(&self) -> u32 {
        self.document.char_count()
    }

    pub fn apply_plain_text(&mut self, text: &str) {
        if text == self.document.to_plain_text() {
            return;
        }
        self.push_undo();
        self.document.replace_plain_text(text);
        self.dirty = true;
    }

    /// Take blocks read from the Markdown page as the text, as one undo
    /// step, unless they say what the document already says.
    pub fn apply_blocks(&mut self, blocks: Vec<writing_studio_document::Block>) -> bool {
        let mut next = self.document.clone();
        next.replace_blocks(blocks);
        if next.to_html() == self.document.to_html() {
            return false;
        }
        self.push_undo();
        self.document = next;
        self.dirty = true;
        true
    }

    /// Change the document in place, as one undo step. The closure returns
    /// whether it changed anything.
    pub fn edit(&mut self, change: impl FnOnce(&mut Document) -> bool) -> bool {
        let mut next = self.document.clone();
        if !change(&mut next) {
            return false;
        }
        self.push_undo();
        self.document = next;
        self.dirty = true;
        true
    }

    pub fn undo(&mut self) -> bool {
        if let Some(previous) = self.undo.pop() {
            self.redo.push(self.document.clone());
            self.document = previous;
            self.dirty = true;
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self) -> bool {
        if let Some(next) = self.redo.pop() {
            self.undo.push(self.document.clone());
            self.document = next;
            self.dirty = true;
            true
        } else {
            false
        }
    }

    /// The text the next undo goes back to.
    pub fn undo_target(&self) -> Option<&Document> {
        self.undo.last()
    }

    /// Forget what redo would bring back, for an undo that also changed
    /// things outside the text, which redo could not do again.
    pub fn clear_redo(&mut self) {
        self.redo.clear();
    }

    fn push_undo(&mut self) {
        self.undo.push(self.document.clone());
        if self.undo.len() > MAX_UNDO {
            self.undo.remove(0);
        }
        self.redo.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn undo_restores_previous_text() {
        let mut session = EditorSession::empty(Id::new());
        session.apply_plain_text("Hello");
        session.apply_plain_text("Hello world");
        assert!(session.undo());
        assert_eq!(session.plain_text(), "Hello");
        assert!(session.redo());
        assert_eq!(session.plain_text(), "Hello world");
    }

    /// The scene with this HTML, as storage reads it.
    fn with_html(html: &str) -> EditorSession {
        let mut document = Document::new(Id::new());
        document.replace_html(html);
        EditorSession::new(document)
    }

    #[test]
    fn resolving_a_note_is_one_undo_step() {
        let mut session = with_html(r#"<p><span class="ws-comment" data-comment="tide">harbor</span></p>"#);
        assert!(session.edit(|doc| doc.set_note_resolved(0, true)));
        assert!(session.is_dirty());
        assert!(session.document().to_html().contains("data-resolved"));
        assert!(!session.edit(|doc| doc.set_note_resolved(0, true)));
        assert!(session.undo());
        assert!(!session.document().to_html().contains("data-resolved"));
    }

    #[test]
    fn blocks_from_the_page_keep_the_ids_and_are_one_undo_step() {
        let mut session = with_html("<p>Hello <strong>there</strong></p><p>Second</p>");
        let ids: Vec<Id> = session.document().blocks.iter().map(|block| block.id).collect();
        let same = with_html("<p>Hello <strong>there</strong></p><p>Second</p>").document().blocks.clone();
        assert!(!session.apply_blocks(same), "the same text is no change");
        assert!(!session.is_dirty());
        let changed = with_html("<p>Hello <em>there</em></p><p>Second line</p>").document().blocks.clone();
        assert!(session.apply_blocks(changed));
        assert_eq!(session.document().blocks.iter().map(|block| block.id).collect::<Vec<_>>(), ids);
        assert_eq!(session.plain_text(), "Hello there\nSecond line");
        assert!(session.undo());
        assert!(session.document().to_html().contains("<strong>there</strong>"));
    }
}
