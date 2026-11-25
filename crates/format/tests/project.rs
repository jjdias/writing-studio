//! Scenes from a project on disk, not only documents built in a test.

use std::path::PathBuf;

use writing_studio_core::Id;
use writing_studio_format::{canonical_blocks, file_name, same_content, SceneFile, SceneHeader};
use writing_studio_project::NodeKind;
use writing_studio_storage::ProjectStore;

/// A project with the kinds of text the editor produces. Set
/// `WS_KEEP_PROJECT` to a folder to keep it, to look at its files.
fn sample_project() -> (ProjectStore, PathBuf, bool) {
    let kept = std::env::var_os("WS_KEEP_PROJECT").map(PathBuf::from);
    let dir = kept
        .clone()
        .unwrap_or_else(|| std::env::temp_dir().join(format!("ws-format-{}", Id::new())));
    let _ = std::fs::remove_dir_all(&dir);
    let store = ProjectStore::create(&dir, "Format Novel").unwrap();
    let mut tree = store.load_binder().unwrap();
    let scene = tree
        .nodes
        .iter_mut()
        .find(|node| node.kind == NodeKind::Scene)
        .unwrap();
    scene.title = "The Harbor: Dawn".into();
    scene.context = "Mara is late.\nThe boat is leaving.\n\n## Goal\n\nReach the boat. [[The Pilot]] has gone.".into();
    scene.status = "draft".into();
    scene.label = "1".into();
    let document_id = scene.document_id.unwrap();
    store.persist_binder(&tree).unwrap();

    let mut document = store.load_document(document_id).unwrap();
    document.replace_html(concat!(
        "<h1>Dawn</h1>",
        "<p>Mara ran for the <strong>north </strong>quay, <em>again</em>. 2 + 2 = 4 * 1.</p>",
        "<p><span class=\"ws-comment\" data-comment=\"which one?\" data-kind=\"question\">The ",
        "<mark data-highlight=\"green\">harbor</mark></span> was <u>very</u> <s>quiet</s>.</p>",
        "<blockquote>Tide tables are a promise.<br>— the pilot</blockquote>",
        "<ol><li>rope</li><li><a href=\"https://example.com/a_(b)\">lantern</a></li></ol>",
        "<pre>depth = 3.5\n  # fathoms</pre>",
        "<hr>",
        "<p>- She did not look back.</p>",
    ));
    store.save_document(&document).unwrap();
    (store, dir, kept.is_some())
}

#[test]
fn every_scene_of_a_project_reads_back_from_its_file() {
    let (store, dir, keep) = sample_project();
    let binder = store.load_binder().unwrap();
    let mut checked = 0;
    let mut names = std::collections::HashSet::new();
    for node in &binder.nodes {
        let Some(document_id) = node.document_id else {
            continue;
        };
        // An item that has never been opened has no text yet.
        let Ok(document) = store.load_document(document_id) else {
            continue;
        };
        let file = SceneFile {
            header: SceneHeader {
                id: Some(node.id),
                title: node.title.clone(),
                context: node.context.clone(),
                status: node.status.clone(),
                label: node.label.clone(),
                color: node.color.clone(),
                ..SceneHeader::default()
            },
            blocks: document.blocks.clone(),
        };
        let text = file.to_markdown();
        let read = SceneFile::parse_markdown(&text).unwrap();
        assert_eq!(read.header, file.header, "{text}");
        assert!(
            same_content(&read.blocks, &canonical_blocks(&document.blocks)),
            "{text}"
        );
        assert_eq!(read.to_markdown(), text);
        assert!(names.insert(file_name(&node.title, node.id)));
        checked += 1;
    }
    assert!(checked >= 1);
    if !keep {
        let _ = std::fs::remove_dir_all(dir);
    }
}
