#!/bin/sh
#
# test_selfcheck.sh -- a comparison rule is only valid if the oracle passes
# it.  Every pinned EmuTOS reference (format 3) is run through
# `tosdiff --selfcheck` against its case table, found by CRC; a violation is
# a bug in a rule or a case table (route: Researcher), never a NewTOS finding.
#
# usage: test_selfcheck.sh <build-dir>
set -u

BUILD="${1:?usage: test_selfcheck.sh <build-dir>}"
HERE=$(cd "$(dirname "$0")" && pwd)
TOSDIFF="$BUILD/tosdiff"
REFS="$HERE/../reference"
CASES="$HERE/../cases"

fails=0
n=0
for ref in "$REFS"/EMUTOS-1a7484ab-*.BIN; do
	[ -f "$ref" ] || continue
	n=$((n + 1))
	matched=""
	for t in "$CASES"/*.tsv; do
		out=$("$TOSDIFF" --selfcheck "$t" "$ref" 2>&1)
		rc=$?
		[ $rc -eq 2 ] && continue   # refused: not this dump's table
		if [ -n "$matched" ]; then
			echo "  FAIL $(basename "$ref"): two tables match ($matched, $(basename "$t"))"
			fails=$((fails + 1))
			continue
		fi
		matched=$(basename "$t")
		if [ $rc -ne 0 ]; then
			echo "  FAIL $(basename "$ref") against $matched: the oracle fails a rule"
			echo "$out" | sed 's/^/  | /'
			fails=$((fails + 1))
		fi
	done
	if [ -z "$matched" ]; then
		echo "  FAIL $(basename "$ref"): no case table in tosdump/cases matches its CRC"
		fails=$((fails + 1))
	fi
done

if [ $n -eq 0 ]; then
	echo "  FAIL no pinned reference found under $REFS"
	fails=$((fails + 1))
fi

# The negative control (TOOL11): the same rule must refuse the oracle's own
# dump once a pointer answer is replaced by the call's function number.
# Iorec (XBIOS 14) is function 0x0E; its first record in the TT BIOS
# reference gets ret = 0x0000000E (ret_hi stays 0, a positive LONG).  A
# record is 320 bytes after the 32-byte header: opcode at 2, ret at 36,
# trap at 300 (core/tosdump.h, the call record).
NEG_REF="$REFS/EMUTOS-1a7484ab-BIOS.BIN"
NEG_TSV="$CASES/bios-r1.tsv"
NEG_DIR="$BUILD/selfcheck-neg"
rm -rf "$NEG_DIR"
mkdir -p "$NEG_DIR"
NEG="$NEG_DIR/iorec-fn.bin"
cp "$NEG_REF" "$NEG"
nrec=$(( ($(wc -c <"$NEG") - 32) / 320 ))
k=0
found=""
while [ $k -lt $nrec ]; do
	at=$((32 + k * 320))
	op=$(od -An -tx1 -j $((at + 2)) -N2 "$NEG" | tr -d ' \n')
	tr=$(od -An -tx1 -j $((at + 300)) -N2 "$NEG" | tr -d ' \n')
	if [ "$op" = "000e" ] && [ "$tr" = "000e" ]; then
		found=$k
		break
	fi
	k=$((k + 1))
done
if [ -z "$found" ]; then
	echo "  FAIL negative control: no Iorec record in $(basename "$NEG_REF")"
	fails=$((fails + 1))
else
	printf '\000\000\000\016' | dd of="$NEG" bs=1 \
	    seek=$((32 + found * 320 + 36)) conv=notrunc status=none
	out=$("$TOSDIFF" --selfcheck "$NEG_TSV" "$NEG" 2>&1)
	rc=$?
	if [ $rc -ne 1 ]; then
		echo "  FAIL negative control: the function number 0x0E as Iorec's pointer passes (exit $rc)"
		echo "$out" | sed 's/^/  | /'
		fails=$((fails + 1))
	elif ! echo "$out" | grep -q "PTR_RAM iorec-fn.bin .*= 0x000000000000000E, below 0x400"; then
		echo "  FAIL negative control: flagged, but not for the lower bound"
		echo "$out" | sed 's/^/  | /'
		fails=$((fails + 1))
	fi
fi
rm -rf "$NEG_DIR"
if [ $fails -ne 0 ]; then
	echo "  $fails failure(s)"
	exit 1
fi
echo "  ok ($n references, every relational rule holds on the oracle; a function number as pointer is refused)"
