#!/usr/bin/env bash
# Pipe-tests for tools/hooks/no-verify-guard.sh. Run from the repo root (or a
# worktree root). Prints one line per case; exits 1 if any line is FAIL.
#
# The "main repo" cases run with CLAUDE_PROJECT_DIR = $PWD, so the main repo is
# the git common dir of $PWD (a worktree counts as its main repo). The
# "allowed" cases target a scratch clone, made with `git clone --shared` in a
# temp dir from mktemp under $NVG_TEST_TMP (else $TMPDIR) and removed at exit;
# the fast-forward cases (TOOL42 point 4) use that clone as the main repo,
# with HEAD set to main or to another branch.
G=$PWD/tools/hooks/no-verify-guard.sh
ROOT=$PWD
fail=0

# t <command> <want exit> <description>: no cwd in the hook input.
t() {
    jq -nc --arg c "$1" '{tool_name:"Bash",tool_input:{command:$c}}' | CLAUDE_PROJECT_DIR=$ROOT "$G" >/dev/null 2>&1
    rc=$?
    if [ "$rc" = "$2" ]; then r=ok; else r=FAIL; fail=1; fi
    echo "$r exit $rc (want $2): $3"
}
# tc <project dir> <cwd> <command> <want exit> <description>
tc() {
    jq -nc --arg c "$3" --arg d "$2" '{tool_name:"Bash",cwd:$d,tool_input:{command:$c}}' \
        | CLAUDE_PROJECT_DIR=$1 "$G" >/dev/null 2>&1
    rc=$?
    if [ "$rc" = "$4" ]; then r=ok; else r=FAIL; fail=1; fi
    echo "$r exit $rc (want $4): $5"
}

t 'git config core.hooksPath tools/git-hooks' 0 "allowed relative"
t "git config core.hooksPath $PWD/tools/git-hooks" 0 "allowed absolute"
t 'git config --local core.hooksPath "tools/git-hooks/"' 0 "allowed quoted, trailing slash"
t 'git config core.hooksPath' 0 "read"
t 'git config --get core.hooksPath' 0 "get"
t 'git config core.hooksPath /dev/null' 2 "/dev/null"
t 'git config core.hooksPath ""' 2 "empty"
t 'git config core.hooksPath other/dir' 2 "other dir"
t 'git config --unset core.hooksPath' 2 "unset"
t 'git config --unset-all core.hooksPath' 2 "unset-all"
t 'git config --global core.HooksPath /var/x' 2 "global, other dir, mixed case"
t 'git config core.hooksPath tools/git-hooks && git config core.hooksPath x' 2 "chained: second bad"
t 'git log -n 1' 0 "unrelated git"
t 'git commit --no-verify -m x' 2 "no-verify"
t 'git commit -n -m x' 2 "commit -n"
t 'git commit -q -F m.txt && git log --oneline -n 1' 0 "chain with log -n"
t 'git -c core.hooksPath=/dev/null commit -m x' 2 "inline override"

# ── TOOL42: hook-bypassing plumbing ─────────────────────────────────────
base=${NVG_TEST_TMP:-${TMPDIR:-}}
if [ -z "$base" ] || [ ! -d "$base" ]; then
    echo "FAIL: no temp base (set NVG_TEST_TMP or TMPDIR to an existing dir)"
    exit 1
fi
tmp=$(mktemp -d "${base%/}/nvg-test.XXXXXX") || { echo "FAIL: mktemp"; exit 1; }
trap 'rm -rf "$tmp"' EXIT
C=$tmp/clone
if ! git clone -q --shared --no-checkout "$ROOT" "$C" 2>/dev/null; then
    echo "FAIL: scratch clone of $ROOT"
    exit 1
fi
MAIN_TOP=$(cd "$(git rev-parse --path-format=absolute --git-common-dir)/.." && pwd -P)
SUB=$ROOT/tools
T=4b825dc642cb6eb9a060e54bf8d69288fbee4904   # the empty tree
H=0123456789abcdef0123456789abcdef01234567

# Each plumbing form: blocked in the main repo (cwd, -C, cd &&, --git-dir,
# GIT_DIR=), allowed in the scratch clone (cwd, -C, cd &&).
forms=(
    "git commit-tree $T -m x"
    "git update-ref refs/heads/main $H"
    "git update-ref HEAD $H"
    "git update-ref main $H"
    "git update-ref -d refs/heads/old"
    "git update-ref -m msg --no-deref HEAD $H"
    "git hash-object -w -t commit c.txt"
    "git hash-object -t commit -w --stdin"
    "git fast-import"
    "git replace $H $T"
    "git symbolic-ref HEAD refs/heads/other"
    "git symbolic-ref --delete HEAD"
    "git filter-branch --tree-filter true HEAD"
    "git filter-repo --path x"
    "git-filter-repo --path x"
)
for f in "${forms[@]}"; do
    sub=${f#git }
    sub=${sub#git-}   # the dashed form: `git -C <dir> filter-repo`
    tc "$ROOT" "$ROOT" "$f" 2 "main repo, cwd: $f"
    tc "$ROOT" "$C" "git -C $ROOT $sub" 2 "main repo, -C: $f"
    tc "$ROOT" "$C" "cd $ROOT && $f" 2 "main repo, cd &&: $f"
    tc "$ROOT" "$C" "$f" 0 "clone, cwd: $f"
    tc "$ROOT" "$ROOT" "git -C $C $sub" 0 "clone, -C: $f"
    tc "$ROOT" "$ROOT" "cd $C && $f" 0 "clone, cd &&: $f"
done
tc "$ROOT" "$C" "git --git-dir=$MAIN_TOP/.git commit-tree $T -m x" 2 "main repo, --git-dir="
tc "$ROOT" "$C" "GIT_DIR=$MAIN_TOP/.git git commit-tree $T -m x" 2 "main repo, GIT_DIR="
tc "$ROOT" "$C" "git -C $MAIN_TOP update-ref refs/heads/main $H" 2 "main tree from a clone cwd"
tc "$ROOT" "$C" "git -C $SUB commit-tree $T -m x" 2 "main repo subdirectory"
tc "$ROOT" "$C" "cd $ROOT; git commit-tree $T -m x" 2 "cd ; (not &&)"
tc "$ROOT" "$C" "cd .git && git commit-tree $T -m x" 0 "relative cd inside the clone"
tc "$ROOT" "$C" "cd tools && git commit-tree $T -m x" 2 "relative cd to a missing dir (undecidable) is blocked"
tc "$ROOT" "$ROOT" "cd tools && git commit-tree $T -m x" 2 "relative cd inside the main repo"
tc "$ROOT" "$ROOT" "git --git-dir=$C/.git commit-tree $T -m x" 0 "clone, --git-dir="
tc "$ROOT" "$ROOT" "GIT_DIR=$C/.git git update-ref refs/heads/main $H" 0 "clone, GIT_DIR="
tc "$ROOT" "$ROOT" "git mktree < t.txt | xargs -I{} git commit-tree {} -m x" 2 "mktree | commit-tree chain (via xargs)"
tc "$ROOT" "$ROOT" "t=\$(git mktree < t.txt) && c=\$(git commit-tree \$t -m x) && git update-ref refs/heads/main \$c" 2 "mktree + commit-tree in \$()"
tc "$ROOT" "$ROOT" "echo \"\$(git commit-tree $T -m x)\"" 2 "commit-tree inside a quoted \$()"
tc "$ROOT" "$ROOT" "printf 'update refs/heads/main $H\\n' | git update-ref --stdin" 2 "update-ref --stdin"
tc "$ROOT" "$ROOT" "( cd $C && git commit-tree $T -m x )" 0 "clone in a subshell"
tc "$ROOT" "$ROOT" "cd $C && t=\$(git write-tree) && c=\$(git commit-tree \$t -p HEAD -m x) && git update-ref refs/heads/main \$c" 0 "clone: write-tree + commit-tree + update-ref (TOOL40 setup)"
tc "$ROOT" "$C" "cd $ROOT && t=\$(git write-tree) && c=\$(git commit-tree \$t -p HEAD -m x) && git update-ref refs/heads/main \$c" 2 "main repo: the same chain"
tc "$ROOT" "$ROOT" "( cd $C && true ); git commit-tree $T -m x" 2 "subshell cd does not leak"
tc "$ROOT" "$ROOT" "cd \"\$X\" && git commit-tree $T -m x" 2 "undecidable target (variable) is blocked"
tc "$ROOT" "$ROOT" "git clone -q $ROOT $tmp/new && git -C $tmp/new commit-tree $T -m x" 2 "target made in the same command (undecidable) is blocked"
tc "$ROOT" "$ROOT" "bash -c 'true' && git -C \"$C\" commit-tree $T -m x" 0 "quoted clone path"
r=$(jq -nc --arg c "cd \"\$TMPDIR/clone\" && git commit-tree $T -m x" --arg d "$ROOT" '{tool_name:"Bash",cwd:$d,tool_input:{command:$c}}' \
    | TMPDIR=$tmp/ CLAUDE_PROJECT_DIR=$ROOT "$G" >/dev/null 2>&1; echo $?)
if [ "$r" = 0 ]; then echo "ok exit 0 (want 0): clone named through \$TMPDIR"; else echo "FAIL exit $r (want 0): clone named through \$TMPDIR"; fail=1; fi
# Tester PARTIAL F1: $CLAUDE_PROJECT_DIR words; canonical path comparison.
# Run with the worktree and with the clone as the project: only a repo that
# is not a worktree (rev-parse prints a relative common dir) showed the bug.
f1() {   # f1 <project> <other cwd> <existing subdir>
    local P=$1 O=$2 D=$3
    tc "$P" "$O" "git -C \$CLAUDE_PROJECT_DIR commit-tree $T -m x" 2 "F1 ($4): -C \$CLAUDE_PROJECT_DIR"
    tc "$P" "$O" "git -C \"\$CLAUDE_PROJECT_DIR\" commit-tree $T -m x" 2 "F1 ($4): -C \"\$CLAUDE_PROJECT_DIR\""
    tc "$P" "$O" "git -C \${CLAUDE_PROJECT_DIR} update-ref refs/heads/main $H" 2 "F1 ($4): -C \${CLAUDE_PROJECT_DIR} update-ref"
    tc "$P" "$O" "git -C \"\$CLAUDE_PROJECT_DIR\" update-ref refs/heads/main $H" 2 "F1 ($4): -C \"\$CLAUDE_PROJECT_DIR\" update-ref"
    tc "$P" "$O" "git -C \$CLAUDE_PROJECT_DIR/$D commit-tree $T -m x" 2 "F1 ($4): -C \$CLAUDE_PROJECT_DIR/$D"
    tc "$P" "$O" "git -C \${CLAUDE_PROJECT_DIR}/$D commit-tree $T -m x" 2 "F1 ($4): -C \${CLAUDE_PROJECT_DIR}/$D"
    tc "$P" "$O" "cd \$CLAUDE_PROJECT_DIR && git commit-tree $T -m x" 2 "F1 ($4): cd \$CLAUDE_PROJECT_DIR &&"
    tc "/$P/" "$O" "git -C $P commit-tree $T -m x" 2 "F1 ($4): project dir with // and trailing /"
    tc "$P" "$O" "git -C /$P// commit-tree $T -m x" 2 "F1 ($4): target with // and trailing //"
    tc "$P" "$O" "git -C \$CLAUDE_PROJECT_DIRX commit-tree $T -m x" 2 "F1 ($4): \$CLAUDE_PROJECT_DIRX is another variable (undecidable)"
}
f1 "$ROOT" "$C" tools worktree
f1 "$C" "$ROOT" .git "clone as project"
# Tester PARTIAL F2: quoted command word.
tc "$ROOT" "$ROOT" "\"git\" commit-tree $T -m x" 2 "F2: \"git\" commit-tree"
tc "$ROOT" "$ROOT" "'git' commit-tree $T -m x" 2 "F2: 'git' commit-tree"
tc "$ROOT" "$ROOT" "\"git\" -C $C commit-tree $T -m x" 0 "F2: \"git\" -C clone commit-tree"
# Allowed in the main repo.
tc "$ROOT" "$ROOT" "git hash-object -w part.txt" 0 "main repo: blob hash-object -w"
tc "$ROOT" "$ROOT" "git hash-object -w --stdin < part.txt" 0 "main repo: blob hash-object -w --stdin"
tc "$ROOT" "$ROOT" "b=\$(git hash-object -w part.txt) && git update-index --cacheinfo 100644,\$b,docs/STATUS.md" 0 "main repo: hash-object + update-index --cacheinfo"
tc "$ROOT" "$ROOT" "git update-index --cacheinfo 100644 $H docs/STATUS.md" 0 "main repo: update-index --cacheinfo (3 args)"
tc "$ROOT" "$ROOT" "git hash-object part.txt" 0 "main repo: hash-object without -w"
tc "$ROOT" "$ROOT" "git update-ref refs/tags/x $H" 0 "main repo: update-ref on a tag"
tc "$ROOT" "$ROOT" "git symbolic-ref --short HEAD" 0 "main repo: symbolic-ref read"
tc "$ROOT" "$ROOT" "git symbolic-ref -q HEAD" 0 "main repo: symbolic-ref -q read"
tc "$ROOT" "$ROOT" "git replace -l" 0 "main repo: replace -l"
tc "$ROOT" "$ROOT" "git cat-file -p HEAD && git ls-tree HEAD" 0 "main repo: read-only plumbing"
tc "$ROOT" "$ROOT" "grep -n commit-tree tools/hooks/no-verify-guard.sh" 0 "main repo: word in a grep argument"
tc "$ROOT" "$ROOT" "git log --grep commit-tree" 0 "main repo: word as a git log argument"
tc "$ROOT" "$ROOT" "echo 'git commit-tree x' > note.txt" 0 "main repo: word in a quoted string"
tc "$ROOT" "$ROOT" "git commit -q -F msg.txt" 0 "main repo: plain git commit"
tc "$ROOT" "$ROOT" "cat > note.txt <<'EOF'
git commit-tree $T -m x
EOF
git status" 0 "main repo: word in a here-document body"
tc "$ROOT" "$ROOT" "ls # git commit-tree $T" 0 "main repo: word in a comment"
tc "$ROOT" "$ROOT" "ls # note
git update-ref refs/heads/main $H" 2 "main repo: command after a comment line"
tc "$ROOT" "$ROOT" "git commit-tree $T -m x 2>&1 | head -1" 2 "main repo: with 2>&1 | head"
tc "$ROOT" "$C" "env GIT_DIR=$MAIN_TOP/.git git update-ref HEAD $H" 2 "main repo: env GIT_DIR="
tc "$ROOT" "$C" "export GIT_DIR=$MAIN_TOP/.git; git update-ref HEAD $H" 2 "main repo: export GIT_DIR"

# ── TOOL42 point 4: no fast-forward into main (clone as the main repo) ──
git -C "$C" symbolic-ref HEAD refs/heads/main
tc "$C" "$C" "git merge --ff-only feature" 2 "main checked out: merge --ff-only"
tc "$C" "$C" "git merge feature" 2 "main checked out: merge without --squash"
tc "$C" "$C" "git merge --no-ff -m x feature" 2 "main checked out: merge --no-ff"
tc "$C" "$ROOT" "git -C $C merge feature" 2 "main checked out, -C: merge"
# TOOL50 point 4: land is the only merge path into main.
tc "$C" "$C" "git merge --squash feature" 2 "TOOL50: main checked out: merge --squash (use cargo xtask land)"
tc "$C" "$ROOT" "git -C $C merge --squash feature" 2 "TOOL50: main checked out, -C: merge --squash"
tc "$C" "$ROOT" "cd $C && git merge --squash -X theirs feature && git commit -F m.txt" 2 "TOOL50: main checked out, cd &&: merge --squash -X"
tc "$C" "$C" "git merge --squash origin/main" 0 "TOOL50: main checked out: merge --squash of a remote-tracking ref"
tc "$C" "$C" "git merge --squash" 0 "TOOL50: main checked out: bare merge --squash (upstream)"
tc "$C" "$C" "cargo xtask land feature -F m.txt" 0 "TOOL50: cargo xtask land"
tc "$C" "$C" "cargo xtask land merge-fix -F merge.txt" 0 "TOOL50: cargo xtask land, branch named merge-*"
tc "$ROOT" "$ROOT" "git -C $C merge --squash feature" 0 "TOOL50: merge --squash in a scratch clone (not the main repo)"
tc "$C" "$C" "git merge --abort" 0 "main checked out: merge --abort"
tc "$C" "$C" "git merge --continue" 0 "main checked out: merge --continue"
tc "$C" "$C" "git pull" 0 "main checked out: git pull"
tc "$C" "$C" "git pull --ff-only origin main" 0 "main checked out: git pull origin"
tc "$C" "$C" "git pull . feature" 2 "main checked out: git pull from the repo itself"
tc "$C" "$C" "git pull ../wt feature" 2 "main checked out: git pull from a local path"
tc "$ROOT" "$ROOT" "git -C $C merge feature" 0 "merge in a foreign repo (not the main repo)"
tc "$C" "$C" "git merge origin/main" 0 "main checked out: merge of a remote-tracking ref (= pull)"
tc "$C" "$C" "git merge --ff-only origin/main" 0 "main checked out: merge --ff-only of a remote-tracking ref"
tc "$C" "$C" "git merge refs/remotes/origin/main" 0 "main checked out: merge refs/remotes/*"
tc "$C" "$C" "git merge origin/main feature" 2 "main checked out: remote-tracking ref plus a local branch"
tc "$C" "$C" "git merge nosuchremote/x" 2 "main checked out: <name>/<branch> without such a remote"
tc "$C" "$ROOT" "git -C \$CLAUDE_PROJECT_DIR merge feature" 2 "F1: -C \$CLAUDE_PROJECT_DIR merge"
# TOOL50: repo paths compare case folded (APFS is case-insensitive).
UPC=$(printf '%s' "$C" | tr 'abcdefghijklmnopqrstuvwxyz' 'ABCDEFGHIJKLMNOPQRSTUVWXYZ')
UPR=$(printf '%s' "$ROOT" | tr 'abcdefghijklmnopqrstuvwxyz' 'ABCDEFGHIJKLMNOPQRSTUVWXYZ')
if [ -d "$UPC" ]; then
    tc "$C" "$ROOT" "git -C $UPC merge --squash feature" 2 "TOOL50: main checked out, path in other letter case: merge --squash"
    tc "$C" "$ROOT" "cd $UPC && git merge --ff-only feature" 2 "TOOL50: main checked out, cd in other letter case: merge --ff-only"
    tc "$ROOT" "$C" "git -C $UPR commit-tree $T -m x" 2 "TOOL50: main repo in other letter case: commit-tree"
    tc "$ROOT" "$ROOT" "git -C $UPC commit-tree $T -m x" 0 "TOOL50: clone in other letter case: commit-tree allowed"
else
    echo "ok (skipped: case-sensitive volume): TOOL50 letter-case cases"
fi
git -C "$C" symbolic-ref HEAD refs/heads/feature
tc "$C" "$C" "git merge --ff-only main" 0 "other branch checked out: merge --ff-only"
tc "$C" "$C" "git merge main" 0 "other branch checked out: merge"
tc "$C" "$C" "git merge --squash main" 0 "TOOL50: other branch checked out (a worker's branch): merge --squash"
exit $fail
