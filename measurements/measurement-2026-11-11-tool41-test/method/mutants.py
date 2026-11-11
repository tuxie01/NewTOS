import sys, subprocess, shutil, os
S, C = sys.argv[1], sys.argv[2]
T = C + "/tools/xtask/src/tickets.rs"
orig = open(S + "/tickets.rs.orig").read()
def sub(src, old, new, count=1):
    assert src.count(old) >= 1, "pattern not found: " + old[:60]
    return src.replace(old, new, count)
muts = {}
# (a) parser accepts a missing required field: the push of the missing-field error is dropped
muts["a-parser-missing-field-accepted"] = ("T", sub(orig,
"""            _ => errors.push(FieldError {
                field: key.to_string(),
                expected: format!("`{key}: ...` (required for {}, in the order {})", st.name(), st.required().join(", ")),
                missing: true,
            }),""", """            _ => {}"""))
# (b) accept does not refuse self-acceptance
muts["b-accept-allows-self"] = ("T", sub(orig, "if devs.contains(&agent.to_lowercase()) {", "if false && devs.contains(&agent.to_lowercase()) {"))
# (c1) baseline: a listed violation is not 'fresh' even for tickets not in baseline -> mutate: the 'only shrinks' check removed
muts["c1-baseline-may-grow"] = ("T", sub(orig, "    if baseline_touched {\n        if let Some(head)", "    if false && baseline_touched {\n        if let Some(head)"))
# (c2) a fixed ticket (stale entry) does not reject, i.e. leftover entries let it come back silently
muts["c2-stale-entry-not-rejected"] = ("T", sub(orig, "let stale = baseline.iter().filter(|e| checked.contains(&e.0) && !have.contains(*e)).cloned().collect();", "let stale: Vec<(String, Code)> = Vec::new();"))
# (c3) any violation of a key present in the baseline passes (code ignored)
muts["c3-baseline-matches-key-only"] = ("T", sub(orig, ".filter(|v| v.code.rejecting() && !baseline.contains(&(v.key.clone(), v.code)))", ".filter(|v| v.code.rejecting() && !baseline.iter().any(|e| e.0 == v.key))"))
# (d1) message_violations: only the first id
muts["d1-fixfeat-first-id-only"] = ("T", sub(orig, "    for id in a.named_in(s) {\n        let t = &a.tickets[&id];", "    for id in a.named_in(s).into_iter().take(1) {\n        let t = &a.tickets[&id];"))
# (d2) commit-msg gap checks: only first id
muts["d2-msg-gapcheck-first-id-only"] = ("T", sub(orig, "(a.named_in(subject(msg)), false)", "(a.named_in(subject(msg)).into_iter().take(1).collect(), false)"))
# (d3) both
muts["d3-both-first-id-only"] = ("T", sub(sub(orig, "    for id in a.named_in(s) {\n        let t = &a.tickets[&id];", "    for id in a.named_in(s).into_iter().take(1) {\n        let t = &a.tickets[&id];"), "(a.named_in(subject(msg)), false)", "(a.named_in(subject(msg)).into_iter().take(1).collect(), false)"))
# (e) hooks: switch-off path runs the checks
cm = open(S + "/commit-msg.orig").read()
pc = open(S + "/pre-commit.orig").read()
muts["e1-commit-msg-ignores-switch"] = ("CM", sub(cm, '[ "$(git config --bool --get newtos.ticketChecks 2>/dev/null)" = true ] || exit 0', ': # switch ignored'))
muts["e2-pre-commit-ignores-switch"] = ("PC", sub(pc, 'if [ "$(git config --bool --get newtos.ticketChecks 2>/dev/null)" = true ]; then', 'if true; then'))
names = sys.argv[3:] or list(muts)
def restore():
    open(T, "w").write(orig); open(C + "/tools/git-hooks/commit-msg", "w").write(cm); open(C + "/tools/git-hooks/pre-commit", "w").write(pc)
for n in names:
    kind, src = muts[n]
    restore()
    if kind == "T": open(T, "w").write(src)
    elif kind == "CM": open(C + "/tools/git-hooks/commit-msg", "w").write(src)
    else: open(C + "/tools/git-hooks/pre-commit", "w").write(src)
    p = subprocess.run(["cargo", "test", "-p", "xtask", "--no-fail-fast", "tickets::"], cwd=C, capture_output=True, text=True)
    out = p.stdout + p.stderr
    failed = sorted(set(l.split()[1] for l in out.splitlines() if l.startswith("test ") and l.endswith("FAILED")))
    res = [l for l in out.splitlines() if l.startswith("test result")]
    print("MUTANT", n, "rc=%d" % p.returncode, "KILLED by" if failed else ("BUILD-ERR" if p.returncode and not res else "SURVIVED"), failed, res[:1], flush=True)
    if p.returncode and not res: print(out[-800:])
restore()
