//! Links to notes by their title, as Obsidian writes them: `[[Mara]]`, or
//! `[[Mara|the pilot's daughter]]` to show other words. They are text in the
//! scene; the writer writes them as they are, never escaped, and the reader
//! keeps them as text, so a link reads the same in any Markdown tool that
//! knows them and as plain words in one that does not.

/// One `[[…]]` in a text: where it is, in characters, the note's title, and
/// the words it shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WikiLink {
    pub start: usize,
    pub end: usize,
    pub title: String,
    pub shown: String,
}

/// The `[[…]]` at the start of `chars`, and how many characters it takes. The
/// title is what comes before a `|`, trimmed; nothing in it may be a bracket
/// or a line end, and it may not be empty.
pub(crate) fn link_at(chars: &[char]) -> Option<(usize, String, String)> {
    if chars.len() < 5 || chars[0] != '[' || chars[1] != '[' {
        return None;
    }
    let mut at = 2;
    while at + 1 < chars.len() {
        match chars[at] {
            ']' if chars[at + 1] == ']' => {
                let inside: String = chars[2..at].iter().collect();
                let (title, shown) = match inside.split_once('|') {
                    Some((title, shown)) => (title.trim().to_string(), shown.trim().to_string()),
                    None => (inside.trim().to_string(), inside.trim().to_string()),
                };
                return (!title.is_empty() && !shown.is_empty()).then_some((at + 2, title, shown));
            }
            '[' | ']' | '\n' => return None,
            _ => at += 1,
        }
    }
    None
}

/// Every `[[…]]` in `text`, in order.
pub fn wikilinks(text: &str) -> Vec<WikiLink> {
    let chars: Vec<char> = text.chars().collect();
    let mut found = Vec::new();
    let mut at = 0;
    while at < chars.len() {
        if let Some((length, title, shown)) = link_at(&chars[at..]) {
            found.push(WikiLink { start: at, end: at + length, title, shown });
            at += length;
        } else {
            at += 1;
        }
    }
    found
}

/// Whether two titles name the same note: case and the spaces around it do
/// not matter, as in Obsidian.
pub fn same_title(a: &str, b: &str) -> bool {
    a.trim().to_lowercase() == b.trim().to_lowercase()
}

/// `text` with every link to `old` pointing at `new` instead, keeping the
/// words a `[[old|words]]` shows. Returns `None` when nothing links to `old`.
pub fn rename_wikilinks(text: &str, old: &str, new: &str) -> Option<String> {
    let links: Vec<WikiLink> = wikilinks(text).into_iter().filter(|link| same_title(&link.title, old)).collect();
    if links.is_empty() {
        return None;
    }
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut at = 0;
    for link in links {
        out.extend(&chars[at..link.start]);
        let inside: String = chars[link.start + 2..link.end - 2].iter().collect();
        match inside.split_once('|') {
            Some((_, shown)) => out.push_str(&format!("[[{new}|{shown}]]")),
            None => out.push_str(&format!("[[{new}]]")),
        }
        at = link.end;
    }
    out.extend(&chars[at..]);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{blocks_to_markdown, markdown_to_blocks};
    use writing_studio_document::Document;

    #[test]
    fn links_are_found_with_their_title_and_words() {
        let found = wikilinks("Ask [[Mara]] and [[The Pilot|the old man]], not [[ ]] or [a] or [[a\nb]].");
        let titles: Vec<(&str, &str)> = found.iter().map(|link| (link.title.as_str(), link.shown.as_str())).collect();
        assert_eq!(titles, [("Mara", "Mara"), ("The Pilot", "the old man")]);
        assert_eq!((found[0].start, found[0].end), (4, 12));
        assert!(same_title(" mara ", "Mara"));
    }

    #[test]
    fn a_link_is_written_as_it_is_and_reads_back_as_text() {
        let blocks = markdown_to_blocks("Ask [[Mara]] about [[The Pilot|the old man]], and *[[Port Ely]]*. A [bracket] stays escaped.");
        let mut document = Document::new(writing_studio_core::Id::new());
        document.blocks = blocks.clone();
        assert_eq!(
            document.to_plain_text(),
            "Ask [[Mara]] about [[The Pilot|the old man]], and [[Port Ely]]. A [bracket] stays escaped."
        );
        assert_eq!(
            blocks_to_markdown(&blocks),
            "Ask [[Mara]] about [[The Pilot|the old man]], and *[[Port Ely]]*. A \\[bracket\\] stays escaped.\n"
        );
    }

    #[test]
    fn renaming_keeps_the_words_a_link_shows() {
        let text = "[[Mara]] met [[mara|her]] and [[Marat]].";
        assert_eq!(rename_wikilinks(text, "Mara", "Mara Vance").unwrap(), "[[Mara Vance]] met [[Mara Vance|her]] and [[Marat]].");
        assert_eq!(rename_wikilinks(text, "Nobody", "Someone"), None);
    }

    #[test]
    fn a_link_right_before_a_bracket_is_escaped_so_it_stays_text() {
        let mut document = Document::new(writing_studio_core::Id::new());
        document.replace_plain_text("[[a]](b)");
        let written = blocks_to_markdown(&document.blocks);
        let read = markdown_to_blocks(&written);
        let mut again = Document::new(writing_studio_core::Id::new());
        again.blocks = read;
        assert_eq!(again.to_plain_text(), "[[a]](b)", "{written}");
    }
}
