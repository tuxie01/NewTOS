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
# and, while main is checked out in the target (TOOL42 point 4: worktree
# branches land by `git merge --squash` only):
#   git merge --ff-only, git merge <branch> without --squash (--squash,
#     --abort, --continue, --quit, a bare `git merge` of the upstream and a
#     merge of remote-tracking refs only -- refs/remotes/*, <remote>/<branch>
#     for a remote the target has, as `git pull` does -- stay allowed), and
#     git pull from `.` or a local path (from a remote: allowed).
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
# Text match on the command, like scratch-guard.sh: a command assembled at run
# time is not seen -- `eval`, `bash -c '...'`, a script file, a git alias,
# `xargs` with the git verb in its input, a verb quoted or escaped inside the
# word (`git commit-"tree"`, `git commit-tr\ee`), a command word held in a
# variable (`g=git; $g commit-tree`). The pattern guard can never be complete
# (owner 2026-11-11); its scope stays as above. Not blocked either, by owner
# decision 2026-11-11: commands that move a branch to a commit that already
# exists or replay commits without the pre-commit hook (git reset, branch -f,
# checkout -B, cherry-pick, rebase, revert, am, fetch/push into the repo,
# fetch + reset, merge on a branch other than main), so a commit made by
# plumbing in a scratch clone can still be fetched in; the backstop for those
# is server-side (ticket TOOL43), not this guard.
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
# The command is split into simple commands, quote-aware (', ", \), at
# ; & | && || newline, ( ) and $( ) / backticks (inner commands are checked
# too), skipping comments and here-document bodies. Each simple command
# whose command word is git (also after env/command/exec/nohup/nice/sudo/
# time/xargs) is classified; cd/pushd/popd and GIT_DIR exports update the
# directory the following commands run in.
printf '%s' "$cmd" | grep -qE 'commit-tree|update-ref|hash-object|fast-import|replace|symbolic-ref|filter-branch|filter-repo|merge|pull' || exit 0

unset GIT_DIR GIT_COMMON_DIR GIT_WORK_TREE
US=$'\x1f'   # word separator inside a simple command
DYN=$'\x02'  # marks a word part the shell expands at run time
PLUMB_WHY="commits in the main repo go through git commit and its pre-commit hook (TASK/TOOL42); use a scratch clone for plumbing"
MERGE_WHY="a worktree branch lands in main by git merge --squash and one git commit -F (TASK/TOOL42, CLAUDE.md Commit Discipline)"

# canon <path> -> stdout: repeated slashes squeezed, a trailing slash dropped.
# Both sides of the main-repo comparison go through it: bash keeps a leading
# `//` in `pwd -P` (POSIX leaves `//` implementation-defined), so
# `//Volumes/x/.git` and `/Volumes/x/.git` would differ as strings.
canon() {
    local p
    p=$(printf '%s' "$1" | tr -s /)
    [ "$p" = / ] || p=${p%/}
    printf '%s' "$p"
}

# common_dir <dir> <git dir or empty>: the canonical absolute git common dir,
# or empty.
common_dir() {
    local a b c
    a=$(canon "$1"); b=$(canon "$2")
    c=$(
        cd -P "$a" 2>/dev/null || exit 1
        if [ -n "$b" ]; then
            cd -P "$b" 2>/dev/null || exit 1
            c=$(git --git-dir=. rev-parse --git-common-dir 2>/dev/null)
        else
            c=$(git rev-parse --git-common-dir 2>/dev/null)
        fi
        [ -n "$c" ] && cd -P "$c" 2>/dev/null && pwd -P
    )
    [ -n "$c" ] && canon "$c"
}

main_common=""
main_repo() {
    [ -n "$main_common" ] && return
    main_common=$(common_dir "${CLAUDE_PROJECT_DIR:-$call_cwd}" "")
    [ -n "$main_common" ] || main_common="?"   # unknown: every target counts as main
}

# expand <word> -> E: $HOME, $TMPDIR, $CLAUDE_PROJECT_DIR replaced; any other
# run-time part stays marked with DYN.
# Walks the word with prefix/suffix removal only: bash 3.2 ends the pattern
# of ${w//pat/rep} at the first `/` even inside quotes, so a pattern holding
# a slash (`$NAME/`) replaced the wrong text (TOOL42 tester F1).
expand() {
    local w=$1 out="" rest name tail v
    while :; do
        case "$w" in *"$DYN"*) ;; *) break ;; esac
        out=$out${w%%"$DYN"*}
        rest=${w#*"$DYN"}
        name=""; tail=$rest
        case "$rest" in
            '${'*'}'*) name=${rest#??}; name=${name%%\}*}; tail=${rest#*\}} ;;
            '$'*) tail=${rest#?}; name=${tail%%[!A-Za-z0-9_]*}; tail=${tail#"$name"} ;;
        esac
        case "$name" in
            HOME|TMPDIR|CLAUDE_PROJECT_DIR) v=${!name:-} ;;
            *) v="" ;;
        esac
        if [ -n "$v" ]; then
            out=$out$v
            w=$tail
        else
            # not expandable: keep it marked as run-time
            out=$out$DYN
            w=$rest
            [ "${w:0:1}" = '$' ] && { out=$out'$'; w=${w#?}; }
        fi
    done
    E=$out$w
}

# resolve <base dir> <path> -> R; "?" = cannot be decided from the text.
resolve() {
    local p=$2
    case "$p" in *"$DYN"*) R="?"; return ;; esac
    case "$p" in "~") p=${HOME:-?} ;; "~/"*) p=${HOME:-?}/${p#"~/"} ;; esac
    case "$p" in
        /*) R=$p ;;
        *) if [ "$1" = "?" ]; then R="?"; else R=$1/$p; fi ;;
    esac
}

dir=$call_cwd   # where the next simple command runs
sgd=""          # GIT_DIR exported earlier in the command

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
            elif [ $n1 = 0 ] && [ $n2 = 1 ]; then why="git merge without --squash"; fi ;;
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

# process_seg <words joined by US>: one simple command.
process_seg() {
    local s=$1 w k=0 nw cw a sub tdir gd lgd="" lgd_set=0 wrap=0 tc head
    local -a SW=()
    while [ -n "$s" ]; do
        SW+=("${s%%"$US"*}")
        s=${s#*"$US"}
    done
    nw=${#SW[@]}
    while [ $k -lt $nw ]; do
        w=${SW[k]}
        if [ $wrap = 1 ]; then
            case "$w" in git|*/git|git-*|*/git-*) break ;; esac
            case "$w" in GIT_DIR=*) expand "${w#GIT_DIR=}"; resolve "$dir" "$E"; lgd=$R; lgd_set=1 ;; esac
            k=$((k + 1)); continue
        fi
        case "$w" in
            GIT_DIR=*) expand "${w#GIT_DIR=}"; resolve "$dir" "$E"; lgd=$R; lgd_set=1 ;;
            [A-Za-z_]*=*) case "${w%%=*}" in *[!A-Za-z0-9_]*) break ;; esac ;;
            then|do|else|elif|if|while|until|'!'|'{'|'}') ;;
            time|env|command|exec|nohup|nice|sudo|xargs|builtin) wrap=1 ;;
            *) break ;;
        esac
        k=$((k + 1))
    done
    [ $k -lt $nw ] || return 0
    cw=${SW[k]}
    k=$((k + 1))
    case "$cw" in
        cd|pushd)
            a=""
            while [ $k -lt $nw ]; do
                w=${SW[k]}; k=$((k + 1))
                case "$w" in -L|-P|-e|-@) ;; *) a=$w; break ;; esac
            done
            if [ -z "$a" ]; then dir=${HOME:-?}
            elif [ "$a" = "-" ]; then dir="?"
            else
                expand "$a"; resolve "$dir" "$E"; dir=$R
                [ "$dir" = "?" ] || [ -d "$dir" ] || dir="?"
            fi
            return 0 ;;
        popd) dir="?"; return 0 ;;
        export)
            while [ $k -lt $nw ]; do
                w=${SW[k]}; k=$((k + 1))
                case "$w" in GIT_DIR=*) expand "${w#GIT_DIR=}"; resolve "$dir" "$E"; sgd=$R ;; esac
            done
            return 0 ;;
        unset)
            while [ $k -lt $nw ]; do
                [ "${SW[k]}" = GIT_DIR ] && sgd=""
                k=$((k + 1))
            done
            return 0 ;;
    esac
    tdir=$dir
    gd=$sgd
    [ $lgd_set = 1 ] && gd=$lgd
    sub=""
    case "$cw" in
        git|*/git)
            while [ $k -lt $nw ]; do
                w=${SW[k]}; k=$((k + 1))
                case "$w" in
                    -C)
                        a=${SW[k]:-}; k=$((k + 1))
                        if [ -n "$a" ]; then expand "$a"; resolve "$tdir" "$E"; tdir=$R; fi ;;
                    --git-dir=*) expand "${w#--git-dir=}"; resolve "$tdir" "$E"; gd=$R ;;
                    --git-dir) expand "${SW[k]:-}"; resolve "$tdir" "$E"; gd=$R; k=$((k + 1)) ;;
                    -c|--work-tree|--namespace|--super-prefix|--config-env) k=$((k + 1)) ;;
                    -*) ;;
                    *) sub=$w; break ;;
                esac
            done ;;
        git-*|*/git-*) sub=${cw##*/}; sub=${sub#git-} ;;
        *) return 0 ;;
    esac
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
        if [ "$main_common" = "?" ] || [ "$tc" = "$main_common" ]; then
            block "$why in the main repo is not allowed for agents" "$PLUMB_WHY"
        fi
        return 0
    fi
    # merge/pull: only a decided target that is the main repo with main checked out.
    [ "$tc" != "?" ] && [ "$tc" = "$main_common" ] || return 0
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

# Tokenizer state: cur/have = the word being read, seg = the simple command
# being read, st = N (unquoted), S ('...'), D ("..."); d = nesting depth of
# ( ), $( ) and backticks, with the outer state saved in K_<d>, ...
cur=""; have=0; seg=""; st=N; d=0
HD=()   # pending here-document delimiters, "<strip-tabs 0|1>:<word>"

emit_word() {
    if [ $have = 1 ]; then seg=$seg$cur$US; fi
    cur=""; have=0
}
flush() {
    emit_word
    if [ -n "$seg" ]; then process_seg "$seg"; fi
    seg=""
}
# open <kind p|s|b>: a sub-shell, $( ) or backtick starts.
open() {
    d=$((d + 1))
    printf -v "K_$d" '%s' "$1"
    printf -v "SEG_$d" '%s' "$seg"
    printf -v "CUR_$d" '%s' "$cur"
    printf -v "HAVE_$d" '%s' "$have"
    printf -v "ST_$d" '%s' "$st"
    printf -v "DIR_$d" '%s' "$dir"
    printf -v "SGD_$d" '%s' "$sgd"
    seg=""; cur=""; have=0; st=N
}
close() {
    local k v
    if [ $d = 0 ]; then flush; return; fi
    flush
    k="K_$d"; k=${!k}
    v="SEG_$d"; seg=${!v}
    v="CUR_$d"; cur=${!v}
    v="HAVE_$d"; have=${!v}
    v="ST_$d"; st=${!v}
    v="DIR_$d"; dir=${!v}
    v="SGD_$d"; sgd=${!v}
    d=$((d - 1))
    if [ "$k" != p ]; then cur=$cur$DYN"(sub)"; have=1; fi
}
backtick() {
    local k="K_$d"
    if [ $d -gt 0 ] && [ "${!k}" = b ]; then close; else open b; fi
}
# At a newline (cmd[i]): skip the bodies of pending here-documents.
skip_heredocs() {
    local e strip word rest line start
    [ ${#HD[@]} -gt 0 ] || return 0
    for e in "${HD[@]}"; do
        strip=${e%%:*}; word=${e#*:}
        while [ $i -lt $n ]; do
            start=$((i + 1))
            [ $start -lt $n ] || { i=$n; break; }
            rest=${cmd:start}
            line=${rest%%$'\n'*}
            i=$((start + ${#line}))
            if [ "$strip" = 1 ]; then line=${line#"${line%%[!$'\t']*}"}; fi
            [ "$line" = "$word" ] && break
        done
    done
    HD=()
}
# At "<<" (cmd[i]): read the delimiter word, leave i on its last character.
heredoc_op() {
    local strip=0 word="" ch
    emit_word
    i=$((i + 2))
    if [ "${cmd:i:1}" = "-" ]; then strip=1; i=$((i + 1)); fi
    while [ $i -lt $n ]; do
        ch=${cmd:i:1}
        case "$ch" in ' '|$'\t') i=$((i + 1)) ;; *) break ;; esac
    done
    while [ $i -lt $n ]; do
        ch=${cmd:i:1}
        case "$ch" in
            ' '|$'\t'|$'\n'|';'|'&'|'|'|'<'|'>'|'('|')') break ;;
            "'"|'"'|'\') ;;
            *) word=$word$ch ;;
        esac
        i=$((i + 1))
    done
    HD+=("$strip:$word")
    i=$((i - 1))
}

n=${#cmd}
i=0
while [ $i -lt $n ]; do
    c=${cmd:i:1}
    case $st in
        S)
            if [ "$c" = "'" ]; then st=N; else cur=$cur$c; fi ;;
        D)
            case "$c" in
                '"') st=N ;;
                '\')
                    nx=${cmd:i+1:1}
                    case "$nx" in
                        '$'|'`'|'"'|'\') cur=$cur$nx; i=$((i + 1)) ;;
                        $'\n') i=$((i + 1)) ;;
                        *) cur=$cur$c ;;
                    esac ;;
                '$')
                    if [ "${cmd:i+1:1}" = "(" ]; then i=$((i + 1)); open s; else cur=$cur$DYN\$; fi ;;
                '`') backtick ;;
                *) cur=$cur$c ;;
            esac ;;
        N)
            case "$c" in
                ' '|$'\t') emit_word ;;
                $'\n') flush; skip_heredocs ;;
                ';'|'|') flush ;;
                '&')
                    case "$cur" in
                        *'>'|*'<') cur=$cur$c ;;
                        *) if [ "${cmd:i+1:1}" = ">" ]; then cur=$cur$c; have=1; else flush; fi ;;
                    esac ;;
                "'") st=S; have=1 ;;
                '"') st=D; have=1 ;;
                '\')
                    nx=${cmd:i+1:1}
                    if [ "$nx" != $'\n' ]; then cur=$cur$nx; have=1; fi
                    i=$((i + 1)) ;;
                '$')
                    if [ "${cmd:i+1:1}" = "(" ]; then i=$((i + 1)); open s; else cur=$cur$DYN\$; have=1; fi ;;
                '`') backtick ;;
                '(')
                    case "$cur" in
                        *'<'|*'>') cur=${cur%?}; open s ;;
                        *) flush; open p ;;
                    esac ;;
                ')') close ;;
                '#')
                    if [ $have = 0 ]; then
                        rest=${cmd:i}
                        line=${rest%%$'\n'*}
                        i=$((i + ${#line} - 1))
                    else
                        cur=$cur$c
                    fi ;;
                '<')
                    case "$cur" in
                        *'<') cur=$cur$c ;;
                        *)
                            if [ "${cmd:i+1:1}" = "<" ] && [ "${cmd:i+2:1}" != "<" ]; then
                                heredoc_op
                            else
                                cur=$cur$c; have=1
                            fi ;;
                    esac ;;
                *) cur=$cur$c; have=1 ;;
            esac ;;
    esac
    i=$((i + 1))
done
while [ $d -gt 0 ]; do close; done
flush
exit 0
