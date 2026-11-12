#!/usr/bin/env bash
#
# role-guard.sh -- PreToolUse hook (.claude/settings.json, Write|Edit|
# NotebookEdit) that holds every team role to the files its brief allows
# (.claude/agents/<role>.md, "Limits" and "Where you work").
#
# The role is the hook input's `agent_type` (the subagent's name, or the
# name the main session runs as with the `agent` setting / `--agent`); an
# explicit first argument overrides it (tests). No `agent_type` = the main
# session without an agent = `lead`. A role this table does not know may
# write nowhere inside the repository.
#
# The file's tree is the git toplevel of its nearest existing directory:
#   main      the main repo's own tree (parent of the git common dir of
#             $CLAUDE_PROJECT_DIR)
#   worktree  another worktree of the main repo, with a branch checked out
#   detached  another worktree of the main repo, HEAD detached
# A file outside every git repo, or in a repo with another common dir (a
# scratch clone), passes: scratch-guard.sh and pin-guard.sh own those.
# Inside any tree, target/ and target-agents/ pass for every role (builds
# and worker scratch, gitignored) unless the path lies in a nested worktree,
# which is its own tree.
#
#   role        main tree                         worktree (branch)                  detached
#   lead        TASK/ (no DONE receipt),          nothing                            nothing
#               docs/STATUS.md, docs/archive/,
#               docs/MEMORY.md, CLAUDE.md
#   developer   nothing                           all but docs/STATUS.md,            nothing
#                                                 docs/archive/, DONE receipts
#   researcher  nothing                           docs/ but docs/STATUS.md,          nothing
#                                                 docs/archive/
#   tester      measurements/measurement-*/       nothing (a developer's branch)     all
#   auditor     nothing                           on audit-*: docs/audits/,          all
#                                                 tosdump/cases/, tosdump/reference/,
#                                                 measurements/; else nothing
#
# A DONE receipt (TASK/*-STATUS-DONE*.MD) is written by `cargo xtask accept`
# only, so no role may Write or Edit one.
#
# Blocks with exit 2 (message on stderr, shown to the agent). Paths compare
# physical and ASCII case folded (tools/hooks/lib/paths.sh). Known limit, as
# in the other guards: only the Write/Edit/NotebookEdit tools are checked; a
# file written by a Bash command (redirection, sed -i, cp, a script) is not
# seen. A guard that cannot find the main repo blocks.

. "$(dirname "${BASH_SOURCE[0]}")/lib/paths.sh" || { echo "role-guard: tools/hooks/lib/paths.sh is missing" >&2; exit 2; }

input=$(cat)
tool=$(printf '%s' "$input" | jq -r '.tool_name // empty')
case "$tool" in
    Write|Edit) file=$(printf '%s' "$input" | jq -r '.tool_input.file_path // empty') ;;
    NotebookEdit) file=$(printf '%s' "$input" | jq -r '.tool_input.notebook_path // empty') ;;
    *) exit 0 ;;
esac
[ -n "$file" ] || exit 0

role=${1:-$(printf '%s' "$input" | jq -r '.agent_type // empty')}
[ -n "$role" ] || role=lead

deny() {
    echo "role-guard: $role may not write $rel in the $kind tree ($ftop): $1" >&2
    echo "role-guard: see .claude/agents/$role.md, Limits / Where you work" >&2
    exit 2
}

# The file, absolute and physical.
case "$file" in
    /*) ;;
    *) cwd=$(printf '%s' "$input" | jq -r '.cwd // empty'); file=${cwd:-$PWD}/$file ;;
esac
path_physical "$file"; file=$PH

# The main repo: parent of the git common dir of the project.
proj=${CLAUDE_PROJECT_DIR:-$PWD}
common_of() {
    ( cd "$1" 2>/dev/null && c=$(git rev-parse --git-common-dir 2>/dev/null) && cd "$c" 2>/dev/null && pwd -P )
}
main_common=$(common_of "$proj")
if [ -z "$main_common" ]; then
    echo "role-guard: cannot find the main repo from CLAUDE_PROJECT_DIR=$proj; write blocked" >&2
    exit 2
fi
path_physical "${main_common%/*}"; main_top=$PH

# The file's tree.
dir=${file%/*}
while [ -n "$dir" ] && [ ! -d "$dir" ]; do dir=${dir%/*}; done
[ -n "$dir" ] || dir=/
ftop=$(git -C "$dir" rev-parse --show-toplevel 2>/dev/null) || exit 0
[ -n "$ftop" ] || exit 0
path_physical "$ftop"; ftop=$PH
file_common=$(common_of "$ftop")
path_same "$file_common" "$main_common" || exit 0     # a scratch clone

lfile=$(path_lower "$file"); ltop=$(path_lower "$ftop")
case "$lfile" in
    "$ltop"/*) rel=${lfile#"$ltop"/} ;;
    *) exit 0 ;;
esac

branch=
if path_same "$ftop" "$main_top"; then
    kind=main
else
    branch=$(git -C "$ftop" symbolic-ref -q --short HEAD 2>/dev/null)
    if [ -n "$branch" ]; then kind=worktree; else kind=detached; fi
fi

# Builds and worker scratch, every role.
case "$rel" in target/*|target-agents/*) exit 0 ;; esac

# Nobody writes a DONE receipt by hand.
case "$rel" in
    task/*-status-done*.md) deny "a DONE receipt is written by cargo xtask accept only" ;;
esac

lead_only() {
    case "$rel" in
        docs/status.md|docs/archive/*) deny "docs/STATUS.md and its archive are the lead's" ;;
    esac
}

case "$role" in
    lead)
        [ "$kind" = main ] || deny "the lead works in the main tree only"
        case "$rel" in
            task/*|docs/status.md|docs/archive/*|docs/memory.md|claude.md) exit 0 ;;
        esac
        deny "the lead writes TASK/, docs/STATUS.md, docs/archive/, docs/MEMORY.md and CLAUDE.md only"
        ;;
    developer)
        [ "$kind" = worktree ] || deny "the developer works on its own branch in its own worktree"
        lead_only
        exit 0
        ;;
    researcher)
        [ "$kind" = worktree ] || deny "the researcher works on its own branch in its own worktree"
        lead_only
        case "$rel" in docs/*) exit 0 ;; esac
        deny "the researcher writes under docs/ only"
        ;;
    tester)
        case "$kind" in
            detached) exit 0 ;;
            worktree) deny "a tree with branch $branch is another worker's; test in your own detached worktree" ;;
        esac
        case "$rel" in measurements/measurement-*/*) exit 0 ;; esac
        deny "in the main tree the tester writes its run directory under measurements/ only"
        ;;
    auditor)
        case "$kind" in
            detached) exit 0 ;;
            main) deny "the auditor never writes in the main tree" ;;
        esac
        case "$branch" in
            audit-*) ;;
            *) deny "a tree with branch $branch is another worker's; write on your audit-* branch" ;;
        esac
        case "$rel" in
            docs/audits/*|tosdump/cases/*|tosdump/reference/*|measurements/*) exit 0 ;;
        esac
        deny "the auditor writes docs/audits/, tosdump/cases/, tosdump/reference/ and measurements/ only"
        ;;
    *)
        deny "unknown role; no write inside the repository"
        ;;
esac
