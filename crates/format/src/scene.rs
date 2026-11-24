//! A scene file.
//!
//! On disk a scene is a Markdown file: its title and the rest of what Context
//! shows as front matter, then its text. The same file is what the Markdown
//! export writes and what the import reads.

use writing_studio_core::Id;
use writing_studio_document::Block;

use crate::header::{split_header, Header, HeaderError};
use crate::{blocks_to_markdown, markdown_to_blocks};

/// What a scene file says about its scene, besides the text.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SceneHeader {
    /// The binder item this file is the text of. `None` in a file a person
    /// made by hand; the program gives it one when it takes the file in.
    pub id: Option<Id>,
    pub title: String,
    pub synopsis: String,
    pub status: String,
    pub label: String,
    pub color: String,
    pub goal: String,
    pub conflict: String,
    pub outcome: String,
    /// Entries this program does not know. They are written back as found.
    pub other: Header,
}

#[derive(Clone, Debug)]
pub struct SceneFile {
    pub header: SceneHeader,
    pub blocks: Vec<Block>,
}

impl SceneHeader {
    /// The entries other than the title, in the order they are written.
    fn entries(&self) -> Header {
        let mut header = Header::default();
        if let Some(id) = self.id {
            header.set_text("id", &id.to_string());
        }
        for (key, value) in [
            ("synopsis", &self.synopsis),
            ("status", &self.status),
            ("label", &self.label),
            ("color", &self.color),
            ("goal", &self.goal),
            ("conflict", &self.conflict),
            ("outcome", &self.outcome),
        ] {
            header.set_text(key, value);
        }
        header.entries.extend(self.other.entries.iter().cloned());
        header
    }

    fn from_entries(title: String, mut header: Header) -> Self {
        let id = header.take_text("id");
        Self {
            id: id.trim().parse().ok(),
            title,
            synopsis: header.take_text("synopsis"),
            status: header.take_text("status"),
            label: header.take_text("label"),
            color: header.take_text("color"),
            goal: header.take_text("goal"),
            conflict: header.take_text("conflict"),
            outcome: header.take_text("outcome"),
            other: header,
        }
    }
}

impl SceneFile {
    /// The scene as the Markdown file that is stored, and exported. The few
    /// things Markdown cannot say are left out; see
    /// [`crate::canonical_blocks`].
    pub fn to_markdown(&self) -> String {
        let mut header = Header::default();
        header.set_text("title", &self.header.title);
        header.entries.extend(self.header.entries().entries);
        // The id reads better first.
        if let Some(at) = header.entries.iter().position(|(key, _)| key == "id") {
            let id = header.entries.remove(at);
            header.entries.insert(0, id);
        }
        // No text is no body, not the `<br>` of one empty paragraph: a book's
        // or a folder's file is only its front matter. It reads back the same.
        let empty = self.blocks.iter().all(|block| match &block.kind {
            writing_studio_document::BlockKind::Paragraph { inlines } => inlines.iter().all(|inline| inline.text.is_empty()),
            _ => false,
        });
        if empty {
            return header.to_text();
        }
        format!("{}\n{}", header.to_text(), blocks_to_markdown(&self.blocks))
    }

    /// Read a scene from a Markdown file, with or without a header.
    pub fn parse_markdown(file: &str) -> Result<Self, HeaderError> {
        let file = file.strip_prefix('\u{feff}').unwrap_or(file);
        let (mut header, body) = split_header(file)?;
        let title = header.take_text("title");
        Ok(Self {
            header: SceneHeader::from_entries(title, header),
            blocks: markdown_to_blocks(body),
        })
    }
}

/// The longest a readable name gets before the id is added.
const NAME_LENGTH: usize = 40;

/// The file name for an item: its title made safe for any file system, then
/// the first four characters of its id, as in `the-harbor-3f9a.md`. The name
/// is chosen once, when the file is made. It does not follow later changes
/// to the title, so that a rename never moves a file.
pub fn file_name(title: &str, id: Id) -> String {
    let id = id.to_string();
    format!("{}-{}.md", slug(title), &id[..4])
}

/// The readable part of a file name: the title in lowercase, with anything
/// but letters and digits turned into `-`, at most 40 characters, and
/// `untitled` when nothing is left.
pub fn slug(title: &str) -> String {
    let mut name = String::new();
    for ch in title.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            name.push(ch);
        } else if let Some(plain) = plain_letter(ch) {
            name.push_str(plain);
        } else if ('\u{300}'..='\u{36f}').contains(&ch) {
            // An accent typed as its own character, as macOS stores names.
            continue;
        } else if !name.is_empty() && !name.ends_with('-') {
            name.push('-');
        }
        if name.len() >= NAME_LENGTH {
            name.truncate(NAME_LENGTH);
            break;
        }
    }
    let name = name.trim_end_matches('-');
    if name.is_empty() { "untitled".into() } else { name.to_string() }
}

/// The plain letters for a lowercase Latin letter with an accent. File names
/// keep to ASCII so every computer and git store them the same way.
fn plain_letter(ch: char) -> Option<&'static str> {
    Some(match ch {
        'à'..='å' | 'ā' | 'ă' | 'ą' => "a",
        'æ' => "ae",
        'ç' | 'ć' | 'ĉ' | 'ċ' | 'č' => "c",
        'ð' | 'ď' | 'đ' => "d",
        'è'..='ë' | 'ē' | 'ĕ' | 'ė' | 'ę' | 'ě' => "e",
        'ĝ' | 'ğ' | 'ġ' | 'ģ' => "g",
        'ĥ' | 'ħ' => "h",
        'ì'..='ï' | 'ĩ' | 'ī' | 'ĭ' | 'į' | 'ı' => "i",
        'ĵ' => "j",
        'ķ' => "k",
        'ĺ' | 'ļ' | 'ľ' | 'ŀ' | 'ł' => "l",
        'ñ' | 'ń' | 'ņ' | 'ň' => "n",
        'ò'..='ö' | 'ø' | 'ō' | 'ŏ' | 'ő' => "o",
        'œ' => "oe",
        'ŕ' | 'ŗ' | 'ř' => "r",
        'ß' => "ss",
        'ś' | 'ŝ' | 'ş' | 'š' => "s",
        'ţ' | 'ť' | 'ŧ' => "t",
        'þ' => "th",
        'ù'..='ü' | 'ũ' | 'ū' | 'ŭ' | 'ů' | 'ű' | 'ų' => "u",
        'ŵ' => "w",
        'ý' | 'ÿ' | 'ŷ' => "y",
        'ź' | 'ż' | 'ž' => "z",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::header::Value;
    use crate::{canonical_blocks, same_content};
    use writing_studio_document::Document;

    fn harbor() -> SceneFile {
        let id: Id = "3f9a2c1e-0b7d-4e55-9a10-6c2f8d4b1a77".parse().unwrap();
        let mut document = Document::new(Id::new());
        document.replace_html("<p>Mara ran for the <strong>north </strong>quay.</p><p>“Wait,” said the pilot.</p>");
        let mut other = Header::default();
        other.entries.push(("mood".into(), Value::Text("grey".into())));
        other
            .entries
            .push(("also".into(), Value::List(vec!["Mara".into(), "the pilot".into()])));
        SceneFile {
            header: SceneHeader {
                id: Some(id),
                title: "The Harbor & the \"Tide\"".into(),
                synopsis: "Mara is late.\nThe boat is leaving.".into(),
                status: "draft".into(),
                goal: "Reach the boat before the tide".into(),
                outcome: "The boat has gone".into(),
                other,
                ..SceneHeader::default()
            },
            blocks: document.blocks.clone(),
        }
    }

    #[test]
    fn a_scene_is_stored_as_markdown_and_reads_back() {
        let file = harbor();
        let text = file.to_markdown();
        assert!(text.starts_with(concat!(
            "---\n",
            "id: 3f9a2c1e-0b7d-4e55-9a10-6c2f8d4b1a77\n",
            "title: The Harbor & the \"Tide\"\n",
            "synopsis: |-\n  Mara is late.\n  The boat is leaving.\n",
        )));
        // Markdown cannot say that the space after "north" is bold.
        assert!(text.ends_with("---\n\nMara ran for the **north** quay.\n\n“Wait,” said the pilot.\n"));
        let read = SceneFile::parse_markdown(&text).unwrap();
        assert_eq!(read.header, file.header);
        assert!(same_content(&read.blocks, &canonical_blocks(&file.blocks)));
        assert_eq!(read.to_markdown(), text, "a stored file is written the same again");
        // Keys this program does not know are kept, a list as a list.
        assert!(text.contains("mood: grey\n") && text.contains("also:\n  - Mara\n  - the pilot\n"), "{text}");
        // A byte-order mark from another editor is not part of the file.
        let marked = format!("\u{feff}{text}");
        assert_eq!(SceneFile::parse_markdown(&marked).unwrap().header, file.header);
    }

    #[test]
    fn a_file_written_by_hand_needs_no_head() {
        let read = SceneFile::parse_markdown("# One\n\nJust a scene.\n").unwrap();
        assert_eq!(read.header, SceneHeader::default());
        assert_eq!(read.blocks.len(), 2);
    }

    #[test]
    fn file_names_are_readable_and_safe() {
        let id: Id = "3f9a2c1e-0b7d-4e55-9a10-6c2f8d4b1a77".parse().unwrap();
        assert_eq!(file_name("The Harbor", id), "the-harbor-3f9a.md");
        assert_eq!(file_name("  Où est / l'Été? ", id), "ou-est-l-ete-3f9a.md");
        assert_eq!(file_name("Größe Straße", id), "grosse-strasse-3f9a.md");
        assert_eq!(file_name("Cafe\u{301} noir", id), "cafe-noir-3f9a.md");
        assert_eq!(file_name("Ἀθῆναι", id), "untitled-3f9a.md");
        assert_eq!(file_name("東京の夜", id), "untitled-3f9a.md");
        assert_eq!(file_name("Rain 🌧 again", id), "rain-again-3f9a.md");
        assert_ne!(file_name("Été", id), file_name("Ete 2", id));
        assert_eq!(file_name("", id), "untitled-3f9a.md");
        assert_eq!(file_name("../..", id), "untitled-3f9a.md");
        let long = file_name(&"word ".repeat(30), id);
        assert!(long.len() <= NAME_LENGTH + ".md".len() + 5);
        assert!(file_name(&"é".repeat(60), id).is_ascii());
        assert!(!long.contains("--"));
    }
}
