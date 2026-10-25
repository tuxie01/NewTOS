/* Auditor probe: evnt_multi wake conditions with several AES processes.
 * Build with -DROLE_MAIN / -DROLE_ACCA / -DROLE_ACCB / -DROLE_ACCC.
 * A112 fix run: copied from measurements/measurement-2026-10-17-a112-audit/
 * method/evprobe.c; changes: the ACCD print names its real period, and the
 * POLL loops are bounded so the evnt_timer(0) loop (one tick per call after
 * the fix) ends within the run (POLL_N iterations of the two non-blocking
 * loops, TMR0_N of evnt_timer(0)). */
#include <newtos.h>

typedef struct { short *control, *global, *intin, *intout; long *addrin, *addrout; } AESPB;
static short ctl[6], glb[16], ii[16], io[16];
static long ai[4], ao[4];

static short aes(short op, short nin, short nout, short nai)
{
    AESPB pb; int i;
    ctl[0] = op; ctl[1] = nin; ctl[2] = nout; ctl[3] = nai; ctl[4] = 0;
    for (i = 0; i < 16; i++) io[i] = 0;
    pb.control = ctl; pb.global = glb; pb.intin = ii; pb.intout = io;
    pb.addrin = ai; pb.addrout = ao;
    register long x0 __asm__("x0") = 200;
    register long x1 __asm__("x1") = (long)&pb;
    __asm__ __volatile__("svc\t#2" : "+r"(x0) : "r"(x1) : "memory", "cc");
    return io[0];
}

static void puthex(unsigned v)
{
    char b[8]; int i;
    for (i = 0; i < 4; i++) { unsigned d = (v >> (12 - 4 * i)) & 15; b[i] = d < 10 ? '0' + d : 'A' + d - 10; }
    b[4] = 0; Cconws(b);
}
static void say(const char *a, unsigned v) { Cconws(a); puthex(v); Cconws("\r\n"); }

#define MU_MESAG 0x10
#define MU_TIMER 0x20

static short multi(short flags, unsigned ms, short *msg)
{
    int i;
    for (i = 0; i < 16; i++) ii[i] = 0;
    ii[0] = flags;
    ii[14] = (short)(ms & 0xffff);
    ii[15] = (short)(ms >> 16);
    ai[0] = (long)msg;
    return aes(25, 16, 7, 1);
}

static void write_msg(short to, short *msg)
{
    ii[0] = to; ii[1] = 16; ai[0] = (long)msg;
    aes(12, 2, 1, 1);
}

static short find(const char *name)
{
    ai[0] = (long)name;
    return aes(13, 0, 1, 1);
}

static void timer(unsigned ms)
{
    ii[0] = (short)(ms & 0xffff); ii[1] = (short)(ms >> 16);
    aes(24, 2, 1, 0);
}


static short wcreate(short x, short y, short w, short h)
{ ii[0]=0x000B; ii[1]=x; ii[2]=y; ii[3]=w; ii[4]=h; return aes(100,5,1,0); }
static void wopen(short hd, short x, short y, short w, short h)
{ ii[0]=hd; ii[1]=x; ii[2]=y; ii[3]=w; ii[4]=h; aes(101,5,1,0); }
static void wclose(short hd) { ii[0]=hd; aes(102,1,1,0); }
static void wdelete(short hd) { ii[0]=hd; aes(103,1,1,0); }

static void pmsg(const char *tag, short w, short *msg)
{ Cconws(tag); Cconws(" ret="); puthex(w); Cconws(" type="); puthex(msg[0]); Cconws(" w3="); puthex(msg[3]); Cconws("\r\n"); }

int main(void)
{
    short msg[8]; short id; int i; unsigned n = 0;
    (void)i; (void)n; (void)find; (void)write_msg; (void)timer; (void)multi;
    id = aes(10, 0, 1, 0);
#ifndef ROLE_MAIN
    ii[0] = id;
    ai[0] = (long)"  Probe";
    aes(35, 1, 1, 1);
#endif
#if defined(ROLE_ACCA)
    for (;;) {
        short w = multi(MU_TIMER, 250, msg);
        n++;
        Cconws("[ACCA] evnt_multi(MU_TIMER,250) ret="); puthex(w); Cconws(" n="); puthex(n); Cconws("\r\n");
    }
#elif defined(ROLE_ACCB) || defined(ROLE_ACCC)
    timer(5000);
#ifdef ROLE_ACCB
    short hd = wcreate(40, 60, 200, 120); wopen(hd, 40, 60, 200, 120);
    say("[ACCB] window open handle=", hd);
#else
    short hd = wcreate(340, 60, 200, 120); wopen(hd, 340, 60, 200, 120);
    say("[ACCC] window open handle=", hd);
#endif
    for (;;) {
        short w;
        for (i = 0; i < 8; i++) msg[i] = 0;
#ifdef ROLE_ACCB
        w = multi(MU_MESAG, 0, msg);
        pmsg("[ACCB] evnt_multi(MU_MESAG)", w, msg);
#else
        ai[0] = (long)msg; w = aes(23, 0, 1, 1);
        pmsg("[ACCC] evnt_mesag", w, msg);
#endif
    }
#elif defined(ROLE_ACCD)
    for (;;) {
#ifdef ACCD_FAST
        timer(1000);
#else
        timer(4000);
#endif
#ifdef ACCD_FAST
        say("[ACCD] evnt_timer(1000) returned k=", ++n);
#else
        say("[ACCD] evnt_timer(4000) returned k=", ++n);
#endif
    }
#elif defined(ROLE_POLL)
#define POLL_N 600000u
#define TMR0_N 250u
    say("[POLL] apid=", id);
    timer(2500);
    Cconws("[POLL] start evnt_multi(MU_TIMER,0) loop\r\n");
    for (n = 1; n <= POLL_N; n++) { multi(MU_TIMER, 0, msg); if (n % 100000 == 0) say("[POLL] evnt_multi(T,0) iter/100000 ", n / 100000); }
    Cconws("[POLL] start appl_yield loop\r\n");
    for (n = 1; n <= POLL_N; n++) { aes(17, 0, 1, 0); if (n % 100000 == 0) say("[POLL] appl_yield iter/100000 ", n / 100000); }
    Cconws("[POLL] start evnt_timer(0) loop\r\n");
    for (n = 1; n <= TMR0_N; n++) { timer(0); if (n % 50 == 0) say("[POLL] evnt_timer(0) iter ", n); }
    Cconws("[POLL] done, heartbeat via evnt_timer(1000)\r\n");
    for (n = 1;; n++) { timer(1000); say("[POLL] heartbeat ", n); }
#else /* MAIN */
    say("[MAIN] apid=", id);
#ifdef MAIN_LATE
    timer(9000);
    Cconws("[MAIN] evnt_timer(9000) returned\r\n");
#else
    timer(3000);
    Cconws("[MAIN] evnt_timer(3000) returned\r\n");
#endif
    {
        short hd = wcreate(0, 20, 640, 460);
        wopen(hd, 0, 20, 640, 460);
        say("[MAIN] cover window opened handle=", hd);
        wclose(hd); wdelete(hd);
        Cconws("[MAIN] cover window closed+deleted\r\n");
    }
#ifdef MAIN_LATE
    for (n = 1; n <= 30; n++) { timer(1000); say("[MAIN] heartbeat ", n); }
#endif
    for (n = 1; n <= 30; n++) {
        short w;
        for (i = 0; i < 8; i++) msg[i] = 0;
        w = multi(MU_MESAG | MU_TIMER, 1000, msg);
        Cconws("[MAIN] loop "); puthex(n); pmsg(" evnt_multi(MESAG|TIMER,1000)", w, msg);
    }
    Cconws("[MAIN] loop finished\r\n");
    for (;;) timer(5000);
#endif
    return 0;
}
