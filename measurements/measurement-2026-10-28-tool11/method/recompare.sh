#!/bin/sh
#
# recompare.sh -- TOOL11 item 3: the committed NewTOS call-area dumps, judged
# by tosdiff before (HEAD's PTR_RAM: non-null, even, < 0x80000000) and after
# the lower bound 0x400 (this change), each against the pinned references of
# its table.  Prints, per dump and reference, the cases that differ under the
# old and the new rule and the two set differences.  Read only: every dump is
# copied into <scratch> first; nothing under measurements/ is written.
#
# usage: recompare.sh <repo> <old-rev> <scratch>
#   <repo>     the repository root (its tosdump/host/build/tosdiff = new rule;
#              built here with `make -C tosdump/host build/tosdiff`)
#   <old-rev>  the commit whose tosdiff is the old rule (a5dc93a1)
#   <scratch>  a new or empty scratch directory
set -u

REPO="${1:?usage: recompare.sh <repo> <old-rev> <scratch>}"
OLDREV="${2:?usage}"
SCR="${3:?usage}"
mkdir -p "$SCR"

# --- the two comparators -----------------------------------------------
mkdir -p "$SCR/old"
(cd "$REPO" && git archive "$OLDREV" tosdump) | tar -x -C "$SCR/old" || exit 1
make -s -C "$SCR/old/tosdump/host" build/tosdiff >/dev/null || exit 1
make -s -C "$REPO/tosdump/host" build/tosdiff >/dev/null || exit 1
OLD="$SCR/old/tosdump/host/build/tosdiff"
NEW="$REPO/tosdump/host/build/tosdiff"
echo "old tosdiff: $OLDREV ($(shasum -a 256 "$OLD" | cut -c1-16))"
echo "new tosdiff: working tree ($(shasum -a 256 "$NEW" | cut -c1-16))"

M="$REPO/measurements"
R="$REPO/tosdump/reference"
C="$REPO/tosdump/cases"

# <dump> <table> <reference>... ; dump paths relative to measurements/
cmpset() {
	dump="$1"; table="$2"; shift 2
	tag=$(echo "$dump" | tr '/' '_')
	cp "$M/$dump" "$SCR/$tag" || return 1
	for ref in "$@"; do
		cp "$R/$ref" "$SCR/ref-$ref"
		"$OLD" "$C/$table" "$SCR/ref-$ref" "$SCR/$tag" >"$SCR/$tag.$ref.old" 2>&1
		orc=$?
		"$NEW" "$C/$table" "$SCR/ref-$ref" "$SCR/$tag" >"$SCR/$tag.$ref.new" 2>&1
		nrc=$?
		grep -o '^case [0-9]*' "$SCR/$tag.$ref.old" | cut -d' ' -f2 | sort >"$SCR/o"
		grep -o '^case [0-9]*' "$SCR/$tag.$ref.new" | cut -d' ' -f2 | sort >"$SCR/n"
		echo
		echo "$dump  [$table]  vs $ref"
		echo "  old: $(grep 'cases differ' "$SCR/$tag.$ref.old") (exit $orc)"
		echo "  new: $(grep 'cases differ' "$SCR/$tag.$ref.new") (exit $nrc)"
		echo "  newly differing: $(comm -13 "$SCR/o" "$SCR/n" | sort -n | tr '\n' ' ')"
		echo "  no longer differing: $(comm -23 "$SCR/o" "$SCR/n" | sort -n | tr '\n' ' ')"
		for k in $(comm -13 "$SCR/o" "$SCR/n" | sort -n); do
			awk -v k="$k" '
				/^case / { on = ($2 == k) }
				on && /PTR_RAM/ { print "    " $0 }' "$SCR/$tag.$ref.new"
		done
	done
}

B1="EMUTOS-1a7484ab-BIOS.BIN EMUTOS-1a7484ab-BIOS-falcontc.BIN"
for d in \
	measurement-2026-10-27-dumpbios/runs/cmp/NEWTOS.BIN \
	measurement-2026-10-28-bios-core-audit/runs/NEWTOS-bios-r1/cmp/NEWTOS.BIN \
	measurement-2026-10-28-tool08/runs/r1/cmp/NEWTOS.BIN \
	measurement-2026-10-28-tool08/runs-control/r1/cmp/NEWTOS.BIN \
	measurement-2026-10-28-tool09/runs/r1-argtrace-head/cmp/NEWTOS.BIN \
	measurement-2026-10-28-tool09/runs/r1-c3db-clone/cmp/NEWTOS.BIN \
	measurement-2026-10-28-tool10/tosdump-r1/cmp/NEWTOS.BIN \
	measurement-2026-10-28-xbios-chardev-audit/runs/NEWTOS-bios-r1/cmp/NEWTOS.BIN \
	measurement-2026-10-28-xbios-system-audit/runs/NEWTOS-bios-r1/cmp/NEWTOS.BIN \
	measurement-2026-10-28-tool11/runs/bios-r1/BIOS.BIN
do
	[ -f "$M/$d" ] || { echo; echo "$d: absent"; continue; }
	# shellcheck disable=SC2086
	cmpset "$d" bios-r1.tsv $B1
done

cmpset measurement-2026-10-28-xbios-chardev-audit/runs/NEWTOS-xbios-chardev-r1/cmp/NEWTOS.BIN \
	xbios-chardev-r1.tsv EMUTOS-1a7484ab-XBIOS-chardev.BIN \
	EMUTOS-1a7484ab-XBIOS-chardev-falcontc.BIN
for d in \
	measurement-2026-10-28-bios-core-audit/runs/NEWTOS-bios-core-r1/cmp/NEWTOS.BIN \
	measurement-2026-10-28-tool08/runs/core/cmp/NEWTOS.BIN \
	measurement-2026-10-28-tool08/runs-control/core/cmp/NEWTOS.BIN
do
	cmpset "$d" bios-core-r1.tsv EMUTOS-1a7484ab-BIOS-core.BIN \
		EMUTOS-1a7484ab-BIOS-core-falcontc.BIN
done
cmpset measurement-2026-10-28-xbios-system-audit/runs/NEWTOS-xbios-system-r1/cmp/NEWTOS.BIN \
	xbios-system-r1.tsv EMUTOS-1a7484ab-XBIOS-system.BIN \
	EMUTOS-1a7484ab-XBIOS-system-falcontc.BIN
for d in \
	measurement-2026-10-28-tool07/runs/newtos-1/BIOS.BIN \
	measurement-2026-10-28-tool07/runs/newtos-2/BIOS.BIN \
	measurement-2026-10-28-tool08/runs/r2/cmp/NEWTOS.BIN \
	measurement-2026-10-28-tool08/runs-control/r2/cmp/NEWTOS.BIN \
	measurement-2026-10-28-tool09/runs/r2-disk-head/cmp/NEWTOS.BIN
do
	cmpset "$d" bios-r2.tsv EMUTOS-1a7484ab-BIOS-r2-disk.BIN
done
