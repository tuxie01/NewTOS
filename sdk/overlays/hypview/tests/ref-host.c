/*
 * ref-host.c -- host harness for HypView's REF reader (hyp/ref.c: ref_load,
 * ref_findnode, ref_findall), TASK/SDK04 probe S2 (finding 6 of
 * docs/analysis/byte-order-survey-2026-10-29.md). Built and run by
 * ref-host.sh beside this file.
 *
 * This file is the translation unit: it pulls in mintlib's <osbind.h>,
 * replaces the GEMDOS calls ref.c makes (Fread, Fseek, Malloc, Mfree -- svc
 * traps in the SDK) with a fake open file in host memory, and then
 * #includes the ref.c under test (REF_C, a quoted path) unchanged.
 *
 * The REF format is what the m68k code reads (ST-Guide reference file):
 * "HREF", then modules of {LONG module_len, LONG num_entries, entries}, ended
 * by a module header with num_entries 0; an entry is {BYTE type, BYTE size,
 * NUL-terminated name, extra}, all LONG/WORD big-endian, a label's line the
 * big-endian WORD in the entry's last two bytes. The checks are the m68k
 * results of the upstream code (read from ref.c), byte for byte -- including
 * the alias quirk: ref_findnode returns as an alias's line the WORD that
 * starts at the alias's NUL, i.e. the next entry's type byte.
 *
 * Data: a synthetic two-module file built here (always), and the real
 * reference files of the pinned FreeMiNT clone if present
 * (references/freemint/doc/{programmer/mint-prg,german/mint-de}.ref; their
 * expected values were read from the files' bytes).
 *
 * The host must be LP64 little-endian like the target (macOS arm64 is);
 * otherwise the harness refuses (exit 77). Exit status: number of failed
 * checks.
 */
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include <osbind.h>

static long host_fread(short h, long n, void *buf);
static long host_fseek(long off, short h, short mode);

#undef Fread
#undef Fseek
#undef Malloc
#undef Mfree
#define Fread(h, n, b)		host_fread((short)(h), (long)(n), (void *)(b))
#define Fseek(off, h, mode)	host_fseek((long)(off), (short)(h), (short)(mode))
#define Malloc(n)		malloc((size_t)(n))
#define Mfree(p)		(free((void *)(p)), 0)

#ifndef REF_C
#error "REF_C (quoted path of the ref.c under test) not defined"
#endif
#include REF_C

/* HypView's own Debug() lives in debug.c; the harness records the message. */
static char last_debug[256];
void Debug(const char *fmt, ...)
{
	va_list ap;

	va_start(ap, fmt);
	vsnprintf(last_debug, sizeof last_debug, fmt, ap);
	va_end(ap);
}

/* ---- the fake open file ---------------------------------------------------- */

static const unsigned char *file_data;
static long file_len, file_pos;

static long host_fread(short h, long n, void *buf)
{
	long k = file_len - file_pos;

	(void)h;
	if (n < k)
		k = n;
	if (k < 0)
		k = 0;
	memcpy(buf, file_data + file_pos, (size_t)k);
	file_pos += k;
	return k;
}

static long host_fseek(long off, short h, short mode)
{
	(void)h;
	switch (mode) {
	case 0: file_pos = off; break;
	case 1: file_pos += off; break;
	case 2: file_pos = file_len + off; break;
	}
	return file_pos;
}

static REF_FILE *load(const unsigned char *data, long len)
{
	file_data = data;
	file_len = len;
	file_pos = 0;
	last_debug[0] = 0;
	return ref_load(1);
}

/* ---- checks --------------------------------------------------------------- */

static int fails, checks;

static void check(int ok, const char *what)
{
	checks++;
	if (!ok) {
		fails++;
		printf("FAIL  %s\n", what);
	} else
		printf("ok    %s\n", what);
}

static void check_node(REF_FILE *ref, const char *name, int found, long node, short line)
{
	char what[160];
	long n = -99;
	short l = -99;
	short r;

	if (ref == NIL || ref == NULL) {
		snprintf(what, sizeof what, "ref_findnode(\"%s\") -- no REF loaded", name);
		check(0, what);
		return;
	}
	r = ref_findnode(ref, (char *)name, &n, &l);
	if (found)
		snprintf(what, sizeof what, "ref_findnode(\"%s\") = TRUE, node %ld, line %d (got %d, %ld, %d)",
			 name, node, line, r, n, l);
	else
		snprintf(what, sizeof what, "ref_findnode(\"%s\") = FALSE (got %d)", name, r);
	check(found ? (r && n == node && l == line) : !r, what);
}

struct hit { const char *path, *node, *dbase; unsigned short line; short is_label; };

static void check_all(REF_FILE *ref, const char *name, const struct hit *want, int nwant)
{
	char what[256];
	RESULT_ENTRY *e;
	short n = -1;
	int i, ok;

	if (ref == NIL || ref == NULL) {
		snprintf(what, sizeof what, "ref_findall(\"%s\") -- no REF loaded", name);
		check(0, what);
		return;
	}
	e = ref_findall(ref, (char *)name, &n);
	snprintf(what, sizeof what, "ref_findall(\"%s\") count %d (got %d)", name, nwant, n);
	check(n == nwant, what);
	for (i = 0; i < nwant; i++, e = e ? e->next : NULL) {
		ok = e != NULL &&
		     strcmp(e->path, want[i].path) == 0 &&
		     strcmp(e->node_name, want[i].node) == 0 &&
		     strcmp(e->dbase_description, want[i].dbase) == 0 &&
		     e->line == want[i].line && e->is_label == want[i].is_label;
		snprintf(what, sizeof what,
			 "ref_findall(\"%s\")[%d] = {%s, %s, \"%s\", line %u, label %d} (got {%s, %s, \"%s\", %u, %d})",
			 name, i, want[i].path, want[i].node, want[i].dbase, want[i].line, want[i].is_label,
			 e ? e->path : "-", e ? e->node_name : "-", e ? e->dbase_description : "-",
			 e ? e->line : 0, e ? e->is_label : -1);
		check(ok, what);
	}
}

/* ---- the synthetic file ---------------------------------------------------- */

static unsigned char syn[512];
static long syn_len;

static void put8(unsigned v) { syn[syn_len++] = (unsigned char)v; }
static void put32(unsigned long v) { put8(v >> 24); put8(v >> 16); put8(v >> 8); put8(v); }
static long entry(unsigned type, const char *name, const unsigned char *extra, int nextra)
{
	long at = syn_len;
	size_t l = strlen(name) + 1;

	put8(type);
	put8((unsigned)(2 + l + nextra));
	memcpy(syn + syn_len, name, l);
	syn_len += (long)l;
	memcpy(syn + syn_len, extra, (size_t)nextra);
	syn_len += nextra;
	return at;
}

static void build_synthetic(void)
{
	static const unsigned char line_0123[2] = { 0x01, 0x23 };	/* label line 291 */
	static const unsigned char line_0201[2] = { 0x02, 0x01 };	/* label line 513 */
	long m, n0;

	syn_len = 0;
	memcpy(syn, "HREF", 4);
	syn_len = 4;
	/* module 1 */
	m = syn_len; put32(0); put32(7); n0 = syn_len;
	entry(REF_FILENAME, "alpha", NULL, 0);
	entry(REF_DATABASE, "Alpha DB", NULL, 0);
	entry(REF_NODENAME, "Main", NULL, 0);
	entry(REF_NODENAME, "Intro", NULL, 0);
	entry(REF_ALIASNAME, "Start", NULL, 0);		/* next entry type 3 */
	entry(REF_LABELNAME, "lab1", line_0123, 2);
	entry(REF_NODENAME, "Shared", NULL, 0);
	syn[m + 0] = 0; syn[m + 1] = 0;
	syn[m + 2] = (unsigned char)((syn_len - n0) >> 8);
	syn[m + 3] = (unsigned char)(syn_len - n0);
	/* module 2 */
	m = syn_len; put32(0); put32(3); n0 = syn_len;
	entry(REF_FILENAME, "beta.hyp", NULL, 0);
	entry(REF_NODENAME, "Shared", NULL, 0);
	entry(REF_LABELNAME, "lab2", line_0201, 2);
	syn[m + 2] = (unsigned char)((syn_len - n0) >> 8);
	syn[m + 3] = (unsigned char)(syn_len - n0);
	/* end */
	put32(0); put32(0);
}

/* ---- real files ---------------------------------------------------------- */

static unsigned char *slurp(const char *path, long *len)
{
	FILE *f = fopen(path, "rb");
	unsigned char *b;

	if (!f)
		return NULL;
	fseek(f, 0, SEEK_END);
	*len = ftell(f);
	fseek(f, 0, SEEK_SET);
	b = malloc((size_t)*len);
	if (b && fread(b, 1, (size_t)*len, f) != (size_t)*len) {
		free(b);
		b = NULL;
	}
	fclose(f);
	return b;
}

int main(int argc, char **argv)
{
	REF_FILE *ref;
	const char *root = argc > 1 ? argv[1] : ".";
	char path[1024];
	unsigned char *real;
	long len;
	union { uint32_t u; unsigned char c[4]; } bo = { 0x01020304u };

	if (sizeof(long) != 8 || bo.c[0] != 0x04) {
		printf("ref-host: host is not LP64 little-endian -- refusing\n");
		return 77;
	}

	/* synthetic */
	build_synthetic();
	ref = load(syn, syn_len);
	check(ref != NIL && ref != NULL, "synthetic: ref_load accepts 'HREF'");
	if (ref == NIL)
		printf("      Debug: %s\n", last_debug);
	check_node(ref, "Main", 1, 0, 0);
	check_node(ref, "Intro", 1, 1, 0);
	check_node(ref, "Start", 1, 1, REF_LABELNAME);	/* alias quirk: next type byte */
	check_node(ref, "Shared", 1, 2, 0);
	check_node(ref, "nothere", 0, 0, 0);
	{
		static const struct hit shared[] = {
			{ "alpha.hyp", "Shared", "Alpha DB", 0, 0 },
			{ "beta.hyp", "Shared", "Alpha DB", 0, 0 },
		};
		static const struct hit lab1[] = { { "alpha.hyp", "Intro", "Alpha DB", 0x0123, 1 } };
		static const struct hit lab2[] = { { "beta.hyp", "Shared", "Alpha DB", 0x0201, 1 } };
		static const struct hit start[] = { { "alpha.hyp", "Intro", "Alpha DB", 0, 0 } };

		check_all(ref, "Shared", shared, 2);
		check_all(ref, "lab1", lab1, 1);
		check_all(ref, "lab2", lab2, 1);
		check_all(ref, "Start", start, 1);
		check_all(ref, "nothere", NULL, 0);
	}

	/* a file without the magic is refused */
	{
		unsigned char bad[16] = { 'H', 'R', 'E', 'G', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0 };

		ref = load(bad, sizeof bad);
		check(ref == NIL, "a non-'HREF' file is refused (NIL)");
	}

	/* real files of the pinned FreeMiNT clone */
	snprintf(path, sizeof path, "%s/references/freemint/doc/programmer/mint-prg.ref", root);
	real = slurp(path, &len);
	if (!real)
		printf("skip  %s absent\n", path);
	else {
		static const struct hit preface[] = {
			{ "mint-prg.hyp", "Preface", "FreeMiNT Programmer's Guide", 0, 0 } };

		ref = load(real, len);
		check(ref != NIL && ref != NULL, "mint-prg.ref: ref_load accepts it");
		if (ref == NIL)
			printf("      Debug: %s\n", last_debug);
		check_node(ref, "Main", 1, 0, 0);
		check_node(ref, "Preface", 1, 4, 0);
		check_all(ref, "Preface", preface, 1);
	}
	snprintf(path, sizeof path, "%s/references/freemint/doc/german/mint-de.ref", root);
	real = slurp(path, &len);
	if (!real)
		printf("skip  %s absent\n", path);
	else {
		ref = load(real, len);
		check(ref != NIL && ref != NULL, "mint-de.ref: ref_load accepts it");
		/* Aliases (type 2): the WORD at the NUL is the next entry's type
		 * byte -- 0x01 behind "environment variables", 0x02 behind
		 * "VFAT=" -- read from the file's bytes, with the node count. */
		check_node(ref, "environment variables", 1, 11, 1);
		check_node(ref, "VFAT=", 1, 13, 2);
	}

	printf("ref-host: %d checks, %d failed\n", checks, fails);
	return fails;
}
