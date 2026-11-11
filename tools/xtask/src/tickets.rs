//! TOOL41 — the ADR-037 ticket tools: `status-check`, `accept`,
//! `set-status`, `report` (`docs/MEMORY.md`, ADR-037 with its addendum of
//! 2026-11-11, points 11–14, decisions e–g).
//!
//! # Line 3
//!
//! Every ticket `TASK/<ID>.md` carries its state on line 3 (point 3):
//!
//! ```text
//! Status: STATE (YYYY-MM-DD) | Priority: Pn | Key: value | ...
//! ```
//!
//! [`parse_line3`] is the one parser every command uses. Line 1 is
//! `# <ID> — <title>`, line 2 empty. A line 3 whose state word is not one of
//! the six [`State`]s is *legacy* (written before ADR-037): it is classified
//! for the report by the mapping of point 2 ([`legacy_state`]) and recorded
//! in the committed baseline [`BASELINE_FILE`], never rewritten here
//! (point 7, TOOL41 Work 9).
//!
//! # Violations
//!
//! [`Code`] names every gap. The *rejecting* ones are the gaps of point 13
//! (plus a malformed line 3, an orphan receipt and an unknown file in
//! `TASK/`, point 9); the *reporting* ones print without failing: a `PARKED`
//! ticket blocked by a `DONE` one (point 10), a flagged self-acceptance
//! (point 12), a `fix`/`feat` commit naming an `OPEN`/`PARKED` ticket in the
//! history, the `docs/STATUS.md` lists against line 3 (point 8, keyword
//! candidates like the inventory's C3) and a stale `TASK/README.md` status
//! column. [`Code::group`] puts each in a gap group. The groups are this
//! tool's machine rules; they resemble the inventory's keyword classes C1–C6
//! (`docs/analysis/ticket-inventory-2026-11-10.md`) but are not those
//! classes and carry other names, so their counts are not comparable
//! (e.g. every legacy line 3 is `form`, every legacy `PARKED` lacks the
//! `Reason`/`Reopen` fields).
//! (`docs/analysis/ticket-inventory-2026-11-10.md`).
//!
//! # Baseline
//!
//! [`BASELINE_FILE`] lists the rejecting violations of the tree at the time
//! the tools landed, one `<key>\t<code>` per line (the key is a ticket id, or
//! the file name of an orphan receipt or unknown file). A violation in the
//! baseline passes; any other rejects. The baseline only shrinks: an entry
//! whose violation is gone rejects until `status-check --prune-baseline`
//! removes it, and the checked baseline may not hold an entry the one at
//! `HEAD` lacks — so a ticket that leaves the baseline cannot come back.
//! `set-status` and `accept` prune the entries of the ticket they write.
//!
//! # Roles (point 12)
//!
//! A receipt `TASK/<ID>[-step]-STATUS-<STATE>.MD` records its role and agent
//! in its *header*, the lines before its first `## ` heading, as the lines
//! `Role: developer|tester` and `Agent: <name>`. The developer of a ticket
//! is every `Agent` of a receipt header with `Role: developer`; `accept`
//! writes `Role: tester` and the acceptor. Names compare trimmed and
//! case-insensitively.
//!
//! # Names
//!
//! A ticket id is *named* by a text when it stands there on a word boundary
//! (the characters around it are not ASCII letters, digits or `_`; `-` is a
//! boundary, so `DESK43-U5a` names `DESK43`). The hooks take the ids from the
//! subject line of the message; `accept` from the whole message of the
//! commit it accepts.
//!
//! # The switch
//!
//! The hooks run the ticket checks only when `git config newtos.ticketChecks`
//! is `true` ([`SWITCH`]); off (unset or `false`) they behave as before TOOL41
//! (point 14: switched on only after the owner's OK).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::doc_check::{Resolver, Tree};

/// The ticket directory, workspace-relative.
pub const TASK_DIR: &str = "TASK";
/// The committed baseline of legacy violations.
pub const BASELINE_FILE: &str = "tools/xtask/ticket-baseline.tsv";
/// The git config key the hooks read (`true` = ticket checks on).
pub const SWITCH: &str = "newtos.ticketChecks";
/// `docs/STATUS.md`, checked against line 3 (point 8).
pub const STATUS_FILE: &str = "docs/STATUS.md";
/// `TASK/README.md`, whose status column is generated from line 3.
pub const README_FILE: &str = "TASK/README.md";

/// The entries of `TASK/` that are not tickets (ADR-037 point 9); no other
/// file is exempt.
pub const NON_TICKETS: &[&str] = &[
    "README.md",
    "COMPARISON.md",
    "COVERAGE.md",
    "GAPS-INDEX-2026-09-12.md",
    "HANDOVER-2026-09-11.md",
    "RSC-AUDIT-INDEX.md",
    "A25-AUDIT-BEANSTANDET.MD",
    "KEYS01-SEQUENCE-CHECK.MD",
    "PORT06-KEYS-DIAGNOSIS.MD",
    "evidence",
    "migration",
    "tasks.json",
];

/// The receipt states of `<ID>-STATUS-<STATE>.MD` (`TASK/README.md`,
/// ADR-037 point 6).
pub const RECEIPT_STATES: &[&str] = &["DONE", "UNFINISHED", "FAILURE", "BLOCKED"];

/// The usage text of the ticket commands, appended to `cargo xtask help`.
pub const USAGE: &str = "\
Ticket tools (ADR-037, TOOL41; tools/xtask/src/tickets.rs):
  status-check [--staged] [--new-only] [--message-file <f>]
               [--write-baseline | --prune-baseline]
                                 — the ADR-037 gap check (point 13) over
                                   TASK/: a rejecting gap not recorded in
                                   tools/xtask/ticket-baseline.tsv exits 1,
                                   reporting ones print; --new-only: only
                                   the tickets the change touches; --staged:
                                   the index; --message-file: the ids its
                                   subject names plus the fix/feat rule (the
                                   commit-msg hook); the baseline only
                                   shrinks (--prune-baseline)
  accept <ID> --commit <hash> --result PASS|FAIL [--note <text>] --agent <name>
                                 — the tester's acceptance (point 11): line 3
                                   IMPLEMENTED, <hash> on the history and
                                   naming <ID>, nothing else staged, <name>
                                   not the recorded developer; PASS writes
                                   <ID>-STATUS-DONE.MD and line 3 DONE, FAIL
                                   appends to <ID>-STATUS-UNFINISHED.MD;
                                   commits by itself (decision e)
  set-status <ID> PARKED --reason R --reopen T | OBSOLETE --reason R
             [--target ID] | ACCEPTED --ref ADR-NNN | OPEN --reason R
             [--priority Pn]
                                 — the lead's transitions (point 11, decision
                                   f): line 3 and a status-history entry;
                                   never IMPLEMENTED or DONE; not committed
  report [--summary | --readme [--check]]
                                 — counts per state, gap group and code;
                                   --summary one line; --readme regenerates
                                   the Status column of TASK/README.md from
                                   line 3 (--check: exit 1 if stale)
";

/// Dispatches the ticket commands; any other `name` is an unknown command.
pub fn cmd(name: &str, root: &Path, args: &[String]) -> Result<(), String> {
    match name {
        "status-check" => cmd_status_check(root, args),
        "accept" => cmd_accept(root, args),
        "set-status" => cmd_set_status(root, args),
        "report" => cmd_report(root, args),
        "migrate" => migrate::cmd_migrate(root, args),
        other => Err(format!("unknown subcommand: {other}")),
    }
}

// ---------------------------------------------------------------------------
// Line 3
// ---------------------------------------------------------------------------

/// The six ticket states (ADR-037 point 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum State {
    Open,
    Implemented,
    Done,
    Parked,
    Accepted,
    Obsolete,
}

pub const STATES: [State; 6] =
    [State::Open, State::Implemented, State::Done, State::Parked, State::Accepted, State::Obsolete];

impl State {
    pub fn name(self) -> &'static str {
        match self {
            State::Open => "OPEN",
            State::Implemented => "IMPLEMENTED",
            State::Done => "DONE",
            State::Parked => "PARKED",
            State::Accepted => "ACCEPTED",
            State::Obsolete => "OBSOLETE",
        }
    }

    pub fn parse(s: &str) -> Option<State> {
        STATES.into_iter().find(|st| st.name() == s)
    }

    /// The required keys after `Priority`, in order (point 1).
    pub fn required(self) -> &'static [&'static str] {
        match self {
            State::Open => &[],
            State::Implemented => &["Commit", "Missing"],
            State::Done => &["Commit", "Receipt"],
            State::Parked => &["Reason", "Reopen"],
            State::Accepted => &["Ref"],
            State::Obsolete => &["Reason"],
        }
    }

    /// The optional keys after the required ones (`Target` of a duplicate or
    /// supersession, point 1).
    pub fn optional(self) -> &'static [&'static str] {
        match self {
            State::Obsolete => &["Target"],
            _ => &[],
        }
    }
}

/// A line 3 in ADR-037 form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line3 {
    pub state: State,
    pub date: String,
    /// 1..=4.
    pub priority: u8,
    /// The keys after `Priority`, in order.
    pub fields: Vec<(String, String)>,
}

impl Line3 {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.fields.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }

    /// The line as written (the inverse of [`parse_line3`]).
    pub fn render(&self) -> String {
        let mut s = format!("Status: {} ({}) | Priority: P{}", self.state.name(), self.date, self.priority);
        for (k, v) in &self.fields {
            let _ = write!(s, " | {k}: {v}");
        }
        s
    }
}

/// One thing wrong with a line 3: the field and what was expected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldError {
    pub field: String,
    pub expected: String,
    /// The field is absent (as opposed to present and wrong).
    pub missing: bool,
}

/// Why a line 3 is not in ADR-037 form: the state word if it is one of the
/// six, and every error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line3Errors {
    pub state: Option<State>,
    pub errors: Vec<FieldError>,
}

fn err(field: &str, expected: impl Into<String>) -> FieldError {
    FieldError { field: field.to_string(), expected: expected.into(), missing: false }
}

/// The state word of a line 3 (`Status: <word> ...`), if it is one of the six.
pub fn state_word(line: &str) -> Option<State> {
    let rest = line.strip_prefix("Status: ")?;
    State::parse(rest.split([' ', '(']).next().unwrap_or(""))
}

/// `YYYY-MM-DD` with month 1–12 and day 1–31.
pub fn is_date(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return false;
    }
    if !s.chars().enumerate().all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit()) {
        return false;
    }
    let m: u32 = s[5..7].parse().unwrap_or(0);
    let d: u32 = s[8..10].parse().unwrap_or(0);
    (1..=12).contains(&m) && (1..=31).contains(&d)
}

/// A commit hash as line 3 writes it: 7–40 lowercase hex digits.
pub fn is_hash(s: &str) -> bool {
    (7..=40).contains(&s.len()) && s.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}

/// A ticket id as it may stand in a `Target`: ASCII letters, digits, `-`.
pub fn is_id(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
}

/// `ADR-NNN`, or a `docs/COMPATIBILITY.md` entry.
pub fn is_ref(s: &str) -> bool {
    let adr = s.strip_prefix("ADR-").is_some_and(|n| n.len() == 3 && n.bytes().all(|c| c.is_ascii_digit()));
    adr || s.starts_with("docs/COMPATIBILITY.md")
}

/// Parses line 3 of `TASK/<id>.md` (ADR-037 point 3, the required fields of
/// point 1, `Commit: self` of point 11).
pub fn parse_line3(id: &str, line: &str) -> Result<Line3, Line3Errors> {
    let state_guess = state_word(line);
    let fail = |errors: Vec<FieldError>| Err(Line3Errors { state: state_guess, errors });
    if let Some((col, c)) = line.char_indices().find(|(_, c)| !c.is_ascii()) {
        return fail(vec![err("line 3", format!("ASCII only, found {c:?} at column {}", col + 1))]);
    }
    let parts: Vec<&str> = line.split(" | ").collect();
    let mut errors = Vec::new();
    if parts.iter().any(|p| p.contains('|')) {
        return fail(vec![err("line 3", "fields separated by ` | `, no `|` inside a value")]);
    }
    // Status: STATE (YYYY-MM-DD)
    let head = parts[0];
    let mut state = None;
    let mut date = String::new();
    match head.strip_prefix("Status: ") {
        None => errors.push(err("Status", "line 3 to start with `Status: `")),
        Some(rest) => {
            let (word, tail) = rest.split_once(' ').unwrap_or((rest, ""));
            match State::parse(word) {
                None => errors.push(err(
                    "Status",
                    format!(
                        "one of {}, found {word:?}",
                        STATES.iter().map(|s| s.name()).collect::<Vec<_>>().join(", ")
                    ),
                )),
                Some(s) => state = Some(s),
            }
            match tail.strip_prefix('(').and_then(|t| t.strip_suffix(')')) {
                Some(d) if is_date(d) => date = d.to_string(),
                _ => errors.push(err("Status", format!("`(YYYY-MM-DD)` after the state, found {tail:?}"))),
            }
        }
    }
    // Priority: Pn, always second.
    let mut priority = 0;
    match parts.get(1).map(|p| p.split_once(": ")) {
        None => errors.push(FieldError { field: "Priority".into(), expected: "`Priority: P1`..`P4` as the second field".into(), missing: true }),
        Some(Some(("Priority", v))) if v.len() == 2 && v.starts_with('P') && (b'1'..=b'4').contains(&v.as_bytes()[1]) => {
            priority = v.as_bytes()[1] - b'0'
        }
        Some(_) => errors.push(err("Priority", format!("`Priority: P1`..`P4` as the second field, found {:?}", parts[1]))),
    }
    let Some(st) = state else {
        return fail(errors);
    };
    // The state's keys, in order; nothing else.
    let mut fields = Vec::new();
    let given: Vec<&str> = parts.iter().skip(2).copied().collect();
    let mut i = 0;
    for key in st.required() {
        match given.get(i).and_then(|f| f.split_once(": ")) {
            Some((k, v)) if k == *key => {
                if let Some(e) = check_value(id, st, key, v) {
                    errors.push(e);
                }
                fields.push((k.to_string(), v.to_string()));
                i += 1;
            }
            _ => errors.push(FieldError {
                field: key.to_string(),
                expected: format!("`{key}: ...` (required for {}, in the order {})", st.name(), st.required().join(", ")),
                missing: true,
            }),
        }
    }
    for key in st.optional() {
        if let Some((k, v)) = given.get(i).and_then(|f| f.split_once(": ")) {
            if k == *key {
                if let Some(e) = check_value(id, st, key, v) {
                    errors.push(e);
                }
                fields.push((k.to_string(), v.to_string()));
                i += 1;
            }
        }
    }
    for extra in given.iter().skip(i) {
        let k = extra.split_once(": ").map_or(*extra, |(k, _)| k);
        errors.push(err(k, format!("no further keys on line 3 for {} (detail goes below it)", st.name())));
    }
    if errors.is_empty() {
        Ok(Line3 { state: st, date, priority, fields })
    } else {
        fail(errors)
    }
}

fn check_value(id: &str, st: State, key: &str, v: &str) -> Option<FieldError> {
    if v.is_empty() || v.trim() != v {
        return Some(err(key, "a non-empty value without surrounding blanks"));
    }
    match key {
        "Commit" if st == State::Implemented && v != "self" && !is_hash(v) => {
            Some(err(key, format!("`self` or a commit hash (7-40 hex digits), found {v:?}")))
        }
        "Commit" if st == State::Done && !is_hash(v) => {
            Some(err(key, format!("a commit hash (7-40 hex digits), found {v:?}")))
        }
        "Receipt" if v != format!("{id}-STATUS-DONE.MD") => {
            Some(err(key, format!("`{id}-STATUS-DONE.MD`, found {v:?}")))
        }
        "Ref" if !is_ref(v) => Some(err(key, format!("`ADR-NNN` or a `docs/COMPATIBILITY.md` entry, found {v:?}"))),
        "Target" if !is_id(v) => Some(err(key, format!("a ticket id, found {v:?}"))),
        _ => None,
    }
}

/// The ADR-037 point 2 mapping of a legacy line 3 (`**` removed, leading
/// words): `None` for a part-done line or an unknown word (the owner
/// decides).
pub fn legacy_state(line: &str) -> Option<State> {
    let t = line.strip_prefix("Status:")?.replace("**", "");
    let t = t.trim_start().to_uppercase();
    const MAP: &[(&str, State)] = &[
        ("ABGESCHLOSSEN ALS BLOCKED", State::Parked),
        ("URSACHE GEFUNDEN UND BEHOBEN", State::Done),
        ("BEFUND ERLEDIGT, ABNAHME OFFEN", State::Implemented),
        ("URSACHE BELEGT", State::Open),
        ("IN ABNAHME", State::Implemented),
        ("OFFEN", State::Open),
        ("IMPLEMENTIERT", State::Implemented),
        ("UMGESETZT", State::Implemented),
        ("TEILWEISE", State::Implemented),
        ("ERLEDIGT", State::Done),
        ("BEHOBEN", State::Done),
        ("GESCHLOSSEN", State::Done),
        ("ABGESCHLOSSEN", State::Done),
        ("GEKLAERT", State::Done),
        ("GEPARKT", State::Parked),
        ("ZURUECKGESTELLT", State::Parked),
        ("ZURÜCKGESTELLT", State::Parked),
        ("BACKLOG", State::Parked),
        ("ZUKUNFT", State::Parked),
        ("IDEE", State::Parked),
        ("ACCEPTED-ABWEICHUNG", State::Accepted),
        ("GEGENSTANDSLOS", State::Obsolete),
        ("ZURUECKGEZOGEN", State::Obsolete),
    ];
    MAP.iter()
        .find(|(w, _)| {
            t.strip_prefix(w).is_some_and(|r| r.chars().next().is_none_or(|c| !c.is_alphanumeric() && c != '-'))
        })
        .map(|(_, s)| *s)
}

/// `[text](target)` → `text`.
pub fn strip_links(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find("](") {
        match (rest[..i].rfind('['), rest[i..].find(')')) {
            (Some(open), Some(close)) => {
                out.push_str(&rest[..open]);
                out.push_str(&rest[open + 1..i]);
                rest = &rest[i + close + 1..];
            }
            _ => break,
        }
    }
    out.push_str(rest);
    out
}

/// The raw status text of a legacy line 3: up to the first ` · ` field
/// separator, `**` and link targets removed, `|` replaced (for a table
/// cell), at most 60 characters.
pub fn raw_status(line: &str) -> String {
    let t = strip_links(&line.strip_prefix("Status:").unwrap_or(line).replace("**", ""));
    let t = t.trim().split(" · ").next().unwrap_or("").trim().replace('|', "/");
    if t.chars().count() > 60 {
        format!("{}…", t.chars().take(59).collect::<String>())
    } else {
        t
    }
}

/// The priority of a legacy line 3 (`Priority`/`Prioritaet`/`Priorität: Pn`).
pub fn legacy_priority(line: &str) -> Option<u8> {
    for key in ["Priority: P", "Prioritaet: P", "Priorität: P"] {
        if let Some(i) = line.find(key) {
            let d = line[i + key.len()..].bytes().next()?;
            if (b'1'..=b'4').contains(&d) {
                return Some(d - b'0');
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Names and messages
// ---------------------------------------------------------------------------

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// `text` names `id` on a word boundary.
pub fn names(text: &str, id: &str) -> bool {
    let (t, n) = (text.as_bytes(), id.len());
    let mut from = 0;
    while let Some(i) = text[from..].find(id).map(|i| i + from) {
        let before = i == 0 || !is_word_byte(t[i - 1]);
        let after = i + n >= t.len() || !is_word_byte(t[i + n]);
        if before && after {
            return true;
        }
        from = i + 1;
    }
    false
}

/// The subject of a commit message as the `commit-msg` hook sees it: the
/// first non-empty line that is no `#` comment.
pub fn subject(msg: &str) -> &str {
    msg.lines().map(str::trim).find(|l| !l.is_empty() && !l.starts_with('#')).unwrap_or("")
}

/// `fix(...)`, `fix:`, `feat(...)!:` and the like (Conventional Commits).
pub fn is_fix_or_feat(subject: &str) -> bool {
    for t in ["fix", "feat"] {
        if let Some(r) = subject.strip_prefix(t) {
            let r = if r.starts_with('(') { r.find(')').map_or("", |i| &r[i + 1..]) } else { r };
            let r = r.strip_prefix('!').unwrap_or(r);
            if r.starts_with(':') {
                return true;
            }
        }
    }
    false
}

// ---------------------------------------------------------------------------
// A view of TASK/
// ---------------------------------------------------------------------------

/// Where the checks read `TASK/`, the baseline, `docs/STATUS.md` from.
pub enum Source {
    /// The working tree.
    Disk,
    /// The index (`--staged`).
    Index(Box<Resolver>),
}

pub struct View {
    pub root: PathBuf,
    pub source: Source,
}

impl View {
    pub fn disk(root: &Path) -> View {
        View { root: root.to_path_buf(), source: Source::Disk }
    }

    pub fn index(root: &Path) -> Result<View, String> {
        let mut r = Resolver::new(root);
        r.use_tree(&Tree::Index, "HEAD")?;
        Ok(View { root: root.to_path_buf(), source: Source::Index(Box::new(r)) })
    }

    /// The entries directly in `TASK/` (file names; a directory by its name).
    pub fn task_entries(&mut self) -> Result<BTreeSet<String>, String> {
        let mut out = BTreeSet::new();
        match &mut self.source {
            Source::Disk => {
                let dir = self.root.join(TASK_DIR);
                let rd = fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
                for e in rd.filter_map(|e| e.ok()) {
                    let n = e.file_name().to_string_lossy().into_owned();
                    if !n.starts_with('.') {
                        out.insert(n);
                    }
                }
            }
            Source::Index(r) => {
                for p in r.tracked()? {
                    if let Ok(rest) = p.strip_prefix(TASK_DIR) {
                        if let Some(first) = rest.components().next() {
                            out.insert(first.as_os_str().to_string_lossy().into_owned());
                        }
                    }
                }
            }
        }
        Ok(out)
    }

    /// Workspace-relative `rel` in the view, `None` if absent.
    pub fn read(&mut self, rel: &str) -> Result<Option<String>, String> {
        match &mut self.source {
            Source::Disk => Ok(fs::read(self.root.join(rel)).ok().map(|b| String::from_utf8_lossy(&b).into_owned())),
            Source::Index(r) => Ok(r.tree_file(Path::new(rel))?.map(|b| String::from_utf8_lossy(&b).into_owned())),
        }
    }
}

/// One receipt file.
#[derive(Debug, Clone)]
pub struct Receipt {
    pub file: String,
    /// The text before `-STATUS-` (a ticket id or `<id>-<step>`).
    pub prefix: String,
    /// The ticket it belongs to (prefix, or its longest `-`-prefix that is a
    /// ticket); `None` = orphan.
    pub ticket: Option<String>,
    pub text: String,
}

impl Receipt {
    /// The header fields (lines before the first `## ` heading) `key: value`.
    pub fn header(&self, key: &str) -> Vec<String> {
        header_fields(&self.text, key)
    }
}

/// The values of `key: value` lines before the first `## ` heading.
pub fn header_fields(text: &str, key: &str) -> Vec<String> {
    let p = format!("{key}:");
    text.lines()
        .take_while(|l| !l.starts_with("## "))
        .filter_map(|l| l.strip_prefix(&p).map(|v| v.trim().to_string()))
        .filter(|v| !v.is_empty())
        .collect()
}

#[derive(Debug, Clone)]
pub struct Ticket {
    pub id: String,
    pub line1: String,
    pub line2: String,
    pub line3: String,
    pub parsed: Result<Line3, Line3Errors>,
    /// The ADR-037 state, or the mapped legacy one.
    pub effective: Option<State>,
    /// Line 3's state word is not one of the six.
    pub legacy: bool,
    /// The `Migrated (owner batch N, ...), evidence: ...` lines (TOOL54).
    pub migrated: Vec<String>,
}

impl Ticket {
    pub fn from_text(id: &str, text: &str) -> Ticket {
        let mut lines = text.lines();
        let line1 = lines.next().unwrap_or("").to_string();
        let line2 = lines.next().unwrap_or("").to_string();
        let line3 = lines.next().unwrap_or("").to_string();
        let parsed = parse_line3(id, &line3);
        let legacy = state_word(&line3).is_none();
        let effective = match &parsed {
            Ok(l) => Some(l.state),
            Err(e) if !legacy => e.state,
            Err(_) => legacy_state(&line3),
        };
        Ticket { id: id.to_string(), line1, line2, line3, parsed, effective, legacy, migrated: migrated_lines(text) }
    }

    /// The README status cell: the state, or the raw legacy text.
    pub fn status_cell(&self) -> String {
        if self.legacy {
            raw_status(&self.line3)
        } else {
            state_word(&self.line3).map(|s| s.name().to_string()).unwrap_or_default()
        }
    }
}

/// `TASK/` as one view records it.
pub struct Analysis {
    pub tickets: BTreeMap<String, Ticket>,
    /// By file name.
    pub receipts: BTreeMap<String, Receipt>,
    /// Entries that are neither ticket, receipt nor in [`NON_TICKETS`].
    pub unknown: BTreeSet<String>,
    /// The commit map and the owner's historical receipts (TOOL54 rework).
    pub aids: Aids,
}

/// `<prefix>-STATUS-<STATE>.MD` → (prefix, state).
pub fn receipt_name(name: &str) -> Option<(&str, &str)> {
    let stem = name.strip_suffix(".MD")?;
    let i = stem.rfind("-STATUS-")?;
    Some((&stem[..i], &stem[i + 8..]))
}

impl Analysis {
    pub fn load(view: &mut View) -> Result<Analysis, String> {
        let entries = view.task_entries()?;
        let mut tickets = BTreeMap::new();
        let mut receipt_files = Vec::new();
        let mut unknown = BTreeSet::new();
        for name in &entries {
            if NON_TICKETS.contains(&name.as_str()) {
                continue;
            }
            if let Some(id) = name.strip_suffix(".md") {
                let text = view.read(&format!("{TASK_DIR}/{name}"))?.unwrap_or_default();
                tickets.insert(id.to_string(), Ticket::from_text(id, &text));
            } else if receipt_name(name).is_some_and(|(_, s)| RECEIPT_STATES.contains(&s)) {
                receipt_files.push(name.clone());
            } else {
                unknown.insert(name.clone());
            }
        }
        let mut receipts = BTreeMap::new();
        for name in receipt_files {
            let (prefix, _) = receipt_name(&name).unwrap();
            let mut p = prefix;
            let ticket = loop {
                if tickets.contains_key(p) {
                    break Some(p.to_string());
                }
                match p.rfind('-') {
                    Some(i) => p = &p[..i],
                    None => break None,
                }
            };
            let text = view.read(&format!("{TASK_DIR}/{name}"))?.unwrap_or_default();
            receipts.insert(
                name.clone(),
                Receipt { file: name.clone(), prefix: prefix.to_string(), ticket, text },
            );
        }
        Ok(Analysis { tickets, receipts, unknown, aids: Aids::load(view)? })
    }

    pub fn receipts_of<'a>(&'a self, id: &'a str) -> impl Iterator<Item = &'a Receipt> + 'a {
        self.receipts.values().filter(move |r| r.ticket.as_deref() == Some(id))
    }

    /// The direct DONE receipt `<id>-STATUS-DONE.MD`.
    pub fn done_receipt(&self, id: &str) -> Option<&Receipt> {
        self.receipts.get(&format!("{id}-STATUS-DONE.MD"))
    }

    /// The developer agents recorded for `id` (point 12).
    pub fn developers(&self, id: &str) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        for r in self.receipts_of(id) {
            if r.header("Role").iter().any(|v| v.eq_ignore_ascii_case("developer")) {
                out.extend(r.header("Agent").iter().map(|a| a.to_lowercase()));
            }
        }
        out
    }

    /// The ids of the tickets `text` names.
    pub fn named_in(&self, text: &str) -> BTreeSet<String> {
        self.tickets.keys().filter(|id| names(text, id)).cloned().collect()
    }

    /// The key a `TASK/` entry is checked and baselined under: the ticket of
    /// a ticket file or receipt, else the entry's file name.
    pub fn key_of(&self, entry: &str) -> String {
        if let Some(id) = entry.strip_suffix(".md") {
            if !NON_TICKETS.contains(&entry) {
                return id.to_string();
            }
        }
        if let Some((prefix, _)) = receipt_name(entry) {
            let mut p = prefix;
            loop {
                if self.tickets.contains_key(p) {
                    return p.to_string();
                }
                match p.rfind('-') {
                    Some(i) => p = &p[..i],
                    None => break,
                }
            }
        }
        entry.to_string()
    }
}

// ---------------------------------------------------------------------------
// Violations
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Code {
    // Rejecting.
    /// Line 3 (or line 1/2) not in ADR-037 form, legacy included.
    Form,
    /// `IMPLEMENTED` without `Commit` or `Missing` (point 13).
    ImplementedFields,
    /// `PARKED` without `Reason` and `Reopen` (point 13).
    ParkedNoReason,
    /// A `<ID>-STATUS-DONE.MD` while line 3 is not `DONE` (point 13).
    DoneReceiptNotDone,
    /// Line 3 `DONE` without a DONE receipt that names its commit.
    DoneNoReceipt,
    /// `Commit: <hash>` that is no commit of the repository.
    BadCommit,
    /// `Target`/`Ref` naming no ticket / no ADR.
    BadTarget,
    /// A receipt with no ticket.
    OrphanReceipt,
    /// A `TASK/` entry that is no ticket, receipt or listed non-ticket.
    UnknownFile,
    /// A `fix`/`feat` subject naming a ticket whose line 3 is not
    /// `IMPLEMENTED` (point 11/13; never baselined).
    FixFeatState,
    /// A baseline entry whose violation is gone, or one `HEAD` lacks.
    Baseline,
    // Reporting.
    /// `PARKED` with `Reason: blocked by <ID>` and `<ID>` is `DONE` (point 10).
    BlockedByDone,
    /// A `DONE` whose acceptor is its developer (point 12).
    SelfAccept,
    /// `OPEN`/`PARKED` and a `fix`/`feat` commit of the history names it.
    FixCommitNotClosed,
    /// `docs/STATUS.md` mention against line 3 (keyword candidate).
    StatusMd,
    /// `TASK/README.md` status column differs from line 3.
    ReadmeStale,
}

pub const CODES: [Code; 16] = [
    Code::Form,
    Code::ImplementedFields,
    Code::ParkedNoReason,
    Code::DoneReceiptNotDone,
    Code::DoneNoReceipt,
    Code::BadCommit,
    Code::BadTarget,
    Code::OrphanReceipt,
    Code::UnknownFile,
    Code::FixFeatState,
    Code::Baseline,
    Code::BlockedByDone,
    Code::SelfAccept,
    Code::FixCommitNotClosed,
    Code::StatusMd,
    Code::ReadmeStale,
];

impl Code {
    pub fn name(self) -> &'static str {
        match self {
            Code::Form => "form",
            Code::ImplementedFields => "implemented-fields",
            Code::ParkedNoReason => "parked-no-reason",
            Code::DoneReceiptNotDone => "done-receipt-not-done",
            Code::DoneNoReceipt => "done-no-receipt",
            Code::BadCommit => "bad-commit",
            Code::BadTarget => "bad-target",
            Code::OrphanReceipt => "orphan-receipt",
            Code::UnknownFile => "unknown-file",
            Code::FixFeatState => "fix-feat-state",
            Code::Baseline => "baseline",
            Code::BlockedByDone => "blocked-by-done",
            Code::SelfAccept => "self-accept",
            Code::FixCommitNotClosed => "fix-commit-not-closed",
            Code::StatusMd => "status-md",
            Code::ReadmeStale => "readme-stale",
        }
    }

    pub fn parse(s: &str) -> Option<Code> {
        CODES.into_iter().find(|c| c.name() == s)
    }

    pub fn rejecting(self) -> bool {
        self < Code::BlockedByDone
    }

    /// May stand in the baseline (legacy violations of a ticket's files).
    pub fn baselinable(self) -> bool {
        self.rejecting() && !matches!(self, Code::FixFeatState | Code::Baseline)
    }

    /// The gap group of `report` (see the module doc: not the inventory's
    /// classes C1–C6).
    pub fn group(self) -> &'static str {
        match self {
            Code::DoneReceiptNotDone | Code::FixCommitNotClosed => "done-not-closed",
            Code::DoneNoReceipt => "closed-without-evidence",
            Code::StatusMd => "status-md-mismatch",
            Code::ParkedNoReason => "parked-without-fields",
            Code::Form | Code::UnknownFile | Code::ImplementedFields => "not-adr037-form",
            Code::OrphanReceipt | Code::BadTarget => "dangling-reference",
            _ => "-",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Violation {
    pub key: String,
    pub code: Code,
    pub msg: String,
}

fn v(key: &str, code: Code, msg: impl Into<String>) -> Violation {
    Violation { key: key.to_string(), code, msg: msg.into() }
}

/// The hex tokens (7+ digits) of `text`.
fn hex_tokens(text: &str) -> Vec<&str> {
    text.split(|c: char| !c.is_ascii_hexdigit())
        .filter(|t| t.len() >= 7 && t.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)))
        .collect()
}

/// `text` names commit `hash` (one is a prefix of the other, 7+ digits).
pub fn names_commit(text: &str, hash: &str) -> bool {
    hex_tokens(text).iter().any(|t| t.starts_with(hash) || hash.starts_with(t))
}

/// What the checks ask git: does a commit exist.
pub trait Repo {
    fn commit_exists(&mut self, hash: &str) -> bool;
}

pub struct GitRepo(pub PathBuf);

impl Repo for GitRepo {
    fn commit_exists(&mut self, hash: &str) -> bool {
        Command::new("git")
            .args(["cat-file", "-e", &format!("{hash}^{{commit}}")])
            .current_dir(&self.0)
            .output()
            .is_ok_and(|o| o.status.success())
    }
}

/// The violations of ticket `id` (its file and receipts) in `a`; `adrs` the
/// ADR numbers of `docs/MEMORY.md`.
pub fn ticket_violations(a: &Analysis, id: &str, adrs: &BTreeSet<u16>, repo: &mut dyn Repo) -> Vec<Violation> {
    let mut out = Vec::new();
    let Some(t) = a.tickets.get(id) else { return out };
    let file = format!("{TASK_DIR}/{id}.md");
    match &t.parsed {
        Ok(_) => {}
        Err(_) if t.legacy => {
            let mapped = t.effective.map_or("no mapping (owner decides)".to_string(), |s| format!("maps to {}", s.name()));
            out.push(v(id, Code::Form, format!("{file}:3: legacy line 3 ({mapped}): {:?}", raw_status(&t.line3))));
            if t.effective == Some(State::Parked) {
                out.push(v(id, Code::ParkedNoReason, format!("{file}:3: legacy PARKED line without `Reason`/`Reopen` fields")));
            }
        }
        Err(e) => {
            for fe in &e.errors {
                let code = match (e.state, fe.field.as_str(), fe.missing) {
                    (Some(State::Implemented), "Commit" | "Missing", true) => Code::ImplementedFields,
                    (Some(State::Parked), "Reason" | "Reopen", true) => Code::ParkedNoReason,
                    _ => Code::Form,
                };
                out.push(v(id, code, format!("{file}:3: {}: expected {}", fe.field, fe.expected)));
            }
        }
    }
    if !t.legacy {
        if !t.line1.starts_with(&format!("# {id} — ")) {
            out.push(v(id, Code::Form, format!("{file}:1: expected `# {id} — <title>`, found {:?}", t.line1)));
        }
        if !t.line2.is_empty() {
            out.push(v(id, Code::Form, format!("{file}:2: expected an empty line")));
        }
    }
    let done_receipt = a.done_receipt(id);
    if done_receipt.is_some() && t.effective != Some(State::Done) {
        let st = t.effective.map_or("unmapped".to_string(), |s| s.name().to_string());
        out.push(v(
            id,
            Code::DoneReceiptNotDone,
            format!("{TASK_DIR}/{id}-STATUS-DONE.MD exists while line 3 of {file} is not DONE ({st})"),
        ));
    }
    if t.effective == Some(State::Done) {
        match (done_receipt, &t.parsed) {
            (None, _) => out.push(v(id, Code::DoneNoReceipt, format!("{file}:3: DONE without {id}-STATUS-DONE.MD"))),
            (Some(r), Ok(l)) => {
                let c = l.get("Commit").unwrap_or("");
                if !evidence_names_commit(&a.aids.map, &r.text, &t.migrated, c) {
                    out.push(v(id, Code::DoneNoReceipt, format!("{TASK_DIR}/{}: does not name the commit {c} of line 3", r.file)));
                }
            }
            (Some(_), Err(_)) => {}
        }
    }
    if let Ok(l) = &t.parsed {
        if let Some(c) = l.get("Commit").filter(|c| *c != "self") {
            if !repo.commit_exists(c) {
                out.push(v(id, Code::BadCommit, format!("{file}:3: Commit: {c} is no commit of this repository (an old hash? tools/oldhash.sh)")));
            }
        }
        if let Some(tg) = l.get("Target") {
            if !a.tickets.contains_key(tg) {
                out.push(v(id, Code::BadTarget, format!("{file}:3: Target: {tg} is no ticket in {TASK_DIR}/")));
            }
        }
        if let Some(r) = l.get("Ref").and_then(|r| r.strip_prefix("ADR-")) {
            if !r.parse::<u16>().is_ok_and(|n| adrs.contains(&n)) {
                out.push(v(id, Code::BadTarget, format!("{file}:3: Ref: ADR-{r} is no ADR heading in docs/MEMORY.md")));
            }
        }
        // Reporting: blocked by a ticket that is DONE (point 10).
        if l.state == State::Parked {
            let reason = l.get("Reason").unwrap_or("");
            if let Some(i) = reason.to_ascii_lowercase().find("blocked by ") {
                let rest = &reason[i + 11..];
                let dep: String = rest.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '-').collect();
                if a.tickets.get(&dep).is_some_and(|d| d.effective == Some(State::Done)) {
                    out.push(v(id, Code::BlockedByDone, format!("{file}:3: Reason: blocked by {dep}, which is DONE")));
                }
            }
        }
        // Reporting: self-acceptance (point 12).
        if l.state == State::Done {
            if let Some(r) = done_receipt {
                let dev = a.developers(id);
                for acc in r.header("Agent") {
                    if dev.iter().any(|d| agent_key(d) == agent_key(&acc)) {
                        out.push(v(id, Code::SelfAccept, format!("{TASK_DIR}/{}: acceptor {acc} is the ticket's developer", r.file)));
                    }
                }
            }
        }
    }
    out
}

/// The violations of `key` (a ticket id, or an entry's file name).
pub fn key_violations(a: &Analysis, key: &str, adrs: &BTreeSet<u16>, repo: &mut dyn Repo) -> Vec<Violation> {
    if a.tickets.contains_key(key) {
        return ticket_violations(a, key, adrs, repo);
    }
    let mut out = Vec::new();
    if let Some(r) = a.receipts.get(key).filter(|r| r.ticket.is_none() && !a.aids.historical.contains_key(key)) {
        out.push(v(key, Code::OrphanReceipt, format!("{TASK_DIR}/{}: receipt without a ticket {}.md", r.file, r.prefix)));
    }
    if a.unknown.contains(key) {
        out.push(v(
            key,
            Code::UnknownFile,
            format!("{TASK_DIR}/{key}: neither a ticket, a receipt `<ID>-STATUS-<{}>.MD` nor a listed non-ticket (ADR-037 point 9)", RECEIPT_STATES.join("|")),
        ));
    }
    out
}

/// Every key of `a`: tickets, orphan receipts, unknown entries.
pub fn all_keys(a: &Analysis) -> BTreeSet<String> {
    let mut k: BTreeSet<String> = a.tickets.keys().cloned().collect();
    k.extend(a.receipts.values().filter(|r| r.ticket.is_none()).map(|r| r.file.clone()));
    k.extend(a.unknown.iter().cloned());
    k
}

/// The `fix`/`feat` rule of point 11/13 for message `msg`: every ticket the
/// subject names must be `IMPLEMENTED` in `a`.
pub fn message_violations(a: &Analysis, msg: &str) -> Vec<Violation> {
    let s = subject(msg);
    if !is_fix_or_feat(s) {
        return Vec::new();
    }
    let mut out = Vec::new();
    for id in a.named_in(s) {
        let t = &a.tickets[&id];
        let ok = matches!(&t.parsed, Ok(l) if l.state == State::Implemented);
        if !ok {
            let now = if t.legacy {
                format!("legacy {:?}", raw_status(&t.line3))
            } else {
                t.effective.map_or("malformed".to_string(), |s| s.name().to_string())
            };
            let hint = if t.effective == Some(State::Done) {
                "; further work on a DONE ticket goes into a follow-up ticket"
            } else {
                ""
            };
            out.push(v(
                &id,
                Code::FixFeatState,
                format!(
                    "the subject is fix/feat and names {id}: line 3 of {TASK_DIR}/{id}.md must be `IMPLEMENTED (date) | Priority | Commit: self | Missing: ...` in this commit, it is {now}{hint}"
                ),
            ));
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Baseline
// ---------------------------------------------------------------------------

/// `(key, code)` pairs of a baseline file.
pub fn parse_baseline(text: &str) -> Result<BTreeSet<(String, Code)>, String> {
    let mut out = BTreeSet::new();
    for (n, l) in text.lines().enumerate() {
        if l.is_empty() || l.starts_with('#') {
            continue;
        }
        let (k, c) = l.split_once('\t').ok_or(format!("{BASELINE_FILE}:{}: expected `<key>\\t<code>`", n + 1))?;
        let code = Code::parse(c)
            .filter(|c| c.baselinable())
            .ok_or(format!("{BASELINE_FILE}:{}: unknown code {c:?}", n + 1))?;
        out.insert((k.to_string(), code));
    }
    Ok(out)
}

pub fn render_baseline(entries: &BTreeSet<(String, Code)>) -> String {
    let mut s = String::from(
        "# TOOL41 baseline of the legacy ticket violations (ADR-037 point 7): `<key>\\t<code>`.\n\
         # Written once by `cargo xtask status-check --write-baseline`; it only shrinks\n\
         # (`--prune-baseline`). An entry passes the check; a violation not listed rejects.\n",
    );
    for (k, c) in entries {
        let _ = writeln!(s, "{k}\t{}", c.name());
    }
    s
}

/// Splits `found` against `baseline`: (new violations, entries of `checked`
/// keys whose violation is gone).
pub fn against_baseline(
    found: &[Violation],
    baseline: &BTreeSet<(String, Code)>,
    checked: &BTreeSet<String>,
) -> (Vec<Violation>, Vec<(String, Code)>) {
    let have: BTreeSet<(String, Code)> = found.iter().map(|v| (v.key.clone(), v.code)).collect();
    let fresh = found
        .iter()
        .filter(|v| v.code.rejecting() && !baseline.contains(&(v.key.clone(), v.code)))
        .cloned()
        .collect();
    let stale = baseline.iter().filter(|e| checked.contains(&e.0) && !have.contains(*e)).cloned().collect();
    (fresh, stale)
}

// ---------------------------------------------------------------------------
// docs/STATUS.md (point 8) and TASK/README.md
// ---------------------------------------------------------------------------

/// What a `docs/STATUS.md` mention says about a ticket.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Claim {
    Open,
    Done,
    Parked,
    Accepted,
}

/// The `[ID](../TASK/ID.md)` mentions of `docs/STATUS.md` with a claim: in
/// `## Blocked` (parked, point 10), in a parked span (from a line with
/// `**Parked` or `Also parked` to the first line ending in `.`), or in a
/// `- **P1:**`/`**P2:**`/`**P3 / P4:**`/`**Queue` bullet — open, unless the
/// rest of the line after the mention (up to the next mention, 110
/// characters) says `done`, `parked` or `accepted`. Keyword candidates like
/// the inventory's C3, reported, never rejecting.
pub fn status_md_claims(text: &str) -> Vec<(usize, String, Claim)> {
    let mut out = Vec::new();
    let (mut section, mut bullet_open, mut parked_span) = (String::new(), false, false);
    for (n, line) in text.lines().enumerate() {
        if let Some(h) = line.strip_prefix("## ") {
            section = h.trim().to_string();
            bullet_open = false;
            parked_span = false;
            continue;
        }
        let tl = line.trim_start();
        if line.starts_with("- ") {
            bullet_open = ["- **P1:**", "- **P2:**", "- **P3 / P4:**", "- **Queue"].iter().any(|p| line.starts_with(p));
            parked_span = false;
        } else if tl.is_empty() {
            bullet_open = false;
            parked_span = false;
        }
        if line.contains("**Parked") || line.contains("Also parked") {
            parked_span = true;
        }
        let mut rest = line;
        while let Some(i) = rest.find("](../TASK/") {
            let Some(open) = rest[..i].rfind('[') else { break };
            let id = rest[open + 1..i].trim_matches('*').to_string();
            let after = &rest[i + 10..];
            let window_end = after.find("](../TASK/").unwrap_or(after.len()).min(110);
            let window: String = after.chars().take(window_end).collect::<String>().to_lowercase();
            let claim = if section == "Blocked" || parked_span {
                Some(Claim::Parked)
            } else if bullet_open {
                Some(if window.contains("done") {
                    Claim::Done
                } else if window.contains("parked") {
                    Claim::Parked
                } else if window.contains("accepted") {
                    Claim::Accepted
                } else {
                    Claim::Open
                })
            } else {
                None
            };
            if let Some(c) = claim {
                out.push((n + 1, id, c));
            }
            rest = &rest[i + 10..];
        }
        if parked_span && tl.trim_end().ends_with('.') {
            parked_span = false;
        }
    }
    out
}

fn claim_fits(c: Claim, s: State) -> bool {
    match c {
        Claim::Open => matches!(s, State::Open | State::Implemented),
        Claim::Done => s == State::Done,
        Claim::Parked => s == State::Parked,
        Claim::Accepted => s == State::Accepted,
    }
}

/// The keyword candidates of `docs/STATUS.md` against the ADR-037 tickets.
pub fn status_md_violations(a: &Analysis, status_md: &str) -> Vec<Violation> {
    let mut out = Vec::new();
    for (n, id, c) in status_md_claims(status_md) {
        let Some(t) = a.tickets.get(&id) else { continue };
        let Ok(l) = &t.parsed else { continue };
        if !claim_fits(c, l.state) {
            out.push(v(
                &id,
                Code::StatusMd,
                format!("{STATUS_FILE}:{n}: lists {id} as {c:?}, line 3 of {TASK_DIR}/{id}.md is {}", l.state.name()),
            ));
        }
    }
    out
}

/// `readme` with the `Status` column of every table whose rows start with a
/// `[ID](ID.md)` link regenerated from `cell(ID)` (`None`: kept).
pub fn readme_with_status(readme: &str, cell: &dyn Fn(&str) -> Option<String>) -> String {
    let mut out = String::new();
    let mut col: Option<usize> = None;
    for line in readme.split_inclusive('\n') {
        let body = line.trim_end_matches('\n');
        if !body.starts_with('|') {
            col = None;
            out.push_str(line);
            continue;
        }
        let cells: Vec<&str> = body.split('|').collect();
        if let Some(i) = cells.iter().position(|c| c.trim() == "Status") {
            col = Some(i);
            out.push_str(line);
            continue;
        }
        let first = cells.get(1).map_or("", |c| c.trim());
        let id = first.strip_prefix('[').and_then(|r| r.split_once("](")).map(|(id, _)| id);
        match (col, id.and_then(|id| cell(id).map(|c| (id, c)))) {
            (Some(i), Some((_, c))) if i < cells.len() - 1 => {
                let mut cells: Vec<String> = cells.iter().map(|s| s.to_string()).collect();
                cells[i] = format!(" {c} ");
                out.push_str(&cells.join("|"));
                out.push_str(&line[body.len()..]);
            }
            _ => out.push_str(line),
        }
    }
    out
}

/// The README regenerated from `a`.
pub fn generated_readme(a: &Analysis, readme: &str) -> String {
    readme_with_status(readme, &|id| a.tickets.get(id).map(|t| t.status_cell()))
}

// ---------------------------------------------------------------------------
// git helpers
// ---------------------------------------------------------------------------

fn git_out(root: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|e| format!("git {}: {e}", args.join(" ")))?;
    if !out.status.success() {
        return Err(format!("git {} failed: {}", args.join(" "), String::from_utf8_lossy(&out.stderr).trim()));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// The `TASK/` entries (and whether the baseline file is among the paths)
/// that `git diff` lists: `--cached` against `HEAD` when `staged`, else the
/// working tree against `HEAD` plus untracked files.
pub fn touched(root: &Path, staged: bool) -> Result<(BTreeSet<String>, bool), String> {
    let mut args = vec!["diff", "--name-only", "--no-renames", "-z"];
    if staged {
        args.push("--cached");
    }
    args.extend(["HEAD", "--", TASK_DIR, BASELINE_FILE]);
    let mut paths: Vec<String> = git_out(root, &args)?.split('\0').filter(|s| !s.is_empty()).map(String::from).collect();
    if !staged {
        let u = git_out(root, &["ls-files", "-z", "--others", "--exclude-standard", "--", TASK_DIR])?;
        paths.extend(u.split('\0').filter(|s| !s.is_empty()).map(String::from));
    }
    let mut entries = BTreeSet::new();
    let mut baseline = false;
    for p in paths {
        if p == BASELINE_FILE {
            baseline = true;
        } else if let Some(rest) = p.strip_prefix("TASK/") {
            entries.insert(rest.split('/').next().unwrap_or(rest).to_string());
        }
    }
    Ok((entries, baseline))
}

/// The commit that last changed line 3 of `TASK/<id>.md` (`git log -L3,3`,
/// which follows the line across renames of the file); `None` if line 3
/// differs from `HEAD`'s (the commit setting it is not made yet).
pub fn resolve_self(root: &Path, id: &str) -> Result<Option<String>, String> {
    let rel = format!("{TASK_DIR}/{id}.md");
    let disk = fs::read_to_string(root.join(&rel)).map_err(|e| format!("{rel}: {e}"))?;
    let head = git_out(root, &["show", &format!("HEAD:{rel}")]).unwrap_or_default();
    if disk.lines().nth(2) != head.lines().nth(2) {
        return Ok(None);
    }
    let out = git_out(root, &["log", "-n1", "--format=%H", "--no-patch", &format!("-L3,3:{rel}")])?;
    Ok(out.lines().next().map(|s| s.to_string()))
}

/// The subjects of the history (`git log --format=%s`).
fn history_subjects(root: &Path) -> Result<Vec<String>, String> {
    Ok(git_out(root, &["log", "--format=%s", "HEAD"])?.lines().map(String::from).collect())
}

fn adrs_of(view: &mut View) -> Result<BTreeSet<u16>, String> {
    Ok(view.read("docs/MEMORY.md")?.map(|m| crate::doc_check::adr_headings(&m)).unwrap_or_default())
}

// ---------------------------------------------------------------------------
// status-check
// ---------------------------------------------------------------------------

/// The result of one check run.
#[derive(Debug, Default)]
pub struct CheckResult {
    /// Rejecting, not in the baseline.
    pub rejected: Vec<Violation>,
    /// Reporting (never fail).
    pub reported: Vec<Violation>,
    /// Rejecting but in the baseline.
    pub legacy: usize,
    /// The keys checked.
    pub checked: usize,
}

/// Options of [`check`].
#[derive(Debug, Default, Clone)]
pub struct CheckOpts {
    pub staged: bool,
    pub new_only: bool,
    /// The commit message (`--message-file`): the keys checked are the ids its
    /// subject names, plus the fix/feat rule.
    pub message: Option<String>,
    /// Scan the history for `fix`/`feat` commits (full runs only).
    pub history: bool,
}

/// `status-check`: see the module doc.
pub fn check(root: &Path, o: &CheckOpts) -> Result<CheckResult, String> {
    let mut view = if o.staged { View::index(root)? } else { View::disk(root) };
    let a = Analysis::load(&mut view)?;
    let adrs = adrs_of(&mut view)?;
    let mut repo = GitRepo(root.to_path_buf());
    let baseline_text = view.read(BASELINE_FILE)?;
    let baseline = parse_baseline(baseline_text.as_deref().unwrap_or(""))?;
    let head_baseline = git_out(root, &["show", &format!("HEAD:{BASELINE_FILE}")]).ok();
    let mut r = CheckResult::default();

    let (keys, baseline_touched): (BTreeSet<String>, bool) = if let Some(msg) = &o.message {
        (a.named_in(subject(msg)), false)
    } else if o.new_only {
        let (entries, b) = touched(root, o.staged)?;
        (entries.iter().map(|e| a.key_of(e)).collect(), b)
    } else {
        (all_keys(&a), true)
    };
    let mut found = Vec::new();
    for k in &keys {
        found.extend(key_violations(&a, k, &adrs, &mut repo));
    }
    if let Some(msg) = &o.message {
        found.extend(message_violations(&a, msg));
    }
    let (fresh, stale) = against_baseline(&found, &baseline, &keys);
    r.checked = keys.len();
    r.legacy = found.iter().filter(|v| v.code.rejecting()).count() - fresh.len();
    r.rejected = fresh;
    for (k, c) in stale {
        r.rejected.push(v(
            &k,
            Code::Baseline,
            format!("{BASELINE_FILE}: `{k}\t{}` is fixed; remove it (`cargo xtask status-check --prune-baseline`)", c.name()),
        ));
    }
    // The baseline only shrinks: no entry HEAD lacks.
    if baseline_touched {
        if let Some(head) = head_baseline.as_deref() {
            let head = parse_baseline(head).unwrap_or_default();
            for (k, c) in baseline.difference(&head) {
                r.rejected.push(v(
                    k,
                    Code::Baseline,
                    format!("{BASELINE_FILE}: `{k}\t{}` is not in HEAD's baseline; the baseline only shrinks", c.name()),
                ));
            }
        }
    }
    r.reported.extend(found.into_iter().filter(|v| !v.code.rejecting()));
    if o.message.is_none() && !o.new_only {
        if let Some(s) = view.read(STATUS_FILE)? {
            r.reported.extend(status_md_violations(&a, &s));
        }
        if let Some(readme) = view.read(README_FILE)? {
            if generated_readme(&a, &readme) != readme {
                r.reported.push(v(
                    "README",
                    Code::ReadmeStale,
                    format!("{README_FILE}: the status column differs from line 3 (`cargo xtask report --readme`)"),
                ));
            }
        }
        if o.history {
            r.reported.extend(history_violations(&a, &history_subjects(root)?));
        }
    }
    Ok(r)
}

/// By the history: an `OPEN`/`PARKED` ticket (ADR-037 or mapped legacy)
/// that a `fix`/`feat` subject names.
pub fn history_violations(a: &Analysis, subjects: &[String]) -> Vec<Violation> {
    let fixes: Vec<&String> = subjects.iter().filter(|s| is_fix_or_feat(s)).collect();
    let mut out = Vec::new();
    for (id, t) in &a.tickets {
        if !matches!(t.effective, Some(State::Open | State::Parked)) {
            continue;
        }
        let n = fixes.iter().filter(|s| names(s, id)).count();
        if n > 0 {
            out.push(v(
                id,
                Code::FixCommitNotClosed,
                format!("{TASK_DIR}/{id}.md is {} and {n} fix/feat commit(s) name it", t.effective.unwrap().name()),
            ));
        }
    }
    out
}

fn print_result(mode: &str, r: &CheckResult) {
    for x in &r.rejected {
        println!("{mode}: REJECT [{}] {}", x.code.name(), x.msg);
    }
    for x in &r.reported {
        println!("{mode}: report [{}] {}", x.code.name(), x.msg);
    }
    println!(
        "{mode}: {} checked, {} rejected, {} reported, {} legacy (in {BASELINE_FILE})",
        r.checked,
        r.rejected.len(),
        r.reported.len(),
        r.legacy
    );
}

/// `cargo xtask status-check [--staged] [--new-only] [--message-file <f>]
/// [--write-baseline | --prune-baseline]`.
pub fn cmd_status_check(root: &Path, args: &[String]) -> Result<(), String> {
    let mut o = CheckOpts { history: true, ..Default::default() };
    let (mut write, mut prune) = (false, false);
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--staged" => o.staged = true,
            "--new-only" => o.new_only = true,
            "--message-file" => {
                let f = it.next().ok_or("status-check: --message-file needs a file")?;
                o.message = Some(fs::read_to_string(f).map_err(|e| format!("status-check: {f}: {e}"))?);
            }
            "--write-baseline" => write = true,
            "--prune-baseline" => prune = true,
            other => return Err(format!("status-check: unknown argument {other:?}")),
        }
    }
    if o.staged || o.new_only || o.message.is_some() {
        o.history = false;
    }
    if write || prune {
        if o.staged || o.new_only || o.message.is_some() || (write && prune) {
            return Err("status-check: --write-baseline/--prune-baseline take no other option".into());
        }
        return baseline_cmd(root, write);
    }
    let mode = if o.message.is_some() {
        "status-check (commit-msg)"
    } else if o.staged {
        "status-check --staged"
    } else {
        "status-check"
    };
    let r = check(root, &o)?;
    print_result(mode, &r);
    if r.rejected.is_empty() {
        Ok(())
    } else {
        Err(format!("status-check: {} rejected (above)", r.rejected.len()))
    }
}

/// The baselinable violations of the working tree.
fn tree_baselinable(root: &Path) -> Result<BTreeSet<(String, Code)>, String> {
    let mut view = View::disk(root);
    let a = Analysis::load(&mut view)?;
    let adrs = adrs_of(&mut view)?;
    let mut repo = GitRepo(root.to_path_buf());
    let mut out = BTreeSet::new();
    for k in all_keys(&a) {
        for x in key_violations(&a, &k, &adrs, &mut repo) {
            if x.code.baselinable() {
                out.insert((x.key, x.code));
            }
        }
    }
    Ok(out)
}

fn baseline_cmd(root: &Path, write: bool) -> Result<(), String> {
    let path = root.join(BASELINE_FILE);
    let now = tree_baselinable(root)?;
    if write {
        if path.exists() {
            return Err(format!("status-check: {BASELINE_FILE} exists; it only shrinks (--prune-baseline)"));
        }
        if let Some(d) = path.parent() {
            fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
        }
        fs::write(&path, render_baseline(&now)).map_err(|e| format!("{}: {e}", path.display()))?;
        println!("status-check: wrote {BASELINE_FILE} ({} entries)", now.len());
        return Ok(());
    }
    let n = prune_baseline(root, None)?;
    println!("status-check: pruned {n} entries from {BASELINE_FILE}");
    Ok(())
}

/// Removes the baseline entries (of `only`, or all) whose violation the
/// working tree no longer has; returns how many.
pub fn prune_baseline(root: &Path, only: Option<&str>) -> Result<usize, String> {
    let path = root.join(BASELINE_FILE);
    let Ok(text) = fs::read_to_string(&path) else { return Ok(0) };
    let old = parse_baseline(&text)?;
    let now = tree_baselinable(root)?;
    let keep: BTreeSet<(String, Code)> =
        old.iter().filter(|e| now.contains(*e) || only.is_some_and(|k| e.0 != k)).cloned().collect();
    let n = old.len() - keep.len();
    if n > 0 {
        fs::write(&path, render_baseline(&keep)).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(n)
}

// ---------------------------------------------------------------------------
// Writing line 3
// ---------------------------------------------------------------------------

/// `text` with line 3 replaced by `line3`.
pub fn with_line3(text: &str, line3: &str) -> Result<String, String> {
    let mut lines: Vec<&str> = text.split_inclusive('\n').collect();
    if lines.len() < 3 {
        return Err("the ticket has no line 3".into());
    }
    let nl = if lines[2].ends_with('\n') { "\n" } else { "" };
    let new = format!("{line3}{nl}");
    lines[2] = &new;
    Ok(lines.concat())
}

/// `text` with `entry` appended under a final `## Status history` section.
pub fn with_history(text: &str, entry: &str) -> String {
    let mut s = text.trim_end_matches('\n').to_string();
    if !s.lines().any(|l| l == "## Status history") {
        s.push_str("\n\n## Status history\n");
    }
    let _ = write!(s, "\n- {entry}");
    s.push('\n');
    s
}

/// Today, local time, `YYYY-MM-DD`.
pub fn today() -> Result<String, String> {
    let out = Command::new("date").arg("+%Y-%m-%d").output().map_err(|e| format!("date: {e}"))?;
    let d = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if is_date(&d) {
        Ok(d)
    } else {
        Err(format!("date printed {d:?}"))
    }
}

fn check_value_text(what: &str, s: &str) -> Result<(), String> {
    if s.trim().is_empty() || s.trim() != s || s.contains('|') || !s.is_ascii() || s.contains('\n') {
        return Err(format!("{what}: a non-empty ASCII value on one line, without `|` or surrounding blanks: {s:?}"));
    }
    Ok(())
}

/// Updates the README rows of `id` (only those) in the working tree;
/// returns whether the file changed.
fn refresh_readme(root: &Path, id: &str) -> Result<bool, String> {
    let path = root.join(README_FILE);
    let Ok(readme) = fs::read_to_string(&path) else { return Ok(false) };
    let text = fs::read_to_string(root.join(TASK_DIR).join(format!("{id}.md"))).map_err(|e| format!("{id}: {e}"))?;
    let cell = Ticket::from_text(id, &text).status_cell();
    let new = readme_with_status(&readme, &|row| (row == id).then(|| cell.clone()));
    if new == readme {
        return Ok(false);
    }
    fs::write(&path, new).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(true)
}

/// `cargo xtask set-status <ID> PARKED|OBSOLETE|ACCEPTED|OPEN ...` (lead
/// only, point 11, decision f).
pub fn cmd_set_status(root: &Path, args: &[String]) -> Result<(), String> {
    set_status(root, args, &today()?)
}

pub fn set_status(root: &Path, args: &[String], date: &str) -> Result<(), String> {
    let usage = "usage: cargo xtask set-status <ID> PARKED --reason R --reopen T | OBSOLETE --reason R [--target ID] \
                 | ACCEPTED --ref ADR-NNN | OPEN --reason R  [--priority Pn]";
    let (Some(id), Some(st)) = (args.first(), args.get(1)) else { return Err(usage.into()) };
    let state = match State::parse(st) {
        Some(s @ (State::Parked | State::Obsolete | State::Accepted | State::Open)) => s,
        Some(s) => {
            return Err(format!(
                "set-status: {} is not set by hand: IMPLEMENTED by the developer's commit, DONE only by `cargo xtask accept` (ADR-037 point 11)",
                s.name()
            ))
        }
        None => return Err(format!("set-status: unknown state {st:?}; {usage}")),
    };
    let mut opt: BTreeMap<&str, String> = BTreeMap::new();
    let mut it = args[2..].iter();
    while let Some(a) = it.next() {
        let key = match a.as_str() {
            "--reason" => "Reason",
            "--reopen" => "Reopen",
            "--target" => "Target",
            "--ref" => "Ref",
            "--priority" => "Priority",
            other => return Err(format!("set-status: unknown argument {other:?}; {usage}")),
        };
        let val = it.next().ok_or(format!("set-status: {a} needs a value"))?;
        check_value_text(a, val)?;
        opt.insert(key, val.clone());
    }
    let need: &[&str] = match state {
        State::Parked => &["Reason", "Reopen"],
        State::Obsolete => &["Reason"],
        State::Accepted => &["Ref"],
        _ => &["Reason"],
    };
    let allowed: &[&str] = match state {
        State::Obsolete => &["Reason", "Target", "Priority"],
        State::Accepted => &["Ref", "Priority"],
        State::Parked => &["Reason", "Reopen", "Priority"],
        _ => &["Reason", "Priority"],
    };
    for n in need {
        if !opt.contains_key(n) {
            return Err(format!("set-status: {} needs --{} (ADR-037 point 1)", state.name(), n.to_lowercase()));
        }
    }
    if let Some(k) = opt.keys().find(|k| !allowed.contains(k)) {
        return Err(format!("set-status: --{} does not apply to {}", k.to_lowercase(), state.name()));
    }
    let rel = format!("{TASK_DIR}/{id}.md");
    let path = root.join(&rel);
    let text = fs::read_to_string(&path).map_err(|e| format!("set-status: {rel}: {e}"))?;
    let t = Ticket::from_text(id, &text);
    if t.effective == Some(State::Done) {
        return Err(format!("set-status: {id} is DONE; further work goes into a follow-up ticket (ADR-037 point 4)"));
    }
    if state == State::Open && t.effective != Some(State::Parked) {
        return Err(format!(
            "set-status: OPEN is for a PARKED ticket after an owner decision (decision f); {id} is {}",
            t.effective.map_or("unmapped legacy".into(), |s| s.name().to_string())
        ));
    }
    let priority = match opt.get("Priority") {
        Some(p) => match p.strip_prefix('P').and_then(|d| d.parse::<u8>().ok()).filter(|d| (1..=4).contains(d)) {
            Some(d) => d,
            None => return Err(format!("set-status: --priority expects P1..P4, found {p:?}")),
        },
        None => match &t.parsed {
            Ok(l) => l.priority,
            Err(_) => legacy_priority(&t.line3)
                .ok_or(format!("set-status: line 3 of {rel} has no readable priority; give --priority Pn"))?,
        },
    };
    let mut fields = Vec::new();
    for k in state.required().iter().chain(state.optional()) {
        if let Some(v) = opt.get(k) {
            fields.push((k.to_string(), v.clone()));
        }
    }
    let line = Line3 { state, date: date.to_string(), priority, fields };
    let rendered = line.render();
    parse_line3(id, &rendered).map_err(|e| format!("set-status: the new line 3 does not parse: {:?}", e.errors))?;
    let mut new = with_line3(&text, &rendered)?;
    if state == State::Open {
        new = with_history(&new, &format!("{date}: reopened (was {:?}): {}", t.line3, opt["Reason"]));
    } else {
        new = with_history(&new, &format!("{date}: set to {} by `cargo xtask set-status` (was {:?})", state.name(), t.line3));
    }
    if let Some(tg) = opt.get("Target") {
        if !root.join(TASK_DIR).join(format!("{tg}.md")).exists() {
            return Err(format!("set-status: --target {tg}: no ticket {TASK_DIR}/{tg}.md"));
        }
    }
    fs::write(&path, new).map_err(|e| format!("set-status: {rel}: {e}"))?;
    let pruned = prune_baseline(root, Some(id))?;
    let readme = refresh_readme(root, id)?;
    println!("set-status: {rel}: {rendered}");
    if pruned > 0 {
        println!("set-status: removed {pruned} fixed entries of {id} from {BASELINE_FILE}");
    }
    if readme {
        println!("set-status: regenerated the status column of {README_FILE}");
    }
    println!("set-status: not committed; the lead commits (git add {rel} ...).");
    Ok(())
}

// ---------------------------------------------------------------------------
// accept
// ---------------------------------------------------------------------------

/// The trailer that ends the message of the commit `accept` makes, as it ends
/// every other commit of this repository (ADR-037 decision h, owner
/// 2026-11-11). Its own last paragraph, for PASS and FAIL.
pub const ACCEPT_TRAILER: &str = "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>";

/// `cargo xtask accept <ID> --commit <hash> --result PASS|FAIL [--note ...]
/// --agent <name>` (tester, point 11, decision e).
pub fn cmd_accept(root: &Path, args: &[String]) -> Result<(), String> {
    accept(root, args, &today()?)
}

pub fn accept(root: &Path, args: &[String], date: &str) -> Result<(), String> {
    let usage = "usage: cargo xtask accept <ID> --commit <hash> --result PASS|FAIL [--note <text>] --agent <name>";
    let id = args.first().filter(|a| !a.starts_with('-')).ok_or(usage)?;
    let (mut commit, mut result, mut note, mut agent) = (None, None, None, None);
    let mut it = args[1..].iter();
    while let Some(a) = it.next() {
        let val = it.next().ok_or(format!("accept: {a} needs a value; {usage}"))?.clone();
        match a.as_str() {
            "--commit" => commit = Some(val),
            "--result" => result = Some(val),
            "--note" => note = Some(val),
            "--agent" => agent = Some(val),
            other => return Err(format!("accept: unknown argument {other:?}; {usage}")),
        }
    }
    let (Some(commit), Some(result), Some(agent)) = (commit, result, agent) else { return Err(usage.into()) };
    let pass = match result.as_str() {
        "PASS" => true,
        "FAIL" => false,
        r => return Err(format!("accept: --result PASS or FAIL, found {r:?}")),
    };
    check_value_text("--agent", &agent)?;

    // Nothing else may be in the commit.
    let staged = git_out(root, &["diff", "--cached", "--name-only"])?;
    if !staged.trim().is_empty() {
        return Err(format!("accept: other changes are staged; it commits only its own:\n{}", staged.trim_end()));
    }
    let rel = format!("{TASK_DIR}/{id}.md");
    let done_rel = format!("{TASK_DIR}/{id}-STATUS-DONE.MD");
    let unf_rel = format!("{TASK_DIR}/{id}-STATUS-UNFINISHED.MD");
    let mut own = vec![rel.clone(), if pass { done_rel.clone() } else { unf_rel.clone() }, BASELINE_FILE.to_string()];
    let readme_names_it = fs::read_to_string(root.join(README_FILE)).is_ok_and(|r| r.contains(&format!("[{id}]({id}.md)")));
    if pass && readme_names_it {
        own.push(README_FILE.to_string());
    }
    let mut st_args = vec!["status", "--porcelain", "--"];
    st_args.extend(own.iter().map(|s| s.as_str()));
    let dirty = git_out(root, &st_args)?;
    if !dirty.trim().is_empty() {
        return Err(format!("accept: uncommitted changes in the files it writes:\n{}", dirty.trim_end()));
    }

    let text = fs::read_to_string(root.join(&rel)).map_err(|e| format!("accept: {rel}: {e}"))?;
    let t = Ticket::from_text(id, &text);
    let line = match &t.parsed {
        Ok(l) if l.state == State::Implemented => l.clone(),
        Ok(l) => return Err(format!("accept: line 3 of {rel} is {}, not IMPLEMENTED", l.state.name())),
        Err(e) => return Err(format!("accept: line 3 of {rel} is not IMPLEMENTED in ADR-037 form: {:?}", e.errors)),
    };
    // The commit: on the current history, its message names the id.
    let full = git_out(root, &["rev-parse", "--verify", "--quiet", &format!("{commit}^{{commit}}")])
        .map_err(|_| format!("accept: --commit {commit}: no such commit"))?;
    let full = full.trim().to_string();
    if Command::new("git")
        .args(["merge-base", "--is-ancestor", &full, "HEAD"])
        .current_dir(root)
        .status()
        .map_or(true, |s| !s.success())
    {
        return Err(format!("accept: --commit {commit} is not on the current history (an ancestor of HEAD)"));
    }
    let msg = git_out(root, &["log", "-n1", "--format=%B", &full])?;
    if !names(&msg, id) {
        return Err(format!("accept: the message of {commit} does not name {id}"));
    }
    let short = git_out(root, &["rev-parse", "--short=9", &full])?.trim().to_string();
    // The acceptor is not the developer (point 12).
    let mut view = View::disk(root);
    let a = Analysis::load(&mut view)?;
    let devs = a.developers(id);
    if devs.is_empty() {
        return Err(format!(
            "accept: no developer recorded for {id}: a receipt {TASK_DIR}/{id}-STATUS-*.MD needs `Role: developer` and `Agent: <name>` in its header (before the first `## `)"
        ));
    }
    if devs.iter().any(|d| agent_key(d) == agent_key(&agent)) {
        return Err(format!("accept: {agent} is the developer recorded for {id}; acceptance needs another agent (ADR-037 point 12)"));
    }
    let note_text = note.clone().unwrap_or_else(|| "No note.".into());
    let what = if pass { "PASS" } else { "FAIL" };
    if pass {
        if root.join(&done_rel).exists() {
            return Err(format!("accept: {done_rel} exists already"));
        }
        let receipt = format!(
            "# {id} — receipt DONE ({date})\n\nRole: tester\nAgent: {agent}\nCommit: {short}\nResult: PASS\n\n\
             Written by `cargo xtask accept` (ADR-037 point 11). Developer: {}. Line 3 before: `{}`.\n\n## Note\n\n{note_text}\n",
            devs.iter().cloned().collect::<Vec<_>>().join(", "),
            t.line3
        );
        fs::write(root.join(&done_rel), receipt).map_err(|e| format!("accept: {done_rel}: {e}"))?;
        let done = Line3 {
            state: State::Done,
            date: date.to_string(),
            priority: line.priority,
            fields: vec![("Commit".into(), short.clone()), ("Receipt".into(), format!("{id}-STATUS-DONE.MD"))],
        };
        fs::write(root.join(&rel), with_line3(&text, &done.render())?).map_err(|e| format!("accept: {rel}: {e}"))?;
    } else {
        let path = root.join(&unf_rel);
        let old = fs::read_to_string(&path).unwrap_or_else(|_| format!("# {id} — receipt UNFINISHED (acceptance FAIL)\n"));
        let section = format!(
            "\n## Acceptance {date} — FAIL\n\nRole: tester\nAgent: {agent}\nCommit: {short}\nResult: FAIL\n\n\
             Written by `cargo xtask accept` (ADR-037 point 11); line 3 stays IMPLEMENTED.\n\n{note_text}\n"
        );
        fs::write(&path, format!("{}\n{section}", old.trim_end_matches('\n'))).map_err(|e| format!("accept: {unf_rel}: {e}"))?;
    }
    prune_baseline(root, Some(id))?;
    if pass {
        refresh_readme(root, id)?;
    }
    let mut add = vec!["add", "--"];
    let existing: Vec<String> = own.iter().filter(|p| root.join(p).exists()).cloned().collect();
    add.extend(existing.iter().map(|s| s.as_str()));
    git_out(root, &add)?;
    let subject = format!("docs(task): accept {id}, {what} on {short} ({id})");
    let body = format!(
        "{subject}\n\nGenerated by `cargo xtask accept` (ADR-037 point 11, decision e).\nRole: tester, agent: {agent}. Result: {what}.\n{}\n\n{ACCEPT_TRAILER}\n",
        if pass {
            format!("Line 3 is DONE; receipt {id}-STATUS-DONE.MD.")
        } else {
            format!("Line 3 stays IMPLEMENTED; the note is in {id}-STATUS-UNFINISHED.MD.")
        }
    );
    // The message goes through a file in the git directory, outside the
    // work tree; removed right after.
    let gitdir = git_out(root, &["rev-parse", "--git-dir"])?;
    let msgfile = {
        let g = PathBuf::from(gitdir.trim());
        let g = if g.is_absolute() { g } else { root.join(g) };
        g.join("ACCEPT_MSG")
    };
    fs::write(&msgfile, body).map_err(|e| format!("accept: {}: {e}", msgfile.display()))?;
    let st = Command::new("git")
        .args(["commit", "-q", "-F"])
        .arg(&msgfile)
        .current_dir(root)
        .status()
        .map_err(|e| format!("accept: git commit: {e}"))?;
    let _ = fs::remove_file(&msgfile);
    if !st.success() {
        return Err("accept: git commit failed (the hooks' output above); the files are written and staged".into());
    }
    let head = git_out(root, &["rev-parse", "--short=9", "HEAD"])?;
    println!("accept: {id} {what} on {short}, committed {}", head.trim());
    // TOOL46: the scratch of the recorded agents goes; the acceptor's own
    // directory is still in use by this run and expires 72 h after its stop.
    print!("{}", crate::scratch::clean_ticket(root, id, Some(&agent)));
    Ok(())
}

// ---------------------------------------------------------------------------
// report
// ---------------------------------------------------------------------------

/// The counts of `report`.
#[derive(Debug, Default)]
pub struct Counts {
    pub tickets: usize,
    pub adr037: BTreeMap<State, usize>,
    pub malformed: usize,
    pub legacy: BTreeMap<Option<State>, usize>,
    pub per_code: BTreeMap<Code, usize>,
    pub per_group: BTreeMap<&'static str, BTreeSet<String>>,
}

pub fn counts(a: &Analysis, found: &[Violation]) -> Counts {
    let mut c = Counts { tickets: a.tickets.len(), ..Default::default() };
    for t in a.tickets.values() {
        match (&t.parsed, t.legacy) {
            (Ok(l), _) => *c.adr037.entry(l.state).or_default() += 1,
            (Err(_), false) => c.malformed += 1,
            (Err(_), true) => *c.legacy.entry(t.effective).or_default() += 1,
        }
    }
    for x in found {
        *c.per_code.entry(x.code).or_default() += 1;
        if x.code.group() != "-" {
            c.per_group.entry(x.code.group()).or_default().insert(x.key.clone());
        }
    }
    c
}

/// The gap groups of [`Code::group`], in report order.
pub const GROUPS: [&str; 6] = [
    "done-not-closed",
    "closed-without-evidence",
    "status-md-mismatch",
    "parked-without-fields",
    "not-adr037-form",
    "dangling-reference",
];

pub fn render_report(c: &Counts) -> String {
    let mut s = String::new();
    let adr: usize = c.adr037.values().sum();
    let leg: usize = c.legacy.values().sum();
    let _ = writeln!(s, "report: {} tickets in {TASK_DIR}/: {adr} ADR-037 line 3, {} malformed, {leg} legacy", c.tickets, c.malformed);
    let _ = writeln!(s, "  {:<12} {:>8} {:>8}", "state", "ADR-037", "legacy");
    for st in STATES {
        let _ = writeln!(s, "  {:<12} {:>8} {:>8}", st.name(), c.adr037.get(&st).unwrap_or(&0), c.legacy.get(&Some(st)).unwrap_or(&0));
    }
    let _ = writeln!(s, "  {:<12} {:>8} {:>8}", "unmapped", "-", c.legacy.get(&None).unwrap_or(&0));
    let _ = writeln!(s, "  gap groups (tickets/files; machine rules, not the inventory's classes C1-C6):");
    for g in GROUPS {
        let _ = writeln!(s, "    {g:<26} {:>5}", c.per_group.get(g).map_or(0, |k| k.len()));
    }
    let _ = writeln!(s, "  violations by code (R = rejecting, r = reporting):");
    for (code, n) in &c.per_code {
        let _ = writeln!(s, "    {} {:<22} {:>5}", if code.rejecting() { 'R' } else { 'r' }, code.name(), n);
    }
    s
}

pub fn render_summary(c: &Counts, rejected: usize, legacy: usize, reported: usize) -> String {
    let st: Vec<String> = STATES
        .iter()
        .filter_map(|s| c.adr037.get(s).filter(|n| **n > 0).map(|n| format!("{} {n}", s.name())))
        .collect();
    format!(
        "tickets {} | ADR-037 {} ({}) | legacy {} | gaps: {rejected} new, {legacy} legacy, {reported} reported",
        c.tickets,
        c.adr037.values().sum::<usize>(),
        st.join(", "),
        c.legacy.values().sum::<usize>()
    )
}

/// `cargo xtask report [--summary | --readme [--check]]`.
pub fn cmd_report(root: &Path, args: &[String]) -> Result<(), String> {
    let (mut summary, mut readme, mut check_only) = (false, false, false);
    for a in args {
        match a.as_str() {
            "--summary" => summary = true,
            "--readme" => readme = true,
            "--check" => check_only = true,
            other => return Err(format!("report: unknown argument {other:?}")),
        }
    }
    if check_only && !readme {
        return Err("report: --check goes with --readme".into());
    }
    let mut view = View::disk(root);
    let a = Analysis::load(&mut view)?;
    if readme {
        let path = root.join(README_FILE);
        let old = fs::read_to_string(&path).map_err(|e| format!("report: {README_FILE}: {e}"))?;
        let new = generated_readme(&a, &old);
        if check_only {
            return if new == old { Ok(()) } else { Err(format!("report: {README_FILE} status column is stale (`cargo xtask report --readme`)")) };
        }
        if new != old {
            fs::write(&path, new).map_err(|e| format!("report: {README_FILE}: {e}"))?;
            println!("report: regenerated the status column of {README_FILE}");
        } else {
            println!("report: {README_FILE} is up to date");
        }
        return Ok(());
    }
    let o = CheckOpts { history: true, ..Default::default() };
    let r = check(root, &o)?;
    let adrs = adrs_of(&mut view)?;
    let mut repo = GitRepo(root.to_path_buf());
    let mut found = Vec::new();
    for k in all_keys(&a) {
        found.extend(key_violations(&a, &k, &adrs, &mut repo));
    }
    found.extend(r.reported.iter().filter(|x| !matches!(x.code, Code::BlockedByDone | Code::SelfAccept)).cloned());
    let c = counts(&a, &found);
    if summary {
        println!("{}", render_summary(&c, r.rejected.len(), r.legacy, r.reported.len()));
        return Ok(());
    }
    print!("{}", render_report(&c));
    println!("  status-check: {} rejected, {} legacy (baseline), {} reported", r.rejected.len(), r.legacy, r.reported.len());
    println!("  ADR-037 tickets:");
    for t in a.tickets.values().filter(|t| !t.legacy) {
        let mut line = format!("    {:<10} {}", t.id, t.line3);
        if let Ok(l) = &t.parsed {
            if l.get("Commit") == Some("self") {
                match resolve_self(root, &t.id)? {
                    Some(h) => {
                        let _ = write!(line, "  [self = {}]", &h[..h.len().min(9)]);
                    }
                    None => line.push_str("  [self = not committed yet]"),
                }
            }
        }
        println!("{line}");
    }
    Ok(())
}

/// The comparison key of an agent name: trimmed, lowercase, without a
/// leading `agent-` (a worktree is named `agent-<id>`, the hook's id is
/// `<id>`). One function for the self-acceptance checks of `accept` and
/// `status-check` and for the scratch clean-up (`scratch.rs`; TOOL46).
pub fn agent_key(name: &str) -> String {
    let n = name.trim().to_lowercase();
    n.strip_prefix("agent-").map(str::to_string).unwrap_or(n)
}

#[cfg(test)]
mod tests;

/// TOOL54 — `migrate --batch <N>`, the one-time migration of legacy tickets
/// (ADR-037 addendum (b), points 15–17).
pub mod migrate;

// ---------------------------------------------------------------------------
// TOOL54 rework (owner 2026-11-11): old hashes, migrated evidence, historical
// receipts
// ---------------------------------------------------------------------------

/// The map of the 2026-10-19 history rewrite (`tools/oldhash.sh`).
pub const COMMIT_MAP_FILE: &str = "docs/commit-map.txt";
/// The owner-approved list of receipts kept although they have no ticket
/// (LEGACY01 Q7): line 1 the approval line of the batch files, then rows
/// `<receipt file><TAB><reason>`; `#` lines are comments, a header row
/// `receipt<TAB>reason` is skipped. The files stay where they are.
pub const HISTORICAL_FILE: &str = "TASK/migration/historical-receipts.tsv";

/// `docs/commit-map.txt`: `old new` full hashes per line, `#` comments.
#[derive(Debug, Default, Clone)]
pub struct CommitMap {
    pub pairs: Vec<(String, String)>,
}

impl CommitMap {
    pub fn parse(text: &str) -> CommitMap {
        let pairs = text
            .lines()
            .filter(|l| !l.starts_with('#'))
            .filter_map(|l| {
                let mut w = l.split_whitespace();
                Some((w.next()?.to_string(), w.next()?.to_string()))
            })
            .collect();
        CommitMap { pairs }
    }

    /// The new hashes of the old hash (prefix, 7+ digits) `old`, as
    /// `tools/oldhash.sh` prints them.
    pub fn translate<'a>(&'a self, old: &'a str) -> impl Iterator<Item = &'a str> + 'a {
        self.pairs.iter().filter(move |(o, _)| old.len() >= 7 && o.starts_with(old)).map(|(_, n)| n.as_str())
    }
}

/// `text` names commit `hash`: directly ([`names_commit`]), or by an old
/// hash that `map` translates to it.
pub fn names_commit_mapped(text: &str, hash: &str, map: &CommitMap) -> bool {
    names_commit(text, hash)
        || hex_tokens(text).iter().any(|t| map.translate(t).any(|n| n.starts_with(hash) || hash.starts_with(n)))
}

/// The one rule for "the evidence of a DONE names its commit", used by
/// `status-check` (the `done-no-receipt` gap) and `migrate`: the receipt
/// text, or one of the ticket's `Migrated ...` lines, names `hash`, old
/// hashes translated by `map`. Receipts are never rewritten (owner
/// 2026-11-11).
pub fn evidence_names_commit(map: &CommitMap, receipt: &str, migrated: &[String], hash: &str) -> bool {
    !hash.is_empty()
        && (names_commit_mapped(receipt, hash, map) || migrated.iter().any(|m| names_commit_mapped(m, hash, map)))
}

/// The `Migrated (owner batch N, ...), evidence: ...` lines of a ticket text.
pub fn migrated_lines(text: &str) -> Vec<String> {
    text.lines().filter(|l| l.starts_with("Migrated (owner batch ")).map(String::from).collect()
}

/// Parses [`HISTORICAL_FILE`]: receipt file name to reason.
pub fn parse_historical(text: &str) -> Result<BTreeMap<String, String>, String> {
    let mut lines = text.lines();
    let first = lines.next().unwrap_or("");
    if !migrate::is_approval(first) {
        return Err(format!(
            "{HISTORICAL_FILE}:1: line 1 must be the owner's approval line `{}YYYY-MM-DD [note]`, found {first:?}",
            migrate::APPROVAL_PREFIX
        ));
    }
    let mut out = BTreeMap::new();
    for (i, l) in lines.enumerate() {
        let n = i + 2;
        if l.trim().is_empty() || l.starts_with('#') || l == "receipt\treason" {
            continue;
        }
        let (file, reason) = l
            .split_once('\t')
            .filter(|(_, r)| !r.trim().is_empty() && !r.contains('\t'))
            .ok_or(format!("{HISTORICAL_FILE}:{n}: expected `<receipt file><TAB><reason>`"))?;
        if !receipt_name(file).is_some_and(|(p, st)| !p.is_empty() && RECEIPT_STATES.contains(&st)) {
            return Err(format!("{HISTORICAL_FILE}:{n}: {file:?} is no receipt name `<ID>-STATUS-<{}>.MD`", RECEIPT_STATES.join("|")));
        }
        if out.insert(file.to_string(), reason.to_string()).is_some() {
            return Err(format!("{HISTORICAL_FILE}:{n}: {file} stands twice"));
        }
    }
    Ok(out)
}

/// What [`Analysis`] reads beside `TASK/`'s tickets and receipts.
#[derive(Debug, Default, Clone)]
pub struct Aids {
    pub map: CommitMap,
    /// [`HISTORICAL_FILE`]: an orphan receipt listed here is no violation.
    pub historical: BTreeMap<String, String>,
}

impl Aids {
    pub fn load(view: &mut View) -> Result<Aids, String> {
        let map = view.read(COMMIT_MAP_FILE)?.map(|t| CommitMap::parse(&t)).unwrap_or_default();
        let historical = match view.read(HISTORICAL_FILE)? {
            Some(t) => parse_historical(&t)?,
            None => BTreeMap::new(),
        };
        Ok(Aids { map, historical })
    }
}
