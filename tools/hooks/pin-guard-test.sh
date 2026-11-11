#!/usr/bin/env bash
# Pipe-tests for tools/hooks/pin-guard.sh (TOOL50). Run from the repo root (or
# a worktree root). Prints one line per case; exits 1 if any line is FAIL.
#
# The guard only reads its JSON input; no tested command is run. The paths
# it resolves lie in a fixture made under a mktemp dir in $AS_TEST_TMP (else
# $TMPDIR) and removed at exit -- never the real references/:
#   $P              a fake main repo, references/ ignored, with
#   $E = $P/references/emutos                 a throw-away git repo,
#   $O = $P/references/emutos-oracle/1a7484ab a clone of it,
#   $P/references/freemint                    an unprotected sibling;
#   $WT             a git worktree of $P whose references/emutos and
#                   references/emutos-oracle are symlinks into $P (TOOL44);
#   $CP             a scratch copy (git clone of $E) under target-agents/,
#   $AR             a `git archive` export of $E under target-agents/.
# PIN_GUARD=<script> (and PIN_BUILD_EMUTOS=<script>) run the cases against
# another guard (and build script) (TOOL50: shown once against the guards
# before TOOL50). Since the rework (owner 2026-11-11) the guard keeps only
# the OS/git lock (chmod, chflags, chown, git remote, git config remote.*,
# Write/Edit); a few "known limit" cases pin what it leaves to the lock.
G=${PIN_GUARD:-$PWD/tools/hooks/pin-guard.sh}
fail=0

base=${AS_TEST_TMP:-${TMPDIR:-}}
if [ -z "$base" ] || [ ! -d "$base" ]; then
    echo "FAIL: no temp base (set AS_TEST_TMP or TMPDIR to an existing dir)"
    exit 1
fi
tmp=$(mktemp -d "${base%/}/pin-test.XXXXXX") || { echo "FAIL: mktemp"; exit 1; }
tmp=$(cd -P "$tmp" && pwd -P)
trap 'rm -rf "$tmp"' EXIT
unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_COMMON_DIR
gq() { git -c user.name=t -c user.email=t@t.invalid -c init.defaultBranch=master -c core.hooksPath=/dev/null "$@" >/dev/null 2>&1; }

P=$tmp/proj
E=$P/references/emutos
O=$P/references/emutos-oracle/1a7484ab
WT=$tmp/wt
TA=$tmp/target-agents/agent-a0123456789abcdef
CP=$TA/emutos
AR=$TA/export
setup() {
    mkdir -p "$P" "$E/bios" "$P/references/freemint" "$TA" "$AR" || return 1
    gq init -q "$P" && printf 'references/\n' > "$P/.gitignore" && mkdir -p "$P/docs" \
        && printf 'x\n' > "$P/docs/x.md" && gq -C "$P" add -A && gq -C "$P" commit -q -m init || return 1
    gq init -q "$E" && printf 'readme\n' > "$E/README" && printf 'c\n' > "$E/bios/kprint.c" \
        && gq -C "$E" add -A && gq -C "$E" commit -q -m pin || return 1
    mkdir -p "$P/references/emutos-oracle" && gq clone -q "$E" "$O" || return 1
    gq -C "$P" worktree add -q -b wt "$WT" || return 1
    mkdir -p "$WT/references" && ln -s "$E" "$WT/references/emutos" \
        && ln -s "$P/references/emutos-oracle" "$WT/references/emutos-oracle" || return 1
    gq clone -q "$E" "$CP" || return 1
    git -C "$E" archive HEAD | tar -x -C "$AR" || return 1
}
if ! setup; then echo "FAIL: fixture setup in $tmp"; exit 1; fi

# t <project> <cwd> <command> <want exit> <description>
t() {
    jq -nc --arg c "$3" --arg d "$2" '{tool_name:"Bash",cwd:$d,tool_input:{command:$c}}' \
        | CLAUDE_PROJECT_DIR=$1 "$G" >/dev/null 2>&1
    rc=$?
    if [ "$rc" = "$4" ]; then r=ok; else r=FAIL; fail=1; fi
    echo "$r exit $rc (want $4): $5"
}
# w <tool> <path> <want exit> <description>
w() {
    jq -nc --arg t "$1" --arg p "$2" --arg d "$P" '{tool_name:$t,cwd:$d,tool_input:{file_path:$p,notebook_path:$p}}' \
        | CLAUDE_PROJECT_DIR=$P "$G" >/dev/null 2>&1
    rc=$?
    if [ "$rc" = "$3" ]; then r=ok; else r=FAIL; fail=1; fi
    echo "$r exit $rc (want $3): $4"
}

# ── TOOL50 rework (owner 2026-11-11): the guard keeps the OS/git lock ──
# Case variants of the protected paths (APFS is case-insensitive; on a
# case-sensitive volume they are compared case folded all the same).
EU=$P/REFERENCES/EmuTOS
OU=$P/References/EMUTOS-ORACLE/1A7484AB

# lk <command with @ for the tree> <description>: blocked with @ = E, O, the
# case variants, the worktree symlink path and $CLAUDE_PROJECT_DIR; allowed
# with @ = the scratch copy and the archive export.
lk() {
    local c=$1
    t "$P" "$P" "${c//@/$E}" 2 "E: $2"
    t "$P" "$P" "${c//@/$O}" 2 "O: $2"
    t "$P" "$P" "${c//@/$EU}" 2 "E, other letter case: $2"
    t "$P" "$P" "${c//@/$OU}" 2 "O, other letter case: $2"
    t "$WT" "$WT" "${c//@/$WT/references/emutos}" 2 "E via the worktree symlink: $2"
    t "$WT" "$WT" "${c//@/references/emutos-oracle/1a7484ab}" 2 "O via the worktree symlink, relative: $2"
    t "$WT" "$WT" "${c//@/\$CLAUDE_PROJECT_DIR/references/emutos}" 2 "E via \$CLAUDE_PROJECT_DIR: $2"
    t "$P" "$P" "${c//@/\$X/references/emutos}" 2 "E named through an undecidable variable (text): $2"
    t "$P" "$P" "${c//@/$CP}" 0 "scratch copy: $2"
    t "$P" "$P" "${c//@/$AR}" 0 "archive export: $2"
}
lk "chmod -R u+w @" "chmod -R u+w"
lk "chmod 644 @/README" "chmod a file"
lk "chflags -R nouchg @" "chflags -R nouchg"
lk "chflags nouchg @/bios/kprint.c" "chflags a file"
lk "chown -R \$USER @" "chown -R"
lk "sudo chmod -R a+w @" "sudo chmod"
lk "find @ -type f -exec chmod u+w {} +" "find -exec chmod"
lk "cd @ && chmod u+w README" "cd && chmod, relative"
lk "( cd @/bios && chflags nouchg kprint.c )" "sub-shell cd && chflags"
t "$P" "$P" "chmod -R u+w $P/references" 2 "chmod -R of a directory that contains E"
t "$P" "$P" "chflags -R nouchg references" 2 "chflags -R of references/, cwd the main repo"
t "$P" "$P" "chmod -R u+w $P/REFERENCES" 2 "chmod -R of references/, other letter case"
t "$P" "$P" "find references -exec chflags nouchg {} +" 2 "find from an ancestor -exec chflags"
t "$P" "$P" "chmod -R u+w \$CLAUDE_PROJECT_DIR" 2 "chmod -R of the project dir"
t "$P" "$P" "chmod -R u+w $P/references/emu*" 2 "chmod with a glob that reaches E"
t "$P" "$P" "chmod u+w $E/bios/*.c" 2 "chmod with a glob inside E"
ln -s "$E" "$tmp/alias"
t "$P" "$P" "chmod -R u+w $tmp/alias/" 2 "chmod through another symlink to E"
t "$P" "$P" "chflags nouchg $tmp/alias/../emutos-oracle/1a7484ab" 2 "chflags through a symlink and .."
t "$P" "$P" "chmod +x $P/tools/x.sh" 0 "chmod in the main repo"
t "$P" "$P" "chmod -R u+w $P/references/freemint" 0 "chmod of an unprotected sibling"
t "$P" "$P" "chmod -R u+w $P/references/emutos2" 0 "chmod of a name that only starts with emutos"
t "$WT" "$WT" "chmod -R u+w $WT/references" 0 "worktree: chmod of its references/ (symlinks only; -R does not follow)"
t "$P" "$P" "ls -lO $E && stat $E/README" 0 "reading the lock"
t "$P" "$P" "find $E -name '*.c' -exec grep -l x {} +" 0 "find -exec grep"
t "$P" "$P" "chmod u+w \$X" 0 "chmod of an undecidable path without the name"

# git remote / git config remote.*: blocked in E and O by every path form.
rem=(
    "remote set-url origin https://example.invalid/emutos.git" "remote add up https://example.invalid/emutos.git"
    "remote remove origin" "remote rm origin" "remote rename origin up"
    "config remote.origin.url https://example.invalid/emutos.git" "config --add remote.origin.fetch +refs/x:refs/x"
    "config --replace-all remote.origin.url x" "config --unset remote.origin.url" "config --unset-all remote.origin.fetch"
    "config set remote.origin.url x" "config unset remote.origin.url" "config --rename-section remote.origin up"
    "config --remove-section remote.origin" "config --local Remote.Origin.URL x" "config -e"
    "config --local url.https://example.invalid/.insteadOf https://github.com/" "config url.x.pushInsteadOf y"
    "config --remove-section url.x" "config branch.master.remote up" "config branch.master.pushRemote up"
)
for f in "${rem[@]}"; do
    t "$P" "$P" "git -C $E $f" 2 "E, -C: git $f"
    t "$P" "$P" "cd $O && git $f" 2 "O, cd &&: git $f"
    t "$P" "$E" "git $f" 2 "E, cwd: git $f"
    t "$P" "$P" "git -C $EU $f" 2 "E, other letter case: git $f"
    t "$WT" "$WT" "git -C references/emutos $f" 2 "E via the worktree symlink: git $f"
    t "$WT" "$WT" "git -C \$CLAUDE_PROJECT_DIR/references/emutos-oracle/1a7484ab $f" 2 "O via \$CLAUDE_PROJECT_DIR: git $f"
    t "$P" "$P" "git --git-dir=$E/.git $f" 2 "E, --git-dir=: git $f"
    t "$P" "$P" "GIT_DIR=$O/.git git $f" 2 "O, GIT_DIR=: git $f"
    t "$P" "$P" "git -C $CP $f" 0 "scratch copy: git $f"
    t "$P" "$CP" "git $f" 0 "scratch copy, cwd: git $f"
done
t "$P" "$P" "git config --file $E/.git/config remote.origin.url x" 2 "config --file in E: remote.* write"
t "$P" "$P" "git config -f $OU/.git/config --unset remote.origin.url" 2 "config -f in O, other letter case"
# TOOL50 retest: -f<path> attached and GIT_CONFIG name the file git config writes.
t "$P" "$P" "git config -f$E/.git/config remote.origin.url x" 2 "config -f<path> attached, in E"
t "$P" "$P" "git config -f$EU/.git/config url.x.insteadOf y" 2 "config -f<path> attached, E in other letter case, insteadOf"
t "$P" "$P" "GIT_CONFIG=$E/.git/config git config remote.origin.url x" 2 "GIT_CONFIG= prefix in E"
t "$P" "$P" "GIT_CONFIG=$O/.git/config git config --unset remote.origin.url" 2 "GIT_CONFIG= prefix in O: --unset"
t "$P" "$P" "export GIT_CONFIG=$E/.git/config; git config remote.origin.url x" 2 "export GIT_CONFIG in E"
t "$P" "$P" "env GIT_CONFIG=$WT/references/emutos/.git/config git config branch.master.remote x" 2 "env GIT_CONFIG= via the worktree symlink"
t "$P" "$P" "GIT_CONFIG_GLOBAL=$E/.git/config git config --global remote.origin.url x" 2 "GIT_CONFIG_GLOBAL= in E"
t "$P" "$P" "GIT_CONFIG=\$X/references/emutos/.git/config git config remote.origin.url x" 2 "GIT_CONFIG through an undecidable variable (text)"
t "$P" "$P" "GIT_CONFIG=$CP/.git/config git config remote.origin.url x" 0 "GIT_CONFIG in the scratch copy"
t "$P" "$P" "git config -f$CP/.git/config remote.origin.url x" 0 "config -f<path> in the scratch copy"
t "$P" "$P" "GIT_CONFIG=$E/.git/config git config --get remote.origin.url" 0 "GIT_CONFIG in E: read"
t "$P" "$P" "( export GIT_CONFIG=$E/.git/config ); git -C $CP config remote.origin.url x" 0 "GIT_CONFIG exported in a sub-shell does not leak"
t "$P" "$P" "git -C $E config branch.master.merge refs/heads/x" 0 "known limit: other config keys (branch.*.merge)"
t "$P" "$P" "chgrp -R staff $E" 0 "known limit: chgrp"
t "$P" "$P" "git --git-dir=$CP/.git --work-tree=$E remote set-url origin x" 2 "E as --work-tree (repo found elsewhere; blocked, the tree is named)"
for f in "remote" "remote -v" "remote show origin" "remote get-url origin" "config remote.origin.url" \
    "config --get remote.origin.url" "config --get-regexp remote" "config -l" "config list" "config get remote.origin.url"; do
    t "$P" "$P" "git -C $E $f" 0 "E, read: git $f"
done
t "$P" "$P" "git config --file $E/.git/config --get remote.origin.url" 0 "config --file in E: read"
t "$P" "$P" "git -C $P remote set-url origin x" 0 "the main repo's own remote"
t "$P" "$P" "git -C $E config core.x 1" 0 "known limit, owner decision: other config writes (the OS lock)"
t "$P" "$P" "git -C $E fetch" 0 "known limit, owner decision: fetch (the OS lock, the invalid remote)"
t "$P" "$P" "rm -rf $E" 0 "known limit, owner decision: rm (the OS lock, uchg)"
t "$P" "$P" "echo x > $E/x" 0 "known limit, owner decision: redirection (the OS lock)"
t "$P" "$P" "timeout 5 chmod -R u+w $E" 0 "known limit: a wrapper the parser does not know (timeout)"
t "$P" "$P" "cargo xtask test" 0 "unrelated"

# ── Write / Edit ────────────────────────────────────────────────────────
w Write "$E/x.c" 2 "Write into E"
w Edit "$E/bios/kprint.c" 2 "Edit in E"
w Write "$O/obj/x" 2 "Write into O (new dir)"
w Edit "$WT/references/emutos/README" 2 "Edit through the worktree symlink"
w NotebookEdit "$E/n.ipynb" 2 "NotebookEdit in E"
w Write "$EU/x.c" 2 "Write into E, other letter case"
w Edit "$OU/README" 2 "Edit in O, other letter case"
w Write "$CP/x.c" 0 "Write into the scratch copy"
w Write "$P/docs/x.md" 0 "Write into the repo"
w Write "$P/references/freemint/x" 0 "Write into an unprotected sibling"
w Read "$E/README" 0 "Read: not checked"

# ── TOOL50 point 2: build-emutos.sh refuses a tree dir under references/ ──
# A copy of the script in the fixture repo, with a cross compiler that does
# not exist: a tree dir the refusal lets through stops at "no cross gcc"
# (exit 1) before any git or make, so even a broken refusal touches nothing.
# b <tree dir> <cwd> <want exit> <description>
mkdir -p "$P/tools/oracle"
mkdir -p "$P/tools/hooks/lib"
cp tools/hooks/lib/paths.sh "$P/tools/hooks/lib/"
cp "${PIN_BUILD_EMUTOS:-tools/oracle/build-emutos.sh}" "$P/tools/oracle/build-emutos.sh"
printf 'ORACLE_CROSS_MINT=%s/no-such-cross/m68k-atari-mint-\n' "$tmp" > "$P/tools/oracle/oracle.env"
b() {
    out=$(cd "$2" && sh "$P/tools/oracle/build-emutos.sh" 1a7484ab 2026-04-14 "$1" 2>&1)
    rc=$?
    if [ "$rc" = "$3" ]; then r=ok; else r=FAIL; fail=1; fi
    echo "$r exit $rc (want $3): build-emutos.sh $4"
    case "$3:$out" in 2:*refusing*|1:*"no cross gcc"*) ;; *) echo "FAIL: build-emutos.sh $4: unexpected output: $out"; fail=1 ;; esac
}
b references/emutos-oracle/1a7484ab "$P" 2 "refuses the oracle tree (relative)"
b "$O" "$tmp" 2 "refuses the oracle tree (absolute)"
b "$P/references/emutos-oracle/new" "$tmp" 2 "refuses a new tree in references/emutos-oracle/"
b "$E" "$tmp" 2 "refuses references/emutos"
b "$OU" "$tmp" 2 "refuses the oracle tree in another letter case"
b "$P/REFERENCES/new" "$tmp" 2 "refuses a new tree under REFERENCES/"
b "$WT/references/emutos-oracle/1a7484ab" "$tmp" 2 "refuses the oracle tree through a worktree's symlink"
b "$tmp/alias/../emutos-oracle/x" "$tmp" 2 "refuses a path through another symlink"
b "$P/references/freemint/x" "$tmp" 2 "refuses any tree dir under references/"
b "$TA/emutos-1a7484ab" "$tmp" 1 "lets a tree dir in target-agents through (then: no cross gcc)"
exit $fail
