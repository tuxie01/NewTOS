/*
 * tosdiff.c -- host-side comparator for TOS reference dumps.
 *
 * Spec: docs/TOS-REFERENCE-DUMPER.md sections 3, 6 and 7.
 *
 * All interpretation of a dump happens here, where there are tools and a
 * debugger; the target programs only write bytes.  A difference is reported
 * by field name and element index, never as a hex offset.
 *
 * usage: tosdiff --validate DUMP.BIN [CASES.TSV]
 *        tosdiff CASES.TSV A.BIN B.BIN
 *
 * exit: 0 = accepted / no differences, 1 = differences found (a value
 *       difference, an address cell that is not < 0x80000000, or an MFDB
 *       pointer cell the call changed -- the kept check), 2 = refused.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "casefile.h"

struct td_dump {
	const char       *path;
	const char       *label;
	struct td_header  h;
	uint8_t          *records;   /* record_count * record_size bytes */
	size_t            bytes;
};

static const char *basename_of(const char *path)
{
	const char *slash = strrchr(path, '/');
	return slash ? slash + 1 : path;
}

static const char *header_error(int rc)
{
	switch (rc) {
	case TD_ERR_MAGIC:    return "not a TDMP dump (bad magic)";
	case TD_ERR_VERSION:  return "unsupported format version (2 and 3 are read)";
	case TD_ERR_PTRINFO:  return "format 3 header states no valid pointer width "
	                             "(4 or 8) or word order (1 or 2)";
	case TD_ERR_RESERVED: return "reserved header bytes are not zero";
	case TD_ERR_RECSIZE:  return "record size does not match the area";
	case TD_ERR_AREA:     return "unknown area code";
	case TD_ERR_SIZE:     return "file is shorter than the header";
	default:              return "malformed header";
	}
}

static const char *word_order_name(uint16_t order)
{
	switch (order) {
	case TD_WORD_ORDER_MSW_FIRST: return "most significant word first";
	case TD_WORD_ORDER_LSW_FIRST: return "least significant word first";
	default:                      return "word order not stated";
	}
}

static int dump_load(struct td_dump *d, const char *path)
{
	FILE *f;
	long size;
	uint8_t *raw;
	size_t got, want;
	int rc;

	memset(d, 0, sizeof *d);
	d->path  = path;
	d->label = basename_of(path);

	f = fopen(path, "rb");
	if (!f) {
		fprintf(stderr, "tosdiff: %s: cannot open\n", path);
		return -1;
	}
	if (fseek(f, 0, SEEK_END) != 0 || (size = ftell(f)) < 0 ||
	    fseek(f, 0, SEEK_SET) != 0) {
		fclose(f);
		fprintf(stderr, "tosdiff: %s: cannot seek\n", path);
		return -1;
	}
	raw = (uint8_t *)malloc((size_t)size ? (size_t)size : 1);
	if (!raw) {
		fclose(f);
		fprintf(stderr, "tosdiff: out of memory\n");
		return -1;
	}
	got = fread(raw, 1, (size_t)size, f);
	fclose(f);
	if (got != (size_t)size) {
		free(raw);
		fprintf(stderr, "tosdiff: %s: short read\n", path);
		return -1;
	}

	rc = td_header_parse(raw, got, &d->h);
	if (rc != 0) {
		free(raw);
		fprintf(stderr, "tosdiff: %s: %s\n", path, header_error(rc));
		return -1;
	}

	want = (size_t)d->h.record_count * (size_t)d->h.record_size;
	if (got != TD_HEADER_SIZE + want) {
		free(raw);
		fprintf(stderr,
		        "tosdiff: %s: file is %lu bytes, header describes %lu "
		        "(%u records of %u bytes plus a %d byte header)\n",
		        path, (unsigned long)got,
		        (unsigned long)(TD_HEADER_SIZE + want),
		        d->h.record_count, d->h.record_size, TD_HEADER_SIZE);
		return -1;
	}

	d->records = (uint8_t *)malloc(want ? want : 1);
	if (!d->records) {
		free(raw);
		fprintf(stderr, "tosdiff: out of memory\n");
		return -1;
	}
	memcpy(d->records, raw + TD_HEADER_SIZE, want);
	d->bytes = want;
	free(raw);
	return 0;
}

static void dump_free(struct td_dump *d)
{
	free(d->records);
	d->records = NULL;
}

static const uint8_t *record_at(const struct td_dump *d, uint16_t i)
{
	return d->records + (size_t)i * (size_t)d->h.record_size;
}

static uint16_t record_case_id(const struct td_dump *d, uint16_t i)
{
	return td_load_be16(record_at(d, i));   /* case id is at offset 0 in all */
}

static uint16_t record_opcode(const struct td_dump *d, uint16_t i)
{
	return td_load_be16(record_at(d, i) + 2);
}

/* ------------------------------------------------------------------ */
/* --validate                                                          */
/* ------------------------------------------------------------------ */

static int cmd_validate(const char *dump_path, const char *cases_path)
{
	struct td_dump d;
	struct td_casefile cf;
	const char *why = NULL;
	int status = 0;
	uint16_t i;

	if (dump_load(&d, dump_path) != 0)
		return 2;

	printf("%s: area %s, %u records of %u bytes, build %s, case CRC %08X\n",
	       d.label, td_area_name(d.h.area), d.h.record_count,
	       d.h.record_size, d.h.build_id, d.h.case_crc);
	if (td_header_states_ptr(&d.h))
		printf("%s: format %u, pointer %u bytes, %s\n", d.label,
		       d.h.version, d.h.ptr_width, word_order_name(d.h.word_order));
	else
		printf("%s: format %u, pointer width and word order not stated\n",
		       d.label, d.h.version);

	/* case ids must be unique and ascending: they are line numbers */
	for (i = 1; i < d.h.record_count; i++) {
		if (record_case_id(&d, i) <= record_case_id(&d, (uint16_t)(i - 1))) {
			fprintf(stderr,
			        "tosdiff: %s: case id %u at record %u does not follow "
			        "%u -- ids are case-table line numbers and must ascend\n",
			        d.label, record_case_id(&d, i), i,
			        record_case_id(&d, (uint16_t)(i - 1)));
			status = 2;
		}
	}

	if (cases_path) {
		if (td_casefile_read(&cf, cases_path, d.h.area, &why) != 0) {
			fprintf(stderr, "tosdiff: %s: %s\n", cases_path,
			        why ? why : "cannot read");
			dump_free(&d);
			return 2;
		}
		if (cf.crc != d.h.case_crc) {
			fprintf(stderr,
			        "tosdiff: case table CRC mismatch: %s carries %08X, "
			        "%s is %08X -- the dump was not produced from this table\n",
			        d.label, d.h.case_crc, basename_of(cases_path), cf.crc);
			status = 2;
		}
		if (cf.count != d.h.record_count) {
			fprintf(stderr,
			        "tosdiff: %s holds %u records but %s holds %u cases\n",
			        d.label, d.h.record_count, basename_of(cases_path),
			        cf.count);
			status = 2;
		}
		if (status == 0) {
			for (i = 0; i < d.h.record_count; i++) {
				if (record_case_id(&d, i) != cf.line[i]) {
					fprintf(stderr,
					        "tosdiff: record %u carries case id %u, the case "
					        "table has %u on that position\n",
					        i, record_case_id(&d, i), cf.line[i]);
					status = 2;
				}
			}
		}
		if (status == 0)
			printf("%s: matches %s (%u cases)\n", d.label,
			       basename_of(cases_path), cf.count);
	}

	if (status == 0)
		printf("%s: accepted\n", d.label);
	dump_free(&d);
	return status;
}

/* ------------------------------------------------------------------ */
/* compare                                                             */
/* ------------------------------------------------------------------ */

static uint32_t field_get(const uint8_t *rec, const struct td_field *f,
                          uint16_t idx)
{
	const uint8_t *p = rec + f->off + (size_t)idx * f->width;

	switch (f->width) {
	case 1:  return (uint32_t)p[0];
	case 2:  return (uint32_t)td_load_be16(p);
	default: return td_load_be32(p);
	}
}

static void print_value(const struct td_field *f, uint32_t v)
{
	switch (f->width) {
	case 1:
		printf("0x%02X", (unsigned)v);
		break;
	case 2:
		if (f->is_signed)
			printf("0x%04X (%d)", (unsigned)v, (int)(int16_t)(uint16_t)v);
		else
			printf("0x%04X", (unsigned)v);
		break;
	default:
		if (f->is_signed)
			printf("0x%08X (%ld)", (unsigned)v, (long)(int32_t)v);
		else
			printf("0x%08X", (unsigned)v);
		break;
	}
}

static void print_case_header(const struct td_casefile *cf, uint16_t case_id,
                              uint16_t opcode, int *printed)
{
	const char *c;

	if (*printed)
		return;
	c = td_casefile_comment(cf, case_id);
	printf("\ncase %u  opcode %u%s%s\n", case_id, opcode, *c ? "  " : "", c);
	*printed = 1;
}

/*
 * The address check, per dump: every cell of every address-class field
 * must be below TD_ADDR_LIMIT (record.h; TASK/MEM11.md).  Its own verdict,
 * reported per side and per cell, never folded into "cases differ".  A dump
 * whose header does not state pointer width and word order (format 2) is
 * not read at all -- reading it would mean guessing both -- and its summary
 * says "not checkable".
 */
struct addr_check {
	unsigned long checked;
	unsigned long violations;
	unsigned long fields;      /* address fields met, for format 2 */
};

static void addr_check_side(const struct td_dump *d, const uint8_t *rec,
                            const struct td_field *f, struct addr_check *ac,
                            int label_w, const struct td_casefile *cf,
                            uint16_t case_id, uint16_t opcode, int *printed)
{
	uint16_t n, cells, words;

	ac->fields++;
	cells = td_addr_cell_count(f, &d->h);
	words = td_addr_cell_words(f, &d->h);
	for (n = 0; n < cells; n++) {
		uint64_t v = td_addr_cell_value(rec, f, &d->h, n);
		unsigned w0 = (unsigned)(f->first + n * words);
		char name[48];

		ac->checked++;
		if (td_addr_ok(v))
			continue;
		ac->violations++;
		print_case_header(cf, case_id, opcode, printed);
		snprintf(name, sizeof name, "%s[%u..%u]", f->name, w0,
		         w0 + words - 1u);
		printf("    %-24s ADDRESS %-*s = 0x%0*llX, not < 0x%08lX\n", name,
		       label_w, d->label, (int)(words * 4u),
		       (unsigned long long)v, (unsigned long)TD_ADDR_LIMIT);
	}
}

static void addr_check_summary(const struct td_dump *d,
                               const struct addr_check *ac)
{
	if (ac->fields == 0)
		return;   /* the area has no address field */
	if (!td_header_states_ptr(&d->h)) {
		printf("address check, %s: not checkable -- format %u states no "
		       "pointer width or word order (%lu address fields unread)\n",
		       d->label, d->h.version, ac->fields);
		return;
	}
	printf("address check, %s: format %u, pointer %u bytes, %s; "
	       "%lu address cells checked, %lu not < 0x%08lX\n",
	       d->label, d->h.version, d->h.ptr_width,
	       word_order_name(d->h.word_order), ac->checked, ac->violations,
	       (unsigned long)TD_ADDR_LIMIT);
}

/*
 * The kept check, per dump: a relational property within ONE side, like the
 * address check.  The raster frame records the MFDB pointer slots twice,
 * after the call (contrl_mfdb) and as they stood going in
 * (contrl_before_mfdb).  The VDI reads the two MFDB pointers and must not
 * write them: every address cell after the call must equal its twin before
 * the call.  The two values come from the same run of the same dumper, so
 * -- unlike a comparison across two machines -- their equality measures the
 * API, not the linker.
 *
 * The table names the pair; the cells come from the descriptors
 * (td_addr_cell_count / td_addr_cell_value), so the check follows the
 * dump's own pointer width and word order: contrl[7..8] and [9..10] on a
 * four-byte m68k, contrl[7..10] and [11..14] on an eight-byte AArch64.  Like
 * the address check it needs a format 3 header, and a format 2 dump is
 * reported as not checkable.  A violation is its own verdict, never folded
 * into "cases differ".
 */
struct kept_pair {
	uint16_t    area;
	const char *after;    /* address field, recorded after the call   */
	const char *before;   /* its twin, recorded before the call       */
};

static const struct kept_pair kept_pairs[] = {
	{ TD_AREA_RASTER, "contrl_mfdb", "contrl_before_mfdb" },
	{ 0, NULL, NULL }
};

/* At most this many pairs per area; kept_resolve refuses more. */
#define KEPT_MAX 4

struct kept_field {
	const struct td_field *after;
	const struct td_field *before;
};

static const struct td_field *field_by_name(const struct td_field *fields,
                                            const char *name)
{
	for (; fields->name; fields++)
		if (strcmp(fields->name, name) == 0)
			return fields;
	return NULL;
}

/*
 * The pairs of `area`, resolved against its descriptors.  Returns the number
 * of pairs, or -1 when a pair does not resolve to two address fields of the
 * same shape -- a renamed or reshaped descriptor must stop the comparison,
 * not switch the check off without a word.
 */
static int kept_resolve(uint16_t area, const struct td_field *fields,
                        struct kept_field *out)
{
	const struct kept_pair *p;
	int n = 0;

	for (p = kept_pairs; p->after; p++) {
		const struct td_field *fa, *fb;

		if (p->area != area)
			continue;
		fa = field_by_name(fields, p->after);
		fb = field_by_name(fields, p->before);
		if (!fa || !fb || n >= KEPT_MAX ||
		    fa->compare != TD_FIELD_ADDRESS ||
		    fb->compare != TD_FIELD_ADDRESS ||
		    fa->addr_cell != fb->addr_cell || fa->count != fb->count ||
		    fa->width != fb->width) {
			fprintf(stderr,
			        "tosdiff: refusing: kept check %s / %s does not "
			        "resolve to two address fields of one shape in area %s\n",
			        p->after, p->before, td_area_name(area));
			return -1;
		}
		out[n].after  = fa;
		out[n].before = fb;
		n++;
	}
	return n;
}

struct kept_check {
	unsigned long checked;
	unsigned long changed;
};

static void kept_check_side(const struct td_dump *d, const uint8_t *rec,
                            const struct kept_field *k, struct kept_check *kc,
                            int label_w, const struct td_casefile *cf,
                            uint16_t case_id, uint16_t opcode, int *printed)
{
	uint16_t n, cells, words;

	cells = td_addr_cell_count(k->after, &d->h);   /* 0 on format 2 */
	words = td_addr_cell_words(k->after, &d->h);
	for (n = 0; n < cells; n++) {
		uint64_t va = td_addr_cell_value(rec, k->after, &d->h, n);
		uint64_t vb = td_addr_cell_value(rec, k->before, &d->h, n);
		unsigned w0 = (unsigned)(k->after->first + n * words);
		char name[48];

		kc->checked++;
		if (va == vb)
			continue;
		kc->changed++;
		print_case_header(cf, case_id, opcode, printed);
		snprintf(name, sizeof name, "%s[%u..%u]", k->after->name, w0,
		         w0 + words - 1u);
		printf("    %-24s KEPT %-*s = 0x%0*llX, before the call 0x%0*llX\n",
		       name, label_w, d->label, (int)(words * 4u),
		       (unsigned long long)va, (int)(words * 4u),
		       (unsigned long long)vb);
	}
}

static void kept_check_summary(const struct td_dump *d,
                               const struct kept_field *k, int nkept,
                               const struct kept_check *kc)
{
	int i;

	for (i = 0; i < nkept; i++) {
		if (!td_header_states_ptr(&d->h))
			printf("kept check, %s: not checkable -- format %u states no "
			       "pointer width or word order (%s against %s unread)\n",
			       d->label, d->h.version, k[i].after->name,
			       k[i].before->name);
		else
			printf("kept check, %s: format %u; %s against %s, %lu cells "
			       "checked, %lu changed by the call\n",
			       d->label, d->h.version, k[i].after->name,
			       k[i].before->name, kc[i].checked, kc[i].changed);
	}
}

static int cmd_compare(const char *cases_path, const char *a_path,
                       const char *b_path)
{
	struct td_casefile cf;
	struct td_dump a, b;
	const struct td_field *fields;
	const char *why = NULL;
	struct addr_check acheck_a, acheck_b;
	struct kept_field kept[KEPT_MAX];
	struct kept_check kcheck_a[KEPT_MAX], kcheck_b[KEPT_MAX];
	unsigned long kept_changed = 0;
	uint16_t rec, differing = 0;
	int label_w, nkept, ki;

	memset(&acheck_a, 0, sizeof acheck_a);
	memset(&acheck_b, 0, sizeof acheck_b);
	memset(kcheck_a, 0, sizeof kcheck_a);
	memset(kcheck_b, 0, sizeof kcheck_b);

	/* The dumps come first: the case table's comment column is per area
	 * (casefile.h), and the area is the dump's to state, not the caller's
	 * to guess from a file name. */
	if (dump_load(&a, a_path) != 0)
		return 2;
	if (dump_load(&b, b_path) != 0) {
		dump_free(&a);
		return 2;
	}
	if (td_casefile_read(&cf, cases_path, a.h.area, &why) != 0) {
		fprintf(stderr, "tosdiff: %s: %s\n", cases_path,
		        why ? why : "cannot read");
		dump_free(&a);
		dump_free(&b);
		return 2;
	}

	/* Spec section 3: refuse rather than compare files that cannot be
	 * compared.  Comparing dumps from different case tables silently
	 * produces nonsense. */
	if (a.h.area != b.h.area) {
		fprintf(stderr, "tosdiff: refusing: %s is area %s, %s is area %s\n",
		        a.label, td_area_name(a.h.area),
		        b.label, td_area_name(b.h.area));
		goto refuse;
	}
	if (a.h.record_size != b.h.record_size) {
		fprintf(stderr, "tosdiff: refusing: record size %u vs %u\n",
		        a.h.record_size, b.h.record_size);
		goto refuse;
	}
	if (a.h.version != b.h.version) {
		/* Format 3 states pointer width and word order, format 2 does
		 * not: the address check would run on one side and not the
		 * other, and a format 2 side cannot be made to say.  Re-record
		 * it instead (docs/TOS-REFERENCE-DUMPER.md section 3). */
		fprintf(stderr,
		        "tosdiff: refusing: %s is format %u, %s is format %u -- "
		        "dumps of different formats are not compared; re-record "
		        "the older one\n",
		        a.label, a.h.version, b.label, b.h.version);
		goto refuse;
	}
	if (a.h.case_crc != b.h.case_crc) {
		fprintf(stderr,
		        "tosdiff: refusing: case table CRC %08X vs %08X -- the two "
		        "dumps were not produced from the same table\n",
		        a.h.case_crc, b.h.case_crc);
		goto refuse;
	}
	if (a.h.case_crc != cf.crc) {
		fprintf(stderr,
		        "tosdiff: refusing: both dumps carry case table CRC %08X, "
		        "%s is %08X\n",
		        a.h.case_crc, basename_of(cases_path), cf.crc);
		goto refuse;
	}
	if (a.h.record_count != b.h.record_count) {
		fprintf(stderr, "tosdiff: refusing: %u records vs %u\n",
		        a.h.record_count, b.h.record_count);
		goto refuse;
	}

	fields = td_area_fields(a.h.area);
	if (!fields) {
		fprintf(stderr, "tosdiff: refusing: no field table for area %u\n",
		        a.h.area);
		goto refuse;
	}
	if (a.h.area != TD_AREA_VDI && a.h.area != TD_AREA_AES &&
	    a.h.area != TD_AREA_RASTER) {
		fprintf(stderr,
		        "tosdiff: comparing area %s needs the retclass and capture "
		        "columns of the case table, which land with task T7; "
		        "--validate works for this area today\n",
		        td_area_name(a.h.area));
		goto refuse;
	}
	nkept = kept_resolve(a.h.area, fields, kept);
	if (nkept < 0)
		goto refuse;

	label_w = (int)strlen(a.label);
	if ((int)strlen(b.label) > label_w)
		label_w = (int)strlen(b.label);

	printf("%s vs %s -- area %s, %u cases from %s\n",
	       a.label, b.label, td_area_name(a.h.area), a.h.record_count,
	       basename_of(cases_path));

	for (rec = 0; rec < a.h.record_count; rec++) {
		const uint8_t *ra = record_at(&a, rec);
		const uint8_t *rb = record_at(&b, rec);
		uint16_t case_id = record_case_id(&a, rec);
		int printed_header = 0, value_differs = 0;
		int fi;

		if (record_case_id(&b, rec) != case_id) {
			fprintf(stderr,
			        "tosdiff: refusing: record %u is case %u in %s but case "
			        "%u in %s\n",
			        rec, case_id, a.label, record_case_id(&b, rec), b.label);
			goto refuse;
		}

		for (fi = 0; fields[fi].name; fi++) {
			const struct td_field *f = &fields[fi];
			uint16_t idx;

			/* Field class address: never compared by value -- the two
			 * values cannot be equal by construction, so a difference
			 * would measure the linker rather than the API.  Each
			 * side's cells are checked against the relational
			 * property instead (record.h). */
			if (f->compare == TD_FIELD_ADDRESS) {
				addr_check_side(&a, ra, f, &acheck_a, label_w, &cf,
				                case_id, record_opcode(&a, rec),
				                &printed_header);
				addr_check_side(&b, rb, f, &acheck_b, label_w, &cf,
				                case_id, record_opcode(&a, rec),
				                &printed_header);
				continue;
			}

			for (idx = 0; idx < f->count; idx++) {
				uint32_t va = field_get(ra, f, idx);
				uint32_t vb = field_get(rb, f, idx);

				if (va == vb)
					continue;

				print_case_header(&cf, case_id, record_opcode(&a, rec),
				                  &printed_header);
				value_differs = 1;
				printf("    %s[%u]", f->name, f->first + idx);
				printf("%*s", (int)(20 - strlen(f->name)) > 0
				              ? (int)(20 - strlen(f->name)) : 1, "");
				printf("%-*s = ", label_w, a.label);
				print_value(f, va);
				printf("   %-*s = ", label_w, b.label);
				print_value(f, vb);
				printf("\n");
			}
		}
		for (ki = 0; ki < nkept; ki++) {
			kept_check_side(&a, ra, &kept[ki], &kcheck_a[ki], label_w, &cf,
			                case_id, record_opcode(&a, rec), &printed_header);
			kept_check_side(&b, rb, &kept[ki], &kcheck_b[ki], label_w, &cf,
			                case_id, record_opcode(&a, rec), &printed_header);
		}
		if (value_differs)
			differing++;
	}

	printf("\n%u of %u cases differ\n", differing, a.h.record_count);
	addr_check_summary(&a, &acheck_a);
	addr_check_summary(&b, &acheck_b);
	kept_check_summary(&a, kept, nkept, kcheck_a);
	kept_check_summary(&b, kept, nkept, kcheck_b);
	for (ki = 0; ki < nkept; ki++)
		kept_changed += kcheck_a[ki].changed + kcheck_b[ki].changed;
	dump_free(&a);
	dump_free(&b);
	return (differing || acheck_a.violations || acheck_b.violations ||
	        kept_changed) ? 1 : 0;

refuse:
	dump_free(&a);
	dump_free(&b);
	return 2;
}

int main(int argc, char **argv)
{
	if (argc >= 3 && strcmp(argv[1], "--validate") == 0) {
		if (argc > 4) {
			fprintf(stderr, "usage: tosdiff --validate DUMP.BIN [CASES.TSV]\n");
			return 2;
		}
		return cmd_validate(argv[2], argc == 4 ? argv[3] : NULL);
	}
	if (argc == 4)
		return cmd_compare(argv[1], argv[2], argv[3]);

	fprintf(stderr,
	        "usage: tosdiff --validate DUMP.BIN [CASES.TSV]\n"
	        "       tosdiff CASES.TSV A.BIN B.BIN\n");
	return 2;
}
