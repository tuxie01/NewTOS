#!/bin/sh
# TOOL04: the same test binaries, freshly built per [profile.test] variant.
# Each variant builds into its own new target dir under target/tool04-probe/
# (so every binary is new to the OS and its deps/ holds only this build),
# then: build wall time, deps/ entry count, size and first/second launch of
# each binary there, and the first launch of a fresh copy of the same binary
# in the shared target/debug/deps (the big directory).
# Usage: variants.sh <repo-root> <outfile> <label>... ; labels below.
M=$(cd "$(dirname "$0")" && pwd); R=$1; OUT=$2; shift 2
TESTS="aes_t04_intin_long_contract bios_reset vdi_reference"
for label in "$@"; do
  case $label in
    full)        envs="" ;;
    lto)         envs="CARGO_PROFILE_TEST_DEBUG=line-tables-only" ;;
    strip)       envs="CARGO_PROFILE_TEST_STRIP=symbols" ;;
    nodebug)     envs="CARGO_PROFILE_TEST_DEBUG=false" ;;
    lto-packed)  envs="CARGO_PROFILE_TEST_DEBUG=line-tables-only CARGO_PROFILE_TEST_SPLIT_DEBUGINFO=packed" ;;
    full-packed) envs="CARGO_PROFILE_TEST_SPLIT_DEBUGINFO=packed" ;;
    *) echo "unknown variant $label" >&2; exit 2 ;;
  esac
  T=$R/target/tool04-probe/tgt-$label
  sel=""; for t in $TESTS; do sel="$sel --test $t"; done
  t0=$(python3 -I -c 'import time; print(time.time())')
  # shellcheck disable=SC2086
  (cd "$R" && env CARGO_TARGET_DIR="$T" $envs cargo test --no-run -p tos-api $sel \
      --message-format=json > "$T.json" 2> "$T.err")
  bt=$(python3 -I -c "import time; print('%.1f' % (time.time() - $t0))")
  n=$(ls -f "$T/debug/deps" | wc -l | tr -d ' ')
  no=$(ls -f "$T/debug/deps" | /usr/bin/grep -c '\.o$')
  echo "== $label [$envs] build ${bt}s deps_entries=$n deps_o=$no" | tee -a "$OUT"
  python3 -I "$M/exes-from-json.py" "$T.json" | while IFS="$(printf '\t')" read -r kind name exe pdir fresh; do
    sz=$(stat -f %z "$exe")
    a=$(python3 -I "$M/t.py" first "$exe" --list | cut -f2)
    b=$(python3 -I "$M/t.py" second "$exe" --list | cut -f2)
    cp "$exe" "$R/target/debug/deps/zz-tool04-$label-$name"
    c=$(python3 -I "$M/t.py" bigdir "$R/target/debug/deps/zz-tool04-$label-$name" --list | cut -f2)
    rm "$R/target/debug/deps/zz-tool04-$label-$name"
    printf '%s\t%s\tsize=%s\tfirst=%s\tsecond=%s\tcopy-in-shared-deps-first=%s\n' "$label" "$name" "$sz" "$a" "$b" "$c" | tee -a "$OUT"
  done
done
