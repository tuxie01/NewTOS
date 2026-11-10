//! TOOL39 — `tools/deps/gen-deps.py` refuses to run without the pinned
//! cflow. The check (`check_cflow`) is driven here with fake `cflow`
//! executables in a scratch directory, so the test needs no real cflow:
//! a missing tool and a wrong version stop it with one line and exit 1,
//! the pinned version passes.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use crate::workspace_root;

/// Loads gen-deps.py as a module (its `main` is guarded) and calls
/// `check_cflow` with the pin given on the command line.
const DRIVER: &str = "\
import importlib.util, sys
spec = importlib.util.spec_from_file_location('gen_deps', sys.argv[1])
mod = importlib.util.module_from_spec(spec)
spec.loader.exec_module(mod)
path, first = mod.check_cflow({'ORACLE_CFLOW': sys.argv[2], 'ORACLE_CFLOW_VERSION': sys.argv[3]})
print('ok ' + path + ' | ' + first)
";

fn scratch_dir(name: &str) -> PathBuf {
    let dir = workspace_root()
        .join("target")
        .join("xtask-gen-deps")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

/// A fake `cflow` that prints `first_line` for `--version`.
fn fake_cflow(dir: &Path, first_line: &str, mode: u32) -> PathBuf {
    let path = dir.join("cflow");
    std::fs::write(
        &path,
        format!("#!/bin/sh\necho '{first_line}'\necho 'Copyright (C) fake'\n"),
    )
    .expect("write fake cflow");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).expect("chmod");
    path
}

fn check(cflow: &Path, version: &str) -> Output {
    let script = workspace_root().join("tools").join("deps").join("gen-deps.py");
    let python = std::env::var_os("PYTHON").unwrap_or_else(|| "python3".into());
    Command::new(&python)
        .arg("-I")
        .arg("-c")
        .arg(DRIVER)
        .arg(&script)
        .arg(cflow)
        .arg(version)
        .output()
        .unwrap_or_else(|e| panic!("cannot run {python:?}: {e}"))
}

/// Exit 1, nothing on stdout, exactly one stderr line naming the tool, the
/// expected version and path, what was found, and the fix; no traceback.
fn assert_stops(out: &Output, cflow: &Path, found: &str) {
    let err = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "exit code; stderr: {err}");
    assert!(out.stdout.is_empty(), "stdout: {}", String::from_utf8_lossy(&out.stdout));
    let want = format!(
        "gen-deps: cflow: expected GNU cflow 1.8 at {}, found {found}; fix: brew install cflow\n",
        cflow.display()
    );
    assert_eq!(err, want);
}

#[test]
fn gen_deps_stops_when_the_pinned_cflow_is_missing() {
    let dir = scratch_dir("missing");
    let cflow = dir.join("cflow");
    assert_stops(&check(&cflow, "1.8"), &cflow, "no file there");
}

#[test]
fn gen_deps_stops_on_another_cflow_version() {
    let dir = scratch_dir("wrong-version");
    let cflow = fake_cflow(&dir, "/x/cflow (GNU cflow) 1.7", 0o755);
    assert_stops(&check(&cflow, "1.8"), &cflow, "version 1.7");
}

#[test]
fn gen_deps_stops_on_a_cflow_that_is_not_gnu_cflow() {
    let dir = scratch_dir("not-gnu");
    let cflow = fake_cflow(&dir, "cflow 1.8", 0o755);
    assert_stops(
        &check(&cflow, "1.8"),
        &cflow,
        "no GNU cflow version line ('cflow 1.8')",
    );
}

#[test]
fn gen_deps_stops_on_a_cflow_it_cannot_run() {
    let dir = scratch_dir("not-executable");
    let cflow = fake_cflow(&dir, "/x/cflow (GNU cflow) 1.8", 0o644);
    assert_stops(
        &check(&cflow, "1.8"),
        &cflow,
        "not runnable (Permission denied)",
    );
}

#[test]
fn gen_deps_accepts_the_pinned_cflow_version() {
    let dir = scratch_dir("right-version");
    let cflow = fake_cflow(&dir, "/x/cflow (GNU cflow) 1.8", 0o755);
    let out = check(&cflow, "1.8");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.stderr.is_empty(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        format!("ok {} | /x/cflow (GNU cflow) 1.8\n", cflow.display())
    );
}
