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

    pub fn html(&self) -> String {
        self.document.to_html()
    }

    pub fn apply_html(&mut self, html: &str) {
        let mut next = self.document.clone();
        next.replace_html(html);
        if next.to_html() == self.document.to_html() {
            return;
        }
        self.push_undo();
        self.document = next;
        self.dirty = true;
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

    #[test]
    fn resolving_a_note_is_one_undo_step() {
        let mut session = EditorSession::empty(Id::new());
        session.apply_html(r#"<p><span class="ws-comment" data-comment="tide">harbor</span></p>"#);
        session.mark_clean();
        assert!(session.edit(|doc| doc.set_note_resolved(0, true)));
        assert!(session.is_dirty());
        assert!(session.html().contains("data-resolved"));
        assert!(!session.edit(|doc| doc.set_note_resolved(0, true)));
        assert!(session.undo());
        assert!(!session.html().contains("data-resolved"));
    }

    #[test]
    fn html_keeps_bold_mark() {
        let mut session = EditorSession::empty(Id::new());
        session.apply_html("<p>Hello <strong>there</strong></p>");
        let html = session.html();
        assert!(html.contains("<strong>there</strong>"));
        assert_eq!(session.plain_text(), "Hello there");
    }

    #[test]
    fn html_keeps_link_comment_and_scene_break() {
        let mut session = EditorSession::empty(Id::new());
        session.apply_html(
            r#"<p><span class="ws-comment" data-comment="tide"><a href="https://example.com">harbor</a></span></p><hr><pre>code</pre>"#,
        );
        let html = session.html();
        assert!(html.contains("https://example.com"));
        assert!(html.contains("data-comment=\"tide\""));
        assert!(html.contains("<hr>"));
        assert!(html.contains("<pre>code</pre>"));
        assert_eq!(session.document().comments().len(), 1);
    }
}
