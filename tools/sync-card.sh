#!/usr/bin/env bash
#
# sync-card.sh -- bring build output onto an EXISTING Raspberry Pi 4 card
# (firmware boot partition + FAT partitions C:, D:, ..., mounted on macOS),
# verify every file by sha256 read back from the card, and write a card
# manifest (TASK/PITEST01 part 2). It never formats, never deletes, never
# touches the firmware, and never guesses a volume: every mount point and
# its volume name are arguments.
#
# Not the QEMU image: tools/make-sd-image.sh and tools/mkcard.sh build QEMU
# images from nothing; this tool adds named files to a card that already
# boots. What goes onto the card is a list file (below) kept with the run;
# the tool holds nothing run-specific. The real card is written only on the
# owner's word (CLAUDE.md, docs/STATUS.md "Regel zur SD-Karte").
#
# Usage:
#   tools/sync-card.sh kernels <out-dir> <card-name>=<features> ...
#       Build one rpi4 release kernel per argument (`cargo xtask build
#       --board rpi4 --release [--features <features>]`, `none` = no
#       feature) in the order given, copy each to <out-dir>/<card-name> and
#       record name, sha256, features, commit and dirty flags in
#       <out-dir>/KERNELS.TXT (refused if it exists). <card-name> is the
#       8.3 name the kernel gets on the boot partition, e.g. kernela.img;
#       kernel8.img is reserved for the active copy (see --active).
#
#   tools/sync-card.sh sync   <card options> [--write]
#       Without --write (the default): dry run. Prints per file new /
#       changed / same (sha256 of the build output against the card), the
#       files on the card that are not in the list (extra, left untouched),
#       and missing sources; writes nothing anywhere.
#       With --write: copies new and changed files (each to a temporary
#       8.3 name in the target folder, then renamed), saves the card's
#       previous *.img files of the boot partition into
#       <run-dir>/card-before/, removes the AppleDouble files (`._<name>`)
#       macOS created beside what it wrote (only those that were not on the
#       card before), syncs, unmounts and remounts every volume
#       (so the check reads the card, not the Mac's cache), re-reads every
#       listed file and compares its sha256 with the build output, and
#       only then writes the manifest to <run-dir>/card.manifest and to
#       C:\CARDMAN.TXT (boot partition if no C: is given), which is read
#       back the same way. Any mismatch: loud FAIL, exit status 1, no card
#       manifest.
#
#   tools/sync-card.sh verify <card options>
#       Read-only check: unmount, remount, compare every listed file on the
#       card with the build output. Exit status 1 on any difference.
#
# Card options:
#   --list <file>             the list file (format below); required
#   --boot <mount> <label>    the firmware boot partition (must carry
#                             start4.elf, fixup4.dat, config.txt); required
#   --drive <L> <mount> <label>
#                             TOS drive L (C, D, ...); repeatable. On the
#                             Pi card the boot partition IS C: -- give the
#                             same mount point for both
#   --kernels <dir>           add the kernels of <dir>/KERNELS.TXT (from
#                             `kernels`) to the boot partition under their
#                             card names, plus kernel8.img = the active one
#   --active <card-name>      which of them becomes kernel8.img (required
#                             with --kernels)
#   --run-dir <dir>           where --write puts card.manifest, card.list,
#                             card-before/ (required with --write; refused
#                             if <dir>/card.manifest exists)
#   --allow-boot-config       permit config.txt / cmdline.txt entries in
#                             the list (refused without it)
#
# List file: one line per file, `#` comments and blank lines ignored:
#   <destination> | <source> | <build command shown when the source is missing>
#   destination  C:\APPS\QED\QED.NAP (a TOS drive, `\` or `/`) or
#                BOOT:kernel8.img (the boot partition); 8.3 names only.
#                A destination ending in `\` with a source ending in `/`
#                stands for every file below that folder. A leading `?`
#                makes the entry optional: a missing source gives an
#                `optional_missing` line instead of an error.
#   source       repository-relative or absolute path of the build output
#
# Refusals (exit status 1, before anything is written): a volume whose
# mount point, volume name or file system (msdos) does not match; volumes
# on different disks or on the Mac's own disk; internal media; a boot
# partition without the Pi 4 firmware files; C: not on slice s1 (the
# kernel maps the first MBR entry to C:, tos-api/src/bios/drive.rs:65@mount)
# or drive letters out of slice order; a non-8.3 destination; a destination
# that is firmware (start*.elf, fixup*.dat, bootcode.bin, *.dtb, overlays,
# armstub*); config.txt or cmdline.txt without --allow-boot-config; two
# entries for one card file; a missing required source; with --kernels, a
# config.txt `kernel=` naming another file than kernel8.img or any
# `os_prefix=` (the firmware would then load another file: Raspberry Pi
# documentation @ 7376533a, documentation/asciidoc/computers/config_txt/
# boot.adoc, options `kernel` and `os_prefix`). The Pi 4 firmware files
# checked on the boot partition are the minimum of docs/HW-TEST-LOG.md
# section 3 (start4.elf, fixup4.dat) plus config.txt.
#
# Manifest (card manifest v1, key per line like tools/sd-manifest.sh):
#   # NewTOS card manifest v1 -- written by tools/sync-card.sh
#   card <whole disk at sync time; device names change, the volumes identify the card>
#   volume <C|BOOT|D...> <label> <file system> <slice> [boot]
#   created <UTC>
#   tree_commit <git rev-parse HEAD> tree_dirty=<yes|no>
#   host_clang <first line of `$CLANG --version`> / host_clang not found
#   list <list file as given> <sha256>
#   kernels <dir> commit <sha> tree_dirty=.. kernel_inputs_dirty=..   (with --kernels)
#   kernel <card-name> <sha256> features=<...> ...                    (one per kernel)
#   kernel_active <card-name> -> kernel8.img
#   boot_config_kernel <kernel= / os_prefix= lines of config.txt, or none>
#   boot_cmdline <cmdline.txt content> / boot_cmdline absent
#   switch n/a
#   optional_missing none / optional_missing <text> (one or more)
#   file <sha256> <bytes> <path>        listed file, read back after remount
#   replaced <old sha256> <path>        content before this sync
#   extra <sha256> <bytes> <path>       on the card, not in the list, untouched
#   metadata <path>                     macOS volume metadata, not hashed
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CARD_MANIFEST=CARDMAN.TXT
META_DIRS=".Spotlight-V100 .fseventsd .Trashes .TemporaryItems"
# 8.3 component: 1-8 name characters, optional dot and 1-3 extension characters.
RE83='^[A-Za-z0-9_$%~!#&@^{}()-]{1,8}(\.[A-Za-z0-9_$%~!#&@^{}()-]{1,3})?$'
export COPYFILE_DISABLE=1

usage() {
  sed -n '/^# Usage:/,/^# Refusals/p' "${BASH_SOURCE[0]}" | sed '$d; s/^# \{0,1\}//' >&2
  exit 2
}
die() { echo "sync-card: $*" >&2; exit 1; }
warn() { echo "sync-card: warning: $*" >&2; }
sha() { shasum -a 256 "$1" | awk '{print $1}'; }
size() { stat -f %z "$1"; }
trim() { local s="$1"; s="${s#"${s%%[![:space:]]*}"}"; s="${s%"${s##*[![:space:]]}"}"; printf '%s' "$s"; }
lower() { printf '%s' "$1" | tr '[:upper:]' '[:lower:]'; }
upper() { printf '%s' "$1" | tr '[:lower:]' '[:upper:]'; }
utc() { date -u +%Y-%m-%dT%H:%M:%SZ; }
host_clang() {
  if command -v "${CLANG:-clang}" >/dev/null 2>&1; then
    echo "host_clang $("${CLANG:-clang}" --version 2>/dev/null | head -1)"
  else
    echo "host_clang not found"
  fi
}
yesno() { if [ -n "$1" ]; then echo yes; else echo no; fi; }
KERNEL_INPUTS="kernel tos-api tos-types programs Cargo.toml Cargo.lock rust-toolchain.toml .cargo"
tree_state() {   # prints: <commit> tree_dirty=<yes|no> kernel_inputs_dirty=<yes|no>
  local c t k
  c=$(git -C "$ROOT" rev-parse HEAD)
  t=$(git -C "$ROOT" status --porcelain)
  # shellcheck disable=SC2086
  k=$(cd "$ROOT" && git status --porcelain -- $KERNEL_INPUTS)
  echo "$c tree_dirty=$(yesno "$t") kernel_inputs_dirty=$(yesno "$k")"
}

TMPD=
cleanup_tmp() { [ -z "$TMPD" ] || rm -rf "$TMPD"; }
trap cleanup_tmp EXIT
TMPD=$(mktemp -d "${TMPDIR:-/tmp}/sync-card.XXXXXX")

# ---------------------------------------------------------------- kernels --
cmd_kernels() {
  [ "$#" -ge 2 ] || usage
  local out="$1" spec name feat lname names="" state state2 log path cmdtxt n=0
  shift
  for spec in "$@"; do
    case "$spec" in *=*) ;; *) die "'$spec' is not <card-name>=<features>" ;; esac
    name="${spec%%=*}"; feat="${spec#*=}"; lname=$(lower "$name")
    [[ "$name" =~ $RE83 ]] || die "kernel card name '$name' is not an 8.3 name"
    case "$lname" in *.img) ;; *) die "kernel card name '$name' must end in .img" ;; esac
    [ "$lname" != kernel8.img ] || die "kernel8.img is the active copy (sync --active), not a card name"
    case " $names " in *" $lname "*) die "kernel card name '$name' given twice" ;; esac
    names="$names $lname"
    [ -n "$feat" ] || die "no features for '$name' (use none)"
  done
  mkdir -p "$out"; out="$(cd "$out" && pwd)"
  [ ! -e "$out/KERNELS.TXT" ] || die "$out/KERNELS.TXT exists; give a new directory"
  state=$(tree_state)
  {
    echo "# NewTOS kernels for tools/sync-card.sh -- written by tools/sync-card.sh kernels"
    echo "created $(utc)"
    echo "commit $state"
    echo "host_rustc $(cd "$ROOT" && rustc --version 2>/dev/null || echo unknown)"
  } > "$TMPD/KERNELS.TXT"
  for spec in "$@"; do
    name="${spec%%=*}"; feat="${spec#*=}"; n=$((n + 1))
    if [ "$feat" = none ]; then
      cmdtxt="cargo xtask build --board rpi4 --release"
    else
      cmdtxt="cargo xtask build --board rpi4 --release --features $feat"
    fi
    log="$out/build-$(lower "${name%.*}").log"
    echo "sync-card: [$n/$#] $cmdtxt -> $out/$name"
    # shellcheck disable=SC2086
    (cd "$ROOT" && $cmdtxt) > "$log" 2>&1 || die "build failed, see $log"
    # The last line names the image (tools/xtask/src/main.rs:392-398@cmd_build, TOOL10).
    path=$(tail -1 "$log" | sed -n 's/^xtask: kernel8\.img for the card: //p')
    [ -n "$path" ] && [ -f "$path" ] || die "build did not name its kernel8.img (last line of $log: $(tail -1 "$log"))"
    cp -X "$path" "$out/$name"
    echo "kernel $name $(sha "$out/$name") features=$feat board=rpi4 profile=release cmd=$cmdtxt" >> "$TMPD/KERNELS.TXT"
  done
  state2=$(tree_state)
  [ "$state2" = "$state" ] || die "the tree changed during the builds ($state -> $state2); KERNELS.TXT not written"
  cp "$TMPD/KERNELS.TXT" "$out/KERNELS.TXT"
  cat "$out/KERNELS.TXT"
}

# ---------------------------------------------------------------- volumes --
# One entry per distinct mount point: V_MNT V_LABEL V_LETTER ("" or C, D, ...)
# V_BOOT (1/0) V_DEV V_WHOLE V_FS.
V_MNT=(); V_LABEL=(); V_LETTER=(); V_BOOT=(); V_DEV=(); V_WHOLE=(); V_FS=()
NV=0

realdir() { (cd "$1" 2>/dev/null && pwd -P) || return 1; }
pl() { plutil -extract "$2" raw -o - "$1" 2>/dev/null || true; }

add_volume() {   # <letter or ""> <mount> <label> <is-boot>
  local letter="$1" mnt="$2" label="$3" isboot="$4" real i
  real=$(realdir "$mnt") || die "mount point '$mnt' does not exist"
  i=0
  while [ "$i" -lt "$NV" ]; do
    if [ "${V_MNT[$i]}" = "$real" ]; then
      [ "${V_LABEL[$i]}" = "$label" ] || die "'$mnt' given with two labels ('${V_LABEL[$i]}', '$label')"
      if [ -n "$letter" ]; then
        [ -z "${V_LETTER[$i]}" ] || die "'$mnt' given as ${V_LETTER[$i]}: and $letter:"
        V_LETTER[$i]="$letter"
      fi
      [ "$isboot" = 0 ] || V_BOOT[$i]=1
      return
    fi
    i=$((i + 1))
  done
  V_MNT[$NV]="$real"; V_LABEL[$NV]="$label"; V_LETTER[$NV]="$letter"; V_BOOT[$NV]="$isboot"
  V_DEV[$NV]=""; V_WHOLE[$NV]=""; V_FS[$NV]=""
  NV=$((NV + 1))
}

vname() { if [ -n "${V_LETTER[$1]}" ]; then echo "${V_LETTER[$1]}"; else echo BOOT; fi; }
boot_index() { local i=0; while [ "$i" -lt "$NV" ]; do [ "${V_BOOT[$i]}" = 1 ] && { echo "$i"; return; }; i=$((i + 1)); done; }
letter_index() { local i=0; while [ "$i" -lt "$NV" ]; do [ "${V_LETTER[$i]}" = "$1" ] && { echo "$i"; return; }; i=$((i + 1)); done; }

# Check one volume against diskutil; fills V_DEV/V_WHOLE/V_FS.
check_volume() {
  local i="$1" p="$TMPD/vol$1.plist" mp vn fs whole dev
  diskutil info -plist "${V_MNT[$i]}" > "$p" 2>/dev/null || die "diskutil knows no volume at ${V_MNT[$i]}"
  mp=$(pl "$p" MountPoint); vn=$(pl "$p" VolumeName); fs=$(pl "$p" FilesystemType)
  dev=$(pl "$p" DeviceIdentifier); whole=$(pl "$p" ParentWholeDisk)
  [ -n "$mp" ] && [ "$(realdir "$mp")" = "${V_MNT[$i]}" ] ||
    die "${V_MNT[$i]} is not the mount point of a volume (diskutil: '$mp')"
  [ "$vn" = "${V_LABEL[$i]}" ] || die "${V_MNT[$i]}: volume name is '$vn', expected '${V_LABEL[$i]}'"
  [ "$fs" = msdos ] || die "${V_MNT[$i]}: file system '$fs', expected msdos (FAT)"
  [ "$(pl "$p" WholeDisk)" = false ] || die "${V_MNT[$i]} is a whole disk, not a partition"
  [ "$(pl "$p" OSInternalMedia)" != true ] || die "${V_MNT[$i]} is internal OS media"
  [ "$(pl "$p" RemovableMediaOrExternalDevice)" = true ] || die "${V_MNT[$i]} is neither removable nor external"
  [ "$whole" != "$ROOT_WHOLE" ] || die "${V_MNT[$i]} is on the Mac's startup disk ($whole)"
  V_DEV[$i]="$dev"; V_WHOLE[$i]="$whole"; V_FS[$i]="$(pl "$p" FilesystemName)"
  [ "$(pl "$p" WritableVolume)" = true ] || WRITABLE_PROBLEM="${WRITABLE_PROBLEM} ${V_MNT[$i]}"
}

check_card() {
  local i b slice prev_l prev_s l s f
  ROOT_WHOLE=$(diskutil info -plist / > "$TMPD/root.plist" 2>/dev/null && pl "$TMPD/root.plist" ParentWholeDisk)
  WRITABLE_PROBLEM=""
  i=0; while [ "$i" -lt "$NV" ]; do check_volume "$i"; i=$((i + 1)); done
  i=1; while [ "$i" -lt "$NV" ]; do
    [ "${V_WHOLE[$i]}" = "${V_WHOLE[0]}" ] || die "volumes on different disks (${V_WHOLE[0]}, ${V_WHOLE[$i]})"
    i=$((i + 1))
  done
  b=$(boot_index)
  for f in start4.elf fixup4.dat config.txt; do
    [ -f "${V_MNT[$b]}/$f" ] || die "boot partition ${V_MNT[$b]} has no $f: not a Pi 4 firmware boot partition"
  done
  # Drive letters: C: on slice s1, the others in slice order.
  prev_l=""; prev_s=0
  for l in C D E F G H I J K L M N O P; do
    i=$(letter_index "$l"); [ -n "$i" ] || continue
    slice="${V_DEV[$i]#"${V_WHOLE[$i]}"s}"
    case "$slice" in ''|*[!0-9]*) die "cannot read the slice number of ${V_DEV[$i]}" ;; esac
    if [ "$l" = C ] && [ "$slice" != 1 ]; then
      die "C: (${V_MNT[$i]}) is ${V_DEV[$i]}, but the kernel maps the first MBR partition (s1) to C:"
    fi
    if [ "$slice" -le "$prev_s" ]; then
      die "$l: (${V_DEV[$i]}) does not come after $prev_l: in slice order"
    fi
    prev_l="$l"; prev_s="$slice"
  done
}

print_volumes() {
  local i=0 extra
  echo "card ${V_WHOLE[0]}"
  while [ "$i" -lt "$NV" ]; do
    extra=""; [ "${V_BOOT[$i]}" = 1 ] && extra=" boot"
    echo "volume $(vname "$i") ${V_LABEL[$i]} ${V_FS[$i]// /_} ${V_DEV[$i]}$extra"
    i=$((i + 1))
  done
}

# Unmount and mount again, so that the following reads come from the card.
remount() {
  local i="$1" dev="${V_DEV[$1]}" mnt="${V_MNT[$1]}" p="$TMPD/re$1.plist" newmnt vn
  sync
  diskutil unmount "$mnt" > /dev/null 2>&1 ||
    die "cannot unmount $mnt (a Finder window or an open file?) -- the files are NOT verified"
  if [ -d "$mnt" ]; then
    diskutil mount -mountPoint "$mnt" "$dev" > /dev/null 2>&1 || die "cannot mount $dev at $mnt again -- NOT verified"
  else
    diskutil mount "$dev" > /dev/null 2>&1 || die "cannot mount $dev again -- NOT verified"
  fi
  diskutil info -plist "$dev" > "$p" 2>/dev/null || die "diskutil lost $dev after the remount"
  newmnt=$(pl "$p" MountPoint); vn=$(pl "$p" VolumeName)
  [ "$vn" = "${V_LABEL[$i]}" ] || die "$dev came back as '$vn', expected '${V_LABEL[$i]}'"
  [ -n "$newmnt" ] || die "$dev is not mounted after the remount"
  newmnt=$(realdir "$newmnt")
  if [ "$newmnt" != "$mnt" ]; then
    warn "$dev ($vn) came back at $newmnt instead of $mnt; using $newmnt"
    V_MNT[$i]="$newmnt"
  fi
}

# ------------------------------------------------------------------- plan --
# $TMPD/plan: one TAB-separated line per card file:
#   <vol index> <rel path, / separated> <source> <display path> <source sha256>
PLAN_ERRORS=0
OPTIONAL_MISSING=()
plan_error() { echo "error: $*" >&2; PLAN_ERRORS=$((PLAN_ERRORS + 1)); }

add_entry() {   # <vol index> <rel> <source> <display>
  local vi="$1" rel="$2" src="$3" disp="$4" comp lrel top old_ifs
  old_ifs="$IFS"; IFS=/
  for comp in $rel; do
    [[ "$comp" =~ $RE83 ]] || { IFS="$old_ifs"; plan_error "$disp: '$comp' is not an 8.3 name"; return; }
  done
  IFS="$old_ifs"
  lrel=$(lower "$rel"); top="${lrel%%/*}"
  if [ "${V_BOOT[$vi]}" = 1 ]; then
    case "$lrel" in
      start*.elf|fixup*.dat|bootcode.bin|*.dtb|armstub*) plan_error "$disp: firmware file, never written"; return ;;
    esac
    [ "$top" != overlays ] || { plan_error "$disp: firmware overlay, never written"; return; }
    case "$lrel" in
      config.txt|cmdline.txt)
        [ "$ALLOW_BOOT_CONFIG" = 1 ] || { plan_error "$disp: boot configuration, needs --allow-boot-config"; return; } ;;
    esac
  fi
  if [ "$lrel" = "$(lower "$CARD_MANIFEST")" ] && [ "$vi" = "$MAN_VOL" ]; then
    plan_error "$disp: reserved for the card manifest"; return
  fi
  if awk -F'\t' -v v="$vi" -v r="$lrel" '$1 == v && tolower($2) == r { f = 1 } END { exit !f }' "$TMPD/plan"; then
    plan_error "$disp: listed twice"; return
  fi
  printf '%s\t%s\t%s\t%s\t%s\n' "$vi" "$rel" "$src" "$disp" "$(sha "$src")" >> "$TMPD/plan"
}

resolve_dest() {   # <dest> -> sets R_VI R_REL R_PREFIX, or returns 1
  local d="$1" letter path
  case "$d" in
    BOOT:*) R_VI=$(boot_index); path="${d#BOOT:}" ;;
    [A-Za-z]:*) letter=$(upper "${d%%:*}"); path="${d#?:}"; R_VI=$(letter_index "$letter")
                [ -n "$R_VI" ] || { plan_error "$d: drive $letter: not given (--drive $letter ...)"; return 1; } ;;
    *) plan_error "'$d' is neither BOOT:... nor <drive>:\\..."; return 1 ;;
  esac
  path=$(printf '%s' "$path" | tr '\\' '/'); path="${path#/}"
  R_REL="$path"; R_PREFIX="$(vname "$R_VI"):\\"
}

disp_of() { printf '%s%s' "$2" "$(printf '%s' "$1" | tr '/' '\\')"; }

build_plan() {
  local line d s h opt src f rel n=0 old_ifs
  : > "$TMPD/plan"
  [ -f "$LIST" ] || die "list file $LIST not found"
  while IFS= read -r line || [ -n "$line" ]; do
    n=$((n + 1)); line="${line%$'\r'}"
    case "$(trim "$line")" in ''|'#'*) continue ;; esac
    old_ifs="$IFS"; IFS='|' read -r d s h <<< "$line"; IFS="$old_ifs"
    d=$(trim "$d"); s=$(trim "${s:-}"); h=$(trim "${h:-}")
    [ -n "$s" ] || { plan_error "$LIST:$n: no source"; continue; }
    opt=0; case "$d" in '?'*) opt=1; d=$(trim "${d#?}") ;; esac
    resolve_dest "$d" || continue
    case "$s" in /*) src="$s" ;; *) src="$ROOT/$s" ;; esac
    case "$R_REL" in
      */|'')
        case "$s" in */) ;; *) plan_error "$LIST:$n: folder destination $d needs a folder source ending in /"; continue ;; esac
        if [ ! -d "$src" ]; then
          if [ "$opt" = 1 ]; then OPTIONAL_MISSING+=("$d -- $s missing (build: $h)")
          else plan_error "$d: source folder $s missing -- build it with: $h"; fi
          continue
        fi
        while IFS= read -r f; do
          rel="${R_REL}${f#./}"
          add_entry "$R_VI" "$rel" "${src%/}/${f#./}" "$(disp_of "$rel" "$R_PREFIX")"
        done < <(cd "$src" && find . -type f ! -name .DS_Store ! -name '._*' | LC_ALL=C sort)
        ;;
      *)
        case "$s" in */) plan_error "$LIST:$n: file destination $d with folder source $s"; continue ;; esac
        if [ ! -f "$src" ]; then
          if [ "$opt" = 1 ]; then OPTIONAL_MISSING+=("$d -- $s missing (build: $h)")
          else plan_error "$d: $s missing -- build it with: $h"; fi
          continue
        fi
        add_entry "$R_VI" "$R_REL" "$src" "$(disp_of "$R_REL" "$R_PREFIX")"
        ;;
    esac
  done < "$LIST"
  if [ -n "$KERNELS" ]; then add_kernels; fi
  [ "${#OPTIONAL_MISSING[@]}" -eq 0 ] || for f in "${OPTIONAL_MISSING[@]}"; do echo "optional missing: $f"; done
}

add_kernels() {
  local kt="$KERNELS/KERNELS.TXT" k name ksha rest have bi found=0
  [ -f "$kt" ] || die "$kt not found (tools/sync-card.sh kernels <dir> ...)"
  bi=$(boot_index)
  while read -r k name ksha rest; do
    [ "$k" = kernel ] || continue
    [ -f "$KERNELS/$name" ] || { plan_error "kernel $KERNELS/$name missing"; continue; }
    have=$(sha "$KERNELS/$name")
    [ "$have" = "$ksha" ] || { plan_error "kernel $name: sha256 $have, KERNELS.TXT says $ksha"; continue; }
    add_entry "$bi" "$name" "$KERNELS/$name" "$(disp_of "$name" "$(vname "$bi"):\\")"
    if [ "$(lower "$name")" = "$(lower "$ACTIVE")" ]; then
      add_entry "$bi" kernel8.img "$KERNELS/$name" "$(disp_of kernel8.img "$(vname "$bi"):\\")"
      found=1
    fi
  done < "$kt"
  [ "$found" = 1 ] || plan_error "--active $ACTIVE is none of the kernels in $kt"
}

# config.txt / cmdline.txt of the boot partition (read only).
read_boot_config() {
  local bi m
  bi=$(boot_index); m="${V_MNT[$bi]}"
  BOOT_KERNEL_LINES=$(tr -d '\r' < "$m/config.txt" | grep -i -E '^[[:space:]]*(kernel|os_prefix)[[:space:]]*=' |
                      sed 's/[[:space:]]*$//' | paste -sd ';' - || true)
  if [ -f "$m/cmdline.txt" ]; then BOOT_CMDLINE=$(tr -d '\r' < "$m/cmdline.txt" | paste -sd ' ' -)
  else BOOT_CMDLINE=""; fi
  if [ -n "$KERNELS" ]; then
    # Whole-output tests, no `grep -q` inside a pipefail pipeline (an early
    # exit there can turn a hit into SIGPIPE).
    local pfx other
    pfx=$(tr -d '\r' < "$m/config.txt" | grep -i -E '^[[:space:]]*os_prefix[[:space:]]*=' || true)
    [ -z "$pfx" ] || plan_error "config.txt sets os_prefix ($pfx): the firmware would not load kernel8.img from the partition root"
    other=$(tr -d '\r' < "$m/config.txt" | grep -i -E '^[[:space:]]*kernel[[:space:]]*=' |
            sed -E 's/^[[:space:]]*[kK][eE][rR][nN][eE][lL][[:space:]]*=[[:space:]]*//; s/[[:space:]]*$//' |
            tr '[:upper:]' '[:lower:]' | grep -v -x -E '/?kernel8\.img' || true)
    [ -z "$other" ] || plan_error "config.txt names another kernel than kernel8.img ($BOOT_KERNEL_LINES): the active kernel would not boot"
  fi
}

# Status of every plan line against the card: $TMPD/status = plan line + <status> <card sha>.
compute_status() {
  local vi rel src disp ssha f csha st
  : > "$TMPD/status"
  while IFS=$'\t' read -r vi rel src disp ssha; do
    f="${V_MNT[$vi]}/$rel"
    if [ -d "$f" ]; then plan_error "$disp is a folder on the card"; continue; fi
    if [ -f "$f" ]; then
      csha=$(sha "$f")
      if [ "$csha" = "$ssha" ]; then st=same; else st=changed; fi
    else
      csha=-; st=new
    fi
    printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$vi" "$rel" "$src" "$disp" "$ssha" "$st" "$csha" >> "$TMPD/status"
  done < "$TMPD/plan"
}

# Files on the card that are not in the plan: $TMPD/extra = <vol index> TAB <rel>; $TMPD/meta = display lines.
compute_extras() {
  local i m d
  : > "$TMPD/extra"; : > "$TMPD/meta"
  i=0
  while [ "$i" -lt "$NV" ]; do
    m="${V_MNT[$i]}"
    for d in $META_DIRS; do
      [ ! -e "$m/$d" ] || echo "metadata $(vname "$i"):\\$d (macOS volume metadata, not hashed)" >> "$TMPD/meta"
    done
    awk -F'\t' -v v="$i" '$1 == v { print tolower($2) }' "$TMPD/plan" > "$TMPD/known"
    [ "$i" != "$MAN_VOL" ] || lower "$CARD_MANIFEST" >> "$TMPD/known"
    (cd "$m" && find . \( -name .Spotlight-V100 -o -name .fseventsd -o -name .Trashes -o -name .TemporaryItems \) \
        -prune -o -type f -print | sed 's|^\./||' | LC_ALL=C sort) |
      awk -v v="$i" -v kf="$TMPD/known" 'BEGIN { while ((getline l < kf) > 0) k[l] = 1 }
                                         !(tolower($0) in k) { print v "\t" $0 }' >> "$TMPD/extra"
    i=$((i + 1))
  done
}

relsrc() { case "$1" in "$ROOT"/*) printf '%s' "${1#"$ROOT"/}" ;; *) printf '%s' "$1" ;; esac; }

print_plan() {
  local vi rel src disp ssha st csha nnew=0 nch=0 nsame=0 nextra
  while IFS=$'\t' read -r vi rel src disp ssha st csha; do
    printf '  %-8s %s <- %s\n' "$st" "$disp" "$(relsrc "$src")"
    case "$st" in new) nnew=$((nnew + 1)) ;; changed) nch=$((nch + 1)) ;; same) nsame=$((nsame + 1)) ;; esac
  done < "$TMPD/status"
  while IFS=$'\t' read -r vi rel; do
    printf '  %-8s %s (on the card, not in the list; left untouched)\n' extra "$(disp_of "$rel" "$(vname "$vi"):\\")"
  done < "$TMPD/extra"
  nextra=$(wc -l < "$TMPD/extra" | tr -d ' ')
  [ "$nextra" = 0 ] || warn "$nextra file(s) on the card are not in the list (extra, left untouched)"
  echo "boot_config_kernel ${BOOT_KERNEL_LINES:-none}"
  if [ -n "$BOOT_CMDLINE" ]; then echo "boot_cmdline $BOOT_CMDLINE"; else echo "boot_cmdline absent"; fi
  echo "summary: $nnew new, $nch changed, $nsame same, $nextra extra, ${#OPTIONAL_MISSING[@]} optional missing, $PLAN_ERRORS error(s)"
  N_TODO=$((nnew + nch))
}

# Re-read every planned file from the card (after a remount) and compare.
verify_card() {
  local vi rel src disp ssha st csha f bad=0 have
  : > "$TMPD/readback"
  while IFS=$'\t' read -r vi rel src disp ssha st csha; do
    f="${V_MNT[$vi]}/$rel"
    if [ ! -f "$f" ]; then
      echo "MISMATCH $disp: not on the card" >&2; bad=$((bad + 1)); continue
    fi
    have=$(sha "$f")
    if [ "$have" != "$ssha" ]; then
      echo "MISMATCH $disp: card $have, build output $ssha ($(relsrc "$src"))" >&2; bad=$((bad + 1))
    fi
    printf 'file %s %s %s\n' "$have" "$(size "$f")" "$disp" >> "$TMPD/readback"
  done < "$TMPD/status"
  VERIFY_BAD=$bad
}

remount_all() { local i=0; while [ "$i" -lt "$NV" ]; do remount "$i"; i=$((i + 1)); done; }

# AppleDouble files (`._<name>`): macOS writes one beside a file or folder it
# creates on FAT when the new item carries an extended attribute (e.g. the
# provenance attribute), even with `cp -X`. They are files TOS sees. The ones
# that appear while this tool writes are its own and are removed again; the
# ones that were on the card before are left alone (listed as extra).
dotunder_list() {   # <vol index> -> sorted rel paths of the ._* files
  (cd "${V_MNT[$1]}" && find . \( -name .Spotlight-V100 -o -name .fseventsd -o -name .Trashes \
      -o -name .TemporaryItems \) -prune -o -type f -name '._*' -print | sed 's|^\./||' | LC_ALL=C sort)
}
snapshot_dotunder() { dotunder_list "$1" > "$TMPD/du.$1"; }
clean_dotunder() {  # <vol index>: remove the ._* files that appeared since snapshot_dotunder
  local i="$1" rel
  dotunder_list "$i" > "$TMPD/du2.$i"
  LC_ALL=C comm -13 "$TMPD/du.$i" "$TMPD/du2.$i" | while IFS= read -r rel; do
    rm -f "${V_MNT[$i]}/$rel"
    echo "  removed  $(disp_of "$rel" "$(vname "$i"):\\") (AppleDouble file macOS made for a file written here)"
  done
}

write_files() {
  local vi rel src disp ssha st csha f dir tmp n=0 bk
  mkdir -p "$RUN_DIR/card-before"
  while IFS=$'\t' read -r vi rel src disp ssha st csha; do
    [ "$st" != same ] || continue
    f="${V_MNT[$vi]}/$rel"; dir=$(dirname "$f")
    if [ "$st" = changed ]; then
      echo "replaced $csha $disp (content before this sync)" >> "$TMPD/replaced"
      case "$(lower "$rel")" in
        *.img) if [ "${V_BOOT[$vi]}" = 1 ]; then
                 bk="$RUN_DIR/card-before/$(vname "$vi")-$(printf '%s' "$rel" | tr '/' '_')"
                 cp -X "$f" "$bk"; [ "$(sha "$bk")" = "$csha" ] || die "backup of $disp is not the card's file"
               fi ;;
      esac
    fi
    mkdir -p "$dir"
    n=$((n + 1)); tmp="$dir/$(printf 'SYN%05d.TMP' "$n")"
    cp -X "$src" "$tmp"
    mv -f "$tmp" "$f"
    echo "  wrote    $disp"
  done < "$TMPD/status"
}

assemble_manifest() {   # <output file>
  local out="$1" vi rel f k
  {
    echo "# NewTOS card manifest v1 -- written by tools/sync-card.sh"
    print_volumes
    echo "created $(utc)"
    echo "tree_commit $(tree_state | cut -d' ' -f1,2)"
    host_clang
    echo "list $(relsrc "$LIST") $(sha "$LIST")"
    if [ -n "$KERNELS" ]; then
      echo "kernels $(relsrc "$KERNELS") $(sed -n 's/^commit //p' "$KERNELS/KERNELS.TXT")"
      grep '^kernel ' "$KERNELS/KERNELS.TXT"
      echo "kernel_active $ACTIVE -> kernel8.img"
    fi
    echo "boot_config_kernel ${BOOT_KERNEL_LINES:-none}"
    if [ -n "$BOOT_CMDLINE" ]; then echo "boot_cmdline $BOOT_CMDLINE"; else echo "boot_cmdline absent"; fi
    echo "switch n/a"
    if [ "${#OPTIONAL_MISSING[@]}" -eq 0 ]; then
      echo "optional_missing none"
    else
      for k in "${OPTIONAL_MISSING[@]}"; do echo "optional_missing $k"; done
    fi
    echo "# file <sha256> <bytes> <path> (listed, read back after remount) / replaced <old sha256> <path> /"
    echo "# extra <sha256> <bytes> <path> (not in the list, untouched) / metadata <path> (macOS, not hashed)"
    cat "$TMPD/readback"
    [ ! -f "$TMPD/replaced" ] || cat "$TMPD/replaced"
    while IFS=$'\t' read -r vi rel; do
      f="${V_MNT[$vi]}/$rel"
      printf 'extra %s %s %s\n' "$(sha "$f")" "$(size "$f")" "$(disp_of "$rel" "$(vname "$vi"):\\")"
    done < "$TMPD/extra"
    cat "$TMPD/meta"
  } > "$out"
}

# ------------------------------------------------------------- sync/verify --
cmd_card() {   # <sync|verify> <args>
  local mode="$1" a l i
  shift
  LIST=""; KERNELS=""; ACTIVE=""; RUN_DIR=""; ALLOW_BOOT_CONFIG=0; WRITE=0
  local nboot=0
  while [ "$#" -gt 0 ]; do
    a="$1"
    case "$a" in
      --list)    [ "$#" -ge 2 ] || usage; LIST="$2"; shift 2 ;;
      --boot)    [ "$#" -ge 3 ] || usage; add_volume "" "$2" "$3" 1; nboot=$((nboot + 1)); shift 3 ;;
      --drive)   [ "$#" -ge 4 ] || usage; l=$(upper "$2")
                 case "$l" in [C-P]) ;; *) die "--drive $2: drive letter C..P expected" ;; esac
                 add_volume "$l" "$3" "$4" 0; shift 4 ;;
      --kernels) [ "$#" -ge 2 ] || usage; KERNELS="$(cd "$2" 2>/dev/null && pwd)" || die "kernels dir $2 not found"; shift 2 ;;
      --active)  [ "$#" -ge 2 ] || usage; ACTIVE="$2"; shift 2 ;;
      --run-dir) [ "$#" -ge 2 ] || usage; RUN_DIR="$2"; shift 2 ;;
      --allow-boot-config) ALLOW_BOOT_CONFIG=1; shift ;;
      --write)   [ "$mode" = sync ] || usage; WRITE=1; shift ;;
      *) usage ;;
    esac
  done
  [ -n "$LIST" ] || die "--list is required"
  [ "$nboot" = 1 ] || die "exactly one --boot <mount> <label> is required"
  if [ -n "$KERNELS" ] && [ -z "$ACTIVE" ]; then die "--kernels needs --active <card-name>"; fi
  if [ -z "$KERNELS" ] && [ -n "$ACTIVE" ]; then die "--active needs --kernels <dir>"; fi
  if [ "$WRITE" = 1 ]; then
    [ -n "$RUN_DIR" ] || die "--write needs --run-dir <dir>"
    [ ! -e "$RUN_DIR/card.manifest" ] || die "$RUN_DIR/card.manifest exists; give a new run directory"
  fi
  case "$LIST" in /*) ;; *) LIST="$(pwd)/$LIST" ;; esac

  check_card
  MAN_VOL=$(letter_index C); [ -n "$MAN_VOL" ] || MAN_VOL=$(boot_index)
  print_volumes
  build_plan
  read_boot_config
  compute_status
  compute_extras
  print_plan
  [ "$PLAN_ERRORS" = 0 ] || die "$PLAN_ERRORS error(s); nothing written"

  if [ "$mode" = verify ]; then
    remount_all
    verify_card
    [ "$VERIFY_BAD" = 0 ] || die "VERIFY FAILED: $VERIFY_BAD file(s) on the card differ from the build output"
    echo "verify: $(wc -l < "$TMPD/readback" | tr -d ' ') file(s) on the card equal the build output (read after remount)"
    return
  fi
  if [ "$WRITE" = 0 ]; then
    echo "dry run: nothing written (add --write to copy $N_TODO file(s))"
    return
  fi
  [ -z "$WRITABLE_PROBLEM" ] || die "read-only volume(s):$WRITABLE_PROBLEM"

  mkdir -p "$RUN_DIR"; RUN_DIR="$(cd "$RUN_DIR" && pwd)"
  if [ -e "$RUN_DIR/card.list" ]; then
    cmp -s "$LIST" "$RUN_DIR/card.list" || die "$RUN_DIR/card.list exists and differs from $LIST"
  else
    cp "$LIST" "$RUN_DIR/card.list"
  fi
  i=0; while [ "$i" -lt "$NV" ]; do snapshot_dotunder "$i"; i=$((i + 1)); done
  write_files
  sync
  i=0; while [ "$i" -lt "$NV" ]; do clean_dotunder "$i"; i=$((i + 1)); done
  remount_all
  verify_card
  if [ "$VERIFY_BAD" != 0 ]; then
    assemble_manifest "$RUN_DIR/card.manifest.FAILED"
    die "VERIFY FAILED after the copy: $VERIFY_BAD file(s) differ (see above, $RUN_DIR/card.manifest.FAILED); no card manifest written"
  fi
  # The manifest lists the card after the copy, so compute the extras again.
  compute_extras
  assemble_manifest "$RUN_DIR/card.manifest"
  snapshot_dotunder "$MAN_VOL"
  cp -X "$RUN_DIR/card.manifest" "${V_MNT[$MAN_VOL]}/SYN00000.TMP"
  mv -f "${V_MNT[$MAN_VOL]}/SYN00000.TMP" "${V_MNT[$MAN_VOL]}/$CARD_MANIFEST"
  sync
  clean_dotunder "$MAN_VOL"
  remount "$MAN_VOL"
  [ "$(sha "${V_MNT[$MAN_VOL]}/$CARD_MANIFEST")" = "$(sha "$RUN_DIR/card.manifest")" ] ||
    die "VERIFY FAILED: $(vname "$MAN_VOL"):\\$CARD_MANIFEST differs from $RUN_DIR/card.manifest"
  echo "written and verified: $N_TODO file(s) copied, $(wc -l < "$TMPD/readback" | tr -d ' ') listed file(s) equal the build output (read after remount)"
  echo "manifest: $RUN_DIR/card.manifest = $(vname "$MAN_VOL"):\\$CARD_MANIFEST ($(sha "$RUN_DIR/card.manifest"))"
}

[ "$#" -ge 1 ] || usage
sub="$1"; shift
case "$sub" in
  kernels) cmd_kernels "$@" ;;
  sync)    cmd_card sync "$@" ;;
  verify)  cmd_card verify "$@" ;;
  *) usage ;;
esac
