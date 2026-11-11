//! TOOL52 — the built xtask binary, started by path from another repository,
//! refuses its writing commands and commits nothing anywhere; the 2026-11-11
//! incident (TASK/TOOL51-STATUS-DONE.MD, *Correction*) reproduced end to end.
//!
//! The commands use ticket ids that do not exist, so even a binary without the
//! guard could not commit here: it would fail later with another error, which
//! these tests tell apart from the refusal.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The workspace this binary was built for.
fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()
}

fn git(dir: &Path, args: &[&str]) -> String {
    let mut c = Command::new("git");
    for v in ["GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE", "GIT_COMMON_DIR"] {
        c.env_remove(v);
    }
    let o = c
        .args(["-c", "user.name=t", "-c", "user.email=t@t", "-c", "commit.gpgsign=false"])
        .args(["-c", "core.hooksPath=/dev/null"])
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(o.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&o.stderr));
    String::from_utf8_lossy(&o.stdout).trim().to_string()
}

/// A scratch repository with one commit under the workspace's `target/`
/// (in the tree, like the 2026-11-11 clone under `target-agents/`).
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Scratch {
        let dir = workspace().join("target").join(format!("xtask-it-repo-guard-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("TASK")).unwrap();
        let s = Scratch(dir.canonicalize().unwrap());
        fs::write(s.0.join("TASK/README.md"), "# Tickets\n").unwrap();
        git(&s.0, &["init", "-q"]);
        git(&s.0, &["add", "-A"]);
        git(&s.0, &["commit", "-q", "-m", "init"]);
        s
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn xtask(cwd: &Path, args: &[&str]) -> Output {
    let mut c = Command::new(env!("CARGO_BIN_EXE_xtask"));
    for v in ["GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE", "GIT_COMMON_DIR"] {
        c.env_remove(v);
    }
    c.args(args).current_dir(cwd).output().unwrap()
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

#[test]
fn the_main_tree_binary_run_from_a_clone_commits_nowhere() {
    let ws = workspace();
    let clone = Scratch::new("clone");
    let (ws_head, clone_head) = (git(&ws, &["rev-parse", "HEAD"]), git(&clone.0, &["rev-parse", "HEAD"]));
    let cases: [&[&str]; 3] = [
        // The 2026-11-11 command, with an id that exists nowhere.
        &["accept", "ZZTOOL52", "--commit", "HEAD", "--result", "PASS", "--note", "x", "--agent", "dev-a"],
        &["set-status", "ZZTOOL52", "PARKED", "--reason", "x"],
        &["land", "zz-tool52-no-branch", "-F", "zz-no-file"],
    ];
    for args in cases {
        let o = xtask(&clone.0, args);
        let e = stderr(&o);
        assert!(!o.status.success(), "{args:?} succeeded from the clone: {e}");
        assert!(e.contains("refused (TOOL52)"), "{args:?} was not refused by the guard: {e}");
        assert!(e.contains(&ws.display().to_string()), "{args:?}: the binary's root missing: {e}");
        assert!(e.contains(&clone.0.display().to_string()), "{args:?}: the clone missing: {e}");
    }
    assert_eq!(git(&ws, &["rev-parse", "HEAD"]), ws_head, "a commit landed in the binary's repository");
    assert_eq!(git(&clone.0, &["rev-parse", "HEAD"]), clone_head, "a commit landed in the clone");
}

#[test]
fn inside_its_own_repository_the_command_runs() {
    let ws = workspace();
    let head = git(&ws, &["rev-parse", "HEAD"]);
    // Past the guard, accept fails on the unknown ticket (or on staged work),
    // not with the refusal; from a subdirectory too.
    for cwd in [ws.clone(), ws.join("tools")] {
        let o = xtask(&cwd, &["accept", "ZZTOOL52", "--commit", "HEAD", "--result", "PASS", "--agent", "zz"]);
        let e = stderr(&o);
        assert!(!o.status.success(), "{e}");
        assert!(e.contains("accept:") && !e.contains("refused (TOOL52)"), "{}: {e}", cwd.display());
    }
    assert_eq!(git(&ws, &["rev-parse", "HEAD"]), head);
}

#[test]
fn read_only_commands_run_from_anywhere() {
    let clone = Scratch::new("ro");
    let o = xtask(&clone.0, &["help"]);
    assert!(o.status.success(), "{}", stderr(&o));
}
