/*
 * dumpbios.c -- DUMPBIOS, the BIOS + XBIOS reference dumper (spec section 7,
 * task T7).
 *
 * Portable.  The same source builds for TOS on the TT (or the oracle) and for
 * NewTOS on the Pi 4; only the ABI backend underneath differs.
 *
 * usage: DUMPBIOS [CASES.TSV [OUT.BIN [OUT.LOG]]]
 *        defaults: BIOS.TSV in the current directory, BIOS.BIN and BIOS.LOG
 *        beside it.  A table that names the unit token "u:0" also reads
 *        DISKUNIT.TXT from the current directory: the physical unit of the
 *        test disk, which the run's profile names (docs/TOS-REFERENCE-
 *        DUMPER.md section 6, "The closing tables").  A table that names a
 *        program "x:<path>" reads PROGEXT.TXT the same way: the extension
 *        of the machine's programs, PRG or NAP (section 6, "GEMDOS after a
 *        media change"; TASK/TOOL20).
 *
 * One table, one dump: trap #13 and trap #14 cases live in the same table
 * and the same BIOS.BIN (header area TD_AREA_BIOS); each record names its
 * trap (docs/TOS-REFERENCE-DUMPER.md section 3).
 *
 * Unlike DUMPVDI and DUMPAES this program is no AES client: nothing it
 * measures needs an application context, and every GEM call it made would
 * be one more thing that could differ between the two machines for reasons
 * that have nothing to do with the BIOS.
 *
 * As in the other dumpers: the case table is read at runtime, records are
 * buffered in RAM and written once at the end (spec section 7), the log is
 * written once before the first case -- so a run that hangs leaves its setup
 * and the line "running N cases" on disk -- and once at the end, and the
 * program never waits for a key.
 */
#include <stdio.h>
#include <string.h>

#include "cases.h"
#include "dump.h"
#include "log.h"
#include "runner_call.h"

#ifndef TD_BUILD_ID
#define TD_BUILD_ID "----"
#endif

static struct td_call_case_table table;
static uint8_t records[(size_t)TD_MAX_CASES * TD_CALL_RECORD_SIZE];

static const char *log_path = "BIOS.LOG";

/* The profile's physical unit of the test disk: DISKUNIT.TXT, one decimal
 * number 0..32767, optionally followed by a line end -- what
 * tools/hatari-oracle.sh (profile disk) and cargo xtask tosdump (--disk)
 * put beside the table.  0 on success. */
static const char *unit_path = "DISKUNIT.TXT";

static int read_disk_unit(int32_t *unit)
{
	char buf[16];
	FILE *f = fopen(unit_path, "rb");
	size_t n, i = 0;
	int32_t v = 0;
	int digits = 0;

	if (!f)
		return -1;
	n = fread(buf, 1, sizeof buf, f);
	fclose(f);
	if (n == sizeof buf)
		return -2;
	for (; i < n && buf[i] >= '0' && buf[i] <= '9'; i++) {
		if (++digits > 5)
			return -2;
		v = v * 10 + (buf[i] - '0');
	}
	if (i < n && buf[i] == '\r')
		i++;
	if (i < n && buf[i] == '\n')
		i++;
	if (digits == 0 || i != n || v > 0x7FFF)
		return -2;
	*unit = v;
	return 0;
}

/* The profile's program extension: PROGEXT.TXT, 1..3 characters A-Z or
 * 0-9, optionally followed by a line end -- what tools/hatari-oracle.sh
 * (profile disk, PRG) and cargo xtask tosdump (--disk, NAP) put beside the
 * table (TOOL20).  0 on success. */
static const char *ext_path = "PROGEXT.TXT";
static char prog_ext[8];

static int read_prog_ext(void)
{
	char *buf = prog_ext;
	FILE *f = fopen(ext_path, "rb");
	size_t n, i = 0, end;

	if (!f)
		return -1;
	n = fread(buf, 1, sizeof prog_ext, f);
	fclose(f);
	if (n == sizeof prog_ext)
		return -2;
	while (i < n && buf[i] != '\r' && buf[i] != '\n')
		i++;
	end = i;
	if (end < n && buf[end] == '\r')
		end++;
	if (end < n && buf[end] == '\n')
		end++;
	if (end != n)
		return -2;
	buf[i] = '\0';
	return td_run_call_set_prog_ext(buf) == 0 ? 0 : -2;
}

static int finish(int rc)
{
	int lr = td_log_write(log_path);

	if (lr != 0)
		printf("cannot write %s (%d)\n", log_path, lr);
	else
		printf("%s: %lu bytes%s\n", log_path,
		       (unsigned long)td_log_len(),
		       td_log_truncated() ? " (truncated)" : "");
	return rc;
}

int main(int argc, char **argv)
{
	const char *table_path = (argc > 1) ? argv[1] : "BIOS.TSV";
	const char *out_path   = (argc > 2) ? argv[2] : "BIOS.BIN";
	struct td_case_error err;
	size_t bytes;
	int rc;

	if (argc > 3)
		log_path = argv[3];

	td_log_init();
	td_log("DUMPBIOS build %s, record format %d\n", TD_BUILD_ID,
	       TD_FORMAT_VERSION);
	td_log("pointer %u bytes, word order %u\n",
	       (unsigned)td_native_ptr_width(), (unsigned)td_native_word_order());

	memset(&err, 0, sizeof err);
	rc = td_call_cases_load(&table, TD_AREA_BIOS, table_path, &err);
	if (rc != 0) {
		if (err.line)
			td_log("%s line %u: %s (%s)\n", table_path, err.line,
			       err.what ? err.what : "", td_case_error_text(rc));
		else
			td_log("%s: %s (%s)\n", table_path, err.what ? err.what : "",
			       td_case_error_text(rc));
		return finish(1);
	}
	td_log("%s: %u cases, CRC %08lX\n", table_path, table.count,
	       (unsigned long)table.crc);
	/* a disk table (docs/TOS-REFERENCE-DUMPER.md section 6, "The disk
	 * round") says so; a round-1 table's log stays as it was */
	if (table.disk)
		td_log("disk table: test drive %c:, %u preludes%s\n",
		       'A' + table.disk_drive, table.prelude_count,
		       table.destructive ? ", destructive (a fresh clone of the "
		                           "test image only)" : "");
	/* structures behind a pointer (TOOL16): what the table reads and
	 * whether it writes through returned pointers */
	if (table.nlayouts)
		td_log("layouts: %u%s\n", table.nlayouts,
		       table.writes ? ", writes through returned pointers "
		                      "(+put)" : "");
	if (table.unit_token) {
		int32_t unit;

		if (read_disk_unit(&unit) != 0) {
			td_log("the table names u:0 and the run's profile names no "
			       "unit (%s missing, or not one number 0..32767): not "
			       "run, no %s written\n", unit_path, out_path);
			return finish(1);
		}
		td_run_call_set_disk_unit(unit);
		td_log("unit token u:0 = physical unit %ld (%s, the run's "
		       "profile)\n", (long)unit, unit_path);
	}
	if (table.prog_ext) {
		if (read_prog_ext() != 0) {
			td_log("the table names a program x:<path> and the run's "
			       "profile names no extension (%s missing, or not 1..3 "
			       "characters A-Z, 0-9): not run, no %s written\n",
			       ext_path, out_path);
			return finish(1);
		}
		td_log("program extension x:<path>.%s (%s, the run's profile)\n",
		       prog_ext, ext_path);
	}
	if (table.count == 0) {
		td_log("nothing to do\n");
		return finish(1);
	}

	/* Written before the first case: a run that hangs still leaves this. */
	td_log("running %u cases\n", table.count);
	td_log_write(log_path);

	bytes = (size_t)table.count * TD_CALL_RECORD_SIZE;
	rc = td_run_call(&table, records, sizeof records);
	if (rc == TD_RUN_CALL_ERR_PRELUDE) {
		/* the buffer holds poison, not the sectors: nothing after it
		 * was issued, and a dump of part of a table is no dump */
		td_log("prelude at line %u returned %ld, not 0: run stopped "
		       "before its case, no %s written\n",
		       td_run_call_stop_line(), td_run_call_stop_ret(), out_path);
		return finish(1);
	}
	if (rc == TD_RUN_CALL_ERR_PROGEXT) {
		td_log("x: without a program extension: nothing issued, no %s "
		       "written\n", out_path);
		return finish(1);
	}
	if (rc == TD_RUN_CALL_ERR_UNIT) {
		td_log("u:0 without a unit: nothing issued, no %s written\n",
		       out_path);
		return finish(1);
	}
	if (rc != 0) {
		td_log("runner failed (%d)\n", rc);
		return finish(1);
	}
	td_log("all %u cases ran\n", table.count);
	/* what each +put did -- logged after the run, so no console output
	 * falls between two measurements */
	{
		uint16_t k;

		for (k = 0; k < table.prelude_count; k++) {
			const struct td_call_case *p = &table.preludes[k];
			uint8_t r;

			if (p->trap != TD_CALL_TRAP_PUT)
				continue;
			r = td_run_call_put_result(k);
			if (r == TD_PUT_WRITTEN)
				td_log("+put line %u: written\n", p->line);
			else if (r == 0)
				td_log("+put line %u: not reached\n", p->line);
			else
				td_log("+put line %u: not written, %s\n", p->line,
				       td_layout_noread_text(r));
		}
	}

	rc = td_dump_write(out_path, TD_AREA_BIOS, table.count, TD_BUILD_ID,
	                   table.crc, records, bytes);
	if (rc != 0) {
		td_log("cannot write %s (%d)\n", out_path, rc);
		return finish(1);
	}
	td_log("%s: %u records, %lu bytes\n", out_path, table.count,
	       (unsigned long)(TD_HEADER_SIZE + bytes));
	return finish(0);
}
