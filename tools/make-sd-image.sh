#!/usr/bin/env bash
#
# Create a raw FAT16 SD-card image for NewTOS (QEMU raspi4b / real Pi-4).
#
# The image is a standard MBR (signature 0x55AA) with one primary FAT16
# partition of type 0x06, matching the kernel's MBR parser in
# `tos-api/src/bios/drive.rs` (`entry()` reads the 16-byte records at 0x1BE;
# type 0x06 = FAT16). The partition is populated with a handful of sample
# files and folders so the desktop shows a `C:` drive with content — including
# the ported GEM editor QED under `C:\Apps\QED\` (double-click to launch).
#
# macOS only (uses hdiutil / diskutil / newfs_msdos). Runs without sudo — the
# raw disk image is attached as a user-owned device.
#
# Usage:
#   tools/make-sd-image.sh [image-path] [size-MiB]
#     image-path   defaults to /tmp/sd.img (the path `xtask run --sd` expects)
#     size-MiB     defaults to 64
#
# Re-run this after a reboot / power outage wiped /tmp.
#
# Every program listed below is MANDATORY (user decision 2026-10-26): a
# missing one ends the script with a non-zero status, naming the file and the
# command that builds it, before the image is touched. It used to print a
# `note:` and build the card without it -- which is how a boot test once ran
# without MXPROBE.NCP and never reached the TASK/MEM11 debug assertions. The
# build order of the prerequisites is in docs/sdk/06-image-and-testing.md.
# TeraDesk's colour icons cicons.rsc are on that list too: they change how the
# desktop renders, and every image this script builds boots a desktop that may
# be measured (user decision 2026-10-26).
#
# A part may stay OPTIONAL only when its own build treats it as optional (today
# only the zView sample pictures, C:\ZVIEW\SAMPLES\). A missing optional part
# is never silent: it prints an `optional missing: ...` line, which is
# repeated in the summary at the end, and it is recorded in the manifest.
#
# Manifest: next to the image the script writes `<image-path>.manifest`, a
# plain-text record of what went onto the card -- one `file <sha256> <bytes>
# <TOS path>` line per file, the `optional_missing` lines, the SD_* switches
# in effect and the image's own sha256 (format v1, written by the shared
# tools/sd-manifest.sh, which tools/mkcard uses too). A measurement run that
# uses the image copies the manifest into its run directory and RUNINFO.TXT
# cites it (docs/sdk/06-image-and-testing.md). The manifest is deleted
# together with the image when a build fails, so a manifest always belongs to
# the image beside it.
#
set -euo pipefail

IMG="${1:-/tmp/sd.img}"
SIZE_MB="${2:-64}"
VOL="NEWTOS"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# The prebuilt programs (and their data files) the image carries, each with
# the command that produces it, run from the repository root. Checked all at
# once, so one run lists everything that is missing. The freestanding
# sdk/examples/*.c programs are not in this list: the script builds them
# itself (`build_example` below) and stops if that fails.
REQUIRED=(
  "sdk/libc/build/argvtest.NAP|sdk/libc/build-libc.sh sdk/examples/argvtest.c sdk/libc/build/argvtest.NAP"
  "sdk/libc/build/libtest.NCP|sdk/libc/build-libc.sh"
  "sdk/libc/build/clocktest.NCP|sdk/libc/build-libc.sh sdk/libc/tests/clocktest.c"
  "sdk/libc/build/setjmptst.NCP|sdk/libc/build-libc.sh sdk/examples/setjmptst.c sdk/libc/build/setjmptst.NCP"
  "sdk/libc/build/MXPROBE.NCP|sdk/libc/build-libc.sh sdk/libc/tests/mxprobe.c sdk/libc/build/MXPROBE.NCP"
  "sdk/libm/build/mathtst.NCP|sdk/libm/build-libm.sh"
  "sdk/ldg/build/EX_LIB.NLD|sdk/ldg/build-ldg-demo.sh"
  "sdk/ldg/build/LDGTEST.NCP|sdk/ldg/build-ldg-demo.sh"
  "sdk/zview/build/dist/ZVIEW/ZVIEW.NAP|sdk/zview/build-zview.sh"
  "sdk/apps/coremark/build/CORE.NCP|sdk/apps/build-coremark.sh"
  "sdk/apps/coremark/build/COREVAL.NCP|sdk/apps/build-coremark.sh"
  "tosdump/build/DUMPVDI.NAP|make -C tosdump dumpvdi-newtos"
  "tosdump/cases/vdi-r2.tsv|(tracked file -- restore it from git)"
  "tosdump/build/VDIDRAW.NAP|tosdump/build-vdidraw-newtos.sh"
  "sdk/apps/qed/build/QED.NAP|sdk/apps/build-qed.sh"
  "sdk/apps/qed/build/qed.rsc|sdk/apps/build-qed.sh"
  "sdk/apps/teradesk/build/dist/DESKTOP.NAP|sdk/apps/build-teradesk.sh"
  "sdk/apps/teradesk/build/dist/desktop.rsc|sdk/apps/build-teradesk.sh"
  "sdk/apps/teradesk/build/dist/icons.rsc|sdk/apps/build-teradesk.sh"
  "sdk/apps/teradesk/build/dist/cicons.rsc|sdk/apps/build-teradesk.sh"
  "sdk/apps/present/build/PRESENT.NAP|sdk/apps/present/build-present.sh"
  "sdk/apps/present/build/TEST.PNG|sdk/apps/present/build-present.sh"
  "sdk/apps/present/sample/SLIDES.TXT|(tracked file -- restore it from git)"
)
missing=0
for entry in "${REQUIRED[@]}"; do
  file="${entry%%|*}"
  if [ ! -f "$ROOT/$file" ]; then
    echo "error: $file is missing -- build it with: ${entry#*|}" >&2
    missing=$((missing + 1))
  fi
done
if [ "$missing" -ne 0 ]; then
  echo "error: $missing required file(s) missing; no image written (build order: docs/sdk/06-image-and-testing.md)" >&2
  exit 1
fi

# Build one freestanding example sdk/examples/<src> with sdk/build.sh into
# <out>. A failure prints the compiler output and ends the script.
build_example() {
  local src="$1" out="$2" log
  if ! log=$("$ROOT/sdk/build.sh" "$ROOT/sdk/examples/$src" "$out" 2>&1); then
    printf '%s\n' "$log" | sed 's/^/    /' >&2
    echo "error: sdk/build.sh sdk/examples/$src failed (clang/lld missing?); no image written" >&2
    exit 1
  fi
}

# The manifest (see the header) is assembled at the very end from the parts
# collected on the way: MANIFEST_FILES gets one `file` line per file on the
# card (written by list_card_files just before each partition is unmounted),
# OPTIONAL_MISSING one entry per optional part that is not on the card.
MANIFEST="${IMG}.manifest"
MANIFEST_FILES="${IMG}.manifest.part"
OPTIONAL_MISSING=()

# Record an optional part that is not on the card: print it now and keep it
# for the summary and the manifest. $1 = what is missing (TOS path), $2 = why
# and how to get it.
optional_missing() {
  OPTIONAL_MISSING+=("$1 -- $2")
  echo "optional missing: $1 -- $2"
}

# Append one `file <sha256> <bytes> <drive>:\<path>` line per file under the
# mounted partition $1 (drive letter $2) to MANIFEST_FILES, sorted by path,
# plus a `metadata <path>` line per macOS volume metadata directory that came
# back -- the shared manifest writer does it (tools/sd-manifest.sh, `files`).
list_card_files() {
  "$SCRIPT_DIR/sd-manifest.sh" files "$1" "$2" "$IMG"
}

echo "Creating ${IMG} (${SIZE_MB} MiB, MBR + FAT16, volume ${VOL})…"
rm -f "$IMG" "$MANIFEST" "$MANIFEST_FILES"
dd if=/dev/zero of="$IMG" bs=1m count="$SIZE_MB" status=none 2>/dev/null \
  || dd if=/dev/zero of="$IMG" bs=1m count="$SIZE_MB"

# Attach the zero-filled file as a raw block device (no filesystem yet).
DEV=$(hdiutil attach -imagekey diskimage-class=CRawDiskImage -nomount "$IMG" \
        | head -1 | awk '{print $1}')
echo "attached ${DEV}"
cleanup() { hdiutil detach "$DEV" >/dev/null 2>&1 || true; }
# On any failure from here on, also delete the half-written image, so that
# nothing (e.g. `cargo xtask run`, which defaults to /tmp/sd.img) boots it.
# The partial manifest goes with it.
trap 'rc=$?; cleanup; [ "$rc" -eq 0 ] || rm -f "$IMG" "$MANIFEST_FILES"' EXIT

# One MBR FAT16 partition spanning the whole disk, mounted at /Volumes/$VOL.
#
# Opt-in, QEMU image only: SD_FSBENCH_D=<dir> splits the card into TWO FAT16
# partitions -- C: (NEWTOS, half) and D: (NEWTOS1, the rest) -- and copies the
# FSBENCH test files from <dir> into D:\FSBENCH\ (TASK/FS08; the files come
# from sdk/examples/fsbench/gen-fsbench-files.py). Use a size of >= 128 MiB.
# Without the variable the layout is the unchanged single partition.
if [ -n "${SD_FSBENCH_D:-}" ]; then
  diskutil partitionDisk "$DEV" 2 MBR "MS-DOS FAT16" "$VOL" 50% "MS-DOS FAT16" "${VOL}1" R >/dev/null
else
  diskutil partitionDisk "$DEV" 1 MBR "MS-DOS FAT16" "$VOL" 100% >/dev/null
fi
V="/Volumes/${VOL}"
if [ -n "${SD_FSBENCH_D:-}" ]; then
  VD="$(diskutil info "${DEV}s2" | sed -n 's/^ *Mount Point: *//p')"
  [ -d "$VD" ] || { echo "error: D: (${DEV}s2) not mounted" >&2; exit 1; }
  mdutil -i off "$VD" >/dev/null 2>&1 || true
  mkdir -p "$VD/FSBENCH"
  for f in "$SD_FSBENCH_D"/*.DAT; do cp "$f" "$VD/FSBENCH/"; done
  echo "  + D:\\FSBENCH\\ ($(ls "$VD/FSBENCH" | wc -l | tr -d ' ') files from $SD_FSBENCH_D)"
  rm -rf "$VD/.Spotlight-V100" "$VD/.fseventsd" "$VD/.Trashes" "$VD/.TemporaryItems" 2>/dev/null || true
  dot_clean -m "$VD" 2>/dev/null || true
  sync
  list_card_files "$VD" D
  diskutil unmount "${DEV}s2" >/dev/null
fi

# Populate with sample content (CR/LF line ends, TOS-style). The names
# deliberately include worst-case 12-char 8.3 filenames (KEYBOARD.KBD,
# LYNXBOOT.IMG) and several folders, so the icon-view label layout is tested
# representatively — a run with only short names once hid a label-overlap bug.
mdutil -i off "$V" >/dev/null 2>&1 || true
printf 'NewTOS test disk (drive C:)\r\nFAT16 image for QEMU + real Pi-4.\r\n' > "$V/README.TXT"
printf 'Hello from the NewTOS desktop!\r\n'                                   > "$V/HELLO.TXT"
printf 'config placeholder\r\n'                                              > "$V/CONFIG.TXT"
printf 'newdesk placeholder\r\n'                                             > "$V/NEWDESK.INF"
printf 'keyboard table placeholder\r\n'                                      > "$V/KEYBOARD.KBD"
head -c 4096 /dev/urandom                                                    > "$V/KERNEL8.IMG"
head -c 512  /dev/urandom                                                    > "$V/LYNXBOOT.IMG"
head -c 4096 /dev/urandom                                                    > "$V/DATA.BIN"
# C:\ACC\ is the first of the two desk-accessory search locations (ACC02);
# the SYSACC.NAC install block below fills it.
mkdir -p "$V/ACC"
mkdir -p "$V/GEMSYS"
mkdir -p "$V/SUB";  printf 'A file inside SUB.\r\n'                           > "$V/SUB/INSIDE.TXT"
# Two files of the SAME bare name, one per directory: what CURDIR.NCP (below)
# reads back names the directory it was launched in. Used by the working-
# directory proof (desktop `do_aopen` -> `set_default_path`, DESK18).
printf 'PROOF: C:\\ (root)\r\n'                                                > "$V/PROOF.TXT"
printf 'PROOF: C:\\SUB\r\n'                                                    > "$V/SUB/PROOF.TXT"
mkdir -p "$V/AUTO"; printf 'AUTO runs native console (.NCP) programs at boot, before the desktop/AES.\r\nGEM apps (.NAP) do NOT belong here - they launch from the desktop.\r\n' > "$V/AUTO/READ.ME"

# ADR-018 / Task #72 stage A: a native NewTOS `.NCP` console program (native
# AArch64 ELF, prints a banner via Cconws then Pterm). Generated by the
# sibling tool from the same proven ELF encodings the Pexec selftest uses --
# a hand-crafted ELF, NOT the clang build of sdk/examples/hello.c (that one is
# HELLO2.NCP below). Placed on C: so it is double-clickable on the desktop.
#
# It used to be copied into \AUTO\ as well, as the end-to-end launch proof.
# That copy is gone (user decision 2026-09-30, TASK/T13): the launch path has
# been proven many times over since, and every boot paid for a banner nobody
# reads. Since 2026-10-26 \AUTO\ holds exactly one program by default,
# MXPROBE.NCP (see its block below), so that every boot of this image runs
# Malloc/Mxalloc through the trap path and reaches the TASK/MEM11 debug
# assertions; \AUTO\READ.ME above still says what the folder is for, and the
# SD_*_AUTO switches below add more on request.
python3 "$SCRIPT_DIR/mk-hello-ncp.py" "$V/HELLO.NCP"

# Native C SDK Stage 1: HELLO2.NCP is the *clang-built* console program (the
# SDK end-to-end proof), compiled from sdk/examples/hello.c + sdk/crt0.S and
# linked with sdk/newtos.ld into a NewTOS-native ELF. Placed on C: so it is
# double-clickable and runnable via run_program(b"C:\\HELLO2.NCP").
build_example hello.c "$V/HELLO2.NCP"
echo "built + installed C:\\HELLO2.NCP (clang-built native C program)"

# BIG.NCP is the multi-page-TPA proof: a clang-built program larger than a
# single 4-KiB page (8-KiB initialized table + 32-KiB BSS + deep-recursion
# stack + an in-TPA heap). The loader hands it a pool-backed multi-MiB TPA
# and frees it at Pterm; run it via run_program(b"C:\\BIG.NCP") to see its
# computed results on the console.
build_example big.c "$V/BIG.NCP"
echo "built + installed C:\\BIG.NCP (clang-built multi-page-TPA program)"

# PHYSBASE.NCP is the direct-screen-access proof: it calls XBIOS Physbase
# (0x02) / Logbase (0x03), prints both addresses, and stores three colour
# bands straight through the returned pointer with NO cache maintenance --
# which only works because those pages are published Normal Non-Cacheable
# into the process (ADR-023 amendment). Run it via
# run_program(b"C:\\PHYSBASE.NCP"); it blocks on a key so the bands stay up
# for a screendump. Deliberately NOT in AUTO: it would stop every boot.
build_example physbase.c "$V/PHYSBASE.NCP"
echo "built + installed C:\\PHYSBASE.NCP (Physbase direct-screen proof)"

# SCRFLIP.NCP is the double-buffering proof: it finds the second screen page
# through the Setscreen/Logbase contract, paints it while it is invisible,
# waits ~10 s, makes it the physical screen with Setscreen (the VideoCore
# set-virtual-offset tag underneath), waits ~10 s again and puts both bases
# back, so the boot carries on into the desktop. Two screendumps around those
# windows are the proof. It restores everything and exits, so unlike
# PHYSBASE.NCP it is safe in \AUTO\ -- set SD_SCRFLIP_AUTO=1 for that.
build_example scrflip.c "$V/SCRFLIP.NCP"
echo "built + installed C:\\SCRFLIP.NCP (Setscreen page-flip proof)"
if [ "${SD_SCRFLIP_AUTO:-0}" = "1" ]; then
  cp "$V/SCRFLIP.NCP" "$V/AUTO/SCRFLIP.NCP"
  echo "  + \\AUTO\\SCRFLIP.NCP (runs at boot; adds ~20 s)"
fi

# CURDIR.NCP is the working-directory proof (DESK18): it prints the current
# drive + Dgetpath, then opens PROOF.TXT by a BARE RELATIVE NAME and prints
# what it read. Installed in \SUB\ on purpose -- launched from the desktop it
# must read C:\SUB\PROOF.TXT ("PROOF: C:\SUB"), and only does so if the
# launcher set the current directory first. Never in \AUTO\: the AUTO path has
# its own (root) directory and the program is there to distinguish the two.
build_example curdir.c "$V/SUB/CURDIR.NCP"
echo "built + installed C:\\SUB\\CURDIR.NCP (working-directory proof)"

# CMDLINE.NCP is the command-line proof (DESK19): it prints its own
# `p_cmdlin` out of the BasePage the loader maps at VA 0, plus the current
# drive + directory. Dropping a file on its icon is a LAUNCH WITH ARGUMENTS in
# EmuTOS (`fun_win2win`, deskfun.c:958-970), not a copy, and the argument is
# either the full path or the bare file name depending on the desktop
# configuration's *Parameter* setting (`build_selected_path`, :903-928). Put in
# the root so the drag is one gesture inside the C:\ window; never in \AUTO\,
# where it would get an empty tail and say nothing.
#
# The SAME program is installed a second time as CMDLINE.NTP, because `.NTP`
# is the one member of the ADR-018 family that had no file on the card at all
# and so nothing to double-click (DESK41's acceptance asks for one). `.NTP` =
# "takes a command line", so the program that PRINTS the command line it got is
# exactly the right demonstration: started from TeraDesk it first asks for a
# command line (PTTP, sdk/apps/teradesk/applik.c:1390) and then shows the tail
# that arrived; started from our own desktop it shows the empty tail that
# launcher passes today (the parameter dialog is DESK44).
build_example cmdline.c "$V/CMDLINE.NCP"
cp "$V/CMDLINE.NCP" "$V/CMDLINE.NTP"
echo "built + installed C:\\CMDLINE.NCP + C:\\CMDLINE.NTP (command-line proof; .NTP demo)"

# ARGVTEST.NAP is the argc/argv proof (PORT01), the other half of CMDLINE.NCP:
# that one reads `p_cmdlin` itself and so only shows what the KERNEL delivered,
# this one goes through the SDK startup (sdk/libc/crt0.S -> __libc_parseargs)
# like every ported application and shows what the APPLICATION receives. It
# prints both in one run, over Cconws, i.e. to the serial console as well as
# the screen. A `.NAP` on purpose -- `*.NAP` is a GEM-application ANODE
# (programs/desktop/src/inf.rs:678), so dropping a file on it is the same launch
# gesture that starts QED with a document. Root of C:, so the drag is one
# gesture inside the C:\ window; never in \AUTO\ (console-only, ADR-018).
# Built by `sdk/libc/build-libc.sh sdk/examples/argvtest.c \
#            sdk/libc/build/argvtest.NAP`.
cp "$ROOT/sdk/libc/build/argvtest.NAP" "$V/ARGVTEST.NAP"
echo "installed C:\\ARGVTEST.NAP (argc/argv proof)"

# DTACHILD.NCP is the per-process-DTA proof (S1c): a console program that moves
# its OWN DTA (Fsetdta(0x2000)) and exits. A parent runs it between its own
# Fsetdta(buf) and Fgetdta() -- if Fgetdta still returns buf, the DTA really
# lives in the running process's basepage (p_xdta) and not in a global.
# Console-only, so it belongs on C: (double-clickable / Pexec-able), never in
# \AUTO\ as a side effect.
build_example dtachild.c "$V/DTACHILD.NCP"
echo "built + installed C:\\DTACHILD.NCP (per-process DTA test child)"

# PTERMCH.NCP is the Pexec exit-code child (TASK/PROC06): it ends with the exit
# code its one-character command tail selects (m = Pterm(-1), h = Pterm(0x8000),
# z = Pterm0, c = wait in Cconin for a Ctrl-C). APITEST case T33 Pexecs it in
# the m/h/z modes and checks that the parent gets rc & 0xFFFF zero-extended
# (65535 / 32768 / 0). CTRLCPAR.NCP Pexecs it in mode c and prints the Pexec
# result after a Ctrl-C (65504) -- it waits for that key, so it lives on C:
# only, never in \AUTO\.
build_example ptermch.c "$V/PTERMCH.NCP"
echo "built + installed C:\\PTERMCH.NCP (Pexec exit-code test child)"
build_example ctrlcpar.c "$V/CTRLCPAR.NCP"
echo "built + installed C:\\CTRLCPAR.NCP (Ctrl-C'd child -> Pexec rc 65504)"

# FPPEXEC.NCP is the ARCH02 probe: it holds a pattern in all of v0..v31 plus
# FPCR/FPSR across Pexec 0, 3+6 and 3+4 of itself (the child role, command tail
# "k", rewrites all of them) and prints one [OK]/[FAIL] line per round and
# "FPPEXEC: RESULT PASS|FAIL". The child is C:\FPPEXEC.NCP, so it is always
# installed in the root; set SD_FPPEXEC_AUTO=1 to also run it from \AUTO\
# (console only, no key wait) for a serial-verifiable boot.
build_example fppexec.c "$V/FPPEXEC.NCP"
echo "built + installed C:\\FPPEXEC.NCP (FP/SIMD registers across Pexec)"
if [ "${SD_FPPEXEC_AUTO:-0}" = "1" ]; then
  cp "$V/FPPEXEC.NCP" "$V/AUTO/FPPEXEC.NCP"
  echo "  + \\AUTO\\FPPEXEC.NCP (runs at boot)"
fi

# SYSINFO.NCP is the ACC06-part-1 proof: it reads the ARM core clock and the
# SoC die temperature through the two NewTOS-own Ssystem subfunctions (2000 /
# 2001) and prints them -- the evidence that the two values leave the kernel
# and reach an EL0 program, long before the desk accessory that will show them
# (ACC02/ACC03) exists. On a board that cannot measure them it prints the
# ERROR it got; that is a result, not a failure.
# Double-clickable in the root. With SD_SYSINFO_AUTO=1 it additionally goes
# into \AUTO\, which is how a headless QEMU run gets the two lines onto the
# serial console without a click.
build_example sysinfo.c "$V/SYSINFO.NCP"
echo "built + installed C:\\SYSINFO.NCP (Ssystem 2000/2001 clock + temperature)"
if [ "${SD_SYSINFO_AUTO:-0}" = "1" ]; then
  cp "$V/SYSINFO.NCP" "$V/AUTO/SYSINFO.NCP"
  echo "  + \\AUTO\\SYSINFO.NCP (runs at boot, prints to the serial console)"
fi

# USBSNAP.NCP is the USB device recorder (USB01 step 2, docs/api/usb/
# usb-snapshot.md §4): it lists the USB devices the kernel enumerated, records
# 30 s of their reports and writes one text file per device to C:\USB\.
# Double-clickable in the root; NOT in \AUTO\ (it needs the user). On QEMU
# raspi4b (no USB controller) it says so and writes nothing.
build_example usbsnap.c "$V/USBSNAP.NCP"
echo "built + installed C:\\USBSNAP.NCP (Ssystem 2022 USB snapshot recorder)"

# ACC05 / ACC02: the desk accessories. SYSACC.NAC is the accessory half of the
# ACC06 pair (SYSINFO.NCP above is the console half): it registers " System
# Info" under *Desk* and answers AC_OPEN with the SYSINFO01 dialog (clock,
# temperature, memory, protection, SD card, display, keyboard, serial).
#
# Installed in BOTH search locations so one card exercises both halves of the
# two-place scan (docs/api/aes/accessories.md §7.3): C:\ACC\SYSACC.NAC is
# found first, C:\SYSACC2.NAC is the root copy. The names deliberately DIFFER
# -- a same-named file in the root is skipped by the scan's file-name dedupe,
# which is the rule, not a defect; two names prove both locations are read.
# Set SD_ACC_ROOT_ONLY=1 to install only the root copy (the scan then finds
# exactly one accessory, in the second location).
build_example sysacc.c "$V/ACC/SYSACC.NAC"
echo "built + installed C:\\ACC\\SYSACC.NAC (desk accessory, folder location)"
if [ "${SD_ACC_ROOT_ONLY:-0}" = "1" ]; then
  mv "$V/ACC/SYSACC.NAC" "$V/SYSACC2.NAC"
  echo "  moved to C:\\SYSACC2.NAC (SD_ACC_ROOT_ONLY=1: root location only)"
else
  cp "$V/ACC/SYSACC.NAC" "$V/SYSACC2.NAC"
  echo "  + C:\\SYSACC2.NAC (root location; a second, independently loaded copy)"
fi

# libc milestone (mintlib port): LIBTEST.NCP is the mintlib-linked proof — real
# printf(), malloc()/free(), and a stdio file round-trip (fopen/fwrite/fread/
# fclose), linked against the ported mintlib libc.a by sdk/libc/build-libc.sh.
# Built beforehand by `sdk/libc/build-libc.sh` (the 600-object archive build is
# slow, so the script does not run it; a missing file stops it, see REQUIRED);
# double-clickable and runnable via run_program(b"C:\\LIBTEST.NCP")
# (or drop it into \AUTO\ for a serial-verifiable auto-run).
cp "$ROOT/sdk/libc/build/libtest.NCP" "$V/LIBTEST.NCP"
echo "installed C:\\LIBTEST.NCP (mintlib-linked libc proof)"

# 200 Hz system-tick proof: CLOCKTST.NCP measures mintlib's own clock() (which
# reads the `_hz_200` sysvar at $4ba) against GEMDOS Tgettime and prints the
# error in ticks — see sdk/libc/tests/clocktest.c. Built beforehand
# (`sdk/libc/build-libc.sh sdk/libc/tests/clocktest.c`). Installed on C: only;
# set SD_CLOCKTEST_AUTO=1 to also drop it into \AUTO\ so it runs and prints
# its verdict on the console at every boot (costs ~12 s of boot time).
CLOCKTST_NCP="$ROOT/sdk/libc/build/clocktest.NCP"
cp "$CLOCKTST_NCP" "$V/CLOCKTST.NCP"
echo "installed C:\\CLOCKTST.NCP (200 Hz tick / _hz_200 proof)"
if [ "${SD_CLOCKTEST_AUTO:-0}" = "1" ]; then
  cp "$CLOCKTST_NCP" "$V/AUTO/CLOCKTST.NCP"
  echo "  + \\AUTO\\CLOCKTST.NCP (runs at boot)"
fi

# setjmp/longjmp proof (TASK/T07): SETJMPT.NCP runs the checks in
# sdk/libc/tests/setjmp-cases.h against the archive's AArch64 setjmp.S through
# mintlib's <setjmp.h> and prints one PASS/FAIL line per check over Cconws --
# see sdk/examples/setjmptst.c. Built beforehand
# (`sdk/libc/build-libc.sh sdk/examples/setjmptst.c sdk/libc/build/setjmptst.NCP`).
# Set SD_SETJMPTST_AUTO=1 to also drop it into \AUTO\ for a serial-verifiable run.
SETJMPT_NCP="$ROOT/sdk/libc/build/setjmptst.NCP"
cp "$SETJMPT_NCP" "$V/SETJMPT.NCP"
echo "installed C:\\SETJMPT.NCP (setjmp/longjmp proof)"
if [ "${SD_SETJMPTST_AUTO:-0}" = "1" ]; then
  cp "$SETJMPT_NCP" "$V/AUTO/SETJMPT.NCP"
  echo "  + \\AUTO\\SETJMPT.NCP (runs at boot)"
fi

# libm proof (TASK/T07): MATHTST.NCP checks sqrt/sin/pow/floor/frexp/atan2/...
# from libm.a (OpenLibm, sdk/libm/) against closed-form values and prints one
# PASS/FAIL line per check over Cconws -- see sdk/examples/mathtst.c. Built
# beforehand (`sdk/libm/build-libm.sh`). Set SD_MATHTST_AUTO=1 to also
# drop it into \AUTO\.
MATHTST_NCP="$ROOT/sdk/libm/build/mathtst.NCP"
cp "$MATHTST_NCP" "$V/MATHTST.NCP"
echo "installed C:\\MATHTST.NCP (libm proof)"
if [ "${SD_MATHTST_AUTO:-0}" = "1" ]; then
  cp "$MATHTST_NCP" "$V/AUTO/MATHTST.NCP"
  echo "  + \\AUTO\\MATHTST.NCP (runs at boot)"
fi

# Mxalloc pool proof (TASK/MEM02): MXPROBE.NCP asks Mxalloc for every pool mode
# (MX_STRAM/TTRAM/PREFSTRAM/PREFTTRAM), classifies each block against phystop,
# writes/reads it from EL0, checks refusal and fallback, one PASS/FAIL line per
# check over Cconws -- see sdk/libc/tests/mxprobe.c. Built beforehand
# (`sdk/libc/build-libc.sh sdk/libc/tests/mxprobe.c sdk/libc/build/MXPROBE.NCP`).
#
# In \AUTO\ BY DEFAULT (user decision 2026-10-26): it is the boot-time caller of
# Mxalloc through the trap path, so a boot of a debug kernel on this image
# runs the TASK/MEM11 assertion on every returned block
# (tos-api/src/gemdos/memory.rs `pointer_result`). It ends by itself and waits
# for no key. SD_MXPROBE_AUTO=0 leaves it out of \AUTO\ (C:\MXPROBE.NCP stays).
MXPROBE_NCP="$ROOT/sdk/libc/build/MXPROBE.NCP"
cp "$MXPROBE_NCP" "$V/MXPROBE.NCP"
echo "installed C:\\MXPROBE.NCP (Mxalloc pool proof)"
if [ "${SD_MXPROBE_AUTO:-1}" = "1" ]; then
  cp "$MXPROBE_NCP" "$V/AUTO/MXPROBE.NCP"
  echo "  + \\AUTO\\MXPROBE.NCP (runs at boot; SD_MXPROBE_AUTO=0 leaves it out)"
fi

# LDG proof (TASK/LDG-M1): EX_LIB.NLD is LDG's unmodified demo library ex_lib
# built as a native LDG module (`.NLD`, ADR-018 addendum: not startable by
# double-click -- the desktop shows it as a document); LDGTEST.NCP is the
# headless client that loads it through libldg's real mechanism (ldg_open ->
# Pexec 5/3/4), calls sinus()/cosinus() and closes it, one PASS/FAIL line per
# check over Cconws -- see sdk/examples/ldgtest.c. The client opens the
# module as C:\LDG\EX_LIB.NLD and by the relative name after Dsetpath(\LDG),
# so the module must sit exactly there. Built beforehand
# (`sdk/ldg/build-ldg-demo.sh`). Set SD_LDGTEST_AUTO=1 to also drop the client
# into \AUTO\ for a serial-verifiable run (no key pause).
LDGTEST_NCP="$ROOT/sdk/ldg/build/LDGTEST.NCP"
mkdir -p "$V/LDG"
cp "$ROOT/sdk/ldg/build/EX_LIB.NLD" "$V/LDG/EX_LIB.NLD"
cp "$LDGTEST_NCP" "$V/LDGTEST.NCP"
echo "installed C:\\LDG\\EX_LIB.NLD + C:\\LDGTEST.NCP (LDG proof)"
if [ "${SD_LDGTEST_AUTO:-0}" = "1" ]; then
  cp "$LDGTEST_NCP" "$V/AUTO/LDGTEST.NCP"
  echo "  + \\AUTO\\LDGTEST.NCP (runs at boot)"
fi

# zView (TASK/ZVIEW-M4) — the z-tools image viewer, LDG version, built from unchanged upstream sources plus per-program patch files (sdk/zview/overlay/)
# apart from its toolchain overlays (sdk/zview/build-zview.sh,
# docs/sdk/13-zview.md), WITHOUT the PDF module. C:\ZVIEW\ holds ZVIEW.NAP,
# ZVIEW.RSC, ICONS\*.GOD (the browser icons, read through the GodPaint codec)
# and CODECS\*.NLD (ten native LDG codecs: bmp degas eureka gif godpaint img jpg
# neo png tga) -- zView looks for "codecs" beside itself (plugins.c:56-60).
# C:\ZVIEW\SAMPLES\ gets one synthesised sample per codec path
# (sdk/zview/samples/gen-samples.py). Start ZVIEW.NAP by double-click (opens
# with its menu bar; File > Open), or with a file argument (main.c:164-165).
# The sample images stay optional: build-zview.sh itself treats a failing
# gen-samples.py (no Pillow) as a note, not an error. Their absence is reported
# as `optional missing` and recorded in the manifest.
ZV_DIST="$ROOT/sdk/zview/build/dist/ZVIEW"
mkdir -p "$V/ZVIEW"
cp -R "$ZV_DIST/" "$V/ZVIEW/"
if [ -d "$ROOT/sdk/zview/build/samples/IMAGES" ]; then
  mkdir -p "$V/ZVIEW/SAMPLES"
  cp "$ROOT/sdk/zview/build/samples/IMAGES/"* "$V/ZVIEW/SAMPLES/"
  echo "installed C:\\ZVIEW\\ (zView: ZVIEW.NAP, ZVIEW.RSC, ICONS\\, CODECS\\*.NLD, SAMPLES\\)"
else
  echo "installed C:\\ZVIEW\\ (zView: ZVIEW.NAP, ZVIEW.RSC, ICONS\\, CODECS\\*.NLD)"
  optional_missing 'C:\ZVIEW\SAMPLES\' \
    "sdk/zview/build/samples/IMAGES absent (gen-samples.py did not run; needs Python with Pillow) -- build it with: sdk/zview/build-zview.sh"
fi

# CoreMark 1.0 (EEMBC) — the CPU benchmark, built UNMODIFIED apart from its own
# porting layer (sdk/overlays/coremark/newtos/), which is what CoreMark's run
# rules put under the porter's control. Two executables, because NewTOS has no
# way to pass a program seeds on a command line: CORE.NCP is the performance run
# (seeds 0,0,0x66 — this is the one whose "CoreMark 1.0 : …" line is the
# reportable score), COREVAL.NCP the validation run (0x3415,0x3415,0x66) that the
# run rules also require to pass. Built by sdk/apps/build-coremark.sh.
#
# They go into their own folder C:\COREMARK\ and are started by DOUBLE-CLICKING
# CORE.NCP on the desktop: `.NCP` gets the VT52 text screen. A run takes ~30 s
# during which the screen is empty by design (the benchmark must run for at
# least 10 s uninterrupted); the program prints a banner saying so first.
#
# It does NOT hold its result on the screen. This comment used to promise a
# desktop-side "[ Press any key to return to the desktop ]" pause; that pause
# was built in 0c6e4ef and deliberately removed again in e0069e5 (DESK43 U3 /
# STOP-3 of docs/analysis/desktop-userspace-plan.md:614,1184 -- EmuTOS'
# `pro_run`, desk/deskpro.c:51-63, returns at once and has no such pause), so
# the promise has been wrong since. Nothing holds the screen for it either way:
# CoreMark is a third-party benchmark built UNMODIFIED, so its score has to be
# read off the serial log.
#
# NOT in \AUTO\ by default: an AUTO run finishes before the AES starts and the
# desktop then paints over the result. Set SD_COREMARK_AUTO=1 to place it there
# anyway — that is the only way to drive it under QEMU raspi4b, which emulates
# the SD controller but no keyboard/mouse, so nothing can be double-clicked; the
# report then has to be read off the serial log.
CM_DIR="$ROOT/sdk/apps/coremark/build"
mkdir -p "$V/COREMARK"
cp "$CM_DIR/CORE.NCP"    "$V/COREMARK/CORE.NCP"
cp "$CM_DIR/COREVAL.NCP" "$V/COREMARK/COREVAL.NCP"
echo "installed C:\\COREMARK\\{CORE,COREVAL}.NCP (EEMBC CoreMark 1.0)"
if [ "${SD_COREMARK_AUTO:-0}" = "1" ]; then
  cp "$CM_DIR/CORE.NCP"    "$V/AUTO/CORE.NCP"
  cp "$CM_DIR/COREVAL.NCP" "$V/AUTO/COREVAL.NCP"
  echo "  + \\AUTO\\{CORE,COREVAL}.NCP (both run at boot; ~1 min extra)"
fi

# QED (Atari GEM editor) — the first ported GEM application, compiled from unchanged upstream sources plus patch files
# for NewTOS/AArch64 as **native LP64** (the app-ABI pivot: apps match the
# kernel's native 64-bit ABI, so the app<->kernel width mismatch vanishes). Its
# two 32-bit-assuming files (the private pool allocator memory.c and the bit-set
# module set.c) are replaced by sdk/overlays/qed/{memory.c,set.c} at build time
# — QED's src/ stays pristine. Installed into its own app dir C:\Apps\QED\
# (classic GEM apps expect their .RSC beside the .NAP): QED.NAP is the GEM
# program, qed.rsc its native NRSC (LP64) resource, both produced by
# sdk/apps/build-qed.sh (the .rsc via `tools/mkrsc.py convert`, staged into
# build/ after the link — docs/sdk/04-porting-anatomy.md). Launched by
# double-clicking QED.NAP on the desktop — NEVER from \AUTO\ (console-only,
# ADR-018). Built beforehand (`sdk/apps/build-qed.sh`).
# ── TOSDUMP: the VDI reference dumper (docs/TOS-REFERENCE-DUMPER.md) ─────────
# DUMPVDI.NAP measures what this machine's VDI does, case by case, so it can be
# compared against the same measurement taken on an Atari TT (the dumps live in
# tosdump/reference/). It is a `.NAP` because it calls appl_init/graf_handle/
# v_opnvwk, so it launches from the desktop -- never from \AUTO\.
#
# Both files go in the ROOT of C: on purpose. The dumper reads its case table
# and writes its output through RELATIVE paths, and the desktop does not set
# the current directory before launching a program (programs/desktop/src/main.rs,
# `launch_native` -- no Dsetpath), so the working directory is whatever the
# boot left it at. The root is the one place that is likely to be.
TOSDUMP_DIR="$ROOT/tosdump"
cp "$TOSDUMP_DIR/build/DUMPVDI.NAP" "$V/DUMPVDI.NAP"
# Round 2: 120 cases, the broadest opcode coverage, and the reference the
# generated Rust tests are built against. Copied verbatim -- its bytes are
# the identity the dump carries, so the CRC must stay A0706A6D.
cp "$TOSDUMP_DIR/cases/vdi-r2.tsv" "$V/VDI.TSV"
echo "  + C:\\DUMPVDI.NAP + C:\\VDI.TSV (VDI reference dumper, round 2)"
# VDIDRAW draws every output primitive into a numbered, captioned cell. The
# dumper cannot see those: the drawing opcodes leave the parameter block
# empty, so a matching dump proves the block survived, not that a pixel
# landed where it should. This is the visual half, and it needs no case table.
# Built by tosdump/build-vdidraw-newtos.sh (no Makefile target).
cp "$TOSDUMP_DIR/build/VDIDRAW.NAP" "$V/VDIDRAW.NAP"
echo "  + C:\\VDIDRAW.NAP (visual VDI primitive sheet)"

# ── The bouncing-ball demo is no longer built from this tree ───────────────
# BALL.NAP and its texture BALL.NTX used to be installed here. The demo is an
# APPLICATION, not part of the operating system -- it also has an Atari TT and
# a PiStorm half that have nothing to do with NewTOS at all -- so on 2026-09-09
# it moved out to /Volumes/Daten/Development/ball-demo/ together with its
# artwork and its texture converter (formerly tools/png2ntx.py).
#
# To put it back on a card, build it there and copy the products in by hand:
#
#   NEWTOS_SDK=<this checkout>/sdk /Volumes/Daten/Development/ball-demo/build-ball.sh
#   cp /Volumes/Daten/Development/ball-demo/{BALL.NAP,BALL.NTX} "$V/"
#
# It is a `.NAP` (it opens a VDI workstation), so it is DOUBLE-CLICKED on the
# desktop, never run from \AUTO\. For a headless QEMU run the demo's own
# notes describe the EmuTOS `#Z` autorun line in C:\NEWDESK.INF.
QED_DIR="$ROOT/sdk/apps/qed/build"
mkdir -p "$V/Apps/QED"
cp "$QED_DIR/QED.NAP" "$V/Apps/QED/QED.NAP"
cp "$QED_DIR/qed.rsc" "$V/Apps/QED/qed.rsc"
echo "installed C:\\Apps\\QED\\QED.NAP + qed.rsc (ported GEM editor, LP64)"

# TeraDesk 4.08 (GEM desktop replacement) — the second ported GEM application,
# compiled from unchanged upstream sources plus seven patch files
# (sdk/overlays/teradesk/, e.g. nf_ops.c NatFeats-not-present retarget,
# window.h WDFLAGS 2-byte fix, main.c reboot widths) as native LP64. Its dist/ is staged by
# sdk/apps/build-teradesk.sh: DESKTOP.NAP (the GEM program), desktop.rsc
# (NRSC/LP64 for the kernel rsrc_load), icons.rsc (native-RSHDR for TeraDesk's
# OWN xrsrc_load parser — NOT kernel-loadable), and cicons.rsc (the colour
# icons, NRSC v13/LP64 for the kernel rsrc_load — an image that begins with a
# native RSHDR, which TeraDesk's load_icons parses on its own after the AES
# loaded it, TASK/A41; TeraDesk tries it first and falls back to the mono
# icons.rsc when it is absent). cicons.rsc is MANDATORY (REQUIRED list, user
# decision 2026-10-26): with or without it TeraDesk renders a different
# desktop, so an image without it is not the measured configuration. A dist/
# staged before A41 part 2 has none -- rerun sdk/apps/build-teradesk.sh.
# Launched by double-clicking DESKTOP.NAP — NEVER from \AUTO\ (ADR-018).
TERA_DIR="$ROOT/sdk/apps/teradesk/build/dist"
mkdir -p "$V/Apps/TERADESK"
cp "$TERA_DIR/DESKTOP.NAP" "$V/Apps/TERADESK/DESKTOP.NAP"
cp "$TERA_DIR/desktop.rsc" "$V/Apps/TERADESK/desktop.rsc"
cp "$TERA_DIR/icons.rsc"   "$V/Apps/TERADESK/icons.rsc"
cp "$TERA_DIR/cicons.rsc"  "$V/Apps/TERADESK/cicons.rsc"
echo "installed C:\\Apps\\TERADESK\\{DESKTOP.NAP,desktop.rsc,icons.rsc,cicons.rsc} (ported GEM desktop, LP64, colour icons)"

# PRESENT.NAP (TASK/PRESENT01) — our own full-screen slide show
# (sdk/apps/present/, docs/sdk/14-present.md). C:\Apps\PRESENT\ gets the
# program, the SAMPLE slide file SLIDES.TXT and its test picture TEST.PNG
# (sdk/apps/present/build-present.sh writes build/PRESENT.NAP + build/TEST.PNG).
# QEMU image only: the slides for a real card are copied there by hand.
# Started by double-click (or #Z); it reads SLIDES.TXT from its folder.
PRES_DIR="$ROOT/sdk/apps/present"
mkdir -p "$V/Apps/PRESENT"
cp "$PRES_DIR/build/PRESENT.NAP" "$V/Apps/PRESENT/PRESENT.NAP"
cp "$PRES_DIR/sample/SLIDES.TXT" "$V/Apps/PRESENT/SLIDES.TXT"
cp "$PRES_DIR/build/TEST.PNG"    "$V/Apps/PRESENT/TEST.PNG"
echo "installed C:\\Apps\\PRESENT\\{PRESENT.NAP,SLIDES.TXT,TEST.PNG} (slide show + sample slides)"

# ── TERADESK.INF: TeraDesk's own configuration — GENERATED here (DESK41) ────
# TeraDesk does NOT know the NewTOS extensions by itself: its built-in defaults
# (`prg_default`, sdk/apps/teradesk/prgtype.c:344-358) register *.PRG, *.APP,
# *.GTP, *.TOS, *.TTP and nothing else, so without this file a `.NAP` is a
# document — no program icon, no launch on double-click. The registration lives
# in the `apptypes` section as one `ptype` block per mask, and it used to exist
# only as a hand-kept file on the card, where the next re-flash deleted it.
# It is written here so every image carries it.
#
# The two numbers per block, from TeraDesk's own source (read-only reference):
#   appt = ApplType (desk.h:130-137): PGEM=0, PGTP=1, PACC=2, PTOS=3, PTTP=4.
#          A zero value is omitted by TeraDesk's own writer (CFGEMP=0,
#          library/xdialog/library.h:145), which is why *.NAP has no `appt`.
#   flag = PRGTYPE.flags (prgtype.h:42-57): PT_ARGV 0x0001 "understands ARGV",
#          PD_PDIR 0x2000 "program directory is the default".
# The four values are TeraDesk's own defaults for the m68k extension each
# native one replaces (prgtype.c:346-347 `pt[]`/`dd[]`), per ADR-018's table:
#   *.NAP (GEM app, = .PRG/.APP) appt 0 PGEM, flag 0x2001 PD_PDIR|PT_ARGV
#   *.NCP (console,  = .TOS)     appt 3 PTOS, flag 0x2000 PD_PDIR
#   *.NTP (takes a command line, = .TTP) appt 4 PTTP, flag 0x2000 PD_PDIR
#         PTTP is what makes TeraDesk ask for a command line before it starts
#         the program (applik.c:1390) and run it on the text screen
#         (startprg.c:475 `appl_type = (PGEM||PGTP) ? 1 : 0`, :286-291
#         `v_enter_cur`) — exactly ADR-018's "takes params, console arm".
#         NOT PT_ARGV: TeraDesk's own .TTP default has no ARGV bit either, and
#         with it set the tail is signalled as ARGV (length byte 127,
#         startprg.c:654-655) instead of being passed as a plain tail.
#   *.NAC (accessory, = .ACC)    appt 2 PACC, flag 0x2000 PD_PDIR
#         PACC is also what makes a double-click REFUSE to start it in
#         single-TOS (startprg.c:643-645 `alert_iprint(MCANTACC)`) — the same
#         "an accessory is not launched on demand" rule ADR-018/ACC02 states
#         for our own desktop. This build is single-TOS (-D_TOS_ONLY in
#         sdk/apps/build-teradesk.sh:101 -> _MINT_ = 0).
#
# Why the whole file and not just the `apptypes` section: TeraDesk reads the
# configuration as ONE document. A missing section is not defaulted (its
# handler simply never runs; Config_table is main.c:984-998), and a missing
# final `end` token makes the load fail altogether (config.c:453,504-514),
# whereupon load_options() throws everything away and uses built-in defaults
# (main.c:1002-1027) — the .NAP registration with it. The body below is
# therefore a complete, TeraDesk-written configuration: the one recovered from
# the card (measurements/hw-round-2026-09-24/TERADESK.INF; `infv=0x0400` is the current
# CFG_VERSION, version.h:24) with the two new `ptype` blocks added.
#
# Both locations are written on purpose. TeraDesk resolves "teradesk.inf"
# through shel_find (main.c:1111-1132, xfilesys.c:1284-1311), and shel_find
# looks in the APPLICATION DIRECTORY first and only then in the current
# directory and the root of the current drive (EmuTOS `findfile`,
# aes/gemshlib.c:406-470; ours: tos-api/src/aes/shell.rs `sh_find`). The copy
# beside DESKTOP.NAP is the one it finds and saves back to; the copy in the
# root is what the card had, and what a DESKTOP.NAP started from somewhere
# else on C: still finds.
cat > "$V/TERADESK.INF" <<'TERAINF'
TeraDesk-inf

; Avoid editing this file by hand !!!
; Generated by tools/make-sd-image.sh (NewTOS, DESK41) -- change the script,
; not this file.  It registers the ADR-018 native extensions with TeraDesk:
; *.NAP GEM app, *.NCP console, *.NTP takes a command line, *.NAC accessory.
; Saving settings from inside TeraDesk rewrites the file and drops these
; comment lines; the apptypes entries below survive.

options=
{
	infv=0x0400
	save=0x0000
	dial=0x0001
	xpre=0x0000
	pref=0x101F
	buff=512
	maxd=256
	plin=80
	tabs=8
	cwin=0
	mode=1
	sort=0
	aarr=1
	attr=0x0014
	flds=0x001F
	vidp=0x0000
	vres=0
	dpat=8
	dcol=3
	wpat=1
	wcol=0
}

shortcuts=
{
	open=0x024F
	show=0x0249
	srch=0x0246
	prin=0x0250
	dele=0x027F
	sela=0x0241
	quit=0x0251
	cycl=0x0257
	svop=0x0253
}

filetypes=
{
	ftype=
	{
		mask=*
	}
	ftype=
	{
		mask=*.*
	}
	ftype=
	{
		mask=*.PRG
	}
	ftype=
	{
		mask=*.APP
	}
	ftype=
	{
		mask=*.GTP
	}
	ftype=
	{
		mask=*.TOS
	}
	ftype=
	{
		mask=*.TTP
	}
	ftype=
	{
		mask=*.ACC
	}
	ftype=
	{
		mask=*.TXT
	}
	ftype=
	{
		mask=*.IMG
	}
}

apptypes=
{
	ptype=
	{
		name=*.PRG
		flag=0x2001
	}
	ptype=
	{
		name=*.APP
		flag=0x2001
	}
	ptype=
	{
		name=*.GTP
		appt=1
		flag=0x2001
	}
	ptype=
	{
		name=*.TOS
		appt=3
		flag=0x2000
	}
	ptype=
	{
		name=*.TTP
		appt=4
		flag=0x2000
	}
	ptype=
	{
		name=*.NAP
		flag=0x2001
	}
	ptype=
	{
		name=*.NCP
		appt=3
		flag=0x2000
	}
	ptype=
	{
		name=*.NTP
		appt=4
		flag=0x2000
	}
	ptype=
	{
		name=*.NAC
		appt=2
		flag=0x2000
	}
}

icontypes=
{
}

deskicons=
{
	iconw=80
	iconh=46
	icon=
	{
		name=HARD@DISC
		labl=HARD@DISC
		type=1
		driv=2
		xpos=2
	}
	icon=
	{
		name=HARD@DISC
		labl=HARD@DISC
		type=1
		driv=3
		xpos=3
	}
	icon=
	{
		name=HARD@DISC
		labl=HARD@DISC
		type=1
		driv=4
		xpos=4
	}
	icon=
	{
		name=HARD@DISC
		labl=HARD@DISC
		type=1
		driv=5
		xpos=5
	}
	icon=
	{
		name=TRASH
		labl=TRASH
		type=2
		ypos=2
	}
}

applications=
{
}

windows=
{
	directories=
	{
		font=
		{
			iden=1
			size=10
			fcol=1
		}
		pos=
		{
			xpos=303
			ypos=123
			winw=103
			winh=38
		}
		pos=
		{
			xpos=496
			ypos=51
			winw=122
			winh=37
		}
		pos=
		{
			xpos=512
			ypos=67
			winw=124
			winh=37
		}
		pos=
		{
			xpos=528
			ypos=83
			winw=126
			winh=37
		}
		pos=
		{
			xpos=544
			ypos=99
			winw=128
			winh=37
		}
		pos=
		{
			xpos=560
			ypos=115
			winw=130
			winh=37
		}
		pos=
		{
			xpos=576
			ypos=131
			winw=132
			winh=37
		}
		pos=
		{
			xpos=592
			ypos=147
			winw=134
			winh=37
		}
	}
	views=
	{
		font=
		{
			iden=1
			size=10
			fcol=1
		}
		pos=
		{
			xpos=240
			ypos=39
			winw=180
			winh=29
		}
		pos=
		{
			xpos=248
			ypos=71
			winw=182
			winh=29
		}
		pos=
		{
			xpos=256
			ypos=103
			winw=182
			winh=29
		}
		pos=
		{
			xpos=264
			ypos=135
			winw=184
			winh=29
		}
		pos=
		{
			xpos=272
			ypos=167
			winw=184
			winh=29
		}
		pos=
		{
			xpos=280
			ypos=199
			winw=186
			winh=29
		}
		pos=
		{
			xpos=288
			ypos=231
			winw=186
			winh=29
		}
		pos=
		{
			xpos=296
			ypos=263
			winw=188
			winh=29
		}
	}
}

avstats=
{
}

end

TERAINF
cp "$V/TERADESK.INF" "$V/Apps/TERADESK/TERADESK.INF"
echo "generated C:\\TERADESK.INF + C:\\Apps\\TERADESK\\TERADESK.INF (registers *.NAP/*.NCP/*.NTP/*.NAC)"

# Strip macOS metadata so it does not clutter the directory listing.
sync
rm -rf "$V/.Spotlight-V100" "$V/.fseventsd" "$V/.Trashes" "$V/.TemporaryItems" 2>/dev/null || true
dot_clean -m "$V" 2>/dev/null || true
sync

list_card_files "$V" C
diskutil unmount "${DEV}s1" >/dev/null
cleanup
# The device is detached; from here a failure only has to take the partial
# manifest with it (the image itself stays, as before, for inspection).
trap 'rc=$?; [ "$rc" -eq 0 ] || rm -f "$MANIFEST_FILES"' EXIT

# Sanity-check the MBR: partition 0 must be type 0x06 (FAT16), 0x55AA present.
python3 - "$IMG" ${SD_FSBENCH_D:+fsbench} <<'PY'
import sys, struct
d = open(sys.argv[1], 'rb').read(512)
assert d[510] == 0x55 and d[511] == 0xAA, "missing 0x55AA MBR signature"
typ = d[0x1BE + 4]
start = struct.unpack('<I', d[0x1BE + 8:0x1BE + 12])[0]
count = struct.unpack('<I', d[0x1BE + 12:0x1BE + 16])[0]
assert typ == 0x06, f"partition type 0x{typ:02x}, expected 0x06 (FAT16)"
print(f"OK: MBR FAT16 partition type=0x{typ:02x} start_lba={start} sectors={count}")
if len(sys.argv) > 2:
    typ2 = d[0x1CE + 4]
    start2 = struct.unpack('<I', d[0x1CE + 8:0x1CE + 12])[0]
    count2 = struct.unpack('<I', d[0x1CE + 12:0x1CE + 16])[0]
    assert typ2 == 0x06, f"D: partition type 0x{typ2:02x}, expected 0x06 (FAT16)"
    print(f"OK: MBR FAT16 partition 2 (D:) type=0x{typ2:02x} start_lba={start2} sectors={count2}")
PY

# Assemble the manifest (format v1: tools/sd-manifest.sh) from MANIFEST_FILES,
# the optional_missing entries and the SD_* switches in the environment; the
# writer deletes MANIFEST_FILES.
om_args=()
for m in ${OPTIONAL_MISSING[@]+"${OPTIONAL_MISSING[@]}"}; do om_args+=(--optional-missing "$m"); done
"$SCRIPT_DIR/sd-manifest.sh" write "$IMG" "$SIZE_MB" tools/make-sd-image.sh \
  ${om_args[@]+"${om_args[@]}"}
trap - EXIT

echo "done: ${IMG}"
if [ "${#OPTIONAL_MISSING[@]}" -eq 0 ]; then
  echo "optional missing: none"
else
  for m in "${OPTIONAL_MISSING[@]}"; do echo "optional missing: $m"; done
fi
echo "manifest: ${MANIFEST} ($(sed -n '/^file /p' "$MANIFEST" | wc -l | tr -d ' ') files; copy it into the measurement run, RUNINFO.TXT cites it)"
echo "run:  cargo xtask run --sd ${IMG} --board rpi4 --window   # raspi4b emulates the SD controller"
