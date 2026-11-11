//! `xtask install-hooks` — point git at the checked-in hooks in
//! [`HOOKS_DIR`] (TOOL29): `git config core.hooksPath tools/git-hooks`.
//!
//! # Why `core.hooksPath` and not a copy into `.git/hooks`
//!
//! A relative `core.hooksPath` is taken relative to the directory the hooks
//! run in, the root of the working tree (githooks(5)). Every worktree of the
//! repository shares the setting and runs the hook script its own checkout
//! carries, so a change to the hook takes effect with the commit that makes
//! it; a copy would go stale silently. The cost: git then ignores
//! `.git/hooks/` — so the command refuses when `core.hooksPath` already
//! names another directory, and names any active hook in the hooks
//! directory that the switch would shadow instead of switching.
//!
//! Undo: `git config --unset core.hooksPath`. Bypass for one commit:
//! `git commit --no-verify` (git's own switch; the hook has no other).

use std::fs;
use std::path::Path;
use std::process::Command;

/// The checked-in git hooks, relative to the workspace root; also the value
/// written to `core.hooksPath`.
pub const HOOKS_DIR: &str = "tools/git-hooks";
/// The hooks [`HOOKS_DIR`] holds; each must exist and be executable.
pub const HOOKS: &[&str] = &["pre-commit", "commit-msg"];

/// What to do, given the current `core.hooksPath` and the active hooks (not
/// `*.sample`) in the hooks directory git uses now.
#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    /// Set `core.hooksPath` to [`HOOKS_DIR`].
    Set,
    /// Already set to [`HOOKS_DIR`]: nothing to do.
    AlreadySet,
    /// Refuse, with the reason.
    Refuse(String),
}

/// The decision of `install-hooks`: `current` is `core.hooksPath` (`None`
/// if unset), `active` the active hooks in the directory git uses now.
pub fn decide(current: Option<&str>, active: &[String]) -> Action {
    match current.map(|c| c.trim_end_matches('/')) {
        Some(HOOKS_DIR) => Action::AlreadySet,
        Some(other) => Action::Refuse(format!(
            "core.hooksPath is already {other:?}; merge its hooks with {HOOKS_DIR} by hand or \
             `git config --unset core.hooksPath` first"
        )),
        None if !active.is_empty() => Action::Refuse(format!(
            "active hooks in the current hooks directory would be shadowed: {}; move them into {HOOKS_DIR} \
             or remove them first",
            active.join(", ")
        )),
        None => Action::Set,
    }
}

fn git(root: &Path, args: &[&str]) -> Result<std::process::Output, String> {
    Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|e| format!("install-hooks: git {}: {e}", args.join(" ")))
}

/// `cargo xtask install-hooks`.
pub fn cmd(root: &Path, args: &[String]) -> Result<(), String> {
    if let Some(a) = args.first() {
        return Err(format!("install-hooks: unknown argument {a:?}"));
    }
    for h in HOOKS {
        let p = root.join(HOOKS_DIR).join(h);
        let mode = fs::metadata(&p).map_err(|e| format!("install-hooks: {}: {e}", p.display()))?;
        if std::os::unix::fs::PermissionsExt::mode(&mode.permissions()) & 0o111 == 0 {
            return Err(format!("install-hooks: {} is not executable", p.display()));
        }
    }
    let out = git(root, &["config", "--get", "core.hooksPath"])?;
    let current = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let current = (out.status.success() && !current.is_empty()).then_some(current);
    let mut active = Vec::new();
    if current.is_none() {
        let out = git(root, &["rev-parse", "--git-path", "hooks"])?;
        if !out.status.success() {
            return Err("install-hooks: not inside a git repository".into());
        }
        let dir = root.join(String::from_utf8_lossy(&out.stdout).trim());
        if let Ok(rd) = fs::read_dir(&dir) {
            for e in rd.filter_map(|e| e.ok()) {
                let name = e.file_name().to_string_lossy().into_owned();
                if !name.ends_with(".sample") && e.path().is_file() {
                    active.push(name);
                }
            }
        }
        active.sort();
    }
    match decide(current.as_deref(), &active) {
        Action::AlreadySet => println!("install-hooks: core.hooksPath is already {HOOKS_DIR}"),
        Action::Refuse(why) => return Err(format!("install-hooks: {why}")),
        Action::Set => {
            let out = git(root, &["config", "core.hooksPath", HOOKS_DIR])?;
            if !out.status.success() {
                return Err(format!(
                    "install-hooks: git config core.hooksPath failed: {}",
                    String::from_utf8_lossy(&out.stderr).trim()
                ));
            }
            println!("install-hooks: core.hooksPath = {HOOKS_DIR} ({})", HOOKS.join(", "));
        }
    }
    println!(
        "install-hooks: the ticket checks (TOOL41) run only with `git config {} true`; now: {}",
        crate::tickets::SWITCH,
        if ticket_checks_on(root) { "on" } else { "off" }
    );
    Ok(())
}

/// The switch of the ticket checks in `pre-commit` and `commit-msg`
/// (TOOL41, [`crate::tickets::SWITCH`]) as the hooks read it: `git config
/// --bool --get` prints `true`. `install-hooks` reports it, never sets it.
pub fn ticket_checks_on(root: &Path) -> bool {
    git(root, &["config", "--bool", "--get", crate::tickets::SWITCH])
        .is_ok_and(|o| o.status.success() && String::from_utf8_lossy(&o.stdout).trim() == "true")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decision() {
        assert_eq!(decide(None, &[]), Action::Set);
        assert_eq!(decide(Some("tools/git-hooks"), &[]), Action::AlreadySet);
        assert_eq!(decide(Some("tools/git-hooks/"), &[]), Action::AlreadySet);
        // Another hooks directory is never overwritten.
        assert!(matches!(decide(Some(".githooks"), &[]), Action::Refuse(m) if m.contains(".githooks")));
        // An active hook in .git/hooks would go dark: refuse and name it.
        let active = vec!["pre-push".to_string()];
        assert!(matches!(decide(None, &active), Action::Refuse(m) if m.contains("pre-push")));
    }

    #[test]
    fn the_checked_in_hooks_are_executable() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
        for h in HOOKS {
            let p = root.join(HOOKS_DIR).join(h);
            let mode = std::os::unix::fs::PermissionsExt::mode(&fs::metadata(&p).unwrap().permissions());
            assert_ne!(mode & 0o111, 0, "{}", p.display());
        }
    }

    /// TOOL42: the pipe tests of the agent guard `tools/hooks/no-verify-guard.sh`
    /// (a PreToolUse hook, not a git hook) run with every `cargo xtask test`.
    /// The script makes its scratch clone under `NVG_TEST_TMP` (here a
    /// directory under `target/`, like doc-check's `TempRepo`) and removes it.
    #[test]
    fn no_verify_guard_pipe_tests_pass() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
        let tmp = root.join("target").join(format!("xtask-test-nvg-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let mut c = Command::new("bash");
        for v in ["GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE", "GIT_COMMON_DIR"] {
            c.env_remove(v);
        }
        let out = c
            .arg("tools/hooks/no-verify-guard-test.sh")
            .env("NVG_TEST_TMP", &tmp)
            .current_dir(root)
            .output()
            .unwrap();
        let _ = fs::remove_dir_all(&tmp);
        let stdout = String::from_utf8_lossy(&out.stdout);
        let fails: Vec<&str> = stdout.lines().filter(|l| l.starts_with("FAIL")).collect();
        assert!(
            out.status.success() && fails.is_empty(),
            "no-verify-guard-test.sh: {:?}\n{}\n{}",
            out.status,
            fails.join("\n"),
            String::from_utf8_lossy(&out.stderr)
        );
    }

    /// Runs `tools/hooks/<script>` from the repo root with `AS_TEST_TMP` set
    /// to a fresh directory under `target/` and asserts no `FAIL` line.
    fn hook_pipe_test(script: &str) {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
        let stem = script.trim_end_matches(".sh");
        let tmp = root.join("target").join(format!("xtask-test-{stem}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let mut c = Command::new("bash");
        for v in ["GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE", "GIT_COMMON_DIR"] {
            c.env_remove(v);
        }
        let out = c
            .arg(format!("tools/hooks/{script}"))
            .env("AS_TEST_TMP", &tmp)
            .current_dir(root)
            .output()
            .unwrap();
        let _ = fs::remove_dir_all(&tmp);
        let stdout = String::from_utf8_lossy(&out.stdout);
        let fails: Vec<&str> = stdout.lines().filter(|l| l.starts_with("FAIL")).collect();
        assert!(
            out.status.success() && fails.is_empty(),
            "{script}: {:?}\n{}\n{}",
            out.status,
            fails.join("\n"),
            String::from_utf8_lossy(&out.stderr)
        );
    }

    /// TOOL46: the scratch guard (PreToolUse) keeps its old cases and blocks
    /// clones and cargo target dirs in the session scratch root.
    #[test]
    fn scratch_guard_pipe_tests_pass() {
        hook_pipe_test("scratch-guard-test.sh");
    }

    /// TOOL46: the SubagentStart/SubagentStop/SessionStart hook removes only
    /// the finished worker's `target-agents/agent-<id>/`, in a fixture repo.
    #[test]
    fn agent_scratch_pipe_tests_pass() {
        hook_pipe_test("agent-scratch-test.sh");
    }
}
