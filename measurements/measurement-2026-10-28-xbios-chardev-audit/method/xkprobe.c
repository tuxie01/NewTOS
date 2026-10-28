/*
 * xkprobe.c -- input probe for the audit docs/audits/2026-10-28-xbios-chardev.md:
 * what Keytbl, Bioskeys and a Kbdvbase mousevec hook do to real input.
 *
 *  1. Kbdvbase()->mousevec = counting hook; Keytbl(my_n,-1,-1) with
 *     my_n[i] = i ^ 0x55 (scancode 0x1E 'a' -> 0x4B 'K').
 *  2. "KEYREADY" on the console; the harness then sends: right button down,
 *     right button up (one IKBD mouse packet each), key 'a'; and about 2 s
 *     later key 'a' again.
 *  3. k1 = Bconin(2)          -- decoded with my_n
 *     Bioskeys()              -- EmuTOS: back to the country tables
 *     k2 = Bconin(2)          -- the second 'a'
 *  4. mousevec restored; Keytbl(n0,s0,c0) restores the tables whatever
 *     Bioskeys did; XKPROBE.LOG.
 *
 * Same C source for m68k (mintlib) and AArch64 (SDK mintlib), like xcprobe.c.
 */
#include <osbind.h>
#include <stdio.h>
#include <string.h>

static volatile long n_mouse;
static volatile unsigned char mpkt[3];
static void h_mouse(void *p)
{
    mpkt[0] = ((unsigned char *)p)[0];
    mpkt[1] = ((unsigned char *)p)[1];
    mpkt[2] = ((unsigned char *)p)[2];
    n_mouse++;
}

static unsigned char my_n[128];

int main(void)
{
    char out[1024];
    int n = 0, i;
    long fh, k1, k2;
    _KBDVECS *kv = Kbdvbase();
    void (*o_mouse)(void *) = kv->mousevec;
    _KEYTAB *kt = (_KEYTAB *)Keytbl(-1L, -1L, -1L);
    char *n0 = kt->unshift, *s0 = kt->shift, *c0 = kt->caps;

    for (i = 0; i < 128; i++)
        my_n[i] = (unsigned char)(i ^ 0x55);
    kv->mousevec = h_mouse;
    Keytbl((long)my_n, -1L, -1L);
    Cconws("KEYREADY\r\n");
    k1 = Bconin(2);
    Bioskeys();
    kt = (_KEYTAB *)Keytbl(-1L, -1L, -1L);
    n += sprintf(out + n, "X01 after Bioskeys unshift_is_my=%d unshift_is_n0=%d\n",
                 kt->unshift == (char *)my_n, kt->unshift == n0);
    k2 = Bconin(2);
    kv->mousevec = o_mouse;
    Keytbl((long)n0, (long)s0, (long)c0);
    n += sprintf(out + n, "X02 k1 Bconin(2) with Keytbl(my_n) ret=%08lx\n",
                 (unsigned long)k1 & 0xFFFFFFFFUL);
    n += sprintf(out + n, "X03 k2 Bconin(2) after Bioskeys ret=%08lx\n",
                 (unsigned long)k2 & 0xFFFFFFFFUL);
    n += sprintf(out + n, "X04 mousevec hook calls=%ld last_packet=%02x%02x%02x\n",
                 (long)n_mouse, mpkt[0], mpkt[1], mpkt[2]);
    fh = Fcreate("XKPROBE.LOG", 0);
    if (fh >= 0) {
        Fwrite((short)fh, (long)n, out);
        Fclose((short)fh);
    }
    Cconws("XKPROBE done\r\n");
    return 0;
}
