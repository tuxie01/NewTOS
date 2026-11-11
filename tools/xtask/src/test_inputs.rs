//! TOOL47 — host tests whose input is a gitignored tree (`references/…`,
//! `sdk/apps/<app>/`) and the one switch that lets them skip.
//!
//! Without the switch a missing input is a **failure**, as before: a local
//! tree is expected to carry the inputs (`cargo xtask worktree-setup` links
//! them into a worktree). With `NEWTOS_CI=1` (exactly that value; anything
//! else counts as unset), meant for a machine that has no copy of them (the
//! TOOL43 CI runner, `docs/analysis/ci-backstop-2026-11-11.md` §1), a test
//! whose input is absent prints one line
//! `SKIPPED <test>: <input> absent (NEWTOS_CI=1)` and returns. A present input
//! always runs the test, switch or not.
//!
//! The line goes to the process's stderr through [`std::io::stderr`], which
//! the test harness does not capture (it captures the `print!` family only),
//! and is appended to the file named by [`SKIP_LOG_VAR`] when that is set:
//! `cargo xtask test` sets it and summarises the skips at the end
//! ([`summary`]).

#[cfg(test)]
use std::path::Path;

/// The switch: `NEWTOS_CI=1` lets a test skip when its input is absent.
pub const CI_VAR: &str = "NEWTOS_CI";
/// The file a skipping test appends its line to (set by `cargo xtask test`).
pub const SKIP_LOG_VAR: &str = "NEWTOS_SKIP_LOG";

/// What a test does about one input.
#[cfg(test)]
#[derive(Debug, PartialEq, Eq)]
pub enum Decision {
    /// The input is present: the test runs.
    Run,
    /// Absent under the switch: the test reports the skip and returns.
    Skip,
    /// Absent without the switch: the test fails.
    Fail,
}

/// The decision for an input that is (or is not) `present`, given the
/// value of [`CI_VAR`] (`None` when unset).
#[cfg(test)]
pub fn decide(ci: Option<&str>, present: bool) -> Decision {
    match (present, ci == Some("1")) {
        (true, _) => Decision::Run,
        (false, true) => Decision::Skip,
        (false, false) => Decision::Fail,
    }
}

/// The one line a skipped test prints.
#[cfg(test)]
pub fn skip_line(test: &str, input: &str) -> String {
    format!("SKIPPED {test}: {input} absent ({CI_VAR}=1)")
}

/// The core every guarded test goes through: `true` when the test should
/// run, `false` when it is skipped (the line printed and logged); panics
/// when `input` is absent and the switch is unset. `input` names what is
/// missing (normally a workspace-relative path).
#[cfg(test)]
pub fn require(test: &str, input: &str, present: bool) -> bool {
    let ci = std::env::var(CI_VAR).ok();
    let log = std::env::var_os(SKIP_LOG_VAR);
    let d = decide(ci.as_deref(), present);
    apply(d, test, input, log.as_deref().map(Path::new), &mut std::io::stderr())
}

/// The workspace-relative paths `rels` a test reads: `true` when all exist;
/// otherwise as [`require`] for the first absent one.
#[cfg(test)]
pub fn require_paths(test: &str, rels: &[&str]) -> bool {
    let root = crate::workspace_root();
    match rels.iter().find(|r| !root.join(r).exists()) {
        None => true,
        Some(absent) => require(test, absent, false),
    }
}

/// Carries out `d` for `test`/`input`: a skip writes its line to `out` (the
/// uncaptured stderr in a real test) and appends it to `log` if given.
#[cfg(test)]
fn apply(d: Decision, test: &str, input: &str, log: Option<&Path>, out: &mut dyn std::io::Write) -> bool {
    use std::io::Write;
    match d {
        Decision::Run => true,
        Decision::Skip => {
            let line = skip_line(test, input);
            let _ = writeln!(out, "{line}");
            if let Some(p) = log {
                // One write per line, O_APPEND: lines of parallel tests stay whole.
                // A skip the summary would miss is an error, not a silent pass.
                std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(p)
                    .and_then(|mut f| f.write_all(format!("{line}\n").as_bytes()))
                    .unwrap_or_else(|e| panic!("{test}: cannot log the skip to {}: {e}", p.display()));
            }
            false
        }
        Decision::Fail => panic!(
            "{test}: test input {input} absent (gitignored; `cargo xtask worktree-setup` links it \
             into a worktree; on a machine without it set {CI_VAR}=1 to skip)"
        ),
    }
}

/// The end-of-run summary of `cargo xtask test` from the skip log's text:
/// `None` when nothing was skipped.
pub fn summary(log_text: &str) -> Option<String> {
    let mut lines: Vec<&str> = log_text.lines().filter(|l| l.starts_with("SKIPPED ")).collect();
    if lines.is_empty() {
        return None;
    }
    lines.sort_unstable();
    let mut s = format!(
        "xtask test: {} test(s) SKIPPED under {CI_VAR}=1 (input absent):",
        lines.len()
    );
    for l in lines {
        s.push_str("\n  ");
        s.push_str(l);
    }
    Some(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn scratch(name: &str) -> PathBuf {
        let dir = crate::workspace_root().join("target").join("xtask-test-inputs").join(name);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// The child below, an absent workspace-relative path, its libtest name.
    const CHILD: &str = "test_inputs::tests::child_require_on_an_absent_input";
    const ABSENT: &str = "target/xtask-test-inputs/no-such-input";

    /// Runs [`CHILD`] in a fresh process of this test binary with `ci` as
    /// `NEWTOS_CI` (`None`: removed) and a scratch skip log: `require` reads
    /// the process environment, which a test must not change under the
    /// other tests running beside it. Returns (success, stderr, log text).
    fn run_child(tag: &str, ci: Option<&str>) -> (bool, String, String) {
        let log = scratch(tag).join("skips.log");
        let _ = std::fs::remove_file(&log);
        let mut c = std::process::Command::new(std::env::current_exe().unwrap());
        c.args([CHILD, "--exact", "--ignored", "--nocapture", "--test-threads=1"]);
        c.env(SKIP_LOG_VAR, &log);
        match ci {
            Some(v) => c.env(CI_VAR, v),
            None => c.env_remove(CI_VAR),
        };
        let o = c.output().expect("run the test binary");
        let stderr = String::from_utf8_lossy(&o.stderr).into_owned();
        (o.status.success(), stderr, std::fs::read_to_string(&log).unwrap_or_default())
    }

    /// Not a test of its own: the body [`run_child`] runs in a subprocess.
    #[test]
    #[ignore = "child process of the require_* tests below, run by them with a set environment"]
    fn child_require_on_an_absent_input() {
        assert!(!crate::workspace_root().join(ABSENT).exists());
        assert!(!require_paths("child::paths", &["tools/xtask/Cargo.toml", ABSENT]));
        assert!(!require("child::direct", "some input", false));
    }

    #[test]
    fn require_on_an_absent_input_fails_without_the_switch() {
        for (tag, ci) in [("unset", None), ("zero", Some("0"))] {
            let (ok, stderr, log) = run_child(tag, ci);
            assert!(!ok, "{tag}: the child must fail\n{stderr}");
            assert!(stderr.contains(&format!("child::paths: test input {ABSENT} absent")), "{tag}: {stderr}");
            assert!(!stderr.contains("SKIPPED") && log.is_empty(), "{tag}: nothing skipped\n{stderr}");
        }
    }

    #[test]
    fn require_on_an_absent_input_skips_under_the_switch() {
        let (ok, stderr, log) = run_child("set", Some("1"));
        assert!(ok, "the child must pass\n{stderr}");
        let want = format!(
            "SKIPPED child::paths: {ABSENT} absent (NEWTOS_CI=1)\n\
             SKIPPED child::direct: some input absent (NEWTOS_CI=1)\n"
        );
        assert_eq!(log, want, "both skips logged, in order");
        for l in want.lines() {
            assert!(stderr.lines().any(|s| s == l), "{l:?} printed\n{stderr}");
        }
    }

    #[test]
    fn only_the_exact_value_1_is_the_switch() {
        assert_eq!(decide(Some("1"), false), Decision::Skip);
        for v in [None, Some(""), Some("0"), Some("true"), Some("yes"), Some(" 1"), Some("1 ")] {
            assert_eq!(decide(v, false), Decision::Fail, "{v:?}");
        }
    }

    #[test]
    fn a_present_input_runs_with_and_without_the_switch() {
        assert_eq!(decide(None, true), Decision::Run);
        assert_eq!(decide(Some("1"), true), Decision::Run);
        let log = scratch("present").join("skips.log");
        let _ = std::fs::remove_file(&log);
        let mut out = Vec::new();
        assert!(apply(Decision::Run, "t::present", "x/y", Some(&log), &mut out));
        assert!(!log.exists() && out.is_empty(), "a run prints and logs nothing");
        // A tracked path is present in every checkout.
        assert!(require_paths("t::tracked", &["tools/xtask/Cargo.toml"]));
    }

    #[test]
    fn absent_under_the_switch_skips_with_one_logged_line() {
        let log = scratch("skip").join("skips.log");
        let _ = std::fs::remove_file(&log);
        let mut out = Vec::new();
        assert!(!apply(decide(Some("1"), false), "m::t", "references/x/y.rsc", Some(&log), &mut out));
        assert!(!apply(decide(Some("1"), false), "m::u", "sdk/apps/z", Some(&log), &mut out));
        let text = std::fs::read_to_string(&log).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), text, "the printed lines are the logged ones");
        assert_eq!(
            text,
            "SKIPPED m::t: references/x/y.rsc absent (NEWTOS_CI=1)\n\
             SKIPPED m::u: sdk/apps/z absent (NEWTOS_CI=1)\n"
        );
        assert_eq!(
            summary(&text).unwrap(),
            "xtask test: 2 test(s) SKIPPED under NEWTOS_CI=1 (input absent):\n  \
             SKIPPED m::t: references/x/y.rsc absent (NEWTOS_CI=1)\n  \
             SKIPPED m::u: sdk/apps/z absent (NEWTOS_CI=1)"
        );
    }

    #[test]
    #[should_panic(expected = "m::t: test input references/x/y.rsc absent")]
    fn absent_without_the_switch_fails() {
        apply(decide(None, false), "m::t", "references/x/y.rsc", None, &mut Vec::new());
    }

    #[test]
    fn nothing_skipped_prints_nothing() {
        assert_eq!(summary(""), None);
        assert_eq!(summary("unrelated\n"), None);
    }
}
