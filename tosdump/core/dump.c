/*
 * dump.c -- write the buffered records out as one file (spec section 7).
 */
#include <stdio.h>
#include <string.h>

#include "dump.h"

int td_dump_write(const char *path, uint16_t area, uint16_t count,
                  const char *build_id, uint32_t case_crc,
                  const uint8_t *records, size_t bytes)
{
	struct td_header h;
	uint8_t hdr[TD_HEADER_SIZE];
	uint16_t rec_size;
	FILE *f;

	rec_size = td_area_record_size(area);
	if (rec_size == 0 || bytes != (size_t)count * (size_t)rec_size)
		return TD_DUMP_ERR_SIZE;

	memset(&h, 0, sizeof h);
	h.version      = TD_FORMAT_VERSION;
	h.area         = area;
	h.record_size  = rec_size;
	h.record_count = count;
	strncpy(h.build_id, build_id ? build_id : "", TD_HDR_BUILDID_LEN);
	h.build_id[TD_HDR_BUILDID_LEN] = '\0';
	h.case_crc     = case_crc;
	/* Format 3: the machine's own pointer width and word order, measured
	 * here rather than configured, so both targets run the same lines. */
	h.ptr_width    = td_native_ptr_width();
	h.word_order   = td_native_word_order();
	td_header_write(hdr, &h);

	f = fopen(path, "wb");
	if (!f)
		return TD_DUMP_ERR_OPEN;
	if (fwrite(hdr, 1, sizeof hdr, f) != sizeof hdr ||
	    (bytes && fwrite(records, 1, bytes, f) != bytes)) {
		fclose(f);
		return TD_DUMP_ERR_WRITE;
	}
	if (fclose(f) != 0)
		return TD_DUMP_ERR_WRITE;
	return 0;
}
