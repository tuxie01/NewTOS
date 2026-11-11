//! Host tests of TOOL58: every `Migrated` line is checked against the
//! committed, owner-approved batch file it names, by one check
//! ([`verify_migrated`]) that `status-check`, the TOOL55 receipt exception,
//! the `done-no-receipt` evidence rule and `accept`'s legacy-developer path
//! share.

use super::tool55_tests::write_batch;
use super::*;
use crate::doc_check::tests::TempRepo;

const D: &str = "2026-11-11";
const EV: &str = "legacy ERLEDIGT, receipt TASK/X-STATUS-DONE.MD";

fn git_text(r: &TempRepo, args: &[&str]) -> String {
    let o = Command::new("git").args(args).current_dir(&r.0).output().unwrap();
    assert!(o.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&o.stderr));
    String::from_utf8_lossy(&o.stdout).trim().to_string()
}

fn commit(r: &TempRepo, msg: &str) -> String {
    r.git(&["add", "-A"]);
    r.git(&["commit", "-q", "-m", msg]);
    git_text(r, &["rev-parse", "HEAD"])
}

fn repo(name: &str) -> TempRepo {
    let r = TempRepo::new(name);
    for (k, v) in [("user.name", "t"), ("user.email", "t@t"), ("commit.gpgsign", "false"), ("core.hooksPath", "/dev/null")] {
        r.git(&["config", k, v]);
    }
    r.write("docs/MEMORY.md", "# Memory\n\n## ADR-001: one\n");
    r
}

/// A migrated ticket: line 3 `line3`, the `Migrated` line `m` below it.
fn migrated_ticket(id: &str, line3: &str, m: &str) -> String {
    format!("# {id} — a title\n\n{line3}\n\n{m}\n\nLine 3 before the migration:\nStatus: ERLEDIGT\n\nBody.\n")
}

fn implemented() -> &'static str {
    "Status: IMPLEMENTED (2026-11-11) | Priority: P1 | Commit: self | Missing: tester acceptance"
}

/// The `migrated-unverified` messages of the working-tree view, by ticket.
fn unverified(r: &TempRepo) -> BTreeMap<String, String> {
    check(&r.0, &CheckOpts::default())
        .unwrap()
        .rejected
        .into_iter()
        .filter(|v| v.code == Code::MigratedUnverified)
        .map(|v| (v.key, v.msg))
        .collect()
}

/// The parser reads what `migrate` writes, with and without `to <STATE>`.
#[test]
fn parse_migrated_reads_what_migrate_writes() {
    let line = migrate::migrated_line("14", D, State::Parked, "was IMPLEMENTED, x), evidence: y");
    let m = parse_migrated(&line).unwrap();
    assert_eq!(m, MigratedMark { batch: "14", date: D, to: Some(State::Parked), evidence: "was IMPLEMENTED, x), evidence: y" });
    let old = "Migrated (owner batch 3, 2026-11-11), evidence: receipt TASK/A114-STATUS-DONE.MD exists";
    assert_eq!(parse_migrated(old).unwrap().to, None);
    for bad in [
        "Migrated (owner batch 3, 2026-11-11, to IMPLEMENTIERT), evidence: x",
        "Migrated (owner batch x, 2026-11-11), evidence: x",
        "Migrated (owner batch 3, 2026-13-11), evidence: x",
        "Migrated (owner batch 3, 2026-11-11, IMPLEMENTED), evidence: x",
        "Migrated (owner batch 3, 2026-11-11, to DONE, extra), evidence: x",
        "Migrated (owner batch 3, 2026-11-11) evidence: x",
    ] {
        assert_eq!(parse_migrated(bad), None, "{bad}");
    }
}

/// Work 4: a hand-written or mismatching line is rejected, naming ticket,
/// line and batch; a real line (with and without `to <STATE>`) passes. The
/// batch file counts only as committed (`HEAD`), or staged under
/// `--staged`.
#[test]
fn hand_written_and_mismatching_lines_are_rejected_real_ones_pass() {
    let r = repo("tool58-lines");
    let ok_to = migrate::migrated_line("5", D, State::Implemented, EV);
    let ok_old = format!("Migrated (owner batch 2, {D}), evidence: {EV}");
    let cases: [(&str, String); 10] = [
        // Real lines: `to <STATE>` (batch 14 on), none (batches 1-13).
        ("R1", ok_to.clone()),
        ("R2", ok_old.clone()),
        // No batch file.
        ("H1", migrate::migrated_line("7", D, State::Implemented, EV)),
        // The wrong batch number: batch 2 exists, the row is in batch 5.
        ("H2", migrate::migrated_line("2", D, State::Implemented, EV)),
        // The id in no batch.
        ("H3", ok_to.clone()),
        // The state differs from the row.
        ("H4", migrate::migrated_line("5", D, State::Parked, EV)),
        // The evidence differs from the row (one character).
        ("H5", migrate::migrated_line("5", D, State::Implemented, &EV.replace("ERLEDIGT", "ERLEDIGT."))),
        // A batch file without the approval line.
        ("H6", migrate::migrated_line("8", D, State::Implemented, EV)),
        // A batch file in the working tree only.
        ("H7", migrate::migrated_line("9", D, State::Implemented, EV)),
        // Not of the form.
        ("H8", format!("Migrated (owner batch 5, {D}, to IMPLEMENTIERT), evidence: {EV}")),
    ];
    for (id, m) in &cases {
        r.write(&format!("TASK/{id}.md"), &migrated_ticket(id, implemented(), m));
    }
    let rows5 = [("R1", "IMPLEMENTED", EV), ("H2", "IMPLEMENTED", EV), ("H4", "IMPLEMENTED", EV), ("H5", "IMPLEMENTED", EV), ("H8", "IMPLEMENTED", EV)];
    write_batch(&r, "5", &rows5);
    write_batch(&r, "2", &[("R2", "OPEN", EV)]);
    r.write(&migrate::batch_file("8"), &format!("id\tstate\tfields\tevidence\nH6\tIMPLEMENTED\t{}\t{EV}\n", super::tool55_tests::row_fields("H6", "IMPLEMENTED")));
    commit(&r, "base");
    write_batch(&r, "9", &[("H7", "IMPLEMENTED", EV)]);

    let got = unverified(&r);
    let ids: BTreeSet<&str> = got.keys().map(String::as_str).collect();
    assert_eq!(ids, BTreeSet::from(["H1", "H2", "H3", "H4", "H5", "H6", "H7", "H8"]), "{got:#?}");
    let expect = [
        ("H1", "batch 7: TASK/migration/batch-7.tsv is not committed"),
        ("H2", "batch 2: TASK/migration/batch-2.tsv has no row for H2"),
        ("H3", "batch 5: TASK/migration/batch-5.tsv has no row for H3"),
        ("H4", "batch 5: the line says `to PARKED`, row TASK/migration/batch-5.tsv:5 says IMPLEMENTED"),
        ("H5", "batch 5: the evidence differs from row TASK/migration/batch-5.tsv:6"),
        ("H6", "batch 8: TASK/migration/batch-8.tsv is no approved batch file"),
        ("H7", "batch 9: TASK/migration/batch-9.tsv is not committed"),
        ("H8", "not of the form"),
    ];
    for (id, needle) in expect {
        let msg = &got[id];
        let (_, m) = cases.iter().find(|(i, _)| *i == id).unwrap();
        assert!(msg.contains(needle) && msg.starts_with(&format!("TASK/{id}.md: {m:?}: ")), "{id}: {msg}");
    }
    // Under --staged the index counts: H7's batch file staged verifies it.
    r.git(&["add", "-A"]);
    let mut iv = View::index(&r.0).unwrap();
    let ai = Analysis::load(&mut iv).unwrap();
    assert!(verify_migrated(&ai.aids, "H7", &cases[8].1).is_ok());
    assert!(verify_migrated(&ai.aids, "R1", &ok_to).is_ok() && verify_migrated(&ai.aids, "R2", &ok_old).is_ok());
    // ... and the working-tree view still reads HEAD.
    let mut dv = View::disk(&r.0);
    let ad = Analysis::load(&mut dv).unwrap();
    assert!(verify_migrated(&ad.aids, "H7", &cases[8].1).is_err());
    assert_eq!(migrated_counts(&ad), (2, 10));
    // A Migrated line of an unverified kind is never baselined.
    assert!(!Code::MigratedUnverified.baselinable() && Code::MigratedUnverified.rejecting());
}

/// Work 2: none of the consumers trusts a line the check rejects — the
/// TOOL55 receipt exception, the `done-no-receipt` evidence rule and
/// `accept`'s legacy-developer path.
#[test]
fn no_consumer_trusts_an_unverified_line() {
    let r = repo("tool58-consumers");
    let receipt = "# R — legacy receipt\n\nDone, open clause.\n";
    for id in ["E1", "E2", "F1", "F2", "A1", "A2"] {
        r.write(&format!("TASK/{id}.md"), &format!("# {id} — a title\n\nStatus: ERLEDIGT\n\nBody.\n"));
        r.write(&format!("TASK/{id}-STATUS-DONE.MD"), receipt);
    }
    let h = commit(&r, "base");
    let h9 = &h[..9];
    let done = |id: &str| format!("Status: DONE (2026-11-11) | Priority: P2 | Commit: {h9} | Receipt: {id}-STATUS-DONE.MD");
    let fix = format!("fix {h9}");
    // E: the receipt exception; F: a DONE whose evidence is the Migrated
    // line only; A: accept without a recorded developer. The *1 tickets
    // have their row, the *2 tickets a hand-written line.
    r.write("TASK/E1.md", &migrated_ticket("E1", implemented(), &migrate::migrated_line("4", D, State::Implemented, EV)));
    r.write("TASK/E2.md", &migrated_ticket("E2", implemented(), &migrate::migrated_line("4", D, State::Implemented, EV)));
    r.write("TASK/F1.md", &migrated_ticket("F1", &done("F1"), &migrate::migrated_line("4", D, State::Done, &fix)));
    r.write("TASK/F2.md", &migrated_ticket("F2", &done("F2"), &migrate::migrated_line("4", D, State::Done, &fix)));
    r.write("TASK/A1.md", &migrated_ticket("A1", implemented(), &migrate::migrated_line("4", D, State::Implemented, EV)));
    r.write("TASK/A2.md", &migrated_ticket("A2", implemented(), &migrate::migrated_line("4", D, State::Implemented, EV)));
    write_batch(&r, "4", &[("E1", "IMPLEMENTED", EV), ("F1", "DONE", &fix), ("A1", "IMPLEMENTED", EV)]);
    let hm = commit(&r, "docs(task): migrate legacy tickets, owner batch 4 (A1, A2)");

    let c = check(&r.0, &CheckOpts::default()).unwrap();
    let got: BTreeSet<(String, Code)> = c.rejected.iter().map(|v| (v.key.clone(), v.code)).collect();
    let want: BTreeSet<(String, Code)> = [
        ("E2", Code::MigratedUnverified),
        ("E2", Code::DoneReceiptNotDone),
        ("F2", Code::MigratedUnverified),
        ("F2", Code::DoneNoReceipt),
        ("A2", Code::MigratedUnverified),
        ("A2", Code::DoneReceiptNotDone),
    ]
    .into_iter()
    .map(|(k, c)| (k.to_string(), c))
    .collect();
    assert_eq!(got, want, "{:#?}", c.rejected);

    // accept: the verified line opens the legacy-developer path, the
    // hand-written one does not.
    let a = |id: &str| -> Vec<String> {
        ["--commit", &hm, "--result", "PASS", "--agent", "tester-b"].iter().fold(vec![id.to_string()], |mut v, s| {
            v.push(s.to_string());
            v
        })
    };
    let e = accept(&r.0, &a("A2"), D).unwrap_err();
    assert!(e.contains("no developer recorded for A2") && e.contains("batch 4: TASK/migration/batch-4.tsv has no row for A2"), "{e}");
    accept(&r.0, &a("A1"), D).unwrap();
    let rc = fs::read_to_string(r.0.join("TASK/A1-STATUS-DONE-2.MD")).unwrap();
    assert!(rc.contains("Developer: legacy, developer unknown"), "{rc}");
}

/// Work 3: every `Migrated` line of the real `TASK/` (batches 1-22) matches
/// its committed batch row; `status-check` prints the count.
#[test]
fn the_real_migrated_lines_all_match_their_batch_rows() {
    let root = crate::workspace_root();
    let mut view = View::disk(&root);
    let a = Analysis::load(&mut view).unwrap();
    let bad: Vec<String> = a.tickets.values().flat_map(|t| migrated_violations(&a, t)).map(|v| v.msg).collect();
    assert!(bad.is_empty(), "{bad:#?}");
    let (ok, all) = migrated_counts(&a);
    assert!(all > 0 && ok == all, "{ok} of {all}");
    for n in 1..=22 {
        assert!(matches!(a.aids.batches.get(&n.to_string()), Some(Ok(_))), "batch {n}");
    }
}

// ---------------------------------------------------------------------------
// The rework (tester PARTIAL on ca1552318): the batch files themselves
// ---------------------------------------------------------------------------

/// The rework on the real repository: the migration is closed and
/// `TASK/migration/` in `HEAD` equals the closing commit's.
#[test]
fn the_real_migration_dir_equals_the_closing_commit() {
    let root = crate::workspace_root();
    let mut view = View::disk(&root);
    let a = Analysis::load(&mut view).unwrap();
    assert!(migration_closing_commit(&root).is_some());
    let frozen = migration_evidence(&root, &a, &CheckOpts::default(), &mut BTreeSet::new()).unwrap();
    assert!(frozen.is_empty(), "{frozen:#?}");
}

/// The staged check the pre-commit hook runs (`--staged --new-only`).
fn staged_check(r: &TempRepo) -> Vec<Violation> {
    let o = CheckOpts { staged: true, new_only: true, ..CheckOpts::default() };
    check(&r.0, &o).unwrap().rejected
}

fn pairs(v: Vec<Violation>) -> BTreeSet<(String, Code)> {
    v.into_iter().map(|v| (v.key, v.code)).collect()
}

fn set(p: &[(&str, Code)]) -> BTreeSet<(String, Code)> {
    p.iter().map(|(k, c)| (k.to_string(), *c)).collect()
}

fn full_check(r: &TempRepo) -> Vec<Violation> {
    check(&r.0, &CheckOpts::default()).unwrap().rejected
}

/// `text` without its first line.
fn without_line1(text: &str) -> String {
    text.split_inclusive('\n').skip(1).collect()
}

/// A repository with T1 (batch 4) and T2 (batch 5), both verified,
/// committed.
fn two_batches(name: &str) -> TempRepo {
    let r = repo(name);
    for (id, n) in [("T1", "4"), ("T2", "5")] {
        r.write(&format!("TASK/{id}.md"), &migrated_ticket(id, implemented(), &migrate::migrated_line(n, D, State::Implemented, EV)));
        write_batch(&r, n, &[(id, "IMPLEMENTED", EV)]);
    }
    commit(&r, "base");
    r
}

/// Rework 1: a staged batch file makes the staged check verify every
/// ticket naming that batch, so an edited row or a dropped approval line
/// rejects in the commit itself (findings a and b); the tickets of another
/// batch stay unchecked; an untouched tree passes.
#[test]
fn a_staged_batch_file_rechecks_every_ticket_naming_it() {
    let r = two_batches("tool58-staged-batch");
    assert_eq!(pairs(staged_check(&r)), set(&[]));
    let b4 = migrate::batch_file("4");
    let orig = fs::read_to_string(r.0.join(&b4)).unwrap();
    // (a) a row edited after the approval.
    r.write(&b4, &orig.replace(EV, "another evidence"));
    r.git(&["add", &b4]);
    assert_eq!(pairs(staged_check(&r)), set(&[("T1", Code::MigratedUnverified)]));
    // (b) the approval line deleted.
    r.write(&b4, &without_line1(&orig));
    r.git(&["add", &b4]);
    let rej = staged_check(&r);
    assert!(rej.len() == 1 && rej[0].key == "T1" && rej[0].msg.contains("batch 4: TASK/migration/batch-4.tsv is no approved batch file"), "{rej:#?}");
    // Back to the committed file: passes.
    r.write(&b4, &orig);
    r.git(&["add", &b4]);
    assert_eq!(pairs(staged_check(&r)), set(&[]));
    assert!(full_check(&r).is_empty());
}

/// Rework 2: once `closed.txt` is committed, every staged addition, change
/// or deletion under `TASK/migration/` rejects, naming the file (the
/// consistent forgery of finding c included); a full check compares `HEAD`
/// with the closing commit, so a change committed past the hook rejects
/// too, also once `closed.txt` itself is gone; the untouched tree passes.
#[test]
fn the_closed_migration_is_frozen() {
    let r = two_batches("tool58-frozen");
    let hist = "TASK/migration/historical-receipts.tsv";
    r.write(hist, "# approved: owner 2026-11-11\n");
    r.write(migrate::CLOSED_FILE, "owner decision 2026-11-11: migration closed\n");
    let closing = commit(&r, "close the migration");
    assert_eq!(migration_closing_commit(&r.0).as_deref(), Some(closing.as_str()));
    assert_eq!(pairs(staged_check(&r)), set(&[]));
    assert!(full_check(&r).is_empty());
    let b4 = migrate::batch_file("4");
    let orig = fs::read_to_string(r.0.join(&b4)).unwrap();
    let t1 = fs::read_to_string(r.0.join("TASK/T1.md")).unwrap();
    let undo = |added: &[&str]| {
        r.git(&["reset", "-q", "HEAD", "--", "TASK"]);
        r.git(&["checkout", "-q", "HEAD", "--", "TASK"]);
        for p in added {
            let _ = fs::remove_file(r.0.join(p));
        }
        assert_eq!(pairs(staged_check(&r)), set(&[]), "undone");
    };
    // A row edited consistently with its ticket line (finding c): the
    // ticket verifies against the staged row, the batch file is frozen.
    r.write(&b4, &orig.replace(EV, "forged"));
    r.write("TASK/T1.md", &t1.replace(EV, "forged"));
    r.git(&["add", "-A"]);
    assert_eq!(pairs(staged_check(&r)), set(&[(&b4, Code::MigrationFrozen)]));
    undo(&[]);
    // The approval line deleted.
    r.write(&b4, &without_line1(&orig));
    r.git(&["add", "-A"]);
    assert_eq!(pairs(staged_check(&r)), set(&[(&b4, Code::MigrationFrozen), ("T1", Code::MigratedUnverified)]));
    undo(&[]);
    // A new batch file.
    write_batch(&r, "6", &[("T1", "IMPLEMENTED", EV)]);
    r.git(&["add", "-A"]);
    assert_eq!(pairs(staged_check(&r)), set(&[("TASK/migration/batch-6.tsv", Code::MigrationFrozen)]));
    undo(&["TASK/migration/batch-6.tsv"]);
    // closed.txt deleted, the historical list changed.
    r.git(&["rm", "-q", migrate::CLOSED_FILE]);
    r.write(hist, "# approved: owner 2026-11-12\n");
    r.git(&["add", "-A"]);
    let rej = staged_check(&r);
    assert_eq!(pairs(rej.clone()), set(&[(migrate::CLOSED_FILE, Code::MigrationFrozen), (hist, Code::MigrationFrozen)]));
    assert!(rej.iter().any(|v| v.msg.starts_with("TASK/migration/closed.txt: staged deletion after the migration was closed")), "{rej:#?}");
    undo(&[]);
    // Committed past the hook: the full check compares HEAD with the
    // closing commit — the consistent forgery, then closed.txt deleted and
    // a batch added.
    r.write(&b4, &orig.replace(EV, "forged"));
    r.write("TASK/T1.md", &t1.replace(EV, "forged"));
    commit(&r, "forge");
    let rej = full_check(&r);
    assert!(rej.len() == 1 && rej[0].key == b4 && rej[0].msg.contains("differs in HEAD from its blob in"), "{rej:#?}");
    r.git(&["rm", "-q", migrate::CLOSED_FILE]);
    write_batch(&r, "7", &[("T2", "IMPLEMENTED", EV)]);
    commit(&r, "unfreeze");
    assert_eq!(
        pairs(full_check(&r)),
        set(&[(&b4, Code::MigrationFrozen), (migrate::CLOSED_FILE, Code::MigrationFrozen), ("TASK/migration/batch-7.tsv", Code::MigrationFrozen)])
    );
}
