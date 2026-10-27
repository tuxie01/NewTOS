/*
 * Utility functions for Teradesk. Copyright 1993 - 2002  W. Klaren,
 *                                           2002 - 2003  H. Robbers,
 *                                           2003 - 2008  Dj. Vukovic
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
 * NewTOS overlay (LP64 ABI correction, per sdk/apps/build-teradesk.sh:15-28;
 * TASK/MEM12). The ONLY change vs. the vendored gettos.c is the width of the
 * _sysbase read:
 *
 *  - _sysbase ($4F2) is a 4-byte LONG cell. Upstream reads it as an
 *    OSHEADER pointer, which is 8 bytes on LP64, so the load also took
 *    _shell_p ($4F6) as the pointer's upper half. That is only right while
 *    _shell_p is 0; once a shell sets it, get_tosversion() dereferences a
 *    broken pointer. It now reads the 32-bit word and widens it through
 *    uintptr_t (NewTOS EL0 VAs are < 2^32).
 *
 * `<stdint.h>` is pulled in for uint32_t/uintptr_t. CRLF line endings kept as
 * in the vendored file.
 */

#include <library.h>
#include <mint/sysvars.h>
#include <stdint.h>

#undef _sysbase
#define _sysbase     ( (OSHEADER *) (uintptr_t) * (volatile uint32_t *) 0x4F2L )

/*
 * Return TOS version to be read as a hex number,
 * e.g. 0x104, 0x206, etc.
 */

_WORD get_tosversion(void)
{
	void *stack;
	_WORD version;

	stack = (void *) Super(NULL);
	version = _sysbase->os_version;
	Super(stack);
	return version;
}


/*
 * Return AES version to be read as a hex number,
 * e.g. 0x340, 0x399, etc.
 */

_WORD get_aesversion(void)
{
	return _AESversion;
}
