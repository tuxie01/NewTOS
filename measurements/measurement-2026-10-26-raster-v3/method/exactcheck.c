/*
 * exactcheck.c -- the RASTER replacement condition (user decision
 * 2026-10-26): the payload of the format 2 reference and of the format 3
 * recording is byte-identical except
 *   (a) the 32-byte header, and
 *   (b) the cells of field class address.
 *
 * The exclusion set is NOT listed by hand: it is derived from the field
 * descriptors the comparator itself uses (td_area_fields(), record.c) --
 * every field with compare == TD_FIELD_ADDRESS, and inside it only its
 * address CELLS (td_addr_cell_count / td_addr_cell_words, record.h).  The
 * cell geometry needs the pointer width and word order, which only the
 * format 3 header states; it is taken from the NEW (format 3) header and
 * applied to both files, which come from the same m68k dumper source family
 * (the format 2 LOG says "pointer 4 bytes, MFDB pointer slots at contrl[7]
 * and [9]" as well).  WORDs of an address field that are not a cell
 * (contrl[11..14] for four-byte pointers) stay COMPARED.
 *
 * usage: exactcheck OLD-v2.BIN NEW-v3.BIN
 * build: cc -std=c99 -Wall -Wextra -I<repo>/tosdump/core exactcheck.c
 *           <repo>/tosdump/core/record.c -o exactcheck
 * exit:  0 = condition holds, 1 = a non-excluded byte differs,
 *        2 = files not comparable.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "record.h"

static uint8_t *load(const char *path, size_t *len)
{
	FILE *f = fopen(path, "rb");
	uint8_t *buf;
	long n;

	if (!f || fseek(f, 0, SEEK_END) != 0 || (n = ftell(f)) < 0 ||
	    fseek(f, 0, SEEK_SET) != 0) {
		fprintf(stderr, "exactcheck: %s: cannot read\n", path);
		exit(2);
	}
	buf = malloc((size_t)n + 1);
	if (!buf || fread(buf, 1, (size_t)n, f) != (size_t)n) {
		fprintf(stderr, "exactcheck: %s: short read\n", path);
		exit(2);
	}
	fclose(f);
	*len = (size_t)n;
	return buf;
}

int main(int argc, char **argv)
{
	uint8_t *a, *b, *mask;
	size_t la, lb, i;
	struct td_header ha, hb;
	const struct td_field *fields, *f;
	unsigned long cells_per_rec = 0, bytes_per_rec = 0;
	unsigned long excl_diff = 0, other_diff = 0;
	uint16_t rs, rec;
	int hdr_diff = 0;

	if (argc != 3) {
		fprintf(stderr, "usage: exactcheck OLD-v2.BIN NEW-v3.BIN\n");
		return 2;
	}
	a = load(argv[1], &la);
	b = load(argv[2], &lb);
	if (td_header_parse(a, la, &ha) != 0 || td_header_parse(b, lb, &hb) != 0) {
		fprintf(stderr, "exactcheck: malformed header\n");
		return 2;
	}
	if (ha.version != 2 || hb.version != 3 || !td_header_states_ptr(&hb) ||
	    ha.area != hb.area || ha.record_size != hb.record_size ||
	    ha.record_count != hb.record_count || ha.case_crc != hb.case_crc ||
	    la != lb || la != TD_HEADER_SIZE +
	                      (size_t)ha.record_count * ha.record_size) {
		fprintf(stderr, "exactcheck: not a format 2 / format 3 pair of the "
		        "same table\n");
		return 2;
	}
	fields = td_area_fields(hb.area);
	if (!fields) {
		fprintf(stderr, "exactcheck: no field table for area %u\n", hb.area);
		return 2;
	}
	rs = hb.record_size;

	printf("old %s: format %u, area %s, %u records of %u bytes, build %s, "
	       "case CRC %08X\n", argv[1], ha.version, td_area_name(ha.area),
	       ha.record_count, rs, ha.build_id, ha.case_crc);
	printf("new %s: format %u, pointer %u bytes, word order %u, build %s\n",
	       argv[2], hb.version, hb.ptr_width, hb.word_order, hb.build_id);

	/* (a) the header: reported, not part of the condition */
	for (i = 0; i < TD_HEADER_SIZE; i++)
		if (a[i] != b[i]) {
			printf("header byte %2lu: %02X -> %02X (excluded)\n",
			       (unsigned long)i, a[i], b[i]);
			hdr_diff++;
		}

	/* (b) the per-record mask, from the descriptors */
	mask = calloc(rs, 1);
	for (f = fields; f->name; f++) {
		uint16_t n, cells = td_addr_cell_count(f, &hb);
		uint16_t words = td_addr_cell_words(f, &hb);

		if (f->compare != TD_FIELD_ADDRESS)
			continue;
		for (n = 0; n < cells; n++) {
			unsigned off = f->off + (unsigned)n * words * 2u, k;
			unsigned w0 = f->first + (unsigned)n * words;

			printf("excluded cell: %s[%u..%u], record bytes %u..%u\n",
			       f->name, w0, w0 + words - 1u, off, off + words * 2u - 1u);
			for (k = 0; k < words * 2u; k++)
				mask[off + k] = 1;
			cells_per_rec++;
			bytes_per_rec += words * 2u;
		}
	}
	printf("excluded per record: %lu cells, %lu bytes; total: %lu cells, "
	       "%lu bytes of %lu payload bytes\n", cells_per_rec, bytes_per_rec,
	       cells_per_rec * hb.record_count, bytes_per_rec * hb.record_count,
	       (unsigned long)(la - TD_HEADER_SIZE));

	for (rec = 0; rec < hb.record_count; rec++) {
		const uint8_t *ra = a + TD_HEADER_SIZE + (size_t)rec * rs;
		const uint8_t *rb = b + TD_HEADER_SIZE + (size_t)rec * rs;
		unsigned o;

		for (o = 0; o < rs; o++) {
			if (ra[o] == rb[o])
				continue;
			if (mask[o]) {
				excl_diff++;
				continue;
			}
			other_diff++;
			printf("DIFF record %u (case %u) byte %u: %02X -> %02X\n",
			       rec, td_load_be16(ra), o, ra[o], rb[o]);
		}
	}
	printf("header bytes differing: %d (excluded)\n", hdr_diff);
	printf("payload bytes differing inside excluded cells: %lu\n", excl_diff);
	printf("payload bytes differing outside excluded cells: %lu\n", other_diff);
	printf("RESULT: %s\n", other_diff ? "FAIL -- a non-address byte differs"
	                                  : "PASS -- every other byte is equal");
	free(mask);
	free(a);
	free(b);
	return other_diff ? 1 : 0;
}
