# mkprofiles.awk -- tosdump/profiles.tsv as a C header for tosdiff (TOOL19).
#
#   awk -f mkprofiles.awk ../profiles.tsv > build/profiles.h
#
# Strict: a row that is not six tab-separated fields of the documented form,
# a runner/profile pair twice or a pointer width/word order twice ends the
# build (exit 1) -- a malformed definition is never compiled in with a guess.
BEGIN { FS = "\t"; n = 0; bad = 0 }
/^#/ || /^[ \t]*$/ { next }
{
	sub(/\r$/, "")
	if (NF != 6 || $1 !~ /^[a-z0-9-]+$/ || $2 !~ /^[a-z0-9-]+$/ ||
	    $3 !~ /^(4|8)$/ || $4 !~ /^(1|2)$/ || $5 !~ /^[0-9]+$/ ||
	    $5 + 0 > 32767 || $6 !~ /^[A-Z0-9]([A-Z0-9]([A-Z0-9])?)?$/) {
		printf "profiles.tsv line %d: not <runner> <profile> <4|8> <1|2> <unit 0..32767> <ext A-Z0-9, 1..3>\n", NR > "/dev/stderr"
		bad = 1; next
	}
	if (($1 SUBSEP $2) in pair) {
		printf "profiles.tsv line %d: %s %s twice\n", NR, $1, $2 > "/dev/stderr"
		bad = 1
	}
	if (($3 SUBSEP $4) in sig) {
		printf "profiles.tsv line %d: pointer %s, word order %s twice\n", NR, $3, $4 > "/dev/stderr"
		bad = 1
	}
	pair[$1 SUBSEP $2] = 1; sig[$3 SUBSEP $4] = 1
	row[n++] = sprintf("\t{ \"%s\", \"%s\", %d, %d, %d, \"%s\" },", $1, $2, $3, $4, $5 + 0, $6)
}
END {
	if (bad) exit 1
	if (n == 0) { print "profiles.tsv: no profile" > "/dev/stderr"; exit 1 }
	print "/* generated from tosdump/profiles.tsv by mkprofiles.awk -- do not edit */"
	print "#ifndef TOSDUMP_PROFILES_H"
	print "#define TOSDUMP_PROFILES_H"
	print "struct td_profile {"
	print "\tconst char *runner, *profile;"
	print "\tunsigned ptr_width, word_order;"
	print "\tlong unit;"
	print "\tconst char *ext;"
	print "};"
	print "static const struct td_profile td_profiles[] = {"
	for (i = 0; i < n; i++) print row[i]
	print "};"
	printf "#define TD_PROFILES_N %d\n", n
	print "#endif"
}
