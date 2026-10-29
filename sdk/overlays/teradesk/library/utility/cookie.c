/*
 * Utility functions for Teradesk. Copyright 1993 - 2002  W. Klaren,
 *                                           2002 - 2009  H. Robbers,
 *                                           2003 - 2009  Dj. Vukovic
 *
 * This file is part of Teradesk.
 *
 * Teradesk is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 2 of the License, or
 * (at your option) any later version.
 *
 * Teradesk is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Teradesk; if not, write to the Free Software
 * Foundation, Inc., 59 Temple Place, Suite 330, Boston, MA  02111-1307  USA
 */


/*
 * NewTOS overlay (LP64 ABI correction, per sdk/apps/build-teradesk.sh:16-31;
 * TASK/SDK04, finding C6 of docs/analysis/lp64-tos-struct-survey-2026-10-28.md).
 * The ONLY change vs. the vendored cookie.c is the jar walk in find_cookie()'s
 * Setexc branch, the branch the -D_TOS_ONLY build compiles (_MINT_ == 0):
 *
 *  - A TOS cookie slot is two 32-bit LONGs, 8 bytes (docs/api/bios/_cookie-jar.md;
 *    the kernel writes the $5A0 mirror that way). Upstream walks it as
 *    `COOKIE *` (library/xdialog/library.h: `long name; long value;`), which is
 *    16 bytes on LP64: `cookie++` skipped every second slot, `cookie->name`
 *    fused a tag with its own value, so a tag matched only in an even slot with
 *    value 0, and then `cookie->value` returned the next slot's tag and value.
 *    Measured with the probe sdk/overlays/teradesk/tests/cookie-probe.c
 *    (measurements/measurement-2026-10-29-sdk04-c6/RUNINFO.TXT): 3 of 12 tags
 *    agreed with Getcookie, only the three absent ones.
 *  - The walk now reads the slot as two uint32_t; the tag compares as the
 *    32-bit LONG it is on m68k, the value is widened through uint32_t (zero-
 *    extended, as Getcookie hands it out; sdk/libc/newtos-arch/getcookie.c).
 *    The jar pointer is Setexc's return value, as upstream; the patch site is
 *    marked "NewTOS overlay, patch site SDK04-C6" below.
 *
 * `<stdint.h>` is pulled in for uint32_t/uintptr_t. The COOKIE type and the
 * `#if 0` block (install_cookie, not used in TeraDesk) are left as they are.
 * CRLF line endings kept as in the vendored file.
 */

#include <library.h>
#include <mint/ssystem.h>
#include <stdint.h>

#undef p_cookie
#define p_cookie	( * (COOKIE **) 0x5A0L )


/*
 * return cookie value or return -1 if cookie not found.
 * If ssystem function is available (i.e. in Mint 1.15 or more)
 * then use that function.
 * Note: earlier version of this routine erronelosly returned
 * input value of "name" instead of -1 if there was no cookie jar
 */

bool find_cookie(long name, long *value)
{
	const uint32_t *cookie;	/* NewTOS overlay, patch site SDK04-C6: one slot = two 32-bit LONGs */
	long cvalue = 0;

#if _MINT_
	if (have_ssystem)
	{
		if (Ssystem(S_GETCOOKIE, name, (long) &cvalue) != 0)
			return FALSE;
	} else
#endif
	{
		/*
		 * cookie jar ptr is longword aligned, thus
		 * we can use Setexc to fetch its value
		 */
		cookie = (const uint32_t *)(uintptr_t)Setexc(0x5A0 / 4, (void (*)())-1);

		if (cookie == NULL)
			return FALSE;
		/* NewTOS overlay, patch site SDK04-C6: cookie[0] = tag, cookie[1] = value, step 8 bytes */
		while (cookie[0] != 0 && cookie[0] != (uint32_t)name)
			cookie += 2;

		if (cookie[0] == 0)
			return FALSE;
		cvalue = (long)cookie[1];
	}

	if (value)
		*value = cvalue;
	return TRUE;
}


#if 0									/* All following routines are currently not used in Teradesk */

long o_resvalid;

void (*o_resvector) (void);

#if __AHCC__
void __asm__ jmpa6(void)
{
	addq.l	#4, sp			; adjust stack, jmpa6 is called
	jmp 	(a6)
}
#else
void jmpa6(void) 0x4ED6;
#endif


static void cookie_reset(void)
{
	p_cookie = NULL;
	resvector = o_resvector;
	resvalid = o_resvalid;
	jmpa6();
}


/* 
 * r = -1 : fout,
 * r =  0 : geen fout,
 * r =  1 : nieuw cookie geinstalleerd, reset vast,
 * r =  2 : nieuw cookie geinstalleerd, niet reset vast. 
 */

_WORD install_cookie(long name, long value, COOKIE *buffer, long l)
{
	void *stack;
	COOKIE *cookie;
	_WORD r, i, j;

	stack = (void *) Super(NULL);

	if ((cookie = p_cookie) != NULL)
	{
		i = 0;

		while (cookie[i].name != 0L)
			i++;

		if (cookie[i].value > (i + 1))
		{
			cookie[i + 1] = cookie[i];
			cookie[i].name = name;
			cookie[i].value = value;

			r = 0;
		} else
		{
			if (l <= cookie[i].value || buffer == NULL)
			{
				r = -1;
			} else
			{
				_WORD e;				/* reserve 'end' for language */

				e = (_WORD) cookie[i].value - 1;

				for (j = 0; j < e; j++)
					buffer[j] = cookie[j];

				p_cookie = buffer;

				cookie[i].name = name;
				cookie[i].value = value;
				cookie[i + 1].name = 0L;
				cookie[i + 1].value = l;

				r = 1;
			}
		}
	} else
	{
		if ((l < 2) || (buffer == NULL))	/* cookie te klein */
		{
			r = -1;
		} else
		{
			o_resvalid = resvalid;
			o_resvector = resvector;
			resvalid = 0x31415926L;
			resvector = cookie_reset;

			p_cookie = buffer;

			buffer[0].name = name;
			buffer[0].value = value;
			buffer[1].name = 0L;
			buffer[1].value = l;

			r = 1;
		}
	}

	Super(stack);

	return r;
}
#endif
