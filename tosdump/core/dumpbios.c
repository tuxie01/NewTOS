/*
 * dumpbios.c -- DUMPBIOS, the BIOS + XBIOS reference dumper (spec section 7,
 * task T7).
 *
 * Portable.  The same source builds for TOS on the TT (or the oracle) and for
 * NewTOS on the Pi 4; only the ABI backend underneath differs.
 *
 * usage: DUMPBIOS [CASES.TSV [OUT.BIN [OUT.LOG]]]
 *        defaults: BIOS.TSV in the current directory, BIOS.BIN and BIOS.LOG
 *        beside it.
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
	if (table.count == 0) {
		td_log("nothing to do\n");
		return finish(1);
	}

	/* Written before the first case: a run that hangs still leaves this. */
	td_log("running %u cases\n", table.count);
	td_log_write(log_path);

	bytes = (size_t)table.count * TD_CALL_RECORD_SIZE;
	rc = td_run_call(&table, records, sizeof records);
	if (rc != 0) {
		td_log("runner failed (%d)\n", rc);
		return finish(1);
	}
	td_log("all %u cases ran\n", table.count);

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
