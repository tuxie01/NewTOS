#!/bin/sh
#
# test_tosdiff.sh -- drives the comparator against synthetic dumps (spec T1).
#
# usage: test_tosdiff.sh <build-dir>
set -u

BUILD="${1:?usage: test_tosdiff.sh <build-dir>}"
HERE=$(cd "$(dirname "$0")" && pwd)
TMP="$BUILD/t"
TOSDIFF="$BUILD/tosdiff"
MKDUMP="$BUILD/mkdump"
CASES="$HERE/fixture-vdi.tsv"
CASES2="$HERE/fixture-vdi-other.tsv"

rm -rf "$TMP"
mkdir -p "$TMP"

fails=0
check() {   # check <description> <expected-status> <actual-status>
	if [ "$2" != "$3" ]; then
		echo "  FAIL $1: expected exit $2, got $3"
		fails=$((fails + 1))
	fi
}
grep_out() {   # grep_out <description> <file> <pattern>
	if ! grep -q -- "$3" "$2"; then
		echo "  FAIL $1: output does not mention '$3'"
		echo "  --- output ---"
		sed 's/^/  | /' "$2"
		echo "  --------------"
		fails=$((fails + 1))
	fi
}

echo "tosdiff against synthetic dumps"

# --- reference and an identical copy -----------------------------------
"$MKDUMP" "$TMP/a.bin" "$CASES"                     || { echo "  FAIL mkdump a"; fails=$((fails+1)); }
"$MKDUMP" "$TMP/b.bin" "$CASES"                     || { echo "  FAIL mkdump b"; fails=$((fails+1)); }

# --- validation ---------------------------------------------------------
"$TOSDIFF" --validate "$TMP/a.bin" >"$TMP/val.out" 2>&1
check "validate accepts a well-formed dump" 0 $?
grep_out "validate names the area" "$TMP/val.out" "VDI"

"$TOSDIFF" --validate "$TMP/a.bin" "$CASES" >"$TMP/val2.out" 2>&1
check "validate accepts dump + matching case table" 0 $?

"$TOSDIFF" --validate "$TMP/a.bin" "$CASES2" >"$TMP/val3.out" 2>&1
check "validate rejects a mismatched case table" 2 $?
grep_out "validate explains the CRC mismatch" "$TMP/val3.out" "CRC"

head -c 100 "$TMP/a.bin" > "$TMP/trunc.bin"
"$TOSDIFF" --validate "$TMP/trunc.bin" >"$TMP/val4.out" 2>&1
check "validate rejects a truncated dump" 2 $?

# --- identical dumps compare clean --------------------------------------
"$TOSDIFF" "$CASES" "$TMP/a.bin" "$TMP/b.bin" >"$TMP/same.out" 2>&1
check "identical dumps report no differences" 0 $?
grep_out "clean run says so" "$TMP/same.out" "0 of 3"

# --- one poisoned field differs, and tosdiff names it -------------------
# case index 1 keeps intout[3] poisoned in b, holds 0x0555 in a.
"$MKDUMP" "$TMP/c.bin" "$CASES" intout 1 3 -8531   # 0xDEAD as a signed int16
"$TOSDIFF" "$CASES" "$TMP/a.bin" "$TMP/c.bin" >"$TMP/diff.out" 2>&1
check "a single differing field is reported" 1 $?
grep_out "the field is named, not an offset" "$TMP/diff.out" "intout\[3\]"
grep_out "the case is named by id"           "$TMP/diff.out" "case 4"
grep_out "the case comment is shown"         "$TMP/diff.out" "v_line placeholder"
grep_out "both values are shown"             "$TMP/diff.out" "DEAD"
grep_out "summary counts the cases"          "$TMP/diff.out" "1 of 3"
if grep -q "intout\[0\]" "$TMP/diff.out"; then
	echo "  FAIL equal elements must not be reported"
	fails=$((fails + 1))
fi

# --- an input array that the call must not touch ------------------------
"$MKDUMP" "$TMP/d.bin" "$CASES" ptsin 0 0 999
"$TOSDIFF" "$CASES" "$TMP/a.bin" "$TMP/d.bin" >"$TMP/diff2.out" 2>&1
check "an input-array difference is reported" 1 $?
grep_out "ptsin is named" "$TMP/diff2.out" "ptsin\[0\]"

# --- refusals (spec section 3) ------------------------------------------
"$MKDUMP" "$TMP/crc.bin" "$CASES" --crc 12345678
"$TOSDIFF" "$CASES" "$TMP/a.bin" "$TMP/crc.bin" >"$TMP/ref1.out" 2>&1
check "differing case-table CRC is refused" 2 $?
grep_out "refusal mentions the CRC" "$TMP/ref1.out" "CRC"

"$MKDUMP" "$TMP/area.bin" "$CASES" --area 5
"$TOSDIFF" "$CASES" "$TMP/a.bin" "$TMP/area.bin" >"$TMP/ref2.out" 2>&1
check "differing area is refused" 2 $?
grep_out "refusal mentions the area" "$TMP/ref2.out" "area"

# --- an AES dump: the comment column is column SEVEN, not column five ----
# The stub AES dump and the table it came from are written by test_runner_aes
# earlier in the suite.  A single patched output word is all that is needed to
# make tosdiff print a case header, which is the thing under test here: with
# the five column VDI rule the "comment" would start at the n_addrout column
# and swallow the int_in values.
AESBIN="$BUILD/stub-aes.bin"
AESTSV="$BUILD/stub-aes.tsv"
if [ -f "$AESBIN" ] && [ -f "$AESTSV" ]; then
	cp "$AESBIN" "$TMP/aes-b.bin"
	# record 0, int_out[3]: 32 byte header + offset 76 + 3*2 = 114
	printf '\377\377' | dd of="$TMP/aes-b.bin" bs=1 seek=114 count=2 \
	    conv=notrunc >/dev/null 2>&1

	"$TOSDIFF" "$AESTSV" "$AESBIN" "$TMP/aes-b.bin" >"$TMP/aes.out" 2>&1
	check "an AES difference is reported" 1 $?
	grep_out "the AES field is named"  "$TMP/aes.out" "int_out\[3\]"
	grep_out "the AES case is named by id" "$TMP/aes.out" "case 2"
	grep_out "the AES comment is the comment" "$TMP/aes.out" \
	    "wind_calc WC_WORK"
	if grep -q "0x0FEF" "$TMP/aes.out"; then
		echo "  FAIL the AES case header holds the int_in column, so the"
		echo "       comment was taken with the five column VDI rule"
		fails=$((fails + 1))
	fi

	"$TOSDIFF" --validate "$AESBIN" "$AESTSV" >"$TMP/aesval.out" 2>&1
	check "validate accepts the AES dump against its table" 0 $?
	grep_out "validate names the AES area" "$TMP/aesval.out" "AES"
else
	echo "  FAIL stub AES dump missing: $AESBIN / $AESTSV"
	fails=$((fails + 1))
fi

# --- the raster area: a field that is recorded and never compared --------
#
# contrl[7..14] hold the two MFDB pointers, at the target's own pointer width
# and pointing at the dumper's own buffers.  They cannot be equal across the
# two machines, so a difference there would measure the linker; the field
# descriptor puts them in field class address, which is never compared by
# value.  Prove it end to end: poke one word of a compared field and one word
# of an address field into the same record, and only the first may be
# reported as a value difference.  (The stub's pointers are HOST addresses,
# above 4 GiB on this machine, so the address check reports them -- as
# address verdicts, "contrl_mfdb[7..10]", never as a value line
# "contrl_mfdb[7] ...".)
RASTBIN="$BUILD/stub-raster.bin"
RASTTSV="$BUILD/stub-raster.tsv"
if [ -f "$RASTBIN" ] && [ -f "$RASTTSV" ]; then
	# buf_a[0] is at record offset 1636; contrl[7] is at 8 + 7*2 = 22.
	cp "$RASTBIN" "$TMP/r_a.bin"
	cp "$RASTBIN" "$TMP/r_b.bin"
	printf '\125\125' | dd of="$TMP/r_b.bin" bs=1 seek=$((32 + 1636)) \
	    conv=notrunc status=none
	printf '\052\052' | dd of="$TMP/r_b.bin" bs=1 seek=$((32 + 22)) \
	    conv=notrunc status=none

	"$TOSDIFF" "$RASTTSV" "$TMP/r_a.bin" "$TMP/r_b.bin" >"$TMP/rast.out" 2>&1
	check "the raster dumps differ" 1 $?
	grep_out "the compared buffer word is reported" "$TMP/rast.out" "buf_a\\[0\\]"
	if grep -q "contrl_mfdb\[[0-9]*\] " "$TMP/rast.out"; then
		echo "  FAIL the MFDB pointer slots were compared by value; they"
		echo "       are field class address"
		fails=$((fails + 1))
	fi

	"$TOSDIFF" --validate "$RASTBIN" >"$TMP/rastval.out" 2>&1
	check "validate accepts the raster dump" 0 $?
	grep_out "validate names the raster area" "$TMP/rastval.out" "VDI-RASTER"

	# --- the kept check: the VDI must not change the MFDB pointers ---------
	#
	# Within one side: every address cell of contrl_mfdb (after the call)
	# must equal its twin in contrl_before_mfdb (before the call).  The stub
	# dump is format 3, eight-byte pointers, least significant word first
	# (asserted below): cell 0 = contrl[7..10] at record offset 22, cell 1 =
	# contrl[11..14] at 30; the before twins at 52 and 60.  The stub's own
	# pointers are host addresses that fail the address check, so every cell
	# is first set to 0x1000 (bytes 10 00 00 .. in this word order): the
	# kept check then decides the exit status alone.
	grep_out "the stub raster dump has eight-byte pointers, LSW first" \
	    "$TMP/rastval.out" "pointer 8 bytes, least significant word first"
	# record count, header WORD at offset 10 (od's column layout differs
	# between BSD and GNU, so let the shell split the two bytes)
	set -- $(od -An -tu1 -j10 -N2 "$RASTBIN")
	nrec=$(($1 * 256 + $2))
	cp "$RASTBIN" "$TMP/r_lo.bin"
	r=0
	while [ "$r" -lt "$nrec" ]; do
		for o in 22 30 52 60; do
			printf '\020\000\000\000\000\000\000\000' | dd of="$TMP/r_lo.bin" \
			    bs=1 seek=$((32 + r * 3696 + o)) conv=notrunc status=none
		done
		r=$((r + 1))
	done
	"$TOSDIFF" "$RASTTSV" "$TMP/r_lo.bin" "$TMP/r_lo.bin" >"$TMP/kept0.out" 2>&1
	check "pointers the call left alone pass the kept check" 0 $?
	grep_out "the kept check states what it compared" "$TMP/kept0.out" \
	    "kept check, r_lo.bin: format 3; contrl_mfdb against contrl_before_mfdb, $((nrec * 2)) cells checked, 0 changed by the call"

	# record 0, cell 1 after the call: 0x1000 -> 0x2000.  B only.
	cp "$TMP/r_lo.bin" "$TMP/r_chg.bin"
	printf '\040\000' | dd of="$TMP/r_chg.bin" bs=1 seek=$((32 + 30)) \
	    conv=notrunc status=none
	"$TOSDIFF" "$RASTTSV" "$TMP/r_lo.bin" "$TMP/r_chg.bin" >"$TMP/kept1.out" 2>&1
	check "a pointer the call changed is flagged" 1 $?
	grep_out "the kept violation names the cell and both values" \
	    "$TMP/kept1.out" \
	    "contrl_mfdb\[4..7\] *KEPT r_chg.bin *= 0x0000000000002000, before the call 0x0000000000001000"
	grep_out "the kept violation is counted on its side" "$TMP/kept1.out" \
	    "kept check, r_chg.bin: format 3; .* 1 changed by the call"
	grep_out "the other side stays clean" "$TMP/kept1.out" \
	    "kept check, r_lo.bin: format 3; .* 0 changed by the call"
	grep_out "a kept violation is not a value difference" "$TMP/kept1.out" \
	    "0 of $nrec cases differ"
	grep_out "nor an address violation" "$TMP/kept1.out" \
	    "r_chg.bin: format 3, .* 0 not < 0x80000000"

	# the same change on both sides: no value difference between the two
	# dumps, and still a violation on each -- the check is within one side
	"$TOSDIFF" "$RASTTSV" "$TMP/r_chg.bin" "$TMP/r_chg.bin" >"$TMP/kept2.out" 2>&1
	check "an identical change on both sides is still flagged" 1 $?
	if [ "$(grep -c "1 changed by the call" "$TMP/kept2.out")" != 2 ]; then
		echo "  FAIL the kept check must flag each side on its own"
		fails=$((fails + 1))
	fi

	# a changed BEFORE twin breaks the relation as well
	cp "$TMP/r_lo.bin" "$TMP/r_bef.bin"
	printf '\040\000' | dd of="$TMP/r_bef.bin" bs=1 seek=$((32 + 52)) \
	    conv=notrunc status=none
	"$TOSDIFF" "$RASTTSV" "$TMP/r_lo.bin" "$TMP/r_bef.bin" >"$TMP/kept3.out" 2>&1
	check "a differing before twin is flagged" 1 $?
	grep_out "the before twin's cell is named" "$TMP/kept3.out" \
	    "contrl_mfdb\[0..3\] *KEPT r_bef.bin"

	# format 2: pointer width and word order unknown, so not checkable
	for n in lo chg; do
		cp "$TMP/r_$n.bin" "$TMP/r_v2$n.bin"
		printf '\000\002' | dd of="$TMP/r_v2$n.bin" bs=1 seek=4 \
		    conv=notrunc status=none
		printf '\000\000\000\000' | dd of="$TMP/r_v2$n.bin" bs=1 seek=20 \
		    conv=notrunc status=none
	done
	"$TOSDIFF" "$RASTTSV" "$TMP/r_v2lo.bin" "$TMP/r_v2chg.bin" >"$TMP/kept4.out" 2>&1
	check "format 2: the kept check does not decide the exit status" 0 $?
	grep_out "format 2: the kept check is not checkable" "$TMP/kept4.out" \
	    "kept check, r_v2chg.bin: not checkable"
	if grep -q "KEPT" "$TMP/kept4.out"; then
		echo "  FAIL a format 2 pointer cell was read for the kept check"
		fails=$((fails + 1))
	fi
else
	echo "  FAIL stub raster dump missing: $RASTBIN / $RASTTSV"
	fails=$((fails + 1))
fi

# --- format 3: the address check (docs/TOS-REFERENCE-DUMPER.md section 3) --
#
# The stub AES dump was written by td_dump_write on this host, so it is
# format 3 and states the host's pointer width and word order.  ap_3resv,
# global[11..12], is record offset 14 + 22 = 36, file offset 68.  On this
# host (little-endian) the LEAST significant WORD comes first, so the LONG
# 0x80000000 is the WORDs 0x0000 0x8000 and 0x7FFFFFFF is 0xFFFF 0x7FFF.
# Asserted first, so a big-endian host fails loudly instead of testing the
# wrong bytes.
if [ -f "$AESBIN" ] && [ -f "$AESTSV" ]; then
	"$TOSDIFF" --validate "$AESBIN" >"$TMP/v3val.out" 2>&1
	check "validate accepts a format 3 dump" 0 $?
	grep_out "validate states the format"      "$TMP/v3val.out" "format 3"
	grep_out "validate states the word order"  "$TMP/v3val.out" "least significant word first"

	cp "$AESBIN" "$TMP/aes-max.bin"
	printf '\377\377\177\377' | dd of="$TMP/aes-max.bin" bs=1 seek=68 \
	    conv=notrunc status=none
	cp "$AESBIN" "$TMP/aes-lim.bin"
	printf '\000\000\200\000' | dd of="$TMP/aes-lim.bin" bs=1 seek=68 \
	    conv=notrunc status=none

	# 0x7FFFFFFF is the largest address there is: passes on both sides,
	# and an address cell is not compared by value, so this is clean
	"$TOSDIFF" "$AESTSV" "$AESBIN" "$TMP/aes-max.bin" >"$TMP/amax.out" 2>&1
	check "an address cell at 0x7FFFFFFF passes" 0 $?
	grep_out "the max case says what it checked" "$TMP/amax.out" \
	    "aes-max.bin: format 3, pointer [48] bytes, least significant word first; 3 address cells checked, 0 not < 0x80000000"
	grep_out "the max case has no value difference" "$TMP/amax.out" "0 of 3"

	# 0x80000000 reads as an error code: its own verdict, not a mismatch
	"$TOSDIFF" "$AESTSV" "$AESBIN" "$TMP/aes-lim.bin" >"$TMP/alim.out" 2>&1
	check "an address cell at 0x80000000 is flagged" 1 $?
	grep_out "the violation names the cell" "$TMP/alim.out" "global\[11..12\]"
	grep_out "the violation shows the value" "$TMP/alim.out" "0x80000000"
	grep_out "the violation is an address verdict" "$TMP/alim.out" \
	    "ADDRESS aes-lim.bin *= 0x80000000, not < 0x80000000"
	grep_out "the violation is not a value difference" "$TMP/alim.out" "0 of 3"
	grep_out "the violation is counted per side" "$TMP/alim.out" \
	    "aes-lim.bin: format 3, .* 3 address cells checked, 1 not < 0x80000000"
	if grep -q "global\[11\] " "$TMP/alim.out"; then
		echo "  FAIL ap_3resv was compared by value"
		fails=$((fails + 1))
	fi

	# --- format 2 stays readable, and says it cannot check ----------------
	# Forge format 2 copies: version WORD 2, and the four bytes format 3
	# uses for pointer width and word order back to zero (reserved in 2).
	for n in a lim; do
		case $n in a) src="$AESBIN" ;; lim) src="$TMP/aes-lim.bin" ;; esac
		cp "$src" "$TMP/aes-v2$n.bin"
		printf '\000\002' | dd of="$TMP/aes-v2$n.bin" bs=1 seek=4 \
		    conv=notrunc status=none
		printf '\000\000\000\000' | dd of="$TMP/aes-v2$n.bin" bs=1 seek=20 \
		    conv=notrunc status=none
	done
	"$TOSDIFF" --validate "$TMP/aes-v2a.bin" "$AESTSV" >"$TMP/v2val.out" 2>&1
	check "validate accepts a format 2 dump" 0 $?
	grep_out "validate states format 2" "$TMP/v2val.out" "format 2"

	"$TOSDIFF" "$AESTSV" "$TMP/aes-v2a.bin" "$TMP/aes-v2lim.bin" >"$TMP/v2v2.out" 2>&1
	check "two format 2 dumps still compare" 0 $?
	grep_out "format 2: the address check is not checkable" "$TMP/v2v2.out" \
	    "aes-v2lim.bin: not checkable"
	grep_out "format 2: both sides say so" "$TMP/v2v2.out" \
	    "aes-v2a.bin: not checkable"
	if grep -q "ADDRESS" "$TMP/v2v2.out"; then
		echo "  FAIL a format 2 address cell was read: width and word order"
		echo "       are not stated, so any reading is a guess"
		fails=$((fails + 1))
	fi

	# --- format 3 against format 2 is refused, either way round ------------
	"$TOSDIFF" "$AESTSV" "$AESBIN" "$TMP/aes-v2a.bin" >"$TMP/v3v2.out" 2>&1
	check "format 3 against format 2 is refused" 2 $?
	grep_out "the refusal names both formats" "$TMP/v3v2.out" \
	    "refusing: stub-aes.bin is format 3, aes-v2a.bin is format 2"
	"$TOSDIFF" "$AESTSV" "$TMP/aes-v2a.bin" "$AESBIN" >"$TMP/v2v3.out" 2>&1
	check "format 2 against format 3 is refused" 2 $?
	grep_out "the refusal names both formats, reversed" "$TMP/v2v3.out" \
	    "refusing: aes-v2a.bin is format 2, stub-aes.bin is format 3"
else
	echo "  FAIL stub AES dump missing: $AESBIN / $AESTSV"
	fails=$((fails + 1))
fi

# --- the call area: retclass and capture (task T7) ---------------------
#
# The stub BIOS dump and its table come from test_runner_call earlier in the
# suite.  Record i of case table line i+2; a record is 320 bytes after the
# 32 byte header: ret at 36, capture at 40, ret_hi at 296, trap at 300.
#   0 Bconstat EXACT      1 Physbase PTR_RAM     2 Protobt IGNORE
#   3 Dsp_Available IGNORE  4 NVMaccess EXACT.W  5 EgetPalette IGNORE
#   6 Getrez EXACT.W      7 Drvmap NONZERO       8 VgetMonitor HANDLE.W
CALLBIN="$BUILD/stub-bios.bin"
CALLTSV="$BUILD/stub-bios.tsv"
cpatch() {   # cpatch <out> <record> <offset> <printf-bytes>  (from CALLBIN)
	[ -f "$1" ] || cp "$CALLBIN" "$1"
	printf "$4" | dd of="$1" bs=1 seek=$((32 + $2 * 320 + $3)) \
	    conv=notrunc status=none
}
if [ -f "$CALLBIN" ] && [ -f "$CALLTSV" ]; then
	"$TOSDIFF" --validate "$CALLBIN" "$CALLTSV" >"$TMP/cval.out" 2>&1
	check "validate accepts the BIOS dump against its table" 0 $?
	grep_out "validate names the BIOS area" "$TMP/cval.out" "area BIOS"
	grep_out "validate matches the table" "$TMP/cval.out" "matches stub-bios.tsv (9 cases)"

	"$TOSDIFF" "$CALLTSV" "$CALLBIN" "$CALLBIN" >"$TMP/csame.out" 2>&1
	check "identical BIOS dumps compare clean" 0 $?
	grep_out "the BIOS area is compared, not refused" "$TMP/csame.out" \
	    "0 of 9 cases differ"

	# EXACT: the low half differs
	rm -f "$TMP/c_ex.bin"; cpatch "$TMP/c_ex.bin" 0 36 '\000\000\000\000'
	"$TOSDIFF" "$CALLTSV" "$CALLBIN" "$TMP/c_ex.bin" >"$TMP/cex.out" 2>&1
	check "an EXACT return difference is reported" 1 $?
	grep_out "EXACT names the class" "$TMP/cex.out" "ret .*EXACT"
	grep_out "EXACT names the case" "$TMP/cex.out" "case 2 "
	grep_out "the comment is column five" "$TMP/cex.out" "Bconstat CON"
	grep_out "EXACT counts the case" "$TMP/cex.out" "1 of 9 cases differ"

	# EXACT at LONG width is the 64-bit register: -1 zero-extended differs
	rm -f "$TMP/c_zx.bin"; cpatch "$TMP/c_zx.bin" 0 296 '\000\000\000\000'
	"$TOSDIFF" "$CALLTSV" "$CALLBIN" "$TMP/c_zx.bin" >"$TMP/czx.out" 2>&1
	check "a LONG returned zero-extended is a difference" 1 $?
	grep_out "the 64-bit value is shown" "$TMP/czx.out" "0x00000000FFFFFFFF"

	# EXACT.W: only the WORD is the contract
	rm -f "$TMP/c_wh.bin"; cpatch "$TMP/c_wh.bin" 6 36 '\377\377'
	"$TOSDIFF" "$CALLTSV" "$CALLBIN" "$TMP/c_wh.bin" >"$TMP/cwh.out" 2>&1
	check "EXACT.W ignores d0 above the WORD" 0 $?
	rm -f "$TMP/c_wl.bin"; cpatch "$TMP/c_wl.bin" 6 38 '\022\064'
	"$TOSDIFF" "$CALLTSV" "$CALLBIN" "$TMP/c_wl.bin" >"$TMP/cwl.out" 2>&1
	check "EXACT.W compares the WORD" 1 $?
	grep_out "the WORD is shown" "$TMP/cwl.out" "0x1234"

	# IGNORE
	rm -f "$TMP/c_ig.bin"; cpatch "$TMP/c_ig.bin" 2 36 '\001\002\003\004'
	cpatch "$TMP/c_ig.bin" 2 296 '\005\006\007\010'
	"$TOSDIFF" "$CALLTSV" "$CALLBIN" "$TMP/c_ig.bin" >"$TMP/cig.out" 2>&1
	check "an IGNORE return is not compared" 0 $?

	# PTR_RAM, per side: misaligned, null, at the limit, above 4 GiB
	rm -f "$TMP/c_pa.bin"; cpatch "$TMP/c_pa.bin" 1 36 '\000\253\315\002'
	"$TOSDIFF" "$CALLTSV" "$CALLBIN" "$TMP/c_pa.bin" >"$TMP/cpa.out" 2>&1
	check "PTR_RAM: a misaligned pointer is flagged" 1 $?
	grep_out "PTR_RAM names the side and the reason" "$TMP/cpa.out" \
	    "ret .*PTR_RAM c_pa.bin *= .*not a multiple of 4"
	if grep -q "PTR_RAM stub-bios.bin" "$TMP/cpa.out"; then
		echo "  FAIL PTR_RAM flagged the side that is fine"
		fails=$((fails + 1))
	fi
	rm -f "$TMP/c_p0.bin"; cpatch "$TMP/c_p0.bin" 1 36 '\000\000\000\000'
	"$TOSDIFF" "$CALLTSV" "$CALLBIN" "$TMP/c_p0.bin" >"$TMP/cp0.out" 2>&1
	check "PTR_RAM: a null pointer is flagged" 1 $?
	grep_out "PTR_RAM null" "$TMP/cp0.out" "PTR_RAM c_p0.bin .*is null"
	rm -f "$TMP/c_pl.bin"; cpatch "$TMP/c_pl.bin" 1 36 '\200\000\000\000'
	"$TOSDIFF" "$CALLTSV" "$CALLBIN" "$TMP/c_pl.bin" >"$TMP/cpl.out" 2>&1
	check "PTR_RAM: 0x80000000 is flagged" 1 $?
	grep_out "PTR_RAM limit" "$TMP/cpl.out" "PTR_RAM c_pl.bin .*not < 0x80000000"
	rm -f "$TMP/c_ph.bin"; cpatch "$TMP/c_ph.bin" 1 296 '\000\000\000\001'
	"$TOSDIFF" "$CALLTSV" "$CALLBIN" "$TMP/c_ph.bin" >"$TMP/cph.out" 2>&1
	check "PTR_RAM: a pointer above 4 GiB is flagged" 1 $?
	grep_out "PTR_RAM high half" "$TMP/cph.out" \
	    "PTR_RAM c_ph.bin *= 0x0000000100ABCD00, not < 0x80000000"
	# the same violation on both sides: equal values, still a violation
	"$TOSDIFF" "$CALLTSV" "$TMP/c_pa.bin" "$TMP/c_pa.bin" >"$TMP/cpp.out" 2>&1
	check "PTR_RAM: a violation on both sides is still flagged" 1 $?
	if [ "$(grep -c "PTR_RAM c_pa.bin" "$TMP/cpp.out")" != 2 ]; then
		echo "  FAIL PTR_RAM must judge each side on its own"
		fails=$((fails + 1))
	fi
	grep_out "PTR_RAM counts the case" "$TMP/cpp.out" "1 of 9 cases differ"

	# NONZERO and HANDLE.W
	rm -f "$TMP/c_nz.bin"; cpatch "$TMP/c_nz.bin" 7 36 '\000\000\000\000'
	cpatch "$TMP/c_nz.bin" 7 296 '\000\000\000\000'
	"$TOSDIFF" "$CALLTSV" "$CALLBIN" "$TMP/c_nz.bin" >"$TMP/cnz.out" 2>&1
	check "NONZERO: zero is flagged" 1 $?
	grep_out "NONZERO names the reason" "$TMP/cnz.out" "NONZERO c_nz.bin .*is zero"
	rm -f "$TMP/c_hd.bin"; cpatch "$TMP/c_hd.bin" 8 38 '\377\376'
	"$TOSDIFF" "$CALLTSV" "$CALLBIN" "$TMP/c_hd.bin" >"$TMP/chd.out" 2>&1
	check "HANDLE.W: a negative WORD is flagged" 1 $?
	grep_out "HANDLE names the reason" "$TMP/chd.out" "HANDLE c_hd.bin .*is negative"
	rm -f "$TMP/c_hp.bin"; cpatch "$TMP/c_hp.bin" 8 36 '\177\377\000\001'
	"$TOSDIFF" "$CALLTSV" "$CALLBIN" "$TMP/c_hp.bin" >"$TMP/chp.out" 2>&1
	check "HANDLE: a different non-negative value is no difference" 0 $?

	# capture, trap and args are compared by value and named
	rm -f "$TMP/c_cp.bin"; cpatch "$TMP/c_cp.bin" 2 43 '\000'
	"$TOSDIFF" "$CALLTSV" "$CALLBIN" "$TMP/c_cp.bin" >"$TMP/ccp.out" 2>&1
	check "a capture difference is reported" 1 $?
	grep_out "the capture byte is named" "$TMP/ccp.out" "capture\[3\]"
	rm -f "$TMP/c_tr.bin"; cpatch "$TMP/c_tr.bin" 6 300 '\000\015'
	"$TOSDIFF" "$CALLTSV" "$CALLBIN" "$TMP/c_tr.bin" >"$TMP/ctr.out" 2>&1
	check "a trap difference is reported" 1 $?
	grep_out "the trap is named" "$TMP/ctr.out" "trap\[0\]"
	rm -f "$TMP/c_ar.bin"; cpatch "$TMP/c_ar.bin" 0 4 '\000\000\000\003'
	"$TOSDIFF" "$CALLTSV" "$CALLBIN" "$TMP/c_ar.bin" >"$TMP/car.out" 2>&1
	check "an argument difference is reported" 1 $?
	grep_out "the argument is named" "$TMP/car.out" "args\[0\]"
	if grep -q "ret_hi\|reserved" "$TMP/car.out"; then
		echo "  FAIL ret_hi or the reserved bytes were compared as a field"
		fails=$((fails + 1))
	fi
else
	echo "  FAIL stub BIOS dump missing: $CALLBIN / $CALLTSV"
	fails=$((fails + 1))
fi

if [ "$fails" -eq 0 ]; then
	echo "  ok"
	exit 0
fi
echo "  $fails failure(s)"
exit 1
