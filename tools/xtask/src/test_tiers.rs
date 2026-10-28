//! TOOL04 — the two tiers of `cargo xtask test`.
//!
//! * **full** (`cargo xtask test`, `cargo xtask test --full`): every
//!   host-runnable test of [`crate::HOST_TEST_PACKAGES`] — unit tests,
//!   integration tests and doctests, one `cargo test` over the three packages.
//! * **fast** (`cargo xtask test --fast`): the same tests without the
//!   integration-test targets in [`FULL_ONLY`] and without the doctests of
//!   [`DOCTEST_PACKAGES`].
//!
//! Every test belongs to exactly one tier: the fast tier is "all targets
//! minus [`FULL_ONLY`] minus the doctests", the full tier is "all", so a new
//! `tos-api/tests/*.rs` file lands in the fast tier without being listed
//! anywhere. The fast tier names what it left out when it ends.
//!
//! Why the selection is what it is, with the numbers it rests on:
//! `measurements/measurement-2026-10-28-tool04/RUNINFO.TXT`. In short: no
//! integration target is slow (the slowest runs in about half a second), so
//! [`FULL_ONLY`] is empty; the doctests are the one part whose cost does not
//! depend on what changed (rustdoc re-reads both lib crates on every run).

use std::fs;
use std::path::{Path, PathBuf};

/// The package that has integration tests (`tests/*.rs`). `tos-types` and
/// `xtask` have unit tests only; the fast tier runs those in full.
pub const INTEGRATION_PACKAGE: &str = "tos-api";

/// Integration-test targets (file stem in `tos-api/tests/`) that run in the
/// full tier only, each with the measured reason. Empty: on 2026-10-28 the
/// 162 test binaries of the suite ran in 3.8 s together, the slowest
/// (`aes_window_rectlist_capacity`) in 0.52 s.
pub const FULL_ONLY: &[(&str, &str)] = &[];

/// The packages with a library target, i.e. the ones `cargo test` runs
/// doctests for (`xtask` is a binary). The fast tier skips these doctests.
pub const DOCTEST_PACKAGES: &[&str] = &["tos-api", "tos-types"];

/// Why the doctests are full-tier only (measured 2026-10-28, TOOL04).
pub const DOCTESTS_WHY: &str = "rustdoc re-reads both lib crates on every run \
     (16 s of an 18.7 s full run with nothing changed) for no doctest that \
     runs (one `ignore`d block)";

/// The integration-test targets cargo discovers in `<package_dir>/tests`:
/// every `tests/*.rs` and every `tests/<dir>/main.rs` (cargo's
/// auto-discovery; the package declares no `[[test]]` and keeps
/// `autotests` on, which [`check_autodiscovery`] holds). Sorted.
pub fn integration_targets(package_dir: &Path) -> Result<Vec<String>, String> {
    let dir = package_dir.join("tests");
    let mut out = Vec::new();
    let entries = fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for e in entries {
        let e = e.map_err(|e| format!("{}: {e}", dir.display()))?;
        let p = e.path();
        let name = e.file_name().to_string_lossy().into_owned();
        if p.is_dir() {
            if p.join("main.rs").is_file() {
                out.push(name);
            }
        } else if let Some(stem) = name.strip_suffix(".rs") {
            out.push(stem.to_string());
        }
    }
    out.sort();
    Ok(out)
}

/// The manifest must leave target discovery to cargo, or
/// [`integration_targets`] would not be the list cargo builds.
pub fn check_autodiscovery(manifest: &str) -> Result<(), String> {
    for line in manifest.lines() {
        let l = line.trim();
        if l == "[[test]]" || l.starts_with("autotests") {
            return Err(format!(
                "{INTEGRATION_PACKAGE}/Cargo.toml declares `{l}`: the fast tier \
                 discovers tests/*.rs itself and must learn about it first"
            ));
        }
    }
    Ok(())
}

/// Split the discovered targets into (fast, full-only). Every [`FULL_ONLY`]
/// entry must name a discovered target, so a renamed or deleted test cannot
/// leave a stale entry behind that silently selects nothing.
pub fn split(targets: &[String]) -> Result<(Vec<&str>, Vec<&str>), String> {
    for (name, _) in FULL_ONLY {
        if !targets.iter().any(|t| t == name) {
            return Err(format!(
                "test tiers: FULL_ONLY names `{name}`, which is not a test target of {INTEGRATION_PACKAGE}"
            ));
        }
    }
    let (full, fast): (Vec<&str>, Vec<&str>) = targets
        .iter()
        .map(String::as_str)
        .partition(|t| FULL_ONLY.iter().any(|(n, _)| n == t));
    Ok((fast, full))
}

/// Above this many entries in the host `debug/deps` directory, `cargo xtask
/// test` warns (both tiers). A cold host build leaves about 3 400; with the
/// old test profile (`split-debuginfo` unpacked) every tos-api edit added
/// ~3 000 objects that nothing removed, and every rustc lists the whole
/// directory at start (516 186 entries made a relink take 165-212 s instead
/// of ~5 s; `measurements/measurement-2026-10-28-tool04/RUNINFO.TXT`,
/// results 4-5).
pub const DEPS_WARN_ENTRIES: usize = 20_000;

/// The directory the host test builds put their artifacts in:
/// `$CARGO_TARGET_DIR/debug/deps` (relative to `root`, the directory cargo
/// runs in, when relative), else `<root>/target/debug/deps`.
pub fn host_deps_dir(root: &Path, cargo_target_dir: Option<&std::ffi::OsStr>) -> PathBuf {
    let target = match cargo_target_dir {
        Some(d) if !d.is_empty() => root.join(d),
        _ => root.join("target"),
    };
    target.join("debug").join("deps")
}

/// Number of entries in `dir`; `None` if it does not exist (no host build
/// yet) or cannot be read.
pub fn count_entries(dir: &Path) -> Option<usize> {
    Some(fs::read_dir(dir).ok()?.count())
}

/// The warning for `count` entries in `dir`, or `None` if `count` does not
/// exceed `limit`.
pub fn deps_warning(count: usize, limit: usize, dir: &Path) -> Option<String> {
    if count <= limit {
        return None;
    }
    Some(format!(
        "warning: xtask test: {} holds {count} entries (warning above {limit}).\n\
         \x20 Cause: every rustc lists this directory when it starts, so a large one slows every\n\
         \x20 build (516 186 entries: relink after a tos-api edit 165-212 s instead of ~5 s;\n\
         \x20 measurements/measurement-2026-10-28-tool04/RUNINFO.TXT, result 4, finding 1).\n\
         \x20 It grows by ~3 000 objects per edit when the test profile leaves the codegen-unit\n\
         \x20 objects there: check that Cargo.toml still has `[profile.test] split-debuginfo = \"packed\"`.\n\
         \x20 Remedy: delete {} (host builds only; the kernel's target/aarch64-*/ stays;\n\
         \x20 a cold host test build takes ~25 s). Not cargo clean, which also drops the kernel builds.",
        dir.display(),
        dir.parent().unwrap_or(dir).display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> std::path::PathBuf {
        crate::workspace_root()
    }

    #[test]
    fn integration_package_leaves_discovery_to_cargo() {
        let m = fs::read_to_string(root().join(INTEGRATION_PACKAGE).join("Cargo.toml")).unwrap();
        check_autodiscovery(&m).unwrap();
    }

    #[test]
    fn a_declared_test_target_is_refused() {
        assert!(check_autodiscovery("[package]\n[[test]]\nname = \"x\"\n").is_err());
        assert!(check_autodiscovery("[package]\nautotests = false\n").is_err());
        assert!(check_autodiscovery("[package]\nname = \"tos-api\"\n").is_ok());
    }

    #[test]
    fn every_full_only_entry_is_a_target_and_has_a_reason() {
        let t = integration_targets(&root().join(INTEGRATION_PACKAGE)).unwrap();
        split(&t).unwrap();
        for (name, why) in FULL_ONLY {
            assert!(!why.trim().is_empty(), "FULL_ONLY `{name}` has no reason");
        }
    }

    #[test]
    fn each_target_is_in_exactly_one_tier() {
        let t = integration_targets(&root().join(INTEGRATION_PACKAGE)).unwrap();
        assert!(!t.is_empty());
        let (fast, full) = split(&t).unwrap();
        assert_eq!(fast.len() + full.len(), t.len());
        for x in &t {
            let n = fast.iter().filter(|f| *f == x).count() + full.iter().filter(|f| *f == x).count();
            assert_eq!(n, 1, "`{x}` is in {n} tiers");
        }
    }

    #[test]
    fn doctest_packages_are_the_host_packages_with_a_lib() {
        for p in crate::HOST_TEST_PACKAGES {
            let dir = if p == "xtask" { root().join("tools").join("xtask") } else { root().join(p) };
            assert!(dir.join("Cargo.toml").is_file(), "no package dir for `{p}`");
            let has_lib = dir.join("src").join("lib.rs").is_file();
            assert_eq!(
                has_lib,
                DOCTEST_PACKAGES.contains(&p),
                "`{p}`: lib target {has_lib}, but DOCTEST_PACKAGES says otherwise"
            );
        }
    }

    #[test]
    fn deps_warning_only_above_the_limit_and_names_count_cause_remedy() {
        let d = Path::new("/x/target/debug/deps");
        assert_eq!(deps_warning(DEPS_WARN_ENTRIES, DEPS_WARN_ENTRIES, d), None);
        assert_eq!(deps_warning(0, DEPS_WARN_ENTRIES, d), None);
        let w = deps_warning(DEPS_WARN_ENTRIES + 1, DEPS_WARN_ENTRIES, d).unwrap();
        assert!(w.contains("20001 entries"), "{w}");
        assert!(w.contains("/x/target/debug/deps"), "{w}");
        assert!(w.contains("measurements/measurement-2026-10-28-tool04/RUNINFO.TXT"), "{w}");
        assert!(w.contains("split-debuginfo = \"packed\""), "{w}");
        assert!(w.contains("delete /x/target/debug "), "{w}");
    }

    #[test]
    fn the_cited_run_exists() {
        assert!(root().join("measurements/measurement-2026-10-28-tool04/RUNINFO.TXT").is_file());
    }

    #[test]
    fn the_test_profile_keeps_packed_debuginfo() {
        let m = fs::read_to_string(root().join("Cargo.toml")).unwrap();
        let profile = m.split("[profile.test]").nth(1).expect("no [profile.test]");
        let profile = profile.split("\n[").next().unwrap();
        assert!(
            profile.lines().any(|l| l.trim() == "split-debuginfo = \"packed\""),
            "[profile.test] lost split-debuginfo = \"packed\""
        );
    }

    #[test]
    fn host_deps_dir_follows_cargo_target_dir() {
        let r = Path::new("/ws");
        assert_eq!(host_deps_dir(r, None), Path::new("/ws/target/debug/deps"));
        assert_eq!(host_deps_dir(r, Some("".as_ref())), Path::new("/ws/target/debug/deps"));
        assert_eq!(host_deps_dir(r, Some("t2".as_ref())), Path::new("/ws/t2/debug/deps"));
        assert_eq!(host_deps_dir(r, Some("/abs".as_ref())), Path::new("/abs/debug/deps"));
    }

    #[test]
    fn counted_directory_triggers_the_warning_past_a_small_limit() {
        let base = root().join("target").join("xtask-test-tiers-deps");
        let _ = fs::remove_dir_all(&base);
        assert_eq!(count_entries(&base), None);
        fs::create_dir_all(base.join("sub.dSYM")).unwrap();
        for i in 0..4 {
            fs::write(base.join(format!("f{i}.o")), "").unwrap();
        }
        let n = count_entries(&base).unwrap();
        assert_eq!(n, 5);
        assert!(deps_warning(n, 5, &base).is_none());
        assert!(deps_warning(n, 4, &base).unwrap().contains("5 entries"));
        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn discovery_takes_rs_files_and_main_rs_dirs_only() {
        let base = root().join("target").join("xtask-test-tiers");
        let d = base.join("pkg").join("tests");
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(d.join("with_main")).unwrap();
        fs::create_dir_all(d.join("helper_mod")).unwrap();
        fs::write(d.join("a.rs"), "").unwrap();
        fs::write(d.join("notes.txt"), "").unwrap();
        fs::write(d.join("with_main").join("main.rs"), "").unwrap();
        fs::write(d.join("helper_mod").join("mod.rs"), "").unwrap();
        let t = integration_targets(&base.join("pkg")).unwrap();
        assert_eq!(t, vec!["a".to_string(), "with_main".to_string()]);
        fs::remove_dir_all(&base).unwrap();
    }
}
