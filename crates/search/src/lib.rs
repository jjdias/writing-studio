//! In-memory full-text index of manuscript documents.
//!
//! The project folder stays the source of truth. This Tantivy index is rebuilt
//! from document text when a project opens and updated when a document is saved.

use std::collections::HashSet;
use std::str::FromStr;

use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{Schema, Value, STORED, STRING, TEXT};
use tantivy::indexer::IndexWriter;
use tantivy::{doc, Index, IndexReader, ReloadPolicy, TantivyDocument, Term};
use writing_studio_core::Id;

const SEARCH_LIMIT: usize = 10_000;
const WRITER_MEMORY: usize = 15_000_000;

pub struct ManuscriptIndex {
    index: Index,
    reader: IndexReader,
    id_field: tantivy::schema::Field,
    body_field: tantivy::schema::Field,
    pending: Option<(Id, String)>,
}

#[derive(Debug)]
pub struct SearchError(String);

impl std::fmt::Display for SearchError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for SearchError {}

impl ManuscriptIndex {
    pub fn new() -> Self {
        Self::empty().expect("manuscript search schema")
    }

    fn empty() -> Result<Self, SearchError> {
        let mut builder = Schema::builder();
        let id_field = builder.add_text_field("document_id", STRING | STORED);
        let body_field = builder.add_text_field("body", TEXT);
        let schema = builder.build();
        let index = Index::create_in_ram(schema);
        // Manual reload keeps commit off a background thread. That thread was
        // taking the index lock while the UI thread also committed, which froze the window.
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()
            .map_err(|err| SearchError(err.to_string()))?;
        Ok(Self {
            index,
            reader,
            id_field,
            body_field,
            pending: None,
        })
    }

    pub fn replace_all(&mut self, documents: &[(Id, &str)]) -> Result<(), SearchError> {
        let fresh = Self::empty()?;
        if !documents.is_empty() {
            {
                let mut writer = fresh.writer()?;
                for (id, text) in documents {
                    writer
                        .add_document(doc!(
                            fresh.id_field => id.to_string(),
                            fresh.body_field => *text,
                        ))
                        .map_err(|err| SearchError(err.to_string()))?;
                }
                writer.commit().map_err(|err| SearchError(err.to_string()))?;
            }
            fresh
                .reader
                .reload()
                .map_err(|err| SearchError(err.to_string()))?;
        }
        *self = fresh;
        Ok(())
    }

    pub fn upsert(&mut self, id: Id, text: &str) -> Result<(), SearchError> {
        if self.pending.as_ref().is_some_and(|(pending_id, _)| *pending_id == id) {
            self.pending = None;
        }
        {
            let mut writer = self.writer()?;
            writer.delete_term(Term::from_field_text(self.id_field, &id.to_string()));
            writer
                .add_document(doc!(
                    self.id_field => id.to_string(),
                    self.body_field => text,
                ))
                .map_err(|err| SearchError(err.to_string()))?;
            writer.commit().map_err(|err| SearchError(err.to_string()))?;
        }
        self.reader.reload().map_err(|err| SearchError(err.to_string()))?;
        Ok(())
    }

    pub fn remove(&mut self, id: Id) -> Result<(), SearchError> {
        if self.pending.as_ref().is_some_and(|(pending_id, _)| *pending_id == id) {
            self.pending = None;
        }
        {
            let mut writer = self.writer()?;
            writer.delete_term(Term::from_field_text(self.id_field, &id.to_string()));
            writer.commit().map_err(|err| SearchError(err.to_string()))?;
        }
        self.reader.reload().map_err(|err| SearchError(err.to_string()))?;
        Ok(())
    }

    fn writer(&self) -> Result<IndexWriter<TantivyDocument>, SearchError> {
        self.index
            .writer_with_num_threads(1, WRITER_MEMORY)
            .map_err(|err| SearchError(err.to_string()))
    }

    /// Unsaved text for the open document. Search includes it before the next save.
    pub fn set_pending(&mut self, id: Id, text: impl Into<String>) {
        self.pending = Some((id, text.into()));
    }

    pub fn search(&self, query: &str) -> HashSet<Id> {
        let query = query.trim();
        if query.is_empty() {
            return HashSet::new();
        }
        let mut hits = self.search_committed(query);
        if let Some((id, text)) = &self.pending {
            if text_matches(query, text) {
                hits.insert(*id);
            } else {
                hits.remove(id);
            }
        }
        hits
    }

    fn search_committed(&self, query: &str) -> HashSet<Id> {
        let Ok(parsed) = parse_query(&self.index, self.body_field, query) else {
            return HashSet::new();
        };
        let searcher = self.reader.searcher();
        let Ok(top_docs) = searcher.search(&parsed, &TopDocs::with_limit(SEARCH_LIMIT)) else {
            return HashSet::new();
        };
        let mut hits = HashSet::new();
        for (_score, address) in top_docs {
            let Ok(retrieved) = searcher.doc::<TantivyDocument>(address) else {
                continue;
            };
            let Some(value) = retrieved.get_first(self.id_field).as_ref().and_then(Value::as_str) else {
                continue;
            };
            if let Ok(id) = Id::from_str(value) {
                hits.insert(id);
            }
        }
        hits
    }
}

/// Word check for text that is not committed yet.
///
/// Building a second Tantivy index here ran on the UI thread on every redraw.
fn text_matches(query: &str, text: &str) -> bool {
    let terms = query_terms(query);
    if terms.is_empty() {
        return false;
    }
    let words: HashSet<String> = query_terms(text).into_iter().collect();
    terms.iter().all(|term| words.contains(term))
}

fn query_terms(query: &str) -> Vec<String> {
    plain_terms(query)
        .split_whitespace()
        .map(|term| term.to_lowercase())
        .filter(|term| !term.is_empty())
        .collect()
}

fn parse_query(
    index: &Index,
    body_field: tantivy::schema::Field,
    query: &str,
) -> Result<Box<dyn tantivy::query::Query>, SearchError> {
    let parser = QueryParser::for_index(index, vec![body_field]);
    parser
        .parse_query(query)
        .or_else(|_| parser.parse_query(&plain_terms(query)))
        .map_err(|err| SearchError(err.to_string()))
}

fn plain_terms(query: &str) -> String {
    query
        .chars()
        .map(|ch| {
            if ch.is_alphanumeric() || ch.is_whitespace() {
                ch
            } else {
                ' '
            }
        })
        .collect()
}

/// Case-insensitive substring match used for binder titles and labels.
pub fn matches_query(haystack: &str, query: &str) -> bool {
    let query = query.trim();
    if query.is_empty() {
        return true;
    }
    haystack.to_lowercase().contains(&query.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_a_word_inside_one_scene() {
        let harbor = Id::new();
        let other = Id::new();
        let mut index = ManuscriptIndex::new();
        index
            .replace_all(&[
                (harbor, "The harbor was quiet after midnight."),
                (other, "A market square at noon."),
            ])
            .unwrap();
        let hits = index.search("harbor");
        assert!(hits.contains(&harbor));
        assert!(!hits.contains(&other));
        assert_eq!(index.search("HARBOR"), hits);
    }

    #[test]
    fn upsert_replaces_old_text_and_remove_drops_it() {
        let id = Id::new();
        let mut index = ManuscriptIndex::new();
        index.upsert(id, "alpha beta").unwrap();
        assert!(index.search("alpha").contains(&id));
        index.upsert(id, "gamma only").unwrap();
        assert!(!index.search("alpha").contains(&id));
        assert!(index.search("gamma").contains(&id));
        index.remove(id).unwrap();
        assert!(index.search("gamma").is_empty());
    }

    #[test]
    fn pending_text_is_searchable_before_save() {
        let id = Id::new();
        let mut index = ManuscriptIndex::new();
        index.upsert(id, "saved words").unwrap();
        index.set_pending(id, "unsaved lantern");
        assert!(index.search("lantern").contains(&id));
        assert!(!index.search("saved").contains(&id));
    }

    #[test]
    fn title_filter_still_matches_substrings() {
        assert!(matches_query("Chapter One", "chap"));
        assert!(!matches_query("Chapter One", "scene"));
        assert!(matches_query("Anything", "  "));
    }
}
