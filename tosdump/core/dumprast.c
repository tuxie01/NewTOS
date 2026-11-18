/*
 * dumprast.c -- DUMPRAST, the VDI raster reference dumper (spec section 7).
 *
 * Portable.  The same source builds for the TT and for NewTOS; only the ABI
 * backend underneath differs.
 *
 * usage: DUMPRAST [CASES.TSV [OUT.BIN [OUT.LOG]]]
 *        defaults: RASTER.TSV in the current directory, RASTER.BIN and
 *        RASTER.LOG beside it.
 *
 * Why this is a separate program from DUMPVDI rather than more cases in its
 * table: the result of a raster call is MEMORY, not a number in INTOUT, so
 * the record has to carry two MFDBs and two pixel buffers, and the control
 * array has to be fifteen WORDs rather than twelve (an AArch64 caller packs
 * the second MFDB pointer at contrl[11..14]).  That is a different record,
 * and a different record is a different area -- which also means the VDI
 * dumps already taken keep comparing against their own table, unchanged.
 *
 * It does NOT wait for a keypress at the end, for the reason DUMPVDI does
 * not: the blocking console read never returns on a real Pi 4 (QUIRK-002).
 * The log file carries everything it printed, on both machines.
 */
#include <stdio.h>
#include <string.h>

#include "cases.h"
#include "dump.h"
#include "gem.h"
#include "log.h"
#include "record.h"
#include "runner_rast.h"

#ifndef TD_BUILD_ID
#define TD_BUILD_ID "----"
#endif

static struct td_rast_case_table table;
/* Sized for the larger record: a wide table (`!screen w,h,32`) writes the
 * wide record of area TD_AREA_RASTER32 (tosdump.h), every other table the
 * narrow one of area TD_AREA_RASTER into the front of the same space. */
static uint8_t records[(size_t)TD_RAST_MAX_CASES * TD_RAST32_RECORD_SIZE];

static const char *log_path = "RASTER.LOG";

/* Write the log out.  Outside the measurement: every case has already run. */
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
	const char *table_path = (argc > 1) ? argv[1] : "RASTER.TSV";
	const char *out_path   = (argc > 2) ? argv[2] : "RASTER.BIN";
	struct td_case_error err;
	struct td_wk wk;
	size_t bytes;
	int rc, i;

	if (argc > 3)
		log_path = argv[3];

	td_log_init();
	td_log("DUMPRAST build %s, record format %d\n", TD_BUILD_ID,
	       TD_FORMAT_VERSION);

	memset(&err, 0, sizeof err);
	rc = td_rast_cases_load(&table, table_path, &err);
	if (rc != 0) {
		if (err.line)
			td_log("%s line %u: %s (%s)\n", table_path, err.line,
			       err.what ? err.what : "", td_case_error_text(rc));
		else
			td_log("%s: %s\n", table_path, td_case_error_text(rc));
		return finish(1);
	}
	td_log("%s: %u cases, CRC %08lX\n", table_path, table.count,
	       (unsigned long)table.crc);
	if (table.count == 0) {
		td_log("nothing to do\n");
		return finish(1);
	}

	/* The MFDB is passed as a struct, so its layout is the target's, not
	 * the record's.  Logging it costs two lines and settles the one
	 * question a difference in the dump could otherwise be blamed on. */
	td_log("pointer %u bytes, MFDB pointer slots at contrl[7] and [%u]\n",
	       (unsigned)sizeof(void *),
	       (unsigned)(7 + sizeof(void *) / sizeof(int16_t)));

	rc = td_gem_open(&wk);
	if (rc != 0) {
		td_log("%s\n", td_gem_error_text(rc));
		return finish(1);
	}
	td_log("appl_init id %d, graf_handle %d, v_opnvwk handle %d\n",
	       wk.ap_id, wk.phys_handle, wk.handle);
	/* DEV_TAB[13] is the colour count and DEV_TAB[0..1] the screen extent:
	 * a raster result that differs because the two machines run different
	 * depths is explained here rather than guessed at later. */
	td_log("DEV_TAB ");
	for (i = 0; i < TD_WORK_OUT_N; i++)
		td_log("%d%s", wk.work_out[i], i + 1 < TD_WORK_OUT_N ? "," : "\n");
	td_log("SIZ_TAB ");
	for (i = 0; i < TD_WORK_PTS_N; i++)
		td_log("%d%s", wk.work_pts[i], i + 1 < TD_WORK_PTS_N ? "," : "\n");

	/* A table that names the screen declared the screen it is written
	 * for (cases.h, THE SCREEN).  Refuse the whole run -- nothing measured
	 * -- unless this workstation is at least that big and exactly that
	 * deep: a rectangle past the end of video memory would scribble over
	 * the machine, and a readback at another depth would not be the word
	 * the table expects. */
	if (table.screen.declared) {
		int16_t planes = td_rast_planes(&wk);

		td_log("table declares screen %dx%d at %d planes; workstation "
		       "reports %dx%d at %d planes\n", table.screen.w,
		       table.screen.h, table.screen.planes, wk.work_out[0] + 1,
		       wk.work_out[1] + 1, planes);
		if (wk.work_out[0] + 1 < table.screen.w ||
		    wk.work_out[1] + 1 < table.screen.h ||
		    planes != table.screen.planes) {
			td_log("screen mismatch: refusing to run the table\n");
			td_gem_close(&wk);
			return finish(1);
		}
		/* EmuTOS has no path above 16 planes (vdi_defs.h:22): what a
		 * wide table measures is NewTOS' own contract, and its dump is
		 * a baseline of area VDI-RASTER32, never an oracle dump. */
		if (table.screen.wide)
			td_log("table declares a 32-plane screen: NewTOS contract "
			       "(ADR-038), no oracle; area %s, record %lu bytes\n",
			       td_area_name(td_rast_area(&table)),
			       (unsigned long)td_rast_record_size(&table));
	}

	/* Written before the first case: a run that hangs still leaves this. */
	td_log("running %u cases\n", table.count);
	td_log_write(log_path);

	bytes = (size_t)table.count * td_rast_record_size(&table);
	rc = td_run_raster(&table, &wk, records, sizeof records);
	td_gem_close(&wk);
	if (rc != 0) {
		td_log("runner failed (%d)\n", rc);
		return finish(1);
	}
	td_log("all %u cases ran\n", table.count);

	rc = td_dump_write(out_path, td_rast_area(&table), table.count, TD_BUILD_ID,
	                   table.crc, records, bytes);
	if (rc != 0) {
		td_log("cannot write %s (%d)\n", out_path, rc);
		return finish(1);
	}

	td_log("%s: %u records, %lu bytes\n", out_path, table.count,
	       (unsigned long)(TD_HEADER_SIZE + bytes));
	return finish(0);
}
