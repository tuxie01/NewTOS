//! Host tests of TOOL55: the `done-receipt-not-done` exception for tickets
//! migrated to `IMPLEMENTED`, and `accept` writing numbered DONE receipts
//! instead of overwriting one.

use super::*;
use crate::doc_check::tests::TempRepo;

const D: &str = "2026-11-11";
const MIGRATED: &str = "Migrated (owner batch 4, 2026-11-11, to IMPLEMENTED), evidence: legacy ERLEDIGT with an open clause";

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

/// A repository with git identity (for `accept`'s own commit) and
/// `docs/MEMORY.md`.
fn repo(name: &str) -> TempRepo {
    let r = TempRepo::new(name);
    for (k, v) in [("user.name", "t"), ("user.email", "t@t"), ("commit.gpgsign", "false"), ("core.hooksPath", "/dev/null")] {
        r.git(&["config", k, v]);
    }
    r.write("docs/MEMORY.md", "# Memory\n\n## ADR-001: one\n");
    r
}

/// A ticket with line 3 `line3` and, if given, `below` as line 5 (where
/// `migrate` writes its `Migrated` line).
fn ticket(id: &str, line3: &str, below: Option<&str>) -> String {
    match below {
        Some(m) => format!("# {id} — a title\n\n{line3}\n\n{m}\n\nLine 3 before the migration:\nStatus: ERLEDIGT\n\nBody.\n"),
        None => format!("# {id} — a title\n\n{line3}\n\nBody.\n"),
    }
}

fn implemented() -> String {
    "Status: IMPLEMENTED (2026-11-11) | Priority: P1 | Commit: self | Missing: tester acceptance".to_string()
}

fn rejected(r: &TempRepo) -> Vec<(String, Code)> {
    check(&r.0, &CheckOpts::default()).unwrap().rejected.into_iter().map(|v| (v.key, v.code)).collect()
}

#[test]
fn the_marker_is_the_target_state_in_the_migrated_parenthesis() {
    assert_eq!(migrated_target(MIGRATED), Some(State::Implemented));
    assert_eq!(migrated_target("Migrated (owner batch 12, 2026-11-12, to DONE), evidence: x"), Some(State::Done));
    // What migrate writes is what the check reads.
    let line = migrate::migrated_line("7", D, State::Implemented, "ev");
    assert_eq!(line, "Migrated (owner batch 7, 2026-11-11, to IMPLEMENTED), evidence: ev");
    assert_eq!(migrated_target(&line), Some(State::Implemented));
    for none in [
        // Batches 1-3: no target state.
        "Migrated (owner batch 3, 2026-11-11), evidence: receipt TASK/A114-STATUS-DONE.MD exists",
        // The words in the evidence, not in the parenthesis.
        "Migrated (owner batch 3, 2026-11-11), evidence: x, to IMPLEMENTED), evidence: y",
        "Migrated (owner batch 3, 2026-11-11, to IMPLEMENTIERT), evidence: x",
        "Migrated (owner batch x, 2026-11-11, to IMPLEMENTED), evidence: x",
        "Migrated (owner batch 3, 2026-13-11, to IMPLEMENTED), evidence: x",
        "Migrated (owner batch 3, 2026-11-11, IMPLEMENTED), evidence: x",
        "Migrated (owner batch 3, 2026-11-11, to IMPLEMENTED, extra), evidence: x",
        " Migrated (owner batch 3, 2026-11-11, to IMPLEMENTED), evidence: x",
    ] {
        assert_eq!(migrated_target(none), None, "{none}");
    }
}

#[test]
fn numbered_done_receipts_parse_and_name() {
    assert_eq!(done_receipt_number("X1", "X1-STATUS-DONE.MD"), Some(1));
    assert_eq!(done_receipt_number("X1", "X1-STATUS-DONE-2.MD"), Some(2));
    assert_eq!(done_receipt_number("X1", "X1-STATUS-DONE-13.MD"), Some(13));
    for none in ["X1-STATUS-DONE-1.MD", "X1-STATUS-DONE-02.MD", "X1-STATUS-DONE-.MD", "X1-3b-STATUS-DONE.MD", "X10-STATUS-DONE.MD", "X1-STATUS-UNFINISHED.MD"] {
        assert_eq!(done_receipt_number("X1", none), None, "{none}");
    }
    assert_eq!((done_receipt_file("X1", 1), done_receipt_file("X1", 3)), ("X1-STATUS-DONE.MD".into(), "X1-STATUS-DONE-3.MD".into()));
    // Line 3 may point to a numbered receipt of its own ticket, no other.
    assert!(parse_line3("X1", "Status: DONE (2026-11-11) | Priority: P1 | Commit: abcdef1 | Receipt: X1-STATUS-DONE-2.MD").is_ok());
    for bad in ["X1-STATUS-DONE-1.MD", "X1-STATUS-DONE-02.MD", "Y1-STATUS-DONE-2.MD", "X1-3b-STATUS-DONE.MD"] {
        let l = format!("Status: DONE (2026-11-11) | Priority: P1 | Commit: abcdef1 | Receipt: {bad}");
        assert!(parse_line3("X1", &l).unwrap_err().errors.iter().any(|e| e.field == "Receipt"), "{bad}");
    }
    // The next free name: after the highest, never an existing one.
    let r = repo("tool55-next");
    r.write("TASK/X1.md", &ticket("X1", &implemented(), None));
    assert_eq!(next_done_receipt(&r.0, "X1").unwrap(), "X1-STATUS-DONE.MD");
    r.write("TASK/X1-STATUS-DONE.MD", "a");
    assert_eq!(next_done_receipt(&r.0, "X1").unwrap(), "X1-STATUS-DONE-2.MD");
    r.write("TASK/X1-STATUS-DONE-3.MD", "b");
    r.write("TASK/X10-STATUS-DONE-7.MD", "c");
    assert_eq!(next_done_receipt(&r.0, "X1").unwrap(), "X1-STATUS-DONE-4.MD");
    // A numbered receipt is a receipt, not an unknown file; a malformed
    // number is unknown.
    r.write("TASK/X1-STATUS-DONE-01.MD", "d");
    let mut view = View::disk(&r.0);
    let a = Analysis::load(&mut view).unwrap();
    assert!(a.receipts.contains_key("X1-STATUS-DONE-3.MD") && !a.unknown.contains("X1-STATUS-DONE-3.MD"));
    assert!(a.unknown.contains("X1-STATUS-DONE-01.MD"));
}

/// The `Migrated` line of target state `st`.
fn migrated_to(st: &str) -> String {
    MIGRATED.replace("to IMPLEMENTED", &format!("to {st}"))
}

/// The exception (rework a): every migrated target state but DONE keeps its
/// legacy `<ID>-STATUS-DONE.MD`, if the receipt is older than the commit
/// that wrote the `Migrated` line; nothing else is exempt.
#[test]
fn the_exception_applies_to_every_non_done_target_with_an_older_receipt_only() {
    let r = repo("tool55-exception");
    let receipt = "# R — legacy receipt\n\nDone, open clause.\n";
    let legacy = |id: &str| ticket(id, "Status: ERLEDIGT · Prioritaet: P2", None);
    let parked = "Status: PARKED (2026-11-11) | Priority: P3 | Reason: r | Reopen: o";
    let obsolete = "Status: OBSOLETE (2026-11-11) | Priority: P3 | Reason: withdrawn";
    let open = "Status: OPEN (2026-11-11) | Priority: P2";
    let accepted = "Status: ACCEPTED (2026-11-11) | Priority: P2 | Ref: ADR-001";
    // Commit 1: the legacy tickets and their receipts (C2 and C3 without).
    let ids = ["S1", "S2", "S3", "S4", "S5", "C1", "C2", "C3", "C4", "D1", "D2", "N1", "N2", "E1", "M5"];
    for id in ids {
        r.write(&format!("TASK/{id}.md"), &legacy(id));
        let rc = if id == "M5" { "M5-STATUS-DONE-2.MD".to_string() } else { format!("{id}-STATUS-DONE.MD") };
        if !matches!(id, "C2" | "C3") {
            r.write(&format!("TASK/{rc}"), receipt);
        }
    }
    let h0 = commit(&r, "base");
    let done = format!("Status: DONE (2026-11-11) | Priority: P2 | Commit: {} | Receipt: D2-STATUS-DONE.MD", &h0[..9]);
    // Commit 2: the migration (C3's receipt added in the same commit).
    let cases: [(&str, String, Option<String>); 15] = [
        ("S1", implemented(), Some(migrated_to("IMPLEMENTED"))),
        ("S2", parked.into(), Some(migrated_to("PARKED"))),
        ("S3", obsolete.into(), Some(migrated_to("OBSOLETE"))),
        ("S4", open.into(), Some(migrated_to("OPEN"))),
        ("S5", accepted.into(), Some(migrated_to("ACCEPTED"))),
        ("C1", implemented(), Some(migrated_to("IMPLEMENTED"))),
        ("C2", implemented(), Some(migrated_to("IMPLEMENTED"))),
        ("C3", implemented(), Some(migrated_to("IMPLEMENTED"))),
        ("C4", implemented(), Some(migrated_to("IMPLEMENTED"))),
        // Migrated to DONE, later PARKED: no exception for a DONE target.
        ("D1", parked.into(), Some(migrated_to("DONE"))),
        // Migrated to DONE and DONE: nothing to except.
        ("D2", done, Some(format!("Migrated (owner batch 4, 2026-11-11, to DONE), evidence: D2-STATUS-DONE.MD, fix {}", &h0[..9]))),
        // Batches 1-3 (no `to <STATE>`): none.
        ("N1", implemented(), Some("Migrated (owner batch 3, 2026-11-11), evidence: legacy".into())),
        // Not migrated: none.
        ("N2", implemented(), None),
        // The words only in the evidence: none.
        ("E1", implemented(), Some("Migrated (owner batch 3, 2026-11-11), evidence: x, to IMPLEMENTED), evidence: y".into())),
        // A numbered receipt: none.
        ("M5", implemented(), Some(migrated_to("IMPLEMENTED"))),
    ];
    for (id, l3, below) in &cases {
        r.write(&format!("TASK/{id}.md"), &ticket(id, l3, below.as_deref()));
    }
    r.write("TASK/C3-STATUS-DONE.MD", receipt);
    let d2_evidence = format!("D2-STATUS-DONE.MD, fix {}", &h0[..9]);
    let mut rows4: Vec<(&str, &str, &str)> = [
        ("S1", "IMPLEMENTED"),
        ("S2", "PARKED"),
        ("S3", "OBSOLETE"),
        ("S4", "OPEN"),
        ("S5", "ACCEPTED"),
        ("C1", "IMPLEMENTED"),
        ("C2", "IMPLEMENTED"),
        ("C3", "IMPLEMENTED"),
        ("C4", "IMPLEMENTED"),
        ("D1", "DONE"),
        ("M5", "IMPLEMENTED"),
    ]
    .map(|(id, st)| (id, st, EVIDENCE))
    .to_vec();
    rows4.push(("D2", "DONE", &d2_evidence));
    write_batch(&r, "4", &rows4);
    write_batch(&r, "3", &[("N1", "IMPLEMENTED", "legacy"), ("E1", "IMPLEMENTED", "x, to IMPLEMENTED), evidence: y")]);
    commit(&r, "docs(task): migrate legacy tickets, owner batch 4");
    // Commit 3, after the migration: C1's receipt changed, C2's added.
    r.write("TASK/C1-STATUS-DONE.MD", "# R — legacy receipt\n\nChanged after the migration.\n");
    r.write("TASK/C2-STATUS-DONE.MD", receipt);
    commit(&r, "docs(task): receipts after the migration");
    // Uncommitted: C4's receipt changed.
    r.write("TASK/C4-STATUS-DONE.MD", "# R — legacy receipt\n\nEdited, not committed.\n");
    let got: BTreeSet<String> =
        rejected(&r).into_iter().filter(|(_, c)| *c == Code::DoneReceiptNotDone).map(|(k, _)| k).collect();
    assert_eq!(got, BTreeSet::from(["C1", "C2", "C3", "C4", "D1", "N1", "N2", "E1", "M5"].map(String::from)));
    let other: Vec<(String, Code)> = rejected(&r).into_iter().filter(|(_, c)| *c != Code::DoneReceiptNotDone).collect();
    assert!(other.is_empty(), "{other:?}");
    // The staged view: C4's change staged is still newer than the migration.
    r.git(&["add", "TASK/C4-STATUS-DONE.MD"]);
    let mut iv = View::index(&r.0).unwrap();
    let ai = Analysis::load(&mut iv).unwrap();
    let mut gr = GitRepo(r.0.clone());
    assert!(ticket_violations(&ai, "C4", &BTreeSet::new(), &mut gr).iter().any(|v| v.code == Code::DoneReceiptNotDone));
    assert!(ticket_violations(&ai, "S1", &BTreeSet::new(), &mut gr).iter().all(|v| v.code != Code::DoneReceiptNotDone));
    // S1 with both the legacy and a numbered receipt: only the numbered one.
    r.write("TASK/S1-STATUS-DONE-2.MD", receipt);
    let mut view = View::disk(&r.0);
    let a = Analysis::load(&mut view).unwrap();
    let v = ticket_violations(&a, "S1", &BTreeSet::new(), &mut gr);
    let msgs: Vec<&str> = v.iter().filter(|v| v.code == Code::DoneReceiptNotDone).map(|v| v.msg.as_str()).collect();
    assert!(msgs.len() == 1 && msgs[0].contains("S1-STATUS-DONE-2.MD"), "{msgs:?}");
}

/// `accept` PASS on a migrated IMPLEMENTED ticket with a legacy DONE receipt
/// writes `-2`, line 3 points to it, the old receipt keeps its bytes; after
/// a reopen the third acceptance writes `-3`; `status-check` passes after
/// each.
#[test]
fn accept_writes_numbered_receipts_and_never_overwrites_one() {
    let r = repo("tool55-accept");
    let legacy: &[u8] = b"# X1 \xe2\x80\x94 receipt\n\nERLEDIGT 2026-09-30, open clause: Pi run.\n";
    r.write("TASK/X1.md", &ticket("X1", "Status: ERLEDIGT · Prioritaet: P1", None));
    fs::write(r.0.join("TASK/X1-STATUS-DONE.MD"), legacy).unwrap();
    commit(&r, "base");
    r.write("TASK/X1.md", &ticket("X1", &implemented(), Some(MIGRATED)));
    write_batch(&r, "4", &[("X1", "IMPLEMENTED", EVIDENCE)]);
    commit(&r, "docs(task): migrate legacy tickets, owner batch 4");
    r.write("TASK/X1-STATUS-UNFINISHED.MD", "# X1 — receipt\n\nRole: developer\nAgent: dev-a\n\n## Done\n\nWork.\n");
    let h = commit(&r, "feat(x): the change (X1)");
    assert_eq!(rejected(&r), [], "the exception holds before the acceptance");

    accept(&r.0, &args(&["X1", "--commit", &h, "--result", "PASS", "--note", "green", "--agent", "tester-b"]), D).unwrap();
    assert_eq!(out(&r, &["status", "--porcelain"]), "", "everything committed");
    assert_eq!(out(&r, &["show", "--name-only", "--format=", "HEAD"]), "TASK/X1-STATUS-DONE-2.MD\nTASK/X1.md");
    assert!(out(&r, &["log", "-n1", "--format=%B"]).contains("receipt X1-STATUS-DONE-2.MD"));
    assert_eq!(fs::read(r.0.join("TASK/X1-STATUS-DONE.MD")).unwrap(), legacy, "the legacy receipt is byte-identical");
    let t = fs::read_to_string(r.0.join("TASK/X1.md")).unwrap();
    assert_eq!(t.lines().nth(2).unwrap(), format!("Status: DONE ({D}) | Priority: P1 | Commit: {} | Receipt: X1-STATUS-DONE-2.MD", &h[..9]));
    let rc2 = fs::read(r.0.join("TASK/X1-STATUS-DONE-2.MD")).unwrap();
    let rc2s = String::from_utf8_lossy(&rc2);
    assert!(rc2s.contains("Role: tester\nAgent: tester-b\n") && rc2s.contains(&format!("Commit: {}", &h[..9])), "{rc2s}");
    let c = check(&r.0, &CheckOpts { history: true, ..CheckOpts::default() }).unwrap();
    assert!(c.rejected.is_empty() && c.reported.is_empty(), "{:?} {:?}", c.rejected, c.reported);

    // A reopen (line 3 back to IMPLEMENTED by a fix commit), then the third
    // acceptance: -3, both older receipts unchanged.
    r.write("TASK/X1.md", &with_line3(&t, "Status: IMPLEMENTED (2026-11-12) | Priority: P1 | Commit: self | Missing: retest").unwrap());
    let h2 = commit(&r, "fix(x): the follow-up (X1)");
    accept(&r.0, &args(&["X1", "--commit", &h2, "--result", "PASS", "--agent", "tester-c"]), "2026-11-12").unwrap();
    assert_eq!(out(&r, &["show", "--name-only", "--format=", "HEAD"]), "TASK/X1-STATUS-DONE-3.MD\nTASK/X1.md");
    assert_eq!(fs::read(r.0.join("TASK/X1-STATUS-DONE.MD")).unwrap(), legacy);
    assert_eq!(fs::read(r.0.join("TASK/X1-STATUS-DONE-2.MD")).unwrap(), rc2);
    let t3 = fs::read_to_string(r.0.join("TASK/X1.md")).unwrap();
    assert!(t3.lines().nth(2).unwrap().ends_with(&format!("Commit: {} | Receipt: X1-STATUS-DONE-3.MD", &h2[..9])), "{t3}");
    assert_eq!(rejected(&r), []);

    // FAIL keeps its behaviour: no DONE receipt, the note appended.
    r.write("TASK/X1.md", &with_line3(&t3, "Status: IMPLEMENTED (2026-11-13) | Priority: P1 | Commit: self | Missing: retest").unwrap());
    let h3 = commit(&r, "fix(x): again (X1)");
    accept(&r.0, &args(&["X1", "--commit", &h3, "--result", "FAIL", "--note", "red", "--agent", "tester-c"]), "2026-11-13").unwrap();
    assert_eq!(out(&r, &["show", "--name-only", "--format=", "HEAD"]), "TASK/X1-STATUS-UNFINISHED.MD");
    assert!(!r.0.join("TASK/X1-STATUS-DONE-4.MD").exists());
}

/// Rework b: `accept` on a migrated ticket without a `Role: developer`
/// receipt accepts and records the legacy developer; the self-acceptance
/// flag does not fire for that receipt. A ticket without a `Migrated` line
/// and without a developer is still refused.
#[test]
fn accept_of_a_migrated_ticket_without_a_developer_records_the_legacy_developer() {
    let r = repo("tool55-legacy-dev");
    let legacy: &[u8] = b"# X1 \xe2\x80\x94 receipt\n\nERLEDIGT 2026-09-30.\n";
    r.write("TASK/X1.md", &ticket("X1", "Status: ERLEDIGT · Prioritaet: P1", None));
    fs::write(r.0.join("TASK/X1-STATUS-DONE.MD"), legacy).unwrap();
    r.write("TASK/Y1.md", &ticket("Y1", "Status: OPEN (2026-11-10) | Priority: P1", None));
    commit(&r, "base");
    r.write("TASK/X1.md", &ticket("X1", &implemented(), Some(MIGRATED)));
    write_batch(&r, "4", &[("X1", "IMPLEMENTED", EVIDENCE)]);
    let hx = commit(&r, "docs(task): migrate legacy tickets, owner batch 4 (X1)");
    r.write("TASK/Y1.md", &ticket("Y1", &implemented(), None));
    let hy = commit(&r, "feat(y): the change (Y1)");

    // Not migrated, no developer: refused as before.
    let e = accept(&r.0, &args(&["Y1", "--commit", &hy, "--result", "PASS", "--agent", "tester-b"]), D).unwrap_err();
    assert!(e.contains("no developer recorded for Y1"), "{e}");

    // Migrated, no developer: accepted.
    accept(&r.0, &args(&["X1", "--commit", &hx, "--result", "PASS", "--agent", "tester-b"]), D).unwrap();
    let rc = fs::read_to_string(r.0.join("TASK/X1-STATUS-DONE-2.MD")).unwrap();
    let header: String = rc.lines().take_while(|l| !l.starts_with("## ")).collect::<Vec<_>>().join("\n");
    assert!(header.contains("Role: tester\nAgent: tester-b\nDeveloper: legacy, developer unknown\nSelf-acceptance: not applicable (ADR-037 point 12)"), "{rc}");
    assert!(rc.contains("Developer: legacy, developer unknown. Line 3 before"), "{rc}");
    assert_eq!(fs::read(r.0.join("TASK/X1-STATUS-DONE.MD")).unwrap(), legacy);
    let c = check(&r.0, &CheckOpts::default()).unwrap();
    assert!(c.rejected.iter().all(|v| v.key != "X1") && c.reported.is_empty(), "{:?} {:?}", c.rejected, c.reported);

    // The self-acceptance flag does not fire for the legacy receipt, even
    // when a developer receipt naming the acceptor turns up later; for an
    // ordinary DONE receipt it would.
    r.write("TASK/X1-STATUS-UNFINISHED.MD", "# X1 — receipt\n\nRole: developer\nAgent: tester-b\n\n## Done\n");
    let mut view = View::disk(&r.0);
    let a = Analysis::load(&mut view).unwrap();
    let mut gr = GitRepo(r.0.clone());
    assert!(ticket_violations(&a, "X1", &BTreeSet::new(), &mut gr).iter().all(|v| v.code != Code::SelfAccept));
    r.write("TASK/X1-STATUS-DONE-2.MD", &rc.replace("Developer: legacy, developer unknown\n", ""));
    let mut view = View::disk(&r.0);
    let a = Analysis::load(&mut view).unwrap();
    assert!(ticket_violations(&a, "X1", &BTreeSet::new(), &mut gr).iter().any(|v| v.code == Code::SelfAccept));
}

// ---------------------------------------------------------------------------
// TOOL58: the committed batch rows the Migrated lines above need
// ---------------------------------------------------------------------------

/// The fields column of a batch row of target state `st` for ticket `id`.
pub(super) fn row_fields(id: &str, st: &str) -> String {
    match st {
        "IMPLEMENTED" => "Priority: P1 | Commit: 0123abcd5 | Missing: m".into(),
        "PARKED" => "Priority: P3 | Reason: r | Reopen: o".into(),
        "OBSOLETE" => "Priority: P3 | Reason: withdrawn".into(),
        "ACCEPTED" => "Priority: P2 | Ref: ADR-001".into(),
        "DONE" => format!("Priority: P2 | Commit: 0123abcd5 | Receipt: {id}-STATUS-DONE.MD"),
        _ => "Priority: P2".into(),
    }
}

/// Writes the owner-approved batch file `n` with rows `(id, state,
/// evidence)` (TOOL58: every `Migrated` line needs its committed row).
pub(super) fn write_batch(r: &TempRepo, n: &str, rows: &[(&str, &str, &str)]) {
    let mut t = String::from("# approved: owner 2026-11-11 test batch\nid\tstate\tfields\tevidence\n");
    for (id, st, ev) in rows {
        t.push_str(&format!("{id}\t{st}\t{}\t{ev}\n", row_fields(id, st)));
    }
    r.write(&migrate::batch_file(n), &t);
}

/// The evidence of [`MIGRATED`].
const EVIDENCE: &str = "legacy ERLEDIGT with an open clause";
