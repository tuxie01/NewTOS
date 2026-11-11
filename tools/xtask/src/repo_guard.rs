//! TOOL52 — a writing xtask command runs only from inside the repository the
//! binary was built for.
//!
//! xtask takes its repository root from its build (`workspace_root()`, the
//! workspace of `CARGO_MANIFEST_DIR`), not from the working directory. On
//! 2026-11-11 the main tree's binary, run from inside a scratch clone, committed
//! `accept` in the main repository (TASK/TOOL51-STATUS-DONE.MD, *Correction*).
//!
//! # The rule: refuse, do not redirect
//!
//! A command that writes or commits checks that `git rev-parse --show-toplevel`
//! of the working directory is the binary's root, and refuses otherwise; it
//! never switches to the working directory's repository. Reasons:
//!
//! - `cargo xtask` builds the xtask of the workspace the working directory is
//!   in, so the normal call always passes; only a binary started by path from
//!   another repository is refused, and that call is the mistake.
//! - The binary carries its own tree's rules (ADR-037 checks, the baseline
//!   format, `tools/hooks/agent-scratch.sh` by build path, the hooks
//!   directory). Acting on another repository would apply one branch's rules to
//!   another's files: the TOOL51 tester wanted the clone's own `accept` and
//!   would silently have tested main's.
//! - A refusal names both paths and changes nothing in either repository.
//!
//! Tests keep calling the commands with an explicit root (`tickets::accept`,
//! `scratch::land`, ...); only the dispatcher in `main` applies [`check_cwd`].
//!
//! # Marking
//!
//! [`COMMANDS`] marks every dispatched command. A name not in it is treated as
//! writing, and a test fails when a dispatcher arm in `main.rs` or
//! `tickets::cmd` is missing from the table.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Whether a command may write to or commit in a repository.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    /// Reads only, in every mode (runs from anywhere, reads the binary's tree).
    ReadOnly,
    /// Writes or commits in at least one mode: guarded.
    Writes,
}

/// Every command the dispatcher knows, with its marking. A command is
/// `ReadOnly` only if none of its modes writes a file or the git state
/// (`status-check --prune-baseline`, `report --readme`,
/// `emutos-consts --update`, `prgflags <file> <value>` all write).
pub const COMMANDS: &[(&str, Access)] = &[
    ("help", Access::ReadOnly),
    ("--help", Access::ReadOnly),
    ("-h", Access::ReadOnly),
    ("doc-check", Access::ReadOnly),
    ("deps", Access::ReadOnly),
    ("build", Access::Writes),
    ("build-programs", Access::Writes),
    ("emutos-consts", Access::Writes),
    ("gen-font", Access::Writes),
    ("gen-fsel-rsc", Access::Writes),
    ("install-hooks", Access::Writes),
    ("prgflags", Access::Writes),
    ("run", Access::Writes),
    ("test", Access::Writes),
    ("tosdump", Access::Writes),
    ("worktree-setup", Access::Writes),
    ("scratch-clean", Access::Writes),
    ("land", Access::Writes),
    ("status-check", Access::Writes),
    ("accept", Access::Writes),
    ("set-status", Access::Writes),
    ("report", Access::Writes),
    ("migrate", Access::Writes),
];

/// The marking of `cmd`; an unmarked name counts as writing.
pub fn access(cmd: &str) -> Access {
    COMMANDS.iter().find(|(n, _)| *n == cmd).map_or(Access::Writes, |(_, a)| *a)
}

/// [`check`] against the process's working directory.
pub fn check_cwd(cmd: &str, bin_root: &Path) -> Result<(), String> {
    if access(cmd) == Access::ReadOnly {
        return Ok(());
    }
    let cwd = std::env::current_dir().map_err(|e| format!("{cmd}: the working directory: {e}"))?;
    check(cmd, bin_root, &cwd)
}

/// `Ok` when `cmd` reads only, or `cwd` lies in the repository whose top
/// level is `bin_root`; otherwise the refusal, naming both paths.
pub fn check(cmd: &str, bin_root: &Path, cwd: &Path) -> Result<(), String> {
    if access(cmd) == Access::ReadOnly {
        return Ok(());
    }
    let bin = canonical(bin_root);
    match toplevel(cwd) {
        Some(top) if canonical(&top) == bin => Ok(()),
        Some(top) => Err(format!(
            "{cmd} refused (TOOL52): the working directory {} is in the repository {}, but this xtask binary \
             was built for {}; it acts only on its own repository. Run `cargo xtask {cmd}` inside {} (it builds \
             that repository's xtask) or change into {}",
            cwd.display(),
            top.display(),
            bin.display(),
            top.display(),
            bin.display()
        )),
        None => Err(format!(
            "{cmd} refused (TOOL52): the working directory {} is not inside a git work tree, and this xtask binary \
             was built for {}; it acts only on its own repository. Change into {}",
            cwd.display(),
            bin.display(),
            bin.display()
        )),
    }
}

/// The top level of the work tree `dir` lies in, as git sees it from there
/// (the caller's `GIT_DIR` and friends do not redirect it).
fn toplevel(dir: &Path) -> Option<PathBuf> {
    let mut c = Command::new("git");
    for v in ["GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE", "GIT_COMMON_DIR"] {
        c.env_remove(v);
    }
    let o = c.args(["rev-parse", "--show-toplevel"]).current_dir(dir).output().ok()?;
    if !o.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
    (!s.is_empty()).then(|| PathBuf::from(s))
}

fn canonical(p: &Path) -> PathBuf {
    std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// The workspace this test binary was built from.
    fn root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()
    }

    /// A scratch git repository with one commit under the workspace's
    /// `target/`, removed on drop.
    struct Repo(PathBuf);

    impl Repo {
        fn new(name: &str) -> Repo {
            let dir = root().join("target").join(format!("xtask-test-repo-guard-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(dir.join("sub")).unwrap();
            let r = Repo(dir.canonicalize().unwrap());
            fs::write(r.0.join("sub/f.txt"), "x\n").unwrap();
            r.git(&["init", "-q"]);
            r.git(&["add", "-A"]);
            r.git(&["commit", "-q", "-m", "init"]);
            r
        }

        fn git_out(&self, args: &[&str]) -> String {
            let mut c = Command::new("git");
            for v in ["GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE", "GIT_COMMON_DIR"] {
                c.env_remove(v);
            }
            let o = c
                .args(["-c", "user.name=t", "-c", "user.email=t@t", "-c", "commit.gpgsign=false"])
                .args(["-c", "core.hooksPath=/dev/null"])
                .args(args)
                .current_dir(&self.0)
                .output()
                .unwrap();
            assert!(o.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&o.stderr));
            String::from_utf8_lossy(&o.stdout).trim().to_string()
        }

        fn git(&self, args: &[&str]) {
            self.git_out(args);
        }

        fn head(&self) -> String {
            self.git_out(&["rev-parse", "HEAD"])
        }
    }

    impl Drop for Repo {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// The string arms of the `match` that opens with `opener` in `src`, up
    /// to its closing line `closer`.
    fn arms(src: &str, opener: &str, closer: &str) -> Vec<String> {
        let start = src.find(opener).unwrap_or_else(|| panic!("{opener:?} not found"));
        let body = &src[start..];
        let end = body.find(closer).unwrap_or_else(|| panic!("{closer:?} after {opener:?} not found"));
        let mut names = Vec::new();
        for line in body[..end].lines() {
            let l = line.trim_start();
            if !l.starts_with('"') || !l.contains("=>") {
                continue;
            }
            let pats = &l[..l.find("=>").unwrap()];
            for p in pats.split('|') {
                let p = p.trim();
                if let Some(n) = p.strip_prefix('"').and_then(|p| p.strip_suffix('"')) {
                    names.push(n.to_string());
                }
            }
        }
        names
    }

    /// The commands the two dispatchers match: `main()` and `tickets::cmd`.
    fn dispatched() -> Vec<String> {
        let main = include_str!("main.rs");
        let tickets = include_str!("tickets.rs");
        let mut v = arms(main, "let result = repo_guard::check_cwd(", "    });");
        v.extend(arms(tickets, "pub fn cmd(name: &str, root: &Path, args: &[String])", "\n}\n"));
        v
    }

    #[test]
    fn every_dispatched_command_is_marked() {
        let names = dispatched();
        // The dispatcher has at least the commands TOOL52 names.
        for want in ["accept", "land", "set-status", "scratch-clean", "worktree-setup", "install-hooks"] {
            assert!(names.iter().any(|n| n == want), "{want} not found in the dispatchers: {names:?}");
        }
        for n in &names {
            assert!(COMMANDS.iter().any(|(c, _)| c == n), "dispatched command {n:?} is not marked in repo_guard::COMMANDS");
        }
        for (c, _) in COMMANDS {
            assert!(names.iter().any(|n| n == c), "COMMANDS marks {c:?}, which no dispatcher matches");
        }
    }

    #[test]
    fn the_writing_commands_are_guarded() {
        for c in ["accept", "land", "set-status", "scratch-clean", "worktree-setup", "install-hooks", "report", "status-check", "migrate"] {
            assert_eq!(access(c), Access::Writes, "{c}");
        }
        assert_eq!(access("some-future-command"), Access::Writes);
        for c in ["doc-check", "deps", "help"] {
            assert_eq!(access(c), Access::ReadOnly, "{c}");
        }
    }

    #[test]
    fn main_applies_the_guard_before_the_dispatch() {
        let main = include_str!("main.rs");
        let f = &main[main.find("fn main() -> ExitCode {").unwrap()..];
        let guard = f.find("repo_guard::check_cwd(&cmd, &workspace_root())").expect("main() calls repo_guard::check_cwd");
        let first_arm = f.find("\"build\" =>").unwrap();
        assert!(guard < first_arm, "the guard must run before the dispatch");
    }

    #[test]
    fn refuses_from_another_repository_and_names_both_paths() {
        let main = Repo::new("main");
        let clone = Repo::new("clone");
        let (h_main, h_clone) = (main.head(), clone.head());
        for cmd in ["accept", "set-status", "land", "scratch-clean", "worktree-setup", "install-hooks"] {
            let e = check(cmd, &main.0, &clone.0.join("sub")).unwrap_err();
            assert!(e.contains(&main.0.display().to_string()), "{e}");
            assert!(e.contains(&clone.0.display().to_string()), "{e}");
            assert!(e.contains("refused"), "{e}");
        }
        assert_eq!((main.head(), clone.head()), (h_main, h_clone));
    }

    #[test]
    fn passes_inside_its_own_repository() {
        let main = Repo::new("own");
        check("accept", &main.0, &main.0).unwrap();
        check("accept", &main.0, &main.0.join("sub")).unwrap();
    }

    #[test]
    fn a_nested_repository_is_another_repository() {
        // The 2026-11-11 clone lay inside the main tree (target-agents/).
        let main = Repo::new("outer");
        let inner = main.0.join("target-agents/agent-x/clone");
        fs::create_dir_all(&inner).unwrap();
        let mut c = Command::new("git");
        for v in ["GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE", "GIT_COMMON_DIR"] {
            c.env_remove(v);
        }
        assert!(c.args(["init", "-q"]).current_dir(&inner).status().unwrap().success());
        let e = check("accept", &main.0, &inner).unwrap_err();
        assert!(e.contains(&inner.display().to_string()), "{e}");
    }

    #[test]
    fn outside_any_repository_is_refused_read_only_passes() {
        let main = Repo::new("outside");
        // `/` is not inside a work tree on any host this runs on.
        let e = check("land", &main.0, Path::new("/")).unwrap_err();
        assert!(e.contains("not inside a git work tree") && e.contains(&main.0.display().to_string()), "{e}");
        check("doc-check", &main.0, Path::new("/")).unwrap();
        check("deps", &main.0, Path::new("/")).unwrap();
    }

    /// The 2026-11-11 case in process: the guard with this binary's real root
    /// (`workspace_root()`) and a scratch repository as the working directory
    /// refuses `accept`; this workspace's HEAD does not move.
    #[test]
    fn the_tool51_case_is_refused() {
        let ws = root();
        let ws_head = Command::new("git").args(["rev-parse", "HEAD"]).current_dir(&ws).output().unwrap().stdout;
        let clone = Repo::new("tool51");
        let h_clone = clone.head();
        let e = check("accept", &crate::workspace_root(), &clone.0).unwrap_err();
        assert!(e.contains(&ws.display().to_string()) && e.contains(&clone.0.display().to_string()), "{e}");
        let after = Command::new("git").args(["rev-parse", "HEAD"]).current_dir(&ws).output().unwrap().stdout;
        assert_eq!(ws_head, after);
        assert_eq!(clone.head(), h_clone);
    }
}
