/*
 * layoutcheck.c -- binds the layouts of the case tables to docs/api/
 * (TASK/TOOL16; docs/TOS-REFERENCE-DUMPER.md section 6, "Structures behind
 * a pointer").
 *
 * The layout of a structure is the spec's, not the dumper's.  A structure
 * spec in docs/api/ states it once, machine-readably, in a fenced block
 *
 *   ```tosdump-layout
 *   !layout <TAB> <name> <TAB> <m68k>/<LP64> <TAB> <fields>
 *   ```
 *
 * next to its prose tables.  A case table carries a COPY of that line (the
 * table is the one file carried to both machines; its CRC is the dump's
 * identity), and this program refuses the copy unless it is the spec's
 * line byte for byte.  Every spec line is parsed with the core's own
 * td_layout_parse, so the spec's numbers are also checked against C: each
 * offset where C puts the field on the m68k and on LP64, each size the
 * end rounded to the structure's alignment.
 *
 * usage: layoutcheck SPEC.md... -- TABLE.tsv...
 * exit:  0 every table layout is a spec layout and every spec block
 *        parses; 1 otherwise; 2 usage or I/O.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "cases.h"

#define MAX_SPEC  64
#define LINE_MAX_LEN 1024

struct spec_line {
	char name[TD_LAYOUT_NAME_LEN];
	char text[LINE_MAX_LEN];
	const char *file;
	int used;
};

static struct spec_line spec[MAX_SPEC];
static int nspec;

static char *slurp(const char *path, size_t *len)
{
	FILE *f = fopen(path, "rb");
	long size;
	char *buf;

	if (!f)
		return NULL;
	if (fseek(f, 0, SEEK_END) != 0 || (size = ftell(f)) < 0 ||
	    fseek(f, 0, SEEK_SET) != 0) {
		fclose(f);
		return NULL;
	}
	buf = (char *)malloc((size_t)size + 1);
	if (!buf) {
		fclose(f);
		return NULL;
	}
	*len = fread(buf, 1, (size_t)size, f);
	fclose(f);
	buf[*len] = '\0';
	return buf;
}

/* Next line of buf from *pos, without its line end; NULL at the end. */
static const char *next_line(const char *buf, size_t len, size_t *pos,
                             size_t *llen)
{
	size_t s = *pos, e;

	if (s >= len)
		return NULL;
	for (e = s; e < len && buf[e] != '\n'; e++)
		;
	*pos = e < len ? e + 1 : e;
	if (e > s && buf[e - 1] == '\r')
		e--;
	*llen = e - s;
	return buf + s;
}

/* The second tab-separated column of an "!layout" line. */
static int layout_name(const char *line, size_t len, char *out)
{
	size_t i = 0, s;

	while (i < len && line[i] != '\t')
		i++;
	if (i >= len)
		return -1;
	s = ++i;
	while (i < len && line[i] != '\t')
		i++;
	if (i - s == 0 || i - s >= TD_LAYOUT_NAME_LEN)
		return -1;
	memcpy(out, line + s, i - s);
	out[i - s] = '\0';
	return 0;
}

static int is_layout_line(const char *line, size_t len)
{
	return len > 8 && memcmp(line, "!layout\t", 8) == 0;
}

static int read_spec(const char *path)
{
	static struct td_layout set[TD_LAYOUT_MAX];
	uint8_t n = 0;
	size_t len, pos = 0, llen;
	const char *line;
	char *buf = slurp(path, &len);
	int in_block = 0, fails = 0, lineno = 0;

	if (!buf) {
		fprintf(stderr, "layoutcheck: %s: cannot read\n", path);
		return -1;
	}
	while ((line = next_line(buf, len, &pos, &llen)) != NULL) {
		lineno++;
		if (!in_block) {
			if (llen == 17 && memcmp(line, "```tosdump-layout", 17) == 0)
				in_block = 1;
			continue;
		}
		if (llen >= 3 && memcmp(line, "```", 3) == 0) {
			in_block = 0;
			continue;
		}
		if (llen == 0)
			continue;
		if (!is_layout_line(line, llen)) {
			printf("  FAIL %s:%d: a tosdump-layout block holds "
			       "\"!layout\" lines only\n", path, lineno);
			fails++;
			continue;
		}
		{
			const char *why = NULL;
			int rc = td_layout_parse(set, &n, line, llen, &why);
			struct spec_line *sl;
			int k;

			if (rc != 0) {
				printf("  FAIL %s:%d: %s\n", path, lineno,
				       why ? why : "refused");
				fails++;
				continue;
			}
			if (nspec >= MAX_SPEC || llen >= LINE_MAX_LEN) {
				printf("  FAIL %s:%d: too many or too long layout "
				       "lines\n", path, lineno);
				fails++;
				continue;
			}
			sl = &spec[nspec];
			memset(sl, 0, sizeof *sl);
			if (layout_name(line, llen, sl->name) != 0) {
				fails++;
				continue;
			}
			for (k = 0; k < nspec; k++)
				if (strcmp(spec[k].name, sl->name) == 0) {
					printf("  FAIL %s:%d: layout %s is also defined in "
					       "%s\n", path, lineno, sl->name, spec[k].file);
					fails++;
				}
			memcpy(sl->text, line, llen);
			sl->text[llen] = '\0';
			sl->file = path;
			nspec++;
		}
	}
	if (in_block) {
		printf("  FAIL %s: a tosdump-layout block is not closed\n", path);
		fails++;
	}
	free(buf);
	return fails;
}

static int check_table(const char *path, int *bound)
{
	size_t len, pos = 0, llen;
	const char *line;
	char *buf = slurp(path, &len);
	int fails = 0, lineno = 0;

	if (!buf) {
		fprintf(stderr, "layoutcheck: %s: cannot read\n", path);
		return -1;
	}
	while ((line = next_line(buf, len, &pos, &llen)) != NULL) {
		char name[TD_LAYOUT_NAME_LEN];
		int k, found = -1;

		lineno++;
		if (!is_layout_line(line, llen))
			continue;
		if (layout_name(line, llen, name) != 0) {
			printf("  FAIL %s:%d: an !layout line without a name\n", path,
			       lineno);
			fails++;
			continue;
		}
		for (k = 0; k < nspec; k++)
			if (strcmp(spec[k].name, name) == 0)
				found = k;
		if (found < 0) {
			printf("  FAIL %s:%d: layout %s is in no docs/api/ spec\n",
			       path, lineno, name);
			fails++;
			continue;
		}
		if (strlen(spec[found].text) != llen ||
		    memcmp(spec[found].text, line, llen) != 0) {
			printf("  FAIL %s:%d: layout %s is not the spec's line "
			       "(%s)\n", path, lineno, name, spec[found].file);
			fails++;
			continue;
		}
		spec[found].used = 1;
		(*bound)++;
	}
	free(buf);
	return fails;
}

int main(int argc, char **argv)
{
	int i, sep = -1, fails = 0, bound = 0, tables = 0, rc;

	for (i = 1; i < argc; i++)
		if (strcmp(argv[i], "--") == 0)
			sep = i;
	if (sep < 2) {
		fprintf(stderr, "usage: layoutcheck SPEC.md... -- TABLE.tsv...\n");
		return 2;
	}
	for (i = 1; i < sep; i++) {
		rc = read_spec(argv[i]);
		if (rc < 0)
			return 2;
		fails += rc;
	}
	if (nspec == 0) {
		printf("  FAIL no tosdump-layout block in the specs\n");
		return 1;
	}
	for (i = sep + 1; i < argc; i++) {
		rc = check_table(argv[i], &bound);
		if (rc < 0)
			return 2;
		fails += rc;
		tables++;
	}
	if (fails) {
		printf("  %d failure(s)\n", fails);
		return 1;
	}
	printf("  ok (%d layouts in the specs, %d table lines in %d tables are "
	       "the specs' lines)\n", nspec, bound, tables);
	return 0;
}
