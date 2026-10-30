#!/usr/bin/env bash
#
# sd-manifest.sh -- the SD image manifest writer, shared by every tool that
# builds a card image a measurement run boots (TASK/TOOL02):
# tools/make-sd-image.sh (the default QEMU image) and tools/mkcard (the
# per-run cards of the injector runs).
#
# Usage (two steps, the second after the image is detached):
#
#   tools/sd-manifest.sh files <mount-point> <drive> <image>
#       Append one `file <sha256> <bytes> <drive>:\<path>` line per file under
#       the mounted partition <mount-point> to the partial manifest
#       `<image>.manifest.part`, sorted by path (call it once per partition,
#       just before the partition is unmounted). The macOS volume metadata
#       directories (.Spotlight-V100 .fseventsd .Trashes .TemporaryItems) can
#       come back after the caller removed them and are not readable; they are
#       not TOS content and are not hashed, but one that is still on the card
#       gets a `metadata <drive>:\<name>` line, so the manifest does not
#       pretend the card is cleaner than it is.
#
#   tools/sd-manifest.sh write <image> <size-mib> <writer> [options]
#       Assemble `<image>.manifest` from the partial manifest and delete the
#       partial one. <writer> names the tool in the header line.
#       --optional-missing <text>   one `optional_missing <text>` line; may be
#                                   repeated. Without it: `optional_missing none`.
#       --switches env              (default) one `switch SD_X=<v>` line per
#                                   SD_* variable in the environment, sorted.
#       --switches n/a              the writer does not read SD_* switches:
#                                   one explicit `switch n/a` line instead.
#
# Manifest format v1 (one key per line; `#` lines are comments; every other
# line starts with its key, so `grep '^optional_missing' <image>.manifest` is
# the whole check a run script needs):
#
#   # NewTOS SD image manifest v1 -- written by <writer>
#   image <image path as given>
#   image_sha256 <sha256 of the image file>
#   size_mib <n>
#   created <UTC time, YYYY-MM-DDThh:mm:ssZ>
#   host_clang <first line of `$CLANG --version`> / host_clang not found
#                          (the clang the SDK build scripts use -- $CLANG,
#                          default clang -- when the manifest is written;
#                          TASK/SDK08)
#   switch SD_X=<v>        (zero or more) / switch n/a
#   optional_missing none  / optional_missing <text> (one or more)
#   # file <sha256> <bytes> <path on the card> / metadata <path> (macOS, not hashed)
#   metadata <drive>:\<name> (macOS volume metadata, not hashed)   (zero or more)
#   file <sha256> <bytes> <drive>:\<path>                          (one per file)
#
# Only `image_sha256` and `created` change between two builds of the same
# inputs -- plus the `file` lines of any content the builder generates at
# random (tools/make-sd-image.sh: C:\KERNEL8.IMG, C:\LYNXBOOT.IMG,
# C:\DATA.BIN). The format is described for runs in
# docs/sdk/06-image-and-testing.md.
#
set -euo pipefail

usage() {
  sed -n '/^# Usage/,/^# Manifest format/p' "${BASH_SOURCE[0]}" | sed '$d; s/^# \{0,1\}//' >&2
  exit 2
}

cmd_files() {
  [ "$#" -eq 3 ] || usage
  local mnt="$1" drive="$2" img="$3" part rel sha size meta
  part="${img}.manifest.part"
  [ -d "$mnt" ] || { echo "sd-manifest: $mnt is not a directory" >&2; exit 1; }
  for meta in .Spotlight-V100 .fseventsd .Trashes .TemporaryItems; do
    if [ -e "$mnt/$meta" ]; then
      printf 'metadata %s:\\%s (macOS volume metadata, not hashed)\n' "$drive" "$meta" >> "$part"
    fi
  done
  ( cd "$mnt" && find . \( -name .Spotlight-V100 -o -name .fseventsd -o -name .Trashes \
      -o -name .TemporaryItems \) -prune -o -type f -print | LC_ALL=C sort ) | while IFS= read -r rel; do
    rel="${rel#./}"
    sha=$(shasum -a 256 "$mnt/$rel" | awk '{print $1}')
    size=$(stat -f %z "$mnt/$rel")
    printf 'file %s %s %s:\\%s\n' "$sha" "$size" "$drive" "$(printf '%s' "$rel" | tr '/' '\\')"
  done >> "$part"
}

cmd_write() {
  [ "$#" -ge 3 ] || usage
  local img="$1" size_mib="$2" writer="$3" switches=env manifest part m
  local optional=()
  shift 3
  while [ "$#" -gt 0 ]; do
    case "$1" in
      --optional-missing) [ "$#" -ge 2 ] || usage; optional+=("$2"); shift 2 ;;
      --switches)         [ "$#" -ge 2 ] || usage; switches="$2"; shift 2 ;;
      *) usage ;;
    esac
  done
  case "$switches" in env|n/a) ;; *) usage ;; esac
  manifest="${img}.manifest"
  part="${img}.manifest.part"
  TMP_MANIFEST="${manifest}.tmp"   # written whole, then renamed: no half manifest
  [ -f "$img" ] || { echo "sd-manifest: image $img not found" >&2; exit 1; }
  [ -f "$part" ] || { echo "sd-manifest: $part not found (no 'files' step ran)" >&2; exit 1; }
  {
    echo "# NewTOS SD image manifest v1 -- written by ${writer}"
    echo "image ${img}"
    echo "image_sha256 $(shasum -a 256 "$img" | awk '{print $1}')"
    echo "size_mib ${size_mib}"
    echo "created $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    if command -v "${CLANG:-clang}" >/dev/null 2>&1; then
      echo "host_clang $("${CLANG:-clang}" --version 2>/dev/null | head -1)"
    else
      echo "host_clang not found"
    fi
    if [ "$switches" = env ]; then
      # Every SD_* switch set in the environment (they change what is on the card).
      env | LC_ALL=C sort | sed -n 's/^\(SD_[A-Za-z0-9_]*=\)/switch \1/p'
    else
      echo "switch n/a"
    fi
    if [ "${#optional[@]}" -eq 0 ]; then
      echo "optional_missing none"
    else
      for m in "${optional[@]}"; do echo "optional_missing $m"; done
    fi
    echo "# file <sha256> <bytes> <path on the card> / metadata <path> (macOS, not hashed)"
    cat "$part"
  } > "$TMP_MANIFEST"
  mv "$TMP_MANIFEST" "$manifest"
  rm -f "$part"
}

TMP_MANIFEST=
trap '[ -z "$TMP_MANIFEST" ] || rm -f "$TMP_MANIFEST"' EXIT
[ "$#" -ge 1 ] || usage
sub="$1"; shift
case "$sub" in
  files) cmd_files "$@" ;;
  write) cmd_write "$@" ;;
  *) usage ;;
esac
