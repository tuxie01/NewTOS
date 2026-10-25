/*
 * a117acc.c -- TASK/A117 probe desk accessory (A117ACC.NAC).
 *
 *   appl_init
 *   menu_register(id, "  Beta acc")
 *   loop: evnt_multi(MU_MESAG) -- the accessory idles in a MESSAGE wait of
 *         evnt_multi (not evnt_mesag), the wait the NewTOS inline Control
 *         Manager runs inside; every message it receives is logged.
 *
 * Progress lines go to the console (Cconws -> serial), prefix "ACCBE:".
 * A line "ACCBE: msg 10 ..." (MN_SELECTED) means a menu selection was
 * delivered to the accessory, which does not own the menu bar.
 * Build: sdk/build.sh <this file> <out>/A117ACC.NAC
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

#define MU_MESAG 0x0010

static short msg[8];
static char title[] = "  Beta acc";

int main(void)
{
    short apid, menu_id, ev;
    int i;

    apid = aes_call(10, 0, 1, 0); /* appl_init */
    aes_intin[0] = apid;
    aes_addrin[0] = (long)title;
    menu_id = aes_call(35, 1, 1, 1); /* menu_register */
    Cconws("ACCBE: up apid "); Cconws(dec(apid));
    Cconws(" menu_id "); Cconws(dec(menu_id)); Cconws("\r\n");

    for (;;) {
        for (i = 0; i < 16; i++) aes_intin[i] = 0;
        for (i = 0; i < 8; i++) msg[i] = 0;
        aes_intin[0] = MU_MESAG;
        aes_addrin[0] = (long)msg;
        ev = aes_call(25, 16, 7, 1); /* evnt_multi(MU_MESAG) */
        Cconws("ACCBE: ev "); Cconws(dec(ev));
        for (i = 0; i < 8; i++) { Cconws(i ? " " : " msg "); Cconws(dec(msg[i])); }
        Cconws("\r\n");
    }
}
