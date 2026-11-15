//! Write a copy of a project as format 2 files, to look at.
//!
//! ```text
//! cargo run -p writing-studio-storage --example to_format_2 -- Novel.writingstudio some/new/folder
//! ```
//!
//! The project is only read. The new folder must not exist yet. The copy is
//! checked by reading its files back and comparing them with the project.
//! The window offers the same when it opens an older project.

use std::process::ExitCode;

use writing_studio_storage::ProjectStore;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [project, target] = args.as_slice() else {
        eprintln!("usage: to_format_2 <project folder> <new folder>");
        return ExitCode::from(2);
    };
    match ProjectStore::convert_copy(project, target) {
        Ok(files) => {
            println!("Wrote {files} scene and bible files, checked against the project.");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("not converted: {err}");
            ExitCode::FAILURE
        }
    }
}
