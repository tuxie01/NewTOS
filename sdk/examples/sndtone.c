/*
 * sndtone.c -- the jack test of the XBIOS DMA sound group (TASK/SND05).
 *
 * A native NewTOS console program (`.NCP`, ADR-018). It plays a 440 Hz sine
 * tone through `Setbuffer`/`Buffoper` in three passes of two seconds each:
 *
 *   1. both channels,
 *   2. the left channel only (right channel silent),
 *   3. the right channel only (left channel silent),
 *
 * and prints every call's return value, so the serial log shows what the
 * ear cannot (the pointer at the end of each pass, how many polls it took).
 *
 * The sound format is the one a Falcon program uses: 16-bit stereo, signed,
 * big-endian (the NewTOS default, docs/api/xbios/_dmasound-group.md §9.3.1),
 * `Devconnect(DMAPLAY, DAC, CLK25M, CLK50K, NO_SHAKE)` = prescale 1 =
 * 49 170 Hz on the Falcon clock (§4.7). No frame-end interrupt exists on
 * NewTOS (§9.1, option D), so the end of a pass is found by polling
 * `Buffoper(-1)` until bit 0 drops (S3).
 *
 * Without a sound backend (QEMU, or a Pi whose firmware reports no PWM
 * clock) `Locksnd` answers its own function number 0x80; the program says so
 * and ends with exit code 1.
 *
 * Spec: docs/api/xbios/_dmasound-group.md (§3, §9, §11).
 * Build: sdk/build.sh sdk/examples/sndtone.c <out>/SNDTONE.NCP
 */
#include <newtos.h>

/* XBIOS function numbers (docs/api/xbios/_dmasound-group.md §2). */
#define LOCKSND     0x80
#define UNLOCKSND   0x81
#define SOUNDCMD    0x82
#define SETBUFFER   0x83
#define SETMODE     0x84
#define BUFFOPER    0x88
#define DEVCONNECT  0x8B
#define BUFFPTR     0x8D

/* GEMDOS Malloc / Mfree (docs/api/gemdos/memory/). */
#define MALLOC      0x48
#define MFREE       0x49

#define RATE_HZ     49170L          /* prescale 1 on the 25.175 MHz clock */
#define SECONDS     2L
#define FRAMES      (RATE_HZ * SECONDS)
#define BYTES       (FRAMES * 4L)   /* 16-bit stereo: L L R R */
#define AMPLITUDE   16384L          /* half scale */

/* Polls without any pointer movement before the program gives up: the
 * pointer moves once per 200 Hz tick, so this is many seconds. */
#define STALL_POLLS 5000000L

/* The native SndBufPtr (§9.2): play and record as 64-bit pointers. */
struct sndbufptr {
    char *play;
    char *record;
    long reserve1;
    long reserve2;
};

/* A quarter of a sine wave, 32767 * sin(i * pi / 128) for i = 0..64. */
static const short quarter[65] = {
    0, 804, 1608, 2410, 3212, 4011, 4808, 5602,
    6393, 7179, 7962, 8739, 9512, 10278, 11039, 11793,
    12539, 13279, 14010, 14732, 15446, 16151, 16846, 17530,
    18204, 18868, 19519, 20159, 20787, 21403, 22005, 22594,
    23170, 23731, 24279, 24811, 25329, 25832, 26319, 26790,
    27245, 27683, 28105, 28510, 28898, 29268, 29621, 29956,
    30273, 30571, 30852, 31113, 31356, 31580, 31785, 31971,
    32137, 32285, 32412, 32521, 32609, 32678, 32728, 32757,
    32767,
};

/* XBIOS with five arguments (Devconnect); the same ABI as xbios4. */
static inline NLONG xbios5(NLONG fn, NLONG a0, NLONG a1, NLONG a2, NLONG a3, NLONG a4)
{
    register NLONG x8 __asm__("x8") = fn;
    register NLONG r0 __asm__("x0") = a0;
    register NLONG r1 __asm__("x1") = a1;
    register NLONG r2 __asm__("x2") = a2;
    register NLONG r3 __asm__("x3") = a3;
    register NLONG r4 __asm__("x4") = a4;
    __asm__ volatile("svc #14"
                     : "+r"(r0)
                     : "r"(x8), "r"(r1), "r"(r2), "r"(r3), "r"(r4)
                     : "memory");
    return r0;
}

static void puts_(const char *s)
{
    Cconws(s);
}

/* Print a signed decimal number, no libc. */
static void putdec(long v)
{
    char buf[24];
    int i = 23;
    unsigned long u = v < 0 ? (unsigned long)(-v) : (unsigned long)v;

    buf[i] = 0;
    do {
        buf[--i] = (char)('0' + u % 10);
        u /= 10;
    } while (u != 0);
    if (v < 0)
        buf[--i] = '-';
    Cconws(&buf[i]);
}

/* Print `label`, the value in decimal, and "(expected <want>)". */
static void report(const char *label, long got, long want)
{
    puts_(label);
    putdec(got);
    puts_("  (expected ");
    putdec(want);
    puts_(got == want ? ")\r\n" : ")  MISMATCH\r\n");
}

/* One sample of the 440 Hz sine at phase `phase` (full circle = 2^32). */
static long sine(unsigned int phase)
{
    unsigned int idx = phase >> 24;    /* 0..255 */
    unsigned int k = idx & 63;
    long v;

    switch (idx >> 6) {
    case 0:  v = quarter[k];      break;
    case 1:  v = quarter[64 - k]; break;
    case 2:  v = -quarter[k];     break;
    default: v = -quarter[64 - k]; break;
    }
    return v * AMPLITUDE / 32767;
}

/* Fill `buf` with the tone: big-endian 16-bit `L R` pairs, a channel whose
 * flag is 0 silent. */
static void fill(unsigned char *buf, int left, int right)
{
    /* 440 * 2^32 / 49170 */
    const unsigned int step = 38433711u;
    unsigned int phase = 0;
    long i;

    for (i = 0; i < FRAMES; i++) {
        long s = sine(phase);
        long l = left ? s : 0;
        long r = right ? s : 0;
        unsigned char *p = buf + i * 4;

        p[0] = (unsigned char)((l >> 8) & 0xFF);
        p[1] = (unsigned char)(l & 0xFF);
        p[2] = (unsigned char)((r >> 8) & 0xFF);
        p[3] = (unsigned char)(r & 0xFF);
        phase += step;
    }
}

static void spin(void)
{
    volatile int n;

    for (n = 0; n < 1000; n++)
        ;
}

/* Play `buf` once and wait for the end of the pass. 0 = ended, 1 = stalled. */
static int play(const char *name, unsigned char *buf)
{
    struct sndbufptr bp;
    char *last = 0;
    long polls = 0, stall = 0;
    long rc;

    puts_("\r\n--- ");
    puts_(name);
    puts_(" ---\r\n");
    report("Setbuffer(0, buf, buf + n) = ", xbios4(SETBUFFER, 0, (NLONG)buf, (NLONG)(buf + BYTES), 0), 0);
    report("Buffoper(1)                 = ", xbios1(BUFFOPER, 1), 0);

    for (;;) {
        rc = xbios1(BUFFOPER, -1);
        bp.play = 0;
        bp.record = (char *)1;
        xbios1(BUFFPTR, (NLONG)&bp);
        polls++;
        if ((rc & 1) == 0)
            break;
        if (bp.play != last) {
            last = bp.play;
            stall = 0;
        } else if (++stall > STALL_POLLS) {
            puts_("STALLED: the play pointer did not move; Buffoper(0)\r\n");
            xbios1(BUFFOPER, 0);
            report("pointer offset at the stall = ", (long)(bp.play - (char *)buf), BYTES);
            return 1;
        }
        spin();
    }
    report("Buffoper(-1) after the pass  = ", rc, 0);
    report("Buffptr play offset          = ", (long)(bp.play - (char *)buf), BYTES);
    report("Buffptr record               = ", (long)bp.record, 0);
    puts_("polls: ");
    putdec(polls);
    puts_("\r\n");
    return 0;
}

int main(void)
{
    unsigned char *buf;
    long rc;
    int bad = 0;

    puts_("SNDTONE.NCP -- 440 Hz on the jack: both, left, right (SND05)\r\n");

    rc = xbios0(LOCKSND);
    if (rc == LOCKSND) {
        puts_("Locksnd() = 0x80: no sound backend (XBIOS 0x80-0x8D answer their\r\n"
              "function number, _SND absent) -- nothing to play.\r\n");
        return 1;
    }
    report("Locksnd()                   = ", rc, 1);
    report("Soundcmd(9, -1)             = ", xbios4(SOUNDCMD, 9, -1, 0, 0), 5);
    report("Setmode(STEREO16)           = ", xbios1(SETMODE, 1), 0);
    report("Devconnect(0, 8, 0, 1, 1)   = ", xbios5(DEVCONNECT, 0, 8, 0, 1, 1), 1);

    buf = (unsigned char *)gemdos1(MALLOC, BYTES);
    if ((long)buf <= 0) {
        puts_("Malloc failed\r\n");
        xbios0(UNLOCKSND);
        return 2;
    }

    fill(buf, 1, 1);
    bad |= play("440 Hz, both channels", buf);
    if (!bad) {
        fill(buf, 1, 0);
        bad |= play("440 Hz, LEFT only", buf);
    }
    if (!bad) {
        fill(buf, 0, 1);
        bad |= play("440 Hz, RIGHT only", buf);
    }

    gemdos1(MFREE, (NLONG)buf);
    report("\r\nUnlocksnd()                 = ", xbios0(UNLOCKSND), 0);
    puts_(bad ? "SNDTONE: STALLED\r\n" : "SNDTONE: done\r\n");
    return bad ? 3 : 0;
}
