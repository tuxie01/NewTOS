#!/usr/bin/env bash
#
# no-verify-guard.sh -- PreToolUse hook (.claude/settings.json, Bash) that
# keeps agents from skipping the git pre-commit hook (TASK/TOOL29; user
# decision 2026-10-30: agents are blocked, the owner may still bypass by hand;
# TASK/TOOL42, owner 2026-11-11: plumbing and fast-forwards into main).
#
# Blocks with exit 2 (message on stderr, shown to the agent) a Bash command
# that runs git commit/push/merge/rebase/am with
#   --no-verify, or `git commit` with a short-flag cluster containing n
#   (`-n`, `-an`, ...: git commit's -n is --no-verify), or
#   an inline hook override `-c core.hooksPath=...`, or
#   `git config core.hooksPath` set to anything but the checked-in hook
#   directory, or unset.
# In the main repo (TOOL42) it also blocks the plumbing that makes or moves a
# branch commit without `git commit`, so without the pre-commit hook:
#   git commit-tree (so every mktree + commit-tree chain), git fast-import,
#   git filter-branch, git filter-repo (also `git-filter-repo`),
#   git replace (listing: no args, -l, --list, stays allowed),
#   git update-ref on HEAD, refs/heads/*, a bare name, also with -d, and
#     --stdin (other refs/*, e.g. refs/tags/*, stay allowed),
#   git symbolic-ref that sets or deletes a ref (reading stays allowed),
#   git hash-object -w -t commit (without -t the object is a blob, also from
#     --stdin, so blob hash-object -w + update-index --cacheinfo stays allowed);
# and, while main is checked out in the target (TOOL42 point 4; TOOL50
# point 4, owner 2026-11-11: `cargo xtask land <branch> -F <msg>` is the only
# merge path into main for agents):
#   git merge --ff-only, git merge <branch> with or without --squash (--abort,
#     --continue, --quit, a bare `git merge` of the upstream and a merge of
#     remote-tracking refs only -- refs/remotes/*, <remote>/<branch> for a
#     remote the target has, as `git pull` does -- stay allowed), and
#     git pull from `.` or a local path (from a remote: allowed).
#   `cargo xtask land` runs its `git merge --squash` as a subprocess, not
#   through this hook. A squash merge in a worktree of the main repo that has
#   another branch checked out (a worker's own branch) is not a merge into
#   main and stays allowed, like every merge on a branch other than main.
# Paths are compared canonical: repeated slashes squeezed, trailing slash
# dropped, symlinks resolved (`cd -P`, `pwd -P`).
# "Main repo" is the git common dir of $CLAUDE_PROJECT_DIR, so its worktrees
# count too. The target repo of each git call is read from the command text:
# `git -C <path>`, `--git-dir`, a `GIT_DIR=` prefix or export, a preceding
# `cd`/`pushd` (a sub-shell or $( ) restores it), else the tool call's `cwd`
# from the hook input (fallback: the hook's own working directory). A target
# with another common dir (a scratch clone) is allowed. A plumbing target the
# text cannot decide (a variable other than $HOME/$TMPDIR/$CLAUDE_PROJECT_DIR,
# `cd -`, a path that does not exist yet) is blocked; for the merge/pull rule
# it is allowed, so the optional rule never over-blocks.
#
# The command-text parser (canon, common_dir, expand, resolve, process_seg,
# parse_command) is shared with tools/hooks/pin-guard.sh in
# tools/hooks/lib/shell-parse.sh; repo paths compare case folded by
# path_same of tools/hooks/lib/paths.sh (APFS is case-insensitive; TOOL50).
#
# Text match on the command, like scratch-guard.sh: a command assembled at run
# time is not seen -- `eval`, `bash -c '...'`, a script file, a git alias,
# `xargs` with the git verb in its input, a verb quoted or escaped inside the
# word (`git commit-"tree"`, `git commit-tr\ee`), a command word held in a
# variable (`g=git; $g commit-tree`). The pattern guard can never be complete
# (owner 2026-11-11); its scope stays as above. Not blocked either, by owner
# decision 2026-11-11: commands that move a branch to a commit that already
# exists or replay commits without the pre-commit hook (git reset, branch -f,
# checkout -B, cherry-pick, rebase, revert, am, apply, read-tree, fetch/push
# into the repo incl. `git fetch . x:main`, fetch + reset, merge on a branch
# other than main), so a commit made by plumbing in a scratch clone can still
# be fetched in; the backstops for those are server-side (ticket TOOL43) and,
# for references/emutos*, the OS-level read-only lock (TOOL50), not this guard.
# Message texts are not parsed: pass a commit message with
# `git commit -F <file>` if it must contain these words.

set -u
export LC_ALL=C

input=$(cat)
tool=$(printf '%s' "$input" | jq -r '.tool_name // empty')
[ "$tool" = "Bash" ] || exit 0
cmd=$(printf '%s' "$input" | jq -r '.tool_input.command // empty')
call_cwd=$(printf '%s' "$input" | jq -r '.cwd // empty')
[ -n "$call_cwd" ] || call_cwd=$PWD

block() {
    echo "no-verify-guard: $1" >&2
    echo "${2:-the pre-commit doc-check must run (TASK/TOOL29); fix the reported docs instead}" >&2
    exit 2
}

# A quoted command word (`"git"`, `'git'`) passes this filter too (TOOL42 F2).
printf '%s' "$cmd" | grep -qE '(^|[^A-Za-z0-9_-])git([[:space:]"'"'"']|-|$)' || exit 0

if printf '%s' "$cmd" | grep -qE '(^|[[:space:]])--no-verify([[:space:]=]|$)'; then
    block "--no-verify is not allowed for agents"
fi
if printf '%s' "$cmd" | grep -qE -- '-c[[:space:]]*core\.hooks[Pp]ath='; then
    block "an inline core.hooksPath override is not allowed for agents"
fi
# `git config core.hooksPath`: agents may only read it or set it to the
# checked-in hook directory (user decision 2026-10-30). Any other value
# (empty, /dev/null, another dir) and --unset/--unset-all are blocked, in any
# config scope. Git config keys are case-insensitive.
repo=$(git -C "${CLAUDE_PROJECT_DIR:-.}" rev-parse --show-toplevel 2>/dev/null || echo "${CLAUDE_PROJECT_DIR:-.}")
allowed_rel="tools/git-hooks"
seg=$(printf '%s\n' "$cmd" | tr ';&|' '\n\n\n' | grep -iE '(^|[^A-Za-z0-9_-])git([[:space:]]+-[^[:space:]]+)*[[:space:]]+config([[:space:]].*)?[[:space:]]core\.hookspath' || true)
if [ -n "$seg" ]; then
    while IFS= read -r line; do
        [ -n "$line" ] || continue
        if printf '%s' "$line" | grep -qE '(^|[[:space:]])--unset(-all)?([[:space:]]|$)'; then
            block "unsetting core.hooksPath is not allowed for agents"
        fi
        # A read (--get, or no value after the key) is allowed.
        printf '%s' "$line" | grep -qE '(^|[[:space:]])--get' && continue
        rest=$(printf '%s' "$line" | sed -nE 's/.*[[:space:]][Cc][Oo][Rr][Ee]\.[Hh][Oo][Oo][Kk][Ss][Pp][Aa][Tt][Hh]([[:space:]]+(.*))?$/\1/p')
        [ -n "$rest" ] || continue
        val=$(printf '%s' "$rest" | sed -E 's/^[[:space:]]+//; s/[[:space:]]+$//')
        val=${val#\"}; val=${val%\"}; val=${val#\'}; val=${val%\'}; val=${val%/}
        case "$val" in
            "$allowed_rel"|"./$allowed_rel"|"$repo/$allowed_rel") ;;
            *) block "core.hooksPath may only be set to $allowed_rel (got: '${val}')" ;;
        esac
    done <<EOF_SEG
$seg
EOF_SEG
fi

# `git commit ... -n` / `-an` / `-nm` ...: short-flag clusters after `git commit`,
# within the same simple command (tokens stop at ; & |).
if printf '%s' "$cmd" | grep -qE '(^|[^A-Za-z0-9_-])git([[:space:]]+-[^[:space:]]+)*[[:space:]]+commit([[:space:]]+[^[:space:];&|]+)*[[:space:]]+-[A-Za-mo-z]*n[A-Za-z]*([[:space:];&|]|$)'; then
    block "git commit -n (= --no-verify) is not allowed for agents"
fi

# ── TOOL42: plumbing and fast-forwards in the main repo ─────────────────
# The command is split into simple commands by the shared parser
# (tools/hooks/lib/shell-parse.sh: quote-aware, $( ) and sub-shells,
# comments and here-documents skipped; cd/pushd/popd and GIT_DIR tracked).
# Each simple command whose command word is git (also after env/command/
# exec/nohup/nice/sudo/time/xargs) is classified by on_command.
printf '%s' "$cmd" | grep -qE 'commit-tree|update-ref|hash-object|fast-import|replace|symbolic-ref|filter-branch|filter-repo|merge|pull' || exit 0

unset GIT_DIR GIT_COMMON_DIR GIT_WORK_TREE
PLUMB_WHY="commits in the main repo go through git commit and its pre-commit hook (TASK/TOOL42); use a scratch clone for plumbing"
MERGE_WHY="a worktree branch lands in main only by cargo xtask land <branch> -F <msg> (TASK/TOOL46, TASK/TOOL50; CLAUDE.md Commit Discipline)"
for l in paths shell-parse; do . "$(dirname "${BASH_SOURCE[0]}")/lib/$l.sh" || block "tools/hooks/lib/$l.sh is missing" "the agent guards need their shared helpers"; done

main_common=""
main_repo() {
    [ -n "$main_common" ] && return
    main_common=$(common_dir "${CLAUDE_PROJECT_DIR:-$call_cwd}" "")
    [ -n "$main_common" ] || main_common="?"   # unknown: every target counts as main
}

# classify <subcommand> <args...> -> why (block reason) and kind (plumb|merge).
classify() {
    local sub=$1 a skip=0 n1=0 n2=0 n3=0 t="" nt=0
    shift
    why=""; kind=plumb; MPOS=""
    case "$sub" in
        commit-tree|fast-import|filter-branch|filter-repo) why="git $sub" ;;
        replace)
            # n1: -l/--list, n2: a positional, n3: another option
            for a in "$@"; do
                case "$a" in -l|--list) n1=1 ;; --format=*) ;; -*) n3=1 ;; *) n2=1 ;; esac
            done
            if [ $n3 = 1 ] || { [ $n2 = 1 ] && [ $n1 = 0 ]; }; then why="git replace"; fi ;;
        update-ref)
            t="-"
            for a in "$@"; do
                if [ $skip = 1 ]; then skip=0; continue; fi
                case "$a" in
                    -m) skip=1 ;;
                    --stdin) why="git update-ref --stdin"; return ;;
                    -*) ;;
                    *) [ "$t" = "-" ] && t=$a ;;
                esac
            done
            case "$t" in
                -) ;;
                *"$DYN"*) why="git update-ref on a ref named at run time" ;;
                refs/heads/*) why="git update-ref $t" ;;
                refs/*) ;;
                *) why="git update-ref $t" ;;
            esac ;;
        symbolic-ref)
            for a in "$@"; do
                if [ $skip = 1 ]; then skip=0; continue; fi
                case "$a" in -m) skip=1 ;; -d|--delete) n1=1 ;; -*) ;; *) n2=$((n2 + 1)) ;; esac
            done
            if [ $n1 = 1 ] || [ $n2 -ge 2 ]; then why="git symbolic-ref (set or delete)"; fi ;;
        hash-object)
            for a in "$@"; do
                if [ $nt = 1 ]; then t=$a; nt=0; continue; fi
                case "$a" in
                    -w) n1=1 ;;
                    -t) nt=1 ;;
                    --type=*) t=${a#--type=} ;;
                    --*) ;;
                    -t*) t=${a#-t} ;;
                    -*) case "$a" in *w*) n1=1 ;; esac
                        case "$a" in *t) nt=1 ;; esac ;;
                esac
            done
            if [ $n1 = 1 ]; then
                case "$t" in
                    commit) why="git hash-object -w -t commit" ;;
                    *"$DYN"*) why="git hash-object -w with a type named at run time" ;;
                esac
            fi ;;
        merge)
            kind=merge
            for a in "$@"; do
                if [ $skip = 1 ]; then skip=0; continue; fi
                case "$a" in
                    --abort|--continue|--quit) why=""; return ;;
                    --squash) n1=1 ;;
                    --no-squash) n1=0 ;;
                    --ff-only) n3=1 ;;
                    -m|-s|-X|-F|--file|--strategy|--strategy-option|--into-name) skip=1 ;;
                    -*) ;;
                    *) n2=1; MPOS=$MPOS$a$US ;;
                esac
            done
            if [ $n3 = 1 ]; then why="git merge --ff-only"
            elif [ $n1 = 0 ] && [ $n2 = 1 ]; then why="git merge without --squash"
            elif [ $n2 = 1 ]; then why="git merge --squash"; fi ;;
        pull)
            kind=merge
            for a in "$@"; do
                case "$a" in
                    -*) ;;
                    .|/*|./*|../*|"~"*|file://*) why="git pull from a local repo"; return ;;
                esac
            done ;;
    esac
}

# on_command (called by process_seg for every simple command): the TOOL42
# rules for a git subcommand.
on_command() {
    local tc head
    [ -n "$sub" ] || return 0
    classify "$sub" "${SW[@]:k}"
    [ -n "$why" ] || return 0
    if [ -n "$gd" ]; then
        if [ "$gd" = "?" ]; then tc="?"; else tc=$(common_dir / "$gd"); fi
    elif [ "$tdir" = "?" ]; then tc="?"
    else tc=$(common_dir "$tdir" "")
    fi
    [ -n "$tc" ] || tc="?"
    main_repo
    if [ "$kind" = plumb ]; then
        [ "$tc" = "?" ] && block "$why: the target repo cannot be decided from the command text (named at run time, or not an existing git repo yet); name an existing repo literally (git -C <path>)" "$PLUMB_WHY"
        if [ "$main_common" = "?" ] || path_same "$tc" "$main_common"; then
            block "$why in the main repo is not allowed for agents" "$PLUMB_WHY"
        fi
        return 0
    fi
    # merge/pull: only a decided target that is the main repo with main checked out.
    [ "$tc" != "?" ] && path_same "$tc" "$main_common" || return 0
    if [ -n "$gd" ]; then head=$(git --git-dir="$gd" symbolic-ref -q HEAD 2>/dev/null)
    else head=$(git -C "$tdir" symbolic-ref -q HEAD 2>/dev/null)
    fi
    [ "$head" = refs/heads/main ] || return 0
    # A merge of remote-tracking refs only (refs/remotes/*, <remote>/<branch>)
    # is what `git pull` does: allowed.
    if [ -n "$MPOS" ] && remote_refs_only "$tdir" "$gd"; then return 0; fi
    block "$why while main is checked out is not allowed for agents" "$MERGE_WHY"
}

# remote_refs_only <dir> <git dir or empty>: every name in MPOS is
# refs/remotes/* or <remote>/<branch> for a remote the target repo has.
remote_refs_only() {
    local remotes s a r hit
    if [ -n "$2" ]; then remotes=$(git --git-dir="$2" remote 2>/dev/null)
    else remotes=$(git -C "$1" remote 2>/dev/null)
    fi
    s=$MPOS
    while [ -n "$s" ]; do
        a=${s%%"$US"*}
        s=${s#*"$US"}
        case "$a" in *"$DYN"*) return 1 ;; refs/remotes/?*) continue ;; esac
        hit=0
        for r in $remotes; do
            case "$a" in "$r"/?*) hit=1; break ;; esac
        done
        [ $hit = 1 ] || return 1
    done
    return 0
}

parse_command
exit 0
