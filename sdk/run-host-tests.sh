#!/usr/bin/env bash
#
# run-host-tests.sh -- build and run every SDK host test in one pass.
#
# These are HOST programs (macOS/arm64 or any LP64 little-endian host): they
# compile a gemlib binding or an arch shim with the host `cc`, call it, and
# check the words/bytes/registers it produced -- no kernel, no QEMU. Each test
# refuses to certify from a host whose word width or byte order differs from
# the AArch64 target. Until TASK/T06 they hung in no script; each file's
# header carries its own build line, and this runner is those lines, verbatim
# in substance, in one place.
#
# Prerequisites: sdk/gemlib/build-gemlib.sh has extracted gemlib-0.44.0 and
# applied the overlay (the tests compile the overlaid bindings); for
# aesglobal_fields the vendored TeraDesk source (sdk/apps/teradesk) must be
# present. Missing prerequisites SKIP that test with a line saying why; a
# failing test FAILS the run.
#
# Usage:  sdk/run-host-tests.sh
# Exit status: the number of failed tests (0 = all passed). Skips do not fail.
set -uo pipefail

SDK="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"          # .../sdk
ROOT="$(cd "$SDK/.." && pwd)"                                  # repository root
CC="${CC:-cc}"
GT="$SDK/gemlib/gemlib-0.44.0"                                 # overlaid gemlib tree
MINT="$SDK/libc/mintlib/include"
TESTS="$SDK/gemlib/tests"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

pass=0; fail=0; skip=0
report() {                       # report <name> <rc> [<skip reason>]
  case "$2" in
    0)  echo "PASS  $1"; pass=$((pass+1)) ;;
    77) echo "SKIP  $1 -- $3"; skip=$((skip+1)) ;;
    *)  echo "FAIL  $1 (rc=$2)"; fail=$((fail+1)) ;;
  esac
}

# run_c <name> <cc flags...> -- build $TESTS/<name>.c (+ the listed gemlib
# sources) with the given flags, run it, keep its output in $WORK/<name>.log
# and show it only on failure.
run_c() {
  local name=$1; shift
  local exe="$WORK/$name" log="$WORK/$name.log"
  if ! "$CC" "$@" -o "$exe" > "$log" 2>&1; then
    echo "FAIL  $name (did not build)"; sed 's/^/      /' "$log"; fail=$((fail+1)); return
  fi
  "$exe" >> "$log" 2>&1; local rc=$?
  [ $rc -eq 0 ] || sed 's/^/      /' "$log"
  report "$name" $rc
}

command -v "$CC" >/dev/null 2>&1 || { echo "error: $CC not found" >&2; exit 1; }
if [ ! -f "$GT/mt_gem.h" ]; then
  echo "error: overlaid gemlib tree missing ($GT/mt_gem.h); run sdk/gemlib/build-gemlib.sh" >&2
  exit 1
fi
cd "$ROOT" || exit 1
echo "host: $(uname -m) $(uname -s); cc: $($CC --version | head -1)"
echo

# --- gemlib host tests (sdk/gemlib/tests/<name>.c, build line in each header) --
# obspec_bits: the BFOBSPEC bit-field overlay (mt_gem.h) matches the GEM
# ob_spec layout. Header-only; needs mintlib's include dir for <compiler.h>.
run_c obspec_bits -I "$GT" -I "$MINT" "$TESTS/obspec_bits.c"

# evnt_timer_words (H03): mt_evnt_timer hands int_in[0]=low, int_in[1]=high.
run_c evnt_timer_words -D__NEWTOS__ -I "$GT" -I "$MINT" \
  "$TESTS/evnt_timer_words.c" "$GT/a_evnt_timer.c"

# wind_set_addrin (A81): wind_set_str/_ptr/_ptr_int pass the pointer in
# addr_in[0] with AIN_LEN 1.
run_c wind_set_addrin -D__NEWTOS__ -I "$GT" -I "$MINT" \
  "$TESTS/wind_set_addrin.c" \
  "$GT/a_wind_set_str.c" "$GT/a_wind_set_ptr.c" "$GT/a_wind_set_ptr_int.c"

# intin_long_words (T04): *_intin_long/*_intout_long are 32-bit [lo, hi] in
# place; wind_get_ptr(WF_SCREEN) reads the LONG. Under AddressSanitizer so a
# binding writing 8 bytes into its WORD array is caught, not just miscounted.
run_c intin_long_words -D__NEWTOS__ -fsanitize=address -O1 -I "$GT" -I "$MINT" \
  "$TESTS/intin_long_words.c" \
  "$GT/n_vst_arbpt32.c" "$GT/n_v_create_ctab.c" "$GT/n_vst_track_offset.c" \
  "$GT/n_v_value2color.c" "$GT/a_wind_get_ptr.c" "$GT/vdi_dummy.c"

# ptr_word_count (T04): the six pointer-in-intin bindings declare N_PTRINTS
# WORDs for a pointer.
run_c ptr_word_count -D__NEWTOS__ -I "$GT" -I "$MINT" \
  "$TESTS/ptr_word_count.c" \
  "$GT/n_v_delete_ctab.c" "$GT/n_v_delete_itab.c" "$GT/n_vqt_fontheader.c" \
  "$GT/u_vqt_fontheader.c" "$GT/a_wind_get.c" "$GT/a_wind_sget.c" \
  "$GT/vdi_array2str.c" "$GT/vdi_userdef.c" "$GT/vdi_dummy.c"

# aesglobal_fields (A60): TeraDesk's library.h + the overlay stack address
# ap_ptree/ap_rscmem as 4-byte fields at byte 10/14. Needs the vendored
# TeraDesk source.
if [ -f "$SDK/apps/teradesk/library/xdialog/library.h" ]; then
  run_c aesglobal_fields -D__NEWTOS__ -D_TOS_ONLY -funsigned-char -Wno-macro-redefined \
    -I "$SDK/overlays/teradesk" -I "$SDK/overlays/teradesk/library/xdialog" \
    -I "$SDK/apps/teradesk" -I "$SDK/apps/teradesk/library/xdialog" \
    -I "$SDK/apps/teradesk/library/utility" \
    -I "$GT" -I "$MINT" "$TESTS/aesglobal_fields.c"
else
  report aesglobal_fields 77 "vendored TeraDesk source absent (sdk/apps/teradesk)"
fi

# --- libc host tests -----------------------------------------------------------
# setjmp-host (T07): newtos-arch/setjmp.S round-trips on an AArch64 host; the
# script itself skips (77) on a non-AArch64 host.
"$SDK/libc/tests/setjmp-host.sh" > "$WORK/setjmp-host.log" 2>&1; rc=$?
[ $rc -eq 0 ] || [ $rc -eq 77 ] || sed 's/^/      /' "$WORK/setjmp-host.log"
report setjmp-host $rc "host is not AArch64"

# cmdargs-host (PORT01): newtos-arch/cmdargs.c turns the BasePage command tail
# into the argc/argv mintlib's crtinit.c parseargs() would -- the two DESK19
# tails, the separator rules, MagX quoting, and the densest 128-byte tail;
# since PORT10 also the published empty `environ` and _spawnve's 0x7f tail
# with a p_env that must not be read.
"$SDK/libc/tests/cmdargs-host.sh" > "$WORK/cmdargs-host.log" 2>&1; rc=$?
[ $rc -eq 0 ] || [ $rc -eq 77 ] || sed 's/^/      /' "$WORK/cmdargs-host.log"
report cmdargs-host $rc "mintlib headers absent or host not LP64"

# gmp-host (PORT10): mintlib's GMP limb width matches mp_limb_t on LP64 --
# the 10^(2^i) table __printf_fp reads, __mpn_extract_double, mpn_mul_1
# (the overlaid stdlib/gmp-mparam.h; with the pristine one 17 of 21 fail).
"$SDK/libc/tests/gmp-host.sh" > "$WORK/gmp-host.log" 2>&1; rc=$?
[ $rc -eq 0 ] || [ $rc -eq 77 ] || sed 's/^/      /' "$WORK/gmp-host.log"
report gmp-host $rc "mintlib tree absent or host not LP64"

# spawn-host (PORT10): the LP64 spawn* overlays hand _spawnve the argv/envp
# the m68k va_list cast did (spawnl/spawnle/spawnlp/spawnv/spawnvp).
"$SDK/libc/tests/spawn-host.sh" > "$WORK/spawn-host.log" 2>&1; rc=$?
[ $rc -eq 0 ] || [ $rc -eq 77 ] || sed 's/^/      /' "$WORK/spawn-host.log"
report spawn-host $rc "mintlib tree absent or host not LP64"

# system-host (PORT10, PROC01): mintlib's own posix/system.c (no overlay since
# PROC01) with no /bin/sh and no $SHELL takes the TOS branch -- word
# splitting, < > >> via Fopen/Fcreate + Fdup/Fforce around spawnvp(P_WAIT),
# DOOM's zenity probe -> -1; the object carries the POSIX branch
# (vfork/execve/waitpid), which the target runs.
"$SDK/libc/tests/system-host.sh" > "$WORK/system-host.log" 2>&1; rc=$?
[ $rc -eq 0 ] || [ $rc -eq 77 ] || sed 's/^/      /' "$WORK/system-host.log"
report system-host $rc "mintlib tree absent or host not LP64"

# quickstat-host (PORT10, user decision 2026-10-02): the overlay
# mintlib/quickstat.c sets the exec bits for .NTP/.NAP/.NCP in any case and no
# longer for mintlib's m68k .app/.gtp/.ttp/.prg/.tos (nor .NAC); access(X_OK)
# through newtos-arch/access.c agrees. With the pristine file 15 of 27 fail.
"$SDK/libc/tests/quickstat-host.sh" > "$WORK/quickstat-host.log" 2>&1; rc=$?
[ $rc -eq 0 ] || [ $rc -eq 77 ] || sed 's/^/      /' "$WORK/quickstat-host.log"
report quickstat-host $rc "mintlib tree absent or host not LP64"

# fstat-host (FS08 part 4, user decision 2026-10-07): newtos-arch/do_fstat.c
# gives a regular file (GEMDOS handle >= 6) st_blksize 32 KiB, so mintlib's
# own stdio/stdio_init.c picks a 32 KiB stream buffer; the console (fds 0-2),
# the other standard handles and BIOS devices keep BUFSIZ (1024). With the
# previous shim 6 of 25 fail. Since TASK/SDK01 F also fstat()'s timestamps
# (Fdatime on a file, time() - 2 otherwise); with the shim before SDK01 F
# 8 of 119 fail.
"$SDK/libc/tests/fstat-host.sh" > "$WORK/fstat-host.log" 2>&1; rc=$?
[ $rc -eq 0 ] || [ $rc -eq 77 ] || sed 's/^/      /' "$WORK/fstat-host.log"
report fstat-host $rc "mintlib tree absent or host not LP64"

# stat-host (TASK/SDK01 F): newtos-arch/do_stat.c's __do_stat fills
# st_mtime/st_atime/st_ctime as mintlib's TOS branch does -- the DTA stamp
# through mintlib's own __unixtime, OLDDATE for a drive root, time() - 2 for a
# device name. fstat-host checks the Fdatime half. With the previous
# do_stat.c (__do_stat = __quickstat, all times 0) 13 of 16 fail.
"$SDK/libc/tests/stat-host.sh" > "$WORK/stat-host.log" 2>&1; rc=$?
[ $rc -eq 0 ] || [ $rc -eq 77 ] || sed 's/^/      /' "$WORK/stat-host.log"
report stat-host $rc "mintlib tree absent or host not LP64"

# waith-host (PORT-WAITH): the overlaid include/sys/wait.h -- WIFEXITED/
# WEXITSTATUS/WIFSIGNALED/WTERMSIG/WCOREDUMP/WIFSTOPPED/WSTOPSIG and the BSD
# `union wait' members on known status words, then through mintlib's own
# unix/wait4.c/waitpid.c/wait.c with the __waitval tfork leaves. With the
# pristine header (big-endian bit-field order) 28 of 74 fail.
"$SDK/libc/tests/waith-host.sh" > "$WORK/waith-host.log" 2>&1; rc=$?
[ $rc -eq 0 ] || [ $rc -eq 77 ] || sed 's/^/      /' "$WORK/waith-host.log"
report waith-host $rc "mintlib tree absent or host not little-endian LP64"

# malloc-host (MEM05 part B, user decision E6): the overlay mintlib/malloc.c's
# __newtos_malloc_maxhunk raises only MAXHUNK -- hunks still start at 8 KiB and
# double, now up to 16 MiB (malloc(32) x 147 605 = 10 hunks instead of 254);
# without the call the counts equal the post-E9 QEMU measurement; a
# hello-world takes one 8 KiB hunk; _mallocChunkSize still overrides. With the
# setter's body emptied 10 of 30 fail.
"$SDK/libc/tests/malloc-host.sh" > "$WORK/malloc-host.log" 2>&1; rc=$?
[ $rc -eq 0 ] || [ $rc -eq 77 ] || sed 's/^/      /' "$WORK/malloc-host.log"
report malloc-host $rc "mintlib tree absent or host not LP64"

# sysinfo-host (SDK03): the overlay unix/sysinfo.c names the platform for every
# _MCH -- the families 0-5, Hades, FireBee, ColdFire EVB ("cfevb"), NewTOS's
# 0xFFFFFFFF (MCH_NOHARD) as "unknown" like EmuTOS's machine_name(), a family
# above 5 as "atariclone" -- through newtos-arch/getcookie.c's pointer form.
# With the pristine file 5 of 15 fail (strlen (NULL), platforms[9] past the
# table, "cfevbatariclone").
"$SDK/libc/tests/sysinfo-host.sh" > "$WORK/sysinfo-host.log" 2>&1; rc=$?
[ $rc -eq 0 ] || [ $rc -eq 77 ] || sed 's/^/      /' "$WORK/sysinfo-host.log"
report sysinfo-host $rc "mintlib tree absent or host not LP64 little-endian"

# lp64-limits (SDK01 A): SIZE_MAX, PTRDIFF_MIN/MAX, SSIZE_MAX (and the
# neighbouring LONG/INTPTR/INTMAX limits) match their AArch64/LP64 types --
# all _Static_assert, compiled for the target with build-libc.sh's flags.
# With mintlib's pristine stdint.h + bits/posix1_lim.h 10 assertions fail.
"$SDK/libc/tests/lp64-limits.sh" > "$WORK/lp64-limits.log" 2>&1; rc=$?
[ $rc -eq 0 ] || [ $rc -eq 77 ] || sed 's/^/      /' "$WORK/lp64-limits.log"
report lp64-limits $rc "mintlib tree absent"

# iorec-layout (XBIOS01): the SDK's _IOREC and the device-0 record built from
# it have the LP64 offsets the kernel lays the Iorec records out at
# (docs/api/xbios/0e-iorec.md F1) -- all _Static_assert, compiled for the
# target like lp64-limits.
"$SDK/libc/tests/iorec-layout.sh" > "$WORK/iorec-layout.log" 2>&1; rc=$?
[ $rc -eq 0 ] || [ $rc -eq 77 ] || sed 's/^/      /' "$WORK/iorec-layout.log"
report iorec-layout $rc "mintlib tree absent"

# libc-link (SDK01 A): against the built libc.a, tmpfile/mkstemp/glob/
# wordexp/getlogin/sem_open (pwdgrp/), ttyname/ctermid (__sys_fstat),
# gets/getwd/siggetmask (.stabs overlays) and the nf_* family (newtos-arch/
# nf_ops.c) link; a program without main does not (no test program's main
# in the archive); no member references the syscall generator's yyparse.
# Against the pre-SDK01 libc.a: 0 of 31 link, plus the two archive checks.
"$SDK/libc/tests/libc-link.sh" > "$WORK/libc-link.log" 2>&1; rc=$?
[ $rc -eq 0 ] || [ $rc -eq 77 ] || sed 's/^/      /' "$WORK/libc-link.log"
report libc-link $rc "libc.a not built"

# rt-host (SDK01 E): compiler-rt's builtins (sdk/compiler-rt/
# libclang_rt.builtins.a) -- a program using long double and complex
# arithmetic links with newtos-cc -lm (0 undefined, binary128 helpers from the
# builtins, __extenddftf2 still libc's); every shipped builtins/libm member
# rebuilds byte-identical; the -ffixed-x18 rebuild runs natively on the
# AArch64 host: binary128 + - * / against the host's double results (double
# rounding innocuous at 113 bits), comparisons, conversions, FPCR rounding
# modes, complex multiply/divide vs the host, and powf/remainder/fdim/
# nexttowardf/casin.../cpowf against the system libm and a host build of the
# same OpenLibm sources (sdk/compiler-rt/tests/rt-host.c lists the oracles).
"$SDK/compiler-rt/tests/rt-host.sh" > "$WORK/rt-host.log" 2>&1; rc=$?
[ $rc -eq 0 ] || [ $rc -eq 77 ] || sed 's/^/      /' "$WORK/rt-host.log"
report rt-host $rc "$(tail -1 "$WORK/rt-host.log")"

# ldbl-host (SDK01 J): `long double' (binary128) in libc and libm -- printf
# %Le/%Lf/%Lg/%La, strtold/sscanf, qecvt/qfcvt_r/qgcvt, the classification
# functions, the OpenLibm `l' functions (scalbnl, expl, logl, sinl, ...) and
# full-width binary128 + - * /, run natively on the AArch64 host from the
# -ffixed-x18 rebuild of the very members a newtos-cc -lm program links (each
# first proven byte-identical to the shipped one), judged by
# sdk/libc/tests/ldbl-oracle.py against exact rational arithmetic.
"$SDK/libc/tests/ldbl-host.sh" > "$WORK/ldbl-host.log" 2>&1; rc=$?
[ $rc -eq 0 ] || [ $rc -eq 77 ] || sed 's/^/      /' "$WORK/ldbl-host.log"
report ldbl-host $rc "$(tail -1 "$WORK/ldbl-host.log")"

# --- cflib host tests ------------------------------------------------------------
# cookie-host (SDK05): the overlay cflib cookie.c (patch site SDK05) hands
# Getcookie the jar tag of a 4-character name -- first character in bits
# 31..24, bits 63..32 zero, bytes >= 0x80 not sign-extended -- so getcookie()
# finds NTOS/_MCH/_IDT/_AKP/EdDI. With the pristine cookie.c (byte union) 17 of
# 26 checks fail.
"$SDK/cflib/tests/cookie-host.sh" > "$WORK/cookie-host.log" 2>&1; rc=$?
[ $rc -eq 0 ] || [ $rc -eq 77 ] || sed 's/^/      /' "$WORK/cookie-host.log"
report cookie-host $rc "mintlib headers absent or host not LP64 little-endian"

# --- app host tests --------------------------------------------------------------
# c2p-host (PORT06, user decision 2026-10-04): DOOM's table chunky-to-planar
# (sdk/apps/doom/doomgeneric_gem.c, compiled without its GEM half) equals the
# bit-by-bit routine it replaced, kept in the test as the oracle -- 1..32
# planes, standard and interleaved form, odd widths, under ASan/UBSan.
# Byte-order-free by construction (16-bit stores), so no host check needed.
DOOMT="$SDK/apps/doom/tests"
if "$CC" -O1 -g -fsanitize=address,undefined -Wall -Wextra -Werror \
     -o "$WORK/c2p-host" "$DOOMT/c2p-host.c" > "$WORK/c2p-host.log" 2>&1; then
  "$WORK/c2p-host" >> "$WORK/c2p-host.log" 2>&1; rc=$?
  [ $rc -eq 0 ] || sed 's/^/      /' "$WORK/c2p-host.log"
  report c2p-host $rc
else
  echo "FAIL  c2p-host (did not build)"; sed 's/^/      /' "$WORK/c2p-host.log"; fail=$((fail+1))
fi

# sysacc-cmdline-host (SYSINFO02): the System Info accessory's boot-argument
# logic (sdk/examples/sysacc-cmdline.h, the header sysacc.c includes) -- the
# parse reads C:\CMDLINE.TXT as kernel/src/bootargs.rs reads the boot
# arguments; the merge keeps every unmanaged token in order, replaces managed
# ones in place, drops defaults, keeps CR LF / LF, one line. Pure byte logic.
SAT="$SDK/examples/tests"
if "$CC" -O1 -g -fsanitize=address,undefined -Wall -Wextra -Werror \
     -o "$WORK/sysacc-cmdline-host" "$SAT/sysacc-cmdline-host.c" > "$WORK/sysacc-cmdline-host.log" 2>&1; then
  "$WORK/sysacc-cmdline-host" >> "$WORK/sysacc-cmdline-host.log" 2>&1; rc=$?
  [ $rc -eq 0 ] || sed 's/^/      /' "$WORK/sysacc-cmdline-host.log"
  report sysacc-cmdline-host $rc
else
  echo "FAIL  sysacc-cmdline-host (did not build)"; sed 's/^/      /' "$WORK/sysacc-cmdline-host.log"; fail=$((fail+1))
fi

# custom-font-host (ZVIEW01): zView's draw_text() (zview/custom_font.c as the
# SDK builds it -- the overlay) hands vrt_cpyfm each glyph as host-order WORDs
# with row r in bits 15..8, exactly the form the 68k original builds; the
# pristine byte walk left the copied columns empty on little-endian (the
# browser's missing names). Needs the extracted z-tools tree for custom_font.h.
ZVT="$SDK/zview/z-tools-04805b7/zview"
if [ ! -f "$ZVT/custom_font.h" ]; then
  report custom-font-host 77 "z-tools tree absent (run sdk/zview/build-zview.sh)"
elif "$CC" -fsigned-char -Wall -Wno-pointer-sign -I "$SDK/zview/tests" -I "$ZVT" \
     -DZVIEW_CUSTOM_FONT_C="\"$SDK/zview/overlay/zview/custom_font.c\"" \
     -o "$WORK/custom-font-host" "$SDK/zview/tests/custom-font-host.c" > "$WORK/custom-font-host.log" 2>&1; then
  "$WORK/custom-font-host" >> "$WORK/custom-font-host.log" 2>&1; rc=$?
  [ $rc -eq 0 ] || sed 's/^/      /' "$WORK/custom-font-host.log"
  report custom-font-host $rc
else
  echo "FAIL  custom-font-host (did not build)"; sed 's/^/      /' "$WORK/custom-font-host.log"; fail=$((fail+1))
fi

# libtest-host (LDG-M2): the codec libraries (zlib, libpng, libjpeg, giflib,
# libexif) built for the host from the same pinned trees + config overlays;
# their results must reproduce the committed host references
# (sdk/examples/libtest/libtest-ref.h) bit for bit and match the independent
# Python references (libtest-data.h). Builds the target archives first if they
# are stale (lib_ensure), since the host build reads their source lists.
"$SDK/examples/libtest/make-ref.sh" --check > "$WORK/libtest-host.log" 2>&1; rc=$?
[ $rc -eq 0 ] || sed 's/^/      /' "$WORK/libtest-host.log"
report libtest-host $rc

# --- LDG module allocator -------------------------------------------------------
# libshare-host (LDG-M3-B2): libshare's malloc as the SDK builds it (pinned
# 0.1.1 + overlay/) serves a module's exit-time stdio buffer from its static
# BSS block, asks the system for nothing during the module's own run, and
# leaves no free-list node in memory the module's Pterm frees; later blocks
# come from ldg_Malloc in the caller's context.
"$SDK/libshare/tests/libshare-host.sh" > "$WORK/libshare-host.log" 2>&1; rc=$?
[ $rc -eq 0 ] || [ $rc -eq 77 ] || sed 's/^/      /' "$WORK/libshare-host.log"
report libshare-host $rc "libshare tree / mintlib headers absent or host not LP64"

# --- compiler wrapper -----------------------------------------------------------
# newtos-cc-host (SDK01 D): sdk/bin/newtos-cc + sdk/mk/newtos.mk build both
# templates (sdk/templates/{gem-hello,console-hello}, copied to a temporary
# directory) into ELF64 AArch64 ET_DYN programs with the PRGFLAGS note; PROT in
# the Makefile sets the protection nibble (0x17); `make card` copies;
# `newtos-cc -bare` builds sdk/examples/hello.c byte-identical to sdk/build.sh;
# an unknown -l is an error.
"$SDK/templates/tests/newtos-cc-host.sh" > "$WORK/newtos-cc-host.log" 2>&1; rc=$?
[ $rc -eq 0 ] || [ $rc -eq 77 ] || sed 's/^/      /' "$WORK/newtos-cc-host.log"
report newtos-cc-host $rc "$(tail -1 "$WORK/newtos-cc-host.log")"

echo
echo "host tests: $pass passed, $fail failed, $skip skipped"
exit $fail
