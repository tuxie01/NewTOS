/*
 * d70prog.c -- DESK70: a text-mode TOS program started from the desktop.
 * One C source for both sides: m68k (mintlib osbind.h, .TOS/.TTP on EmuTOS)
 * and AArch64 (sdk/include/newtos.h, .NCP/.NTP on NewTOS).
 *
 * Variant, at build time:
 *   D70_MODE=0  print and Pterm(0) at once
 *   D70_MODE=1  print, wait about 6 s by polling Tgettime (no key, no Cconin),
 *               print one more line, Pterm(0)
 *   D70_MODE=2  print, Cconin(), print one more line, Pterm(0): the control,
 *               it needs a key, so the desktop must NOT come back without one
 */
#ifdef __m68k__
#include <osbind.h>
#define GETTIME() Tgettime()
#else
#include <newtos.h>
#define GETTIME() gemdos0(0x2C)
#endif

#ifndef D70_MODE
#define D70_MODE 0
#endif

int main(void)
{
    Cconws("DESK70 TEXT PROGRAM, LINE 1\r\n");
    Cconws("DESK70 TEXT PROGRAM, LINE 2\r\n");
    Cconws("DESK70 TEXT PROGRAM, LINE 3\r\n");
    Cconws("DESK70 TEXT PROGRAM, LINE 4\r\n");
#if D70_MODE == 1
    {
        long t0 = GETTIME(), t;
        int n = 0;
        while (n < 6) {
            t = GETTIME();
            if (t != t0) {
                t0 = t;
                n += 2;
            }
        }
    }
    Cconws("DESK70 AFTER DELAY, LAST LINE\r\n");
#elif D70_MODE == 2
    Cconws("DESK70 WAITING IN CCONIN\r\n");
    Cconin();
    Cconws("DESK70 GOT A KEY, LAST LINE\r\n");
#endif
    return 0;
}
