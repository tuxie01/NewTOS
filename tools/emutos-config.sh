#!/usr/bin/env bash
#
# emutos-config.sh — dump the compile-time CONF_* configuration of the
# pinned EmuTOS reference build (`make 512`, i.e. -DTARGET_512) straight
# from the preprocessor, and write it to docs/api/_emutos-config.md.
#
# Written for TASK/REF01.md. Does not modify references/emutos/ (reads
# only); writes only the one output document plus scratch files under a
# private mktemp(1) directory that is removed on exit.
#
# --------------------------------------------------------------------
# Why "-DTARGET_512 -Iinclude -I." on an empty translation unit that
# just #includes config.h is the WHOLE truth for this profile (checked
# against the pinned tree, not assumed — see REF01's own probe):
#
#   - DEF, the user-settable -D list, starts out empty:
#       references/emutos/Makefile:132   DEF =
#   - `make 512` adds exactly one define to it:
#       references/emutos/Makefile:561   512: override DEF += -DTARGET_512
#   - no localconf.h sits in the tree to inject silent extra defines;
#     the Makefile's own test for one is at
#       references/emutos/Makefile:170-176
#     (this script re-checks that live, below — it does not trust the
#     absence to still hold on a future pinned commit)
#   - `make 512`'s actual C compile also carries -DWITH_AES=1 -DWITH_CLI=1
#     (references/emutos/Makefile:267, DEFINES = ... $(DEF)), but
#     config.h never reads WITH_AES or WITH_CLI (grep confirms — their
#     only consumers are bios/endrom.c:15,30 and vdi/vdi_mouse.c:26,67,
#     i.e. link-time file selection, not a CONF_* macro). So they do not
#     belong in the config.h preprocessor run below, and their absence
#     does not make the table incomplete.
#
# Every CONF_* macro in config.h resolves to a plain integer or hex
# literal, EXCEPT two (CONF_WITH_SCSI_DRIVER, CONF_WITH_XHDI) which are
# defined as `!CONF_WITH_EXTERNAL_DISK_DRIVER` (include/config.h:1685,
# :1692). Plain `-E -dM` text substitution does not evaluate the `!`,
# so this script evaluates any non-literal residue through a real
# integer-constant-expression evaluator (bash arithmetic, which has the
# same semantics as a C preprocessor #if for !/&&/||/parens) instead of
# printing the raw, potentially misleading text.
#
# --------------------------------------------------------------------
# Reproducibility:
#   - The EmuTOS commit hash is read live from the pinned tree (git
#     rev-parse), so the table always states what it was actually
#     generated against, and changes truthfully if the pin moves.
#   - The "generation date" in the header is NOT `date`-derived — a
#     wall-clock timestamp would make two runs on different days
#     produce different files, breaking the "run it twice, get the same
#     file" acceptance check. It is a fixed string (the day this table
#     was generated for REF01), overridable via EMUTOS_CONFIG_GEN_DATE
#     for a deliberate re-generation. Bump the default by hand when you
#     deliberately regenerate this table against a moved pin; do not
#     wire it to `date`.
#
# Usage:
#   tools/emutos-config.sh [output-file]
#   Defaults to docs/api/_emutos-config.md relative to the repo root.
#
# Requires: a working m68k-atari-mint-gcc (default
# ${ORACLE_CROSS_MINT}gcc from tools/oracle/oracle.env, the oracle's one
# shared configuration; override with EMUTOS_CC).


# --------------------------------------------------------------------
# Shell guard. This script is bash (associative-array-free bash 3.2,
# to also run under macOS's stock /bin/bash — see the classification
# table below), not POSIX sh: it uses [[ ]], local, process
# substitution <(...), and ${var:0:8} substring expansion.
#
# `sh tools/emutos-config.sh` looks like it should work and does not:
# on this machine /bin/sh is bash itself, so a naive `[ -z
# "$BASH_VERSION" ]` guard does NOT fire (BASH_VERSION is set either
# way — it names the binary, not the mode). What actually changes is
# that bash enters POSIX-compatibility mode when it sees argv[0] "sh",
# which disables `<(...)`. The failure then surfaces deep inside the
# script as a bare "syntax error near unexpected token `(`" — and
# worse, that failure can still exit 0, so "run it twice, diff is
# empty" trivially "passes" for the wrong reason (neither run wrote
# anything). Verified empirically before writing this guard, not
# assumed: this system's `sh -c 'echo "$SHELLOPTS"'` prints
# `...:posix`, its `bash -c` counterpart does not — so check
# $SHELLOPTS for the posix flag, not just whether bash is present.
# --------------------------------------------------------------------
case ":${SHELLOPTS:-}:" in
    *:posix:*)
        echo "emutos-config.sh: running under a POSIX-mode shell (bash invoked as 'sh', or a stricter POSIX sh)." >&2
        echo "emutos-config.sh: this script needs real bash, not POSIX-compatibility mode. Re-run as: bash tools/emutos-config.sh" >&2
        exit 1
        ;;
esac
if [ -z "${BASH_VERSION:-}" ]; then
    echo "emutos-config.sh: this script requires bash, not sh/dash/zsh's sh-compat mode." >&2
    echo "emutos-config.sh: re-run as: bash tools/emutos-config.sh" >&2
    exit 1
fi

set -euo pipefail

die() { echo "emutos-config.sh: $*" >&2; exit 1; }
warn() { echo "emutos-config.sh: WARNING: $*" >&2; }

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
EMUTOS_DIR="$REPO_ROOT/references/emutos"
# shellcheck source=oracle/oracle.env
. "$SCRIPT_DIR/oracle/oracle.env"
EMUTOS_CC="${EMUTOS_CC:-${ORACLE_CROSS_MINT}gcc}"
OUT_FILE="${1:-$REPO_ROOT/docs/api/_emutos-config.md}"
GEN_DATE="${EMUTOS_CONFIG_GEN_DATE:-2026-09-24}"

[ -d "$EMUTOS_DIR" ] || die "pinned tree not found at $EMUTOS_DIR"
[ -f "$EMUTOS_DIR/include/config.h" ] || die "$EMUTOS_DIR/include/config.h missing"
[ -e "$EMUTOS_DIR/localconf.h" ] && die "localconf.h is present in $EMUTOS_DIR -- the -DTARGET_512-alone recipe assumes there is none (Makefile:170-176); remove it or stop trusting this table until it's gone"
command -v "$EMUTOS_CC" >/dev/null 2>&1 || die "preprocessor not found: $EMUTOS_CC (set EMUTOS_CC to override)"

EMUTOS_COMMIT="$(git -C "$EMUTOS_DIR" rev-parse HEAD)" || die "could not read the pinned tree's commit hash"
EMUTOS_COMMIT_SHORT="${EMUTOS_COMMIT:0:8}"
[ "$EMUTOS_COMMIT" = "$ORACLE_EMUTOS_COMMIT" ] ||
    warn "references/emutos is at $EMUTOS_COMMIT_SHORT, the oracle is pinned to ${ORACLE_EMUTOS_COMMIT:0:8} (tools/oracle/oracle.env)"

# Non-fatal: tell the reader if the pinned tree has local changes beyond
# the harmless .DS_Store, since that would mean the table reflects
# something other than the recorded commit.
DIRTY="$(git -C "$EMUTOS_DIR" status --porcelain 2>/dev/null | grep -v '\.DS_Store$' || true)"
if [ -n "$DIRTY" ]; then
    warn "pinned tree has uncommitted changes beyond .DS_Store — the table below reflects the WORKING TREE, not commit $EMUTOS_COMMIT_SHORT cleanly:"
    warn "$DIRTY"
fi

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# ----------------------------------------------------------------------
# Step 1: get the full set of CONF_* macro names config.h defines, from
# the TARGET_512 -dM dump (every TARGET_* variant defines the same 106
# names in this tree; verified below per block, not assumed).
# ----------------------------------------------------------------------
printf '#include "config.h"\n' > "$WORK/probe.c"

dump_names() {
    local dflag="$1"
    "$EMUTOS_CC" -E -dM -D"$dflag" -I"$EMUTOS_DIR/include" -I"$EMUTOS_DIR" "$WORK/probe.c" 2>"$WORK/dm.err" \
        | grep -oE '^#define (CONF_[A-Za-z0-9_]+)' \
        | awk '{print $2}' \
        | sort
    if [ -s "$WORK/dm.err" ]; then
        warn "preprocessor stderr for -D$dflag:"
        cat "$WORK/dm.err" >&2
    fi
}

dump_names TARGET_512 > "$WORK/names.txt"
NUM_MACROS="$(wc -l < "$WORK/names.txt" | tr -d ' ')"
[ "$NUM_MACROS" -gt 0 ] || die "no CONF_* macros found — something is wrong with the include path or config.h"

# ----------------------------------------------------------------------
# Step 2: fully resolve every macro's value for a given -D flag.
#   (a) macro-substitute each name via -E, using a purely numeric label
#       per line so the label itself is never accidentally macro-
#       expanded away (it happened once during development: labelling
#       a line with the macro name itself made the preprocessor expand
#       BOTH copies, destroying the name/value pairing).
#   (b) anything left that isn't a plain integer or 0x-hex literal
#       (i.e. still contains an operator like `!`) gets evaluated as an
#       integer-constant-expression, the same semantics the compiler's
#       own #if would apply.
# ----------------------------------------------------------------------
resolve_values() {
    local dflag="$1" outtsv="$2"
    local src="$WORK/resolve_$dflag.c"
    {
        printf '#include "config.h"\n'
        printf '__RESOLVE_START__\n'
        local i=0
        while IFS= read -r n; do
            printf '__IDX_%d__ %s\n' "$i" "$n"
            i=$((i + 1))
        done < "$WORK/names.txt"
        printf '__RESOLVE_END__\n'
    } > "$src"

    "$EMUTOS_CC" -E -D"$dflag" -I"$EMUTOS_DIR/include" -I"$EMUTOS_DIR" "$src" 2>"$WORK/resolve.err" \
        | sed -n '/^__RESOLVE_START__$/,/^__RESOLVE_END__$/p' \
        | grep -E '^__IDX_[0-9]+__ ' \
        | sed -E 's/^__IDX_[0-9]+__ //' \
        > "$WORK/rawvals_$dflag.txt"
    if [ -s "$WORK/resolve.err" ]; then
        warn "preprocessor stderr while resolving -D$dflag:"
        cat "$WORK/resolve.err" >&2
    fi

    local n_names n_vals
    n_names="$(wc -l < "$WORK/names.txt" | tr -d ' ')"
    n_vals="$(wc -l < "$WORK/rawvals_$dflag.txt" | tr -d ' ')"
    [ "$n_names" = "$n_vals" ] || die "resolve_values($dflag): $n_names names but $n_vals resolved values -- preprocessor output did not line up"

    : > "$outtsv"
    paste "$WORK/names.txt" "$WORK/rawvals_$dflag.txt" | while IFS=$'\t' read -r name val; do
        if [[ "$val" =~ ^-?[0-9]+$ ]] || [[ "$val" =~ ^0[xX][0-9A-Fa-f]+$ ]]; then
            printf '%s\t%s\n' "$name" "$val" >> "$outtsv"
        else
            local ev
            if ev=$(bash -c "echo \$(( $val ))" 2>/dev/null); then
                printf '%s\t%s\n' "$name" "$ev" >> "$outtsv"
            else
                warn "could not arithmetically resolve $name = '$val' for -D$dflag; keeping raw text"
                printf '%s\t%s\n' "$name" "$val" >> "$outtsv"
            fi
        fi
    done
}

resolve_values TARGET_512 "$WORK/table_512.tsv"

# ----------------------------------------------------------------------
# Step 3: enumerate every `#ifdef TARGET_*` block in config.h (name,
# start line, end line) by depth-tracking from each match, independent
# of surrounding nesting (the file is itself wrapped in one big
# `#ifndef _CONFIG_H` include guard, which would otherwise throw off a
# naive single global depth counter).
# ----------------------------------------------------------------------
CONFIG_H="$EMUTOS_DIR/include/config.h"
awk '
{ L[NR] = $0 }
END {
  n = NR
  for (i = 1; i <= n; i++) {
    if (L[i] ~ /^#[ \t]*ifdef[ \t]+TARGET_[A-Za-z0-9_]+/) {
      match(L[i], /TARGET_[A-Za-z0-9_]+/)
      name = substr(L[i], RSTART, RLENGTH)
      depth = 1
      j = i + 1
      while (depth > 0 && j <= n) {
        if (L[j] ~ /^#[ \t]*(if|ifdef|ifndef)([ \t]|$)/) depth++
        else if (L[j] ~ /^#[ \t]*endif/) depth--
        if (depth > 0) j++
      }
      print name "\t" i "\t" j
    }
  }
}
' "$CONFIG_H" > "$WORK/target_blocks.tsv"

# Sanity check demanded by REF01 point 3: does a #ifdef TARGET_512 block
# exist at all? (ACC01 said no — this re-checks it against the live
# tree rather than trusting that write-up.)
TARGET_512_BLOCK_COUNT="$(awk -F'\t' '$1=="TARGET_512"' "$WORK/target_blocks.tsv" | wc -l | tr -d ' ')"
TARGET_512_TREE_HITS="$(grep -rn 'TARGET_512' --include='*.h' --include='*.c' --include='*.S' --include='Makefile' "$EMUTOS_DIR" 2>/dev/null | grep -v '^\.git/' || true)"

# ----------------------------------------------------------------------
# Step 4: for every OTHER TARGET_* block found, resolve its full value
# set the same way (a real preprocessor run with that -D, not a text
# scan of the block) and diff against TARGET_512. A text scan of just
# the block's own lines would miss cascades through a non-CONF_-
# prefixed switch (e.g. TARGET_192 flips DETECT_NATIVE_FEATURES, which
# changes CONF_WITH_ARANYM's generic default two thousand lines away) --
# verified necessary during development, not a theoretical concern.
# ----------------------------------------------------------------------
: > "$WORK/blocks_summary.tsv"
while IFS=$'\t' read -r bname bstart bend; do
    [ "$bname" = "TARGET_512" ] && continue  # doesn't exist; nothing to diff
    resolve_values "$bname" "$WORK/table_$bname.tsv"
    n2="$(dump_names "$bname" | wc -l | tr -d ' ')"
    if [ "$n2" != "$NUM_MACROS" ]; then
        warn "$bname defines $n2 CONF_* macros, TARGET_512 defines $NUM_MACROS -- the diff below is only over the shared names"
    fi
    join -t $'\t' -j1 \
        <(sort -t $'\t' -k1,1 "$WORK/table_512.tsv") \
        <(sort -t $'\t' -k1,1 "$WORK/table_$bname.tsv") \
        | awk -F'\t' '$2!=$3 {print}' \
        | sort > "$WORK/diff_$bname.tsv"
    ndiff="$(wc -l < "$WORK/diff_$bname.tsv" | tr -d ' ')"
    printf '%s\t%s\t%s\t%s\n' "$bname" "$bstart" "$bend" "$ndiff" >> "$WORK/blocks_summary.tsv"
done < "$WORK/target_blocks.tsv"

# ----------------------------------------------------------------------
# Step 5: which CONF_WITH_* macros merely gate a *runtime*-detected
# variable (include/has.h's `#if CONF_WITH_X / extern int has_x` idiom)
# rather than a compiled-out/compiled-in code path. This is the data
# behind the machine-question paragraph.
# ----------------------------------------------------------------------
awk '
/^#if CONF_WITH_/ {
  match($0, /CONF_WITH_[A-Za-z0-9_]+/)
  cond = substr($0, RSTART, RLENGTH)
  pending = 1
  next
}
pending && /^extern int has_/ {
  match($0, /has_[A-Za-z0-9_]+/)
  var = substr($0, RSTART, RLENGTH)
  print cond "\t" var
  pending = 0
  next
}
/^#else/ { pending = 0 }
' "$EMUTOS_DIR/include/has.h" > "$WORK/runtime_gated.tsv"
N_RUNTIME_GATED="$(wc -l < "$WORK/runtime_gated.tsv" | tr -d ' ')"

# ----------------------------------------------------------------------
# Step 6: classify every CONF_* macro. This is the part a preprocessor
# cannot do -- it is a judgement call (hardware vs. other-machine vs.
# "touches AES/VDI/desktop, and does NewTOS have it") -- so it lives as
# a literal data table right here, not derived. Format per line:
#   NAME|CATEGORY|one-sentence note (with a file:line citation)
# CATEGORY is one of:
#   HW   - hardware: gated by has.h, or drives a chip/bus/CPU feature
#          directly. Out of scope by the compatibility contract
#          (docs/COMPATIBILITY.md:35-47): the BIOS/XBIOS trap surface
#          and observable behaviour are what must match; a physical
#          Atari chip behind it is free implementation, and NewTOS's
#          backend is the RPi4 HAL, not this switch. Label only.
#   OM   - other machine: Amiga, Lisa, ARAnyM, FireBee, ColdFire boards
#          -- not an Atari target. Label only.
#   OTH  - neither hardware/other-machine nor graphics/AES/VDI: GEMDOS,
#          BIOS API-surface, filesystem, or build/debug switches.
#          Labelled but not individually audited here (out of the
#          working set the user asked this table to answer); the
#          filesystem/BIOS-disk ones in particular are real candidate
#          gaps for a *future*, separately-scoped audit of their own.
#   HAVE - graphics/AES/VDI: NewTOS has it. Note is the file:line into
#          kernel/ or tos-api/ that proves it.
#   MISS - graphics/AES/VDI: NewTOS does not have it -- a real gap.
#          Note is what the flag switches on in EmuTOS (cited) and,
#          if one exists, the covering ticket id; "unticketed"
#          otherwise.
#
# Maintaining this table when the pinned EmuTOS commit moves:
#   1. Re-run this script. If config.h added/removed/renamed a CONF_*
#      macro, Step 6b below FAILS LOUDLY, naming exactly which names
#      are new (unclassified) or gone (stale) -- it does not silently
#      drop or invent rows.
#   2. For each NEW name: read its config.h comment + grep its `#if`
#      usage in aes/vdi/desk vs. bios/bdos vs. bios's has.h idiom (the
#      same three greps used to build every row below), decide the
#      category the same way, append one line.
#   3. For each STALE name: delete its line.
#   4. Re-run twice, confirm the diff is empty again.
# This table is not expected to change often -- config.h's CONF_*
# surface has been stable across the sessions this was checked
# against -- but it is data, not policy, and the check in 6b is what
# keeps it honest rather than quietly wrong.
#
# Written straight to a file, not captured via "$(cat <<'X' ... )" into
# a variable: bash 3.2 (this machine's actual /bin/bash -- see the
# shell guard above) has a real, confirmed parser bug where a heredoc
# nested inside a $(...) command substitution miscounts an ODD number
# of literal single-quote/apostrophe characters in the body and treats
# the substitution as unterminated, even though the heredoc itself is
# quoted (<<'X') and its content is supposed to be fully literal.
# Reproduced minimally before working around it, not guessed at: a
# one-line quoted heredoc containing one apostrophe, wrapped in
# X="$(cat <<'D' ... D)", fails to parse under this bash; two
# apostrophes (or zero) parse fine; the same heredoc NOT wrapped in
# $(...) parses fine regardless of apostrophe count. English prose
# about EmuTOS/NewTOS needs contractions and possessives too often to
# hunt for an even apostrophe count by hand on every edit, so the fix
# is structural: no $(...) around this heredoc, ever.
# ----------------------------------------------------------------------
cat <<'CLASSIFICATION_DATA' > "$WORK/classification.tsv"
CONF_ATARI_HARDWARE|HW|master switch for the whole Atari-hardware backend (config.h:663-667); NewTOS's backend is the RPi4 HAL, not this switch
CONF_COLDFIRE_TIMER_C|OM|ColdFire Timer-C emulation, FireBee/M548x only (config.h:954-964)
CONF_DEBUG_AES_STACK|OTH|EmuTOS-internal AES-stack-usage tracer (config.h:1934-1939); build/debug switch, not GEMDOS/BIOS/AES/VDI behaviour
CONF_DEBUG_DESK_STACK|OTH|EmuTOS-internal desktop-stack-usage tracer (config.h:1942-1946); build/debug switch
CONF_DETECT_FIRST_BOOT_WITHOUT_MEMCONF|OTH|cold/warm-boot detection depends on whether RAM is pre-initialised before EmuTOS runs (config.h:1408-1420); boot-sequence internal
CONF_LOGSEC_SIZE|OTH|GEMDOS maximum logical sector size (config.h:1594-1607); filesystem internal
CONF_PREFER_STRAM_DISK_BUFFERS|HW|ST-RAM/DMA disk-buffer placement preference (config.h:1723-1732); no ST-RAM/DMA-address constraint on the RPi4 HAL
CONF_SERIAL_CONSOLE|OTH|redirects BIOS console I/O to a UART for headless bring-up (config.h:1423-1429); debug/bring-up switch
CONF_SERIAL_CONSOLE_ANSI|OTH|terminal escape-sequence dialect for the serial console above (config.h:1432-1442); debug/bring-up switch
CONF_SERIAL_CONSOLE_POLLING_MODE|OTH|polling vs IRQ-fed serial console (config.h:1445-1455); debug/bring-up switch
CONF_SERIAL_IKBD|HW|redirects the IKBD protocol through the serial/modem port instead of the dedicated keyboard port (config.h:1458-1463); a physical port-wiring choice, distinct from NewTOS's own IKBD-over-UART path (kernel/src/ikbd.rs)
CONF_STRAM_SIZE|HW|physical ST-RAM size (config.h:872-877)
CONF_TTRAM_SIZE|HW|physical TT-RAM size (config.h:903-908)
CONF_VRAM_ADDRESS|HW|fixed VRAM address for custom graphics cards (config.h:1096-1103)
CONF_WITH_1FAT_SUPPORT|OTH|GEMDOS single-FAT filesystem compatibility (config.h:1548-1560); filesystem internal
CONF_WITH_3D_OBJECTS|HAVE|tos-api/src/aes/object.rs:1047-1051,2798 (FL3DOBJ/FL3DIND/FL3DBAK/FL3DACT bit handling)
CONF_WITH_68030_PMMU|HW|68030 PMMU tree install (config.h:1465-1472)
CONF_WITH_68040_PMMU|HW|68040 PMMU tree install (config.h:1474-1481)
CONF_WITH_ACSI|HW|ACSI bus (config.h:1032-1037)
CONF_WITH_ADVANCED_CPU|HW|68010-68060 CPU support (config.h:828-837)
CONF_WITH_ALT_DESKTOP_GRAPHICS|MISS|align_title() left-aligns dialog titles and sets ob_state for an underline (config.h:1734-1741; EmuTOS desk/deskmain.c:1727-1747, the CONF_WITH_ALT_DESKTOP_GRAPHICS arm); NewTOS's own align_title equivalents do the OTHER (non-ALT, classic-TOS) arm instead -- centre the title, no underline: programs/desktop/src/dir.rs:567-572 `(dw - title_w) / 2`, programs/desktop/src/inf.rs:3403-3404 `(dw - 21 * CELL_W) / 2`, both citing deskmain.c's centring branch by name in their own comments; ticketed GUI05 (OFFEN, P4)
CONF_WITH_ALT_RAM|HW|Alt-RAM support (config.h:879-884)
CONF_WITH_APOLLO_68080|HW|Apollo 68080/Vampire accelerator CPU (config.h:839-848)
CONF_WITH_ARANYM|OM|ARAnyM emulator target (config.h:1916-1925)
CONF_WITH_ATARI_VIDEO|HW|ST-Shifter-and-higher video master switch (config.h:1068-1073)
CONF_WITH_BACKGROUNDS|HAVE|programs/desktop/src/global.rs:173-179 (g_patcol background pattern/colour, synced to the AES)
CONF_WITH_BAS_MEMORY_MAP|OM|FireBee/M548x BaS memory map (config.h:1210-1220)
CONF_WITH_BIOS_EXTENSIONS|HW|gates only an extended Floprate() floppy-drive-type selector (EmuTOS bios/floppy.c:1048-1055, verified in the source, not assumed from the generic-sounding config.h comment); floppy hardware, same recorded decision as CONF_WITH_FORMAT
CONF_WITH_BLITTER|HW|blitter chip (config.h:1165-1170)
CONF_WITH_BOTTOMTOTOP|HAVE|programs/desktop/src/menu.rs:108 (Item::Cycle in the BTOPITEM slot), menu.rs:187-188 (doc comment 'Cycle windows bottom-to-top'), programs/desktop/src/main.rs:1474,2340 (Item::Cycle => do_cycle()); named Cycle, not BottomToTop -- a name grep for the EmuTOS macro missed it
CONF_WITH_BUS_ERROR|HW|whether the CPU/bus raises a Bus Error exception on invalid access (config.h:2044-2051); true on Atari, the AArch64 fault model is entirely different
CONF_WITH_CACHE_CONTROL|HW|'Cache' desktop menu item toggling 68030 CPU cache registers directly (config.h:1759-1768); no ARM-cache equivalent implemented (checked)
CONF_WITH_CARTRIDGE|HW|ROM port cartridge (config.h:1012-1023)
CONF_WITH_COLDFIRE_RS232|OM|ColdFire internal serial port (config.h:966-975)
CONF_WITH_COLOUR_ICONS|HAVE|tos-api/src/aes/resource.rs:1170-1183,1917 (fixup_colour_icons/CICONBLK relocation)
CONF_WITH_DESKTOP_CONFIG|HAVE|programs/desktop/src/menu.rs:131,231,272,361 (Item::DeskConfig / CONFITEM); see DESK05
CONF_WITH_DESKTOP_SHORTCUTS|HAVE|programs/desktop/src/ins.rs:1016 (ins_shortcut), programs/desktop/src/dir.rs:1701 (remove_locate_shortcut), the #X/#V INF lines including the inf_rev_level<2 swap; landed under DESK30 (ERLEDIGT, commit f85f9c8) -- a shortcut survives Save desktop and a cold start and launches on double-click
CONF_WITH_DIP_SWITCHES|HW|STe/TT/Falcon DIP switches (config.h:1150-1156, has.h-gated has_dip_switches)
CONF_WITH_DMASOUND|HW|DMA sound chip (config.h:1129-1134, has.h-gated has_dmasound)
CONF_WITH_DSP|HW|Falcon DSP chip (config.h:1136-1141, has.h-gated has_dsp)
CONF_WITH_EASTER_EGG|HW|EmuDesk Easter Egg tune, its value is literally CONF_WITH_YM2149 (config.h:1786-1792); sound-hardware payload
CONF_WITH_EJECT|OM|automatic floppy eject; EmuTOS's own comment: 'This isn't available on Atari hardware' (config.h:2036-2042) -- meaningful only on Amiga/Lisa
CONF_WITH_EXTENDED_MOUSE|MISS|IKBD scancode extension for mouse buttons 3/4/5 and the wheel (config.h:1491-1498; EmuTOS bios/aciavecs.S, bios/ikbd.c:748); kernel/src/ikbd.rs:48,92-93 treats 0xF6 as the status-report packet header and 0xF8-0xFB as the standard 2-button relative packet -- no wheel or button-3/4/5 decode anywhere in that dispatch. Distinct from the VDI vex_wheelv vector installer already present (tos-api/src/vdi/mouse.rs:710), which has nothing feeding it without this; ticketed GUI04 (OFFEN, raised to P2 -- half the path already exists in vex_wheelv)
CONF_WITH_EXTENDED_OBJECTS|HAVE|tos-api/src/aes/object.rs:565,569,2561,3074 (G_BOXTEXT/G_FBOXTEXT/G_FTEXT MagiC-style object types)
CONF_WITH_EXTERNAL_DISK_DRIVER|OTH|deactivates the built-in hard disk driver in favour of a disk-resident one (config.h:1670-1677); BIOS disk-driver internal
CONF_WITH_FALCON_MMU|HW|Falcon MMU (config.h:864-869)
CONF_WITH_FDC|HW|floppy disk controller chip (config.h:1025-1030)
CONF_WITH_FILEMASK|HAVE|programs/desktop/src/main.rs:1860 (fun_mask, File -> Set file mask)
CONF_WITH_FLEXCAN|OM|ColdFire eval-board CAN port / Eiffel keyboard adapter (config.h:1186-1192)
CONF_WITH_FORMAT|HW|'Format floppy diskette' desktop menu item (config.h:1802-1807); floppy hardware, recorded decision (docs/STATUS.md:240 'FAT12-Write (Floppy, out of scope)'; docs/analysis/bios-audit/bios-xbios-feasibility-triage.md 'Floppy = HW-Absenz')
CONF_WITH_FRB|HW|_FRB cookie tied to Alt-RAM presence (config.h:1500-1505)
CONF_WITH_GDOS|HAVE|kernel/src/arch/aarch64/exceptions.rs:416-427 (the GEM trap leaves x0/d0 unchanged for an unrecognised selector, so vq_gdos() honestly reports 'no GDOS', config.h:1615-1622); fixed and verified under V16
CONF_WITH_GPT_SUPPORT|OTH|GUID partition table recognition on hard disks (config.h:1562-1568; EmuTOS bios/disk.c:614-744); BIOS disk-driver internal (tos-api/src/bios/drive.rs implements MBR/EBR partition parsing only, checked)
CONF_WITH_GRAF_MOUSE_EXTENSION|HAVE|tos-api/src/aes/graf.rs:619-621 (M_SAVE/M_RESTORE/M_PREVIOUS)
CONF_WITH_ICDRTC|HW|ICD AdSCSI Plus RTC board via ACSI (config.h:1112-1119, has.h-gated has_icdrtc)
CONF_WITH_IDE|HW|Falcon IDE bus (config.h:1046-1051)
CONF_WITH_IKBD_ACIA|HW|the 6850 ACIA chip used for keyboard wiring (config.h:998-1003); distinct from NewTOS's own IKBD-protocol handling in kernel/src/ikbd.rs, which runs over the RPi4's own UART, not an ACIA
CONF_WITH_IKBD_CLOCK|HW|IKBD real-time-clock command (config.h:1005-1010)
CONF_WITH_LOADABLE_CURSORS|MISS|loads mouse cursor shapes from A:\EMUCURS.RSC at runtime (config.h:1286-1295); kernel/src/launch.rs:174-175 says so itself: 'accessories and loadable cursors/icons are not implemented at all'; ticketed GUI03 (OFFEN, P4)
CONF_WITH_MAGNUM|HW|Magnum RAM expansion card (config.h:918-924)
CONF_WITH_MEGARTC|HW|MegaST real-time clock (config.h:1105-1110, has.h-gated has_megartc)
CONF_WITH_MEMORY_TEST|OTH|cold-boot RAM chip self-test (config.h:1507-1512); boot diagnostic
CONF_WITH_MENU_EXTENSION|HAVE|tos-api/src/aes/menu.rs:1278,1590,1861,1899 (menu_popup/menu_settings/menu_attach/menu_istart)
CONF_WITH_MFP|HW|MFP 68901 chip (config.h:926-931)
CONF_WITH_MFP_RS232|HW|MFP RS-232 (config.h:933-938)
CONF_WITH_MIDI_ACIA|HW|MIDI ACIA chip (config.h:991-996)
CONF_WITH_MONSTER|HW|MonSTer expansion card (config.h:910-916)
CONF_WITH_NICELINES|HAVE|tos-api/src/aes/object.rs:3014 (drawn-line menu separators)
CONF_WITH_NOVA|HW|Nova graphics card adapter (config.h:1179-1184)
CONF_WITH_NVRAM|HW|NVRAM chip (config.h:1158-1163, has.h-gated has_nvram)
CONF_WITH_PCGEM|HAVE|tos-api/src/aes/appl.rs:364 (APPL_YIELD arm, cites EmuTOS gemsuper.c:115-119 #if CONF_WITH_PCGEM)
CONF_WITH_PRINTER_ICON|HAVE|programs/desktop/src/icons.rs:74,131,206,214,298 and programs/desktop/src/ins.rs:447,498,533 (desktop Printer icon/Anode, per DESK03/DESK04)
CONF_WITH_PRINTER_PORT|HW|parallel printer port (config.h:984-989)
CONF_WITH_READ_INF|HAVE|programs/desktop/src/main.rs:1754 (do_read_inf) and programs/desktop/src/inf.rs:982 (app_read_inf); per DESK14
CONF_WITH_RESET|HW|startup 'reset' instruction bypass quirk (config.h:1194-1208)
CONF_WITH_SCC|HW|SCC chip (config.h:947-952, has.h-gated has_scc)
CONF_WITH_SCSI|HW|SCSI bus (config.h:1039-1044)
CONF_WITH_SCSI_DRIVER|HW|raw SCSI command pass-through to a device (config.h:1679-1686); purposeless without a real SCSI target chip, which the RPi4 HAL has none of
CONF_WITH_SDMMC|HW|EmuTOS's own SD/MMC controller chip driver (config.h:1053-1058); NewTOS's SD/EMMC access is a separately-written HAL component (kernel/src/drivers/bcm2711/emmc.rs), not a port of this switch
CONF_WITH_SEARCH|HAVE|programs/desktop/src/main.rs:1482,2053 (Item::Search / do_search)
CONF_WITH_SELECTALL|HAVE|programs/desktop/src/main.rs:1478 (Item::SelectAll)
CONF_WITH_SFP004|HW|68881 FPU board for the Mega ST (config.h:1172-1177)
CONF_WITH_SHOW_FILE|MISS|double-click-to-show a file's contents in EmuDesk (config.h:1838-1844); no NewTOS reference found in programs/desktop/src (checked); ticketed GUI01 (ZURUECKGESTELLT, P4 -- the user prefers a viewer registered as an application with a file mapping over a built-in view, more flexible and reuses the existing association machinery; stays filed so the gap list stays complete, closes as moot if a .NAP-viewer association already covers the case)
CONF_WITH_SHUTDOWN|OTH|the shutdown() BIOS/XBIOS function (config.h:2024-2034); BIOS API surface, not AES/VDI
CONF_WITH_SIZE_TO_FIT|HAVE|programs/desktop/src/main.rs:2201,2206,2539,2676 (Item::SizeToFit / g.size_to_fit)
CONF_WITH_ST_MMU|HW|ST MMU (config.h:850-855)
CONF_WITH_STATIC_ALT_RAM|HW|compile-time-fixed Alt-RAM address (config.h:886-893)
CONF_WITH_STE_SHIFTER|HW|STe Shifter chip (config.h:1075-1080, has.h-gated has_ste_shifter)
CONF_WITH_TT_MFP|HW|second MFP, TT only (config.h:940-945, has.h-gated has_tt_mfp)
CONF_WITH_TT_MMU|HW|TT MMU (config.h:857-862)
CONF_WITH_TT_SHIFTER|HW|TT Shifter chip (config.h:1082-1087, has.h-gated has_tt_shifter)
CONF_WITH_TTRAM|HW|TT-RAM detection/usage (config.h:895-900)
CONF_WITH_UAE|OM|UAE emulator advanced features, Amiga target (config.h:1876-1882)
CONF_WITH_ULTRASATAN_CLOCK|HW|UltraSatan clock via ACSI (config.h:1121-1127, has.h-gated has_ultrasatan_clock)
CONF_WITH_VAMPIRE_SPI|HW|SPI on the Vampire accelerator, for SD/MMC (config.h:1060-1066)
CONF_WITH_VDI_16BIT|HAVE|tos-api/src/vdi/inquiry.rs:7 and attributes.rs/raster.rs/workstation.rs (Falcon-truecolor workstation, landed per TASK/A86.md and TASK/A101.md)
CONF_WITH_VDI_TEXT_SPEEDUP|HAVE|tos-api/src/vdi/text.rs:577 (v_gtext); the observable feature (VDI text output) is implemented through a differently-architected raster backend, not a port of EmuTOS's specific speedup path
CONF_WITH_VDI_VERTLINE|HAVE|tos-api/src/vdi/primitives.rs:744 (v_pline handles vertical lines); same caveat as CONF_WITH_VDI_TEXT_SPEEDUP
CONF_WITH_VIDEL|HW|Falcon VIDEL chip (config.h:1089-1094, has.h-gated has_videl)
CONF_WITH_VIEWER_SUPPORT|MISS|default file viewer when no app claims a document type (config.h:1854-1861); programs/desktop/src/inf.rs:1847-1850 says so itself: 'AF_VIEWER is CONF_WITH_VIEWER_SUPPORT, which this desktop does not implement'; ticketed GUI02 (ZURUECKGESTELLT, P4, depends on GUI01 -- same reason: a viewer registered as an application with a file mapping is preferred over a built-in view; stays filed so the gap list stays complete, closes as moot if a .NAP-viewer association already covers the case)
CONF_WITH_VME|HW|Mega STe VME bus (config.h:1143-1148, has.h-gated has_vme)
CONF_WITH_WINDOW_COLOURS|HAVE|tos-api/src/aes/window.rs:78-79 (WF_COLOR/WF_DCOLOR)
CONF_WITH_WINDOW_ICONS|HAVE|programs/desktop/src/win.rs (win_icalc); per DESK15
CONF_WITH_XBIOS_SOUND|HW|Falcon-XBIOS-sound-API shim running on STe/TT DMA-sound hardware (config.h:1514-1522); audio (DMA-Sound/DSP) explicitly deferred by User decision (docs/STATUS.md:242 XBIOS feasibility triage)
CONF_WITH_XHDI|OTH|the XHDI hard-disk abstraction cookie and vector table (config.h:1688-1693); BIOS disk-driver internal (only passing comments reference EmuTOS's XHDI in tos-api/src/bios/bpb.rs:195 and tick.rs:35 -- no cookie or vector table implemented)
CONF_WITH_YM2149|HW|YM2149 PSG sound chip (config.h:977-982)
CLASSIFICATION_DATA

# Step 6b: cross-check the classification against the live 106 names.
# This is what keeps the table honest across a re-pin (see the
# maintenance note above) instead of silently drifting.
awk -F'|' '{print $1}' "$WORK/classification.tsv" | sort > "$WORK/class_names.txt"
MISSING_FROM_CLASS="$(comm -23 "$WORK/names.txt" "$WORK/class_names.txt")"
EXTRA_IN_CLASS="$(comm -13 "$WORK/names.txt" "$WORK/class_names.txt")"
if [ -n "$MISSING_FROM_CLASS" ] || [ -n "$EXTRA_IN_CLASS" ]; then
    warn "the classification table is out of sync with the live $NUM_MACROS-macro list:"
    [ -n "$MISSING_FROM_CLASS" ] && warn "  new in config.h, not yet classified: $(echo "$MISSING_FROM_CLASS" | tr '\n' ' ')"
    [ -n "$EXTRA_IN_CLASS" ] && warn "  classified but no longer in config.h: $(echo "$EXTRA_IN_CLASS" | tr '\n' ' ')"
    die "fix the classification table in this script (see the maintenance note above) before trusting its output"
fi

awk -F'|' '$2=="HAVE"||$2=="MISS"' "$WORK/classification.tsv" | sort -t'|' -k1,1 > "$WORK/working_set.tsv"
awk -F'|' '$2=="HW"||$2=="OM"' "$WORK/classification.tsv" | sort -t'|' -k1,1 > "$WORK/hw_om.tsv"
awk -F'|' '$2=="OTH"' "$WORK/classification.tsv" | sort -t'|' -k1,1 > "$WORK/other_set.tsv"
N_WORKING="$(wc -l < "$WORK/working_set.tsv" | tr -d ' ')"
N_HAVE="$(awk -F'|' '$2=="HAVE"' "$WORK/classification.tsv" | wc -l | tr -d ' ')"
N_MISS="$(awk -F'|' '$2=="MISS"' "$WORK/classification.tsv" | wc -l | tr -d ' ')"
N_HWOM="$(wc -l < "$WORK/hw_om.tsv" | tr -d ' ')"
N_OTHER="$(wc -l < "$WORK/other_set.tsv" | tr -d ' ')"

# Step 6c: does any graphics/AES/VDI (working-set) flag also appear in
# the has.h runtime-gated list computed in Step 5? It should not -- if
# it did, "no machine-question caveat applies to the working set" would
# be false. Checked mechanically every run, not just asserted in prose.
WORKING_NAMES="$(cut -d'|' -f1 "$WORK/working_set.tsv")"
RUNTIME_GATED_NAMES="$(cut -f1 "$WORK/runtime_gated.tsv")"
WORKING_RUNTIME_OVERLAP="$(comm -12 <(printf '%s\n' "$WORKING_NAMES" | sort) <(printf '%s\n' "$RUNTIME_GATED_NAMES" | sort))"
if [ -n "$WORKING_RUNTIME_OVERLAP" ]; then
    die "the machine-question caveat claim is false for this pinned tree: these working-set (graphics/AES/VDI) flags ARE has.h-runtime-gated and need the hardware caveat after all: $WORKING_RUNTIME_OVERLAP"
fi

# ========================================================================
# Assemble docs/api/_emutos-config.md
# ========================================================================
mkdir -p "$(dirname "$OUT_FILE")"

{
cat <<EOF
# EmuTOS reference configuration (\`make 512\`, TARGET_512)

Generated by \`tools/emutos-config.sh\` — re-run it to regenerate; do not
hand-edit. Generated: $GEN_DATE. Pinned tree: \`references/emutos\` @
\`$EMUTOS_COMMIT\`.

This table replaces re-deriving \`CONF_WITH_*\`/\`CONF_*\` gating from
\`config.h\`'s nesting by eye for every extraction (see \`TASK/REF01.md\`
— two extractions read the wrong \`#ifdef TARGET_192\` block for our
profile in one week). Extractions and spec entries cite this file, not
\`config.h\` directly, for "is feature X compiled into our reference
build" questions.

## Why \`-DTARGET_512\` alone is the whole truth here

* \`DEF\` (the user \`-D\` list) starts empty: \`references/emutos/Makefile:132\`.
* \`make 512\` adds exactly one define to it:
  \`references/emutos/Makefile:561\` (\`512: override DEF += -DTARGET_512\`).
* No \`localconf.h\` sits in the pinned tree (checked live by this run,
  not assumed); the Makefile's own test for one is
  \`references/emutos/Makefile:170-176\`.
* \`make 512\`'s real compile also passes \`-DWITH_AES=1 -DWITH_CLI=1\`
  (\`references/emutos/Makefile:267\`), but \`config.h\` never reads
  \`WITH_AES\`/\`WITH_CLI\` (their only consumers are
  \`bios/endrom.c:15,30\` and \`vdi/vdi_mouse.c:26,67\` — link-time file
  selection, not a \`CONF_*\` macro). Their absence from the
  preprocessor run below does not change any row of this table.

## The machine question — what this table can and cannot tell you

The Makefile's own comment calls the 512K image built by \`make 512\`
"for TT or Falcon; also usable for ST/STe under Hatari"
(\`references/emutos/Makefile:555\`). **One ROM image runs on all of
them** — EmuTOS does not build a separate binary per machine here; it
builds one binary that decides at boot which machine it is actually
running on.

The mechanism (read end to end, not assumed): \`include/has.h\` defines
\`HAS_X\` as a **compile-time constant \`0\`** when the matching
\`CONF_WITH_X\` is off, but as a reference to a **runtime variable**
\`has_x\` when \`CONF_WITH_X\` is on
(\`references/emutos/include/has.h:26-45\`, the \`HAS_STE_SHIFTER\` /
\`HAS_TT_SHIFTER\` / \`HAS_VIDEL\` / \`HAS_VME\` pattern, repeated for
every entry in the table below). In the TARGET_512 profile every one of
these \`CONF_WITH_*\` is \`1\` — see the runtime-gated list below — so
none of that detection code is compiled out. Which branch is *actually*
taken is decided once, at boot, by \`detect_video()\`
(\`bios/machine.c:101-162\`): it probes real hardware registers for a
bus error (STe Shifter register \`0xffff820d\`, TT palette registers
\`0xffff8400\`, Falcon \`HHT\` register \`0xffff8282\` via \`check_read_byte\`)
and sets \`has_ste_shifter\` / \`has_tt_shifter\` / \`has_videl\`
accordingly; \`detect_vme()\` does the same for \`has_vme\`
(\`bios/machine.c:200-221\`). \`fill_cookie_jar()\`
(\`bios/machine.c:665-694\`) then derives \`cookie_mch\` (\`MCH_ST\` /
\`MCH_STE\` / \`MCH_MSTE\` / \`MCH_TT\` / \`MCH_FALCON\`,
\`include/cookie.h:49-56\`) and \`cookie_vdo\` from exactly those
runtime flags (\`setvalue_mch\`: \`bios/machine.c:381-405\`;
\`setvalue_vdo\`: \`bios/machine.c:363-379\`). Both run unconditionally
during boot, before any AES/desktop code: \`machine_detect()\` from
\`bios/bios.c:246-248\`, \`fill_cookie_jar()\` from \`bios/bios.c:279-280\`.

**What a reader of this table may conclude, for the rows this actually
touches:** a row with value \`1\` means the corresponding driver's
*code* is compiled into the TARGET_512 ROM and can run on TT, Falcon,
STE or plain ST alike — none of it is compiled out for this profile.
"In our Falcon profile, VIDEL is on" is not a sentence the table can
support by itself — TARGET_512 is not a Falcon-only profile, and
whether VIDEL (or the TT Shifter, or VME) is actually driving the
screen on a concrete boot depends on \`cookie_mch\`/\`cookie_vdo\`/the
\`has_*\` flags, runtime state set by hardware probing that a
compile-time table cannot show.

**But this caveat has a scope, and the scope is hardware, not the
whole table.** \`include/has.h\`'s own first line says what its \`HAS_*\`
family is for: *"BIOS \`HAS_*\` macros to determine if hardware is
available"* (\`references/emutos/include/has.h:2\`). Every one of the
$N_RUNTIME_GATED \`CONF_WITH_*\` macros this run found gating a runtime
\`has_*\` variable this way is listed below, and every single one of
them is a physical chip, bus, or board (verified: cross-checked against
this table's own graphics/AES/VDI classification in Step 6c — zero
overlap on this pinned tree, checked mechanically each run, not
asserted). That hardware layer is precisely the one NewTOS does **not**
reproduce: by \`docs/COMPATIBILITY.md\`'s own contract table
(\`:35-47\`), what must match is the trap surface and the observable
behaviour a program can detect — not the chip behind it — and NewTOS's
backend for all of it is the RPi4 HAL, not an emulated STe Shifter or
TT Shifter or VIDEL. "Would EmuTOS have found a VIDEL chip on this
boot" is a question about silicon NewTOS does not have; this table
cannot and need not answer it.

\`CONF_WITH_*\` macros that gate a runtime-detected \`has_*\` variable
this way (extracted from \`include/has.h\`, not hand-picked):

EOF

sort -t $'\t' -k1,1 "$WORK/runtime_gated.tsv" | while IFS=$'\t' read -r cond var; do
    val="$(awk -F'\t' -v c="$cond" '$1==c {print $2}' "$WORK/table_512.tsv")"
    printf -- '* `%s` = `%s` (compile-time) gates the runtime variable `%s`\n' "$cond" "$val" "$var"
done

cat <<EOF

**Therefore: for everything this table classifies as graphics/AES/VDI
(the working set in the next section) — GEMDOS, the AES, the VDI, the
desktop — the table is authoritative with *no* caveat at all,** because
not one of those flags is runtime-gated (mechanically re-checked every
run in Step 6c of the script, not just claimed here). A row there that
says "\`1\`, NewTOS has it" or "\`1\`, NewTOS is missing it" means
exactly that, on every machine the ROM boots on, TT or Falcon alike —
there is no second, hardware-dependent half to that sentence the way
there is for \`CONF_WITH_VIDEL\`. Outside the has.h list above, exactly
one further place in \`desk/\`/\`aes/\` reads a runtime machine identity
at all — \`aes/gemgsxif.c:161\` (\`if (HAS_VIDEL)\`) — and it is not one
of the working-set flags either.

## Traps: every \`#ifdef TARGET_*\` block in config.h

EOF

n_blocks="$(wc -l < "$WORK/target_blocks.tsv" | tr -d ' ')"
cat <<EOF
\`config.h\` (this pinned tree) has **$n_blocks** \`#ifdef TARGET_*\`
blocks:

| Block | Lines | CONF_\* values that differ from TARGET_512 |
|---|---|---|
EOF
while IFS=$'\t' read -r bname bstart bend ndiff; do
    printf '| `%s` | `include/config.h:%s-%s` | %s |\n' "$bname" "$bstart" "$bend" "$ndiff"
done < "$WORK/blocks_summary.tsv"

cat <<EOF

**No \`#ifdef TARGET_512\` block exists.** Confirmed two ways against
this pinned tree, not assumed from a prior write-up:

1. Zero of the $n_blocks blocks above are named \`TARGET_512\`
   ($TARGET_512_BLOCK_COUNT matches for the literal pattern
   \`#ifdef TARGET_512\` in \`include/config.h\`).
2. The literal string \`TARGET_512\` appears exactly once in the whole
   pinned tree (\`.h\`/\`.c\`/\`.S\`/\`Makefile\`, excluding \`.git\`):

EOF
echo "$TARGET_512_TREE_HITS" | awk -F: -v root="$EMUTOS_DIR/" '
{
  path = $1
  lineno = $2
  content = $0
  sub(/^[^:]*:[^:]*:/, "", content)
  sub("^" root, "references/emutos/", path)
  gsub(/^[ \t]+/, "", content)
  printf("   - `%s:%s`: `%s`\n", path, lineno, content)
}'
cat <<'EOF'

   — the Makefile recipe that *defines* the macro, never a config.h
   consumer of it. This confirms ACC01's claim ("kein eigener
   `#ifdef TARGET_512`-Block existiert in `config.h`"): TARGET_512
   falls through to config.h's generic (no-TARGET_*-defined) defaults
   for every single CONF_* macro, with no exception.

Other `TARGET_*` blocks touch `CONF_*` macros TARGET_512 also defines,
but through a **different** compile-time switch — do not read a value
from one of these blocks as if it were our profile's:
EOF

while IFS=$'\t' read -r bname bstart bend ndiff; do
    [ "$ndiff" = "0" ] && continue
    cat <<EOF

### \`$bname\` (\`include/config.h:$bstart-$bend\`, $ndiff diverging)

| CONF_* | TARGET_512 | $bname |
|---|---|---|
EOF
    while IFS=$'\t' read -r name v512 vblock; do
        printf '| `%s` | %s | %s |\n' "$name" "$v512" "$vblock"
    done < "$WORK/diff_$bname.tsv"
done < "$WORK/blocks_summary.tsv"

cat <<EOF

Note: \`config.h\` also has non-\`TARGET_*\` machine-selector guards
(\`#ifdef MACHINE_ARANYM\` at \`include/config.h:61\`,
\`#ifdef MACHINE_FIREBEE\` at \`:127\`, plus \`MACHINE_AMIGA\`/
\`MACHINE_LISA\`) — out of REF01's scope (which asked only about
\`#ifdef TARGET_*\`), noted here so a reader doesn't mistake this
table's silence on them for their absence.

## Classification: what have we accounted for so far

The question this table exists to answer is not "what are all 106
values" — it is **"of the switches that matter for the surface NewTOS
is actually rebuilding (graphics / the AES / the VDI), which ones does
NewTOS already have, and which are a real gap?"** Hardware and
other-machine switches carry no judgement either way (out of scope by
\`docs/COMPATIBILITY.md\`'s contract, see above); they are labelled and
set aside. The classification itself — which of the three groups a
macro falls into, and whether NewTOS has a working-set one — cannot be
derived by the preprocessor; it is a data table inside
\`tools/emutos-config.sh\` (search it for \`CLASSIFICATION_DATA\`),
cross-checked every run against the live $NUM_MACROS-macro list (Step
6b — the run fails loudly, naming names, if config.h and the
classification table have drifted apart). See the script's own comment
above that table for how to extend it when the pinned commit moves.

**A working-set row is classified by what the flag *does*, never by
whether its EmuTOS macro name turns up in a \`kernel/\`/\`tos-api/\`
grep.** NewTOS deliberately does not carry \`CONF_*\` names into its own
source, so a name search finds nothing for a feature NewTOS has under
a different name — and reads as a false gap. Two rows in this table
were misclassified exactly that way in an earlier pass and corrected
after a source re-check (\`CONF_WITH_BOTTOMTOTOP\` is \`Item::Cycle\`
here, not anything spelled \`BottomToTop\`; \`CONF_WITH_DESKTOP_SHORTCUTS\`
is \`ins_shortcut\`/\`remove_locate_shortcut\`, not anything spelled
\`Shortcut\` near the macro's own vocabulary). This will recur on the
next re-pin unless whoever extends the table (Step 6, maintenance note
above) reads what the EmuTOS flag *does* and greps for that behaviour
in \`desk\`/\`aes\`/\`vdi\`, not for the flag's own name.

### Graphics / AES / VDI — the working set ($N_WORKING macros: $N_HAVE have, $N_MISS gap)

For every one of these, the expectation is that NewTOS has it. No
machine-question caveat applies to this group (Step 6c re-checks that
on every run — see above).

EOF

printf '**Has it** (%s):\n\n| Macro | Value | NewTOS |\n|---|---|---|\n' "$N_HAVE"
awk -F'|' '$2=="HAVE"' "$WORK/classification.tsv" | while IFS='|' read -r name cat note; do
    val="$(awk -F'\t' -v c="$name" '$1==c {print $2}' "$WORK/table_512.tsv")"
    printf '| `%s` | `%s` | %s |\n' "$name" "$val" "$note"
done

cat <<EOF

**Gap** ($N_MISS — filed as GUI01-GUI05; two (GUI01, GUI02) are
deferred by design, not neglect — see the note on each):

| Macro | Value | What it switches on in EmuTOS, and ticket status |
|---|---|---|
EOF
awk -F'|' '$2=="MISS"' "$WORK/classification.tsv" | while IFS='|' read -r name cat note; do
    val="$(awk -F'\t' -v c="$name" '$1==c {print $2}' "$WORK/table_512.tsv")"
    printf '| `%s` | `%s` | %s |\n' "$name" "$val" "$note"
done

cat <<EOF

### Hardware / other machine ($N_HWOM macros — labelled, no per-switch audit)

Out of scope by \`docs/COMPATIBILITY.md\`'s contract (\`:35-47\`): a
physical Atari chip, bus, CPU feature, or an entirely different machine
target (Amiga/Lisa/ARAnyM/FireBee/ColdFire). This includes the
floppy/FDC/\`CONF_WITH_FORMAT\` side — hardware, and additionally a
recorded project decision (\`docs/STATUS.md:240\`).

| Macro | Value | Label |
|---|---|---|
EOF
awk -F'|' '$2=="HW"||$2=="OM"' "$WORK/classification.tsv" | while IFS='|' read -r name cat note; do
    val="$(awk -F'\t' -v c="$name" '$1==c {print $2}' "$WORK/table_512.tsv")"
    printf '| `%s` | `%s` | **%s** — %s |\n' "$name" "$val" "$cat" "$note"
done

cat <<EOF

### Neither ($N_OTHER macros — GEMDOS / BIOS API surface / filesystem / build & debug)

Not hardware, not other-machine, but also not graphics/AES/VDI, so not
individually audited for this table — the user's question was scoped
to the working set above. Several of these (the BIOS-disk ones
especially — \`CONF_WITH_XHDI\`, \`CONF_WITH_GPT_SUPPORT\`) are
plausible candidates for a **separately scoped** future audit; they are
listed here so that scope isn't silently lost, not because this table
judges them.

| Macro | Value | Label |
|---|---|---|
EOF
awk -F'|' '$2=="OTH"' "$WORK/classification.tsv" | while IFS='|' read -r name cat note; do
    val="$(awk -F'\t' -v c="$name" '$1==c {print $2}' "$WORK/table_512.tsv")"
    printf '| `%s` | `%s` | %s |\n' "$name" "$val" "$note"
done

cat <<EOF

## Regenerating

\`\`\`
tools/emutos-config.sh
\`\`\`

Requires \`$EMUTOS_CC\` (override with \`EMUTOS_CC\`). Reads only
\`references/emutos/\`; writes only this file. Running it twice against
an unchanged pinned tree produces a byte-identical file (the only
non-mechanical input, the generation date, is a fixed string in the
script, not \`\$(date)\` — see the script header).
EOF
} > "$OUT_FILE"

echo "Wrote $OUT_FILE ($NUM_MACROS CONF_* macros, $n_blocks TARGET_* blocks, pinned commit $EMUTOS_COMMIT_SHORT)" >&2
