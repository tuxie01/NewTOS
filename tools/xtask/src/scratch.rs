//! TOOL46 — worker scratch clean-up: `scratch-clean` and `land`, and the
//! clean-up `accept` runs after its commit.
//!
//! A worker's test clones and cargo target dirs live in
//! `<main repo>/target-agents/agent-<id>/` (`tools/hooks/agent-scratch.sh`).
//! They are deleted when the work they belong to is finished: when a ticket is
//! accepted (`accept`) and when its branch is landed (`land`), for every agent
//! the ticket's receipts record (`Agent:` header lines of
//! `TASK/<ID>-STATUS-*.MD`); by hand with `scratch-clean`; and 72 h after the
//! last stop by the SessionStart hook (owner decision 2026-11-11).
//!
//! The deletion and all its safety checks (id pattern, no symlinks, physical
//! path `<top>/target-agents/agent-<id>`) exist once, in the `clean` mode of
//! the shell script; this module only decides *which* ids and calls it.
//!
//! # Why `land` and not a git hook
//!
//! `git merge --squash` leaves `.git/SQUASH_MSG`, no `MERGE_HEAD` and nothing
//! naming the branch; `git commit` removes `SQUASH_MSG` before `post-commit`
//! runs (measured with git 2.54: `pre-commit`, `prepare-commit-msg` and
//! `commit-msg` see it, `post-commit` does not). A hook-based trigger would
//! need a hand-over file between two hooks and would fire on a stale
//! `SQUASH_MSG` too; `land` does merge, commit and clean-up in one command
//! and leaves the main tree's git hooks as they are.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::tickets::{agent_key, subject, Analysis, View};

/// The usage lines, appended to `xtask help`.
pub const USAGE: &str = "\
Worker scratch (TOOL46; tools/xtask/src/scratch.rs, tools/hooks/agent-scratch.sh):
  scratch-clean <agent-id>... | --ticket <ID> [--except <agent>]
                                 — delete target-agents/agent-<id>/ of the
                                   main repo (agent- prefix optional), or of
                                   every agent the ticket's receipts record
                                   (Agent: header lines); refuses bad ids,
                                   symlinks, paths outside target-agents/
  land <branch> -F <message file>
                                 — git merge --squash <branch> plus git commit
                                   -F (normal hooks), then scratch-clean for
                                   every ticket the commit subject names
";

/// The clean-up script of the workspace this xtask was built from.
fn script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/hooks/agent-scratch.sh")
}

/// Runs the script's `clean` mode for `ids` against the repository of
/// `root`. Returns its output; `Err` (with the output) when it refused one.
pub fn clean(root: &Path, ids: &[String]) -> Result<String, String> {
    if ids.is_empty() {
        return Ok(String::new());
    }
    let mut c = Command::new("bash");
    for v in ["GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE", "GIT_COMMON_DIR"] {
        c.env_remove(v);
    }
    let o = c
        .arg(script())
        .arg("clean")
        .args(ids)
        .env("CLAUDE_PROJECT_DIR", root)
        .current_dir(root)
        .output()
        .map_err(|e| format!("scratch-clean: bash {}: {e}", script().display()))?;
    let text = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    if o.status.success() {
        Ok(text)
    } else {
        Err(text)
    }
}

/// The agents the receipts of ticket `id` record in their header.
pub fn recorded_agents(root: &Path, id: &str) -> Result<BTreeSet<String>, String> {
    let mut view = View::disk(root);
    let a = Analysis::load(&mut view)?;
    Ok(a.receipts_of(id).flat_map(|r| r.header("Agent")).collect())
}

/// Cleans the recorded agents of `id` except `except`; one report line per
/// agent, never an error (the clean-up must not undo a finished commit).
pub fn clean_ticket(root: &Path, id: &str, except: Option<&str>) -> String {
    let agents = match recorded_agents(root, id) {
        Ok(a) => a,
        Err(e) => return format!("scratch-clean: {id}: {e}\n"),
    };
    let skip = except.map(agent_key);
    let mut ids = Vec::new();
    let mut s = String::new();
    for a in agents {
        if skip.as_deref() == Some(agent_key(&a).as_str()) {
            s.push_str(&format!("scratch-clean: {id}: {a} is the running agent; its directory expires 72 h after its last stop\n"));
        } else {
            ids.push(a);
        }
    }
    match clean(root, &ids) {
        Ok(t) => s.push_str(&t),
        Err(t) => s.push_str(&format!("{t}scratch-clean: {id}: refused for at least one agent (above)\n")),
    }
    s
}

/// `cargo xtask scratch-clean <agent-id>... | --ticket <ID> [--except <agent>]`.
pub fn cmd_scratch_clean(root: &Path, args: &[String]) -> Result<(), String> {
    let usage = "usage: cargo xtask scratch-clean <agent-id>... | --ticket <ID> [--except <agent>]";
    if args.is_empty() {
        return Err(usage.into());
    }
    if args[0] == "--ticket" {
        let id = args.get(1).ok_or(usage)?;
        let except = match &args[2..] {
            [] => None,
            [f, a] if f == "--except" => Some(a.as_str()),
            _ => return Err(usage.into()),
        };
        print!("{}", clean_ticket(root, id, except));
        return Ok(());
    }
    if args.iter().any(|a| a.starts_with("--")) {
        return Err(usage.into());
    }
    match clean(root, args) {
        Ok(t) => {
            print!("{t}");
            Ok(())
        }
        Err(t) => Err(format!("{t}scratch-clean: refused (above)")),
    }
}

fn git(root: &Path, args: &[&str]) -> Result<String, String> {
    let o = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|e| format!("git {args:?}: {e}"))?;
    if !o.status.success() {
        return Err(format!("git {args:?}: {}", String::from_utf8_lossy(&o.stderr).trim_end()));
    }
    Ok(String::from_utf8_lossy(&o.stdout).into_owned())
}

/// `cargo xtask land <branch> -F <message file>`.
pub fn cmd_land(root: &Path, args: &[String]) -> Result<(), String> {
    print!("{}", land(root, args)?);
    Ok(())
}

/// Squash-merges `<branch>`, commits with the normal hooks, cleans the
/// scratch of the tickets the subject names. Returns the report.
pub fn land(root: &Path, args: &[String]) -> Result<String, String> {
    let usage = "usage: cargo xtask land <branch> -F <message file>";
    let (branch, file) = match args {
        [b, f, m] if f == "-F" && !b.starts_with('-') => (b, m),
        _ => return Err(usage.into()),
    };
    let msg = fs::read_to_string(file).map_err(|e| format!("land: {file}: {e}"))?;
    let staged = git(root, &["diff", "--cached", "--name-only"])?;
    if !staged.trim().is_empty() {
        return Err(format!("land: changes are staged already; the squash commit holds only the branch:\n{}", staged.trim_end()));
    }
    let current = git(root, &["symbolic-ref", "--short", "-q", "HEAD"]).unwrap_or_default();
    if current.trim() == branch {
        return Err(format!("land: {branch} is checked out here; land it from the branch it goes into"));
    }
    git(root, &["rev-parse", "--verify", "--quiet", &format!("{branch}^{{commit}}")])
        .map_err(|_| format!("land: no branch {branch}"))?;
    git(root, &["merge", "--squash", branch]).map_err(|e| format!("land: {e}"))?;
    let st = Command::new("git")
        .args(["commit", "-q", "-F"])
        .arg(file)
        .current_dir(root)
        .status()
        .map_err(|e| format!("land: git commit: {e}"))?;
    if !st.success() {
        return Err(format!(
            "land: git commit failed (the hooks' output above); the squash of {branch} stays staged: fix, `git commit -F {file}`, then `cargo xtask scratch-clean --ticket <ID>`"
        ));
    }
    let head = git(root, &["rev-parse", "--short=9", "HEAD"])?;
    let mut s = format!("land: {branch} squashed into {}\n", head.trim());
    let mut view = View::disk(root);
    let ids = Analysis::load(&mut view)?.named_in(subject(&msg));
    if ids.is_empty() {
        s.push_str("land: the subject names no ticket; no scratch cleaned\n");
    }
    for id in ids {
        s.push_str(&clean_ticket(root, &id, None));
    }
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc_check::tests::TempRepo;

    fn repo(name: &str) -> TempRepo {
        let r = TempRepo::new(name);
        for (k, v) in [("user.name", "t"), ("user.email", "t@t"), ("commit.gpgsign", "false"), ("core.hooksPath", "/dev/null")] {
            r.git(&["config", k, v]);
        }
        r.write(".gitignore", "/target-agents/\n");
        r.write("docs/MEMORY.md", "# Memory\n\n## ADR-001: one\n");
        r.write("TASK/X1.md", "# X1 — a title\n\nStatus: OPEN (2026-11-10) | Priority: P1\n\nBody.\n");
        r.write("TASK/X2.md", "# X2 — a title\n\nStatus: OPEN (2026-11-10) | Priority: P1\n\nBody.\n");
        r.write("TASK/X2-STATUS-UNFINISHED.MD", "# X2 — receipt\n\nRole: developer\nAgent: agent-dev-x2\n\n## Done\n");
        r.git(&["add", "-A"]);
        r.git(&["commit", "-q", "-m", "base"]);
        r
    }

    fn dirs(r: &TempRepo, names: &[&str]) {
        for n in names {
            r.write(&format!("target-agents/agent-{n}/clone/f"), "x");
        }
    }

    fn has(r: &TempRepo, n: &str) -> bool {
        r.0.join(format!("target-agents/agent-{n}/clone/f")).exists()
    }

    #[test]
    fn scratch_clean_by_id_and_by_ticket() {
        let r = repo("scratch-clean");
        r.write("TASK/X1-STATUS-UNFINISHED.MD", "# X1 — receipt\n\nRole: developer\nAgent: agent-dev-a\n\n## Done\n\nAgent: in-body\n");
        r.write("TASK/X1-STATUS-DONE.MD", "# X1 — receipt DONE\n\nRole: tester\nAgent: Tester-B\n\n## Note\n");
        dirs(&r, &["dev-a", "tester-b", "in-body", "dev-x2", "other"]);
        let t = clean_ticket(&r.0, "X1", Some("agent-tester-b"));
        assert!(!has(&r, "dev-a"), "{t}");
        assert!(has(&r, "tester-b") && t.contains("Tester-B is the running agent"), "{t}");
        assert!(has(&r, "in-body") && has(&r, "dev-x2") && has(&r, "other"), "{t}");
        cmd_scratch_clean(&r.0, &["--ticket".into(), "X1".into()]).unwrap();
        assert!(!has(&r, "tester-b") && has(&r, "other"));
        cmd_scratch_clean(&r.0, &["agent-other".into()]).unwrap();
        assert!(!has(&r, "other") && has(&r, "dev-x2"));
        assert!(cmd_scratch_clean(&r.0, &["../x".into()]).is_err());
        assert!(cmd_scratch_clean(&r.0, &[]).is_err());
    }

    #[test]
    fn land_squashes_commits_and_cleans_the_named_tickets_agents() {
        let r = repo("scratch-land");
        let main = git(&r.0, &["symbolic-ref", "--short", "HEAD"]).unwrap();
        r.git(&["switch", "-q", "-c", "feat"]);
        r.write("TASK/X1-STATUS-UNFINISHED.MD", "# X1 — receipt\n\nRole: developer\nAgent: agent-dev-a\n\n## Done\n");
        r.write("code.txt", "change\n");
        r.git(&["add", "-A"]);
        r.git(&["commit", "-q", "-m", "feat(x): the change (X1)"]);
        r.write("TASK/X1-STATUS-DONE.MD", "# X1 — receipt DONE\n\nRole: tester\nAgent: tester-b\n\n## Note\n");
        r.git(&["add", "-A"]);
        r.git(&["commit", "-q", "-m", "docs(task): accept X1 (X1)"]);
        r.git(&["switch", "-q", main.trim()]);
        dirs(&r, &["dev-a", "tester-b", "dev-x2", "other"]);
        r.write("target-agents/msg", "feat(x): the change (X1)\n\nBody.\n");
        let ms = r.0.join("target-agents/msg").to_string_lossy().to_string();
        let rep = land(&r.0, &["feat".into(), "-F".into(), ms.clone()]).unwrap();
        assert!(rep.contains("squashed into") && rep.contains("removed"), "{rep}");
        let st = git(&r.0, &["log", "-n1", "--format=%s%n%P"]).unwrap();
        assert!(st.starts_with("feat(x): the change (X1)\n"), "{st}");
        assert_eq!(st.lines().nth(1).unwrap().split_whitespace().count(), 1, "a squash, not a merge commit");
        assert!(r.0.join("code.txt").exists());
        assert!(!has(&r, "dev-a") && !has(&r, "tester-b"), "the X1 agents are cleaned");
        assert!(has(&r, "dev-x2") && has(&r, "other"), "other tickets' and unrecorded agents stay");
        // Refusals: staged changes; the branch itself checked out; no branch.
        r.write("y.txt", "y");
        r.git(&["add", "y.txt"]);
        assert!(land(&r.0, &["feat".into(), "-F".into(), ms.clone()]).unwrap_err().contains("staged"));
        r.git(&["reset", "-q"]);
        assert!(land(&r.0, &["nosuch".into(), "-F".into(), ms.clone()]).unwrap_err().contains("no branch"));
        r.git(&["switch", "-q", "feat"]);
        assert!(land(&r.0, &["feat".into(), "-F".into(), ms]).unwrap_err().contains("checked out"));
        assert!(land(&r.0, &["feat".into()]).is_err());
    }
}
