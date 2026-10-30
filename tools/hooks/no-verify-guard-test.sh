#!/usr/bin/env bash
# Pipe-tests for tools/hooks/no-verify-guard.sh. Run from the repo root.
G=tools/hooks/no-verify-guard.sh
fail=0
t() {
    jq -nc --arg c "$1" '{tool_name:"Bash",tool_input:{command:$c}}' | CLAUDE_PROJECT_DIR=$PWD $G >/dev/null 2>&1
    rc=$?
    if [ "$rc" = "$2" ]; then r=ok; else r=FAIL; fail=1; fi
    echo "$r exit $rc (want $2): $3"
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
exit $fail
