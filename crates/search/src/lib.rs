//! Placeholder for Tantivy indexing. Phase 1 uses in-memory title filters in the UI.

pub fn matches_query(haystack: &str, query: &str) -> bool {
    if query.trim().is_empty() {
        return true;
    }
    haystack.to_lowercase().contains(&query.to_lowercase())
}
