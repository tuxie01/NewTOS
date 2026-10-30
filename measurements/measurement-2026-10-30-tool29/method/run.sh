#!/usr/bin/env bash
#
# run.sh -- TOOL29 acceptance: the doc-check pre-commit hook in a throwaway
# clone. Usage: run.sh <repo> <scratch-dir> [variant-a]
#
# Second run (lead review): cases 7 and 8 added (legacy errors in a staged
# doc never block; a newly broken cite does).
#
# Clones <repo> (git clone --shared, so the repo's git state is untouched)
# into <scratch-dir>/clone, links the gitignored reference trees in, copies
# the uncommitted TOOL29 change from <repo>'s working tree, commits it there
# as the base, runs `cargo xtask install-hooks` and drives the cases below,
# each timed (wall clock of the git commit, hook included). With
# "variant-a" it first removes the cite pre-filter of `introduced` in the
# clone's copy (every cite of every doc the commit leaves alone is resolved
# twice), to measure the full-repo variant. Writes nothing into <repo>.

set -u
repo=$1
work=$2
variant=${3:-b}
clone=$work/clone
TIMEFORMAT='    wall %R s'

rm -rf "$clone"
git clone -q --shared "$repo" "$clone" || exit 1
for d in emutos freemint emutos-oracle; do
    [ -e "$repo/references/$d" ] && ln -s "$repo/references/$d" "$clone/references/$d"
done
cd "$clone" || exit 1
for f in tools/xtask/src/doc_check.rs tools/xtask/src/main.rs tools/xtask/src/git_hooks.rs \
         tools/git-hooks/pre-commit TASK/MEM12.md docs/sdk/06-image-and-testing.md; do
    mkdir -p "$(dirname "$f")"
    cp -p "$repo/$f" "$f"
done
if [ "$variant" = variant-a ]; then
    # Drop the pre-filter: resolve every cite of every untouched doc.
    perl -0pi -e 's/if !components\.contains\(last\) \{\n\s*continue;\n\s*\}/let _ = last;/' tools/xtask/src/doc_check.rs
    grep -q 'let _ = last;' tools/xtask/src/doc_check.rs || { echo "variant-a patch failed"; exit 1; }
fi
cargo -q build -p xtask 2>/dev/null || { echo "xtask build failed"; exit 1; }
git add -A tools TASK docs
git commit -q -m "base: TOOL29 change under test" || exit 1
echo "clone HEAD: $(git log -1 --format='%h %s')  (repo HEAD $(git -C "$repo" rev-parse --short HEAD))"
cargo -q xtask install-hooks
echo "core.hooksPath = $(git config --get core.hooksPath)"

head_is() { echo "    HEAD now: $(git log -1 --format=%s)"; }
case_() { echo; echo "=== $1"; }

main_line=$(grep -n '^fn main() -> ExitCode' tools/xtask/src/main.rs | cut -d: -f1)

if [ "$variant" = b ]; then
    case_ "1: a staged doc with a broken line reference -> rejected"
    printf '\nHook test: `tools/xtask/src/main.rs:99999@main`\n' >> TASK/TOOL29.md
    git add TASK/TOOL29.md
    time git commit -q -m "case 1"; echo "    git commit exit $?"; head_is

    case_ "3: chained like 'doc-check ... | tail -1 && git commit' -> still rejected"
    echo '    $ cargo -q xtask doc-check TASK/TOOL29.md | tail -1 && git commit -q -m "case 3"'
    time { cargo -q xtask doc-check TASK/TOOL29.md 2>&1 | tail -1 && git commit -q -m "case 3"; }
    echo "    chain exit $?"; head_is

    case_ "2: the same doc with the reference fixed -> passes"
    sed -i '' "s/main.rs:99999@main/main.rs:${main_line}@main/" TASK/TOOL29.md
    git add TASK/TOOL29.md
    time git commit -q -m "case 2"; echo "    git commit exit $?"; head_is

    case_ "6a: partial staging, staged copy broken, working copy clean -> rejected"
    sed -i '' "s/main.rs:${main_line}@main/main.rs:99998@main/" TASK/TOOL29.md
    git add TASK/TOOL29.md
    sed -i '' "s/main.rs:99998@main/main.rs:${main_line}@main/" TASK/TOOL29.md
    time git commit -q -m "case 6a"; echo "    git commit exit $?"; head_is
    git add TASK/TOOL29.md

    case_ "6b: partial staging, staged copy clean, working copy broken -> passes"
    printf 'Second line: `tools/xtask/src/main.rs:%s@main`\n' "$main_line" >> TASK/TOOL29.md
    git add TASK/TOOL29.md
    printf 'Unstaged: `tools/xtask/src/main.rs:99997@main`\n' >> TASK/TOOL29.md
    time git commit -q -m "case 6b"; echo "    git commit exit $?"; head_is

    case_ "6c: git commit -a picks up the broken working copy -> rejected (hook reads GIT_INDEX_FILE)"
    time git commit -q -a -m "case 6c"; echo "    git commit exit $?"; head_is
    sed -i '' '$d' TASK/TOOL29.md

    case_ "7: a staged doc with legacy errors (TASK/X03.md) plus an unrelated clean edit -> passes"
    echo "    legacy before the edit (plain doc-check, working tree):"
    cargo -q xtask doc-check TASK/X03.md 2>&1 | sed 's/^/      /'
    printf '\nHook test: a note without cites.\n' >> TASK/X03.md
    git add TASK/X03.md
    time git commit -q -m "case 7"; echo "    git commit exit $?"; head_is

    case_ "8: the same doc with a newly broken cite -> rejected"
    printf 'Hook test: `tools/xtask/src/main.rs:99996@main`\n' >> TASK/X03.md
    git add TASK/X03.md
    time git commit -q -m "case 8"; echo "    git commit exit $?"; head_is
    git reset -q TASK/X03.md
    git checkout -q -- TASK/X03.md

    case_ "4a: a commit without docs or code (README.md) -> passes, no cargo run"
    echo "" >> README.md
    git add README.md
    time git commit -q -m "case 4a"; echo "    git commit exit $?"; head_is
fi

case_ "4b: a code-only commit that moves no cited line (comment at the end of tos-api/src/bios/chardev.rs) -> passes"
echo "// TOOL29 hook test" >> tos-api/src/bios/chardev.rs
git add tos-api/src/bios/chardev.rs
time git commit -q -m "case 4b"; echo "    git commit exit $?"; head_is

case_ "5: a staged code change shifting lines under anchored cites in docs that are NOT staged -> rejected"
{ head -1 tos-api/src/bios/chardev.rs; for i in $(seq 20); do echo "// TOOL29 hook test filler $i"; done; tail -n +2 tos-api/src/bios/chardev.rs; } > chardev.tmp
mv chardev.tmp tos-api/src/bios/chardev.rs
git add tos-api/src/bios/chardev.rs
time git commit -q -m "case 5" > case5.out 2>&1; rc=$?
grep -c 'this commit breaks it' case5.out | sed 's/^/    broken cites reported: /'
head -3 case5.out | sed 's/^/    /'; echo "    ..."; tail -3 case5.out | sed 's/^/    /'
rm -f case5.out
echo "    git commit exit $rc"; head_is
