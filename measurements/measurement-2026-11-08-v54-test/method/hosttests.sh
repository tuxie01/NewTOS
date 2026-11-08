#!/bin/bash
# host tests: base and chg archive trees; new test file copied into base tree (read-only copy of the chg file)
set -u
S=${S:?}; A=${A:?}; W=/Volumes/Daten/Development/PiRusTOS
cp "$S/t/chg/tos-api/tests/vdi_v54_vsc_form_no_redraw.rs" "$S/t/base/tos-api/tests/"
(cd "$S/t/base" && cargo test -p tos-api --test vdi_v54_vsc_form_no_redraw > "$A/newtest-base.txt" 2>&1; echo "rc $?" >> "$A/newtest-base.txt")
(cd "$S/t/chg" && cargo test -p tos-api --test vdi_v54_vsc_form_no_redraw > "$A/newtest-chg.txt" 2>&1; echo "rc $?" >> "$A/newtest-chg.txt")
rm "$S/t/base/tos-api/tests/vdi_v54_vsc_form_no_redraw.rs"
(cd "$S/t/base" && cargo xtask test > "$A/xtask-test-base.txt" 2>&1; echo "rc $?" >> "$A/xtask-test-base.txt")
(cd "$S/t/chg" && cargo xtask test > "$A/xtask-test-chg.txt" 2>&1; echo "rc $?" >> "$A/xtask-test-chg.txt")
echo HOSTDONE >> "$A/xtask-test-chg.txt"
