# shell-parse.sh -- the command-text parser shared by the agent guards
# (sourced, not run). Written for TASK/TOOL42 in no-verify-guard.sh, moved
# here by TASK/TOOL50 so tools/hooks/pin-guard.sh uses the same parser.
#
# parse_command splits $cmd into simple commands, quote-aware (', ", \), at
# ; & | && || newline, ( ) and $( ) / backticks (inner commands are seen
# too), skipping comments and here-document bodies. For each simple command
# process_seg
#   - calls on_seg (the words in SW, before cd/export take effect);
#   - skips assignments, the keywords then/do/if/..., and the wrappers
#     env/command/exec/nohup/nice/sudo/time/xargs/builtin (after a wrapper,
#     every word up to the first one `is_cmd_word` accepts);
#   - tracks cd/pushd/popd (dir), `export GIT_DIR=`/`GIT_WORK_TREE=`
#     (sgd/sgwt) and `unset`;
#   - GIT_CONFIG/GIT_CONFIG_GLOBAL/GIT_CONFIG_SYSTEM as prefix or export
#     (gc: the config file named last, text in gct; TOOL50);
#   - for git, reads -C, --git-dir, --work-tree and GIT_DIR=/GIT_WORK_TREE=
#     prefixes (tdir, gd, gwt) and the subcommand (sub; also `git-<sub>`);
#   - calls on_command with SW, cw (the command word), k (index of the first
#     argument after cw, or after sub for git), sub, tdir, gd, gwt.
# A guard defines on_seg, on_command and is_cmd_word after sourcing this file.
#
# Word marks: DYN before a part the shell expands at run time (`$`, $( ),
# backticks); RD before every unquoted `>`, so a guard can tell a redirection
# from a quoted ">" argument. A directory or path the text cannot decide is
# "?"; its text (expanded, base-joined) is kept in the companion variable:
# dir -> dtxt, tdir -> tdirt, gd -> gdt, gwt -> gwtt.
#
# Callers set before parse_command: cmd, call_cwd. Globals it uses: n, i, c,
# nx, rest, line, cur, have, seg, st, d, HD, dir, dtxt, sgd, sgdt, sgwt, sgwtt,
# and K_<d>, SEG_<d>, ... for the nesting levels.

US=$'\x1f'   # word separator inside a simple command
DYN=$'\x02'  # marks a word part the shell expands at run time
RD=$'\x03'   # marks an unquoted `>` (a redirection)

# Defaults; a guard overrides them after sourcing.
on_seg() { :; }
on_command() { :; }
is_cmd_word() { case "$1" in git|*/git|git-*|*/git-*) return 0 ;; esac; return 1; }

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
# RT: the path's text joined to the base (or to the base's text <3>, when
# the base is "?"), for a guard that matches on text when R is "?".
resolve() {
    local p=$2 bt=${3:-?}
    case "$p" in "~") p=${HOME:-?} ;; "~/"*) p=${HOME:-?}/${p#"~/"} ;; esac
    case "$p" in
        /*) RT=$p ;;
        *) if [ "$1" = "?" ]; then RT=$bt/$p; else RT=$1/$p; fi ;;
    esac
    case "$p" in *"$DYN"*) R="?"; return ;; esac
    case "$p" in
        /*) R=$p ;;
        *) if [ "$1" = "?" ]; then R="?"; else R=$1/$p; fi ;;
    esac
}

dir=$call_cwd   # where the next simple command runs
dtxt=""         # the text of dir when dir is "?"
sgd=""; sgdt="" # GIT_DIR exported earlier in the command
sgwt=""; sgwtt="" # GIT_WORK_TREE exported earlier in the command
sgc=""; sgct=""   # GIT_CONFIG* exported earlier in the command

# process_seg <words joined by US>: one simple command.
process_seg() {
    local s=$1 w k=0 nw cw a sub tdir tdirt gd gdt gwt gwtt gc gct
    local lgd="" lgdt="" lgd_set=0 lgwt="" lgwtt="" lgwt_set=0 lgc="" lgct="" lgc_set=0 wrap=0
    local -a SW=()
    while [ -n "$s" ]; do
        SW+=("${s%%"$US"*}")
        s=${s#*"$US"}
    done
    nw=${#SW[@]}
    on_seg
    while [ $k -lt $nw ]; do
        w=${SW[k]}
        if [ $wrap = 1 ]; then
            is_cmd_word "$w" && break
            case "$w" in
                GIT_DIR=*) expand "${w#GIT_DIR=}"; resolve "$dir" "$E" "$dtxt"; lgd=$R; lgdt=$RT; lgd_set=1 ;;
                GIT_WORK_TREE=*) expand "${w#GIT_WORK_TREE=}"; resolve "$dir" "$E" "$dtxt"; lgwt=$R; lgwtt=$RT; lgwt_set=1 ;;
                GIT_CONFIG=*|GIT_CONFIG_GLOBAL=*|GIT_CONFIG_SYSTEM=*) expand "${w#*=}"; resolve "$dir" "$E" "$dtxt"; lgc=$R; lgct=$RT; lgc_set=1 ;;
            esac
            k=$((k + 1)); continue
        fi
        case "$w" in
            GIT_DIR=*) expand "${w#GIT_DIR=}"; resolve "$dir" "$E" "$dtxt"; lgd=$R; lgdt=$RT; lgd_set=1 ;;
            GIT_WORK_TREE=*) expand "${w#GIT_WORK_TREE=}"; resolve "$dir" "$E" "$dtxt"; lgwt=$R; lgwtt=$RT; lgwt_set=1 ;;
            GIT_CONFIG=*|GIT_CONFIG_GLOBAL=*|GIT_CONFIG_SYSTEM=*) expand "${w#*=}"; resolve "$dir" "$E" "$dtxt"; lgc=$R; lgct=$RT; lgc_set=1 ;;
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
            if [ -z "$a" ]; then dir=${HOME:-?}; dtxt=$dir
            elif [ "$a" = "-" ]; then dir="?"; dtxt="?"
            else
                expand "$a"; resolve "$dir" "$E" "$dtxt"; dir=$R; dtxt=$RT
                [ "$dir" = "?" ] || [ -d "$dir" ] || dir="?"
            fi
            return 0 ;;
        popd) dir="?"; dtxt="?"; return 0 ;;
        export)
            while [ $k -lt $nw ]; do
                w=${SW[k]}; k=$((k + 1))
                case "$w" in
                    GIT_DIR=*) expand "${w#GIT_DIR=}"; resolve "$dir" "$E" "$dtxt"; sgd=$R; sgdt=$RT ;;
                    GIT_WORK_TREE=*) expand "${w#GIT_WORK_TREE=}"; resolve "$dir" "$E" "$dtxt"; sgwt=$R; sgwtt=$RT ;;
                    GIT_CONFIG=*|GIT_CONFIG_GLOBAL=*|GIT_CONFIG_SYSTEM=*) expand "${w#*=}"; resolve "$dir" "$E" "$dtxt"; sgc=$R; sgct=$RT ;;
                esac
            done
            return 0 ;;
        unset)
            while [ $k -lt $nw ]; do
                [ "${SW[k]}" = GIT_DIR ] && sgd=""
                [ "${SW[k]}" = GIT_WORK_TREE ] && sgwt=""
                case "${SW[k]}" in GIT_CONFIG|GIT_CONFIG_GLOBAL|GIT_CONFIG_SYSTEM) sgc="" ;; esac
                k=$((k + 1))
            done
            return 0 ;;
    esac
    tdir=$dir; tdirt=$dtxt
    gd=$sgd; gdt=$sgdt
    [ $lgd_set = 1 ] && { gd=$lgd; gdt=$lgdt; }
    gwt=$sgwt; gwtt=$sgwtt
    [ $lgwt_set = 1 ] && { gwt=$lgwt; gwtt=$lgwtt; }
    gc=$sgc; gct=$sgct
    [ $lgc_set = 1 ] && { gc=$lgc; gct=$lgct; }
    sub=""
    case "$cw" in
        git|*/git)
            while [ $k -lt $nw ]; do
                w=${SW[k]}; k=$((k + 1))
                case "$w" in
                    -C)
                        a=${SW[k]:-}; k=$((k + 1))
                        if [ -n "$a" ]; then expand "$a"; resolve "$tdir" "$E" "$tdirt"; tdir=$R; tdirt=$RT; fi ;;
                    --git-dir=*) expand "${w#--git-dir=}"; resolve "$tdir" "$E" "$tdirt"; gd=$R; gdt=$RT ;;
                    --git-dir) expand "${SW[k]:-}"; resolve "$tdir" "$E" "$tdirt"; gd=$R; gdt=$RT; k=$((k + 1)) ;;
                    --work-tree=*) expand "${w#--work-tree=}"; resolve "$tdir" "$E" "$tdirt"; gwt=$R; gwtt=$RT ;;
                    --work-tree) expand "${SW[k]:-}"; resolve "$tdir" "$E" "$tdirt"; gwt=$R; gwtt=$RT; k=$((k + 1)) ;;
                    -c|--namespace|--super-prefix|--config-env) k=$((k + 1)) ;;
                    -*) ;;
                    *) sub=$w; break ;;
                esac
            done ;;
        git-*|*/git-*) sub=${cw##*/}; sub=${sub#git-} ;;
    esac
    on_command
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
    printf -v "DTXT_$d" '%s' "$dtxt"
    printf -v "SGD_$d" '%s' "$sgd"
    printf -v "SGDT_$d" '%s' "$sgdt"
    printf -v "SGWT_$d" '%s' "$sgwt"
    printf -v "SGWTT_$d" '%s' "$sgwtt"
    printf -v "SGC_$d" '%s' "$sgc"
    printf -v "SGCT_$d" '%s' "$sgct"
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
    v="DTXT_$d"; dtxt=${!v}
    v="SGD_$d"; sgd=${!v}
    v="SGDT_$d"; sgdt=${!v}
    v="SGWT_$d"; sgwt=${!v}
    v="SGWTT_$d"; sgwtt=${!v}
    v="SGC_$d"; sgc=${!v}
    v="SGCT_$d"; sgct=${!v}
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

# parse_command: runs process_seg on every simple command of $cmd.
parse_command() {
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
                            *'<'|*'>') cur=${cur%?}; cur=${cur%"$RD"}; open s ;;
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
                    '>') cur=$cur$RD$c; have=1 ;;
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
}
