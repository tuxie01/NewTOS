Scripts reused unchanged from measurements/measurement-2026-11-11-tool41-test/method/: mutants.py, acceptance.sh, switchoff.sh (not copied).
Setup (scratch clone c1 of branch 47050966f, core.hooksPath=tools/git-hooks, references/* and sdk/apps/{coremark,highwire,hypview,qed,teradesk} symlinked from the main tree, the sdk ones in .git/info/exclude):
  python3 -I <first-run>/method/mutants.py $S $S/c1      ($S has tickets.rs.orig commit-msg.orig pre-commit.orig = the branch's)
  bash <first-run>/method/acceptance.sh $S/c1 $S/out.tmp
  bash <first-run>/method/switchoff.sh $S $S/fakebin     (pre-commit.old = git show 9bc27d82b:tools/git-hooks/pre-commit; branch alias worktree-agent-a823a57e109be35e1 created in the clone)
  cargo xtask test; cargo xtask report [--summary | --readme --check]; cargo xtask status-check; cargo clippy -p xtask [--tests]; cargo xtask doc-check
