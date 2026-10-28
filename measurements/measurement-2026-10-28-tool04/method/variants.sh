#!/bin/sh
# TOOL04: the same test binaries, freshly built per [profile.test] variant,
# each into its own new CARGO_TARGET_DIR outside the repo (scratch), so every
# binary is new to the OS and that deps/ holds only this build.
# Per variant: build wall time (cold: the whole dependency graph of the
# selected tests), deps/ entry and .o count, deps/ size; per binary: size,
# first/second launch at its build path, and the first launch of a fresh copy
# of it in the shared target/debug/deps (the big directory, ~500k entries).
# Derived from measurement-2026-10-27-tool04/method/variants.sh (never run);
# changed: target dirs in scratch ($3), split-debuginfo=off variant added.
# Usage: variants.sh <repo-root> <outfile> <scratch-dir> <label>...
M=$(cd "$(dirname "$0")" && pwd); R=$1; OUT=$2; SCR=$3; shift 3
TESTS="aes_t04_intin_long_contract bios_reset vdi_reference"
for label in "$@"; do
  case $label in
    full)        envs="" ;;
    lto)         envs="CARGO_PROFILE_TEST_DEBUG=line-tables-only" ;;
    strip)       envs="CARGO_PROFILE_TEST_STRIP=symbols" ;;
    nodebug)     envs="CARGO_PROFILE_TEST_DEBUG=false" ;;
    lto-packed)  envs="CARGO_PROFILE_TEST_DEBUG=line-tables-only CARGO_PROFILE_TEST_SPLIT_DEBUGINFO=packed" ;;
    full-packed) envs="CARGO_PROFILE_TEST_SPLIT_DEBUGINFO=packed" ;;
    full-off)    envs="CARGO_PROFILE_TEST_SPLIT_DEBUGINFO=off" ;;
    *) echo "unknown variant $label" >&2; exit 2 ;;
  esac
  T=$SCR/tgt-$label
  rm -rf "$T" "$T.json" "$T.err"
  sel=""; for t in $TESTS; do sel="$sel --test $t"; done
  t0=$(python3 -I -c 'import time; print(time.time())')
  # shellcheck disable=SC2086
  (cd "$R" && env CARGO_TARGET_DIR="$T" $envs cargo test --no-run -p tos-api $sel \
      --message-format=json > "$T.json" 2> "$T.err")
  bt=$(python3 -I -c "import time; print('%.1f' % (time.time() - $t0))")
  n=$(ls -f "$T/debug/deps" | wc -l | tr -d ' ')
  no=$(ls -f "$T/debug/deps" | /usr/bin/grep -c '\.o$')
  nd=$(ls -f "$T/debug/deps" | /usr/bin/grep -c '\.dSYM$')
  du=$(du -sk "$T/debug/deps" | cut -f1)
  echo "== $label [$envs] build ${bt}s deps_entries=$n deps_o=$no deps_dsym=$nd deps_kib=$du" | tee -a "$OUT"
  python3 -I "$M/exes-from-json.py" "$T.json" | while IFS="$(printf '\t')" read -r kind name exe pdir fresh; do
    sz=$(stat -f %z "$exe")
    # debug map: object files the binary points lldb/backtraces at, and how
    # many of them still exist; plus a .dSYM next to it (packed)
    oso=$(nm -ap "$exe" | /usr/bin/grep ' OSO ' | awk '{print $NF}')
    no_oso=$(printf '%s\n' "$oso" | /usr/bin/grep -c .)
    ex_oso=0; for f in $oso; do [ -e "$f" ] && ex_oso=$((ex_oso + 1)); done
    dsym=no; [ -d "$exe.dSYM" ] && dsym=yes
    a=$(python3 -I "$M/t.py" first "$exe" --list | cut -f2)
    b=$(python3 -I "$M/t.py" second "$exe" --list | cut -f2)
    cp "$exe" "$R/target/debug/deps/zz-tool04-$label-$name"
    c=$(python3 -I "$M/t.py" bigdir "$R/target/debug/deps/zz-tool04-$label-$name" --list | cut -f2)
    rm "$R/target/debug/deps/zz-tool04-$label-$name"
    printf '%s\t%s\t%s\tsize=%s\tfirst=%s\tsecond=%s\tcopy-in-shared-deps-first=%s\toso=%s\toso_present=%s\tdsym=%s\n' \
      "$label" "$kind" "$name" "$sz" "$a" "$b" "$c" "$no_oso" "$ex_oso" "$dsym" | tee -a "$OUT"
  done
done
