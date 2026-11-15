//! Check that every scene of a project can be kept as an HTML scene file.
//!
//! ```text
//! cargo run -p writing-studio-format --example check_project -- path/to/Novel.writingstudio
//! ```
//!
//! Each scene is written as a scene file, read back, and compared with the
//! scene in the project. The project is only read. Add `--write folder` to
//! also save the files, each with its Markdown export beside it, to look at.

use std::path::PathBuf;
use std::process::ExitCode;

use writing_studio_format::{file_name, same_content, stored_blocks, SceneFile, SceneHeader};
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
    let (mut scenes, mut failed) = (0, 0);
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
        let text = file.to_html();
        let name = file_name(&node.title, node.id);
        if let Some(folder) = &write_to {
            std::fs::write(folder.join(&name), &text)?;
            std::fs::write(folder.join(&name).with_extension("md"), file.to_markdown())?;
        }
        let read = SceneFile::parse_html(&text);
        let same = read.header == file.header
            && same_content(&read.blocks, &stored_blocks(&document.blocks))
            && read.to_html() == text;
        if !same {
            failed += 1;
            println!("FAILED  {name}: “{}” does not read back the same", node.title);
        }
    }
    println!(
        "{scenes} scenes checked. {} read back exactly. {failed} failed.",
        scenes - failed
    );
    Ok(failed == 0)
}
