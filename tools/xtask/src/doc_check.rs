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
//! Markdown renderer looks. A backtick span tries, first hit wins: the citing
//! file's directory, the workspace root, then [`SEARCH_ROOTS`]. A bare
//! `name.ext:N` resolves by basename over `git ls-files` plus the files under
//! [`REFERENCE_ROOTS`]: one match resolves, several are "ambiguous". A hit must
//! match the on-disk name's case (macOS would accept `status.md`, Linux and
//! GitHub would not). The highest cited line must not exceed the line count.
//!
//! # `references/` may be absent
//!
//! `references/` is gitignored (`references/README.md`). A cite that does not
//! resolve and could have resolved under a missing reference tree is counted
//! as skipped, not failed: a cite under `references/emutos/…` itself, or a
//! path whose first component is no top-level entry of the workspace
//! (`aes/gemaplib.c`), or a bare name.
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
//! [`ARCHIVE_DIR`]: the archive is history, never evidence.

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

/// A workspace-relative path belongs to the default set's docs walk: a `.md`
/// file under [`DOCS_DIR`], not under [`ARCHIVE_DIR`].
pub fn is_default_doc(rel: &Path) -> bool {
    rel.starts_with(DOCS_DIR)
        && !rel.starts_with(ARCHIVE_DIR)
        && rel.extension().is_some_and(|e| e == "md")
}

/// The files checked when none are given: [`DEFAULT_FILES`], then the
/// Markdown files under [`DOCS_DIR`] (sorted), archive excluded.
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

/// Resolves cites against one workspace. Caches directory listings (for the
/// exact-case test) and the basename index (built on first bare cite).
pub struct Resolver {
    root: PathBuf,
    dir_cache: HashMap<PathBuf, Option<Vec<String>>>,
    basenames: Option<BTreeMap<String, Vec<PathBuf>>>,
}

impl Resolver {
    pub fn new(root: &Path) -> Self {
        Resolver { root: root.to_path_buf(), dir_cache: HashMap::new(), basenames: None }
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

    fn reference_missing(&self, rel: &str) -> bool {
        REFERENCE_ROOTS.iter().any(|r| rel.starts_with(&format!("{r}/")) && !self.root.join(r).is_dir())
    }

    fn any_reference_missing(&self) -> bool {
        REFERENCE_ROOTS.iter().any(|r| !self.root.join(r).is_dir())
    }

    /// Workspace-relative candidates for `path` cited from `doc_dir`
    /// (workspace-relative), in lookup order.
    fn candidates(&self, doc_dir: &Path, path: &str, kind: CiteKind) -> Vec<PathBuf> {
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
            for r in REFERENCE_ROOTS {
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
        let root = self.root.clone();
        let want_dir = cite.path.ends_with('/');
        let path = cite.path.trim_end_matches('/');
        let hit = if cite.kind == CiteKind::Span && !cite.path.contains('/') {
            let matches = self.basename_index()?.get(path).cloned().unwrap_or_default();
            match matches.len() {
                0 if self.any_reference_missing() => return Ok(Outcome::Skipped),
                0 => return Ok(Outcome::Fail("missing".into())),
                1 => matches[0].clone(),
                n => return Ok(Outcome::Fail(format!("ambiguous ({n} matches)"))),
            }
        } else {
            let doc_dir = doc.parent().unwrap_or(Path::new(""));
            let cands = self.candidates(doc_dir, path, cite.kind);
            let mut found = None;
            for c in &cands {
                if self.exists_exact(&root, c) && (!want_dir || root.join(c).is_dir()) {
                    found = Some(c.clone());
                    break;
                }
            }
            match found {
                Some(f) => f,
                None => return Ok(self.unresolved(&cands, path, cite.kind)),
            }
        };
        let full = root.join(&hit);
        if let Some(max) = cite.max_line {
            if full.is_dir() {
                return Ok(Outcome::Fail("is a directory, has a line suffix".into()));
            }
            let n = line_count(&full).map_err(|e| format!("doc-check: {}: {e}", full.display()))?;
            if max as usize > n || max == 0 {
                return Ok(Outcome::Fail(format!("line {max} beyond end ({n} lines) of {}", hit.display())));
            }
        }
        Ok(Outcome::Ok)
    }

    /// Why nothing matched: skipped (a reference tree it may live in is
    /// absent), a case mismatch, or missing.
    fn unresolved(&self, cands: &[PathBuf], path: &str, kind: CiteKind) -> Outcome {
        let under_missing_ref =
            cands.iter().any(|c| self.reference_missing(&c.to_string_lossy()));
        let first = path.split('/').next().unwrap_or("");
        let foreign_top = kind == CiteKind::Span
            && first != "."
            && first != ".."
            && !self.root.join(first).exists()
            && self.any_reference_missing();
        if under_missing_ref || foreign_top {
            return Outcome::Skipped;
        }
        if cands.iter().any(|c| self.root.join(c).exists()) {
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
    let b = fs::read(p)?;
    let nl = b.iter().filter(|&&c| c == b'\n').count();
    Ok(nl + usize::from(!b.is_empty() && *b.last().unwrap() != b'\n'))
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
}
