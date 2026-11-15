//! A scene file.
//!
//! On disk a scene is an HTML file: its title and the rest of what the
//! inspector shows in the `<head>`, and the text in the `<body>`. Any browser
//! opens it. The same scene can be written as Markdown with a front-matter
//! header, for taking a book to another program, and read from one.

use writing_studio_core::Id;
use writing_studio_document::Block;

use crate::header::{split_header, Header, HeaderError, Value};
use crate::html::{attribute, blocks_to_html, decode, find, html_to_blocks, tokens, Token};
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

/// The stylesheet every scene file points at, so that a browser shows a
/// scene as a page. It sits in the project folder, one level up.
pub const STYLESHEET: &str = "../scene.css";

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
    /// The scene as the HTML file that is stored. It holds the document
    /// exactly.
    pub fn to_html(&self) -> String {
        let mut out = String::from("<!doctype html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n");
        out.push_str(&format!("<title>{}</title>\n", attribute(&self.header.title)));
        for (key, value) in &self.header.entries().entries {
            let values: &[String] = match value {
                Value::Text(text) => std::slice::from_ref(text),
                Value::List(items) => items,
            };
            for value in values {
                out.push_str(&format!(
                    "<meta name=\"{}\" content=\"{}\">\n",
                    attribute(key),
                    attribute(value)
                ));
            }
        }
        out.push_str(&format!("<link rel=\"stylesheet\" href=\"{STYLESHEET}\">\n"));
        out.push_str("</head>\n<body>\n");
        out.push_str(&blocks_to_html(&self.blocks));
        out.push_str("</body>\n</html>\n");
        out
    }

    /// Read a stored scene. A file that is only the text, with no `<head>`
    /// or `<body>`, is read as a scene with no title.
    pub fn parse_html(file: &str) -> Self {
        let file = file.strip_prefix('\u{feff}').unwrap_or(file);
        let lower = file.to_ascii_lowercase();
        let (head, body) = match lower.find("<body") {
            Some(at) => {
                let start = file[at..].find('>').map_or(file.len(), |end| at + end + 1);
                let end = lower.rfind("</body").filter(|end| *end >= start).unwrap_or(file.len());
                (&file[..at], &file[start..end])
            }
            None => ("", file),
        };

        let mut title = String::new();
        let mut in_title = false;
        let mut header = Header::default();
        for token in tokens(head) {
            match token {
                Token::Open(name, _) if name == "title" => in_title = true,
                Token::Close(name) if name == "title" => in_title = false,
                Token::Text(text) if in_title => title.push_str(&decode(&text)),
                Token::Open(name, attributes) if name == "meta" => {
                    let (Some(key), Some(value)) = (find(&attributes, "name"), find(&attributes, "content"))
                    else {
                        continue;
                    };
                    // The same name twice is a list.
                    let value = match header.list(key).map(<[String]>::to_vec) {
                        Some(mut items) => {
                            items.push(value.to_string());
                            Value::List(items)
                        }
                        None => match header.text(key) {
                            Some(first) => Value::List(vec![first.to_string(), value.to_string()]),
                            None => Value::Text(value.to_string()),
                        },
                    };
                    match header.entries.iter_mut().find(|(name, _)| name == key) {
                        Some(entry) => entry.1 = value,
                        None => header.entries.push((key.to_string(), value)),
                    }
                }
                _ => {}
            }
        }
        Self {
            header: SceneHeader::from_entries(title, header),
            blocks: html_to_blocks(body),
        }
    }

    /// The scene as a Markdown file, for another program to read. A few
    /// things HTML holds and Markdown cannot are left out; see
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
        format!("{}\n{}", header.to_text(), blocks_to_markdown(&self.blocks))
    }

    /// Read a scene from a Markdown file, with or without a header.
    pub fn parse_markdown(file: &str) -> Result<Self, HeaderError> {
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
/// the first four characters of its id, as in `the-harbor-3f9a.html`. The name
/// is chosen once, when the file is made. It does not follow later changes
/// to the title, so that a rename never moves a file.
pub fn file_name(title: &str, id: Id) -> String {
    let mut name = String::new();
    for ch in title.chars().flat_map(char::to_lowercase) {
        if ch.is_alphanumeric() {
            name.push(ch);
        } else if !name.is_empty() && !name.ends_with('-') {
            name.push('-');
        }
        if name.chars().count() >= NAME_LENGTH {
            break;
        }
    }
    let name = name.trim_end_matches('-');
    let name = if name.is_empty() { "untitled" } else { name };
    let id = id.to_string();
    format!("{name}-{}.html", &id[..4])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{canonical_blocks, same_content, stored_blocks};
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
    fn a_stored_scene_is_an_html_file_that_reads_back_exactly() {
        let file = harbor();
        let text = file.to_html();
        assert_eq!(
            text,
            concat!(
                "<!doctype html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n",
                "<title>The Harbor &amp; the &quot;Tide&quot;</title>\n",
                "<meta name=\"id\" content=\"3f9a2c1e-0b7d-4e55-9a10-6c2f8d4b1a77\">\n",
                "<meta name=\"synopsis\" content=\"Mara is late.&#10;The boat is leaving.\">\n",
                "<meta name=\"status\" content=\"draft\">\n",
                "<meta name=\"goal\" content=\"Reach the boat before the tide\">\n",
                "<meta name=\"outcome\" content=\"The boat has gone\">\n",
                "<meta name=\"mood\" content=\"grey\">\n",
                "<meta name=\"also\" content=\"Mara\">\n",
                "<meta name=\"also\" content=\"the pilot\">\n",
                "<link rel=\"stylesheet\" href=\"../scene.css\">\n",
                "</head>\n<body>\n",
                "<p>Mara ran for the <strong>north </strong>quay.</p>\n",
                "<p>“Wait,” said the pilot.</p>\n",
                "</body>\n</html>\n",
            )
        );
        let read = SceneFile::parse_html(&text);
        assert_eq!(read.header, file.header);
        assert!(same_content(&read.blocks, &stored_blocks(&file.blocks)));
        assert_eq!(read.to_html(), text);
    }

    #[test]
    fn the_same_scene_exports_to_markdown_and_comes_back() {
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
    }

    #[test]
    fn a_file_written_by_hand_needs_no_head() {
        let read = SceneFile::parse_html("<h1>One</h1>\n<p>Just a scene.</p>\n");
        assert_eq!(read.header, SceneHeader::default());
        assert_eq!(read.blocks.len(), 2);
        let read = SceneFile::parse_markdown("# One\n\nJust a scene.\n").unwrap();
        assert_eq!(read.header, SceneHeader::default());
        assert_eq!(read.blocks.len(), 2);
    }

    #[test]
    fn file_names_are_readable_and_safe() {
        let id: Id = "3f9a2c1e-0b7d-4e55-9a10-6c2f8d4b1a77".parse().unwrap();
        assert_eq!(file_name("The Harbor", id), "the-harbor-3f9a.html");
        assert_eq!(file_name("  Où est / l'Été? ", id), "où-est-l-été-3f9a.html");
        assert_eq!(file_name("", id), "untitled-3f9a.html");
        assert_eq!(file_name("../..", id), "untitled-3f9a.html");
        let long = file_name(&"word ".repeat(30), id);
        assert!(long.chars().count() <= NAME_LENGTH + ".html".len() + 5);
        assert!(!long.contains("--"));
    }
}
