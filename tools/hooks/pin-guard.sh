#!/usr/bin/env bash
#
# pin-guard.sh -- PreToolUse hook (.claude/settings.json, Bash|Write|Edit|
# NotebookEdit) for the EmuTOS checkouts on the pin 1a7484ab (TASK/TOOL50;
# CLAUDE.md "The EmuTOS checkouts stay on the pin").
#
# Owner decision 2026-11-11 (TOOL50 rework): no pattern race. The pin is
# protected at OS and git level -- the owner makes the trees read-only
# (chmod -R a-w, chflags -R uchg) and gives them an invalid remote URL -- so
# a write, fetch, checkout, make or rm there fails by itself. This guard
# only keeps an agent from undoing that lock. It blocks with exit 2
# (message on stderr, shown to the agent):
#   chmod, chflags, chown on a path in a protected tree, or on a directory
#     that contains one (`chmod -R u+w references`); also as the command of
#     `find <path> -exec/-execdir/-ok/-okdir` with such a start path;
#   git remote add|set-url|remove|rm|rename, and a `git config` write of a
#     key that redirects a fetch -- remote.*, url.* (insteadOf,
#     pushInsteadOf), branch.<b>.remote / pushRemote -- (set, --add,
#     --replace-all, --unset*, set/unset subcommands, --rename-section/
#     --remove-section, -e), in a protected repo (git -C, --git-dir,
#     --work-tree, GIT_DIR=/GIT_WORK_TREE= prefix or export, cd/pushd, the
#     call's cwd), or with its config file in one (--file, -f, -f<path>, a
#     GIT_CONFIG / GIT_CONFIG_GLOBAL / GIT_CONFIG_SYSTEM prefix or export);
#   Write/Edit/NotebookEdit whose file path lies in a protected tree (kept:
#     exact, the path comes as one string, and cheap).
#
# Protected trees: references/emutos and references/emutos-oracle (every
# tree in it) of the main repo (parent of the git common dir of
# $CLAUDE_PROJECT_DIR) and of $CLAUDE_PROJECT_DIR. Paths are compared by
# tools/hooks/lib/paths.sh: physical (`cd -P` of the nearest existing
# directory: a worktree's references/ symlink (TOOL44), any other symlinked
# directory and `..` after a symlink count) and ASCII case folded (APFS is
# case-insensitive: references/EMUTOS is the same tree). A path the text
# cannot decide (a variable other than $HOME/$TMPDIR/$CLAUDE_PROJECT_DIR)
# counts when its text names .../references/emutos[-oracle]/... (case
# folded); a glob or brace word when its literal part before * ? [ { can
# reach a protected tree. The command text is parsed by the shared parser
# (tools/hooks/lib/shell-parse.sh), as in no-verify-guard.sh.
#
# Known limits (TOOL50 tester runs measurement-2026-11-11-tool50-test and
# -retest, untracked; not chased by owner decision): a text match cannot be
# complete. Not seen: a command assembled at run time (`eval`, `bash -c`, a
# script, an alias, a command word in a variable, paths from stdin as in
# `xargs chmod`); scripts that change modes themselves (`python3 -c
# 'os.chmod(...)'`, `perl -e 'chmod ...'`); other lock-relevant commands on
# the trees (chgrp, setfacl, xattr); a wrapper inside `find -exec`
# (`-exec sh -c 'chmod ...'`, `-exec env chmod`); wrappers the parser does
# not know (timeout, gtimeout, caffeinate, xcrun, /usr/bin/env, `command
# cd`, `cd --`); `env -C <dir>` (env is a known wrapper, but its directory
# change is not tracked: a relative path after it resolves against the
# previous directory); a path spelled with $'...' escapes (only the literal
# text of the word is matched, e.g. $'\x72eferences' is not); a symlink made
# earlier in the same command line (resolved before it exists); other git
# config keys (core.*, include.path, ...). Not guarded either (owner
# 2026-11-11): url.*.insteadOf / url.*.pushInsteadOf in the GLOBAL or
# SYSTEM git config (~/.gitconfig, /etc/gitconfig, $XDG_CONFIG_HOME/git/
# config, or a file named by GIT_CONFIG_GLOBAL/GIT_CONFIG_SYSTEM) can
# redirect the trees' pinned:/ URLs; writes to the global or system config
# are not blocked, the backstop is the OS lock (a fetch cannot write .git in
# the locked trees). Only a GIT_CONFIG* file that lies in a protected tree
# is checked, as above. Every other write (rm, mv, cp,
# redirection incl. >| >! &>|, sed -i/-Ei, awk -i, sort -o, curl/wget -o,
# vim, scp, find -delete, make, tar, git fetch/pull/checkout/push into a
# protected repo, ...) is left to the OS lock by owner decision. Backstops:
# the OS-level read-only lock (applied 2026-11-11,
# measurement-2026-11-11-tool50-pinlock) and the server-side check of
# TASK/TOOL43.

set -u
export LC_ALL=C

input=$(cat)
tool=$(printf '%s' "$input" | jq -r '.tool_name // empty')
case "$tool" in Bash|Write|Edit|NotebookEdit) ;; *) exit 0 ;; esac
call_cwd=$(printf '%s' "$input" | jq -r '.cwd // empty')
[ -n "$call_cwd" ] || call_cwd=$PWD

PIN_WHY="references/emutos and references/emutos-oracle/* stay on the pin 1a7484ab and locked read-only (CLAUDE.md; TASK/TOOL50); read them, and build or experiment on a copy (git clone references/emutos <your target-agents dir>/emutos)"
block() {
    echo "pin-guard: $1" >&2
    echo "$PIN_WHY" >&2
    exit 2
}

LIB=$(dirname "${BASH_SOURCE[0]}")/lib
. "$LIB/paths.sh" || block "tools/hooks/lib/paths.sh is missing"
. "$LIB/shell-parse.sh" || block "tools/hooks/lib/shell-parse.sh is missing"
unset GIT_DIR GIT_COMMON_DIR GIT_WORK_TREE

ROOTS=()
add_root() {
    local r
    path_physical "$1"
    for r in "${ROOTS[@]+"${ROOTS[@]}"}"; do path_same "$r" "$PH" && return 0; done
    ROOTS+=("$PH")
}
proj=${CLAUDE_PROJECT_DIR:-}
[ -n "$proj" ] || proj=$(git -C "$call_cwd" rev-parse --show-toplevel 2>/dev/null || true)
if [ -n "$proj" ]; then
    mc=$(common_dir "$proj" "")
    for b in "${mc%/.git}" "$proj"; do
        [ -n "$b" ] || continue
        case "$b" in /*) ;; *) continue ;; esac
        add_root "$b/references/emutos"
        add_root "$b/references/emutos-oracle"
    done
fi
[ ${#ROOTS[@]} -gt 0 ] || exit 0

# hit_root <physical path or glob> <in|del> -> 0 when it lies in a protected
# tree (del: or contains one); HIT = that tree.
hit_root() {
    local p=$1 r pre
    case "$p" in
        *[\*\?\[\{]*)
            pre=$(path_lower "${p%%[\*\?\[\{]*}")
            for r in "${ROOTS[@]}"; do
                case "$(path_lower "$r")/" in "$pre"*) HIT=$r; return 0 ;; esac
                path_in "${pre%/}" "$r" && { HIT=$r; return 0; }
            done
            return 1 ;;
    esac
    for r in "${ROOTS[@]}"; do
        path_in "$p" "$r" && { HIT=$r; return 0; }
        if [ "$2" = del ] && path_contains "$p" "$r"; then HIT=$r; return 0; fi
    done
    return 1
}

# text_prot <text>: the text of an undecidable path names a protected tree.
text_prot() {
    case "/$(path_lower "$1")/" in
        */references/emutos/*|*/references/emutos-oracle/*) HIT="references/${1##*[Rr][Ee][Ff][Ee][Rr][Ee][Nn][Cc][Ee][Ss]/}"; return 0 ;;
    esac
    return 1
}

# check_path <base dir> <base text> <word> <in|del> -> 0 when protected.
check_path() {
    local w=$3
    w=${w//"$RD"/}
    [ -n "$w" ] || return 1
    expand "$w"
    resolve "$1" "$E" "$2"
    if [ "$R" = "?" ]; then text_prot "$RT"; return; fi
    path_physical "$R"
    hit_root "$PH" "$4"
}

# prot_dir <resolved dir or ?> <its text> -> 0 when protected.
prot_dir() {
    if [ "$1" = "?" ]; then text_prot "$2"; return; fi
    path_physical "$1"
    hit_root "$PH" in
}

# show <word>: the word without parser marks, for messages.
show() {
    local x=${1//"$RD"/}
    printf '%s' "${x//"$DYN"/}"
}

# ── Write / Edit ────────────────────────────────────────────────────────
case "$tool" in
    Write|Edit|NotebookEdit)
        fp=$(printf '%s' "$input" | jq -r '.tool_input.file_path // .tool_input.notebook_path // empty')
        [ -n "$fp" ] || exit 0
        case "$fp" in /*) ;; *) fp=$call_cwd/$fp ;; esac
        path_physical "$fp"
        hit_root "$PH" in && block "$tool of $fp: it lies in the protected tree $HIT"
        exit 0 ;;
esac

# ── Bash ────────────────────────────────────────────────────────────────
cmd=$(printf '%s' "$input" | jq -r '.tool_input.command // empty')
[ -n "$cmd" ] || exit 0
# Cheap filter: only these command words matter.
printf '%s' "$cmd" | grep -qE 'chmod|chflags|chown|git' || exit 0

LOCKERS=" chmod chflags chown "
is_cmd_word() {
    local b=${1##*/}
    case "$b" in git|git-*) return 0 ;; esac
    case "$LOCKERS" in *" $b "*) return 0 ;; esac
    [ "$b" = find ]
}

# redir_skip <word>: 0 when the word is a redirection; RT_SKIP=1 when its
# target is the next word.
redir_skip() {
    local t
    RT_SKIP=0
    case "$1" in *"$RD>"*) ;; *) return 1 ;; esac
    t=${1#*"$RD>"}
    t=${t#"$RD>"}
    [ -n "$t" ] || RT_SKIP=1
    return 0
}

# collect_pos <first index> -> POS: the arguments that are neither options
# (before `--`) nor redirections.
collect_pos() {
    local j=$1 nw=${#SW[@]} w dd=0
    POS=()
    while [ $j -lt $nw ]; do
        w=${SW[j]}; j=$((j + 1))
        if redir_skip "$w"; then
            [ $RT_SKIP = 1 ] && j=$((j + 1))
            continue
        fi
        if [ $dd = 0 ]; then
            case "$w" in
                --) dd=1; continue ;;
                -) ;;
                -*) continue ;;
            esac
        fi
        POS+=("$w")
    done
}

on_command() {
    local b=${cw##*/} w
    case "$b" in
        git|git-*) pin_git ;;
        chmod|chflags|chown)
            collect_pos $k
            for w in "${POS[@]+"${POS[@]}"}"; do
                check_path "$dir" "$dtxt" "$w" del && block "$b $(show "$w"): it would change the lock of the protected tree $HIT"
            done ;;
        find) pin_find ;;
    esac
    return 0
}

# find <start paths> ... -exec chmod|chflags|chown ...
pin_find() {
    local j=$k nw=${#SW[@]} w bad=""
    local -a S=()
    while [ $j -lt $nw ]; do
        w=${SW[j]}
        case "$w" in -H|-L|-P|-E|-X|-s|-x|-d) j=$((j + 1)); continue ;; -*|'('|'!') break ;; esac
        S+=("$w"); j=$((j + 1))
    done
    [ ${#S[@]} -gt 0 ] || S=(.)
    while [ $j -lt $nw ]; do
        w=${SW[j]}; j=$((j + 1))
        case "$w" in
            -exec|-execdir|-ok|-okdir)
                case "$LOCKERS" in *" ${SW[j]##*/} "*) bad="$w ${SW[j]}" ;; esac ;;
        esac
    done
    [ -n "$bad" ] || return 0
    for w in "${S[@]}"; do
        check_path "$dir" "$dtxt" "$w" del && block "find $(show "$w") $bad: it would change the lock of the protected tree $HIT"
    done
    return 0
}

# config_remote_write <args...>: 0 when the git config call writes a key or
# section that redirects a fetch: remote.*, url.* (insteadOf, pushInsteadOf),
# branch.<b>.remote / branch.<b>.pushRemote.
config_remote_write() {
    local a skip=0 first="" key="" wr=0 rd=0 np=0 sect=0
    for a in "$@"; do
        if [ $skip = 1 ]; then skip=0; continue; fi
        if redir_skip "$a"; then skip=$RT_SKIP; continue; fi
        case "$a" in
            -f|--file|--blob|--type|--default|--comment|--value) skip=1 ;;
            --get*|-l|--list) rd=1 ;;
            --unset*|--add|--replace-all) wr=1 ;;
            --rename-section|--remove-section) wr=1; sect=1 ;;
            -e|--edit) wr=1; key=remote. ;;
            -*) ;;
            *)
                np=$((np + 1))
                if [ -z "$first" ]; then first=$a
                    case "$a" in get|list) rd=1 ;; set|unset|rename-section|remove-section|edit) wr=1; continue ;; esac
                fi
                [ -n "$key" ] || key=$a ;;
        esac
    done
    case "$first" in get|list|set|unset|rename-section|remove-section|edit) np=$((np - 1)) ;; esac
    case "$(path_lower "$key")" in remote|remote.*|url|url.*|branch.*.remote|branch.*.pushremote) ;; *) return 1 ;; esac
    [ $rd = 1 ] && [ $wr = 0 ] && return 1
    [ $wr = 1 ] || [ $np -ge 2 ] || [ $sect = 1 ]
}

pin_git() {
    [ -n "$sub" ] || return 0
    local tprot=0 tp="" w
    case "$sub" in
        remote)
            collect_pos $k
            case "${POS[0]:-}" in add|set-url|remove|rm|rename) ;; *) return 0 ;; esac
            w="git remote ${POS[0]}" ;;
        config)
            config_remote_write "${SW[@]:k}" || return 0
            w="git config (a remote.*/url.*/branch.*.remote write)"
            local j=$k nw=${#SW[@]} f=""
            while [ $j -lt $nw ]; do
                case "${SW[j]}" in
                    -f|--file) f=${SW[j+1]:-}; j=$((j + 1)) ;;
                    --file=*) f=${SW[j]#--file=} ;;
                    -f?*) f=${SW[j]#-f} ;;
                esac
                j=$((j + 1))
            done
            if [ -n "$f" ] && check_path "$tdir" "$tdirt" "$f" in; then
                block "$w with --file $(show "$f"): it lies in the protected tree $HIT"
            fi
            # GIT_CONFIG (also _GLOBAL/_SYSTEM) names the file git config writes.
            if [ -n "$gc" ] && prot_dir "$gc" "$gct"; then
                block "$w with GIT_CONFIG $(show "$gct"): it lies in the protected tree $HIT"
            fi ;;
        *) return 0 ;;
    esac
    if [ -n "$gd" ] && prot_dir "$gd" "$gdt"; then tprot=1; tp=$HIT; fi
    if [ -n "$gwt" ] && prot_dir "$gwt" "$gwtt"; then tprot=1; tp=$HIT; fi
    if [ -z "$gd" ] && prot_dir "$tdir" "$tdirt"; then tprot=1; tp=$HIT; fi
    [ $tprot = 1 ] && block "$w in the protected tree $tp: its remote stays invalid (the pin lock)"
    return 0
}

parse_command
exit 0
