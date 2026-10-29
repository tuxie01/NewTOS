/*
 * nohard.h -- NewTOS overlay helper for TeraDesk (TASK/SDK07, patch site
 * SDK07). Not an upstream file: it exists only in sdk/overlays/teradesk/ and
 * is layered into the shadow tree by sdk/apps/build-teradesk.sh like every
 * other overlay file. Included by the video.c overlay only.
 *
 * NewTOS publishes _MCH = 0xFFFFFFFF (MCH_NOHARD: no Atari machine hardware;
 * ADR-012 amendment 2026-10-28, measured in
 * measurements/measurement-2026-10-29-arch06/RUNINFO.TXT RESULT 1). On such a
 * machine there is no ACIA at 0xFFFC00, so TeraDesk's ST overscan (Lacescan)
 * switch in get_set_video() must not run: it writes that register after
 * Super(0L) and dies with a Bus Error (same RUNINFO.TXT, RESULT 2).
 *
 * find_cookie() (library/utility/cookie.c, patch site SDK04-C6) hands the
 * 32-bit cookie value out zero-extended, so the test compares the low
 * 32 bits. An absent _MCH (a real ST without a jar) is "hardware present",
 * as upstream assumes.
 */

#ifndef NEWTOS_NOHARD_H
#define NEWTOS_NOHARD_H

#include <stdint.h>

#ifndef C__MCH
#define C__MCH 0x5F4D4348L			/* Machine Type */
#endif

#define NEWTOS_MCH_NOHARD 0xFFFFFFFFUL	/* _MCH value: no Atari machine hardware */

/* TRUE when the cookie jar says there is no Atari machine hardware. */
static inline bool newtos_mch_nohard(void)
{
	long mch = 0;

	if (!find_cookie(C__MCH, &mch))
		return FALSE;
	return (uint32_t)mch == (uint32_t)NEWTOS_MCH_NOHARD;
}

#endif /* NEWTOS_NOHARD_H */
