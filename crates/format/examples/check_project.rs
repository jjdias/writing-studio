//! Check that every scene of a project can be kept as a Markdown file.
//!
//! ```text
//! cargo run -p writing-studio-format --example check_project -- path/to/Novel.writingstudio
//! ```
//!
//! Each scene is written as a scene file, read back, and compared with what
//! was written. The project is only read. Add `--write folder` to also save
//! the files, to look at them.

use std::path::PathBuf;
use std::process::ExitCode;

use writing_studio_document::Block;
use writing_studio_format::{canonical_blocks, same_content, SceneFile, SceneHeader, file_name};
use writing_studio_storage::ProjectStore;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let Some(project) = args.next() else {
        eprintln!("usage: check_project <project folder> [--write <folder>]");
        return ExitCode::from(2);
    };
    let write_to: Option<PathBuf> = match (args.next().as_deref(), args.next()) {
        (Some("--write"), Some(folder)) => Some(folder.into()),
        _ => None,
    };
    match check(&project, write_to) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(err) => {
            eprintln!("could not check {project}: {err}");
            ExitCode::from(2)
        }
    }
}

fn check(project: &str, write_to: Option<PathBuf>) -> Result<bool, Box<dyn std::error::Error>> {
    let store = ProjectStore::open(project)?;
    let binder = store.load_binder()?;
    let beats = store.load_scene_beats()?;
    if let Some(folder) = &write_to {
        std::fs::create_dir_all(folder)?;
    }
    let (mut scenes, mut adjusted, mut failed) = (0, 0, 0);
    for node in &binder.nodes {
        let Some(document_id) = node.document_id else {
            continue;
        };
        let Ok(document) = store.load_document(document_id) else {
            continue;
        };
        scenes += 1;
        let beat = beats.get(&node.id).cloned().unwrap_or_default();
        let file = SceneFile {
            header: SceneHeader {
                id: Some(node.id),
                title: node.title.clone(),
                synopsis: node.synopsis.clone(),
                status: node.status.clone(),
                label: node.label.clone(),
                color: node.color.clone(),
                goal: beat.goal,
                conflict: beat.conflict,
                outcome: beat.outcome,
                ..SceneHeader::default()
            },
            blocks: document.blocks.clone(),
        };
        let text = file.to_text();
        let name = file_name(&node.title, node.id);
        if let Some(folder) = &write_to {
            std::fs::write(folder.join(&name), &text)?;
        }
        let wanted = canonical_blocks(&document.blocks);
        let same = match SceneFile::parse(&text) {
            Ok(read) => {
                read.header == file.header
                    && same_content(&read.blocks, &wanted)
                    && read.to_text() == text
            }
            Err(err) => {
                println!("FAILED  {name}: the header does not read back: {err}");
                false
            }
        };
        if !same {
            failed += 1;
            println!("FAILED  {name}: “{}” does not read back the same", node.title);
        } else if !same_content(&wanted, &merged(&document.blocks)) {
            // The file is exact, but it holds a little less than the
            // database did: see the list in `canonical.rs`.
            adjusted += 1;
            println!("adjusted {name}: “{}”", node.title);
        }
    }
    println!(
        "{scenes} scenes checked. {} read back exactly. {adjusted} of those lose formatting that \
         Markdown cannot hold (bold or italic on a space at the edge of a stretch, a line break at \
         the end of a paragraph, or formatting inside a code block). {failed} failed.",
        scenes - failed
    );
    Ok(failed == 0)
}

/// The document as stored, with nothing changed except that the comparison
/// ignores how the text is cut into runs.
fn merged(blocks: &[Block]) -> Vec<Block> {
    use writing_studio_document::{BlockKind, Inline, ListItem};
    fn join(inlines: &[Inline]) -> Vec<Inline> {
        let mut out: Vec<Inline> = Vec::new();
        for inline in inlines.iter().filter(|inline| !inline.text.is_empty()) {
            match out.last_mut() {
                Some(last) if last.marks == inline.marks => last.text.push_str(&inline.text),
                _ => out.push(inline.clone()),
            }
        }
        out
    }
    blocks
        .iter()
        .map(|block| Block {
            id: block.id,
            kind: match &block.kind {
                BlockKind::Paragraph { inlines } => BlockKind::Paragraph { inlines: join(inlines) },
                BlockKind::Heading { level, inlines } => BlockKind::Heading {
                    level: *level,
                    inlines: join(inlines),
                },
                BlockKind::Quote { inlines } => BlockKind::Quote { inlines: join(inlines) },
                BlockKind::Code { inlines } => BlockKind::Code { inlines: join(inlines) },
                BlockKind::List { ordered, items } => BlockKind::List {
                    ordered: *ordered,
                    items: items
                        .iter()
                        .map(|item| ListItem {
                            id: item.id,
                            inlines: join(&item.inlines),
                        })
                        .collect(),
                },
                BlockKind::SceneBreak => BlockKind::SceneBreak,
            },
        })
        .collect()
}
