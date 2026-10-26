/*
 * a118acc.c -- TASK/A118 probe desk accessory (A118DL.NAC): a file selector opened by a desk accessory.
 * Adapted from measurements/measurement-2026-10-25-a115/method/a117/a117acc.c (same AES binding).
 *
 *   appl_init; menu_register(id, "  A118 probe")
 *   loop: evnt_multi(MU_MESAG); on AC_OPEN for our menu id: fsel_input("C:\*.*", "") and log the result.
 *
 * EmuTOS runs the selector's dialog through fm_do, whose fm_own(TRUE) makes the accessory the mouse owner
 * (aes/gemfslib.c:718, aes/gemfmlib.c:93): a press while the selector is up completes no other process's
 * parked MU_BUTTON. Progress lines go to the console (Cconws -> serial), prefix "A118DL:".
 * Build: sdk/build.sh <this file> <out>/A118DL.NAC
 */
#include <newtos.h>

typedef struct {
    short *control;
    short *global;
    short *intin;
    short *intout;
    long  *addrin;
    long  *addrout;
} AESPB;

static short aes_control[6];
static short aes_global[16];
static short aes_intin[16];
static short aes_intout[16];
static long  aes_addrin[4];
static long  aes_addrout[4];

static void aes_trap(void *pb)
{
    register long x0 __asm__("x0") = 200;
    register long x1 __asm__("x1") = (long)pb;
    __asm__ __volatile__("svc\t#2" : "+r"(x0) : "r"(x1) : "memory", "cc");
    (void)x0;
}

static short aes_call(short opcode, short nin, short nout, short naddrin)
{
    AESPB pb;
    int i;
    aes_control[0] = opcode;
    aes_control[1] = nin;
    aes_control[2] = nout;
    aes_control[3] = naddrin;
    aes_control[4] = 0;
    for (i = 0; i < 16; i++)
        aes_intout[i] = 0;
    pb.control = aes_control;
    pb.global  = aes_global;
    pb.intin   = aes_intin;
    pb.intout  = aes_intout;
    pb.addrin  = aes_addrin;
    pb.addrout = aes_addrout;
    aes_trap(&pb);
    return aes_intout[0];
}

static char numbuf[16];
static const char *dec(long v)
{
    char t[16]; int n = 0, i = 0;
    unsigned long u = v < 0 ? (unsigned long)-v : (unsigned long)v;
    if (v < 0) numbuf[i++] = '-';
    do { t[n++] = (char)('0' + u % 10); u /= 10; } while (u);
    while (n) numbuf[i++] = t[--n];
    numbuf[i] = 0;
    return numbuf;
}


#define HOLD_MS 0
#define MU_MESAG 0x0010
#define AC_OPEN  40
#define AC_CLOSE 41

static void log(const char *a, long v)
{
    Cconws("A118DL: "); Cconws(a); Cconws(" "); Cconws(dec(v)); Cconws("\r\n");
}

static void wind_update(short m) { aes_intin[0] = m; aes_call(107, 1, 1, 0); }
static void timer_ms(long ms) { aes_intin[0] = (short)(ms & 0xFFFF); aes_intin[1] = (short)((ms >> 16) & 0xFFFF); aes_call(24, 2, 1, 0); }

static short msg[8];
static char title[] = "  A118 DL";

int main(void)
{
    short apid, menu_id, ev;
    int i;

    apid = aes_call(10, 0, 1, 0); /* appl_init */
    aes_intin[0] = apid;
    aes_addrin[0] = (long)title;
    menu_id = aes_call(35, 1, 1, 1); /* menu_register */
    log("up apid", apid);
    /* scenario 1: hold the window-tree lock for 6 s right after start */
    if (HOLD_MS) {
        log("BEG_UPDATE (startup hold)", 0);
        wind_update(1);
        timer_ms(HOLD_MS);
        wind_update(0);
        log("END_UPDATE (startup hold)", 0);
    }

    for (;;) {
        for (i = 0; i < 16; i++) aes_intin[i] = 0;
        for (i = 0; i < 8; i++) msg[i] = 0;
        aes_intin[0] = MU_MESAG;
        aes_addrin[0] = (long)msg;
        ev = aes_call(25, 16, 7, 1); /* evnt_multi(MU_MESAG) */
        if (ev & MU_MESAG) {
            if (msg[0] == AC_OPEN && msg[4] == menu_id) log("AC_OPEN", 0);
            if (msg[0] == AC_CLOSE && msg[3] == menu_id) {
                /* scenario 2: BEG_UPDATE inside the AC_CLOSE answer (ap_exit holds the lock) */
                log("AC_CLOSE seen, BEG_UPDATE", 0);
                wind_update(1);
                timer_ms(500);
                wind_update(0);
                log("AC_CLOSE answered, END_UPDATE", 0);
            }
        }
    }
}
