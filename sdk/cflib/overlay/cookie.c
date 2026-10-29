/*
 * CFLIB, a GEM library for ATARI/TOS
 * Copyright (C) 1999, 2000 Christian Felsch
 *
 * Modified for FreeMiNT CVS by Frank Naumann <fnaumann@freemint.de>
 *
 * Please send suggestions, patches or bug reports to me or
 * the MiNT mailing list.
 *
 *
 * This library is free software; you can redistribute it and/or
 * modify it under the terms of the GNU Lesser General Public
 * License as published by the Free Software Foundation; either
 * version 2.1 of the License, or (at your option) any later version.
 *
 * This library is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
 * Lesser General Public License for more details.
 *
 * You should have received a copy of the GNU Lesser General Public
 * License along with this library; if not, write to the Free Software
 * Foundation, Inc., 59 Temple Place, Suite 330, Boston, MA 02111-1307  USA
 *
 */

#include "intern.h"

#if !defined(__PUREC__)

#include <mint/cookie.h>

/*
 * NewTOS overlay, patch site SDK05 (TASK/SDK05): the cookie tag.
 *
 * Upstream builds the tag by writing the four name characters into the first
 * four bytes of a `union { long; char[4]; }` and handing the long to
 * Getcookie(). That is the tag only where `long` is 4 bytes and big-endian
 * (m68k). On AArch64 LP64 the four bytes are the LOW half of an 8-byte
 * little-endian long, so "_IDT" becomes 0x5444495F instead of the jar tag
 * 0x5F494454 (tos-api/src/cookie.rs, `u32::from_be_bytes`), and the upper four
 * bytes are never written at all. getcookie() then found no cookie.
 *
 * The tag is a TOS cookie name: four ASCII characters, the first in bits
 * 31..24 (mint/cookie.h C_* constants, e.g. C__IDT 0x5F494454L; a multi-
 * character literal '_IDT' has the same value). We build exactly that with
 * shifts -- independent of byte order and of the width of `long` -- and the
 * whole long is defined (bits 63..32 zero). The characters are taken as
 * unsigned so a byte >= 0x80 cannot sign-extend into the upper bits.
 */
#ifdef __NEWTOS__
#define CFLIB_COOKIE_TAG(c) \
	((long)(((unsigned long)(unsigned char)(c)[0] << 24) | \
		((unsigned long)(unsigned char)(c)[1] << 16) | \
		((unsigned long)(unsigned char)(c)[2] << 8) | \
		 (unsigned long)(unsigned char)(c)[3]))
#endif

int getcookie(const char *cookie, long *p_value)
{
	long v;
	int i;
#ifdef __NEWTOS__
	i = Getcookie(CFLIB_COOKIE_TAG(cookie), &v);
#else
	union
	{
		long as_long;
		char as_char[4];
	} cast;

	/* genialer Cast, oder? :-)) */
	cast.as_char[0] = cookie[0];
	cast.as_char[1] = cookie[1];
	cast.as_char[2] = cookie[2];
	cast.as_char[3] = cookie[3];

	i = Getcookie(cast.as_long, &v);
#endif
	if (p_value)
		*p_value = v;

	return i == C_FOUND;
}

#else

static long *my_cookiejar = NULL;

static long get_cookie_adr(void)
{
	return (long)(*((long **) 0x5A0L));
}


int getcookie(const char *cookie, long *p_value)
{
	long *cookiejar, c;
	union
	{
		long as_long;
		char as_char[4];
	} cast;

	if (my_cookiejar == NULL)
		my_cookiejar = (long *)Supexec(get_cookie_adr);

	cookiejar = my_cookiejar;

	if (!cookiejar || !cookiejar[0])
		return FALSE;

	/* Ein einfacherer Cast fiel mir nicht ein :-)) */
	cast.as_char[0] = cookie[0];
	cast.as_char[1] = cookie[1];
	cast.as_char[2] = cookie[2];
	cast.as_char[3] = cookie[3];
	c = cast.as_long;

	do
	{
		if (cookiejar[0] == c)
		{
			if (p_value)
				*p_value = cookiejar[1];
			return TRUE;
		}
		else
			cookiejar = &(cookiejar[2]);
	} while (cookiejar[0] || !cookie);

	if (p_value)
		*p_value = 0;

	return FALSE;
}

#endif
