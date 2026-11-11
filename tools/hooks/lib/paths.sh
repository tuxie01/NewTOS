# paths.sh -- the path comparison shared by the agent guards (pin-guard.sh,
# no-verify-guard.sh, scratch-guard.sh) and tools/oracle/build-emutos.sh
# (TASK/TOOL50). Sourced, not run; POSIX sh (build-emutos.sh is #!/bin/sh).
#
# APFS and HFS+ volumes are case-insensitive by default: references/EMUTOS,
# References/emutos and /TMP name the same directory as references/emutos
# and /tmp, and `cd -P` + `pwd -P` may keep the typed case. So every
# comparison here folds ASCII case (tools/hooks; owner 2026-11-11, TOOL50
# tester finding). On a case-sensitive volume this over-matches two
# directories that differ only in case -- the safe side for a guard. Not
# folded: non-ASCII letters (APFS folds Unicode too).
#
# Functions (results in globals; names start with path_ / _p):
#   path_canon <p>         stdout: repeated slashes squeezed, trailing slash dropped
#   path_lower <p>         stdout: ASCII lower case
#   path_physical <p>      PH: absolute <p> with its nearest existing directory
#                          resolved by `cd -P` (so symlinks and `..` after a
#                          symlink are the kernel's), the rest appended and
#                          `.`/`..` removed lexically
#   path_same <a> <b>      0 when equal, case folded
#   path_in <p> <root>     0 when <p> is <root> or lies below it, case folded
#   path_contains <p> <r>  0 when <p> is / or a directory above <r>, case folded

path_canon() {
    _pc=$(printf '%s' "$1" | tr -s /)
    [ "$_pc" = / ] || _pc=${_pc%/}
    printf '%s' "$_pc"
}

path_lower() {
    printf '%s' "$1" | tr 'ABCDEFGHIJKLMNOPQRSTUVWXYZ' 'abcdefghijklmnopqrstuvwxyz'
}

# path_lexnorm <absolute path> -> LN
path_lexnorm() {
    _ls=${1#/}; _lo=
    while [ -n "$_ls" ]; do
        case "$_ls" in
            */*) _lc=${_ls%%/*}; _ls=${_ls#*/} ;;
            *) _lc=$_ls; _ls= ;;
        esac
        case "$_lc" in
            ''|.) ;;
            ..) _lo=${_lo%/*} ;;
            *) _lo=$_lo/$_lc ;;
        esac
    done
    LN=${_lo:-/}
}

path_physical() {
    _pp=$(path_canon "$1"); _pr=
    while [ "$_pp" != / ] && [ -n "$_pp" ] && [ ! -d "$_pp" ]; do
        _pr=/${_pp##*/}$_pr
        _pp=${_pp%/*}
    done
    [ -n "$_pp" ] || _pp=/
    _pq=$(cd -P "$_pp" 2>/dev/null && pwd -P) || _pq=$_pp
    path_lexnorm "$_pq$_pr"
    PH=$(path_canon "$LN")
}

path_same() {
    [ "$(path_lower "$1")" = "$(path_lower "$2")" ]
}

path_in() {
    _pa=$(path_lower "$1"); _pb=$(path_lower "$2")
    [ "$_pa" = "$_pb" ] && return 0
    case "$_pa" in "$_pb"/*) return 0 ;; esac
    return 1
}

path_contains() {
    [ "$1" = / ] && return 0
    _pa=$(path_lower "$1"); _pb=$(path_lower "$2")
    case "$_pb" in "$_pa"/*) return 0 ;; esac
    return 1
}
