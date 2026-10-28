//! TOOL09 — `cargo xtask tosdump`: one tosdump case table on NewTOS in one
//! command, the procedure the per-run scripts carried until 2026-10-28
//! (`measurements/measurement-2026-10-28-bios-storage-audit/method/run-dumpbios-newtos.sh`
//! and its copies).
//!
//! ```text
//! cargo xtask tosdump <dumper> <table.tsv> --out <dir> [--reference <BIN>]...
//!       [--tree <dir>] [--features a,b] [--disk | --testdisk <img>]
//!       [--scratch <dir>] [--time-limit <s>]
//! ```
//!
//! The run, in order:
//!
//! 1. **Measured tree** (`--tree`, default the tree this xtask belongs to):
//!    `cargo xtask build --board rpi4 --release [--features …]` *in that
//!    tree*, with the variables the outer `cargo run` set removed (so the
//!    tree's own `rust-toolchain.toml` and its own `target/` apply), then the
//!    dumper with the tree's `tosdump/build-dump*-newtos.sh`. The tree's
//!    `target/` must be its own: before and after the build every artifact
//!    dep-info file (`target/**/<artifact>.d`) must name an artifact under
//!    that `target/`; a `target/` copied from another tree (an APFS clone of
//!    the main tree) is refused, because its xtask binary and dep-info carry
//!    the other tree's absolute paths and would build — and measure — that
//!    tree (`docs/audits/2026-10-28-bios-core.md` section 11, point 2). A
//!    separate `CARGO_TARGET_DIR` is not used: the build code of every
//!    commit up to TOOL09 hardcodes `<tree>/target` (xtask's kernel ELF
//!    path, `kernel/embed.rs`, `programs/build.sh`), so a foreign target
//!    directory would link stale programs.
//! 2. **Instruments** come from the harness tree (this xtask's): the card
//!    (`tools/mkcard.sh`, `tools/mktestdisk.sh`), `tosdiff`
//!    (`tosdump/host`), `tools/biostrace-check.py`.
//! 3. **Card** in the scratch directory, `NEWDESK.INF` = `#R 02` + `#Z 01
//!    <drive>:\<DUMPER>.NAP@` (the auditors' `z-dumpbios.inf`); booted as an
//!    APFS clone (`cp -c`) so the card itself stays as built.
//!    `--disk`/`--testdisk`: the disk round of TOOL07 — C: is the test image
//!    (built by `tools/mktestdisk.sh` with the autorun INF, or the given one),
//!    the dumper and table on D: (`MKCARD_TESTDISK`); afterwards the test
//!    partition of the card and of the booted clone are compared (raw, then
//!    with the root directory's FAT time stamps zeroed) and `fsck_msdos -n`
//!    runs on both.
//! 4. **QEMU** `raspi4b`, headless, `-append serial=on`, serial to
//!    `<out>/BOOT-SERIAL.txt`; the run ends when the desktop is up after the
//!    dumper, on a kernel panic / unhandled exception / program crash screen
//!    (own message, [`scan_serial`]), when QEMU exits, or at the time limit.
//! 5. **Results** into `--out` (must be new or empty): `<AREA>.BIN`/`.LOG`
//!    (mcopy from the program partition), the card manifest(s), the dumper,
//!    the build logs, `tosdiff --validate` and one `tosdiff` per
//!    `--reference`, with `--features bios-arg-trace` the input check
//!    (`tools/biostrace-check.py`), and `tosdump-run.txt` — commit, dirty
//!    state, kernel and image sha256 (the image also after the run), the
//!    manifests' `optional_missing` lines verbatim, every result line.
//!
//! Scratch: `--scratch <dir>` or `$CLAUDE_JOB_DIR/tmp`, never `/tmp` itself;
//! the run's subdirectory (card, clone, test partitions) is deleted at the
//! end, also on failure. Differences found by tosdiff or the checker are
//! findings, not failures of this command (exit 0); a refused dump, a
//! crash, a timeout or a failed step ends with a non-zero status after
//! everything that exists has been saved.

use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

/// One tosdump dumper: its program file, the area its files are named after
/// (`<AREA>.TSV/.BIN/.LOG`, `tosdump/core/dump*.c` defaults) and its NewTOS
/// build script.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dumper {
    pub nap: &'static str,
    pub area: &'static str,
    pub script: &'static str,
}

/// The table-driven dumpers with a NewTOS build script.
pub const DUMPERS: [Dumper; 4] = [
    Dumper { nap: "DUMPBIOS.NAP", area: "BIOS", script: "tosdump/build-dumpbios-newtos.sh" },
    Dumper { nap: "DUMPVDI.NAP", area: "VDI", script: "tosdump/build-dumpvdi-newtos.sh" },
    Dumper { nap: "DUMPAES.NAP", area: "AES", script: "tosdump/build-dumpaes-newtos.sh" },
    Dumper { nap: "DUMPRAST.NAP", area: "RASTER", script: "tosdump/build-dumprast-newtos.sh" },
];

/// Where the dumper comes from: built in the measured tree, or a prebuilt
/// `DUMP*.NAP` given by path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DumperSource {
    Build(Dumper),
    Prebuilt(Dumper, PathBuf),
}

impl DumperSource {
    pub fn dumper(&self) -> Dumper {
        match self {
            DumperSource::Build(d) | DumperSource::Prebuilt(d, _) => *d,
        }
    }
}

/// The disk round (TOOL07): none, the standard test image built here, or a
/// given one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Disk {
    None,
    Build,
    Image(PathBuf),
}

/// The parsed command line; every path absolute (relative ones resolved
/// against the caller's working directory, not the measured tree).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Opts {
    pub dumper: DumperSource,
    pub table: PathBuf,
    pub out: PathBuf,
    pub references: Vec<PathBuf>,
    pub tree: Option<PathBuf>,
    pub features: Vec<String>,
    pub disk: Disk,
    pub scratch: Option<PathBuf>,
    pub time_limit: u64,
}

/// Default time limit of the QEMU run, seconds (the per-run scripts waited
/// 240 × 2 s).
pub const DEFAULT_TIME_LIMIT: u64 = 480;

/// The kernel feature that turns on the BIOS/XBIOS argument tracer (TOOL08);
/// with it the run ends with the input check.
pub const ARG_TRACE_FEATURE: &str = "bios-arg-trace";

pub const USAGE: &str = "\
usage: cargo xtask tosdump <dumper> <table.tsv> --out <dir> [--reference <BIN>]...
         [--tree <dir>] [--features a,b] [--disk | --testdisk <img>]
         [--scratch <dir>] [--time-limit <s>]
  <dumper>  bios | vdi | aes | rast (built in the measured tree), or the path
            of a prebuilt DUMPBIOS/DUMPVDI/DUMPAES/DUMPRAST.NAP";

/// `p` made absolute against `cwd` and lexically normalised (`.` dropped,
/// `..` folded); the file need not exist.
pub fn absolute(cwd: &Path, p: &Path) -> PathBuf {
    let joined = if p.is_absolute() { p.to_path_buf() } else { cwd.join(p) };
    let mut out = PathBuf::new();
    for c in joined.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// The dumper argument: a name (`bios`, `DUMPBIOS`, `dumpbios.nap`, …) or a
/// path to a prebuilt `DUMP*.NAP` (it contains a `/`).
pub fn parse_dumper(arg: &str, cwd: &Path) -> Result<DumperSource, String> {
    if arg.contains('/') {
        let path = absolute(cwd, Path::new(arg));
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_ascii_uppercase())
            .unwrap_or_default();
        let d = DUMPERS
            .iter()
            .find(|d| d.nap == name)
            .ok_or_else(|| format!("prebuilt dumper {}: the file must be one of {}", path.display(), nap_names()))?;
        return Ok(DumperSource::Prebuilt(*d, path));
    }
    let mut key = arg.to_ascii_uppercase();
    if let Some(k) = key.strip_suffix(".NAP") {
        key = k.to_string();
    }
    if let Some(k) = key.strip_prefix("DUMP") {
        key = k.to_string();
    }
    let area = if key == "RAST" { "RASTER".to_string() } else { key };
    DUMPERS
        .iter()
        .find(|d| d.area == area)
        .map(|d| DumperSource::Build(*d))
        .ok_or_else(|| format!("unknown dumper '{arg}': bios, vdi, aes, rast or a path to {}", nap_names()))
}

fn nap_names() -> String {
    DUMPERS.iter().map(|d| d.nap).collect::<Vec<_>>().join(", ")
}

/// Parse `tosdump`'s arguments; `cwd` resolves every relative path.
pub fn parse_args(args: &[String], cwd: &Path) -> Result<Opts, String> {
    let mut positional: Vec<&str> = Vec::new();
    let mut out = None;
    let mut references = Vec::new();
    let mut tree = None;
    let mut features: Vec<String> = Vec::new();
    let mut disk = Disk::None;
    let mut scratch = None;
    let mut time_limit = DEFAULT_TIME_LIMIT;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let mut value = |flag: &str| it.next().cloned().ok_or(format!("{flag} requires a value"));
        match a.as_str() {
            "--out" => out = Some(absolute(cwd, Path::new(&value("--out")?))),
            "--reference" => references.push(absolute(cwd, Path::new(&value("--reference")?))),
            "--tree" => tree = Some(absolute(cwd, Path::new(&value("--tree")?))),
            "--scratch" => scratch = Some(absolute(cwd, Path::new(&value("--scratch")?))),
            "--features" => features.extend(
                value("--features")?
                    .split([',', ' '])
                    .filter(|s| !s.is_empty())
                    .map(str::to_string),
            ),
            "--time-limit" => {
                let v = value("--time-limit")?;
                time_limit = v
                    .parse::<u64>()
                    .ok()
                    .filter(|&s| s > 0)
                    .ok_or(format!("--time-limit: '{v}' is not a positive number of seconds"))?;
            }
            "--disk" | "--testdisk" => {
                if disk != Disk::None {
                    return Err("--disk and --testdisk are one choice; give one of them once".into());
                }
                disk = if a == "--disk" {
                    Disk::Build
                } else {
                    Disk::Image(absolute(cwd, Path::new(&value("--testdisk")?)))
                };
            }
            s if s.starts_with("--") => return Err(format!("tosdump: unknown option {s}\n{USAGE}")),
            s => positional.push(s),
        }
    }
    let [dumper, table] = positional[..] else {
        return Err(format!("tosdump: expected <dumper> <table.tsv>, got {} argument(s)\n{USAGE}", positional.len()));
    };
    let dumper = parse_dumper(dumper, cwd)?;
    if disk != Disk::None && dumper.dumper().area != "BIOS" {
        return Err("--disk/--testdisk: the disk round is a DUMPBIOS table (TOOL07)".into());
    }
    Ok(Opts {
        dumper,
        table: absolute(cwd, Path::new(table)),
        out: out.ok_or(format!("tosdump: --out <dir> is required\n{USAGE}"))?,
        references,
        tree,
        features,
        disk,
        scratch,
        time_limit,
    })
}

/// The scratch directory: `--scratch`, else `$CLAUDE_JOB_DIR/tmp`; the shared
/// `/tmp` itself is refused (CLAUDE.md, placement rules: scratch never in
/// the shared `/tmp`).
pub fn scratch_dir(given: Option<&Path>, job_dir: Option<&Path>) -> Result<PathBuf, String> {
    let dir = match (given, job_dir) {
        (Some(g), _) => g.to_path_buf(),
        (None, Some(j)) => j.join("tmp"),
        (None, None) => return Err("tosdump: no scratch directory: pass --scratch <dir> or set CLAUDE_JOB_DIR".into()),
    };
    let s = dir.to_string_lossy().trim_end_matches('/').to_string();
    if s == "/tmp" || s == "/private/tmp" || s.is_empty() {
        return Err(format!("tosdump: scratch {} is the shared /tmp; give a directory of your own", dir.display()));
    }
    Ok(dir)
}

/// `NEWDESK.INF` that autostarts `nap` from `drive` (`#Z`, ISGEM 01), with
/// the `#R 02` revision line every desktop writer emits — byte for byte the
/// INF of the per-run scripts (`z-dumpbios.inf`, `newdesk-d.inf`).
pub fn autorun_inf(drive: char, nap: &str) -> String {
    format!("#R 02\r\n#Z 01 {drive}:\\{nap}@\r\n")
}

/// Does `inf` autostart `drive:\nap` (a `#Z` line naming it)? Used to check
/// a given test image's `C:\NEWDESK.INF`.
pub fn inf_autoruns(inf: &[u8], drive: char, nap: &str) -> bool {
    let want = format!("{drive}:\\{nap}@").to_ascii_uppercase();
    String::from_utf8_lossy(inf).split(['\r', '\n']).any(|line| {
        let Some(rest) = line.strip_prefix("#Z") else { return false };
        let rest = rest.trim_start();
        let rest = rest.get(2..).unwrap_or("").trim_start();
        rest.to_ascii_uppercase() == want
    })
}

/// One MBR partition entry: start LBA and sector count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Part {
    pub start: u64,
    pub count: u64,
}

/// The four primary MBR entries (`None` for an empty slot) of a disk image's
/// first sector.
pub fn mbr_partitions(sector0: &[u8]) -> Result<[Option<Part>; 4], String> {
    if sector0.len() < 512 || sector0[510] != 0x55 || sector0[511] != 0xAA {
        return Err("no MBR signature 55 AA".into());
    }
    let mut p = [None; 4];
    for (i, slot) in p.iter_mut().enumerate() {
        let e = &sector0[446 + 16 * i..446 + 16 * (i + 1)];
        let le = |o: usize| u32::from_le_bytes([e[o], e[o + 1], e[o + 2], e[o + 3]]) as u64;
        if e[4] != 0 {
            *slot = Some(Part { start: le(8), count: le(12) });
        }
    }
    Ok(p)
}

/// How a run stands, read off the serial log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunState {
    /// Before the dumper started.
    Booting,
    /// The dumper started, the desktop is not up yet.
    Running,
    /// The desktop is up after the dumper; `rc` from its exit line.
    Done { rc: Option<i64> },
    /// The desktop came up and the dumper never started (the `#Z` line did
    /// not take).
    NoDumper,
    /// A kernel panic, an unhandled exception or a program crash screen; the
    /// marker line and what follows it.
    Abnormal(String),
}

/// The serial lines that end a run abnormally: the kernel panic handler
/// (`kernel/src/panic.rs:17@on_panic`), an unhandled exception
/// (`kernel/src/arch/aarch64/exceptions.rs:2420@unhandled`; both halt), and
/// a program crash screen (`kernel/src/arch/aarch64/crash.rs:76@report_and_wait`),
/// which waits for a key that a headless run never sends.
const ABNORMAL: [(&str, &str); 3] = [
    ("[PANIC]", "kernel panic"),
    ("[EXC]", "unhandled exception"),
    ("[crash] slot", "program crash screen (waits for a key)"),
];

/// Read the serial log of a run of `nap` (e.g. `DUMPBIOS.NAP`): the shell's
/// `sh_ldapp: starting <d>:\<nap>,` and `sh_ldapp: '<d>:\<nap>' exited with
/// rc=<n>` lines, the desktop's `desktop up`, and the abnormal markers
/// ([`ABNORMAL`]). An abnormal marker wins over everything after it.
pub fn scan_serial(text: &str, nap: &str) -> RunState {
    let nap_uc = nap.to_ascii_uppercase();
    let mut started = false;
    let mut rc = None;
    let lines: Vec<&str> = text.lines().collect();
    for (i, raw) in lines.iter().enumerate() {
        let line = raw.trim_start();
        if let Some((_, what)) = ABNORMAL.iter().find(|(m, _)| line.starts_with(m)) {
            let mut msg = format!("{what}: {}", plain(line));
            for next in lines[i + 1..].iter().map(|l| plain(l)).filter(|l| !l.is_empty()).take(2) {
                msg.push_str(" | ");
                msg.push_str(&next);
            }
            return RunState::Abnormal(msg);
        }
        let uc = line.to_ascii_uppercase();
        if let Some(p) = uc.find("SH_LDAPP: STARTING ") {
            let prog = &uc[p + "SH_LDAPP: STARTING ".len()..];
            if prog.get(1..3) == Some(":\\") && prog[3..].starts_with(&format!("{nap_uc},")) {
                started = true;
            }
        }
        if started {
            if let Some(p) = uc.find(&format!(":\\{nap_uc}' EXITED WITH RC=")) {
                let tail = &line[p + nap.len() + ":\\' exited with rc=".len()..];
                let num: String = tail.chars().take_while(|c| *c == '-' || c.is_ascii_digit()).collect();
                rc = num.parse::<i64>().ok();
            }
        }
        if line.contains("desktop up") {
            return if started { RunState::Done { rc } } else { RunState::NoDumper };
        }
    }
    if started {
        RunState::Running
    } else {
        RunState::Booting
    }
}

/// A console line without its VT52 escape sequences (`ESC x`; `ESC Y r c`,
/// `ESC b c`, `ESC c c` with their argument bytes) and control characters,
/// trimmed — the crash screen starts with `ESC f ESC v`.
fn plain(line: &str) -> String {
    let mut out = String::new();
    let mut it = line.chars();
    while let Some(c) = it.next() {
        if c == '\x1b' {
            let skip = match it.next() {
                Some('Y') => 2,
                Some('b') | Some('c') => 1,
                _ => 0,
            };
            for _ in 0..skip {
                it.next();
            }
        } else if !c.is_control() {
            out.push(c);
        }
    }
    out.trim().to_string()
}

/// The partition with the time stamps of every root directory entry zeroed
/// (bytes 13..=19 create tenth/time/date and access date, 22..=25 write
/// time/date) — the normalisation of
/// `measurements/measurement-2026-10-28-tool07/method/normalise-fat-times.py`.
/// Root directory only, as there: a difference anywhere else stays one.
pub fn normalise_root_times(part: &[u8]) -> Vec<u8> {
    let mut d = part.to_vec();
    if d.len() < 512 {
        return d;
    }
    let u16_at = |o: usize| u16::from_le_bytes([d[o], d[o + 1]]) as usize;
    let bps = u16_at(11);
    let res = u16_at(14);
    let nfats = d[16] as usize;
    let nroot = u16_at(17);
    let spf = u16_at(22);
    let start = (res + nfats * spf) * bps;
    for k in 0..nroot {
        let off = start + 32 * k;
        if off + 32 > d.len() || d[off] == 0 {
            break;
        }
        for i in (13..=19).chain(22..=25) {
            d[off + i] = 0;
        }
    }
    d
}

/// The test-partition comparison of the disk round.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartCompare {
    pub identical: bool,
    /// Partition-relative 512-byte sectors that differ (raw).
    pub differing_sectors: Vec<usize>,
    /// Only when not identical: equal after [`normalise_root_times`]?
    pub identical_after_normalising: Option<bool>,
}

pub fn compare_partitions(before: &[u8], after: &[u8]) -> PartCompare {
    if before == after {
        return PartCompare { identical: true, differing_sectors: Vec::new(), identical_after_normalising: None };
    }
    let n = before.len().max(after.len());
    let mut sectors: Vec<usize> = (0..n)
        .filter(|&i| before.get(i) != after.get(i))
        .map(|i| i / 512)
        .collect();
    sectors.dedup();
    let norm = normalise_root_times(before) == normalise_root_times(after);
    PartCompare { identical: false, differing_sectors: sectors, identical_after_normalising: Some(norm) }
}

/// The artifact a cargo dep-info file (`<artifact>: <deps…>`) was written
/// for, when that artifact is **not** under `target` (`target` given in
/// every spelling the tree may have, e.g. as passed and canonical) — the
/// mark of a `target/` copied from another tree.
pub fn foreign_dep_info(targets: &[PathBuf], dep_info: &str) -> Option<String> {
    let first = dep_info.lines().next()?;
    // The artifact path ends at the first unescaped ": " (cargo escapes a
    // space in a path as "\ ").
    let mut artifact = String::new();
    let mut chars = first.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' if chars.peek() == Some(&' ') => {
                artifact.push(' ');
                chars.next();
            }
            ':' if matches!(chars.peek(), Some(' ') | None) => break,
            c => artifact.push(c),
        }
    }
    // `programs/build.sh` builds with `target-dir = "../target/programs"`,
    // so its artifacts read `<tree>/programs/../target/…`: compare lexically
    // normalised.
    let a = absolute(Path::new("/"), Path::new(&artifact));
    if targets.iter().any(|t| a.starts_with(t)) {
        None
    } else {
        Some(artifact)
    }
}

/// The variables the outer `cargo run` (and rustup's proxy) set for this
/// process that must not reach the build of the measured tree: they would
/// pin this tree's toolchain (`RUSTUP_TOOLCHAIN`, `CARGO`), name this
/// package (`CARGO_PKG_*`, `CARGO_MANIFEST_*`, …) or move the tree's
/// `target/` (`CARGO_TARGET_DIR`).
pub fn scrubbed_env<'a>(keys: impl Iterator<Item = &'a str>) -> Vec<String> {
    keys.filter(|k| {
        matches!(*k, "CARGO" | "RUSTUP_TOOLCHAIN" | "CARGO_TARGET_DIR" | "CARGO_PRIMARY_PACKAGE" | "OUT_DIR")
            || ["CARGO_PKG_", "CARGO_MANIFEST_", "CARGO_BIN_", "CARGO_CRATE_"].iter().any(|p| k.starts_with(p))
    })
    .map(str::to_string)
    .collect()
}

/// The `N of M cases differ` line of a tosdiff comparison (absent when the
/// comparison was refused).
pub fn tosdiff_summary(output: &str) -> Option<&str> {
    output.lines().rev().find(|l| l.ends_with("cases differ")).map(str::trim)
}

// ── the run ─────────────────────────────────────────────────────────────

/// The summary file and stdout, one line per entry.
struct Report {
    lines: Vec<String>,
    path: PathBuf,
}

impl Report {
    fn line(&mut self, s: impl Into<String>) {
        let s = s.into();
        println!("tosdump: {s}");
        self.lines.push(s);
        // Rewritten after every line, so a failed run keeps what it had.
        let _ = fs::write(&self.path, self.lines.join("\n") + "\n");
    }
}

/// QEMU, killed when dropped (normal end, error, panic).
struct Qemu(Child);

impl Drop for Qemu {
    fn drop(&mut self) {
        if let Ok(None) = self.0.try_wait() {
            let _ = Command::new("kill").arg(self.0.id().to_string()).status();
            std::thread::sleep(Duration::from_secs(1));
            let _ = self.0.kill();
        }
        let _ = self.0.wait();
    }
}

/// The run's scratch subdirectory, removed when dropped.
struct ScratchRun(PathBuf);

impl Drop for ScratchRun {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn sha256(p: &Path) -> Result<String, String> {
    let o = Command::new("shasum")
        .args(["-a", "256"])
        .arg(p)
        .output()
        .map_err(|e| format!("shasum: {e}"))?;
    if !o.status.success() {
        return Err(format!("shasum {}: {}", p.display(), String::from_utf8_lossy(&o.stderr).trim()));
    }
    Ok(String::from_utf8_lossy(&o.stdout).split_whitespace().next().unwrap_or("").to_string())
}

/// Run `cmd` with stdout and stderr into `log`.
fn run_logged(mut cmd: Command, log: &Path) -> Result<ExitStatus, String> {
    let f = fs::File::create(log).map_err(|e| format!("{}: {e}", log.display()))?;
    let f2 = f.try_clone().map_err(|e| format!("{}: {e}", log.display()))?;
    let printable = format!("{cmd:?}");
    cmd.stdout(Stdio::from(f))
        .stderr(Stdio::from(f2))
        .status()
        .map_err(|e| format!("failed to spawn `{printable}`: {e}"))
}

fn require_ok(status: ExitStatus, what: &str, log: &Path) -> Result<(), String> {
    if status.success() {
        Ok(())
    } else {
        Err(format!("{what} failed ({status}); see {}", log.display()))
    }
}

/// A command for the measured tree, with [`scrubbed_env`] removed.
fn tree_command(program: &str, tree: &Path) -> Command {
    let mut c = Command::new(program);
    c.current_dir(tree);
    let keys: Vec<String> = std::env::vars_os().map(|(k, _)| k.to_string_lossy().into_owned()).collect();
    for k in scrubbed_env(keys.iter().map(String::as_str)) {
        c.env_remove(k);
    }
    c
}

fn git_line(tree: &Path, args: &[&str]) -> String {
    Command::new("git")
        .arg("-C")
        .arg(tree)
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim_end().to_string())
        .unwrap_or_else(|| "(not a git checkout)".into())
}

/// Every artifact dep-info file of `tree/target` (depth ≤ 3, without the
/// `deps`/`build`/`.fingerprint`/`incremental` subtrees) must name an
/// artifact under that `target/` ([`foreign_dep_info`]).
fn check_target_is_own(tree: &Path) -> Result<usize, String> {
    let target = tree.join("target");
    let mut spellings = vec![target.clone()];
    if let Ok(c) = target.canonicalize() {
        spellings.push(c);
    }
    let mut checked = 0;
    let mut stack = vec![(target.clone(), 0usize)];
    while let Some((dir, depth)) = stack.pop() {
        let Ok(rd) = fs::read_dir(&dir) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            let name = e.file_name().to_string_lossy().into_owned();
            if p.is_dir() {
                if depth < 3 && !matches!(name.as_str(), "deps" | "build" | ".fingerprint" | "incremental" | "examples") {
                    stack.push((p, depth + 1));
                }
            } else if name.ends_with(".d") {
                let text = fs::read_to_string(&p).unwrap_or_default();
                checked += 1;
                if let Some(a) = foreign_dep_info(&spellings, &text) {
                    return Err(format!(
                        "{} belongs to another tree (artifact {a}): this target/ was copied, and building here \
                         would build and measure that tree. Remove {} (or measure a fresh worktree) and run again.",
                        p.display(),
                        target.display()
                    ));
                }
            }
        }
    }
    Ok(checked)
}

fn mcopy_out(image: &Path, offset: u64, name: &str, dest: &Path) -> bool {
    Command::new("mcopy")
        .env("MTOOLS_SKIP_CHECK", "1")
        .env("MTOOLSRC", "/dev/null")
        .args(["-o", "-n", "-i"])
        .arg(format!("{}@@{offset}", image.display()))
        .arg(format!("::/{name}"))
        .arg(dest)
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn read_sector0(image: &Path) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let mut f = fs::File::open(image).map_err(|e| format!("{}: {e}", image.display()))?;
    let mut b = vec![0u8; 512];
    f.read_exact(&mut b).map_err(|e| format!("{}: {e}", image.display()))?;
    Ok(b)
}

fn read_part(image: &Path, p: Part) -> Result<Vec<u8>, String> {
    use std::io::{Read, Seek, SeekFrom};
    let mut f = fs::File::open(image).map_err(|e| format!("{}: {e}", image.display()))?;
    f.seek(SeekFrom::Start(p.start * 512)).map_err(|e| e.to_string())?;
    let mut b = vec![0u8; (p.count * 512) as usize];
    f.read_exact(&mut b).map_err(|e| format!("{}: {e}", image.display()))?;
    Ok(b)
}

fn manifest_lines(manifest: &Path) -> Vec<String> {
    fs::read_to_string(manifest)
        .unwrap_or_default()
        .lines()
        .filter(|l| l.starts_with("image_sha256 ") || l.starts_with("optional_missing"))
        .map(str::to_string)
        .collect()
}

/// `cargo xtask tosdump …` — see the module documentation.
pub fn cmd(harness: &Path, args: &[String]) -> Result<(), String> {
    let cwd = std::env::current_dir().map_err(|e| format!("cwd: {e}"))?;
    let o = parse_args(args, &cwd)?;
    let job = std::env::var_os("CLAUDE_JOB_DIR").map(PathBuf::from);
    let scratch = scratch_dir(o.scratch.as_deref(), job.as_deref())?;
    let tree = o.tree.clone().unwrap_or_else(|| harness.to_path_buf());
    let dumper = o.dumper.dumper();
    let arg_trace = o.features.iter().any(|f| f == ARG_TRACE_FEATURE);

    // Inputs first: every problem before anything is built or booted.
    let mut bad = Vec::new();
    if !tree.join("Cargo.toml").is_file() || !tree.join("tosdump").is_dir() {
        bad.push(format!("--tree {}: not a NewTOS tree (no Cargo.toml / tosdump/)", tree.display()));
    }
    if !o.table.is_file() {
        bad.push(format!("table {} not found", o.table.display()));
    }
    if let DumperSource::Prebuilt(_, p) = &o.dumper {
        if !p.is_file() {
            bad.push(format!("dumper {} not found", p.display()));
        }
    }
    let mut references = Vec::new();
    for r in &o.references {
        let with_bin = PathBuf::from(format!("{}.BIN", r.display()));
        if r.is_file() {
            references.push(r.clone());
        } else if with_bin.is_file() {
            references.push(with_bin);
        } else {
            bad.push(format!("reference {} not found (also not as .BIN)", r.display()));
        }
    }
    if let Disk::Image(t) = &o.disk {
        if !t.is_file() {
            bad.push(format!("--testdisk {} not found", t.display()));
        }
    }
    if o.out.exists() && fs::read_dir(&o.out).map(|mut d| d.next().is_some()).unwrap_or(true) {
        bad.push(format!("--out {} exists and is not empty (runs are not overwritten)", o.out.display()));
    }
    let mut tools = vec!["qemu-system-aarch64", "mcopy", "shasum", "make"];
    if o.disk != Disk::None {
        tools.extend(["fsck_msdos", "mformat"]);
    }
    if arg_trace {
        tools.push("python3");
    }
    for t in tools {
        let found = Command::new("sh")
            .args(["-c", &format!("command -v {t}")])
            .stdout(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !found {
            bad.push(format!("{t} not found on PATH"));
        }
    }
    if !bad.is_empty() {
        return Err(format!("tosdump: {} problem(s), nothing run:\n  {}", bad.len(), bad.join("\n  ")));
    }
    let tree = tree.canonicalize().map_err(|e| format!("{}: {e}", tree.display()))?;

    fs::create_dir_all(&o.out).map_err(|e| format!("{}: {e}", o.out.display()))?;
    let out = o.out.canonicalize().map_err(|e| e.to_string())?;
    let run_dir = scratch.join(format!("tosdump-{}", std::process::id()));
    fs::create_dir_all(&run_dir).map_err(|e| format!("{}: {e}", run_dir.display()))?;
    let _scratch_guard = ScratchRun(run_dir.clone());
    let mut rep = Report { lines: Vec::new(), path: out.join("tosdump-run.txt") };
    let t0 = Instant::now();

    rep.line(format!("command   cargo xtask tosdump {}", args.join(" ")));
    rep.line(format!("cwd       {}", cwd.display()));
    rep.line(format!("tree      {} (measured: kernel, dumper)", tree.display()));
    rep.line(format!("harness   {} (instruments: mkcard, mktestdisk, tosdiff, biostrace-check)", harness.display()));
    rep.line(format!("commit    {}", git_line(&tree, &["rev-parse", "HEAD"])));
    let dirty = git_line(&tree, &["status", "--porcelain"]);
    rep.line(format!("dirty     {} path(s) in git status --porcelain", dirty.lines().filter(|l| !l.is_empty()).count()));
    for l in dirty.lines().filter(|l| !l.is_empty()) {
        rep.line(format!("  {l}"));
    }
    rep.line(format!("harness commit {}", git_line(harness, &["rev-parse", "HEAD"])));
    rep.line(format!("features  {}", if o.features.is_empty() { "(none)".into() } else { o.features.join(",") }));
    rep.line(format!("table     {} sha256 {}", o.table.display(), sha256(&o.table)?));

    // 1. the measured tree's kernel.
    let n = check_target_is_own(&tree)?;
    rep.line(format!("target/   own ({n} artifact dep-info files checked before the build)"));
    let mut c = tree_command("cargo", &tree);
    c.args(["xtask", "build", "--board", "rpi4", "--release"]);
    if !o.features.is_empty() {
        c.args(["--features", &o.features.join(",")]);
    }
    let log = out.join("kernel-build.log");
    rep.line("building  kernel (cargo xtask build --board rpi4 --release) in the measured tree …");
    require_ok(run_logged(c, &log)?, "kernel build", &log)?;
    let n = check_target_is_own(&tree)?;
    rep.line(format!("target/   own ({n} artifact dep-info files checked after the build)"));
    let kimg = run_dir.join("kernel8.img");
    fs::copy(tree.join("kernel8.img"), &kimg).map_err(|e| format!("kernel8.img: {e}"))?;
    let elf = tree.join("target/aarch64-unknown-none-softfloat/release/kernel");
    rep.line(format!("kernel8.img sha256 {}", sha256(&kimg)?));
    rep.line(format!("kernel ELF  sha256 {} ({})", sha256(&elf)?, elf.display()));

    // 2. the dumper.
    let nap = run_dir.join(dumper.nap);
    match &o.dumper {
        DumperSource::Build(d) => {
            let log = out.join("dumper-build.log");
            let mut c = tree_command("bash", &tree);
            c.arg(tree.join(d.script)).arg(&nap);
            rep.line(format!("building  {} ({}) in the measured tree …", d.nap, d.script));
            require_ok(run_logged(c, &log)?, "dumper build", &log)?;
            let text = fs::read_to_string(&log).unwrap_or_default();
            if let Some(id) = text.lines().find_map(|l| l.strip_prefix("build : ")) {
                rep.line(format!("dumper build id {id}"));
            }
        }
        DumperSource::Prebuilt(_, p) => {
            fs::copy(p, &nap).map_err(|e| format!("{}: {e}", p.display()))?;
            rep.line(format!("dumper    prebuilt {}", p.display()));
        }
    }
    fs::copy(&nap, out.join(dumper.nap)).map_err(|e| e.to_string())?;
    rep.line(format!("{} sha256 {}", dumper.nap, sha256(&nap)?));

    // 3. the card.
    let card = run_dir.join("card.img");
    let inf = run_dir.join("NEWDESK.INF");
    let tsv_dest = format!("{}.TSV", dumper.area);
    let mut mk = Command::new("bash");
    mk.current_dir(harness).arg(harness.join("tools/mkcard.sh")).arg(&card).arg(&inf);
    mk.arg(format!("{}={}", dumper.nap, nap.display()));
    mk.arg(format!("{tsv_dest}={}", o.table.display()));
    let mut testimg = None;
    match &o.disk {
        Disk::None => {
            fs::write(&inf, autorun_inf('C', dumper.nap)).map_err(|e| e.to_string())?;
        }
        Disk::Build => {
            fs::write(&inf, autorun_inf('D', dumper.nap)).map_err(|e| e.to_string())?;
            let t = run_dir.join("test.img");
            let log = out.join("mktestdisk.log");
            let mut c = Command::new("bash");
            c.current_dir(harness)
                .arg(harness.join("tools/mktestdisk.sh"))
                .arg(&t)
                .arg(format!("NEWDESK.INF={}", inf.display()));
            require_ok(run_logged(c, &log)?, "tools/mktestdisk.sh", &log)?;
            fs::copy(run_dir.join("test.img.manifest"), out.join("test.img.manifest")).map_err(|e| e.to_string())?;
            rep.line(format!("test image built by tools/mktestdisk.sh NEWDESK.INF=<autorun D:\\{}>", dumper.nap));
            testimg = Some(t);
        }
        Disk::Image(t) => {
            let parts = mbr_partitions(&read_sector0(t)?).map_err(|e| format!("{}: {e}", t.display()))?;
            let p1 = parts[0].ok_or(format!("{}: no partition 1", t.display()))?;
            if !mcopy_out(t, p1.start * 512, "NEWDESK.INF", &inf) {
                return Err(format!("{}: no C:\\NEWDESK.INF on the test image", t.display()));
            }
            if !inf_autoruns(&fs::read(&inf).map_err(|e| e.to_string())?, 'D', dumper.nap) {
                return Err(format!(
                    "{}: its C:\\NEWDESK.INF does not autostart D:\\{} (tools/mktestdisk.sh <img> NEWDESK.INF=<inf with #Z 01 D:\\{}@>)",
                    t.display(),
                    dumper.nap,
                    dumper.nap
                ));
            }
            rep.line(format!("test image {}", t.display()));
            testimg = Some(t.clone());
        }
    }
    if let Some(t) = &testimg {
        rep.line(format!("test image sha256 {}", sha256(t)?));
        mk.env("MKCARD_TESTDISK", t);
    }
    let log = out.join("mkcard.log");
    require_ok(run_logged(mk, &log)?, "tools/mkcard.sh", &log)?;
    fs::copy(run_dir.join("card.img.manifest"), out.join("card.img.manifest")).map_err(|e| e.to_string())?;
    for l in manifest_lines(&out.join("card.img.manifest")) {
        rep.line(format!("card.img.manifest: {l}"));
    }
    if testimg.is_some() && o.disk == Disk::Build {
        for l in manifest_lines(&out.join("test.img.manifest")) {
            rep.line(format!("test.img.manifest: {l}"));
        }
    }
    let parts = mbr_partitions(&read_sector0(&card)?).map_err(|e| format!("card: {e}"))?;
    let prog = if testimg.is_some() { parts[1] } else { parts[0] }.ok_or("card: program partition missing")?;
    let boot = run_dir.join("run.img");
    let st = Command::new("cp").arg("-c").arg(&card).arg(&boot).status().map_err(|e| e.to_string())?;
    if !st.success() {
        return Err("cp -c of the card failed (APFS clone needs the scratch on APFS)".into());
    }
    rep.line(format!("card      {} sha256 {} (booted as an APFS clone)", card.display(), sha256(&card)?));

    // 4. QEMU.
    let serial = out.join("BOOT-SERIAL.txt");
    fs::write(&serial, b"").map_err(|e| e.to_string())?;
    let mut q = Command::new("qemu-system-aarch64");
    q.args(["-M", "raspi4b", "-no-reboot", "-display", "none"])
        .arg("-serial")
        .arg(format!("file:{}", serial.display()))
        .args(["-append", "serial=on", "-kernel"])
        .arg(&kimg)
        .arg("-drive")
        .arg(format!("file={},if=sd,format=raw", boot.display()))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::from(fs::File::create(out.join("qemu-stderr.txt")).map_err(|e| e.to_string())?));
    rep.line(format!("qemu      raspi4b, -append serial=on, time limit {} s", o.time_limit));
    let started = Instant::now();
    let mut qemu = Qemu(q.spawn().map_err(|e| format!("qemu-system-aarch64: {e}"))?);
    let limit = Duration::from_secs(o.time_limit);
    let end: Result<Option<i64>, String> = loop {
        std::thread::sleep(Duration::from_millis(500));
        let text = fs::read_to_string(&serial).unwrap_or_default();
        match scan_serial(&text, dumper.nap) {
            RunState::Done { rc } => {
                // The per-run scripts' grace after "desktop up".
                std::thread::sleep(Duration::from_secs(3));
                break Ok(rc);
            }
            RunState::Abnormal(_) => {
                std::thread::sleep(Duration::from_secs(1));
                let text = fs::read_to_string(&serial).unwrap_or_default();
                let RunState::Abnormal(m) = scan_serial(&text, dumper.nap) else { unreachable!() };
                break Err(m);
            }
            RunState::NoDumper => break Err(format!("the desktop came up without starting {} (#Z autorun)", dumper.nap)),
            state => {
                if let Ok(Some(s)) = qemu.0.try_wait() {
                    break Err(format!("QEMU exited ({s}) in state {state:?}; see qemu-stderr.txt"));
                }
                if started.elapsed() >= limit {
                    let last = text.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("").trim().to_string();
                    break Err(format!("time limit {} s reached in state {state:?}; last serial line: {last}", o.time_limit));
                }
            }
        }
    };
    drop(qemu);
    let secs = started.elapsed().as_secs();
    match &end {
        Ok(rc) => rep.line(format!(
            "run       ended normally after {secs} s: desktop up after {}, rc={}",
            dumper.nap,
            rc.map_or("(no exit line)".into(), |r| r.to_string())
        )),
        Err(m) => rep.line(format!("run       ENDED ABNORMALLY after {secs} s: {m}")),
    }
    rep.line(format!("card after the run sha256 {}", sha256(&boot)?));

    // 5. results.
    let bin = out.join(format!("{}.BIN", dumper.area));
    let logf = out.join(format!("{}.LOG", dumper.area));
    for (name, dest) in [(format!("{}.BIN", dumper.area), &bin), (format!("{}.LOG", dumper.area), &logf)] {
        if mcopy_out(&boot, prog.start * 512, &name, dest) {
            rep.line(format!("{name} {} bytes, sha256 {}", fs::metadata(dest).map(|m| m.len()).unwrap_or(0), sha256(dest)?));
        } else {
            rep.line(format!("{name} MISSING on the card"));
        }
    }
    let mut failures = Vec::new();
    if let Err(m) = &end {
        failures.push(m.clone());
    } else if let Ok(Some(rc)) = end {
        if rc != 0 {
            failures.push(format!("{} exited with rc={rc}", dumper.nap));
        }
    }

    if let Some(t) = &testimg {
        let tp = mbr_partitions(&read_sector0(&boot)?)?[0].ok_or("card: test partition missing")?;
        let before = read_part(&card, tp)?;
        let after = read_part(&boot, tp)?;
        let cmp = compare_partitions(&before, &after);
        let mut txt = vec![
            format!("test image {}", t.display()),
            format!("test partition LBA {}+{}", tp.start, tp.count),
        ];
        if cmp.identical {
            txt.push("testpart: identical".into());
        } else {
            let s: Vec<String> = cmp.differing_sectors.iter().take(50).map(|s| s.to_string()).collect();
            txt.push(format!("testpart: DIFFERENT; differing sectors (partition-relative): {}", s.join(" ")));
            txt.push(if cmp.identical_after_normalising == Some(true) {
                "testpart: identical after normalising root directory time stamps".into()
            } else {
                "testpart: still different after normalising root directory time stamps".into()
            });
        }
        for (label, bytes) in [("before", &before), ("after", &after)] {
            let f = run_dir.join(format!("testpart-{label}.img"));
            fs::write(&f, bytes).map_err(|e| e.to_string())?;
            let o = Command::new("fsck_msdos").arg("-n").arg(&f).output().map_err(|e| e.to_string())?;
            txt.push(format!("fsck_msdos -n testpart-{label}.img: exit {}", o.status.code().unwrap_or(-1)));
            for l in String::from_utf8_lossy(&o.stdout).lines().chain(String::from_utf8_lossy(&o.stderr).lines()) {
                txt.push(format!("  {l}"));
            }
            if !o.status.success() {
                failures.push(format!("fsck_msdos -n on the test partition {label} the run failed"));
            }
        }
        fs::write(out.join("testpart-compare.txt"), txt.join("\n") + "\n").map_err(|e| e.to_string())?;
        for l in txt.iter().filter(|l| l.starts_with("testpart") || l.starts_with("fsck")) {
            rep.line(l.clone());
        }
    }

    if bin.is_file() {
        let log = out.join("make-tosdiff.log");
        let mut c = Command::new("make");
        c.arg("-C").arg(harness.join("tosdump/host")).arg("build/tosdiff");
        require_ok(run_logged(c, &log)?, "make tosdiff", &log)?;
        let tosdiff = harness.join("tosdump/host/build/tosdiff");
        rep.line(format!("tosdiff   {} sha256 {}", tosdiff.display(), sha256(&tosdiff)?));
        let v = Command::new(&tosdiff)
            .arg("--validate")
            .arg(&bin)
            .arg(&o.table)
            .output()
            .map_err(|e| e.to_string())?;
        let vtext = format!("{}{}", String::from_utf8_lossy(&v.stdout), String::from_utf8_lossy(&v.stderr));
        fs::write(out.join("tosdiff-validate.txt"), &vtext).map_err(|e| e.to_string())?;
        rep.line(format!("tosdiff --validate: exit {} {}", v.status.code().unwrap_or(-1), vtext.lines().next().unwrap_or("").trim()));
        if !v.status.success() {
            failures.push("tosdiff --validate refused the dump".into());
        }
        let cmpdir = out.join("cmp");
        fs::create_dir_all(&cmpdir).map_err(|e| e.to_string())?;
        fs::copy(&bin, cmpdir.join("NEWTOS.BIN")).map_err(|e| e.to_string())?;
        for r in &references {
            let name = r.file_name().unwrap().to_string_lossy().into_owned();
            fs::copy(r, cmpdir.join(&name)).map_err(|e| e.to_string())?;
            let d = Command::new(&tosdiff)
                .current_dir(&cmpdir)
                .arg(&o.table)
                .arg(&name)
                .arg("NEWTOS.BIN")
                .output()
                .map_err(|e| e.to_string())?;
            let text = format!("{}{}", String::from_utf8_lossy(&d.stdout), String::from_utf8_lossy(&d.stderr));
            let stem = name.trim_end_matches(".BIN");
            fs::write(out.join(format!("tosdiff-{stem}.txt")), &text).map_err(|e| e.to_string())?;
            let code = d.status.code().unwrap_or(-1);
            rep.line(format!(
                "tosdiff {name} vs NEWTOS.BIN: exit {code}, {}",
                tosdiff_summary(&text).unwrap_or("no summary line")
            ));
            if code == 2 {
                failures.push(format!("tosdiff refused {name}"));
            }
        }
        if arg_trace {
            let checker = harness.join("tools/biostrace-check.py");
            if dumper.area != "BIOS" {
                rep.line(format!("input check skipped: {} is for DUMPBIOS tables", checker.display()));
            } else {
                let c = Command::new("python3")
                    .arg("-I")
                    .arg(&checker)
                    .arg(&serial)
                    .arg(&o.table)
                    .arg("--bin")
                    .arg(&bin)
                    .output()
                    .map_err(|e| e.to_string())?;
                let text = format!("{}{}", String::from_utf8_lossy(&c.stdout), String::from_utf8_lossy(&c.stderr));
                fs::write(out.join("biostrace-check.txt"), &text).map_err(|e| e.to_string())?;
                let code = c.status.code().unwrap_or(-1);
                rep.line(format!(
                    "input check (biostrace-check.py): exit {code}, {}",
                    text.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("").trim()
                ));
                if code == 2 {
                    failures.push("biostrace-check.py could not check the run".into());
                }
            }
        }
    } else {
        failures.push(format!("no {} from the run", bin.file_name().unwrap().to_string_lossy()));
    }
    rep.line(format!("total     {} s", t0.elapsed().as_secs()));
    if failures.is_empty() {
        rep.line("result    run complete (differences above are findings, not failures)");
        Ok(())
    } else {
        rep.line(format!("result    FAILED: {}", failures.join("; ")));
        Err(format!("tosdump: {}", failures.join("; ")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    fn repo() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    #[test]
    fn relative_paths_resolve_against_the_callers_cwd() {
        let cwd = Path::new("/work/audit");
        let o = parse_args(
            &s(&["../tree/DUMPBIOS.NAP", "cases/bios-r1.tsv", "--out", "./runs/r1", "--reference", "/abs/REF"]),
            cwd,
        )
        .unwrap();
        assert_eq!(o.table, Path::new("/work/audit/cases/bios-r1.tsv"));
        assert_eq!(o.out, Path::new("/work/audit/runs/r1"));
        assert_eq!(o.references, vec![PathBuf::from("/abs/REF")]);
        assert_eq!(o.dumper, DumperSource::Prebuilt(DUMPERS[0], PathBuf::from("/work/tree/DUMPBIOS.NAP")));
        assert_eq!(o.tree, None);
        assert_eq!(o.disk, Disk::None);
        assert_eq!(o.time_limit, DEFAULT_TIME_LIMIT);
    }

    #[test]
    fn dumper_names() {
        let cwd = Path::new("/x");
        for (arg, area) in [("bios", "BIOS"), ("DUMPBIOS", "BIOS"), ("dumpvdi.nap", "VDI"), ("aes", "AES"), ("rast", "RASTER"), ("raster", "RASTER")] {
            assert_eq!(parse_dumper(arg, cwd).unwrap().dumper().area, area, "{arg}");
        }
        assert!(parse_dumper("gemdos", cwd).is_err());
        assert!(parse_dumper("./XCPROBE.NAP", cwd).is_err(), "a prebuilt file must be a known dumper");
    }

    #[test]
    fn options() {
        let cwd = Path::new("/c");
        let o = parse_args(
            &s(&["bios", "t.tsv", "--out", "o", "--tree", "../wt", "--features", "bios-arg-trace,x", "--disk", "--scratch", "s", "--time-limit", "60"]),
            cwd,
        )
        .unwrap();
        assert_eq!(o.tree, Some(PathBuf::from("/wt")));
        assert_eq!(o.features, s(&["bios-arg-trace", "x"]));
        assert_eq!(o.disk, Disk::Build);
        assert_eq!(o.scratch, Some(PathBuf::from("/c/s")));
        assert_eq!(o.time_limit, 60);
        let o = parse_args(&s(&["bios", "t.tsv", "--out", "o", "--testdisk", "img/test.img"]), cwd).unwrap();
        assert_eq!(o.disk, Disk::Image(PathBuf::from("/c/img/test.img")));
    }

    #[test]
    fn bad_command_lines_are_refused() {
        let cwd = Path::new("/c");
        for bad in [
            &["bios", "t.tsv"][..],                                  // no --out
            &["bios", "--out", "o"][..],                             // no table
            &["bios", "t.tsv", "x", "--out", "o"][..],               // three positionals
            &["bios", "t.tsv", "--out"][..],                         // flag without value
            &["bios", "t.tsv", "--out", "o", "--disk", "--disk"][..], // twice
            &["bios", "t.tsv", "--out", "o", "--disk", "--testdisk", "i"][..],
            &["vdi", "t.tsv", "--out", "o", "--disk"][..],            // disk round is BIOS
            &["bios", "t.tsv", "--out", "o", "--time-limit", "0"][..],
            &["bios", "t.tsv", "--out", "o", "--bogus"][..],
        ] {
            assert!(parse_args(&s(bad), cwd).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn scratch_is_never_the_shared_tmp() {
        assert!(scratch_dir(None, None).is_err());
        assert!(scratch_dir(Some(Path::new("/tmp")), None).is_err());
        assert!(scratch_dir(Some(Path::new("/private/tmp/")), None).is_err());
        assert_eq!(scratch_dir(None, Some(Path::new("/job"))).unwrap(), Path::new("/job/tmp"));
        assert_eq!(scratch_dir(Some(Path::new("/private/tmp/claude-501/s")), Some(Path::new("/job"))).unwrap(), Path::new("/private/tmp/claude-501/s"));
    }

    #[test]
    fn autorun_inf_is_the_per_run_scripts_inf() {
        let z = fs::read(repo().join("measurements/measurement-2026-10-28-bios-storage-audit/method/z-dumpbios.inf")).unwrap();
        assert_eq!(autorun_inf('C', "DUMPBIOS.NAP").as_bytes(), &z[..]);
        let d = fs::read(repo().join("measurements/measurement-2026-10-28-tool07/method/newdesk-d.inf")).unwrap();
        assert_eq!(autorun_inf('D', "DUMPBIOS.NAP").as_bytes(), &d[..]);
        assert!(inf_autoruns(&d, 'D', "DUMPBIOS.NAP"));
        assert!(inf_autoruns(b"#Z 01 d:\\dumpbios.nap@\n", 'D', "DUMPBIOS.NAP"));
        assert!(!inf_autoruns(&d, 'C', "DUMPBIOS.NAP"));
        assert!(!inf_autoruns(&z, 'D', "DUMPBIOS.NAP"));
        assert!(!inf_autoruns(b"#R 02\r\n", 'D', "DUMPBIOS.NAP"));
    }

    #[test]
    fn mbr_entries() {
        let mut m = vec![0u8; 512];
        m[510] = 0x55;
        m[511] = 0xAA;
        assert!(mbr_partitions(&m).unwrap().iter().all(Option::is_none));
        m[446 + 4] = 6;
        m[446 + 8..446 + 12].copy_from_slice(&63u32.to_le_bytes());
        m[446 + 12..446 + 16].copy_from_slice(&130977u32.to_le_bytes());
        m[462 + 4] = 6;
        m[462 + 8..462 + 12].copy_from_slice(&131072u32.to_le_bytes());
        m[462 + 12..462 + 16].copy_from_slice(&131072u32.to_le_bytes());
        let p = mbr_partitions(&m).unwrap();
        assert_eq!(p[0], Some(Part { start: 63, count: 130977 }));
        assert_eq!(p[1], Some(Part { start: 131072, count: 131072 }));
        assert_eq!(p[2], None);
        m[511] = 0;
        assert!(mbr_partitions(&m).is_err());
    }

    const OK_RUN: &str = "\
[shell] sh_ldapp: starting C:\\DUMPBIOS.NAP, sh_nextapp=2, sh_isgem=1
[launch] EL0: slot 0 entry 0x1a08000
[shell] sh_ldapp: 'C:\\DUMPBIOS.NAP' exited with rc=0
[shell] sh_ldapp: starting desktop NEWDESK (D.s_cmd NEWDESK), sh_nextapp=3, sh_isgem=1
[desk] desktop up (1 drives)
";

    #[test]
    fn serial_states_of_a_normal_run() {
        let cut = |marker: &str| &OK_RUN[..OK_RUN.find(marker).unwrap()];
        assert_eq!(scan_serial(cut("[shell] sh_ldapp: starting C"), "DUMPBIOS.NAP"), RunState::Booting);
        assert_eq!(scan_serial(cut("[launch]"), "DUMPBIOS.NAP"), RunState::Running);
        assert_eq!(scan_serial(cut("[desk]"), "DUMPBIOS.NAP"), RunState::Running);
        assert_eq!(scan_serial(OK_RUN, "DUMPBIOS.NAP"), RunState::Done { rc: Some(0) });
        let d = OK_RUN.replace("C:\\DUMPBIOS", "D:\\DUMPBIOS").replace("rc=0", "rc=-1");
        assert_eq!(scan_serial(&d, "DUMPBIOS.NAP"), RunState::Done { rc: Some(-1) });
        // Another program than the dumper does not count.
        assert_eq!(scan_serial(OK_RUN, "DUMPVDI.NAP"), RunState::NoDumper);
        assert_eq!(scan_serial("[desk] desktop up (1 drives)\n", "DUMPBIOS.NAP"), RunState::NoDumper);
    }

    #[test]
    fn serial_markers_of_an_abnormal_end() {
        let p = "[shell] sh_ldapp: starting C:\\DUMPBIOS.NAP, x\n\n[PANIC] index out of bounds\n  at kernel/src/x.rs:1:2\n";
        assert_eq!(
            scan_serial(p, "DUMPBIOS.NAP"),
            RunState::Abnormal("kernel panic: [PANIC] index out of bounds | at kernel/src/x.rs:1:2".into())
        );
        let e = "\n[EXC] unhandled exception (SError): ESR=0x0\n[EXC] FAR=0x0\n";
        assert!(matches!(scan_serial(e, "DUMPBIOS.NAP"), RunState::Abnormal(m) if m.starts_with("unhandled exception: [EXC]")));
        // A panic after the desktop is up is not reached: the run ended there.
        assert_eq!(scan_serial(&format!("{OK_RUN}[PANIC] late\n"), "DUMPBIOS.NAP"), RunState::Done { rc: Some(0) });
    }

    /// The recorded serial log of an expected program crash (XBIOS character
    /// device audit, XCIOFL): the crash screen waits for a key, which ended
    /// that run only at the script's time limit.
    #[test]
    fn recorded_crash_screen_ends_the_run() {
        let log = fs::read_to_string(
            repo().join("measurements/measurement-2026-10-28-xbios-chardev-audit/runs/XCIOFL-NEWTOS-raspi4b/BOOT-SERIAL.txt"),
        )
        .unwrap();
        assert_eq!(scan_serial(&log, "XCIOFL.NAP"), RunState::Abnormal(
            "program crash screen (waits for a key): [crash] slot 0 Bus Error esr=0x9200004f pc=0x1a080dc far=Some(24) | Panic: Bus Error | esr=9200004f".into()
        ));
    }

    /// The recorded serial log of a complete DUMPBIOS run (TOOL08, bios-r1).
    #[test]
    fn recorded_normal_run_is_done() {
        let log = fs::read_to_string(repo().join("measurements/measurement-2026-10-28-tool08/runs/r1/BOOT-SERIAL.txt")).unwrap();
        assert_eq!(scan_serial(&log, "DUMPBIOS.NAP"), RunState::Done { rc: Some(0) });
    }

    /// A FAT16 partition of `sectors` sectors: BPB bps 512, 1 reserved
    /// sector, 2 FATs of 1 sector, 16 root entries; one root entry "A".
    fn fat16(sectors: usize) -> Vec<u8> {
        let mut d = vec![0u8; sectors * 512];
        d[11..13].copy_from_slice(&512u16.to_le_bytes());
        d[14..16].copy_from_slice(&1u16.to_le_bytes());
        d[16] = 2;
        d[17..19].copy_from_slice(&16u16.to_le_bytes());
        d[22..24].copy_from_slice(&1u16.to_le_bytes());
        let root = 3 * 512;
        d[root..root + 11].copy_from_slice(b"A          ");
        d
    }

    #[test]
    fn test_partition_comparison() {
        let a = fat16(8);
        assert_eq!(compare_partitions(&a, &a), PartCompare { identical: true, differing_sectors: vec![], identical_after_normalising: None });
        // A changed access date (byte 18) and write time (22) of the root entry.
        let mut b = a.clone();
        b[3 * 512 + 18] = 0x55;
        b[3 * 512 + 22] = 0x66;
        let c = compare_partitions(&a, &b);
        assert_eq!(c.differing_sectors, vec![3]);
        assert_eq!(c.identical_after_normalising, Some(true));
        // The name byte or a data sector stays a difference.
        let mut n = b.clone();
        n[3 * 512 + 1] = b'B';
        n[7 * 512] = 1;
        let c = compare_partitions(&a, &n);
        assert_eq!(c.differing_sectors, vec![3, 7]);
        assert_eq!(c.identical_after_normalising, Some(false));
        // The entry after the end marker is not normalised.
        let mut e = a.clone();
        e[3 * 512 + 64 + 22] = 9;
        assert_eq!(compare_partitions(&a, &e).identical_after_normalising, Some(false));
    }

    #[test]
    fn dep_info_of_a_copied_target_is_foreign() {
        let t = vec![PathBuf::from("/wt/target"), PathBuf::from("/Volumes/D/wt/target")];
        assert_eq!(foreign_dep_info(&t, "/wt/target/debug/xtask: /wt/tools/xtask/src/main.rs\n"), None);
        assert_eq!(foreign_dep_info(&t, "/Volumes/D/wt/target/x/release/kernel: /a.rs"), None);
        assert_eq!(
            foreign_dep_info(&t, "/main/target/debug/xtask: /main/tools/xtask/src/main.rs"),
            Some("/main/target/debug/xtask".into())
        );
        // A sibling whose name only starts like the tree is foreign too.
        assert!(foreign_dep_info(&t, "/wt/target2/debug/x: /a").is_some());
        let sp = vec![PathBuf::from("/my tree/target")];
        assert_eq!(foreign_dep_info(&sp, "/my\\ tree/target/debug/x: /my\\ tree/a.rs"), None);
        assert_eq!(foreign_dep_info(&t, ""), None);
        assert_eq!(foreign_dep_info(&t, "/wt/programs/../target/programs/a/release/libx.a: /wt/programs/x.rs"), None);
        assert!(foreign_dep_info(&t, "/wt/../main/target/debug/x: /a").is_some());
    }

    #[test]
    fn outer_cargo_variables_are_scrubbed() {
        let keys = ["CARGO", "CARGO_HOME", "CARGO_MANIFEST_DIR", "CARGO_MANIFEST_PATH", "CARGO_PKG_NAME", "RUSTUP_TOOLCHAIN", "RUSTUP_HOME", "CARGO_TARGET_DIR", "PATH", "CARGO_PRIMARY_PACKAGE"];
        assert_eq!(
            scrubbed_env(keys.into_iter()),
            s(&["CARGO", "CARGO_MANIFEST_DIR", "CARGO_MANIFEST_PATH", "CARGO_PKG_NAME", "RUSTUP_TOOLCHAIN", "CARGO_TARGET_DIR", "CARGO_PRIMARY_PACKAGE"])
        );
    }

    #[test]
    fn tosdiff_summary_line() {
        let t = fs::read_to_string(repo().join("measurements/measurement-2026-10-28-tool08/runs/r1/tosdiff-newtos-vs-emutos.txt")).unwrap();
        assert_eq!(tosdiff_summary(&t), Some("42 of 147 cases differ"));
        assert_eq!(tosdiff_summary("tosdiff: refused\n"), None);
    }

    #[test]
    fn lexical_normalisation() {
        assert_eq!(absolute(Path::new("/a/b"), Path::new("../c/./d")), Path::new("/a/c/d"));
        assert_eq!(absolute(Path::new("/a"), Path::new("/x/../y")), Path::new("/y"));
    }
}
