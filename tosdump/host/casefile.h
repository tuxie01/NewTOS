/*
 * casefile.h -- host-side reader for a case table (spec section 6).
 *
 * T1 needs three things from a case table: its CRC32 (the identity the record
 * header carries), how many cases it holds, and the comment column so that a
 * difference report can name the case.  Typed parsing of the value columns is
 * the runner's job and lands with T2.
 *
 * Two of the three are area-agnostic: the CRC is over the raw bytes, and a
 * case id is a physical line number, which the shared line iterator and the
 * shared prelude rule in core/cases.c settle for every area alike.  The
 * COMMENT COLUMN is not.  It is column five in the VDI form and column seven
 * in the AES form (spec section 6), and no amount of looking at a line
 * settles which, because a VDI comment may itself hold tabs -- counting the
 * tabs on such a line would make it look like the AES form.  So the area is
 * an argument.
 */
#ifndef TOSDUMP_CASEFILE_H
#define TOSDUMP_CASEFILE_H

#include "record.h"

struct td_casefile {
	uint32_t crc;                                  /* CRC32 of the raw file  */
	uint16_t count;                                /* cases held             */
	uint16_t line[TD_MAX_CASES];                   /* physical line = case id*/
	char     comment[TD_MAX_CASES][TD_COMMENT_LEN];
	/* Call areas only (BIOS / XBIOS / GEMDOS): the retclass column, which
	 * the comparator needs to judge `ret` (spec section 6).  Read with the
	 * core's own call-table parser, so a table the dumper would refuse is
	 * refused here too, and with the same reason. */
	uint8_t  retclass[TD_MAX_CASES];               /* TD_RET_*               */
	uint8_t  retwidth[TD_MAX_CASES];               /* TD_RET_WIDTH_*         */
};

/*
 * `area` is one of TD_AREA_*: it selects the column count the comment is
 * taken from, and -- for a call area -- that the retclass column is read.
 * Returns 0 on success, -1 on I/O failure, -2 if the table holds more than
 * TD_MAX_CASES cases, -3 if a call table is refused by the core's loader.
 * On failure *why is set to a static message.
 */
int td_casefile_read(struct td_casefile *cf, const char *path, uint16_t area,
                     const char **why);

/* Comment for a case id (physical line number), or "" if the id is unknown. */
const char *td_casefile_comment(const struct td_casefile *cf, uint16_t case_id);

#endif /* TOSDUMP_CASEFILE_H */
