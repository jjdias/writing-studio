//! Write a copy of a project as format 2 files, to look at.
//!
//! ```text
//! cargo run -p writing-studio-storage --example to_format_2 -- Novel.writingstudio some/new/folder
//! ```
//!
//! The project is only read. The new folder must not exist yet. The copy is
//! checked by reading its files back and comparing them with the project.
//! The desktop app cannot open the copy yet.

use std::path::Path;
use std::process::ExitCode;

use writing_studio_storage::{
    verify_project_files, write_format_2_marker, write_project_files, ProjectContent, ProjectStore,
};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [project, target] = args.as_slice() else {
        eprintln!("usage: to_format_2 <project folder> <new folder>");
        return ExitCode::from(2);
    };
    match convert(Path::new(project), Path::new(target)) {
        Ok(summary) => {
            println!("{summary}");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("not converted: {err}");
            ExitCode::FAILURE
        }
    }
}

fn convert(project: &Path, target: &Path) -> Result<String, Box<dyn std::error::Error>> {
    if target.exists() {
        return Err("the new folder already exists; choose a name that is not taken".into());
    }
    let content = ProjectContent::from_store(&ProjectStore::open_as_is(project)?)?;
    let files = write_project_files(target, &content)?;
    verify_project_files(target, &content)?;
    write_format_2_marker(target)?;
    let mut copied = 0;
    for folder in ["attachments", "dictionaries"] {
        copied += copy_folder(&project.join(folder), &target.join(folder))?;
    }
    Ok(format!(
        "Wrote {} scene and bible files, checked against the project, and copied {copied} other files.",
        files.len()
    ))
}

fn copy_folder(from: &Path, to: &Path) -> std::io::Result<usize> {
    if !from.is_dir() {
        return Ok(0);
    }
    std::fs::create_dir_all(to)?;
    let mut copied = 0;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copied += copy_folder(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), target)?;
            copied += 1;
        }
    }
    Ok(copied)
}
