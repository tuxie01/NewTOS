//! Host tests of the ticket tools (TOOL41 Work 7).

use super::*;
use crate::doc_check::tests::TempRepo;
use std::process::Stdio;

const D: &str = "2026-11-11";

fn ticket(id: &str, line3: &str) -> String {
    format!("# {id} — a title\n\n{line3}\n\nBody.\n")
}

fn errs(id: &str, line: &str) -> Vec<FieldError> {
    parse_line3(id, line).unwrap_err().errors
}

// ---------------------------------------------------------------------------
// The parser
// ---------------------------------------------------------------------------

#[test]
fn every_state_parses_with_its_fields() {
    let ok = [
        "Status: OPEN (2026-11-10) | Priority: P2",
        "Status: IMPLEMENTED (2026-11-08) | Priority: P4 | Commit: 9c72d3ec | Missing: Pi run, scenario \"Pi handover\"",
        "Status: IMPLEMENTED (2026-11-11) | Priority: P1 | Commit: self | Missing: tester acceptance",
        "Status: DONE (2026-11-10) | Priority: P4 | Commit: 8ef6cf96 | Receipt: X-STATUS-DONE.MD",
        "Status: PARKED (2026-10-31) | Priority: P3 | Reason: no card program calls Bioskeys | Reopen: a port calls it",
        "Status: ACCEPTED (2026-10-30) | Priority: P2 | Ref: ADR-033",
        "Status: ACCEPTED (2026-10-30) | Priority: P2 | Ref: docs/COMPATIBILITY.md#bioskeys",
        "Status: OBSOLETE (2026-09-20) | Priority: P3 | Reason: duplicate | Target: FS10",
        "Status: OBSOLETE (2026-09-20) | Priority: P3 | Reason: withdrawn",
    ];
    for l in ok {
        let p = parse_line3("X", l).unwrap_or_else(|e| panic!("{l}: {e:?}"));
        assert_eq!(p.render(), l);
    }
    let p = parse_line3("X", ok[2]).unwrap();
    assert_eq!((p.state, p.priority, p.get("Commit")), (State::Implemented, 1, Some("self")));
}

#[test]
fn every_missing_field_is_named() {
    for st in STATES {
        for (i, key) in st.required().iter().enumerate() {
            // All fields but `key`.
            let mut l = format!("Status: {} ({D}) | Priority: P2", st.name());
            for (j, k) in st.required().iter().enumerate() {
                if j != i {
                    let v = match *k {
                        "Commit" => "abcdef1",
                        "Receipt" => "X-STATUS-DONE.MD",
                        "Ref" => "ADR-001",
                        _ => "something",
                    };
                    l.push_str(&format!(" | {k}: {v}"));
                }
            }
            let e = parse_line3("X", &l).unwrap_err();
            assert_eq!(e.state, Some(st));
            assert!(e.errors.iter().any(|e| e.field == *key && e.missing), "{l}: {:?}", e.errors);
        }
    }
    // Priority is mandatory in every state and always second.
    assert!(errs("X", "Status: OPEN (2026-11-10)").iter().any(|e| e.field == "Priority" && e.missing));
    assert!(errs("X", "Status: PARKED (2026-11-10) | Reason: r | Priority: P2 | Reopen: x")
        .iter()
        .any(|e| e.field == "Priority"));
    assert!(errs("X", "Status: OPEN (2026-11-10) | Priority: P5").iter().any(|e| e.field == "Priority"));
}

#[test]
fn form_errors_name_field_and_expectation() {
    // Non-ASCII anywhere on the line.
    let e = errs("X", "Status: OPEN (2026-11-10) | Priority: P2 | Missing: Pi-Abnahme ü");
    assert!(e[0].field == "line 3" && e[0].expected.contains("ASCII"), "{e:?}");
    assert_eq!(parse_line3("X", "Status: OPEN (2026-11-10) — x").unwrap_err().state, Some(State::Open));
    // `self` only for IMPLEMENTED; a hash is hex.
    assert!(errs("X", "Status: DONE (2026-11-10) | Priority: P1 | Commit: self | Receipt: X-STATUS-DONE.MD")
        .iter()
        .any(|e| e.field == "Commit"));
    assert!(errs("X", "Status: IMPLEMENTED (2026-11-10) | Priority: P1 | Commit: HEAD | Missing: m")
        .iter()
        .any(|e| e.field == "Commit"));
    // The receipt is the ticket's own.
    assert!(errs("X", "Status: DONE (2026-11-10) | Priority: P1 | Commit: abcdef1 | Receipt: Y-STATUS-DONE.MD")
        .iter()
        .any(|e| e.field == "Receipt" && e.expected.contains("X-STATUS-DONE.MD")));
    // Keys: no extra, in order.
    assert!(errs("X", "Status: OPEN (2026-11-10) | Priority: P2 | Type: tool").iter().any(|e| e.field == "Type"));
    assert!(!errs("X", "Status: PARKED (2026-11-10) | Priority: P2 | Reopen: x | Reason: y").is_empty());
    // Date, state word, separator, Ref.
    assert!(errs("X", "Status: OPEN (2026-13-10) | Priority: P2").iter().any(|e| e.field == "Status"));
    assert!(errs("X", "Status: OPEN 2026-11-10 | Priority: P2").iter().any(|e| e.field == "Status"));
    assert_eq!(parse_line3("X", "Status: OFFEN · Prioritaet: P2").unwrap_err().state, None);
    assert!(errs("X", "Status: OPEN (2026-11-10) | Priority: P2 |x").iter().any(|e| e.expected.contains('|')));
    assert!(errs("X", "Status: ACCEPTED (2026-11-10) | Priority: P2 | Ref: ADR-33").iter().any(|e| e.field == "Ref"));
    assert!(errs("X", "Status: OPEN (2026-11-10) | Priority: P2 | ").iter().any(|_| true));
}

#[test]
fn legacy_lines_map_by_point_2() {
    let cases = [
        ("Status: **ERLEDIGT** (2026-09-19) · Prioritaet: P2", Some(State::Done)),
        ("Status: OFFEN · Prioritaet: P1", Some(State::Open)),
        ("Status: **BACKLOG** · Prioritaet: P3", Some(State::Parked)),
        ("Status: ZURÜCKGESTELLT · Priorität: P3", Some(State::Parked)),
        ("Status: IMPLEMENTIERT — HW OFFEN · Priorität: P1", Some(State::Implemented)),
        ("Status: BEFUND ERLEDIGT, ABNAHME OFFEN — Pi-Lauf", Some(State::Implemented)),
        ("Status: ABGESCHLOSSEN ALS BLOCKED (2026-10-01)", Some(State::Parked)),
        ("Status: **ACCEPTED-Abweichung** · Prioritaet: P2", Some(State::Accepted)),
        ("Status: GEGENSTANDSLOS (2026-09-21)", Some(State::Obsolete)),
        ("Status: TEIL 1 ERLEDIGT (2026-09-24)", None),
        ("Status: Stufe 1 erledigt", None),
        ("no status", None),
    ];
    for (l, s) in cases {
        assert_eq!(legacy_state(l), s, "{l}");
    }
    assert_eq!(legacy_priority("Status: OFFEN · Priorität: P3 · Typ"), Some(3));
    assert_eq!(raw_status("Status: **ERLEDIGT** 2026-10-17 ([Quittung](A29-STATUS-DONE.MD)) · Prioritaet: P1"), "ERLEDIGT 2026-10-17 (Quittung)");
    let t = Ticket::from_text("A1", &ticket("A1", "Status: OFFEN · Prioritaet: P1"));
    assert!(t.legacy && t.effective == Some(State::Open) && t.status_cell() == "OFFEN");
}

#[test]
fn names_and_subjects() {
    assert!(names("fix(hooks): x (TOOL40)", "TOOL40"));
    assert!(!names("fix(hooks): x (TOOL400)", "TOOL40"));
    assert!(!names("ATOOL40", "TOOL40"));
    assert!(names("see DESK43-U5a", "DESK43"));
    assert!(names("G-AES-011", "G-AES-011"));
    assert_eq!(subject("# comment\n\nfeat(x): y (A1)\n\nbody"), "feat(x): y (A1)");
    for s in ["fix: a", "fix(x): a", "feat(x)!: a", "feat!: a"] {
        assert!(is_fix_or_feat(s), "{s}");
    }
    for s in ["docs(task): fix it", "fixup! x", "refactor(x): a", "feature: x"] {
        assert!(!is_fix_or_feat(s), "{s}");
    }
    assert!(names_commit("Commit `b7bd932a` (implementation)", "b7bd932a"));
    assert!(names_commit("commit b7bd932a1234", "b7bd932a"));
    assert!(!names_commit("commit b7bd932b", "b7bd932a"));
}

#[test]
fn readme_column_and_status_md_claims() {
    let readme = "# R\n\n| ID | Task | Status | Dep |\n|---|---|---|---|\n| [A1](A1.md) | t | OLD | — |\n| [B2](B2.md) | t | keep | — |\n\ntext | not a table\n";
    let out = readme_with_status(readme, &|id| (id == "A1").then(|| "DONE".to_string()));
    assert!(out.contains("| [A1](A1.md) | t | DONE | — |"), "{out}");
    assert!(out.contains("| [B2](B2.md) | t | keep | — |"));
    assert!(out.ends_with("text | not a table\n"));

    let status = "## Current state\n\n- **P2:** [T1](../TASK/T1.md) (tool), [T2](../TASK/T2.md) (done 2026-11-01).\n  **Parked 2026-10-31:**\n  [T3](../TASK/T3.md) (reason).\n- other [T4](../TASK/T4.md)\n\n## Blocked\n\n- [T5](../TASK/T5.md) waits.\n";
    let c = status_md_claims(status);
    assert_eq!(
        c.iter().map(|(_, id, c)| (id.as_str(), *c)).collect::<Vec<_>>(),
        [("T1", Claim::Open), ("T2", Claim::Done), ("T3", Claim::Parked), ("T5", Claim::Parked)]
    );
}

/// The report's groups do not reuse the inventory's class names C1–C6,
/// whose (keyword) definitions they do not compute.
#[test]
fn report_groups_do_not_reuse_the_inventory_class_names() {
    let found = [
        v("A1", Code::Form, "x"),
        v("A1", Code::ImplementedFields, "x"),
        v("B1", Code::DoneReceiptNotDone, "x"),
        v("C1x", Code::StatusMd, "x"),
    ];
    let a = Analysis { tickets: BTreeMap::new(), receipts: BTreeMap::new(), unknown: BTreeSet::new() };
    let text = render_report(&counts(&a, &found));
    for c in ["C1", "C2", "C3", "C4", "C5", "C6"] {
        assert!(!text.contains(&format!("    {c} ")), "{text}");
    }
    assert!(text.contains("not-adr037-form") && text.contains("done-not-closed"), "{text}");
    assert_eq!(counts(&a, &found).per_group["not-adr037-form"].len(), 1, "a ticket counts once per group");
    for code in CODES {
        assert!(code.group() == "-" || GROUPS.contains(&code.group()), "{code:?}");
    }
}

// ---------------------------------------------------------------------------
// Checks in a scratch repository
// ---------------------------------------------------------------------------

fn out(r: &TempRepo, args: &[&str]) -> String {
    let o = Command::new("git").args(args).current_dir(&r.0).output().unwrap();
    assert!(o.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&o.stderr));
    String::from_utf8_lossy(&o.stdout).trim().to_string()
}

/// A repository with git identity (for `accept`'s own commit), `docs/MEMORY.md`
/// with ADR-001, and the given tickets committed.
fn repo(name: &str, files: &[(&str, &str)]) -> TempRepo {
    let r = TempRepo::new(name);
    for (k, v) in [("user.name", "t"), ("user.email", "t@t"), ("commit.gpgsign", "false"), ("core.hooksPath", "/dev/null")] {
        r.git(&["config", k, v]);
    }
    r.write("docs/MEMORY.md", "# Memory\n\n## ADR-001: one\n");
    for (p, t) in files {
        r.write(p, t);
    }
    r.git(&["add", "-A"]);
    r.git(&["commit", "-q", "-m", "base"]);
    r
}

fn commit(r: &TempRepo, msg: &str) -> String {
    r.git(&["add", "-A"]);
    r.git(&["commit", "-q", "-m", msg]);
    out(r, &["rev-parse", "HEAD"])
}

fn staged(r: &TempRepo, msg: Option<&str>) -> CheckResult {
    check(&r.0, &CheckOpts { staged: true, new_only: true, message: msg.map(String::from), history: false }).unwrap()
}

fn rejected(c: &CheckResult) -> Vec<(String, Code)> {
    c.rejected.iter().map(|v| (v.key.clone(), v.code)).collect()
}

#[test]
fn staged_gap_check_rejects_each_gap_of_point_13() {
    let r = repo("tickets-gaps", &[("TASK/A1.md", &ticket("A1", "Status: OPEN (2026-11-10) | Priority: P2"))]);
    let base = out(&r, &["rev-parse", "--short=9", "HEAD"]);

    // Nothing staged: nothing checked.
    assert_eq!(staged(&r, None).checked, 0);

    // fix/feat naming an OPEN ticket (commit-msg) rejects; docs naming it does not.
    let fix = "fix(x): repair (A1)\n";
    assert_eq!(rejected(&staged(&r, Some(fix))), [("A1".to_string(), Code::FixFeatState)]);
    assert!(staged(&r, Some("docs(task): note (A1)\n")).rejected.is_empty());
    // IMPLEMENTED | Commit: self | Missing staged: passes.
    r.write("TASK/A1.md", &ticket("A1", "Status: IMPLEMENTED (2026-11-11) | Priority: P2 | Commit: self | Missing: tester"));
    r.git(&["add", "-A"]);
    assert!(staged(&r, Some(fix)).rejected.is_empty());
    assert!(staged(&r, None).rejected.is_empty());
    // ... but only staged: the working copy does not count.
    r.write("TASK/A1.md", &ticket("A1", "Status: OPEN (2026-11-10) | Priority: P2"));
    assert!(staged(&r, Some(fix)).rejected.is_empty());
    r.git(&["add", "-A"]);
    assert_eq!(rejected(&staged(&r, Some(fix))), [("A1".to_string(), Code::FixFeatState)]);
    // IMPLEMENTED without Missing / without Commit.
    r.write("TASK/A1.md", &ticket("A1", "Status: IMPLEMENTED (2026-11-11) | Priority: P2 | Commit: self"));
    r.git(&["add", "-A"]);
    assert_eq!(rejected(&staged(&r, None)), [("A1".to_string(), Code::ImplementedFields)]);
    r.write("TASK/A1.md", &ticket("A1", "Status: IMPLEMENTED (2026-11-11) | Priority: P2 | Missing: x"));
    r.git(&["add", "-A"]);
    assert_eq!(rejected(&staged(&r, None)), [("A1".to_string(), Code::ImplementedFields)]);
    // A DONE receipt while line 3 is not DONE.
    r.write("TASK/A1.md", &ticket("A1", "Status: IMPLEMENTED (2026-11-11) | Priority: P2 | Commit: self | Missing: x"));
    r.write("TASK/A1-STATUS-DONE.MD", &format!("# A1 — receipt\n\nCommit {base}.\n"));
    r.git(&["add", "-A"]);
    assert_eq!(rejected(&staged(&r, None)), [("A1".to_string(), Code::DoneReceiptNotDone)]);
    // DONE: passes with the receipt naming the commit; a fix commit on it rejects.
    r.write("TASK/A1.md", &ticket("A1", &format!("Status: DONE (2026-11-11) | Priority: P2 | Commit: {base} | Receipt: A1-STATUS-DONE.MD")));
    r.git(&["add", "-A"]);
    assert!(staged(&r, None).rejected.is_empty(), "{:?}", staged(&r, None).rejected);
    assert_eq!(rejected(&staged(&r, Some(fix))), [("A1".to_string(), Code::FixFeatState)]);
    // DONE without receipt / a receipt that names another commit / an unknown commit.
    r.git(&["rm", "-q", "--cached", "TASK/A1-STATUS-DONE.MD"]);
    assert_eq!(rejected(&staged(&r, None)), [("A1".to_string(), Code::DoneNoReceipt)]);
    r.write("TASK/A1-STATUS-DONE.MD", "# A1 — receipt\n\nCommit 1234567.\n");
    r.git(&["add", "-A"]);
    assert_eq!(rejected(&staged(&r, None)), [("A1".to_string(), Code::DoneNoReceipt)]);
    r.write("TASK/A1.md", &ticket("A1", "Status: DONE (2026-11-11) | Priority: P2 | Commit: 1234567 | Receipt: A1-STATUS-DONE.MD"));
    r.git(&["add", "-A"]);
    assert_eq!(rejected(&staged(&r, None)), [("A1".to_string(), Code::BadCommit)]);
    // PARKED without Reopen.
    r.git(&["rm", "-q", "--cached", "TASK/A1-STATUS-DONE.MD"]);
    std::fs::remove_file(r.0.join("TASK/A1-STATUS-DONE.MD")).unwrap();
    r.write("TASK/A1.md", &ticket("A1", "Status: PARKED (2026-11-11) | Priority: P2 | Reason: later"));
    r.git(&["add", "-A"]);
    assert_eq!(rejected(&staged(&r, None)), [("A1".to_string(), Code::ParkedNoReason)]);
    // An unknown file and an orphan receipt in TASK/.
    r.write("TASK/A1.md", &ticket("A1", "Status: PARKED (2026-11-11) | Priority: P2 | Reason: blocked by B9 | Reopen: B9 lands"));
    r.write("TASK/notes.txt", "x\n");
    r.write("TASK/Z9-STATUS-DONE.MD", "x\n");
    r.git(&["add", "-A"]);
    let mut got = rejected(&staged(&r, None));
    got.sort();
    assert_eq!(got, [("Z9-STATUS-DONE.MD".to_string(), Code::OrphanReceipt), ("notes.txt".to_string(), Code::UnknownFile)]);
}

/// Decision g: a commit naming several ids — every named id satisfies the
/// rules, not only the first (the ids sort, so `A1` comes first).
#[test]
fn every_named_id_is_checked_not_only_the_first() {
    let imp = |id: &str| ticket(id, "Status: IMPLEMENTED (2026-11-11) | Priority: P2 | Commit: self | Missing: tester");
    let r = repo(
        "tickets-every-id",
        &[
            ("TASK/A1.md", &imp("A1")),
            ("TASK/B1.md", &ticket("B1", "Status: OPEN (2026-11-10) | Priority: P2")),
        ],
    );
    let h = out(&r, &["rev-parse", "--short=9", "HEAD"]);
    let fix = "fix(x): repair (A1, B1)\n";
    let only = |c: &CheckResult| rejected(c);
    // fix/feat rule: the second id OPEN, DONE or PARKED rejects.
    assert_eq!(only(&staged(&r, Some(fix))), [("B1".to_string(), Code::FixFeatState)]);
    for line3 in [
        format!("Status: DONE (2026-11-11) | Priority: P2 | Commit: {h} | Receipt: B1-STATUS-DONE.MD"),
        "Status: PARKED (2026-11-11) | Priority: P2 | Reason: later | Reopen: owner".to_string(),
    ] {
        r.write("TASK/B1.md", &ticket("B1", &line3));
        r.write("TASK/B1-STATUS-DONE.MD", &format!("# B1 — receipt\n\nCommit {h}.\n"));
        if line3.contains("PARKED") {
            fs::remove_file(r.0.join("TASK/B1-STATUS-DONE.MD")).unwrap();
        }
        commit(&r, "docs: B1");
        assert_eq!(only(&staged(&r, Some(fix))), [("B1".to_string(), Code::FixFeatState)], "{line3}");
    }
    // Both IMPLEMENTED: passes.
    r.write("TASK/B1.md", &imp("B1"));
    commit(&r, "docs: B1 implemented");
    assert!(staged(&r, Some(fix)).rejected.is_empty());
    // The gap checks of the message (commit-msg) cover every named id too:
    // a docs subject naming A1 and a B1 whose line 3 lacks `Missing`.
    r.write("TASK/B1.md", &ticket("B1", "Status: IMPLEMENTED (2026-11-11) | Priority: P2 | Commit: self"));
    commit(&r, "docs: B1 broken");
    assert_eq!(only(&staged(&r, Some("docs(task): note (A1, B1)\n"))), [("B1".to_string(), Code::ImplementedFields)]);
}

#[test]
fn reporting_gaps_do_not_reject() {
    let r = repo("tickets-report", &[("TASK/B1.md", &ticket("B1", "Status: OPEN (2026-11-10) | Priority: P2"))]);
    let h = out(&r, &["rev-parse", "--short=9", "HEAD"]);
    r.write("TASK/B1.md", &ticket("B1", &format!("Status: DONE (2026-11-11) | Priority: P2 | Commit: {h} | Receipt: B1-STATUS-DONE.MD")));
    r.write("TASK/B1-STATUS-UNFINISHED.MD", "# B1 — receipt\n\nRole: developer\nAgent: dev-1\n\n## Done\n");
    r.write("TASK/B1-STATUS-DONE.MD", &format!("# B1 — receipt DONE\n\nRole: tester\nAgent: Dev-1\nCommit: {h}\n"));
    r.write("TASK/P1.md", &ticket("P1", "Status: PARKED (2026-11-11) | Priority: P3 | Reason: blocked by B1 | Reopen: B1 lands"));
    r.git(&["add", "-A"]);
    let c = staged(&r, None);
    assert!(c.rejected.is_empty(), "{:?}", c.rejected);
    let mut codes: Vec<Code> = c.reported.iter().map(|v| v.code).collect();
    codes.sort();
    assert_eq!(codes, [Code::BlockedByDone, Code::SelfAccept]);
}

/// status-check flags a self-acceptance with and without the `agent-`
/// prefix, both ways (TOOL46), and not a different agent.
#[test]
fn self_accept_flag_normalises_the_agent_prefix() {
    for (dev, acc, flagged) in [("dev-1", "agent-dev-1", true), ("agent-dev-1", "Dev-1", true), ("agent-dev-1", "dev-2", false)] {
        let r = repo("tickets-self-flag", &[("TASK/B1.md", &ticket("B1", "Status: OPEN (2026-11-10) | Priority: P2"))]);
        let h = out(&r, &["rev-parse", "--short=9", "HEAD"]);
        r.write("TASK/B1.md", &ticket("B1", &format!("Status: DONE (2026-11-11) | Priority: P2 | Commit: {h} | Receipt: B1-STATUS-DONE.MD")));
        r.write("TASK/B1-STATUS-UNFINISHED.MD", &format!("# B1 — receipt\n\nRole: developer\nAgent: {dev}\n\n## Done\n"));
        r.write("TASK/B1-STATUS-DONE.MD", &format!("# B1 — receipt DONE\n\nRole: tester\nAgent: {acc}\nCommit: {h}\n"));
        r.git(&["add", "-A"]);
        let c = staged(&r, None);
        let n = c.reported.iter().filter(|v| v.code == Code::SelfAccept).count();
        assert_eq!(n, usize::from(flagged), "{dev} / {acc}: {:?}", c.reported);
    }
}

#[test]
fn baseline_lets_legacy_pass_and_only_shrinks() {
    let r = repo(
        "tickets-baseline",
        &[
            ("TASK/L1.md", &ticket("L1", "Status: OFFEN · Prioritaet: P2")),
            ("TASK/L2.md", &ticket("L2", "Status: ERLEDIGT · Prioritaet: P2")),
        ],
    );
    // Without a baseline, touching a legacy ticket rejects.
    r.write("TASK/L1.md", &(ticket("L1", "Status: OFFEN · Prioritaet: P2") + "more\n"));
    r.git(&["add", "-A"]);
    assert_eq!(rejected(&staged(&r, None)), [("L1".to_string(), Code::Form)]);
    // The written baseline records both; commit it.
    baseline_cmd(&r.0, true).unwrap();
    assert!(baseline_cmd(&r.0, true).is_err(), "--write-baseline refuses an existing baseline");
    let b = parse_baseline(&fs::read_to_string(r.0.join(BASELINE_FILE)).unwrap()).unwrap();
    assert_eq!(
        b.into_iter().collect::<Vec<_>>(),
        [("L1".to_string(), Code::Form), ("L2".to_string(), Code::Form), ("L2".to_string(), Code::DoneNoReceipt)]
    );
    commit(&r, "baseline");
    // Legacy passes; a full check is clean.
    r.write("TASK/L1.md", &(ticket("L1", "Status: OFFEN · Prioritaet: P2") + "again\n"));
    r.git(&["add", "-A"]);
    let c = staged(&r, None);
    assert!(c.rejected.is_empty() && c.legacy == 1, "{:?}", c.rejected);
    assert!(check(&r.0, &CheckOpts::default()).unwrap().rejected.is_empty());
    // A new violation on a legacy ticket rejects: a stray DONE receipt.
    r.write("TASK/L1-STATUS-DONE.MD", "# L1\n");
    r.git(&["add", "-A"]);
    assert_eq!(rejected(&staged(&r, None)), [("L1".to_string(), Code::DoneReceiptNotDone)]);
    r.git(&["rm", "-q", "--cached", "TASK/L1-STATUS-DONE.MD"]);
    fs::remove_file(r.0.join("TASK/L1-STATUS-DONE.MD")).unwrap();
    // A new ticket gets no baseline.
    r.write("TASK/N1.md", &ticket("N1", "Status: OFFEN · Prioritaet: P2"));
    r.git(&["add", "-A"]);
    assert_eq!(rejected(&staged(&r, None)), [("N1".to_string(), Code::Form)]);
    r.git(&["rm", "-q", "--cached", "TASK/N1.md"]);
    fs::remove_file(r.0.join("TASK/N1.md")).unwrap();
    // Migrating L1 leaves the baseline: stale until pruned.
    r.write("TASK/L1.md", &ticket("L1", "Status: OPEN (2026-11-11) | Priority: P2"));
    r.git(&["add", "-A"]);
    assert_eq!(rejected(&staged(&r, None)), [("L1".to_string(), Code::Baseline)]);
    assert_eq!(prune_baseline(&r.0, None).unwrap(), 1);
    r.git(&["add", "-A"]);
    assert!(staged(&r, None).rejected.is_empty(), "{:?}", staged(&r, None).rejected);
    commit(&r, "migrate L1");
    // ... and cannot come back: reverting line 3 rejects, so does re-adding the entry.
    r.write("TASK/L1.md", &ticket("L1", "Status: OFFEN · Prioritaet: P2"));
    r.git(&["add", "-A"]);
    assert_eq!(rejected(&staged(&r, None)), [("L1".to_string(), Code::Form)]);
    let mut text = fs::read_to_string(r.0.join(BASELINE_FILE)).unwrap();
    text.push_str("L1\tform\n");
    r.write(BASELINE_FILE, &text);
    r.git(&["add", "-A"]);
    let got = rejected(&staged(&r, None));
    assert!(got.contains(&("L1".to_string(), Code::Baseline)), "{got:?}");
}

#[test]
fn self_resolves_to_the_commit_of_line_3_across_a_rename() {
    let r = repo("tickets-self", &[("TASK/S1.md", &ticket("S1", "Status: OPEN (2026-11-10) | Priority: P2"))]);
    r.write("TASK/S1.md", &ticket("S1", "Status: IMPLEMENTED (2026-11-11) | Priority: P2 | Commit: self | Missing: x"));
    let set = commit(&r, "feat(x): y (S1)");
    r.write("TASK/S1.md", &(ticket("S1", "Status: IMPLEMENTED (2026-11-11) | Priority: P2 | Commit: self | Missing: x") + "later\n"));
    commit(&r, "docs: body only (S1)");
    assert_eq!(resolve_self(&r.0, "S1").unwrap(), Some(set.clone()));
    // A rename keeps the line: -L follows it.
    r.git(&["mv", "TASK/S1.md", "TASK/S2.md"]);
    commit(&r, "rename");
    assert_eq!(resolve_self(&r.0, "S2").unwrap(), Some(set));
    // An uncommitted line 3: not resolvable yet.
    r.write("TASK/S2.md", &ticket("S2", "Status: IMPLEMENTED (2026-11-11) | Priority: P2 | Commit: self | Missing: y"));
    assert_eq!(resolve_self(&r.0, "S2").unwrap(), None);
}

// ---------------------------------------------------------------------------
// accept and set-status
// ---------------------------------------------------------------------------

fn args(a: &[&str]) -> Vec<String> {
    a.iter().map(|s| s.to_string()).collect()
}

fn accept_repo(name: &str) -> (TempRepo, String) {
    let r = repo(name, &[("TASK/X1.md", &ticket("X1", "Status: OPEN (2026-11-10) | Priority: P1"))]);
    r.write("TASK/X1.md", &ticket("X1", "Status: IMPLEMENTED (2026-11-11) | Priority: P1 | Commit: self | Missing: tester"));
    r.write("TASK/X1-STATUS-UNFINISHED.MD", "# X1 — receipt\n\nRole: developer\nAgent: dev-a\n\n## Done\n\nWork.\n");
    let h = commit(&r, "feat(x): the change (X1)");
    (r, h)
}

/// The message of `accept`'s commit ends with the trailer of every commit
/// (ADR-037 decision h), and git itself parses it as a trailer.
fn assert_accept_trailer(r: &TempRepo) {
    let msg = out(r, &["log", "-1", "--format=%B"]);
    assert_eq!(msg.lines().last(), Some(ACCEPT_TRAILER), "{msg}");
    let mut child = Command::new("git")
        .args(["interpret-trailers", "--parse"])
        .current_dir(&r.0)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    {
        use std::io::Write;
        child.stdin.take().unwrap().write_all(format!("{msg}\n").as_bytes()).unwrap();
    }
    let o = child.wait_with_output().unwrap();
    assert!(o.status.success());
    assert_eq!(String::from_utf8_lossy(&o.stdout).trim(), ACCEPT_TRAILER, "{msg}");
}

#[test]
fn accept_pass_commits_receipt_and_line_3() {
    let (r, h) = accept_repo("tickets-accept-pass");
    let before = out(&r, &["rev-list", "--count", "HEAD"]);
    accept(&r.0, &args(&["X1", "--commit", &h[..9], "--result", "PASS", "--note", "all green", "--agent", "tester-b"]), D).unwrap();
    assert_eq!(out(&r, &["rev-list", "--count", "HEAD"]).parse::<u32>().unwrap(), before.parse::<u32>().unwrap() + 1);
    assert_eq!(out(&r, &["status", "--porcelain"]), "", "everything committed");
    let files = out(&r, &["show", "--name-only", "--format=", "HEAD"]);
    assert_eq!(files, "TASK/X1-STATUS-DONE.MD\nTASK/X1.md");
    assert!(out(&r, &["log", "-n1", "--format=%s"]).contains("accept X1, PASS"));
    assert_accept_trailer(&r);
    let t = fs::read_to_string(r.0.join("TASK/X1.md")).unwrap();
    assert_eq!(t.lines().nth(2).unwrap(), format!("Status: DONE ({D}) | Priority: P1 | Commit: {} | Receipt: X1-STATUS-DONE.MD", &h[..9]));
    let rc = fs::read_to_string(r.0.join("TASK/X1-STATUS-DONE.MD")).unwrap();
    assert!(rc.contains("Role: tester\nAgent: tester-b\n") && rc.contains("all green"), "{rc}");
    // The result passes the full check.
    let c = check(&r.0, &CheckOpts::default()).unwrap();
    assert!(c.rejected.is_empty() && c.reported.is_empty(), "{:?} {:?}", c.rejected, c.reported);
    // A second accept: line 3 is DONE now.
    assert!(accept(&r.0, &args(&["X1", "--commit", &h, "--result", "PASS", "--agent", "t"]), D).unwrap_err().contains("not IMPLEMENTED"));
}

/// TOOL46: after its commit, `accept` removes the scratch directories of the
/// agents the ticket's receipts record, not the acceptor's own (still
/// running) and not those of other agents.
#[test]
fn accept_cleans_the_recorded_agents_scratch() {
    let (r, h) = accept_repo("tickets-accept-scratch");
    r.write("TASK/X1-STATUS-BLOCKED.MD", "# X1 — receipt\n\nRole: developer\nAgent: agent-dev-c\n\n## Why\n");
    commit(&r, "docs(task): second receipt (X1)");
    for n in ["dev-a", "dev-c", "tester-b", "other"] {
        r.write(&format!("target-agents/agent-{n}/clone/f"), "x");
    }
    let has = |n: &str| r.0.join(format!("target-agents/agent-{n}/clone/f")).exists();
    accept(&r.0, &args(&["X1", "--commit", &h, "--result", "FAIL", "--note", "red", "--agent", "tester-b"]), D).unwrap();
    assert!(!has("dev-a") && !has("dev-c"), "the recorded developers' dirs go");
    assert!(has("tester-b"), "the acceptor's own dir stays");
    assert!(has("other"), "an agent the receipts do not record stays");
}

/// TOOL46: the PASS path cleans the same way; the acceptor's directory,
/// recorded by the DONE receipt `accept` itself writes, stays.
#[test]
fn accept_pass_cleans_the_developer_but_not_the_acceptor() {
    let (r, h) = accept_repo("tickets-accept-scratch-pass");
    for n in ["dev-a", "tester-b", "other"] {
        r.write(&format!("target-agents/agent-{n}/clone/f"), "x");
    }
    let has = |n: &str| r.0.join(format!("target-agents/agent-{n}/clone/f")).exists();
    accept(&r.0, &args(&["X1", "--commit", &h, "--result", "PASS", "--agent", "agent-tester-b"]), D).unwrap();
    assert!(fs::read_to_string(r.0.join("TASK/X1-STATUS-DONE.MD")).unwrap().contains("Agent: agent-tester-b\n"));
    assert!(!has("dev-a"), "the developer's dir goes");
    assert!(has("tester-b"), "the acceptor's own dir stays, though the DONE receipt records it");
    assert!(has("other"));
}

/// The developer is recognised with and without the `agent-` prefix, both
/// ways (`agent_key`; TOOL46, a hole in the TOOL41 check).
#[test]
fn accept_refuses_the_developer_in_either_spelling() {
    for (recorded, given) in [("dev-a", "agent-dev-a"), ("agent-dev-a", "dev-a"), ("Agent-Dev-A", "dev-a"), ("dev-a", "DEV-A")] {
        let r = repo("tickets-accept-self", &[("TASK/X1.md", &ticket("X1", "Status: OPEN (2026-11-10) | Priority: P1"))]);
        r.write("TASK/X1.md", &ticket("X1", "Status: IMPLEMENTED (2026-11-11) | Priority: P1 | Commit: self | Missing: tester"));
        r.write("TASK/X1-STATUS-UNFINISHED.MD", &format!("# X1 — receipt\n\nRole: developer\nAgent: {recorded}\n\n## Done\n"));
        let h = commit(&r, "feat(x): the change (X1)");
        let e = accept(&r.0, &args(&["X1", "--commit", &h, "--result", "PASS", "--agent", given]), D).unwrap_err();
        assert!(e.contains("is the developer recorded"), "{recorded} / {given}: {e}");
    }
    assert_eq!(agent_key(" Agent-A1b2 "), "a1b2");
    assert_eq!(agent_key("agent-"), "");
    assert_eq!(agent_key("tester-b"), "tester-b");
}

#[test]
fn accept_fail_keeps_line_3_and_appends_the_note() {
    let (r, h) = accept_repo("tickets-accept-fail");
    accept(&r.0, &args(&["X1", "--commit", &h, "--result", "FAIL", "--note", "case 3 red", "--agent", "tester-b"]), D).unwrap();
    assert_eq!(out(&r, &["show", "--name-only", "--format=", "HEAD"]), "TASK/X1-STATUS-UNFINISHED.MD");
    assert_accept_trailer(&r);
    let t = fs::read_to_string(r.0.join("TASK/X1.md")).unwrap();
    assert!(t.lines().nth(2).unwrap().starts_with("Status: IMPLEMENTED"));
    let rc = fs::read_to_string(r.0.join("TASK/X1-STATUS-UNFINISHED.MD")).unwrap();
    assert!(rc.starts_with("# X1 — receipt\n\nRole: developer\nAgent: dev-a\n"), "the developer's header stays: {rc}");
    assert!(rc.contains("## Acceptance 2026-11-11 — FAIL") && rc.contains("case 3 red"));
    // The developer stays recorded: the tester's lines are below a heading.
    let mut view = View::disk(&r.0);
    assert_eq!(Analysis::load(&mut view).unwrap().developers("X1").into_iter().collect::<Vec<_>>(), ["dev-a"]);
}

#[test]
fn accept_refuses() {
    let (r, h) = accept_repo("tickets-accept-refuse");
    let head = out(&r, &["rev-parse", "HEAD"]);
    let refuse = |a: &[&str], why: &str| {
        let e = accept(&r.0, &args(a), D).unwrap_err();
        assert!(e.contains(why), "{a:?}: {e}");
        assert_eq!(out(&r, &["rev-parse", "HEAD"]), head, "no commit");
    };
    // Self-acceptance, any case.
    refuse(&["X1", "--commit", &h, "--result", "PASS", "--agent", "DEV-A"], "is the developer");
    // Unknown commit; a commit not naming the id; bad result.
    refuse(&["X1", "--commit", "1234567", "--result", "PASS", "--agent", "t"], "no such commit");
    let base = out(&r, &["rev-parse", "HEAD~1"]);
    refuse(&["X1", "--commit", &base, "--result", "PASS", "--agent", "t"], "does not name X1");
    refuse(&["X1", "--commit", &h, "--result", "MAYBE", "--agent", "t"], "PASS or FAIL");
    // Other staged changes.
    r.write("other.txt", "x\n");
    r.git(&["add", "other.txt"]);
    refuse(&["X1", "--commit", &h, "--result", "PASS", "--agent", "t"], "other changes are staged");
    r.git(&["rm", "-q", "--cached", "other.txt"]);
    // Wrong state.
    r.write("TASK/X1.md", &ticket("X1", "Status: OPEN (2026-11-10) | Priority: P1"));
    commit(&r, "docs: reopen (X1)");
    let head2 = out(&r, &["rev-parse", "HEAD"]);
    let e = accept(&r.0, &args(&["X1", "--commit", &h, "--result", "PASS", "--agent", "t"]), D).unwrap_err();
    assert!(e.contains("not IMPLEMENTED"), "{e}");
    assert_eq!(out(&r, &["rev-parse", "HEAD"]), head2);
    // No developer recorded.
    r.write("TASK/X1.md", &ticket("X1", "Status: IMPLEMENTED (2026-11-11) | Priority: P1 | Commit: self | Missing: t"));
    r.write("TASK/X1-STATUS-UNFINISHED.MD", "# X1 — receipt\n\nNo role.\n");
    let h3 = commit(&r, "fix(x): again (X1)");
    let e = accept(&r.0, &args(&["X1", "--commit", &h3, "--result", "PASS", "--agent", "t"]), D).unwrap_err();
    assert!(e.contains("no developer recorded"), "{e}");
}

#[test]
fn set_status_enforces_the_fields_and_writes_line_3() {
    let r = repo(
        "tickets-set-status",
        &[
            ("TASK/Y1.md", &ticket("Y1", "Status: OPEN (2026-11-10) | Priority: P2")),
            ("TASK/Y2.md", &ticket("Y2", "Status: GEPARKT 2026-10-31 (reason) · Prioritaet: P3")),
            ("TASK/Y3.md", &ticket("Y3", "Status: OPEN (2026-11-10) | Priority: P2")),
        ],
    );
    let refuse = |a: &[&str], why: &str| {
        let before = fs::read_to_string(r.0.join(format!("TASK/{}.md", a[0]))).unwrap();
        let e = set_status(&r.0, &args(a), D).unwrap_err();
        assert!(e.contains(why), "{a:?}: {e}");
        assert_eq!(fs::read_to_string(r.0.join(format!("TASK/{}.md", a[0]))).unwrap(), before);
    };
    refuse(&["Y1", "PARKED", "--reason", "later"], "--reopen");
    refuse(&["Y1", "PARKED", "--reopen", "x"], "--reason");
    refuse(&["Y1", "ACCEPTED"], "--ref");
    refuse(&["Y1", "OBSOLETE"], "--reason");
    refuse(&["Y1", "OBSOLETE", "--reason", "duplicate", "--target", "NOPE"], "no ticket");
    refuse(&["Y1", "DONE"], "not set by hand");
    refuse(&["Y1", "IMPLEMENTED"], "not set by hand");
    refuse(&["Y1", "OPEN", "--reason", "x"], "PARKED ticket");
    refuse(&["Y1", "PARKED", "--reason", "a | b", "--reopen", "x"], "without `|`");
    refuse(&["Y1", "PARKED", "--reason", "Prüfung", "--reopen", "x"], "ASCII");
    refuse(&["Y1", "ACCEPTED", "--ref", "ADR-001", "--reason", "x"], "does not apply");

    set_status(&r.0, &args(&["Y1", "PARKED", "--reason", "blocked by Y3", "--reopen", "Y3 lands"]), D).unwrap();
    let t = fs::read_to_string(r.0.join("TASK/Y1.md")).unwrap();
    assert_eq!(t.lines().nth(2).unwrap(), "Status: PARKED (2026-11-11) | Priority: P2 | Reason: blocked by Y3 | Reopen: Y3 lands");
    assert!(t.contains("## Status history\n\n- 2026-11-11: set to PARKED"));
    set_status(&r.0, &args(&["Y3", "OBSOLETE", "--reason", "duplicate", "--target", "Y1"]), D).unwrap();
    assert!(fs::read_to_string(r.0.join("TASK/Y3.md")).unwrap().contains("| Reason: duplicate | Target: Y1\n"));
    // Legacy PARKED -> OPEN keeps its priority; the reason goes to the history.
    set_status(&r.0, &args(&["Y2", "OPEN", "--reason", "owner 2026-11-11: now"]), D).unwrap();
    let t = fs::read_to_string(r.0.join("TASK/Y2.md")).unwrap();
    assert_eq!(t.lines().nth(2).unwrap(), "Status: OPEN (2026-11-11) | Priority: P3");
    assert!(t.contains("reopened (was \"Status: GEPARKT") && t.contains("owner 2026-11-11: now"));
    // Nothing was committed: the lead commits.
    assert!(!out(&r, &["status", "--porcelain"]).is_empty());
}

#[test]
fn set_status_prunes_the_migrated_ticket_from_the_baseline() {
    let r = repo("tickets-set-status-baseline", &[("TASK/Y2.md", &ticket("Y2", "Status: GEPARKT · Prioritaet: P3"))]);
    baseline_cmd(&r.0, true).unwrap();
    commit(&r, "baseline");
    set_status(&r.0, &args(&["Y2", "PARKED", "--reason", "r", "--reopen", "o"]), D).unwrap();
    assert!(parse_baseline(&fs::read_to_string(r.0.join(BASELINE_FILE)).unwrap()).unwrap().is_empty());
    r.git(&["add", "-A"]);
    assert!(staged(&r, Some("docs(task): park Y2 (Y2)\n")).rejected.is_empty());
    assert!(staged(&r, None).rejected.is_empty());
}

// ---------------------------------------------------------------------------
// The hooks (switch off = TOOL40's behaviour; switch on = the checks run)
// ---------------------------------------------------------------------------

/// The pre-commit hook as TOOL40 left it (`ced5e68f6`), from this repository.
fn tool40_pre_commit() -> String {
    let o = Command::new("git")
        .args(["show", "ced5e68f6:tools/git-hooks/pre-commit"])
        .current_dir(crate_root())
        .output()
        .unwrap();
    assert!(o.status.success(), "commit ced5e68f6 (TOOL40's hook) must be in this repository's history");
    String::from_utf8(o.stdout).unwrap()
}

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap().to_path_buf()
}

struct Hooks {
    dir: PathBuf,
}

impl Hooks {
    fn new(r: &TempRepo) -> Hooks {
        let dir = PathBuf::from(format!("{}-hooks", r.0.display()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("bin")).unwrap();
        fs::write(dir.join("bin/cargo"), "#!/bin/sh\necho \"cargo $*\" >> \"$FAKE_LOG\"\necho \"fake cargo $*\"\nexit ${FAKE_RC:-0}\n").unwrap();
        fs::write(dir.join("old-pre-commit"), tool40_pre_commit()).unwrap();
        let hooks = crate_root().join("tools/git-hooks");
        fs::copy(hooks.join("pre-commit"), dir.join("pre-commit")).unwrap();
        fs::copy(hooks.join("commit-msg"), dir.join("commit-msg")).unwrap();
        Command::new("chmod").args(["+x"]).arg(dir.join("bin/cargo")).status().unwrap();
        Hooks { dir }
    }

    /// Runs hook `name` in `r` with fake cargo exiting `rc`: (exit code,
    /// stdout, stderr, cargo calls).
    fn run(&self, r: &TempRepo, name: &str, args: &[&str], rc: i32) -> (i32, String, String, String) {
        let log = self.dir.join("cargo.log");
        let _ = fs::remove_file(&log);
        let path = format!("{}:{}", self.dir.join("bin").display(), std::env::var("PATH").unwrap());
        let mut c = Command::new("sh");
        for v in ["GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE"] {
            c.env_remove(v);
        }
        let o = c
            .arg(self.dir.join(name))
            .args(args)
            .current_dir(&r.0)
            .env("PATH", path)
            .env("FAKE_LOG", &log)
            .env("FAKE_RC", rc.to_string())
            .stdin(Stdio::null())
            .output()
            .unwrap();
        (
            o.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&o.stdout).into_owned(),
            String::from_utf8_lossy(&o.stderr).into_owned(),
            fs::read_to_string(&log).unwrap_or_default(),
        )
    }
}

impl Drop for Hooks {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn accept_updates_only_its_own_readme_row() {
    let (r, h) = accept_repo("tickets-accept-readme");
    let readme = "# R\n\n| ID | Task | Status |\n|---|---|---|\n| [X1](X1.md) | t | IMPLEMENTED |\n| [Z1](Z1.md) | t | stale |\n";
    r.write("TASK/README.md", readme);
    r.write("TASK/Z1.md", &ticket("Z1", "Status: OPEN (2026-11-10) | Priority: P2"));
    commit(&r, "docs: readme");
    accept(&r.0, &args(&["X1", "--commit", &h, "--result", "PASS", "--agent", "tester-b"]), D).unwrap();
    assert_eq!(out(&r, &["status", "--porcelain"]), "", "everything committed");
    assert_eq!(out(&r, &["show", "--name-only", "--format=", "HEAD"]), "TASK/README.md\nTASK/X1-STATUS-DONE.MD\nTASK/X1.md");
    let now = fs::read_to_string(r.0.join("TASK/README.md")).unwrap();
    assert!(now.contains("| [X1](X1.md) | t | DONE |") && now.contains("| [Z1](Z1.md) | t | stale |"), "{now}");
}

#[test]
fn hooks_with_the_switch_off_behave_as_after_tool40() {
    let r = repo("tickets-hooks-off", &[("TASK/H1.md", &ticket("H1", "Status: OPEN (2026-11-10) | Priority: P2")), ("x.txt", "x\n")]);
    r.write("docs/analysis/deps/mapping.tsv", "# h\n");
    commit(&r, "mapping");
    let h = Hooks::new(&r);
    // The cases: nothing the hook looks at; a ticket; the mapping; both.
    let stage: [&[(&str, &str)]; 4] = [
        &[("x.txt", "y\n")],
        &[("TASK/H1.md", "# H1 — t\n\nStatus: OFFEN\n")],
        &[("docs/analysis/deps/mapping.tsv", "# h2\n")],
        &[("TASK/H1.md", "# H1 — t\n\nStatus: OFFEN 2\n"), ("docs/analysis/deps/mapping.tsv", "# h3\n")],
    ];
    let mut calls = 0;
    for files in stage {
        for (p, t) in files {
            r.write(p, t);
        }
        r.git(&["add", "-A"]);
        for switch in [None, Some("false"), Some("no")] {
            match switch {
                None => {
                    let _ = Command::new("git").args(["config", "--unset", SWITCH]).current_dir(&r.0).status();
                }
                Some(v) => r.git(&["config", SWITCH, v]),
            }
            for rc in [0, 1] {
                let old = h.run(&r, "old-pre-commit", &[], rc);
                let new = h.run(&r, "pre-commit", &[], rc);
                assert_eq!(new, old, "files {files:?}, switch {switch:?}, cargo rc {rc}");
                assert!(!new.3.contains("status-check"));
                calls += new.3.lines().count();
                // commit-msg: exit 0 at once, no output, no cargo.
                fs::write(r.0.join("MSG"), "fix(x): y (H1)\n").unwrap();
                assert_eq!(h.run(&r, "commit-msg", &["MSG"], rc), (0, String::new(), String::new(), String::new()));
            }
        }
        commit(&r, "step");
    }
    assert!(calls > 0, "the cases reached cargo");
}

#[test]
fn hooks_with_the_switch_on_run_the_ticket_checks() {
    let r = repo("tickets-hooks-on", &[("TASK/H1.md", &ticket("H1", "Status: OPEN (2026-11-10) | Priority: P2")), ("x.txt", "x\n")]);
    let h = Hooks::new(&r);
    r.git(&["config", SWITCH, "true"]);
    // No TASK/ path staged: no ticket check (and, as before, no doc-check).
    r.write("x.txt", "y\n");
    r.git(&["add", "-A"]);
    assert_eq!(h.run(&r, "pre-commit", &[], 0).3, "");
    // A staged ticket: doc-check and the gap check; each failure rejects.
    r.write("TASK/H1.md", "# H1 — t\n\nStatus: OFFEN\n");
    r.git(&["add", "-A"]);
    let (code, _, _, log) = h.run(&r, "pre-commit", &[], 0);
    assert_eq!((code, log.as_str()), (0, "cargo -q xtask doc-check --staged\ncargo -q xtask status-check --staged --new-only\n"));
    let (code, _, err, _) = h.run(&r, "pre-commit", &[], 1);
    assert_eq!(code, 1);
    assert!(err.contains("the ticket gap check failed"), "{err}");
    // commit-msg passes the absolute message path and propagates the result.
    fs::write(r.0.join("MSG"), "fix(x): y (H1)\n").unwrap();
    let (code, _, _, log) = h.run(&r, "commit-msg", &["MSG"], 0);
    assert_eq!((code, log), (0, format!("cargo -q xtask status-check --staged --new-only --message-file {}/MSG\n", r.0.display())));
    let (code, _, err, _) = h.run(&r, "commit-msg", &["MSG"], 1);
    assert_eq!(code, 1);
    assert!(err.contains("commit rejected"), "{err}");
}
