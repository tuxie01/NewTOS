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
if [ $fails -ne 0 ]; then
	echo "  $fails failure(s)"
	exit 1
fi
echo "  ok ($n references, every relational rule holds on the oracle)"
