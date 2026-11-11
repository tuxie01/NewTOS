Pipe: jq -nc --arg c "$cmd" --arg d "$dir" '{tool_name:"Bash",cwd:$d,tool_input:{command:$c}}' | CLAUDE_PROJECT_DIR=$M <guard>
check1: worktree tools/hooks/no-verify-guard-test.sh (new guard) / same file with base d033a78a2 guard in a scratch dir (old guard)
check2: method/adv.sh <worktree guard>   check3-4: scratch clone (git clone --shared --no-checkout) loops (logs/clone.log, logs/merge.log)
check5: cargo xtask test in worktree; cargo test --no-fail-fast -p tos-api -p tos-types -p xtask; scratch clone of the branch with old guard: cargo test -p xtask no_verify_guard
check6: cargo clippy -p xtask --tests in scratch clone at d033a78a2 and fa053cef5   check7: cargo xtask doc-check
