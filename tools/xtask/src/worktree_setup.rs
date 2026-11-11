//! `xtask worktree-setup [--check]` (TOOL44) — give a linked git worktree
//! the gitignored inputs the main tree has: a symlink per missing input,
//! pointing at the main tree's copy.
//!
//! # Which inputs
//!
//! An input is a directory directly under one of [`INPUT_DIRS`] in the main
//! tree that the main tree's ignore rules ignore *as a directory* (`git
//! check-ignore <path>/`) and that holds no tracked file there (`git
//! ls-files` empty). Today that derives the `references/*` checkouts (`.gitignore`:
//! `/references/*`) and the vendored `sdk/apps/<app>/` trees
//! (`sdk/.gitignore`: `/apps/*/`, minus the negated, tracked `apitest/`,
//! `chaintest/`, ...); no name is hard-coded. A directory with tracked
//! content (`sdk/apps/doom/`) is never an input: a link there would replace
//! tracked files.
//!
//! # Never destructive
//!
//! An existing correct link is left alone. A link elsewhere (or dangling) and
//! a non-directory are reported and make the command fail; a real directory
//! (a copy) is reported and kept. Nothing is ever replaced or deleted.
//!
//! # Why `git status` stays clean
//!
//! git records a symlink as a file, so a dir-only rule does not match it:
//! `/references/*` covers a link `references/emutos`, but `sdk/.gitignore`'s
//! `/apps/*/` does not cover a link `sdk/apps/teradesk` (it shows as `??`).
//! For every link the worktree's rules do not ignore, the command appends the
//! anchored path (`/sdk/apps/teradesk`) to `info/exclude` of the repository's
//! common git dir ([`EXCLUDE_HEADER`] marks the block). That file is untracked
//! and local, and git reads it from the common dir for every worktree
//! (gitrepository-layout(5), `info`), so one entry covers all worktrees; in
//! the main tree it names a directory the tracked rules ignore anyway. The
//! alternative, widening the tracked `/apps/*/` to `/apps/*`, would also
//! ignore new build scripts placed directly in `sdk/apps/`.

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The usage lines, appended to `xtask help` after the ticket tools'.
pub const USAGE: &str = "\
Worktree (TOOL44; tools/xtask/src/worktree_setup.rs):
  worktree-setup [--check]       — in a linked git worktree: symlink each
                                   gitignored input the main tree has
                                   (references/*, sdk/apps/<app>/) to the
                                   main tree's copy; never replaces a
                                   wrong link or a real dir, reports it;
                                   --check lists what is missing (exit 1);
                                   refuses in the main tree
";

/// The directories, relative to the tree root, whose gitignored child
/// directories are worktree inputs.
pub const INPUT_DIRS: &[&str] = &["references", "sdk/apps"];

/// The comment line above the entries this command adds to `info/exclude`.
pub const EXCLUDE_HEADER: &str =
    "# cargo xtask worktree-setup (TOOL44): input symlinks of linked worktrees";

/// The state of one input in the worktree.
#[derive(Debug, PartialEq, Eq)]
pub enum State {
    /// Nothing there.
    Missing,
    /// A symlink resolving to the main tree's copy.
    Linked,
    /// A real directory (e.g. a copy): kept.
    RealDir,
    /// A symlink elsewhere, a dangling one or a non-directory: kept, an error.
    Wrong(String),
}

/// One input: its path relative to the tree root and its state.
#[derive(Debug)]
pub struct Input {
    pub rel: String,
    pub state: State,
}

/// What a run did or (with `check`) found.
#[derive(Debug, Default)]
pub struct Report {
    /// Links created (empty with `check`).
    pub created: Vec<String>,
    /// `info/exclude` entries appended (empty with `check`).
    pub excluded: Vec<String>,
    /// Inputs already linked correctly before the run.
    pub present: Vec<String>,
    /// Real directories in the worktree where a link would go.
    pub real_dirs: Vec<String>,
    /// Wrong links and non-directories, with the reason.
    pub wrong: Vec<String>,
    /// With `check`: missing links and links git does not ignore.
    pub missing: Vec<String>,
}

impl Report {
    /// The run left the worktree incomplete or found a wrong entry.
    pub fn failed(&self) -> bool {
        !self.wrong.is_empty() || !self.missing.is_empty()
    }
}

/// A git command in `dir`, the repository found from `dir` alone (no
/// inherited `GIT_DIR` and friends, e.g. from a hook).
fn git(dir: &Path, args: &[&str]) -> Result<std::process::Output, String> {
    let mut c = Command::new("git");
    for v in ["GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE", "GIT_COMMON_DIR"] {
        c.env_remove(v);
    }
    c.args(args).current_dir(dir).output().map_err(|e| format!("worktree-setup: git {}: {e}", args.join(" ")))
}

fn git_ok(dir: &Path, args: &[&str]) -> Result<String, String> {
    let out = git(dir, args)?;
    if !out.status.success() {
        return Err(format!(
            "worktree-setup: git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// The repository facts of a linked worktree rooted at `wt`.
#[derive(Debug)]
pub struct Context {
    /// The worktree root (canonical).
    pub worktree: PathBuf,
    /// The main tree root (canonical).
    pub main: PathBuf,
    /// The common git dir (holds `info/exclude`).
    pub common_dir: PathBuf,
}

/// Refuse unless `wt` is the root of a linked worktree whose main tree is a
/// checkout (not bare). `cmd` passes the xtask's own workspace root, so the
/// repository is this one by construction; a `wt` that is no git checkout,
/// only a subdirectory of one, or the main tree itself is refused.
pub fn context(wt: &Path) -> Result<Context, String> {
    let worktree = fs::canonicalize(wt).map_err(|e| format!("worktree-setup: {}: {e}", wt.display()))?;
    let out = git(&worktree, &["rev-parse", "--path-format=absolute", "--show-toplevel", "--git-dir", "--git-common-dir"])?;
    if !out.status.success() {
        return Err(format!("worktree-setup: {} is not inside a git worktree of this repository", worktree.display()));
    }
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    let lines: Vec<&str> = text.lines().collect();
    let [top, git_dir, common] = lines[..] else {
        return Err(format!("worktree-setup: unexpected git rev-parse output {text:?}"));
    };
    let canon = |p: &str| fs::canonicalize(p).map_err(|e| format!("worktree-setup: {p}: {e}"));
    if canon(top)? != worktree {
        return Err(format!(
            "worktree-setup: {} is not the root of a worktree (its root is {top})",
            worktree.display()
        ));
    }
    let common_dir = canon(common)?;
    if canon(git_dir)? == common_dir {
        return Err(format!(
            "worktree-setup: {} is the main tree; run it inside a linked worktree (git worktree add)",
            worktree.display()
        ));
    }
    let list = git_ok(&worktree, &["worktree", "list", "--porcelain"])?;
    // The first record is the main worktree (git-worktree(1), porcelain).
    let mut first = list.split("\n\n").next().unwrap_or("").lines();
    let main = first
        .next()
        .and_then(|l| l.strip_prefix("worktree "))
        .ok_or_else(|| format!("worktree-setup: unexpected git worktree list output {list:?}"))?;
    if first.any(|l| l == "bare") {
        return Err("worktree-setup: the main repository is bare; there is no main tree to link to".into());
    }
    Ok(Context { worktree, main: canon(main)?, common_dir })
}

/// The inputs: the child directories of [`INPUT_DIRS`] in the main tree that
/// its ignore rules ignore as directories and that hold no tracked file,
/// sorted, with their state in the worktree.
pub fn inputs(ctx: &Context) -> Result<Vec<Input>, String> {
    let mut found = Vec::new();
    for dir in INPUT_DIRS {
        let Ok(rd) = fs::read_dir(ctx.main.join(dir)) else { continue };
        let mut names: Vec<String> = rd
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        for name in names {
            let rel = format!("{dir}/{name}");
            // Asked in the main tree, where the input is a real directory:
            // in the worktree a link may already stand there, and git
            // refuses a pathspec "beyond a symbolic link".
            let ignored = git(&ctx.main, &["check-ignore", "-q", "--no-index", &format!("{rel}/")])?;
            if !ignored.status.success() {
                continue;
            }
            if !git_ok(&ctx.main, &["ls-files", "--", &rel])?.trim().is_empty() {
                continue;
            }
            let state = state(&ctx.worktree.join(&rel), &ctx.main.join(&rel));
            found.push(Input { rel, state });
        }
    }
    Ok(found)
}

/// The state of `at` in the worktree against the main tree's copy `want`.
fn state(at: &Path, want: &Path) -> State {
    let Ok(meta) = fs::symlink_metadata(at) else { return State::Missing };
    if meta.file_type().is_symlink() {
        let target = fs::read_link(at).map(|t| t.display().to_string()).unwrap_or_default();
        return match (fs::canonicalize(at), fs::canonicalize(want)) {
            (Ok(a), Ok(w)) if a == w => State::Linked,
            (Ok(_), _) => State::Wrong(format!("a symlink to {target}, not to {}", want.display())),
            (Err(_), _) => State::Wrong(format!("a dangling symlink to {target}")),
        };
    }
    if meta.is_dir() {
        State::RealDir
    } else {
        State::Wrong("a file, not a directory".into())
    }
}

/// git ignores the path `rel` as it is now in the worktree (a link: as a
/// file).
fn ignored_now(wt: &Path, rel: &str) -> Result<bool, String> {
    Ok(git(wt, &["check-ignore", "-q", rel])?.status.success())
}

/// Append `/rel` lines to the common dir's `info/exclude`, the header first
/// if the file does not carry it yet.
fn exclude(common_dir: &Path, rels: &[String]) -> Result<(), String> {
    let info = common_dir.join("info");
    fs::create_dir_all(&info).map_err(|e| format!("worktree-setup: {}: {e}", info.display()))?;
    let path = info.join("exclude");
    let old = fs::read_to_string(&path).unwrap_or_default();
    let mut add = String::new();
    if !old.is_empty() && !old.ends_with('\n') {
        add.push('\n');
    }
    if !old.lines().any(|l| l == EXCLUDE_HEADER) {
        add.push_str(EXCLUDE_HEADER);
        add.push('\n');
    }
    for r in rels {
        add.push_str(&format!("/{r}\n"));
    }
    fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .and_then(|mut f| f.write_all(add.as_bytes()))
        .map_err(|e| format!("worktree-setup: {}: {e}", path.display()))
}

/// Link the missing inputs of the worktree `wt` (with `check`: only look).
pub fn run(wt: &Path, check: bool) -> Result<Report, String> {
    let ctx = context(wt)?;
    let mut rep = Report::default();
    let mut linked = Vec::new();
    for input in inputs(&ctx)? {
        match input.state {
            State::Missing if check => rep.missing.push(format!("{}: no link", input.rel)),
            State::Missing => {
                let at = ctx.worktree.join(&input.rel);
                if let Some(parent) = at.parent() {
                    fs::create_dir_all(parent).map_err(|e| format!("worktree-setup: {}: {e}", parent.display()))?;
                }
                std::os::unix::fs::symlink(ctx.main.join(&input.rel), &at)
                    .map_err(|e| format!("worktree-setup: link {}: {e}", at.display()))?;
                rep.created.push(input.rel.clone());
                linked.push(input.rel);
            }
            State::Linked => {
                rep.present.push(input.rel.clone());
                linked.push(input.rel);
            }
            State::RealDir => rep.real_dirs.push(input.rel),
            State::Wrong(why) => rep.wrong.push(format!("{}: {why}", input.rel)),
        }
    }
    let mut unignored = Vec::new();
    for rel in linked {
        if !ignored_now(&ctx.worktree, &rel)? {
            unignored.push(rel);
        }
    }
    if check {
        rep.missing.extend(unignored.into_iter().map(|r| format!("{r}: link not ignored by git (no info/exclude entry)")));
    } else if !unignored.is_empty() {
        exclude(&ctx.common_dir, &unignored)?;
        rep.excluded = unignored;
    }
    Ok(rep)
}

/// `cargo xtask worktree-setup [--check]`.
pub fn cmd(root: &Path, args: &[String]) -> Result<(), String> {
    let mut check = false;
    for a in args {
        match a.as_str() {
            "--check" => check = true,
            other => return Err(format!("worktree-setup: unknown argument {other:?}")),
        }
    }
    let rep = run(root, check)?;
    for r in &rep.created {
        println!("worktree-setup: linked {r}");
    }
    for r in &rep.excluded {
        println!("worktree-setup: info/exclude += /{r}");
    }
    for r in &rep.real_dirs {
        println!("worktree-setup: {r} is a real directory, kept (not a link to the main tree)");
    }
    for r in &rep.wrong {
        println!("worktree-setup: WRONG {r} (left alone; fix by hand)");
    }
    for r in &rep.missing {
        println!("worktree-setup: MISSING {r}");
    }
    if rep.created.is_empty() && rep.excluded.is_empty() && !rep.failed() {
        println!("worktree-setup: nothing changed ({} inputs linked)", rep.present.len());
    }
    if rep.failed() {
        let what = if check { "is incomplete" } else { "has wrong entries" };
        return Err(format!("worktree-setup: the worktree {what}"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc_check::tests::TempRepo;

    /// The main tree of a scratch repository shaped like this one (both rule
    /// styles, a negated tracked app, an app dir with tracked content) and a
    /// linked worktree at `.claude/worktrees/w`.
    fn setup(name: &str) -> (TempRepo, PathBuf) {
        let r = TempRepo::new(name);
        r.write(".gitignore", "/references/*\n!/references/README.md\n/.claude/worktrees/\n");
        r.write("references/README.md", "readme\n");
        r.write("sdk/.gitignore", "/apps/*/\n!/apps/own/\n**/*.o\n");
        r.write("sdk/apps/own/a.c", "tracked\n");
        r.write("sdk/apps/build-x.sh", "tracked\n");
        r.write("sdk/apps/mixed/tracked.txt", "tracked\n");
        r.git(&["add", "-A"]);
        r.git(&["add", "-f", "sdk/apps/mixed/tracked.txt"]);
        r.git(&["commit", "-qm", "init"]);
        r.write("references/emu/x", "ignored input\n");
        r.write("sdk/apps/app1/y", "ignored input\n");
        r.write("sdk/apps/mixed/build/z", "ignored, but beside tracked\n");
        r.write("sdk/apps/own/b.o", "untracked in a tracked app\n");
        r.git(&["worktree", "add", "-q", ".claude/worktrees/w"]);
        let wt = r.0.join(".claude/worktrees/w");
        (r, wt)
    }

    fn status(wt: &Path) -> String {
        git_ok(wt, &["status", "--porcelain"]).unwrap()
    }

    #[test]
    fn links_inputs_keeps_status_clean_and_is_idempotent() {
        let (r, wt) = setup("wt-setup-links");
        let before = run(&wt, true).unwrap();
        assert_eq!(before.missing.len(), 2, "{before:?}");
        assert!(before.created.is_empty() && !wt.join("references/emu").exists());

        let rep = run(&wt, false).unwrap();
        assert_eq!(rep.created, vec!["references/emu", "sdk/apps/app1"]);
        // The dir-only rule misses the link; `/references/*` does not.
        assert_eq!(rep.excluded, vec!["sdk/apps/app1"]);
        assert!(!rep.failed());
        assert_eq!(fs::read_to_string(wt.join("sdk/apps/app1/y")).unwrap(), "ignored input\n");
        assert!(fs::symlink_metadata(wt.join("references/emu")).unwrap().file_type().is_symlink());
        assert!(!wt.join("sdk/apps/mixed/build").exists());
        assert_eq!(status(&wt), "");
        assert_eq!(status(&r.0), "");
        let ex = fs::read_to_string(r.0.join(".git/info/exclude")).unwrap();
        assert!(ex.contains(EXCLUDE_HEADER) && ex.contains("\n/sdk/apps/app1\n"), "{ex}");

        let again = run(&wt, false).unwrap();
        assert!(again.created.is_empty() && again.excluded.is_empty() && !again.failed(), "{again:?}");
        assert_eq!(again.present.len(), 2);
        assert!(!run(&wt, true).unwrap().failed());
        assert_eq!(fs::read_to_string(r.0.join(".git/info/exclude")).unwrap(), ex);
    }

    #[test]
    fn wrong_link_and_real_dir_are_reported_not_replaced() {
        let (r, wt) = setup("wt-setup-wrong");
        r.write("references/other/x", "main\n");
        r.write("references/copy/x", "main\n");
        std::os::unix::fs::symlink(r.0.join("sdk"), wt.join("references/other")).unwrap();
        std::os::unix::fs::symlink(r.0.join("nowhere"), wt.join("references/emu")).unwrap();
        fs::create_dir_all(wt.join("references/copy")).unwrap();
        let rep = run(&wt, false).unwrap();
        assert!(rep.failed());
        assert_eq!(rep.wrong.len(), 2, "{rep:?}");
        assert!(rep.wrong[0].starts_with("references/emu: a dangling symlink"), "{:?}", rep.wrong);
        assert!(rep.wrong[1].starts_with("references/other: a symlink to"), "{:?}", rep.wrong);
        assert_eq!(rep.real_dirs, vec!["references/copy"]);
        assert_eq!(rep.created, vec!["sdk/apps/app1"]);
        assert_eq!(fs::read_link(wt.join("references/other")).unwrap(), r.0.join("sdk"));
        assert!(fs::symlink_metadata(wt.join("references/copy")).unwrap().is_dir());
        assert_eq!(status(&wt), "");
    }

    #[test]
    fn refuses_in_the_main_tree_and_outside_a_worktree_root() {
        let (r, wt) = setup("wt-setup-refuse");
        let e = run(&r.0, false).unwrap_err();
        assert!(e.contains("is the main tree"), "{e}");
        let e = run(&wt.join("sdk"), false).unwrap_err();
        assert!(e.contains("not the root of a worktree"), "{e}");
        assert!(!wt.join("references/emu").exists());
        assert!(fs::read_to_string(r.0.join(".git/info/exclude")).map_or(true, |t| !t.contains(EXCLUDE_HEADER)));
    }

    #[test]
    fn refuses_outside_a_git_checkout() {
        // A directory whose `.git` points nowhere: git finds no repository
        // there and does not search upward into this checkout.
        let r = TempRepo::new("wt-setup-nogit");
        let dir = r.0.join("plain");
        r.write("plain/.git", "gitdir: /nonexistent-tool44\n");
        let e = run(&dir, false).unwrap_err();
        assert!(e.contains("is not inside a git worktree"), "{e}");
    }
}
