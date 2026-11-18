/*
 * casecheck.c -- parse a case table and print what the runner will do with it.
 *
 * A case table is data that two machines must agree on byte for byte, so it is
 * worth being able to see exactly how it parses -- the case ids, the contrl
 * values each case produces, and the CRC the dump header will carry -- before
 * carrying it to a machine that has no debugger.
 *
 * usage: casecheck [--aes [--aes-version XXXX] | --raster] CASES.TSV
 *
 * --aes reads the AES form -- seven columns, or eight where the table's own
 * "!columns" directive says so -- and --raster the seven column VDI raster
 * form, instead of the five column VDI one.  The AREA cannot be told from a
 * line (a VDI comment may hold tabs), so which one it is has to be said; the
 * AES column count, in contrast, IS in the file, and casecheck prints what
 * it read so a table can be checked before it is carried to a machine with
 * no debugger.
 *
 * --aes-version ASKS A QUESTION THE HOST CANNOT ANSWER BY ITSELF.  Five AES
 * opcodes exist only on a machine whose ap_version establishes them
 * (core/cases.h, the version guard), and casecheck has no machine: it runs
 * on a Mac and the table may be carried to either of two different AESs.  So
 * it does not refuse such a table -- that would lock out functions both
 * target machines have, which is the defect TASK/M06.md is about -- it
 * PRINTS what the table needs, and enforces it only against a version the
 * caller states, for "would this table run on a 3.20 machine?".  The binding
 * answer is the machine's own, taken by the dumper against the ap_version
 * its appl_init wrote.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "cases.h"

static struct td_case_table      table;
static struct td_aes_case_table  aes_table;
static struct td_rast_case_table rast_table;

static void print_list(const char *name, const int16_t *v, uint16_t n)
{
	uint16_t i;

	if (n == 0)
		return;
	printf("      %s[%u] =", name, n);
	for (i = 0; i < n && i < 12; i++)
		printf(" %d", v[i]);
	if (n > 12)
		printf(" ... (%u more)", n - 12);
	printf("\n");
}

static void print_error(const char *path, int rc,
                        const struct td_case_error *err)
{
	if (err->line)
		fprintf(stderr, "%s line %u: %s (%s)\n", path, err->line,
		        err->what ? err->what : "", td_case_error_text(rc));
	else
		fprintf(stderr, "%s: %s\n", path, td_case_error_text(rc));
}

static int check_vdi(const char *path)
{
	struct td_case_error err;
	uint16_t i;
	int rc;

	memset(&err, 0, sizeof err);
	rc = td_cases_load(&table, path, &err);
	if (rc != 0) {
		print_error(path, rc, &err);
		return 1;
	}

	printf("%s: %u cases, %u preludes, CRC %08X\n", path, table.count,
	       table.prelude_count, table.crc);
	for (i = 0; i < table.count; i++) {
		const struct td_case *c = &table.cases[i];
		uint16_t k;

		/* Preludes run after the reset and before the case, unrecorded --
		 * print them with the case they belong to, since that is the only
		 * way to see what state the case is actually measuring. */
		for (k = 0; k < c->prelude_count; k++) {
			const struct td_prelude *p =
				&table.preludes[c->prelude_first + k];
			printf("    prelude: opcode %d  contrl[1]=%u contrl[3]=%u "
			       "contrl[5]=%d\n",
			       p->opcode, p->n_ptsin / 2, p->n_intin, p->sub);
			print_list("intin", p->intin, p->n_intin);
			print_list("ptsin", p->ptsin, p->n_ptsin);
		}

		printf("  case %u  opcode %d  contrl[1]=%u contrl[3]=%u contrl[5]=%d"
		       "  %s\n",
		       c->line, c->opcode, c->n_ptsin / 2, c->n_intin, c->sub,
		       c->comment);
		print_list("intin", c->intin, c->n_intin);
		print_list("ptsin", c->ptsin, c->n_ptsin);
	}
	return 0;
}

/* The addr_in column, as the runner will act on it (cases.h). */
static void print_plants(const struct td_aes_plant *p, uint16_t n)
{
	uint16_t i;

	if (n == 0)
		return;
	printf("      addr_in column, %u plant%s:", n, n == 1 ? "" : "s");
	for (i = 0; i < n; i++)
		printf(" buffer %d -> %s%d%s", p[i].buffer,
		       p[i].kind == TD_AES_PLANT_INTIN ? "a pointer at int_in["
		                                       : "addr_in[",
		       p[i].where, "]");
	printf("\n");
}

/* `ap_version` is a measured AES version to enforce the table against, or
 * -1 for "no machine stated" -- see the usage note at the top. */
static int check_aes(const char *path, long ap_version)
{
	struct td_case_error err;
	uint16_t i;
	int rc;

	memset(&err, 0, sizeof err);
	rc = td_aes_cases_load(&aes_table, path, &err);
	if (rc != 0) {
		print_error(path, rc, &err);
		return 1;
	}

	printf("%s: %u cases, %u preludes, %d columns, CRC %08X\n", path,
	       aes_table.count, aes_table.prelude_count, aes_table.columns,
	       aes_table.crc);
	if (aes_table.features != 0)
		printf("  needs of the machine: %s\n",
		       td_aes_features_text(aes_table.features));
	if (ap_version >= 0) {
		rc = td_aes_cases_version_refusal(&aes_table,
		                                  (int16_t)ap_version, &err);
		printf("  against ap_version %04lX: %s\n",
		       (unsigned long)ap_version, rc == 0 ? "issuable" : "REFUSED");
		if (rc != 0) {
			print_error(path, rc, &err);
			return 1;
		}
	}
	for (i = 0; i < aes_table.count; i++) {
		const struct td_aes_case *c = &aes_table.cases[i];
		uint16_t k;

		for (k = 0; k < c->prelude_count; k++) {
			const struct td_aes_prelude *p =
				&aes_table.preludes[c->prelude_first + k];
			printf("    prelude: opcode %d  control[1..4]= %d %d %d %d\n",
			       p->opcode, p->n_intin, p->n_intout, p->n_addrin,
			       p->n_addrout);
			print_list("int_in", p->intin, p->n_values);
			print_plants(p->plant, p->n_plants);
		}

		printf("  case %u  opcode %d  control[1..4]= %d %d %d %d  %s\n",
		       c->line, c->opcode, c->n_intin, c->n_intout, c->n_addrin,
		       c->n_addrout, c->comment);
		print_list("int_in", c->intin, c->n_values);
		print_plants(c->plant, c->n_plants);
	}
	return 0;
}

static void print_mfdb(const char *name, const struct td_rast_mfdb *m)
{
	if (m->buffer == TD_RAST_BUF_SCREEN) {
		printf("      %s the screen (fd_addr NULL), declared %dx%d at %d "
		       "planes\n", name, m->w, m->h, m->nplanes);
		return;
	}
	printf("      %s buffer %c  fd_w %d fd_h %d fd_wdwidth %d fd_stand %d "
	       "fd_nplanes %d\n",
	       name, m->buffer == TD_RAST_BUF_B ? 'B' : 'A',
	       m->w, m->h, m->wdwidth, m->stand, m->nplanes);
}

static int check_raster(const char *path)
{
	struct td_case_error err;
	uint16_t i;
	int rc;

	memset(&err, 0, sizeof err);
	rc = td_rast_cases_load(&rast_table, path, &err);
	if (rc != 0) {
		print_error(path, rc, &err);
		return 1;
	}

	printf("%s: %u cases, CRC %08X\n", path, rast_table.count,
	       rast_table.crc);
	if (rast_table.screen.declared)
		printf("  !screen %dx%d at %d planes, %u preludes\n",
		       rast_table.screen.w, rast_table.screen.h,
		       rast_table.screen.planes, rast_table.prelude_count);
	if (rast_table.screen.wide)
		printf("  wide table: area VDI-RASTER32, NewTOS contract "
		       "(ADR-038), no oracle\n");
	for (i = 0; i < rast_table.count; i++) {
		const struct td_rast_case *c = &rast_table.cases[i];
		uint16_t j;

		for (j = 0; j < c->prelude_count; j++) {
			const struct td_rast_prelude *p =
				&rast_table.preludes[c->prelude_first + j];

			printf("  prelude opcode %d\n", p->opcode);
			print_list("intin", p->intin, p->n_intin);
			print_list("ptsin", p->ptsin, p->n_ptsin);
			if (p->opcode == 109 || p->opcode == 121) {
				print_mfdb("src", &p->src);
				print_mfdb("dst", &p->dst);
			}
		}
		printf("  case %u  opcode %d  contrl[1]=%u contrl[3]=%u  %s\n",
		       c->line, c->opcode, c->n_ptsin / 2, c->n_intin, c->comment);
		print_list("intin", c->intin, c->n_intin);
		print_list("ptsin", c->ptsin, c->n_ptsin);
		print_mfdb("src", &c->src);
		print_mfdb("dst", &c->dst);
		printf("      fill  A seed %u, B seed %u\n", c->seed_a, c->seed_b);
		if (c->has_read)
			printf("      readback (%d,%d) %dx%d into buffer B\n",
			       c->rx, c->ry, c->rw, c->rh);
	}
	return 0;
}

int main(int argc, char **argv)
{
	if (argc == 5 && strcmp(argv[1], "--aes") == 0 &&
	    strcmp(argv[2], "--aes-version") == 0) {
		char *end;
		long v = strtol(argv[3], &end, 16);

		if (*end != '\0' || v < 0 || v > 0xFFFFL) {
			fprintf(stderr, "casecheck: --aes-version takes an AES "
			                "version in hex, e.g. 0340\n");
			return 2;
		}
		return check_aes(argv[4], v);
	}
	if (argc == 3 && strcmp(argv[1], "--aes") == 0)
		return check_aes(argv[2], -1);
	if (argc == 3 && strcmp(argv[1], "--raster") == 0)
		return check_raster(argv[2]);
	if (argc == 2)
		return check_vdi(argv[1]);

	fprintf(stderr, "usage: casecheck [--aes [--aes-version XXXX] | "
	                "--raster] CASES.TSV\n");
	return 2;
}
