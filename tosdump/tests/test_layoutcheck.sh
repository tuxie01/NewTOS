#!/bin/sh
#
# test_layoutcheck.sh -- the layouts of the case tables are the specs'
# (TASK/TOOL16): every "!layout" line in tosdump/cases/*.tsv must be, byte
# for byte, a line of a ```tosdump-layout block in docs/api/structures/,
# and every such block must parse (the C-layout check of td_layout_parse).
# Then three negative controls: a table line that differs from the spec, a
# table layout no spec has, and a spec block whose offset is not C's.
#
# usage: test_layoutcheck.sh <build-dir>
set -u

BUILD="${1:?usage: test_layoutcheck.sh <build-dir>}"
HERE=$(cd "$(dirname "$0")" && pwd)
CHECK="$BUILD/layoutcheck"
SPECS="$HERE/../../docs/api/structures"
CASES="$HERE/../cases"
TMP="$BUILD/layoutcheck-tmp"
fails=0

rm -rf "$TMP"
mkdir -p "$TMP"

out=$("$CHECK" "$SPECS"/*.md -- "$CASES"/*.tsv 2>&1)
rc=$?
echo "$out"
if [ $rc -ne 0 ]; then
	echo "  FAIL the tables' layouts are not the specs'"
	fails=$((fails + 1))
fi

# a table line with a different class than the spec's
iorec=$(grep -h '^!layout	iorec	' "$SPECS"/iorec.md | head -1)
if [ -z "$iorec" ]; then
	echo "  FAIL docs/api/structures/iorec.md has no iorec layout line"
	fails=$((fails + 1))
else
	printf '%s\n' "$iorec" | sed 's/ibufhd:w@6\/10/ibufhd:w@6\/10:IGNORE/' \
	    >"$TMP/differs.tsv"
	"$CHECK" "$SPECS"/*.md -- "$TMP/differs.tsv" >"$TMP/differs.out" 2>&1
	if [ $? -ne 1 ] || ! grep -q "is not the spec's line" "$TMP/differs.out"; then
		echo "  FAIL a table layout that differs from the spec passes"
		fails=$((fails + 1))
	fi
fi

printf '!layout\tnosuch\t2/2\ta:w@0/0\n' >"$TMP/nosuch.tsv"
"$CHECK" "$SPECS"/*.md -- "$TMP/nosuch.tsv" >"$TMP/nosuch.out" 2>&1
if [ $? -ne 1 ] || ! grep -q "is in no docs/api/ spec" "$TMP/nosuch.out"; then
	echo "  FAIL a table layout no spec has passes"
	fails=$((fails + 1))
fi

printf '```tosdump-layout\n!layout\tbad\t8/16\ta:w@0/0,b:p@2/4\n```\n' \
    >"$TMP/bad.md"
"$CHECK" "$TMP/bad.md" -- >"$TMP/bad.out" 2>&1
if [ $? -ne 1 ] || ! grep -q "LP64 offset is not where C puts" "$TMP/bad.out"; then
	echo "  FAIL a spec block whose offset is not C's passes"
	fails=$((fails + 1))
fi

rm -rf "$TMP"
if [ $fails -ne 0 ]; then
	echo "  $fails failure(s)"
	exit 1
fi
echo "  ok (a differing line, an unknown layout and a non-C offset are refused)"
