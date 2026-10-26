//! `xtask doc-check` — every path and `file:line` cite and every ADR id in
//! the living docs (`CLAUDE.md` and every Markdown file under `docs/` except
//! `docs/archive/`) must resolve.
//!
//! # Why
//!
//! The living docs cite source by path and line (`tos-api/src/aes/mod.rs:120`,
//! `aes/gemaplib.c:209`). Code moves (the desktop went from `kernel/src/desk/`
//! to `programs/desktop/`), files shrink, and the cites rot silently. This
//! check is the gate that keeps them resolvable.
//!
//! # What is a cite
//!
//! Outside fenced (```` ``` ````) code blocks only:
//!
//! 1. a backtick span that looks like a path: it contains `/` or carries a
//!    line suffix (`:N`, `:N-M`, `:N–M`, `:N,M`, `:N-M,K-L`), and it has a file
//!    extension (one ASCII letter at least, so `1.04/1.06` is not one) or ends
//!    in `/` (a directory). A `#fragment` is stripped.
//! 2. a Markdown link target `[text](target)` that is not a URL and not a pure
//!    `#anchor`; its `#fragment` is stripped.
//!
//! Skipped silently: URLs; spans with whitespace or any of `< > * ? { } … $`;
//! host paths (`/…`, `~…`); TOS paths (`\`, `C:…`); bare names without `/`
//! and without a line suffix; a `:` that is not a valid line suffix
//! (`foo.c:12ff`, Rust paths).
//!
//! # Resolution
//!
//! A link resolves relative to the citing file only — that is where a
//! Markdown renderer looks. A backtick span whose first component is a
//! source-root prefix of [`PREFIXES`] (`ET/bdos/proc.c:541`) resolves under
//! that prefix's root only. Any other span tries, first hit wins: the citing
//! file's directory, the workspace root, then [`SEARCH_ROOTS`]. A bare
//! `name.ext:N` resolves by basename over `git ls-files`, the EmuTOS tree at
//! the pin and the files under `references/freemint`: one match resolves,
//! several are "ambiguous". A hit must match the name's case (macOS would
//! accept `status.md`, Linux and GitHub would not). The highest cited line
//! must not exceed the line count.
//!
//! # EmuTOS at the pin
//!
//! Everything under [`EMUTOS_ROOT`] resolves against the pinned commit named
//! by [`EMUTOS_PIN_VAR`] in [`ORACLE_ENV`] (`docs/analysis/ORACLE.md` §1),
//! not against the checkout's working tree: existence via `git ls-tree` and
//! line counts via `git show <pin>:<path>`. The checkout may sit on another
//! commit; the cites describe the pin. A path the pin lacks may be a product
//! of the pinned build (`desk/desk_rsc.c`): it resolves in
//! [`EMUTOS_BUILD_ROOT`]`/<pin[..8]>/` on disk, and a bare name may match
//! such a product. Without that build tree such a cite is skipped.
//!
//! # `references/` may be absent
//!
//! `references/` is gitignored (`references/README.md`). A cite that does not
//! resolve and could have resolved under a missing reference tree is counted
//! as skipped, not failed: a cite under `references/emutos/…` itself, or a
//! path whose first component is no top-level entry of the workspace
//! (`aes/gemaplib.c`), or a bare name. The EmuTOS tree counts as missing when
//! the checkout is absent or does not hold the pinned commit. A prefixed cite
//! whose root is a gitignored local tree ([`Prefix::local`]) that is absent is
//! skipped too.
//!
//! # ADR ids
//!
//! Outside fenced code blocks (inline code included), `ADR-` followed by
//! exactly three digits (`ADR-028`, `ADR-022s`, `ADR-023-Amendment`) cites
//! ADR 028, 022, 023. Placeholders (`ADR-NNN`, `ADR-0xx`) and named ADRs
//! (`ADR-Supexec`, `ADR-Line-A-01`) are no cites. A cite resolves if
//! [`MEMORY_FILE`] has a heading `## ADR-028:` outside a fenced block.
//!
//! # Default set
//!
//! `CLAUDE.md` plus every `*.md` under `docs/`, recursively, except
//! [`ARCHIVE_DIR`] and dated snapshots (`*-YYYY-MM-DD.md`,
//! [`is_dated_snapshot`]): both are history, never evidence. A file named on
//! the command line is always checked, dated or archived.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

/// Single files checked when none are given, besides the Markdown files
/// under [`DOCS_DIR`]; relative to the workspace root.
pub const DEFAULT_FILES: &[&str] = &["CLAUDE.md"];
/// Walked recursively for `*.md` when no files are given.
pub const DOCS_DIR: &str = "docs";
/// Excluded from the walk: history, never evidence.
pub const ARCHIVE_DIR: &str = "docs/archive";
/// The file whose `## ADR-NNN:` headings define which ADRs exist.
pub const MEMORY_FILE: &str = "docs/MEMORY.md";
/// The gitignored reference trees, relative to the workspace root.
pub const REFERENCE_ROOTS: &[&str] = &["references/emutos", "references/freemint"];
/// Where a backtick span is looked up after the citing file's directory and
/// the workspace root, in order, relative to the workspace root.
pub const SEARCH_ROOTS: &[&str] =
    &["docs", "references/emutos", "references/freemint", "tos-api/src", "kernel/src"];
/// The EmuTOS checkout, resolved at the pin, relative to the workspace root.
pub const EMUTOS_ROOT: &str = "references/emutos";
/// Where the pinned EmuTOS build lives: `<this>/<first 8 digits of the pin>`
/// (`docs/analysis/ORACLE.md` §3, `tools/oracle/build-emutos.sh`), a clone at
/// the pin plus the build's products (`desk/desk_rsc.c`, `include/i18nconf.h`).
pub const EMUTOS_BUILD_ROOT: &str = "references/emutos-oracle";
/// The file that names the oracle's pinned inputs.
pub const ORACLE_ENV: &str = "tools/oracle/oracle.env";
/// The [`ORACLE_ENV`] variable holding the pinned EmuTOS commit.
pub const EMUTOS_PIN_VAR: &str = "ORACLE_EMUTOS_COMMIT";

/// A short source-root prefix the docs use in cites (`ET/bdos/proc.c:541`).
#[derive(Debug, Clone, Copy)]
pub struct Prefix {
    /// The first path component as written, without `/`.
    pub name: &'static str,
    /// The root it stands for, workspace-relative, without trailing `/`.
    pub root: &'static str,
    /// The root is a gitignored local tree that may be absent.
    pub local: bool,
    /// Where the docs define it: `file:line` of each legend, and the
    /// expansion as that legend writes it. Read only by the unit test that
    /// checks each legend still says so.
    #[cfg_attr(not(test), allow(dead_code))]
    pub legends: &'static [(&'static str, &'static str)],
}

/// The source-root prefixes, each with the legends in `docs/` that define it.
pub const PREFIXES: &[Prefix] = &[
    // EmuTOS.
    Prefix {
        name: "ET",
        root: "references/emutos",
        local: true,
        legends: &[
            ("docs/analysis/arch03-implementation-plan.md:9", "references/emutos"),
            ("docs/analysis/arch03-freemint-memory-model.md:9", "references/emutos"),
            ("docs/analysis/ldg-after-arch03-2026-10-06.md:10", "references/emutos"),
            ("docs/analysis/ldg-module-malloc-2026-10-06.md:7", "references/emutos/"),
            ("docs/analysis/vfork-malloc-ownership-2026-10-06.md:5", "references/emutos"),
        ],
    },
    // FreeMiNT kernel. The dated snapshot
    // `docs/analysis/ldg-module-malloc-2026-10-06.md:7` writes `FM/` =
    // `references/freemint/` (its cites read `FM/sys/k_exec.c`); it is
    // history and does not decide the mapping, so an explicit check of that
    // file reports its `FM/` cites as missing.
    Prefix {
        name: "FM",
        root: "references/freemint/sys",
        local: true,
        legends: &[
            ("docs/analysis/arch03-implementation-plan.md:9", "references/freemint/sys"),
            ("docs/analysis/arch03-freemint-memory-model.md:9", "references/freemint/sys"),
            ("docs/analysis/ldg-after-arch03-2026-10-06.md:10", "references/freemint/sys"),
            ("docs/analysis/vfork-malloc-ownership-2026-10-06.md:5", "references/freemint/sys"),
        ],
    },
    // NewTOS process layer.
    Prefix {
        name: "NT",
        root: "tos-api/src/bdos/process",
        local: false,
        legends: &[
            ("docs/analysis/arch03-implementation-plan.md:9", "tos-api/src/bdos/process"),
            ("docs/analysis/arch03-freemint-memory-model.md:9", "tos-api/src/bdos/process"),
            ("docs/analysis/ldg-after-arch03-2026-10-06.md:10", "tos-api/src/bdos/process"),
            ("docs/analysis/zview-ldg-2026-10-04.md:7", "tos-api/src/bdos/process"),
        ],
    },
    // NewTOS kernel, AArch64 part.
    Prefix {
        name: "KA",
        root: "kernel/src/arch/aarch64",
        local: false,
        legends: &[
            ("docs/analysis/arch03-implementation-plan.md:9", "kernel/src/arch/aarch64"),
            ("docs/analysis/arch03-freemint-memory-model.md:9", "kernel/src/arch/aarch64"),
        ],
    },
    // FreeMiNT XaAES kernel module.
    Prefix {
        name: "XA",
        root: "references/freemint/xaaes/src.km",
        local: true,
        legends: &[("docs/analysis/arch03-implementation-plan.md:9", "references/freemint/xaaes/src.km")],
    },
    // LDG sources (gitignored, `/Applications/`).
    Prefix {
        name: "LDG",
        root: "Applications/ldg-src/ldg",
        local: true,
        legends: &[
            ("docs/analysis/ldg-after-arch03-2026-10-06.md:10", "Applications/ldg-src/ldg"),
            ("docs/analysis/ldg-module-malloc-2026-10-06.md:7", "Applications/ldg-src/ldg/"),
            ("docs/analysis/zview-ldg-2026-10-04.md:7", "Applications/ldg-src/ldg"),
        ],
    },
    // LDG developer kit; the legend writes it as `LDG/src/devel`.
    Prefix {
        name: "DEV",
        root: "Applications/ldg-src/ldg/src/devel",
        local: true,
        legends: &[("docs/analysis/ldg-after-arch03-2026-10-06.md:10", "LDG/src/devel")],
    },
    // zView sources (gitignored, `/Applications/`).
    Prefix {
        name: "ZV",
        root: "Applications/z-tools/zview",
        local: true,
        legends: &[
            ("docs/analysis/ldg-after-arch03-2026-10-06.md:10", "Applications/z-tools/zview"),
            ("docs/analysis/zview-ldg-2026-10-04.md:7", "Applications/z-tools/zview"),
        ],
    },
    // mintlib sources (gitignored, `sdk/.gitignore`).
    Prefix {
        name: "ML",
        root: "sdk/libc/mintlib",
        local: true,
        legends: &[("docs/analysis/ldg-module-malloc-2026-10-06.md:7", "sdk/libc/mintlib/")],
    },
    // libshare sources.
    Prefix {
        name: "LS",
        root: "references/libshare/orig-0.1.1/libshare-0.1.1",
        local: true,
        legends: &[(
            "docs/analysis/ldg-module-malloc-2026-10-06.md:7",
            "references/libshare/orig-0.1.1/libshare-0.1.1/",
        )],
    },
    // One measurement run.
    Prefix {
        name: "RUN",
        root: "measurements/measurement-2026-10-06-ldg-b2",
        local: false,
        legends: &[(
            "docs/analysis/ldg-module-malloc-2026-10-06.md:7",
            "measurements/measurement-2026-10-06-ldg-b2/",
        )],
    },
];

/// The prefix entry for a path's first component, if it is one.
pub fn prefix_of(path: &str) -> Option<&'static Prefix> {
    let (first, _) = path.split_once('/')?;
    PREFIXES.iter().find(|p| p.name == first)
}

/// `path` with its prefix replaced by the prefix's root, if it has one.
pub fn expand_prefix(path: &str) -> Option<String> {
    let p = prefix_of(path)?;
    let rest = &path[p.name.len() + 1..];
    Some(if rest.is_empty() { p.root.to_string() } else { format!("{}/{rest}", p.root) })
}

/// The two forms a cite takes in Markdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CiteKind {
    /// `` `path/to/file.c:12` ``
    Span,
    /// `[text](path/to/file.md)`
    Link,
}

/// One cite found in a document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cite {
    /// 1-based line of the citing document.
    pub line: usize,
    /// The cite as written (span content or link target).
    pub text: String,
    pub kind: CiteKind,
    /// The path part: fragment and line suffix removed.
    pub path: String,
    /// The highest line number of the suffix, if there is one.
    pub max_line: Option<u32>,
}

/// Parses a line suffix (the part after `:`): `N`, `N-M`, `N–M`, `N,M`,
/// `N-M,K-L`. Returns the highest number, or `None` if it is no suffix.
pub fn parse_line_suffix(s: &str) -> Option<u32> {
    if s.is_empty() {
        return None;
    }
    let mut max = 0u32;
    for range in s.split(',') {
        let mut parts = range.splitn(2, ['-', '–']);
        for p in parts.by_ref() {
            if p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            max = max.max(p.parse().ok()?);
        }
    }
    Some(max)
}

/// The last component has an extension with at least one ASCII letter and a
/// non-empty stem (`gemaplib.c`, `vdi_asm.S`; not `1.06`, not `...`).
fn has_extension(path: &str) -> bool {
    let last = path.rsplit('/').next().unwrap_or(path);
    match last.rfind('.') {
        Some(0) | None => false,
        Some(i) => {
            let ext = &last[i + 1..];
            !ext.is_empty()
                && ext.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                && ext.bytes().any(|b| b.is_ascii_alphabetic())
        }
    }
}

/// Host paths, TOS paths and characters that mark a span as prose, a glob or
/// a placeholder rather than a path.
fn is_skipped_text(s: &str) -> bool {
    s.chars().any(|c| c.is_whitespace() || "<>*?{}…$\\".contains(c))
        || s.starts_with('/')
        || s.starts_with('~')
        || is_url(s)
        || {
            let b = s.as_bytes();
            b.len() >= 2 && b[0].is_ascii_uppercase() && b[1] == b':'
        }
}

fn is_url(s: &str) -> bool {
    let l = s.to_ascii_lowercase();
    l.starts_with("http:") || l.starts_with("https:") || l.starts_with("mailto:") || l.contains("://")
}

/// Classifies a backtick span's content. `None` = not a cite.
pub fn span_cite(content: &str) -> Option<(String, Option<u32>)> {
    let s = content.trim();
    if s.is_empty() || is_skipped_text(s) {
        return None;
    }
    let s = s.split('#').next().unwrap_or(s);
    let (path, max_line) = match s.rfind(':') {
        Some(i) => (&s[..i], Some(parse_line_suffix(&s[i + 1..])?)),
        None => (s, None),
    };
    if path.is_empty() || path.contains(':') {
        return None;
    }
    if !path.contains('/') && max_line.is_none() {
        return None;
    }
    if !(path.ends_with('/') || has_extension(path)) {
        return None;
    }
    Some((path.to_string(), max_line))
}

/// Classifies a link target. `None` = not a cite.
pub fn link_cite(target: &str) -> Option<String> {
    let t = target.trim();
    if t.is_empty() || t.starts_with('#') || is_url(t) {
        return None;
    }
    let path = t.split('#').next().unwrap_or(t);
    if path.is_empty() || is_skipped_text(path) {
        return None;
    }
    Some(path.to_string())
}

/// The backtick spans of one line: (byte start, byte end incl. delimiters,
/// content). A run of n backticks closes at the next run of exactly n; an
/// unclosed run is literal.
fn code_spans(line: &str) -> Vec<(usize, usize, &str)> {
    let b = line.as_bytes();
    let run_at = |i: usize| b[i..].iter().take_while(|&&c| c == b'`').count();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] != b'`' {
            i += 1;
            continue;
        }
        let n = run_at(i);
        let mut j = i + n;
        let mut close = None;
        while j < b.len() {
            if b[j] == b'`' {
                let m = run_at(j);
                if m == n {
                    close = Some(j);
                    break;
                }
                j += m;
            } else {
                j += 1;
            }
        }
        match close {
            Some(j) => {
                out.push((i, j + n, &line[i + n..j]));
                i = j + n;
            }
            None => i += n,
        }
    }
    out
}

/// The link targets of one line (code spans already blanked out).
fn link_targets(line: &str) -> Vec<&str> {
    let b = line.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(off) = line[i..].find("](") {
        let start = i + off + 2;
        let mut depth = 1;
        let mut j = start;
        while j < b.len() {
            match b[j] {
                b'(' => depth += 1,
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            j += 1;
        }
        if j >= b.len() {
            break;
        }
        let inner = line[start..j].trim();
        let target = if let Some(rest) = inner.strip_prefix('<') {
            rest.split('>').next().unwrap_or("")
        } else {
            // `(target "title")`
            inner.split(char::is_whitespace).next().unwrap_or("")
        };
        out.push(target);
        i = j + 1;
    }
    out
}

/// Every cite of a document, in order.
pub fn scan(text: &str) -> Vec<Cite> {
    let mut out = Vec::new();
    let mut in_fence = false;
    for (idx, line) in text.lines().enumerate() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        let spans = code_spans(line);
        let mut blanked = line.to_string();
        for &(s, e, content) in &spans {
            if let Some((path, max_line)) = span_cite(content) {
                out.push(Cite {
                    line: idx + 1,
                    text: content.trim().to_string(),
                    kind: CiteKind::Span,
                    path,
                    max_line,
                });
            }
            blanked.replace_range(s..e, &" ".repeat(e - s));
        }
        for target in link_targets(&blanked) {
            if let Some(path) = link_cite(target) {
                out.push(Cite {
                    line: idx + 1,
                    text: target.to_string(),
                    kind: CiteKind::Link,
                    path,
                    max_line: None,
                });
            }
        }
    }
    out
}

/// `ADR-` followed by exactly three digits at byte `i` of `b`, preceded by no
/// ASCII alphanumeric: the number, else `None`.
fn adr_id_at(b: &[u8], i: usize) -> Option<u16> {
    if !b[i..].starts_with(b"ADR-") || (i > 0 && b[i - 1].is_ascii_alphanumeric()) {
        return None;
    }
    let d = &b[i + 4..];
    if d.len() < 3 || !d[..3].iter().all(u8::is_ascii_digit) || d.get(3).is_some_and(u8::is_ascii_digit) {
        return None;
    }
    Some(d[..3].iter().fold(0u16, |n, &c| n * 10 + u16::from(c - b'0')))
}

/// Every ADR id cited outside fenced code blocks: (1-based line, number).
pub fn scan_adr_ids(text: &str) -> Vec<(usize, u16)> {
    let mut out = Vec::new();
    let mut in_fence = false;
    for (idx, line) in text.lines().enumerate() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        let b = line.as_bytes();
        let mut from = 0;
        while let Some(off) = line[from..].find("ADR-") {
            let i = from + off;
            if let Some(n) = adr_id_at(b, i) {
                out.push((idx + 1, n));
            }
            from = i + 4;
        }
    }
    out
}

/// The ADRs a memory file defines: its `## ADR-NNN:` headings outside fenced
/// code blocks.
pub fn adr_headings(text: &str) -> BTreeSet<u16> {
    let mut out = BTreeSet::new();
    let mut in_fence = false;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        if let Some(rest) = line.strip_prefix("## ") {
            let b = rest.as_bytes();
            if let Some(n) = adr_id_at(b, 0) {
                if b.get(7) == Some(&b':') {
                    out.insert(n);
                }
            }
        }
    }
    out
}

/// A dated snapshot: the file name ends in `-YYYY-MM-DD.md` with a non-empty
/// stem before it, a month 01–12 and a day 01–31. Only the file name counts;
/// a dated directory (`fsel-reference-2026-09-17/README.md`) does not.
pub fn is_dated_snapshot(rel: &Path) -> bool {
    let Some(name) = rel.file_name().and_then(|n| n.to_str()) else { return false };
    let Some(stem) = name.strip_suffix(".md") else { return false };
    if stem.len() < 12 || !stem.is_char_boundary(stem.len() - 11) {
        return false;
    }
    let (head, date) = stem.split_at(stem.len() - 11);
    let d = date.as_bytes();
    let digits = |r: std::ops::Range<usize>| d[r].iter().all(u8::is_ascii_digit);
    if d[0] != b'-' || d[5] != b'-' || d[8] != b'-' || !digits(1..5) || !digits(6..8) || !digits(9..11) {
        return false;
    }
    let month: u8 = date[6..8].parse().unwrap_or(0);
    let day: u8 = date[9..11].parse().unwrap_or(0);
    !head.is_empty() && (1..=12).contains(&month) && (1..=31).contains(&day)
}

/// A workspace-relative path belongs to the default set's docs walk: a `.md`
/// file under [`DOCS_DIR`], not under [`ARCHIVE_DIR`], not a dated snapshot.
pub fn is_default_doc(rel: &Path) -> bool {
    rel.starts_with(DOCS_DIR)
        && !rel.starts_with(ARCHIVE_DIR)
        && rel.extension().is_some_and(|e| e == "md")
        && !is_dated_snapshot(rel)
}

/// The files checked when none are given: [`DEFAULT_FILES`], then the
/// Markdown files under [`DOCS_DIR`] (sorted), archive and dated snapshots
/// excluded.
pub fn default_files(root: &Path) -> Vec<PathBuf> {
    let mut docs = Vec::new();
    walk(root, Path::new(DOCS_DIR), &mut docs);
    docs.retain(|p| is_default_doc(p));
    docs.sort();
    DEFAULT_FILES.iter().map(PathBuf::from).chain(docs).collect()
}

/// Outcome of resolving one cite.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Ok,
    Skipped,
    Fail(String),
}

/// Lexical normalisation (`a/./b/../c` → `a/c`); `None` if it climbs above
/// the start.
fn normalize(p: &Path) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    return None;
                }
            }
            other => out.push(other),
        }
    }
    Some(out)
}

/// The value of `NAME=value` in a file of plain `NAME=value` lines with `#`
/// comments ([`ORACLE_ENV`]); `None` if absent or empty.
pub fn env_value(text: &str, var: &str) -> Option<String> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.starts_with('#'))
        .find_map(|l| l.strip_prefix(var)?.strip_prefix('='))
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// A git checkout's tree at one commit: its files and directories (relative
/// to the checkout) and a per-file line-count cache.
struct PinTree {
    repo: PathBuf,
    commit: String,
    files: BTreeSet<String>,
    dirs: BTreeSet<String>,
    lines: HashMap<String, usize>,
}

impl PinTree {
    /// `None` if `repo` is no git checkout of its own or lacks `commit`.
    fn load(repo: &Path, commit: &str) -> Option<PinTree> {
        if !repo.join(".git").exists() {
            return None;
        }
        let out = Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(["ls-tree", "-r", "-z", "--name-only", commit])
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        let files: BTreeSet<String> = out
            .stdout
            .split(|&b| b == 0)
            .filter(|s| !s.is_empty())
            .map(|s| String::from_utf8_lossy(s).into_owned())
            .collect();
        let mut dirs = BTreeSet::new();
        for f in &files {
            let mut cur = f.as_str();
            while let Some(i) = cur.rfind('/') {
                cur = &cur[..i];
                if !dirs.insert(cur.to_string()) {
                    break;
                }
            }
        }
        Some(PinTree { repo: repo.to_path_buf(), commit: commit.to_string(), files, dirs, lines: HashMap::new() })
    }

    /// `Some(is_dir)` if `rel` exists with exactly this case; `""` is the root.
    fn kind(&self, rel: &str) -> Option<bool> {
        if rel.is_empty() || self.dirs.contains(rel) {
            Some(true)
        } else if self.files.contains(rel) {
            Some(false)
        } else {
            None
        }
    }

    fn has_case_variant(&self, rel: &str) -> bool {
        self.files.iter().chain(&self.dirs).any(|f| f.eq_ignore_ascii_case(rel))
    }

    fn line_count(&mut self, rel: &str) -> Result<usize, String> {
        if let Some(&n) = self.lines.get(rel) {
            return Ok(n);
        }
        let out = Command::new("git")
            .arg("-C")
            .arg(&self.repo)
            .arg("show")
            .arg(format!("{}:{rel}", self.commit))
            .output()
            .map_err(|e| format!("doc-check: git show {}:{rel}: {e}", self.commit))?;
        if !out.status.success() {
            return Err(format!("doc-check: git show {}:{rel} failed", self.commit));
        }
        let n = count_lines(&out.stdout);
        self.lines.insert(rel.to_string(), n);
        Ok(n)
    }
}

/// `rel` relative to [`EMUTOS_ROOT`], if it lies under it.
fn under_emutos(rel: &Path) -> Option<String> {
    rel.strip_prefix(EMUTOS_ROOT).ok().map(|p| p.to_string_lossy().into_owned())
}

/// Resolves cites against one workspace. Caches directory listings (for the
/// exact-case test), the basename index (built on first bare cite) and the
/// EmuTOS tree at the pin (loaded on first resolve).
pub struct Resolver {
    root: PathBuf,
    dir_cache: HashMap<PathBuf, Option<Vec<String>>>,
    basenames: Option<BTreeMap<String, Vec<PathBuf>>>,
    /// The EmuTOS commit; `None` = read it from [`ORACLE_ENV`].
    emutos_commit: Option<String>,
    /// `None` = not loaded yet; `Some(None)` = checkout or commit absent.
    emutos: Option<Option<PinTree>>,
    /// The pinned build tree, workspace-relative ([`EMUTOS_BUILD_ROOT`]);
    /// set by [`Self::load_emutos`].
    emutos_build: PathBuf,
}

impl Resolver {
    pub fn new(root: &Path) -> Self {
        Resolver {
            root: root.to_path_buf(),
            dir_cache: HashMap::new(),
            basenames: None,
            emutos_commit: None,
            emutos: None,
            emutos_build: PathBuf::new(),
        }
    }

    /// Resolves EmuTOS cites at `commit` instead of the [`ORACLE_ENV`] pin.
    #[cfg(test)]
    pub fn with_emutos_commit(root: &Path, commit: &str) -> Self {
        Resolver { emutos_commit: Some(commit.to_string()), ..Resolver::new(root) }
    }

    fn load_emutos(&mut self) -> Result<(), String> {
        if self.emutos.is_some() {
            return Ok(());
        }
        let commit = match &self.emutos_commit {
            Some(c) => c.clone(),
            None => {
                let env = fs::read_to_string(self.root.join(ORACLE_ENV))
                    .map_err(|e| format!("doc-check: {ORACLE_ENV}: {e}"))?;
                env_value(&env, EMUTOS_PIN_VAR).ok_or_else(|| format!("doc-check: {ORACLE_ENV}: no {EMUTOS_PIN_VAR}"))?
            }
        };
        self.emutos = Some(PinTree::load(&self.root.join(EMUTOS_ROOT), &commit));
        self.emutos_build = Path::new(EMUTOS_BUILD_ROOT).join(commit.get(..8).unwrap_or(&commit));
        Ok(())
    }

    /// The EmuTOS tree at the pin; call [`Self::load_emutos`] first.
    fn pin_tree(&self) -> Option<&PinTree> {
        self.emutos.as_ref().and_then(Option::as_ref)
    }

    fn listing(&mut self, dir: &Path) -> Option<&Vec<String>> {
        self.dir_cache
            .entry(dir.to_path_buf())
            .or_insert_with(|| {
                fs::read_dir(dir).ok().map(|rd| {
                    rd.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().into_owned()).collect()
                })
            })
            .as_ref()
    }

    /// `rel` exists under `base` with exactly this case. `rel` must be
    /// normalised (no `..`).
    fn exists_exact(&mut self, base: &Path, rel: &Path) -> bool {
        let mut cur = base.to_path_buf();
        for c in rel.components() {
            let name = c.as_os_str().to_string_lossy().into_owned();
            match self.listing(&cur) {
                Some(l) if l.contains(&name) => cur.push(&name),
                _ => return false,
            }
        }
        true
    }

    /// The pinned build tree is absent on disk.
    fn build_missing(&self) -> bool {
        !self.root.join(&self.emutos_build).is_dir()
    }

    /// Where workspace-relative `rel` exists with exactly this case (and is a
    /// directory if `want_dir`). Under [`EMUTOS_ROOT`]: at the pin, else the
    /// same path in the pinned build tree (a build product); elsewhere on
    /// disk. The result is workspace-relative.
    fn locate(&mut self, rel: &Path, want_dir: bool) -> Option<PathBuf> {
        let root = self.root.clone();
        let rel = match under_emutos(rel) {
            Some(sub) => {
                if self.pin_tree().and_then(|t| t.kind(&sub)).is_some_and(|d| d || !want_dir) {
                    return Some(rel.to_path_buf());
                }
                self.emutos_build.join(sub)
            }
            None => rel.to_path_buf(),
        };
        (self.exists_exact(&root, &rel) && (!want_dir || root.join(&rel).is_dir())).then_some(rel)
    }

    fn is_dir(&self, rel: &Path) -> bool {
        match under_emutos(rel) {
            Some(sub) => self.pin_tree().and_then(|t| t.kind(&sub)) == Some(true),
            None => self.root.join(rel).is_dir(),
        }
    }

    fn lines(&mut self, rel: &Path) -> Result<usize, String> {
        match under_emutos(rel) {
            Some(sub) => match self.emutos.as_mut().and_then(Option::as_mut) {
                Some(t) => t.line_count(&sub),
                None => Err(format!("doc-check: {}: EmuTOS pin not loaded", rel.display())),
            },
            None => {
                let full = self.root.join(rel);
                line_count(&full).map_err(|e| format!("doc-check: {}: {e}", full.display()))
            }
        }
    }

    /// The reference tree `r` (one of [`REFERENCE_ROOTS`]) is unavailable:
    /// EmuTOS without the pinned commit, others absent on disk.
    fn tree_missing(&self, r: &str) -> bool {
        if r == EMUTOS_ROOT {
            self.pin_tree().is_none()
        } else {
            !self.root.join(r).is_dir()
        }
    }

    fn reference_missing(&self, rel: &str) -> bool {
        REFERENCE_ROOTS.iter().any(|r| rel.starts_with(&format!("{r}/")) && self.tree_missing(r))
    }

    fn any_reference_missing(&self) -> bool {
        REFERENCE_ROOTS.iter().any(|r| self.tree_missing(r))
    }

    /// Workspace-relative candidates for `path` cited from `doc_dir`
    /// (workspace-relative), in lookup order. A prefix-expanded path has
    /// exactly one candidate.
    fn candidates(&self, doc_dir: &Path, path: &str, kind: CiteKind, expanded: bool) -> Vec<PathBuf> {
        if expanded {
            return normalize(Path::new(path)).into_iter().collect();
        }
        let mut v = Vec::new();
        if let Some(p) = normalize(&doc_dir.join(path)) {
            v.push(p);
        }
        if kind == CiteKind::Span {
            for base in std::iter::once("").chain(SEARCH_ROOTS.iter().copied()) {
                if let Some(p) = normalize(&Path::new(base).join(path)) {
                    if !v.contains(&p) {
                        v.push(p);
                    }
                }
            }
        }
        v
    }

    /// Basename → paths over `git ls-files`, the EmuTOS tree at the pin and
    /// the other [`REFERENCE_ROOTS`] on disk. Call [`Self::load_emutos`] first.
    fn basename_index(&mut self) -> Result<&BTreeMap<String, Vec<PathBuf>>, String> {
        if self.basenames.is_none() {
            let out = Command::new("git")
                .arg("ls-files")
                .current_dir(&self.root)
                .output()
                .map_err(|e| format!("doc-check: git ls-files: {e}"))?;
            if !out.status.success() {
                return Err("doc-check: git ls-files failed".into());
            }
            let mut files: Vec<PathBuf> =
                String::from_utf8_lossy(&out.stdout).lines().map(PathBuf::from).collect();
            if let Some(t) = self.pin_tree() {
                files.extend(t.files.iter().map(|f| Path::new(EMUTOS_ROOT).join(f)));
            }
            // The build's products: files of the build tree the pin lacks.
            let mut built = Vec::new();
            walk(&self.root, &self.emutos_build, &mut built);
            let in_pin = |f: &Path| {
                let sub = f.strip_prefix(&self.emutos_build).unwrap_or(f).to_string_lossy().into_owned();
                self.pin_tree().is_some_and(|t| t.files.contains(&sub))
            };
            files.extend(built.into_iter().filter(|f| !in_pin(f)));
            for r in REFERENCE_ROOTS.iter().filter(|&&r| r != EMUTOS_ROOT) {
                walk(&self.root, Path::new(r), &mut files);
            }
            let mut idx: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();
            for f in files {
                if let Some(n) = f.file_name() {
                    idx.entry(n.to_string_lossy().into_owned()).or_default().push(f);
                }
            }
            self.basenames = Some(idx);
        }
        Ok(self.basenames.as_ref().unwrap())
    }

    /// Resolves `cite` found in `doc` (workspace-relative).
    pub fn resolve(&mut self, doc: &Path, cite: &Cite) -> Result<Outcome, String> {
        self.load_emutos()?;
        let want_dir = cite.path.ends_with('/');
        let prefix = if cite.kind == CiteKind::Span { prefix_of(&cite.path) } else { None };
        let full_path = match prefix {
            Some(_) => expand_prefix(&cite.path).unwrap_or_else(|| cite.path.clone()),
            None => cite.path.clone(),
        };
        let path = full_path.trim_end_matches('/');
        let hit = if cite.kind == CiteKind::Span && !cite.path.contains('/') {
            let matches = self.basename_index()?.get(path).cloned().unwrap_or_default();
            match matches.len() {
                0 if self.any_reference_missing() || self.build_missing() => return Ok(Outcome::Skipped),
                0 => return Ok(Outcome::Fail("missing".into())),
                1 => matches[0].clone(),
                n => return Ok(Outcome::Fail(format!("ambiguous ({n} matches)"))),
            }
        } else {
            let doc_dir = doc.parent().unwrap_or(Path::new(""));
            let cands = self.candidates(doc_dir, path, cite.kind, prefix.is_some());
            match cands.iter().find_map(|c| self.locate(c, want_dir)) {
                Some(f) => f,
                None => return Ok(self.unresolved(&cands, path, cite.kind, prefix)),
            }
        };
        if let Some(max) = cite.max_line {
            if self.is_dir(&hit) {
                return Ok(Outcome::Fail("is a directory, has a line suffix".into()));
            }
            let n = self.lines(&hit)?;
            if max as usize > n || max == 0 {
                return Ok(Outcome::Fail(format!("line {max} beyond end ({n} lines) of {}", hit.display())));
            }
        }
        Ok(Outcome::Ok)
    }

    /// Why nothing matched: skipped (a reference or local tree it may live
    /// in is absent), a case mismatch, or missing.
    fn unresolved(&self, cands: &[PathBuf], path: &str, kind: CiteKind, prefix: Option<&Prefix>) -> Outcome {
        let under_missing_ref =
            cands.iter().any(|c| self.reference_missing(&c.to_string_lossy()));
        let local_absent = prefix.is_some_and(|p| p.local && !self.root.join(p.root).is_dir());
        // An EmuTOS cite that may name a build product, without the build.
        let emutos_unbuilt = under_emutos(Path::new(path)).is_some() && self.build_missing();
        let first = path.split('/').next().unwrap_or("");
        let foreign_top = kind == CiteKind::Span
            && first != "."
            && first != ".."
            && !self.root.join(first).exists()
            && (self.any_reference_missing() || self.build_missing());
        if under_missing_ref || local_absent || emutos_unbuilt || foreign_top {
            return Outcome::Skipped;
        }
        let case_variant = |c: &PathBuf| match under_emutos(c) {
            Some(sub) => self.pin_tree().is_some_and(|t| t.has_case_variant(&sub)),
            None => self.root.join(c).exists(),
        };
        if cands.iter().any(case_variant) {
            return Outcome::Fail("missing (only a case-different name exists)".into());
        }
        Outcome::Fail("missing".into())
    }
}

/// Collects the files under `root/rel` (workspace-relative), skipping `.git`.
fn walk(root: &Path, rel: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(root.join(rel)) else { return };
    for e in rd.filter_map(|e| e.ok()) {
        let name = e.file_name();
        if name == ".git" {
            continue;
        }
        let p = rel.join(&name);
        match e.file_type() {
            Ok(t) if t.is_dir() => walk(root, &p, out),
            Ok(_) => out.push(p),
            Err(_) => {}
        }
    }
}

/// Lines of a file: newlines, plus one for a last line without one.
fn line_count(p: &Path) -> std::io::Result<usize> {
    Ok(count_lines(&fs::read(p)?))
}

/// Lines of a byte string: newlines, plus one for a last line without one.
fn count_lines(b: &[u8]) -> usize {
    let nl = b.iter().filter(|&&c| c == b'\n').count();
    nl + usize::from(!b.is_empty() && *b.last().unwrap() != b'\n')
}

/// `cargo xtask doc-check [<file>...]`.
pub fn cmd(root: &Path, args: &[String]) -> Result<(), String> {
    let cwd = std::env::current_dir().map_err(|e| format!("doc-check: {e}"))?;
    let files: Vec<PathBuf> = if args.is_empty() {
        default_files(root)
    } else {
        let mut v = Vec::new();
        for a in args {
            if a.starts_with('-') {
                return Err(format!("doc-check: unknown argument {a:?}"));
            }
            // Given relative to the caller's directory, kept workspace-relative.
            let abs = if Path::new(a).is_absolute() { PathBuf::from(a) } else { cwd.join(a) };
            let abs = fs::canonicalize(&abs).map_err(|e| format!("doc-check: {a}: {e}"))?;
            let canon_root = fs::canonicalize(root).map_err(|e| format!("doc-check: {e}"))?;
            let rel = abs
                .strip_prefix(&canon_root)
                .map_err(|_| format!("doc-check: {a}: not inside the workspace"))?;
            v.push(rel.to_path_buf());
        }
        v
    };

    let memory = fs::read_to_string(root.join(MEMORY_FILE)).map_err(|e| format!("doc-check: {MEMORY_FILE}: {e}"))?;
    let adrs = adr_headings(&memory);
    let mut resolver = Resolver::new(root);
    let (mut checked, mut failed, mut skipped, mut adr_checked) = (0usize, 0usize, 0usize, 0usize);
    for f in &files {
        let text = fs::read_to_string(root.join(f)).map_err(|e| format!("doc-check: {}: {e}", f.display()))?;
        for (line, n) in scan_adr_ids(&text) {
            adr_checked += 1;
            if !adrs.contains(&n) {
                failed += 1;
                println!("{}:{line}: ADR-{n:03}: no such ADR in {MEMORY_FILE}", f.display());
            }
        }
        for cite in scan(&text) {
            checked += 1;
            match resolver.resolve(f, &cite)? {
                Outcome::Ok => {}
                Outcome::Skipped => skipped += 1,
                Outcome::Fail(reason) => {
                    failed += 1;
                    println!("{}:{}: {}: {reason}", f.display(), cite.line, cite.text);
                }
            }
        }
    }
    println!(
        "doc-check: {} files, {checked} path cites and {adr_checked} ADR cites checked, {failed} failures, \
         {skipped} skipped (reference tree missing)",
        files.len()
    );
    if failed == 0 {
        Ok(())
    } else {
        Err(format!("doc-check: {failed} unresolved cites (see above)"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap().to_path_buf()
    }

    fn paths(text: &str) -> Vec<(String, Option<u32>, CiteKind)> {
        scan(text).into_iter().map(|c| (c.path, c.max_line, c.kind)).collect()
    }

    #[test]
    fn line_suffix_forms() {
        assert_eq!(parse_line_suffix("209"), Some(209));
        assert_eq!(parse_line_suffix("120-140"), Some(140));
        assert_eq!(parse_line_suffix("120–140"), Some(140));
        assert_eq!(parse_line_suffix("12,7"), Some(12));
        assert_eq!(parse_line_suffix("3690-3748,4246-4272"), Some(4272));
        assert_eq!(parse_line_suffix("26,671-681,1095-1110"), Some(1110));
        for bad in ["", "12ff", "-3", "3-", "1,,2", "a", "1-2-3"] {
            assert_eq!(parse_line_suffix(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn span_detection() {
        assert_eq!(span_cite("tos-api/src/aes/mod.rs:120-140"), Some(("tos-api/src/aes/mod.rs".into(), Some(140))));
        assert_eq!(span_cite("aes/gemaplib.c:209"), Some(("aes/gemaplib.c".into(), Some(209))));
        assert_eq!(span_cite("screen.c:1030"), Some(("screen.c".into(), Some(1030))));
        assert_eq!(span_cite("programs/desktop/"), Some(("programs/desktop/".into(), None)));
        assert_eq!(span_cite("docs/api/"), Some(("docs/api/".into(), None)));
        assert_eq!(span_cite("vdi/vdi_asm.S:40-49"), Some(("vdi/vdi_asm.S".into(), Some(49))));
        assert_eq!(span_cite("docs/api/gemdos.md#0x01"), Some(("docs/api/gemdos.md".into(), None)));
    }

    #[test]
    fn span_skip_rules() {
        for s in [
            "NEWDESK.INF",
            "cmdline.txt",
            ":1323-1324",
            "https://example.org/a.c",
            "a b/c.rs",
            "kernel/src/<x>.rs",
            "selftest_*.rs",
            "a?/b.c",
            "x/{a,b}.rs",
            "kernel/src/desk/…",
            "$CLAUDE_JOB_DIR/tmp/x.img",
            "/tmp/x.c",
            "~/.claude/CLAUDE.md",
            "C:\\AUTO\\X.NCP",
            "C:/APPS/QED.NAP",
            "kernel/src/desk",
            "1.04/1.06",
            "kernel::desk::x",
            "foo.c:12ff",
            "kernel/src/desk/...",
            "",
        ] {
            assert_eq!(span_cite(s), None, "{s:?}");
        }
    }

    #[test]
    fn link_targets_and_skips() {
        let t = "See [a](api/bios/_cookie-jar.md), [b](../TASK/DESK44.md#x \"t\"), \
                 [u](https://x.org/a.md), [m](mailto:a@b.c), [h](#anchor), [n](<sp ace.md>).";
        assert_eq!(
            paths(t),
            vec![
                ("api/bios/_cookie-jar.md".into(), None, CiteKind::Link),
                ("../TASK/DESK44.md".into(), None, CiteKind::Link),
            ]
        );
        // A link inside a code span is not a link.
        assert!(paths("`[a](b/c.md)`").is_empty());
        // Nested parentheses in a target.
        assert_eq!(paths("[x](a/b(1).md)")[0].0, "a/b(1).md");
    }

    #[test]
    fn spans_per_line_and_fences() {
        let t = "\
x `a/b.rs:3` y ``c/d.rs`` z `unclosed/e.rs
```
`in/fence.rs:1` [l](in/fence.md)
```
`after/f.rs:9-10`
";
        assert_eq!(
            paths(t),
            vec![
                ("a/b.rs".into(), Some(3), CiteKind::Span),
                ("c/d.rs".into(), None, CiteKind::Span),
                ("after/f.rs".into(), Some(10), CiteKind::Span),
            ]
        );
        let lines: Vec<usize> = scan(t).iter().map(|c| c.line).collect();
        assert_eq!(lines, vec![1, 1, 5]);
    }

    #[test]
    fn resolves_against_the_workspace() {
        let mut r = Resolver::new(&root());
        let doc = Path::new("docs/STATUS.md");
        let one = |text: &str| scan(text).into_iter().next().unwrap();
        // Repo root, docs/ and citing-file-relative lookups.
        assert_eq!(r.resolve(doc, &one("`tools/xtask/src/doc_check.rs:1`")).unwrap(), Outcome::Ok);
        assert_eq!(r.resolve(doc, &one("`api/`")).unwrap(), Outcome::Ok);
        assert_eq!(r.resolve(doc, &one("[m](MEMORY.md)")).unwrap(), Outcome::Ok);
        // A link resolves relative to its file only.
        assert!(matches!(r.resolve(doc, &one("[c](tools/xtask/Cargo.toml)")).unwrap(), Outcome::Fail(_)));
        // Beyond the end.
        let long = one("`tools/xtask/src/doc_check.rs:999999`");
        assert!(matches!(r.resolve(doc, &long).unwrap(), Outcome::Fail(m) if m.starts_with("line 999999")));
        // A top-level path that is gone fails even without references/.
        assert!(matches!(
            r.resolve(doc, &one("`kernel/src/no_such_module_x.rs`")).unwrap(),
            Outcome::Fail(m) if m == "missing"
        ));
        // Case must match exactly.
        assert!(matches!(r.resolve(doc, &one("`tools/XTASK/Cargo.toml`")).unwrap(), Outcome::Fail(_)));
        // Bare name: unique and ambiguous (over the tracked files).
        assert_eq!(r.resolve(doc, &one("`emutos_consts.rs:1`")).unwrap(), Outcome::Ok);
        assert!(matches!(r.resolve(doc, &one("`Cargo.toml:1`")).unwrap(), Outcome::Fail(m) if m.starts_with("ambiguous")));
    }

    #[test]
    fn adr_id_scanner() {
        let ids = |t: &str| scan_adr_ids(t).into_iter().map(|(_, n)| n).collect::<Vec<_>>();
        // Digits, also inside inline code and with a suffix that is no digit.
        assert_eq!(ids("ADR-028, `ADR-010` (ADR-022s; ADR-023-Amendment)."), vec![28, 10, 22, 23]);
        // Placeholders and named ADRs are no cites.
        for t in ["ADR-NNN", "ADR-MMM", "ADR-xxx", "ADR-0xx", "ADR-Supexec", "ADR-Line-A-01", "ADR-21s"] {
            assert!(ids(t).is_empty(), "{t:?}");
        }
        // Exactly three digits; `ADR-` must start a word.
        assert!(ids("ADR-0280 XADR-010 ADR-01").is_empty());
        assert_eq!(ids("ADR-001…ADR-009"), vec![1, 9]);
        // Fenced blocks are skipped; line numbers are 1-based.
        let t = "a ADR-010\n```\nADR-011\n```\nb ADR-012\n";
        assert_eq!(scan_adr_ids(t), vec![(1, 10), (5, 12)]);
    }

    #[test]
    fn adr_heading_detection() {
        let t = "\
## ADR-010: One
### ADR-011: level three
## ADR-012 no colon
 ## ADR-013: indented
## ADR-NNN: Title
## ADR-0140: four digits
## ADR-015:
```markdown
## ADR-016: in a fence
```
";
        assert_eq!(adr_headings(t), BTreeSet::from([10, 15]));
        // The real file defines ADR-010 and has no ADR-001 heading.
        let real = adr_headings(&fs::read_to_string(root().join(MEMORY_FILE)).unwrap());
        assert!(real.contains(&10) && !real.contains(&1));
    }

    #[test]
    fn docs_walk_excludes_the_archive() {
        assert!(is_default_doc(Path::new("docs/STATUS.md")));
        assert!(is_default_doc(Path::new("docs/api/bios/_cookie-jar.md")));
        assert!(!is_default_doc(Path::new("docs/archive/memory-2026-10-26.md")));
        assert!(!is_default_doc(Path::new("docs/archive/sub/x.md")));
        assert!(!is_default_doc(Path::new("docs/commit-map.txt")));
        assert!(!is_default_doc(Path::new("docs-old/x.md")));
        assert!(!is_default_doc(Path::new("TASK/A1.md")));
        assert!(is_default_doc(Path::new("docs/archived.md")));
        let files = default_files(&root());
        assert_eq!(files[0], PathBuf::from("CLAUDE.md"));
        for want in ["docs/STATUS.md", "docs/MEMORY.md", "docs/api/bios/_cookie-jar.md"] {
            assert!(files.contains(&PathBuf::from(want)), "{want}");
        }
        assert!(files.iter().all(|f| !f.starts_with(ARCHIVE_DIR)));
        assert!(files[1..].iter().all(|f| is_default_doc(f)));
    }

    #[test]
    fn dated_snapshots_are_left_out() {
        for dated in [
            "docs/analysis/handover-2026-10-21.md",
            "docs/api/desktop/_hw-test-2026-09-17.md",
            "docs/analysis/a-2026-01-31.md",
            "docs/analysis/x-1999-12-01.md",
        ] {
            assert!(is_dated_snapshot(Path::new(dated)), "{dated}");
            assert!(!is_default_doc(Path::new(dated)), "{dated}");
        }
        for undated in [
            "docs/analysis/ORACLE.md",
            "docs/analysis/aes-audit/p4-form.md",
            // A dated directory, an undated file.
            "docs/analysis/fsel-reference-2026-09-17/README.md",
            // No stem, date not last, not a date, not Markdown.
            "docs/analysis/2026-10-21.md",
            "docs/analysis/-2026-10-21.md",
            "docs/analysis/x-2026-10-21-notes.md",
            "docs/analysis/x-2026-13-01.md",
            "docs/analysis/x-2026-00-01.md",
            "docs/analysis/x-2026-10-32.md",
            "docs/analysis/x-2026-10-00.md",
            "docs/analysis/x-26-10-21.md",
            "docs/analysis/x-2026-1-021.md",
            "docs/analysis/x-2026_10_21.md",
            "docs/analysis/x-2026-10-21.txt",
            "docs/analysis/ü-2026-10-2ü.md",
        ] {
            assert!(!is_dated_snapshot(Path::new(undated)), "{undated}");
        }
        assert!(is_default_doc(Path::new("docs/analysis/ORACLE.md")));
        assert!(is_default_doc(Path::new("docs/analysis/fsel-reference-2026-09-17/README.md")));
        let files = default_files(&root());
        assert!(files.contains(&PathBuf::from("docs/analysis/ORACLE.md")));
        assert!(files.iter().all(|f| !is_dated_snapshot(f)));
    }

    #[test]
    fn prefixes_expand_and_resolve() {
        assert_eq!(expand_prefix("ET/bdos/proc.c"), Some("references/emutos/bdos/proc.c".into()));
        assert_eq!(expand_prefix("NT/loader.rs"), Some("tos-api/src/bdos/process/loader.rs".into()));
        assert_eq!(expand_prefix("KA/"), Some("kernel/src/arch/aarch64".into()));
        assert_eq!(expand_prefix("DEV/gcc.mak"), Some("Applications/ldg-src/ldg/src/devel/gcc.mak".into()));
        assert_eq!(expand_prefix("FM/k_exec.c"), Some("references/freemint/sys/k_exec.c".into()));
        // Exact case, first component only, known prefixes only.
        for p in ["et/bdos/proc.c", "x/ET/proc.c", "ET", "ESR/a.c", "ETX/a.c"] {
            assert_eq!(expand_prefix(p), None, "{p:?}");
        }
        // One entry per prefix; each legend still says so.
        let names: BTreeSet<&str> = PREFIXES.iter().map(|p| p.name).collect();
        assert_eq!(names.len(), PREFIXES.len());
        for p in PREFIXES {
            assert!(!p.legends.is_empty(), "{}", p.name);
            for (cite, written) in p.legends {
                let (file, line) = cite.rsplit_once(':').unwrap();
                let text = fs::read_to_string(root().join(file)).unwrap();
                let l = text.lines().nth(line.parse::<usize>().unwrap() - 1).unwrap();
                assert!(
                    l.contains(&format!("`{}`", p.name)) || l.contains(&format!("`{}/`", p.name)),
                    "{cite}: no `{}`",
                    p.name
                );
                assert!(l.contains(&format!("`{written}`")), "{cite}: no `{written}`");
            }
        }
        let mut r = Resolver::new(&root());
        let doc = Path::new("docs/analysis/x.md");
        let one = |text: &str| scan(text).into_iter().next().unwrap();
        assert_eq!(r.resolve(doc, &one("`NT/loader.rs:1`")).unwrap(), Outcome::Ok);
        assert_eq!(r.resolve(doc, &one("`KA/exceptions.rs:1`")).unwrap(), Outcome::Ok);
        assert_eq!(r.resolve(doc, &one("`KA/`")).unwrap(), Outcome::Ok);
        assert!(matches!(
            r.resolve(doc, &one("`NT/loader.rs:999999`")).unwrap(),
            Outcome::Fail(m) if m.ends_with("of tos-api/src/bdos/process/loader.rs")
        ));
        // A prefixed path resolves under its root only, not in the search
        // roots (`kernel/src/arch/aarch64/mod.rs` exists, `NT/mod.rs` no).
        assert!(matches!(r.resolve(doc, &one("`NT/no_such_x.rs`")).unwrap(), Outcome::Fail(m) if m == "missing"));
        // FreeMiNT: resolved, or skipped without `references/freemint`.
        let fm = r.resolve(doc, &one("`FM/k_exec.c:1`")).unwrap();
        assert!(fm == Outcome::Ok || (fm == Outcome::Skipped && !root().join("references/freemint").is_dir()));
        // An unmapped prefix stays a plain path.
        assert!(matches!(r.resolve(doc, &one("`ESR/x.c:1`")).unwrap(), Outcome::Fail(_) | Outcome::Skipped));
    }

    #[test]
    fn bare_names_resolve_when_unique() {
        let mut r = Resolver::new(&root());
        let doc = Path::new("docs/STATUS.md");
        let one = |text: &str| scan(text).into_iter().next().unwrap();
        // Unique over `git ls-files`.
        assert_eq!(r.resolve(doc, &one("`doc_check.rs:1`")).unwrap(), Outcome::Ok);
        assert!(matches!(
            r.resolve(doc, &one("`doc_check.rs:999999`")).unwrap(),
            Outcome::Fail(m) if m.ends_with("of tools/xtask/src/doc_check.rs")
        ));
        // Ambiguous and missing fail (missing is skipped without references/).
        assert!(matches!(r.resolve(doc, &one("`Cargo.toml:1`")).unwrap(), Outcome::Fail(m) if m.starts_with("ambiguous")));
        let gone = r.resolve(doc, &one("`no_such_file_x.c:1`")).unwrap();
        assert!(matches!(gone, Outcome::Fail(ref m) if m == "missing") || gone == Outcome::Skipped);
        // Without a line suffix a bare name is no cite.
        assert!(scan("`doc_check.rs`").is_empty());
    }

    /// Needs `references/emutos` with the pin and the 1.4 calibration commit
    /// (`tools/oracle/oracle.env`); skipped otherwise.
    #[test]
    fn emutos_resolves_at_the_pin() {
        let env = fs::read_to_string(root().join(ORACLE_ENV)).unwrap();
        let pin = env_value(&env, EMUTOS_PIN_VAR).expect("pin in oracle.env");
        let cal = env_value(&env, "ORACLE_CAL_EMUTOS_COMMIT").expect("calibration commit in oracle.env");
        assert_eq!(env_value("# X=1\nX=abc\n", "X"), Some("abc".into()));
        assert_eq!(env_value("XY=1\n", "X"), None);
        let repo = root().join(EMUTOS_ROOT);
        if PinTree::load(&repo, &pin).is_none() || PinTree::load(&repo, &cal).is_none() {
            eprintln!("emutos_resolves_at_the_pin: references/emutos or a commit absent, skipped");
            return;
        }
        let doc = Path::new("docs/STATUS.md");
        let one = |text: &str| scan(text).into_iter().next().unwrap();
        // `util/nlsasm.S` (225 lines) exists in 1.4 and is gone at the pin;
        // `include/string.h` has 73 lines in 1.4 and 103 at the pin.
        let mut at_pin = Resolver::new(&root());
        let mut at_cal = Resolver::with_emutos_commit(&root(), &cal);
        let nls = one("`references/emutos/util/nlsasm.S:1`");
        assert_eq!(at_cal.resolve(doc, &nls).unwrap(), Outcome::Ok);
        assert!(matches!(at_pin.resolve(doc, &nls).unwrap(), Outcome::Fail(m) if m == "missing"));
        let s = one("`ET/include/string.h:103`");
        assert_eq!(at_pin.resolve(doc, &s).unwrap(), Outcome::Ok);
        assert!(matches!(at_cal.resolve(doc, &s).unwrap(), Outcome::Fail(m) if m.contains("(73 lines)")));
        assert!(matches!(at_pin.resolve(doc, &one("`ET/include/string.h:104`")).unwrap(), Outcome::Fail(_)));
        // Search roots, directories, bare names and case at the pin.
        assert_eq!(at_pin.resolve(doc, &one("`bdos/proc.c:1`")).unwrap(), Outcome::Ok);
        assert_eq!(at_pin.resolve(doc, &one("`ET/bdos/`")).unwrap(), Outcome::Ok);
        assert_eq!(at_pin.resolve(doc, &one("`gemaplib.c:1`")).unwrap(), Outcome::Ok);
        assert!(matches!(at_cal.resolve(doc, &one("`nlsasm.S:1`")).unwrap(), Outcome::Ok));
        assert!(matches!(at_pin.resolve(doc, &one("`nlsasm.S:1`")).unwrap(), Outcome::Fail(m) if m == "missing"));
        assert!(matches!(
            at_pin.resolve(doc, &one("`ET/BDOS/proc.c`")).unwrap(),
            Outcome::Fail(m) if m.contains("case-different")
        ));
        // A build product the pin lacks resolves in the pinned build tree;
        // without that tree it is skipped.
        let product = one("`references/emutos/desk/desk_rsc.c:1`");
        if root().join(EMUTOS_BUILD_ROOT).join(&pin[..8]).join("desk/desk_rsc.c").is_file() {
            assert_eq!(at_pin.resolve(doc, &product).unwrap(), Outcome::Ok);
            assert_eq!(at_pin.resolve(doc, &one("`ET/aes/gem_rsc.c:1`")).unwrap(), Outcome::Ok);
            assert_eq!(at_pin.resolve(doc, &one("`desk_rsc.c:1`")).unwrap(), Outcome::Ok);
            assert!(matches!(
                at_pin.resolve(doc, &one("`ET/desk/no_such_product_x.c:1`")).unwrap(),
                Outcome::Fail(m) if m == "missing"
            ));
        }
        // The calibration commit has no build tree here (`<cal[..8]>/`).
        if !root().join(EMUTOS_BUILD_ROOT).join(&cal[..8]).is_dir() {
            assert_eq!(at_cal.resolve(doc, &product).unwrap(), Outcome::Skipped);
        }
        // A commit the checkout lacks counts as an absent tree: skipped.
        let mut absent = Resolver::with_emutos_commit(&root(), "0000000000000000000000000000000000000000");
        assert_eq!(absent.resolve(doc, &one("`ET/bdos/proc.c:1`")).unwrap(), Outcome::Skipped);
    }
}
