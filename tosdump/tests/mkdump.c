/*
 * mkdump.c -- synthetic dump generator for the host-side tosdiff tests.
 *
 * T1 verifies the comparator against dumps the tests generate themselves, not
 * against any target program (spec section 9, T1).  Every array is poisoned
 * first, then a deterministic pattern is laid over it, so that a single
 * overridden element is the only difference between two otherwise equal files.
 *
 * usage: mkdump OUT.BIN CASES.TSV [FIELD CASEINDEX ELEMINDEX VALUE]
 *        mkdump OUT.BIN CASES.TSV --crc HEX          (forge the case-table CRC)
 *        mkdump OUT.BIN CASES.TSV --area N           (forge the area code)
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "casefile.h"

int main(int argc, char **argv)
{
	struct td_casefile cf;
	struct td_header h;
	struct td_vdi_arrays v;
	int16_t before[TD_VDI_CONTRL_N];
	uint8_t hdr[TD_HEADER_SIZE];
	uint8_t rec[TD_VDI_RECORD_SIZE];
	const char *why = NULL;
	const char *field = NULL;
	long case_index = -1, elem_index = -1, value = 0;
	uint16_t area = TD_AREA_VDI;
	uint32_t crc_override = 0;
	int have_crc_override = 0;
	FILE *f;
	int i, c;

	if (argc < 3) {
		fprintf(stderr, "usage: mkdump OUT.BIN CASES.TSV "
		                "[FIELD CASEINDEX ELEMINDEX VALUE]\n");
		return 2;
	}
	if (argc == 5 && strcmp(argv[3], "--crc") == 0) {
		crc_override = (uint32_t)strtoul(argv[4], NULL, 16);
		have_crc_override = 1;
	} else if (argc == 5 && strcmp(argv[3], "--area") == 0) {
		area = (uint16_t)strtoul(argv[4], NULL, 0);
	} else if (argc == 7) {
		field      = argv[3];
		case_index = strtol(argv[4], NULL, 0);
		elem_index = strtol(argv[5], NULL, 0);
		value      = strtol(argv[6], NULL, 0);
	} else if (argc != 3) {
		fprintf(stderr, "mkdump: bad argument count\n");
		return 2;
	}

	/* mkdump only builds VDI records, so the table it is handed is a VDI
	 * table -- the --area switch forges the HEADER's area code, which is
	 * what the comparator's refusal tests need, not the table's form. */
	if (td_casefile_read(&cf, argv[2], TD_AREA_VDI, &why) != 0) {
		fprintf(stderr, "mkdump: %s: %s\n", argv[2], why ? why : "?");
		return 1;
	}

	memset(&h, 0, sizeof h);
	h.version      = TD_FORMAT_VERSION;
	h.area         = area;
	h.record_size  = td_area_record_size(area);
	h.record_count = cf.count;
	memcpy(h.build_id, "test", 4);
	h.case_crc     = have_crc_override ? crc_override : cf.crc;
	h.ptr_width    = td_native_ptr_width();
	h.word_order   = td_native_word_order();
	td_header_write(hdr, &h);

	f = fopen(argv[1], "wb");
	if (!f) {
		perror(argv[1]);
		return 1;
	}
	if (fwrite(hdr, 1, sizeof hdr, f) != sizeof hdr) {
		perror("fwrite");
		fclose(f);
		return 1;
	}

	for (c = 0; c < (int)cf.count; c++) {
		td_poison_vdi(&v);
		for (i = 0; i < TD_VDI_CONTRL_N; i++)
			before[i] = (int16_t)(c * 16 + i);

		/* a deterministic "result" so the two files agree by default */
		v.contrl[0] = (int16_t)(c + 1);
		v.contrl[1] = 2;
		v.intin[0]  = (int16_t)(0x0100 + c);
		v.ptsin[0]  = (int16_t)(0x0200 + c);
		v.intout[0] = (int16_t)(0x0300 + c);
		v.intout[3] = 0x0555;
		v.ptsout[0] = (int16_t)(0x0400 + c);

		if (field && case_index == c) {
			int16_t val16 = (int16_t)value;
			if (strcmp(field, "contrl") == 0)
				v.contrl[elem_index] = val16;
			else if (strcmp(field, "intin") == 0)
				v.intin[elem_index] = val16;
			else if (strcmp(field, "ptsin") == 0)
				v.ptsin[elem_index] = val16;
			else if (strcmp(field, "intout") == 0)
				v.intout[elem_index] = val16;
			else if (strcmp(field, "ptsout") == 0)
				v.ptsout[elem_index] = val16;
			else if (strcmp(field, "contrl_before") == 0)
				before[elem_index] = val16;
			else {
				fprintf(stderr, "mkdump: unknown field '%s'\n", field);
				fclose(f);
				return 2;
			}
		}

		td_vdi_record_write(rec, cf.line[c], (uint16_t)v.contrl[0], 1,
		                    &v, before);
		if (fwrite(rec, 1, sizeof rec, f) != sizeof rec) {
			perror("fwrite");
			fclose(f);
			return 1;
		}
	}

	fclose(f);
	return 0;
}
