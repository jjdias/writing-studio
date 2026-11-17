//! Sync through git, with the `git` program the writer already has, and the
//! keys or tokens they set up for it. Nothing here stores a password.
//!
//! The writer sees one Sync button. Branches, staging, and rebasing stay
//! out of sight; a commit message is written from what changed.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime};

use writing_studio_core::{Error, Id, Result};
use writing_studio_document::Document;
use writing_studio_storage::{project_path, read_project_files, write_project_files};

use crate::{merge, Conflict};

/// How long fetching or pushing may take before it is given up.
const NETWORK_LIMIT: Duration = Duration::from_secs(120);
const REMOTE: &str = "origin";

/// What sync found and did.
#[derive(Clone, Debug, Default)]
pub struct SyncReport {
    /// This machine's changes were sent.
    pub sent: bool,
    /// The other machine's changes came in, so the project must be opened
    /// again to show them.
    pub received: bool,
    pub conflicts: Vec<Conflict>,
    /// Track-changes baselines for scenes where both versions were kept.
    pub baselines: Vec<Document>,
}

/// Where a connected project stands, without asking the network.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyncStatus {
    pub remote: String,
    /// When the project last heard from the remote.
    pub last_synced: Option<SystemTime>,
    /// Commits and changes here that the remote does not have.
    pub to_send: usize,
    /// Commits the remote had, when it was last asked, that are not here.
    pub to_receive: usize,
}

/// Whether the `git` program can be run.
pub fn available() -> bool {
    Command::new("git").arg("--version").output().is_ok_and(|out| out.status.success())
}

fn run(root: &Path, args: &[&str], limit: Option<Duration>) -> Result<String> {
    let mut command = Command::new("git");
    command
        .current_dir(root)
        .args(args)
        // Fail instead of waiting for a password the window cannot show.
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if std::env::var_os("GIT_SSH_COMMAND").is_none() {
        command.env("GIT_SSH_COMMAND", "ssh -o BatchMode=yes");
    }
    let mut child = command.spawn().map_err(|err| {
        Error::message(format!("git could not be run ({err}); install git to sync"))
    })?;
    let drain = |pipe: Option<Box<dyn Read + Send>>| {
        std::thread::spawn(move || {
            let mut text = String::new();
            if let Some(mut pipe) = pipe {
                let _ = pipe.read_to_string(&mut text);
            }
            text
        })
    };
    let out = drain(child.stdout.take().map(|p| Box::new(p) as Box<dyn Read + Send>));
    let err = drain(child.stderr.take().map(|p| Box::new(p) as Box<dyn Read + Send>));
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if limit.is_some_and(|limit| started.elapsed() > limit) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(Error::message("git took too long; check the network, then sync again"));
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let (out, err) = (out.join().unwrap_or_default(), err.join().unwrap_or_default());
    if status.success() {
        Ok(out.trim_end().to_string())
    } else {
        let said = err.trim();
        let said = if said.is_empty() { out.trim() } else { said };
        Err(Error::message(format!("git {}: {said}", args.first().unwrap_or(&""))))
    }
}

/// The folder must be the top of its own repository, so that a sync never
/// commits anything outside the project.
fn check_repository(root: &Path) -> Result<()> {
    let top = run(root, &["rev-parse", "--show-toplevel"], None)
        .map_err(|_| Error::message("this project is not connected for sync yet"))?;
    let same = fs::canonicalize(&top).ok() == fs::canonicalize(root).ok();
    if !same {
        return Err(Error::message(
            "this project is inside another git repository; it can only be synced from a repository of its own",
        ));
    }
    Ok(())
}

/// Whether the project has its own repository with a remote. Reads one
/// file and runs nothing, so the window can ask on every redraw.
pub fn is_connected(root: &Path) -> bool {
    fs::read_to_string(root.join(".git").join("config")).is_ok_and(|config| config.contains("[remote \"origin\"]"))
}

/// Make the project folder a repository, if it is not one, and connect it to
/// `remote`. The first sync sends the project there.
pub fn connect(root: &Path, remote: &str) -> Result<()> {
    let remote = remote.trim();
    if remote.is_empty() {
        return Err(Error::message("give the address of the repository to sync with"));
    }
    if !root.join(".git").exists() {
        run(root, &["init"], None)?;
        run(root, &["symbolic-ref", "HEAD", "refs/heads/main"], None)?;
    }
    check_repository(root)?;
    if run(root, &["remote", "get-url", REMOTE], None).is_ok() {
        run(root, &["remote", "set-url", REMOTE, remote], None)?;
    } else {
        run(root, &["remote", "add", REMOTE, remote], None)?;
    }
    Ok(())
}

pub fn status(root: &Path) -> Result<SyncStatus> {
    check_repository(root)?;
    let remote = run(root, &["remote", "get-url", REMOTE], None)?;
    let report = run(root, &["status", "--porcelain=v2", "--branch"], None)?;
    let (mut ahead, mut behind, mut changed) = (0, 0, 0);
    for line in report.lines() {
        if let Some(counts) = line.strip_prefix("# branch.ab ") {
            let mut parts = counts.split_whitespace();
            ahead = parts.next().and_then(|n| n.trim_start_matches('+').parse().ok()).unwrap_or(0);
            behind = parts.next().and_then(|n| n.trim_start_matches('-').parse().ok()).unwrap_or(0);
        } else if !line.starts_with('#') {
            changed += 1;
        }
    }
    let last_synced = fs::metadata(root.join(".git").join("FETCH_HEAD")).and_then(|m| m.modified()).ok();
    Ok(SyncStatus {
        remote,
        last_synced,
        to_send: ahead + usize::from(changed > 0),
        to_receive: behind,
    })
}

/// Commit everything that changed, with a message that names the scenes.
/// Returns whether there was anything to commit. Nothing goes over the
/// network.
pub fn commit_all(root: &Path) -> Result<bool> {
    check_repository(root)?;
    run(root, &["add", "-A"], None)?;
    let staged = run(root, &["diff", "--cached", "--name-status"], None)?;
    if staged.is_empty() {
        return Ok(false);
    }
    let message = commit_message(root, &staged);
    run(root, &["commit", "-q", "-m", &message], None).map_err(|err| {
        if err.to_string().contains("tell me who you are") {
            Error::message("git does not know your name and email yet; set them as the sync guide shows, then sync again")
        } else {
            err
        }
    })?;
    Ok(true)
}

/// "Changed: The Harbor, Dawn. Added: Dust. Removed: Cut." from the staged files.
fn commit_message(root: &Path, staged: &str) -> String {
    let mut lists: [(&str, Vec<String>); 3] = [("Changed", vec![]), ("Added", vec![]), ("Removed", vec![])];
    let mut other = false;
    for line in staged.lines() {
        let mut parts = line.split('\t');
        let (Some(kind), Some(path)) = (parts.next(), parts.last()) else {
            continue;
        };
        if !(path.starts_with("manuscript/") && path.ends_with(".html")) {
            other = true;
            continue;
        }
        let title = fs::read_to_string(root.join(path))
            .ok()
            .and_then(|text| {
                let start = text.find("<title>")? + "<title>".len();
                let end = text[start..].find("</title>")? + start;
                Some(text[start..end].replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\""))
            })
            .unwrap_or_else(|| path.trim_start_matches("manuscript/").to_string());
        let slot = match kind.chars().next() {
            Some('A') => 1,
            Some('D') => 2,
            _ => 0,
        };
        lists[slot].1.push(title);
    }
    let mut parts: Vec<String> = lists
        .iter()
        .filter(|(_, names)| !names.is_empty())
        .map(|(label, names)| {
            let shown: Vec<&str> = names.iter().take(8).map(String::as_str).collect();
            let more = names.len().saturating_sub(shown.len());
            let tail = if more > 0 { format!(", and {more} more") } else { String::new() };
            format!("{label}: {}{tail}.", shown.join(", "))
        })
        .collect();
    if other || parts.is_empty() {
        parts.push("Notes, structure, or settings changed.".into());
    }
    format!("Writing Studio: {}", parts.join(" "))
}

/// One commit that changed a file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Revision {
    pub commit: String,
    pub date: String,
    pub message: String,
}

/// The commits that changed `path`, newest first, at most fifty. Nothing
/// goes over the network.
pub fn revisions(root: &Path, path: &str) -> Result<Vec<Revision>> {
    check_repository(root)?;
    let log = run(
        root,
        &["log", "-n", "50", "--date=format:%Y-%m-%d %H:%M", "--format=%H%x1f%cd%x1f%s", "--", path],
        None,
    )?;
    Ok(log
        .lines()
        .filter_map(|line| {
            let mut parts = line.split('\u{1f}');
            Some(Revision {
                commit: parts.next()?.to_string(),
                date: parts.next()?.to_string(),
                message: parts.next().unwrap_or_default().to_string(),
            })
        })
        .collect())
}

/// What `path` held at `commit`.
pub fn file_at(root: &Path, commit: &str, path: &str) -> Result<String> {
    if commit.is_empty() || !commit.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err(Error::message("that is not a commit"));
    }
    check_repository(root)?;
    run(root, &["show", &format!("{commit}:{path}")], None)
}

fn branch(root: &Path) -> Result<String> {
    run(root, &["symbolic-ref", "--short", "HEAD"], None)
}

/// Save this machine's changes as a commit, bring in the other machine's,
/// merge them if both changed, and send the result. The caller saves the
/// project first, and opens it again when `received` is set.
pub fn sync(root: &Path) -> Result<SyncReport> {
    check_repository(root)?;
    let git = root.join(".git");
    if ["MERGE_HEAD", "rebase-merge", "rebase-apply"].iter().any(|name| git.join(name).exists()) {
        return Err(Error::message(
            "git is in the middle of a merge or rebase in this project; finish it with git first",
        ));
    }
    let mut report = SyncReport::default();
    commit_all(root)?;
    let branch = branch(root)?;
    let upstream = format!("{REMOTE}/{branch}");
    for attempt in 0..2 {
        run(root, &["fetch", "-q", REMOTE], Some(NETWORK_LIMIT))?;
        let known = run(root, &["rev-parse", "--verify", "-q", &upstream], None).is_ok();
        if known {
            let counts = run(root, &["rev-list", "--left-right", "--count", &format!("HEAD...{upstream}")], None)?;
            let mut numbers = counts.split_whitespace().map(|n| n.parse::<usize>().unwrap_or(0));
            let (ahead, behind) = (numbers.next().unwrap_or(0), numbers.next().unwrap_or(0));
            if behind > 0 && ahead == 0 {
                run(root, &["merge", "-q", "--ff-only", &upstream], None)?;
                report.received = true;
                return Ok(report);
            }
            if behind > 0 {
                merge_with(root, &upstream, &mut report)?;
            } else if ahead == 0 {
                return Ok(report);
            }
        }
        let pushed = run(root, &["push", "-q", "-u", REMOTE, &format!("HEAD:{branch}")], Some(NETWORK_LIMIT));
        match pushed {
            Ok(_) => {
                report.sent = true;
                return Ok(report);
            }
            // The remote moved while this ran: fetch and merge again, once.
            Err(err) if attempt == 0 && err.to_string().contains("rejected") => continue,
            Err(err) => return Err(err),
        }
    }
    Ok(report)
}

/// Both sides have commits: merge with this program, not with git, and
/// record the result as a merge commit.
fn merge_with(root: &Path, upstream: &str, report: &mut SyncReport) -> Result<()> {
    let base_rev = run(root, &["merge-base", "HEAD", upstream], None)?;
    let scratch = std::env::temp_dir().join(format!("ws-sync-{}", Id::new()));
    let (base_dir, theirs_dir) = (scratch.join("base"), scratch.join("theirs"));
    let result = (|| -> Result<()> {
        for (dir, rev) in [(&base_dir, base_rev.as_str()), (&theirs_dir, upstream)] {
            let dir = dir.to_string_lossy().into_owned();
            run(root, &["worktree", "add", "-q", "--detach", &dir, rev], None)?;
        }
        let ours = read_project_files(root)?;
        let merged = merge(&read_project_files(&base_dir)?, &ours, &read_project_files(&theirs_dir)?)?;
        // Keep this machine's tree for now; the merged files replace it below.
        run(root, &["merge", "-q", "--no-commit", "--no-ff", "-s", "ours", upstream], None)?;
        let written = write_project_files(root, &merged.content)?;
        let kept: HashSet<&String> = written.values().collect();
        for path in ours.files.values().filter(|path| !kept.contains(path)) {
            let _ = fs::remove_file(root.join(path));
        }
        merge_attachments(&base_dir, root, &theirs_dir, &mut report.conflicts)?;
        run(root, &["add", "-A"], None)?;
        run(root, &["commit", "-q", "-m", "Writing Studio: merged the changes from another machine."], None)?;
        report.received = true;
        report.conflicts.extend(merged.conflicts);
        report.baselines = merged.baselines;
        Ok(())
    })();
    if result.is_err() && root.join(".git").join("MERGE_HEAD").exists() {
        let _ = run(root, &["merge", "--abort"], None);
    }
    for dir in [&base_dir, &theirs_dir] {
        let _ = run(root, &["worktree", "remove", "--force", &dir.to_string_lossy()], None);
    }
    let _ = fs::remove_dir_all(&scratch);
    result
}

/// Research files are whole files. One changed on one side is taken; one
/// changed differently on both keeps this machine's, and the other's is put
/// beside it with `-theirs` before the extension. A link is never followed:
/// one that came through git could point anywhere.
fn merge_attachments(base: &Path, ours: &Path, theirs: &Path, conflicts: &mut Vec<Conflict>) -> Result<()> {
    let list = |root: &Path| -> HashMap<String, Vec<u8>> {
        let mut out = HashMap::new();
        let mut stack = vec![root.join("attachments")];
        while let Some(dir) = stack.pop() {
            for entry in fs::read_dir(&dir).into_iter().flatten().flatten() {
                let path = entry.path();
                let Ok(kind) = entry.file_type() else { continue };
                if kind.is_dir() {
                    stack.push(path);
                } else if !kind.is_file() {
                    continue;
                } else if let (Ok(bytes), Ok(relative)) = (fs::read(&path), path.strip_prefix(root)) {
                    out.insert(relative.to_string_lossy().replace('\\', "/"), bytes);
                }
            }
        }
        out
    };
    let (b, o, t) = (list(base), list(ours), list(theirs));
    let names: HashSet<&String> = b.keys().chain(o.keys()).chain(t.keys()).collect();
    for name in names {
        let target = project_path(ours, name)?;
        match (b.get(name), o.get(name), t.get(name)) {
            (Some(bv), Some(ov), None) if ov == bv => fs::remove_file(&target)?,
            (_, None, Some(tv)) if b.get(name) != Some(tv) || b.get(name).is_none() => {
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&target, tv)?;
            }
            (bv, Some(ov), Some(tv)) if ov != tv => {
                if bv == Some(ov) {
                    fs::write(&target, tv)?;
                } else if bv != Some(tv) {
                    let beside: PathBuf = match name.rsplit_once('.') {
                        Some((stem, ext)) if !ext.contains('/') => project_path(ours, &format!("{stem}-theirs.{ext}"))?,
                        _ => project_path(ours, &format!("{name}-theirs"))?,
                    };
                    fs::write(&beside, tv)?;
                    conflicts.push(Conflict {
                        item: Id::nil(),
                        what: format!("{name} was changed on both machines; the other machine's is beside it, ending in -theirs"),
                    });
                }
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use writing_studio_project::NodeKind;
    use writing_studio_storage::ProjectStore;

    fn temp() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ws-git-{}", Id::new()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn who(dir: &Path) {
        run(dir, &["config", "user.name", "Test Writer"], None).unwrap();
        run(dir, &["config", "user.email", "test@example.com"], None).unwrap();
    }

    /// A remote, and the first machine with a project sent to it.
    fn first_machine(top: &Path) -> (PathBuf, PathBuf) {
        let remote = top.join("remote.git");
        run(top, &["init", "-q", "--bare", "--initial-branch=main", &remote.to_string_lossy()], None).unwrap();
        let a = top.join("a.writingstudio");
        ProjectStore::create(&a, "Shared").unwrap();
        connect(&a, &remote.to_string_lossy()).unwrap();
        who(&a);
        let report = sync(&a).unwrap();
        assert!(report.sent && !report.received);
        (remote, a)
    }

    fn second_machine(top: &Path, remote: &Path) -> PathBuf {
        let b = top.join("b.writingstudio");
        run(top, &["clone", "-q", &remote.to_string_lossy(), &b.to_string_lossy()], None).unwrap();
        who(&b);
        b
    }

    fn scene(root: &Path) -> (ProjectStore, Id) {
        let store = ProjectStore::open(root).unwrap();
        let id = store.load_binder().unwrap().nodes.into_iter().find(|n| n.kind == NodeKind::Scene).unwrap().id;
        (store, id)
    }

    fn write(root: &Path, text: &str) {
        let (store, id) = scene(root);
        store.save_document(&Document::from_plain_text(id, text)).unwrap();
    }

    fn read(root: &Path) -> String {
        let (store, id) = scene(root);
        store.load_document(id).unwrap().to_plain_text()
    }

    #[test]
    fn two_machines_keep_one_project_in_step() {
        let top = temp();
        let (remote, a) = first_machine(&top);
        let nothing = sync(&a).unwrap();
        assert!(!nothing.sent && !nothing.received);
        let b = second_machine(&top, &remote);
        assert!(is_connected(&b));
        assert!(b.join(".gitattributes").exists() && !b.join(".local").exists());

        // Written on one machine, read on the other.
        write(&b, "One.\nTwo.\nThree.");
        assert!(sync(&b).unwrap().sent);
        assert!(sync(&a).unwrap().received);
        assert_eq!(read(&a), "One.\nTwo.\nThree.");
        let message = run(&b, &["log", "-1", "--format=%s"], None).unwrap();
        assert!(message.contains("Changed: Scene 1."), "{message}");

        // Different scenes on each machine: both are kept, on both.
        write(&a, "One, from A.\nTwo.\nThree.");
        let added = {
            let store = ProjectStore::open(&b).unwrap();
            let chapter = store.load_binder().unwrap().nodes.into_iter().find(|n| n.kind == NodeKind::Chapter).unwrap();
            let node = writing_studio_project::BinderNode::new(NodeKind::Scene, "From B", Some(chapter.id), 1);
            store.upsert_node(&node).unwrap();
            store.save_document(&Document::from_plain_text(node.id, "Written on B.")).unwrap();
            node.id
        };
        assert!(sync(&a).unwrap().sent);
        let merged = sync(&b).unwrap();
        assert!(merged.received && merged.sent && merged.conflicts.is_empty(), "{merged:?}");
        assert!(sync(&a).unwrap().received);
        for machine in [&a, &b] {
            assert_eq!(read(machine), "One, from A.\nTwo.\nThree.");
            let store = ProjectStore::open(machine).unwrap();
            assert_eq!(store.load_document(added).unwrap().to_plain_text(), "Written on B.");
            assert!(store.problems().is_empty(), "{:?}", store.problems());
        }

        // The same paragraph on both: both are kept, with a baseline.
        write(&a, "One, from A.\nTwo, from A.\nThree.");
        write(&b, "One, from A.\nTwo, from B.\nThree.");
        sync(&a).unwrap();
        let clash = sync(&b).unwrap();
        assert_eq!(clash.baselines.len(), 1);
        assert!(!clash.conflicts.is_empty());
        assert_eq!(read(&b), "One, from A.\nTwo, from B.\nTwo, from A.\nThree.");
        assert!(status(&b).unwrap().to_send == 0 && status(&b).unwrap().last_synced.is_some());
        let _ = fs::remove_dir_all(top);
    }

    #[cfg(unix)]
    #[test]
    fn research_files_that_are_links_are_not_followed() {
        use std::os::unix::fs::symlink;
        let top = temp();
        let (base, ours, theirs, outside) = (top.join("base"), top.join("ours"), top.join("theirs"), top.join("outside"));
        for dir in [&base, &ours, &theirs] {
            fs::create_dir_all(dir.join("attachments/note")).unwrap();
        }
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("secret.txt"), "not the book's").unwrap();
        // The other machine's copy holds a link to a file, and to a folder, outside.
        symlink(outside.join("secret.txt"), theirs.join("attachments/note/secret.txt")).unwrap();
        symlink(&outside, theirs.join("attachments/elsewhere")).unwrap();
        fs::write(theirs.join("attachments/note/map.png"), "a map").unwrap();
        merge_attachments(&base, &ours, &theirs, &mut Vec::new()).unwrap();
        assert!(ours.join("attachments/note/map.png").exists());
        assert!(!ours.join("attachments/note/secret.txt").exists());
        assert!(!ours.join("attachments/elsewhere").exists());

        // Here, a folder that is a link outside: a file arriving into it is refused.
        symlink(&outside, ours.join("attachments/out")).unwrap();
        fs::create_dir_all(theirs.join("attachments/out")).unwrap();
        fs::write(theirs.join("attachments/out/new.txt"), "new").unwrap();
        let error = merge_attachments(&base, &ours, &theirs, &mut Vec::new()).unwrap_err().to_string();
        assert!(error.contains("through a link"), "{error}");
        assert!(!outside.join("new.txt").exists());
        let _ = fs::remove_dir_all(top);
    }

    #[test]
    fn sync_refuses_what_it_cannot_do_safely() {
        let top = temp();
        let lone = top.join("lone.writingstudio");
        ProjectStore::create(&lone, "Alone").unwrap();
        assert!(!is_connected(&lone));
        let error = sync(&lone).unwrap_err().to_string();
        assert!(error.contains("not connected"), "{error}");
        assert!(connect(&lone, "  ").is_err());

        let (_, a) = first_machine(&top);
        fs::write(a.join(".git").join("MERGE_HEAD"), "0000").unwrap();
        let error = sync(&a).unwrap_err().to_string();
        assert!(error.contains("middle of a merge"), "{error}");
        let _ = fs::remove_dir_all(top);
    }
}
