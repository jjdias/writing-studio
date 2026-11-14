//! A scene file: a header, then the scene as Markdown.

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
    /// The text, in canonical form.
    pub blocks: Vec<Block>,
}

impl SceneFile {
    pub fn to_text(&self) -> String {
        let scene = &self.header;
        let mut header = Header::default();
        if let Some(id) = scene.id {
            header.set_text("id", &id.to_string());
        }
        for (key, value) in [
            ("title", &scene.title),
            ("synopsis", &scene.synopsis),
            ("status", &scene.status),
            ("label", &scene.label),
            ("color", &scene.color),
            ("goal", &scene.goal),
            ("conflict", &scene.conflict),
            ("outcome", &scene.outcome),
        ] {
            header.set_text(key, value);
        }
        header.entries.extend(scene.other.entries.iter().cloned());
        format!("{}\n{}", header.to_text(), blocks_to_markdown(&self.blocks))
    }

    pub fn parse(file: &str) -> Result<Self, HeaderError> {
        let (mut header, body) = split_header(file)?;
        let id = header.take_text("id");
        let scene = SceneHeader {
            id: id.trim().parse().ok(),
            title: header.take_text("title"),
            synopsis: header.take_text("synopsis"),
            status: header.take_text("status"),
            label: header.take_text("label"),
            color: header.take_text("color"),
            goal: header.take_text("goal"),
            conflict: header.take_text("conflict"),
            outcome: header.take_text("outcome"),
            other: header,
        };
        Ok(Self {
            header: scene,
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
    format!("{name}-{}.md", &id[..4])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::header::Value;
    use crate::same_content;
    use writing_studio_document::Document;

    #[test]
    fn a_scene_file_is_a_header_and_markdown() {
        let id: Id = "3f9a2c1e-0b7d-4e55-9a10-6c2f8d4b1a77".parse().unwrap();
        let mut document = Document::new(Id::new());
        document.replace_html("<p>Mara ran for the harbor.</p><p>“Wait,” said the pilot.</p>");
        let mut other = Header::default();
        other.entries.push(("mood".into(), Value::Text("grey".into())));
        let file = SceneFile {
            header: SceneHeader {
                id: Some(id),
                title: "The Harbor".into(),
                synopsis: "Mara is late.\nThe boat is leaving.".into(),
                status: "draft".into(),
                goal: "Reach the boat before the tide".into(),
                outcome: "The boat has gone".into(),
                other,
                ..SceneHeader::default()
            },
            blocks: document.blocks.clone(),
        };
        let text = file.to_text();
        assert_eq!(
            text,
            concat!(
                "---\n",
                "id: 3f9a2c1e-0b7d-4e55-9a10-6c2f8d4b1a77\n",
                "title: The Harbor\n",
                "synopsis: |-\n  Mara is late.\n  The boat is leaving.\n",
                "status: draft\n",
                "goal: Reach the boat before the tide\n",
                "outcome: The boat has gone\n",
                "mood: grey\n",
                "---\n",
                "\n",
                "Mara ran for the harbor.\n",
                "\n",
                "“Wait,” said the pilot.\n",
            )
        );
        let read = SceneFile::parse(&text).unwrap();
        assert_eq!(read.header, file.header);
        assert!(same_content(&read.blocks, &crate::canonical_blocks(&file.blocks)));
        assert_eq!(read.to_text(), text);
    }

    #[test]
    fn a_file_written_by_hand_needs_no_header() {
        let read = SceneFile::parse("# One\n\nJust a scene.\n").unwrap();
        assert_eq!(read.header, SceneHeader::default());
        assert_eq!(read.blocks.len(), 2);
    }

    #[test]
    fn file_names_are_readable_and_safe() {
        let id: Id = "3f9a2c1e-0b7d-4e55-9a10-6c2f8d4b1a77".parse().unwrap();
        assert_eq!(file_name("The Harbor", id), "the-harbor-3f9a.md");
        assert_eq!(file_name("  Où est / l'Été? ", id), "où-est-l-été-3f9a.md");
        assert_eq!(file_name("", id), "untitled-3f9a.md");
        assert_eq!(file_name("../..", id), "untitled-3f9a.md");
        let long = file_name(&"word ".repeat(30), id);
        assert!(long.chars().count() <= NAME_LENGTH + ".md".len() + 5);
        assert!(!long.contains("--"));
    }
}
