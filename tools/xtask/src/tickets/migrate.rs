//! TOOL54 — `cargo xtask migrate --batch <N>`: the one-time migration of
//! legacy tickets (`docs/MEMORY.md`, ADR-037 addendum (b) of 2026-11-11,
//! points 15–17).
//!
//! # The batch file
//!
//! `TASK/migration/batch-<N>.tsv` (`<N>` decimal digits), committed before the
//! run and unchanged in the working tree; `migrate` reads it from `HEAD`.
//!
//! - **Line 1** is the owner's approval line, exactly
//!   `# approved: owner YYYY-MM-DD` optionally followed by a blank and a note
//!   (`# approved: owner 2026-11-12 batch of the closed C1 tickets`). Like the
//!   roles of point 12 it is a record, not a proof.
//! - Further lines starting with `#` are comments; empty lines are skipped; a
//!   header row `id<TAB>state<TAB>fields<TAB>evidence` is skipped.
//! - Every other line is a row of four tab-separated columns:
//!   1. `id` — the ticket `TASK/<id>.md`;
//!   2. the target state, one of the six of point 1;
//!   3. the fields in line-3 syntax, in the order of point 1, separated by
//!      ` | `: `Priority: Pn | Key: value | ...`. `Priority` is required in
//!      every state; `Commit: self` is refused (it would resolve to the
//!      migration commit);
//!   4. the evidence, free text on one line.
//!
//! Example rows (`→` stands for a tab):
//!
//! ```text
//! # approved: owner 2026-11-12 first batch
//! A06→DONE→Priority: P2 | Commit: 0123abcd | Receipt: A06-STATUS-DONE.MD→A06-STATUS-DONE.MD names fix 0123abcd
//! A07→PARKED→Priority: P3 | Reason: no program needs it | Reopen: a port calls it→LEGACY01 sample 3
//! A08→OBSOLETE→Priority: P3 | Reason: duplicate | Target: A07→same finding as A07
//! A09→OPEN→Priority: P2→legacy OFFEN, work not started
//! ```
//!
//! # What a run does
//!
//! It checks every row first and writes nothing if one fails; then for each
//! row it writes line 3 in ADR-037 form (state date: the day of the run) and,
//! below it, the lines
//!
//! ```text
//! <empty>
//! Migrated (owner batch <N>, <date>), evidence: <evidence>
//! <empty>
//! Line 3 before the migration:
//! <the old line 3, verbatim, then the ticket's old line 4 onward>
//! ```
//!
//! so the old status text keeps its words (git sees it as an unchanged line);
//! removes every baseline entry of the migrated ids (and no other); refreshes
//! their `TASK/README.md` rows; commits once, through the hooks, with the
//! subject `docs(task): migrate legacy tickets, owner batch <N>` (the ids are
//! listed in the body; the `commit-msg` checks read ids from the subject).
//! If the commit fails (a hook rejects it), every file the run wrote and the
//! index go back to their state before the run, which was clean.
//!
//! # Refusals (point 16, TOOL54 Work 2)
//!
//! A run refuses, changing nothing: once [`CLOSED_FILE`] exists (in the
//! working tree or `HEAD`; the owner's decision that ends the migration);
//! once the baseline holds no entry of a ticket; a batch file not committed or
//! changed since; a missing or malformed approval line; a row without one of
//! the fields point 1 requires, or a line 3 that does not parse; an id twice;
//! an id with no ticket or not in the baseline (so a second run of the same
//! batch refuses: its ids left the baseline with the first); a `Commit` that
//! is no commit on the current history (an old hash is refused with its
//! translation); a `DONE` row whose receipt `<ID>-STATUS-DONE.MD` does not
//! exist or whose evidence does not name that receipt and the commit (an old
//! hash that `docs/commit-map.txt` translates to the commit counts,
//! [`evidence_names_commit`], the rule `status-check` applies too); a migrated ticket that would still have a
//! rejecting gap (`status-check`, e.g. a DONE receipt under a non-DONE state);
//! anything staged, or uncommitted changes in a file it writes. It never
//! writes a `-STATUS-DONE.MD` and is no acceptance (point 17).

use super::*;

/// The directory of the batch files (listed in [`NON_TICKETS`]).
pub const MIGRATION_DIR: &str = "TASK/migration";
/// The owner's decision that ends the migration: once this file exists, in
/// the working tree or in `HEAD`, `migrate` refuses every run. Its content
/// is the decision (date, owner, note); `migrate` reads only its existence.
pub const CLOSED_FILE: &str = "TASK/migration/closed.txt";
/// The start of the approval line (line 1 of a batch file).
pub const APPROVAL_PREFIX: &str = "# approved: owner ";
/// The header row a batch file may carry.
pub const HEADER_ROW: &str = "id\tstate\tfields\tevidence";

/// The usage text of `migrate`, appended to `cargo xtask help`.
pub const USAGE: &str = "\
Legacy migration (ADR-037 points 15-17, TOOL54; tools/xtask/src/tickets/migrate.rs):
  migrate --batch <N>            — rewrites line 3 of the legacy tickets of the
                                   committed TASK/migration/batch-<N>.tsv: line
                                   1 `# approved: owner YYYY-MM-DD [note]`,
                                   rows `id<TAB>STATE<TAB>Priority: Pn | Key:
                                   value ...<TAB>evidence` (the fields of
                                   point 1, in order); adds `Migrated (owner
                                   batch <N>, <date>), evidence: ...` below
                                   line 3, removes the ids from the baseline,
                                   commits once; refuses an id not in the
                                   baseline (a second run), a missing field,
                                   an uncommitted or unapproved batch, DONE
                                   without a named receipt and fix commit, and
                                   every run once TASK/migration/closed.txt
                                   exists or no ticket is left in the baseline;
                                   a hook rejecting the commit: every write is
                                   reverted
";

/// One row of a batch file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// 1-based line of the batch file.
    pub line: usize,
    pub id: String,
    pub line3: Line3,
    pub evidence: String,
}

/// A parsed batch file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Batch {
    pub approval: String,
    pub rows: Vec<Row>,
}

/// The batch file of batch `n`, workspace-relative.
pub fn batch_file(n: &str) -> String {
    format!("{MIGRATION_DIR}/batch-{n}.tsv")
}

/// `# approved: owner YYYY-MM-DD` with an optional ` <note>`.
pub fn is_approval(line: &str) -> bool {
    let Some(rest) = line.strip_prefix(APPROVAL_PREFIX) else { return false };
    let (Some(date), Some(note)) = (rest.get(..10), rest.get(10..)) else { return false };
    is_date(date) &&(note.is_empty() || (note.starts_with(' ') && !note.trim().is_empty()))
}

/// Parses batch file `file` (its text `text`) for line 3 dated `date`;
/// every error, one per line.
pub fn parse_batch(file: &str, text: &str, date: &str) -> Result<Batch, Vec<String>> {
    let mut errors = Vec::new();
    let mut lines = text.lines();
    let first = lines.next().unwrap_or("");
    if !is_approval(first) {
        errors.push(format!(
            "{file}:1: line 1 must be the owner's approval line `{APPROVAL_PREFIX}YYYY-MM-DD [note]` (ADR-037 point 16), found {first:?}"
        ));
    }
    let mut rows: Vec<Row> = Vec::new();
    for (i, l) in lines.enumerate() {
        let n = i + 2;
        if l.trim().is_empty() || l.starts_with('#') || l == HEADER_ROW {
            continue;
        }
        let cols: Vec<&str> = l.split('\t').collect();
        if cols.len() != 4 {
            errors.push(format!("{file}:{n}: expected 4 tab-separated columns (id, state, fields, evidence), found {}", cols.len()));
            continue;
        }
        let (id, st, fields, evidence) = (cols[0], cols[1], cols[2], cols[3]);
        if !is_id(id) {
            errors.push(format!("{file}:{n}: {id:?} is no ticket id"));
            continue;
        }
        let Some(state) = State::parse(st) else {
            errors.push(format!(
                "{file}:{n}: {id}: target state {st:?} is not one of {}",
                STATES.iter().map(|s| s.name()).collect::<Vec<_>>().join(", ")
            ));
            continue;
        };
        if evidence.trim().is_empty() || evidence.trim() != evidence {
            errors.push(format!("{file}:{n}: {id}: the evidence column must be non-empty, without surrounding blanks"));
            continue;
        }
        if !fields.starts_with("Priority: ") {
            errors.push(format!(
                "{file}:{n}: {id}: {} lacks the required field Priority (ADR-037 point 1): the fields column starts with `Priority: Pn`, found {fields:?}",
                state.name()
            ));
            continue;
        }
        let text3 = format!("Status: {} ({date}) | {fields}", state.name());
        let line3 = match parse_line3(id, &text3) {
            Ok(l) => l,
            Err(e) => {
                for fe in e.errors {
                    if fe.missing {
                        errors.push(format!(
                            "{file}:{n}: {id}: {} lacks the required field {} (ADR-037 point 1): expected {}",
                            state.name(),
                            fe.field,
                            fe.expected
                        ));
                    } else {
                        errors.push(format!("{file}:{n}: {id}: {}: expected {}", fe.field, fe.expected));
                    }
                }
                continue;
            }
        };
        if line3.get("Commit") == Some("self") {
            errors.push(format!("{file}:{n}: {id}: `Commit: self` would name the migration commit; give the hash of the work"));
            continue;
        }
        if let Some(prev) = rows.iter().find(|r| r.id == id) {
            errors.push(format!("{file}:{n}: {id} stands twice (also line {})", prev.line));
            continue;
        }
        rows.push(Row { line: n, id: id.to_string(), line3, evidence: evidence.to_string() });
    }
    if rows.is_empty() && errors.is_empty() {
        errors.push(format!("{file}: no rows"));
    }
    if errors.is_empty() {
        Ok(Batch { approval: first.to_string(), rows })
    } else {
        Err(errors)
    }
}

/// The line written below line 3 (point 15).
pub fn migrated_line(n: &str, date: &str, evidence: &str) -> String {
    format!("Migrated (owner batch {n}, {date}), evidence: {evidence}")
}

/// `text` with line 3 replaced by `line3`, followed by the migrated line and
/// the old line 3 onward (see the module doc).
pub fn migrated_text(text: &str, line3: &str, migrated: &str) -> Result<String, String> {
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    if lines.len() < 3 {
        return Err("the ticket has no line 3".into());
    }
    let mut s = lines[..2].concat();
    let _ = write!(s, "{line3}\n\n{migrated}\n\nLine 3 before the migration:\n");
    s.push_str(&lines[2..].concat());
    if !s.ends_with('\n') {
        s.push('\n');
    }
    Ok(s)
}

/// The keys of `baseline` that are tickets of `root` (`TASK/<key>.md`).
fn ticket_keys(root: &Path, baseline: &BTreeSet<(String, Code)>) -> BTreeSet<String> {
    baseline
        .iter()
        .map(|(k, _)| k.clone())
        .filter(|k| is_id(k) && root.join(TASK_DIR).join(format!("{k}.md")).is_file())
        .collect()
}

fn in_head(root: &Path, rel: &str) -> bool {
    Command::new("git")
        .args(["cat-file", "-e", &format!("HEAD:{rel}")])
        .current_dir(root)
        .output()
        .is_ok_and(|o| o.status.success())
}

fn on_history(root: &Path, hash: &str) -> bool {
    let Ok(full) = git_out(root, &["rev-parse", "--verify", "--quiet", &format!("{hash}^{{commit}}")]) else {
        return false;
    };
    Command::new("git")
        .args(["merge-base", "--is-ancestor", full.trim(), "HEAD"])
        .current_dir(root)
        .status()
        .is_ok_and(|s| s.success())
}

/// `cargo xtask migrate --batch <N>`.
pub fn cmd_migrate(root: &Path, args: &[String]) -> Result<(), String> {
    migrate(root, args, &today()?)
}

/// `migrate` with the date of line 3 and of the migrated line given.
pub fn migrate(root: &Path, args: &[String], date: &str) -> Result<(), String> {
    let usage = "usage: cargo xtask migrate --batch <N>";
    let n = match args {
        [flag, n] if flag == "--batch" => n.clone(),
        _ => return Err(usage.into()),
    };
    if n.is_empty() || !n.bytes().all(|c| c.is_ascii_digit()) {
        return Err(format!("migrate: --batch expects a decimal number, found {n:?}"));
    }
    // The owner's end of the migration.
    if root.join(CLOSED_FILE).exists() || in_head(root, CLOSED_FILE) {
        return Err(format!("migrate: refused: {CLOSED_FILE} exists, the owner has ended the migration (ADR-037 point 16)"));
    }
    // The baseline must still hold a ticket.
    let baseline_text = fs::read_to_string(root.join(BASELINE_FILE)).map_err(|e| format!("migrate: {BASELINE_FILE}: {e}"))?;
    let baseline = parse_baseline(&baseline_text)?;
    let legacy = ticket_keys(root, &baseline);
    if legacy.is_empty() {
        return Err(format!("migrate: refused: {BASELINE_FILE} holds no ticket entry, the migration is complete (ADR-037 point 16)"));
    }
    // The batch file: committed, unchanged, read from HEAD.
    let file = batch_file(&n);
    if !in_head(root, &file) {
        return Err(format!("migrate: refused: {file} is not committed (ADR-037 point 16: committed before the run)"));
    }
    let dirty = git_out(root, &["status", "--porcelain", "--", &file])?;
    if !dirty.trim().is_empty() {
        return Err(format!("migrate: refused: {file} differs from its committed version:\n{}", dirty.trim_end()));
    }
    let text = git_out(root, &["show", &format!("HEAD:{file}")])?;
    let batch = parse_batch(&file, &text, date).map_err(|e| format!("migrate: refused:\n  {}", e.join("\n  ")))?;

    // Every row, before anything is written.
    let mut errors = Vec::new();
    let aids = Aids::load(&mut View::disk(root))?;
    let mut new_texts: Vec<(String, String)> = Vec::new();
    for row in &batch.rows {
        let id = &row.id;
        let at = format!("{file}:{}: {id}", row.line);
        let rel = format!("{TASK_DIR}/{id}.md");
        let Ok(old) = fs::read_to_string(root.join(&rel)) else {
            errors.push(format!("{at}: no ticket {rel}"));
            continue;
        };
        if !legacy.contains(id) {
            errors.push(format!(
                "{at}: not in {BASELINE_FILE}; migrate rewrites legacy tickets only (ADR-037 point 16; a batch that ran already refuses here)"
            ));
            continue;
        }
        if let Some(c) = row.line3.get("Commit") {
            if !on_history(root, c) {
                let new: Vec<&str> = aids.map.translate(c).collect();
                let hint = if new.is_empty() {
                    String::new()
                } else {
                    format!("; it is an old hash, {COMMIT_MAP_FILE} translates it to {}", new.join(", "))
                };
                errors.push(format!("{at}: Commit: {c} is no commit on the current history{hint}"));
            }
        }
        let migrated = migrated_line(&n, date, &row.evidence);
        if row.line3.state == State::Done {
            let receipt = row.line3.get("Receipt").unwrap_or("");
            let commit = row.line3.get("Commit").unwrap_or("");
            if !root.join(TASK_DIR).join(receipt).is_file() {
                errors.push(format!("{at}: DONE needs the existing receipt {TASK_DIR}/{receipt}; migrate writes none (ADR-037 point 17)"));
            }
            // The row's evidence, as the ticket's `Migrated` line, names the
            // commit (old hashes translated): the rule status-check applies.
            if !row.evidence.contains(receipt) || !evidence_names_commit(&aids.map, "", std::slice::from_ref(&migrated), commit) {
                errors.push(format!("{at}: DONE only from evidence that names the receipt {receipt} and the fix commit {commit} (ADR-037 point 17)"));
            }
        }
        match migrated_text(&old, &row.line3.render(), &migrated) {
            Ok(t) => new_texts.push((id.clone(), t)),
            Err(e) => errors.push(format!("{at}: {e}")),
        }
    }
    // The migrated tickets have no rejecting gap left (status-check's rules).
    if errors.is_empty() {
        let mut view = View::disk(root);
        let mut a = Analysis::load(&mut view)?;
        for (id, t) in &new_texts {
            a.tickets.insert(id.clone(), Ticket::from_text(id, t));
        }
        let adrs = adrs_of(&mut view)?;
        let mut repo = GitRepo(root.to_path_buf());
        for (id, _) in &new_texts {
            for x in ticket_violations(&a, id, &adrs, &mut repo).into_iter().filter(|x| x.code.rejecting()) {
                errors.push(format!("{file}: {id}: after the migration: [{}] {}", x.code.name(), x.msg));
            }
        }
    }
    if !errors.is_empty() {
        return Err(format!("migrate: refused, nothing written:\n  {}", errors.join("\n  ")));
    }

    // The files it writes are clean, nothing else is staged.
    let staged = git_out(root, &["diff", "--cached", "--name-only"])?;
    if !staged.trim().is_empty() {
        return Err(format!("migrate: refused: other changes are staged; it commits only its own:\n{}", staged.trim_end()));
    }
    let mut own: Vec<String> = new_texts.iter().map(|(id, _)| format!("{TASK_DIR}/{id}.md")).collect();
    own.push(BASELINE_FILE.to_string());
    if root.join(README_FILE).is_file() {
        own.push(README_FILE.to_string());
    }
    let mut st_args = vec!["status", "--porcelain", "--"];
    st_args.extend(own.iter().map(String::as_str));
    let dirty = git_out(root, &st_args)?;
    if !dirty.trim().is_empty() {
        return Err(format!("migrate: refused: uncommitted changes in the files it writes:\n{}", dirty.trim_end()));
    }

    // Write and commit; on any failure (a hook rejecting the commit
    // included) every file it wrote and the index go back to the state
    // before the run, which was clean (checked above).
    let snapshot: Vec<(String, Option<Vec<u8>>)> = own.iter().map(|p| (p.clone(), fs::read(root.join(p)).ok())).collect();
    let ids: BTreeSet<&str> = new_texts.iter().map(|(id, _)| id.as_str()).collect();
    let keep: BTreeSet<(String, Code)> = baseline.iter().filter(|(k, _)| !ids.contains(k.as_str())).cloned().collect();
    let removed = baseline.len() - keep.len();
    let head = git_out(root, &["rev-parse", "--short=9", "HEAD"])?.trim().to_string();
    let subject = format!("docs(task): migrate legacy tickets, owner batch {n}");
    let mut body = format!(
        "{subject}\n\nGenerated by `cargo xtask migrate --batch {n}` (ADR-037 points 15-17, TOOL54)\n\
         from {file} as committed at {head}.\nApproval: {}\n\
         Not an acceptance; no -STATUS-DONE.MD written (point 17).\n\
         {removed} entries of the migrated tickets removed from {BASELINE_FILE}.\n\nMigrated ({}):\n",
        batch.approval,
        batch.rows.len()
    );
    for row in &batch.rows {
        let _ = writeln!(body, "- {} -> {}", row.id, row.line3.state.name());
    }
    let _ = write!(body, "\n{ACCEPT_TRAILER}\n");
    let written = (|| -> Result<(), String> {
        for (id, t) in &new_texts {
            let rel = format!("{TASK_DIR}/{id}.md");
            fs::write(root.join(&rel), t).map_err(|e| format!("migrate: {rel}: {e}"))?;
        }
        fs::write(root.join(BASELINE_FILE), render_baseline(&keep)).map_err(|e| format!("migrate: {BASELINE_FILE}: {e}"))?;
        for id in &ids {
            refresh_readme(root, id)?;
        }
        let mut add = vec!["add", "--"];
        add.extend(own.iter().map(String::as_str));
        git_out(root, &add)?;
        commit_with_message(root, &body)
    })();
    if let Err(e) = written {
        return Err(match revert(root, &snapshot) {
            Ok(()) => format!("{e}\nmigrate: reverted its writes; the tree, index and baseline are as before the run"),
            Err(r) => format!("{e}\nmigrate: the revert failed too ({r}); restore {} by hand", own.join(", ")),
        });
    }
    let new_head = git_out(root, &["rev-parse", "--short=9", "HEAD"])?;
    println!(
        "migrate: batch {n}: {} tickets migrated, {removed} baseline entries removed, committed {}",
        batch.rows.len(),
        new_head.trim()
    );
    Ok(())
}

/// `git commit -F` with `body`, through the hooks; the message file lies in
/// the git directory and is removed right after.
fn commit_with_message(root: &Path, body: &str) -> Result<(), String> {
    let gitdir = git_out(root, &["rev-parse", "--git-dir"])?;
    let msgfile = {
        let g = PathBuf::from(gitdir.trim());
        let g = if g.is_absolute() { g } else { root.join(g) };
        g.join("MIGRATE_MSG")
    };
    fs::write(&msgfile, body).map_err(|e| format!("migrate: {}: {e}", msgfile.display()))?;
    let st = Command::new("git")
        .args(["commit", "-q", "-F"])
        .arg(&msgfile)
        .current_dir(root)
        .status()
        .map_err(|e| format!("migrate: git commit: {e}"));
    let _ = fs::remove_file(&msgfile);
    if !st?.success() {
        return Err("migrate: git commit failed (the hooks' output above)".into());
    }
    Ok(())
}

/// Puts the files of `snapshot` back (content, or absent) and drops them
/// from the index, which held nothing else: the state before the run.
fn revert(root: &Path, snapshot: &[(String, Option<Vec<u8>>)]) -> Result<(), String> {
    let mut reset = vec!["reset", "-q", "HEAD", "--"];
    reset.extend(snapshot.iter().map(|(p, _)| p.as_str()));
    git_out(root, &reset)?;
    for (p, bytes) in snapshot {
        match bytes {
            Some(b) => fs::write(root.join(p), b).map_err(|e| format!("{p}: {e}"))?,
            None => {
                let _ = fs::remove_file(root.join(p));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
