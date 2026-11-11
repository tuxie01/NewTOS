//! Host tests of `migrate` (TOOL54 Work 4), in scratch repositories under
//! the workspace's `target/`.

use super::*;
use crate::doc_check::tests::TempRepo;

const D: &str = "2026-11-12";
const APPROVAL: &str = "# approved: owner 2026-11-12 test batch";

fn out(r: &TempRepo, args: &[&str]) -> String {
    let o = Command::new("git").args(args).current_dir(&r.0).output().unwrap();
    assert!(o.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&o.stderr));
    String::from_utf8_lossy(&o.stdout).trim().to_string()
}

fn args(a: &[&str]) -> Vec<String> {
    a.iter().map(|s| s.to_string()).collect()
}

fn commit(r: &TempRepo, msg: &str) -> String {
    r.git(&["add", "-A"]);
    r.git(&["commit", "-q", "-m", msg]);
    out(r, &["rev-parse", "HEAD"])
}

fn legacy(id: &str, line3: &str) -> String {
    format!("# {id} — a legacy title\n\n{line3}\n\n## Facts\n\nBody of {id}.\n")
}

fn read(r: &TempRepo, rel: &str) -> String {
    fs::read_to_string(r.0.join(rel)).unwrap()
}

fn baseline(r: &TempRepo) -> BTreeSet<(String, Code)> {
    parse_baseline(&read(r, BASELINE_FILE)).unwrap()
}

/// A repository with legacy tickets L1–L7, an ADR-037 ticket N1 (not in the
/// baseline), an orphan receipt, a README table, a fix commit for L3 named
/// by L3's DONE receipt, and the baseline written and committed. Returns the
/// fix commit's 9-digit hash.
fn setup(name: &str) -> (TempRepo, String) {
    let r = TempRepo::new(name);
    for (k, v) in [("user.name", "t"), ("user.email", "t@t"), ("commit.gpgsign", "false"), ("core.hooksPath", "/dev/null")] {
        r.git(&["config", k, v]);
    }
    r.write("docs/MEMORY.md", "# Memory\n\n## ADR-001: one\n");
    // L1's status runs on into line 4 (as A35, X01 do).
    r.write("TASK/L1.md", "# L1 — a legacy title\n\nStatus: OFFEN · Prioritaet: P2\n· Typ: tool\n\n## Facts\n\nBody.\n");
    r.write("TASK/L2.md", &legacy("L2", "Status: GEPARKT · Prioritaet: P3"));
    r.write("TASK/L3.md", &legacy("L3", "Status: OFFEN · Prioritaet: P1"));
    r.write("TASK/L4.md", &legacy("L4", "Status: GEGENSTANDSLOS · Prioritaet: P3"));
    r.write("TASK/L5.md", &legacy("L5", "Status: OFFEN · Prioritaet: P4"));
    r.write("TASK/L6.md", &legacy("L6", "Status: **ACCEPTED-Abweichung** · Prioritaet: P2"));
    r.write("TASK/L7.md", &legacy("L7", "Status: IMPLEMENTIERT — HW OFFEN · Prioritaet: P1"));
    r.write("TASK/N1.md", "# N1 — new\n\nStatus: OPEN (2026-11-10) | Priority: P2\n");
    r.write("TASK/ZZ-STATUS-DONE.MD", "# ZZ — orphan\n");
    r.write("TASK/README.md", "# Tasks\n\n| Ticket | Status | Title |\n|---|---|---|\n| [L1](L1.md) | OFFEN | a |\n| [L5](L5.md) | OFFEN | b |\n");
    commit(&r, "base");
    r.write("src.txt", "the work\n");
    let h = commit(&r, "fix(x): the work (L3)");
    let h9 = h[..9].to_string();
    r.write("TASK/L3-STATUS-DONE.MD", &format!("# L3 — receipt\n\nThe fix is {h9}.\n"));
    baseline_cmd(&r.0, true).unwrap();
    commit(&r, "baseline");
    (r, h9)
}

/// Writes batch `n` with `text` and commits it.
fn batch(r: &TempRepo, n: &str, text: &str) {
    r.write(&batch_file(n), text);
    commit(r, &format!("docs(task): batch {n}"));
}

fn assert_refused(r: &TempRepo, n: &str, needle: &str) {
    let head = out(r, &["rev-parse", "HEAD"]);
    let status = out(r, &["status", "--porcelain"]);
    let before = baseline(r);
    let e = migrate(&r.0, &args(&["--batch", n]), D).unwrap_err();
    assert!(e.contains(needle), "expected {needle:?} in: {e}");
    assert_eq!(out(r, &["rev-parse", "HEAD"]), head, "no commit");
    assert_eq!(out(r, &["status", "--porcelain"]), status, "nothing written");
    assert_eq!(baseline(r), before);
}

#[test]
fn approval_line_and_batch_syntax() {
    assert!(is_approval("# approved: owner 2026-11-12"));
    assert!(is_approval("# approved: owner 2026-11-12 the C1 batch"));
    for l in ["# approved: owner 2026-11-12 ", "# approved: owner yesterday", "# approved: 2026-11-12", "approved: owner 2026-11-12", "# approved: owner 2026-11-1ü"] {
        assert!(!is_approval(l), "{l}");
    }
    let f = "TASK/migration/batch-9.tsv";
    let b = parse_batch(f, &format!("{APPROVAL}\n{HEADER_ROW}\n# a comment\n\nA1\tOPEN\tPriority: P2\tev\n"), D).unwrap();
    assert_eq!(b.approval, APPROVAL);
    assert_eq!((b.rows[0].line, b.rows[0].line3.render()), (5, format!("Status: OPEN ({D}) | Priority: P2")));
    let e = parse_batch(f, "A1\tOPEN\tPriority: P2\tev\n", D).unwrap_err();
    assert!(e[0].contains("approval line"), "{e:?}");
    let e = parse_batch(f, &format!("{APPROVAL}\n"), D).unwrap_err();
    assert!(e[0].contains("no rows"), "{e:?}");
    let e = parse_batch(f, &format!("{APPROVAL}\nA1\tOPEN\tPriority: P2\n"), D).unwrap_err();
    assert!(e[0].contains("4 tab-separated columns"), "{e:?}");
    let e = parse_batch(f, &format!("{APPROVAL}\nA1\tCLOSED\tPriority: P2\tev\n"), D).unwrap_err();
    assert!(e[0].contains("not one of"), "{e:?}");
    let e = parse_batch(f, &format!("{APPROVAL}\nA1\tOPEN\tPriority: P2\t\n"), D).unwrap_err();
    assert!(e[0].contains("evidence"), "{e:?}");
    // Each state's required fields (point 1), Priority included.
    for (row, field) in [
        ("A1\tOPEN\t\tev", "Priority"),
        ("A1\tOBSOLETE\tReason: dup\tev", "Priority"),
        ("A1\tPARKED\tPriority: P3 | Reason: r\tev", "Reopen"),
        ("A1\tPARKED\tPriority: P3\tev", "Reason"),
        ("A1\tOBSOLETE\tPriority: P3\tev", "Reason"),
        ("A1\tACCEPTED\tPriority: P3\tev", "Ref"),
        ("A1\tDONE\tPriority: P3 | Commit: abcdef1\tev", "Receipt"),
        ("A1\tDONE\tPriority: P3 | Receipt: A1-STATUS-DONE.MD\tev", "Commit"),
        ("A1\tIMPLEMENTED\tPriority: P3 | Commit: abcdef1\tev", "Missing"),
    ] {
        let e = parse_batch(f, &format!("{APPROVAL}\n{row}\n"), D).unwrap_err();
        assert!(e.iter().any(|e| e.contains(&format!("lacks the required field {field}"))), "{row}: {e:?}");
    }
    let e = parse_batch(f, &format!("{APPROVAL}\nA1\tIMPLEMENTED\tPriority: P3 | Commit: self | Missing: m\tev\n"), D).unwrap_err();
    assert!(e[0].contains("Commit: self"), "{e:?}");
    let e = parse_batch(f, &format!("{APPROVAL}\nA1\tOPEN\tPriority: P2\tev\nA1\tOPEN\tPriority: P2\tev\n"), D).unwrap_err();
    assert!(e[0].contains("stands twice"), "{e:?}");
}

#[test]
fn migrated_text_keeps_the_old_status_below_and_parses() {
    let old = "# X1 — t\n\nStatus: OFFEN · Prioritaet: P2\n· Typ: x\n\nBody.\n";
    let new = migrated_text(old, "Status: OPEN (2026-11-12) | Priority: P2", &migrated_line("3", D, State::Open, "ev")).unwrap();
    let l: Vec<&str> = new.lines().collect();
    assert_eq!(
        l,
        [
            "# X1 — t",
            "",
            "Status: OPEN (2026-11-12) | Priority: P2",
            "",
            "Migrated (owner batch 3, 2026-11-12, to OPEN), evidence: ev",
            "",
            "Line 3 before the migration:",
            "Status: OFFEN · Prioritaet: P2",
            "· Typ: x",
            "",
            "Body.",
        ]
    );
    let t = Ticket::from_text("X1", &new);
    assert!(!t.legacy && t.parsed.is_ok(), "{:?}", t.parsed);
    assert!(migrated_text("# X1 — t\n\n", "x", "y").is_err());
}

#[test]
fn a_batch_of_every_target_state_migrates_in_one_commit() {
    let (r, h) = setup("migrate-states");
    let base_before = baseline(&r);
    let done_receipts = |r: &TempRepo| {
        fs::read_dir(r.0.join(TASK_DIR)).unwrap().filter(|e| e.as_ref().unwrap().file_name().to_string_lossy().ends_with("-STATUS-DONE.MD")).count()
    };
    let receipts_before = done_receipts(&r);
    let rows = [
        ("L1", "OPEN", "Priority: P2".to_string(), "legacy OFFEN, not started".to_string()),
        ("L2", "PARKED", "Priority: P3 | Reason: no program needs it | Reopen: a port calls it".into(), "LEGACY01 sample".into()),
        ("L3", "DONE", format!("Priority: P1 | Commit: {h} | Receipt: L3-STATUS-DONE.MD"), format!("L3-STATUS-DONE.MD names the fix {h}")),
        ("L4", "OBSOLETE", "Priority: P3 | Reason: duplicate | Target: L1".into(), "same finding as L1".into()),
        ("L6", "ACCEPTED", "Priority: P2 | Ref: ADR-001".into(), "ADR-001 sanctions it".into()),
        ("L7", "IMPLEMENTED", format!("Priority: P1 | Commit: {h} | Missing: Pi run"), format!("work in {h}, no Pi run")),
    ];
    let mut text = format!("{APPROVAL}\n{HEADER_ROW}\n");
    for (id, st, f, ev) in &rows {
        text.push_str(&format!("{id}\t{st}\t{f}\t{ev}\n"));
    }
    batch(&r, "1", &text);
    let count = |r: &TempRepo| out(r, &["rev-list", "--count", "HEAD"]).parse::<u32>().unwrap();
    let before = count(&r);
    migrate(&r.0, &args(&["--batch", "1"]), D).unwrap();
    assert_eq!(count(&r), before + 1, "one commit");
    assert_eq!(out(&r, &["status", "--porcelain"]), "", "everything committed");
    let subject = out(&r, &["log", "-n1", "--format=%s"]);
    assert_eq!(subject, "docs(task): migrate legacy tickets, owner batch 1");
    assert!(subject.len() <= 72);
    let msg = out(&r, &["log", "-n1", "--format=%B"]);
    assert!(msg.contains("- L3 -> DONE") && msg.contains(APPROVAL) && msg.ends_with(ACCEPT_TRAILER), "{msg}");
    let files = out(&r, &["show", "--name-only", "--format=", "HEAD"]);
    assert_eq!(
        files,
        "TASK/L1.md\nTASK/L2.md\nTASK/L3.md\nTASK/L4.md\nTASK/L6.md\nTASK/L7.md\nTASK/README.md\ntools/xtask/ticket-baseline.tsv"
    );
    for (id, st, f, ev) in &rows {
        let t = read(&r, &format!("TASK/{id}.md"));
        let l: Vec<&str> = t.lines().collect();
        assert_eq!(l[2], format!("Status: {st} ({D}) | {f}"), "{id}");
        assert_eq!(l[4], format!("Migrated (owner batch 1, {D}, to {st}), evidence: {ev}"), "{id}");
        assert_eq!(l[6], "Line 3 before the migration:", "{id}");
        assert!(l[7].starts_with("Status: "), "{id}: {}", l[7]);
        assert!(Ticket::from_text(id, &t).parsed.is_ok(), "{id}");
    }
    assert_eq!(read(&r, "TASK/L1.md").lines().nth(8), Some("· Typ: tool"));
    // The baseline shrinks by exactly the migrated keys.
    let after = baseline(&r);
    let gone: BTreeSet<(String, Code)> = base_before.difference(&after).cloned().collect();
    let migrated: BTreeSet<&str> = rows.iter().map(|r| r.0).collect();
    assert!(after.is_subset(&base_before));
    assert_eq!(gone, base_before.iter().filter(|(k, _)| migrated.contains(k.as_str())).cloned().collect());
    assert!(gone.contains(&("L3".into(), Code::DoneReceiptNotDone)) && gone.contains(&("L2".into(), Code::ParkedNoReason)));
    assert!(after.iter().any(|(k, _)| k == "L5") && after.iter().any(|(k, _)| k == "ZZ-STATUS-DONE.MD"));
    // No receipt written; the README row of L1 regenerated, L5's kept.
    assert_eq!(done_receipts(&r), receipts_before);
    let readme = read(&r, README_FILE);
    assert!(readme.contains("| [L1](L1.md) | OPEN |") && readme.contains("| [L5](L5.md) | OFFEN |"), "{readme}");
    // status-check passes: the full check, and the hooks' checks on exactly
    // the commit's staged content and message.
    let c = check(&r.0, &CheckOpts::default()).unwrap();
    assert!(c.rejected.is_empty(), "{:?}", c.rejected);
    r.git(&["reset", "-q", "--soft", "HEAD~1"]);
    let pre = check(&r.0, &CheckOpts { staged: true, new_only: true, message: None, history: false }).unwrap();
    assert!(pre.rejected.is_empty() && pre.checked >= rows.len(), "{:?}", pre.rejected);
    let cm = check(&r.0, &CheckOpts { staged: true, new_only: true, message: Some(msg.clone()), history: false }).unwrap();
    assert!(cm.rejected.is_empty(), "{:?}", cm.rejected);
    r.git(&["commit", "-q", "-m", &msg]);
    // A second run of the same batch refuses: its ids left the baseline.
    assert_refused(&r, "1", "not in tools/xtask/ticket-baseline.tsv");
}

#[test]
fn refusals_change_nothing() {
    let (r, h) = setup("migrate-refusals");
    // An id not in the baseline (also when another row is fine: all or nothing).
    batch(&r, "1", &format!("{APPROVAL}\nL1\tOPEN\tPriority: P2\tev\nN1\tPARKED\tPriority: P2 | Reason: r | Reopen: o\tev\n"));
    assert_refused(&r, "1", "N1: not in tools/xtask/ticket-baseline.tsv");
    // A missing required field.
    batch(&r, "2", &format!("{APPROVAL}\nL2\tPARKED\tPriority: P3 | Reason: r\tev\n"));
    assert_refused(&r, "2", "lacks the required field Reopen");
    batch(&r, "3", &format!("{APPROVAL}\nL4\tOBSOLETE\tReason: dup\tev\n"));
    assert_refused(&r, "3", "lacks the required field Priority");
    // Not committed: untracked, then changed after its commit.
    r.write(&batch_file("4"), &format!("{APPROVAL}\nL1\tOPEN\tPriority: P2\tev\n"));
    assert_refused(&r, "4", "is not committed");
    commit(&r, "batch 4");
    r.write(&batch_file("4"), &format!("{APPROVAL}\nL1\tOPEN\tPriority: P3\tev\n"));
    assert_refused(&r, "4", "differs from its committed version");
    r.write(&batch_file("4"), &format!("{APPROVAL}\nL1\tOPEN\tPriority: P2\tev\n"));
    assert_refused(&r, "99", "is not committed");
    // Without or with a malformed approval line.
    batch(&r, "5", "L1\tOPEN\tPriority: P2\tev\n");
    assert_refused(&r, "5", "approval line");
    batch(&r, "6", "# approved: owner soon\nL1\tOPEN\tPriority: P2\tev\n");
    assert_refused(&r, "6", "approval line");
    // DONE: the receipt must exist, the evidence name receipt and commit,
    // the commit be on the history.
    batch(&r, "7", &format!("{APPROVAL}\nL1\tDONE\tPriority: P2 | Commit: {h} | Receipt: L1-STATUS-DONE.MD\tL1-STATUS-DONE.MD fix {h}\n"));
    assert_refused(&r, "7", "needs the existing receipt TASK/L1-STATUS-DONE.MD");
    batch(&r, "8", &format!("{APPROVAL}\nL3\tDONE\tPriority: P1 | Commit: {h} | Receipt: L3-STATUS-DONE.MD\tsee L3-STATUS-DONE.MD\n"));
    assert_refused(&r, "8", "names the receipt");
    batch(&r, "9", &format!("{APPROVAL}\nL3\tDONE\tPriority: P1 | Commit: {h} | Receipt: L3-STATUS-DONE.MD\tfix {h}\n"));
    assert_refused(&r, "9", "names the receipt");
    batch(&r, "10", &format!("{APPROVAL}\nL3\tDONE\tPriority: P1 | Commit: deadbeef1 | Receipt: L3-STATUS-DONE.MD\tL3-STATUS-DONE.MD deadbeef1\n"));
    assert_refused(&r, "10", "Commit: deadbeef1 is no commit on the current history");
    // `Commit: self`.
    batch(&r, "11", &format!("{APPROVAL}\nL7\tIMPLEMENTED\tPriority: P1 | Commit: self | Missing: m\tev\n"));
    assert_refused(&r, "11", "Commit: self");
    // A state that leaves a rejecting gap: L3's DONE receipt is newer than
    // the migration (changed in the working tree; TOOL55 rework a).
    batch(&r, "12", &format!("{APPROVAL}\nL3\tOBSOLETE\tPriority: P1 | Reason: withdrawn\tev\n"));
    let l3_receipt = read(&r, "TASK/L3-STATUS-DONE.MD");
    r.write("TASK/L3-STATUS-DONE.MD", "# L3 — receipt\n\nEdited.\n");
    assert_refused(&r, "12", "after the migration: [done-receipt-not-done]");
    r.write("TASK/L3-STATUS-DONE.MD", &l3_receipt);
    // A Target that is no ticket.
    batch(&r, "13", &format!("{APPROVAL}\nL4\tOBSOLETE\tPriority: P3 | Reason: duplicate | Target: Q9\tev\n"));
    assert_refused(&r, "13", "[bad-target]");
    // Staged changes elsewhere; uncommitted changes in a file it writes.
    batch(&r, "14", &format!("{APPROVAL}\nL1\tOPEN\tPriority: P2\tev\n"));
    r.write("src.txt", "staged\n");
    r.git(&["add", "src.txt"]);
    assert_refused(&r, "14", "other changes are staged");
    r.git(&["reset", "-q", "HEAD", "--", "src.txt"]);
    r.write("src.txt", "the work\n");
    r.write("TASK/L1.md", &legacy("L1", "Status: OFFEN · Prioritaet: P2 (edited)"));
    assert_refused(&r, "14", "uncommitted changes in the files it writes");
    r.git(&["checkout", "-q", "--", "TASK/L1.md"]);
    // A ticket file that does not exist.
    batch(&r, "15", &format!("{APPROVAL}\nQ9\tOPEN\tPriority: P2\tev\n"));
    assert_refused(&r, "15", "no ticket TASK/Q9.md");
    // Arguments.
    assert!(migrate(&r.0, &args(&["--batch", "x"]), D).unwrap_err().contains("decimal"));
    assert!(migrate(&r.0, &args(&["--batch"]), D).unwrap_err().contains("usage"));
    // The owner's end of the migration: in the working tree, then committed.
    r.write(CLOSED_FILE, "owner 2026-11-20: migration ended\n");
    assert_refused(&r, "14", "ended the migration");
    commit(&r, "close the migration");
    assert_refused(&r, "14", "ended the migration");
    // Batch 14 itself was valid all along: without the switch it runs.
    r.git(&["rm", "-q", CLOSED_FILE]);
    commit(&r, "reopen");
    migrate(&r.0, &args(&["--batch", "14"]), D).unwrap();
}

#[test]
fn refuses_once_the_baseline_holds_no_ticket() {
    let r = TempRepo::new("migrate-no-ticket");
    for (k, v) in [("user.name", "t"), ("user.email", "t@t"), ("commit.gpgsign", "false"), ("core.hooksPath", "/dev/null")] {
        r.git(&["config", k, v]);
    }
    r.write("docs/MEMORY.md", "# Memory\n\n## ADR-001: one\n");
    r.write("TASK/N1.md", "# N1 — new\n\nStatus: OPEN (2026-11-10) | Priority: P2\n");
    r.write("TASK/ZZ-STATUS-DONE.MD", "# ZZ — orphan\n");
    commit(&r, "base");
    baseline_cmd(&r.0, true).unwrap();
    assert_eq!(baseline(&r), BTreeSet::from([("ZZ-STATUS-DONE.MD".to_string(), Code::OrphanReceipt)]));
    commit(&r, "baseline");
    batch(&r, "1", &format!("{APPROVAL}\nN1\tPARKED\tPriority: P2 | Reason: r | Reopen: o\tev\n"));
    assert_refused(&r, "1", "holds no ticket entry");
}

// ---------------------------------------------------------------------------
// Rework (owner 2026-11-11): old hashes, revert on a hook rejection,
// historical receipts
// ---------------------------------------------------------------------------

/// The workspace this test binary was built from.
fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()
}

fn bare_repo(name: &str) -> TempRepo {
    let r = TempRepo::new(name);
    for (k, v) in [("user.name", "t"), ("user.email", "t@t"), ("commit.gpgsign", "false"), ("core.hooksPath", "/dev/null")] {
        r.git(&["config", k, v]);
    }
    r.write("docs/MEMORY.md", "# Memory\n\n## ADR-001: one\n");
    r
}

#[test]
fn the_real_commit_map_translates_the_a01_receipt_hash() {
    let ws = workspace();
    let map = CommitMap::parse(&fs::read_to_string(ws.join(COMMIT_MAP_FILE)).unwrap());
    let new: Vec<&str> = map.translate("d7ab92d").collect();
    assert_eq!(new, ["87b03c250f9b04418337a01922f742fdab8ba2ec"]);
    assert!(map.translate("d7ab92").next().is_none(), "7 digits at least");
    let receipt = A01_RECEIPT;
    assert!(!names_commit(receipt, "87b03c250"), "the receipt names the old hash only");
    assert!(names_commit_mapped(receipt, "87b03c250", &map));
    assert!(evidence_names_commit(&map, receipt, &[], "87b03c250"));
    // The ticket's Migrated line counts as evidence too; nothing names a
    // foreign commit.
    let m = [migrated_line("1", D, State::Done, "fix 1234abcd5")];
    assert!(evidence_names_commit(&CommitMap::default(), "no hash here", &m, "1234abcd5"));
    assert!(!evidence_names_commit(&CommitMap::default(), "no hash here", &[], "1234abcd5"));
    assert!(!evidence_names_commit(&map, receipt, &m, "abcdef123"));
}

/// The real A01 (legacy ERLEDIGT, its receipt names the pre-2026-10-19 hash
/// `d7ab92d`) in a scratch repository whose commit map translates that hash
/// to the scratch fix commit. The ticket and receipt are the fixtures of
/// A01 as before its migration, not the live `TASK/` files: owner batch 20
/// migrated the live A01 to `DONE` (TOOL57).
#[test]
fn the_a01_case_migrates_to_done_through_the_commit_map() {
    let a01 = A01_TICKET;
    let receipt = A01_RECEIPT.as_bytes();
    let r = bare_repo("migrate-a01");
    r.write("TASK/A01.md", a01);
    fs::write(r.0.join("TASK/A01-STATUS-DONE.MD"), receipt).unwrap();
    r.write(COMMIT_MAP_FILE, "# Columns: old new\n");
    commit(&r, "base");
    r.write("src.txt", "work\n");
    let h = commit(&r, "fix(x): the work (A01)");
    let h9 = &h[..9];
    baseline_cmd(&r.0, true).unwrap();
    assert!(baseline(&r).contains(&("A01".into(), Code::Form)));
    commit(&r, "baseline");
    let row = format!("A01\tDONE\tPriority: P2 | Commit: {h9} | Receipt: A01-STATUS-DONE.MD\tA01-STATUS-DONE.MD names the fix d7ab92d\n");
    // Without the map line the old hash names nothing.
    batch(&r, "1", &format!("{APPROVAL}\n{row}"));
    assert_refused(&r, "1", "names the receipt A01-STATUS-DONE.MD and the fix commit");
    r.write(COMMIT_MAP_FILE, &format!("# Columns: old new\nd7ab92d256759bfa186dfcf67344a3ac4ea54bde {h}\n"));
    commit(&r, "map");
    // A row giving the old hash itself is refused with the translation.
    batch(&r, "2", &format!("{APPROVAL}\nA01\tDONE\tPriority: P2 | Commit: d7ab92d | Receipt: A01-STATUS-DONE.MD\tA01-STATUS-DONE.MD d7ab92d\n"));
    assert_refused(&r, "2", &format!("translates it to {h}"));
    batch(&r, "3", &format!("{APPROVAL}\n{row}"));
    migrate(&r.0, &args(&["--batch", "3"]), D).unwrap();
    assert_eq!(fs::read(r.0.join("TASK/A01-STATUS-DONE.MD")).unwrap(), receipt, "receipts are never rewritten");
    let c = check(&r.0, &CheckOpts::default()).unwrap();
    assert!(c.rejected.is_empty(), "{:?}", c.rejected);
    // status-check: the Migrated line alone names a commit too. A DONE whose
    // receipt names no hash passes with a Migrated line naming the commit,
    // and fails without it. The line matches a row of the committed batch
    // 9 (TOOL58: an unmatched line is no evidence).
    batch(&r, "9", &format!("{APPROVAL}\nD2\tDONE\tPriority: P2 | Commit: {h9} | Receipt: D2-STATUS-DONE.MD\tfix {h9}\n"));
    r.write(
        "TASK/D2.md",
        &format!("# D2 — t\n\nStatus: DONE ({D}) | Priority: P2 | Commit: {h9} | Receipt: D2-STATUS-DONE.MD\n\nMigrated (owner batch 9, {D}), evidence: fix {h9}\n"),
    );
    r.write("TASK/D2-STATUS-DONE.MD", "# D2 — receipt\n\nDone.\n");
    let c = check(&r.0, &CheckOpts::default()).unwrap();
    assert!(c.rejected.is_empty(), "{:?}", c.rejected);
    r.write("TASK/D2.md", &format!("# D2 — t\n\nStatus: DONE ({D}) | Priority: P2 | Commit: {h9} | Receipt: D2-STATUS-DONE.MD\n"));
    let c = check(&r.0, &CheckOpts::default()).unwrap();
    assert!(c.rejected.iter().any(|v| v.key == "D2" && v.code == Code::DoneNoReceipt), "{:?}", c.rejected);
}

/// Every file of the work tree (`.git` aside) with its bytes.
fn tree_bytes(dir: &Path, base: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
    for e in fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.file_name().is_some_and(|n| n == ".git") {
            continue;
        }
        if p.is_dir() {
            tree_bytes(&p, base, out);
        } else {
            out.insert(p.strip_prefix(base).unwrap().display().to_string(), fs::read(&p).unwrap());
        }
    }
}

type RepoState = (BTreeMap<String, Vec<u8>>, String, String, String);

/// Work tree bytes, `git status`, the staged blobs and `HEAD`.
fn state(r: &TempRepo) -> RepoState {
    let mut t = BTreeMap::new();
    tree_bytes(&r.0, &r.0, &mut t);
    (t, out(r, &["status", "--porcelain"]), out(r, &["ls-files", "-s"]), out(r, &["rev-parse", "HEAD"]))
}

#[test]
fn a_rejecting_hook_reverts_every_write_and_a_rerun_works() {
    let (r, _) = setup("migrate-hook-revert");
    batch(&r, "1", &format!("{APPROVAL}\nL1\tOPEN\tPriority: P2\tev\nL2\tPARKED\tPriority: P3 | Reason: r | Reopen: o\tev\n"));
    let hooks = r.0.join(".git/test-hooks");
    fs::create_dir_all(&hooks).unwrap();
    for hook in ["pre-commit", "commit-msg"] {
        for h in ["pre-commit", "commit-msg"] {
            let _ = fs::remove_file(hooks.join(h));
        }
        let p = hooks.join(hook);
        fs::write(&p, "#!/bin/sh\necho rejected by the test hook >&2\nexit 1\n").unwrap();
        assert!(Command::new("chmod").arg("+x").arg(&p).status().unwrap().success());
        r.git(&["config", "core.hooksPath", hooks.to_str().unwrap()]);
        let before = state(&r);
        let e = migrate(&r.0, &args(&["--batch", "1"]), D).unwrap_err();
        assert!(e.contains("git commit failed") && e.contains("reverted its writes"), "{hook}: {e}");
        assert_eq!(state(&r), before, "{hook}: tree, index, baseline and HEAD as before the run");
    }
    r.git(&["config", "core.hooksPath", "/dev/null"]);
    migrate(&r.0, &args(&["--batch", "1"]), D).unwrap();
    assert_eq!(out(&r, &["status", "--porcelain"]), "");
    assert!(read(&r, "TASK/L1.md").lines().nth(2).unwrap().starts_with("Status: OPEN"));
}

#[test]
fn historical_receipts_are_no_orphans_once_listed() {
    let r = bare_repo("migrate-historical");
    r.write("TASK/N1.md", "# N1 — new\n\nStatus: OPEN (2026-11-10) | Priority: P2\n");
    r.write("TASK/ZZ-STATUS-DONE.MD", "# ZZ — an old round\n");
    r.write("TASK/YY-3-STATUS-UNFINISHED.MD", "# YY — another\n");
    commit(&r, "base");
    baseline_cmd(&r.0, true).unwrap();
    commit(&r, "baseline");
    let orphan =
        |c: &CheckResult, k: &str| c.reported.iter().chain(&c.rejected).any(|v| v.key == k && v.code == Code::OrphanReceipt);
    // Listed without the owner's approval line: the check fails loudly.
    r.write(HISTORICAL_FILE, "ZZ-STATUS-DONE.MD\thistory of the 2026-09 round\n");
    assert!(check(&r.0, &CheckOpts::default()).unwrap_err().contains("approval line"));
    for bad in ["ZZ-STATUS-DONE.MD\n", "ZZ-STATUS-DONE.MD\t \n", "ZZ.md\treason\n", "ZZ-STATUS-DONE.MD\tr\nZZ-STATUS-DONE.MD\tr\n"] {
        r.write(HISTORICAL_FILE, &format!("{APPROVAL}\n{bad}"));
        assert!(check(&r.0, &CheckOpts::default()).is_err(), "{bad:?}");
    }
    r.write(
        HISTORICAL_FILE,
        &format!("{APPROVAL}\nreceipt\treason\n# kept as history\nZZ-STATUS-DONE.MD\thistory of the 2026-09 round\n"),
    );
    let c = check(&r.0, &CheckOpts::default()).unwrap();
    assert!(!orphan(&c, "ZZ-STATUS-DONE.MD"), "{:?}", c.rejected);
    assert!(c.rejected.iter().any(|v| v.key == "ZZ-STATUS-DONE.MD" && v.code == Code::Baseline), "the entry is fixed: prune it");
    assert_eq!(prune_baseline(&r.0, None).unwrap(), 1);
    assert_eq!(baseline(&r), BTreeSet::from([("YY-3-STATUS-UNFINISHED.MD".to_string(), Code::OrphanReceipt)]));
    let c = check(&r.0, &CheckOpts::default()).unwrap();
    assert!(c.rejected.is_empty(), "{:?}", c.rejected);
    // The unlisted one stays a (baselined) orphan; nothing is moved.
    let mut v = View::disk(&r.0);
    let a = Analysis::load(&mut v).unwrap();
    let adrs = BTreeSet::new();
    let mut repo = GitRepo(r.0.clone());
    assert!(key_violations(&a, "YY-3-STATUS-UNFINISHED.MD", &adrs, &mut repo).iter().any(|v| v.code == Code::OrphanReceipt));
    assert!(key_violations(&a, "ZZ-STATUS-DONE.MD", &adrs, &mut repo).is_empty());
    assert!(r.0.join("TASK/ZZ-STATUS-DONE.MD").is_file());
    // The list and the pruned baseline pass the hooks' staged check.
    r.git(&["add", "-A"]);
    let s = check(&r.0, &CheckOpts { staged: true, new_only: true, message: None, history: false }).unwrap();
    assert!(s.rejected.is_empty(), "{:?}", s.rejected);
}

/// TOOL55: a row to a non-DONE state for a legacy ticket whose committed
/// DONE receipt exists migrates (the Migrated line records the target
/// state, the exception of `stray_done_receipts`); while the receipt has an
/// uncommitted change (newer than the migration) the row is refused.
#[test]
fn an_implemented_row_with_a_legacy_done_receipt_migrates() {
    let (r, h) = setup("migrate-tool55");
    let legacy_receipt = "# L7 — receipt\n\nLegacy DONE, open clause.\n";
    r.write("TASK/L7-STATUS-DONE.MD", legacy_receipt);
    commit(&r, "docs(task): legacy receipt (L7)");
    // L7's receipt changed in the working tree: refused.
    batch(&r, "1", &format!("{APPROVAL}\nL7\tIMPLEMENTED\tPriority: P1 | Commit: {h} | Missing: tester acceptance\twork in {h}\n"));
    r.write("TASK/L7-STATUS-DONE.MD", "# L7 — receipt\n\nEdited.\n");
    assert_refused(&r, "1", "[done-receipt-not-done]");
    r.write("TASK/L7-STATUS-DONE.MD", legacy_receipt);
    // L7 (DONE receipt) to IMPLEMENTED: migrates.
    batch(&r, "2", &format!("{APPROVAL}\nL7\tIMPLEMENTED\tPriority: P1 | Commit: {h} | Missing: tester acceptance\twork in {h}\n"));
    migrate(&r.0, &args(&["--batch", "2"]), D).unwrap();
    let t = read(&r, "TASK/L7.md");
    assert_eq!(t.lines().nth(4), Some(format!("Migrated (owner batch 2, {D}, to IMPLEMENTED), evidence: work in {h}").as_str()));
    assert_eq!(migrated_not_done(&Ticket::from_text("L7", &t)).len(), 1);
    assert_eq!(read(&r, "TASK/L7-STATUS-DONE.MD"), legacy_receipt, "the receipt is untouched");
    let c = check(&r.0, &CheckOpts::default()).unwrap();
    assert!(c.rejected.is_empty(), "{:?}", c.rejected);
}

/// `TASK/A01.md` as before its migration (`add86da91^`, owner batch 20): a
/// legacy ticket, `ERLEDIGT`, in the baseline. A committed copy, not a
/// `git show` of that commit, so the test needs no history (a shallow clone
/// runs it too) and never reads the live ticket (TOOL57).
const A01_TICKET: &str = include_str!("fixtures/A01.md");
/// `TASK/A01-STATUS-DONE.MD` as of `add86da91^`; it names the old fix hash
/// `d7ab92d` only.
const A01_RECEIPT: &str = include_str!("fixtures/A01-STATUS-DONE.MD");
/// The git blob ids of the two fixtures, as recorded at `add86da91^`.
const A01_BLOBS: [(&str, &str); 2] = [
    ("fixtures/A01.md", "1e1fee28514cc4e622c429feec5ca70f8af4cf9c"),
    ("fixtures/A01-STATUS-DONE.MD", "1c7b2ed74884e041b3ca6bc7940a312367d7a7e9"),
];

/// The fixtures are byte-identical to the blobs of `add86da91^`: their git
/// blob ids (`git hash-object`, no history needed) match the recorded ones.
#[test]
fn the_a01_fixtures_are_the_pre_migration_blobs() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/tickets/migrate");
    for (rel, blob) in A01_BLOBS {
        let o = Command::new("git").arg("hash-object").arg(dir.join(rel)).output().unwrap();
        assert!(o.status.success(), "git hash-object {rel}: {}", String::from_utf8_lossy(&o.stderr));
        assert_eq!(String::from_utf8_lossy(&o.stdout).trim(), blob, "{rel} is not the blob of add86da91^");
    }
    assert!(A01_TICKET.lines().nth(2).unwrap().starts_with("Status: **ERLEDIGT"), "the fixture is the legacy ticket");
}
