/* vecp.c - PORT12 Q5 probe, the GEM side, run by #Z autorun:
 * what does a vector 34 set with Setexc do; what is left of it after Ptermres; rights; call costs. */
#include <gem.h>
#include "vec-common.h"
static volatile long my_count;
static void my_trap2(void) { my_count++; }
static long ret_noop(void) { return 1; }
static long hz(void) { return (long)*(volatile unsigned int *)0x4BAL; }
static long sv_hdr, sv_beg, sv_ver, sv_runp, sv_run;
static long find_prun(void)
{
    /* upstream src/main.c:43-50 find_prun_s, read in super state */
    unsigned int h = *(volatile unsigned int *)0x4F2L;
    sv_hdr = h;
    sv_beg = *(volatile unsigned int *)(unsigned long)(h + 8);
    sv_ver = *(volatile unsigned short *)(unsigned long)(sv_beg + 2);
    sv_runp = *(volatile unsigned int *)(unsigned long)(sv_beg + 0x28);
    sv_run = *(volatile unsigned int *)(unsigned long)sv_runp;
    return 7;
}
static long gem2(long sel, void *pb)
{
    register long x0 __asm__("x0") = sel;
    register long x1 __asm__("x1") = (long)pb;
    __asm__ __volatile__("svc #2" : "+r"(x0) : "r"(x1) : "memory", "cc");
    return x0;
}
int main(void)
{
    short wi[11], wo[57], h, i, cw, ch, bw, bh, attr[10];
    long r, v = 0, t0, t1, n, k, boot34;
    short ctl[16], iin[4], iout[8], pts[8];
    VDIPB pb;
    struct shared *s;
    strcpy(lf_name, "C:\\VECP.LOG");
    appl_init();
    h = graf_handle(&cw, &ch, &bw, &bh);
    for (i = 0; i < 10; i++) wi[i] = 1;
    wi[10] = 2;
    v_opnvwk(wi, &h, wo);
    boot34 = SETX(34, -1L);
    lg("1 Setexc(34,-1) -> %#lx  (JUST_RTS stub expected); vq_gdos trap -2 -> %ld, trap -1 -> %ld\n", SETX(34, -1L), gem2(-2, 0), gem2(-1, 0));
    lg("2 handler at %p: Setexc(34,h) -> %#lx, query %#lx, page-0 $88 %#x\n", (void *)my_trap2, SETX(34, (long)my_trap2), SETX(34, -1L), *(volatile unsigned int *)0x88L);
    vqt_attributes(h, attr);
    v_gtext(h, 8, 40, "VECP: VDI through svc #2");
    (void)gem2(-2, 0);
    graf_handle(&cw, &ch, &bw, &bh);
    lg("3 after VDI (svc #2, 115), vq_gdos (-2), AES (200): handler called %ld times\n", my_count);
    lg("4 Setexc(34,old) -> %#lx, query %#lx\n", SETX(34, boot34), SETX(34, -1L));
    /* cookie jar + basepage variables as XGDOS reads them */
    r = Supexec(find_prun);
    lg("5 Supexec -> %ld; $4F2 hdr %#lx os_beg %#lx os_version %#lx os_run cell %#lx holds %#lx; _base %p\n", r, sv_hdr, sv_beg, sv_ver, sv_runp, sv_run, (void *)_base);
    lg("6 cookie MiNT -> %ld, XGDS -> %ld; Sversion %#lx; Syield %ld; Pgetpid %ld; Pkill(self,0) %ld; Super(1) %ld\n",
       Ssystem(S_GETCOOKIE, 0x4D694E54L, 0L), Ssystem(S_GETCOOKIE, 0x58474453L, 0L), Sversion(), Syield(), Pgetpid(), Pkill(Pgetpid(), 0), Super(1L));
    /* Ptermres like XGDOS.PRG, then a sibling, then this process */
    r = Pexec(0, "C:\\VECTSR.NAP", "", 0L);
    lg("7 Pexec VECTSR -> %ld; Setexc(34,-1) -> %#lx\n", r, SETX(34, -1L));
    r = Pexec(0, "C:\\VECCLI.NAP", "", 0L);
    lg("8 Pexec VECCLI -> %ld (0xFFFF = fault)\n", r);
    r = Ssystem(S_GETCOOKIE, COOKIE_XGTS, (long)&v);
    lg("9 cookie XGTS -> %ld value %#lx\n", r, v);
    if (v) {
        s = (struct shared *)v;
        lg("10 parent: TSR counter %ld\n", *s->count);
        r = s->fn();
        vqt_attributes(h, attr);
        (void)gem2(-2, 0);
        lg("11 parent called TSR code -> %#lx; after VDI and vq_gdos with the resident vector 34: counter %ld\n", r, *s->count);
    }
    /* call costs (QEMU TCG, 200 Hz clock, not a Pi figure) */
    n = 20000;
    t0 = Supexec(hz); for (k = 0; k < n; k++) r = Supexec(ret_noop); t1 = Supexec(hz);
    lg("12 %ld Supexec(noop), EL0 upcall and back: %ld ticks of 5 ms\n", n, t1 - t0);
    n = 200000;
    memset(ctl, 0, sizeof ctl);
    pb.control = ctl; pb.intin = iin; pb.ptsin = pts; pb.intout = iout; pb.ptsout = pts;
    t0 = Supexec(hz);
    for (k = 0; k < n; k++) { ctl[0] = 38; ctl[1] = 0; ctl[3] = 0; ctl[5] = 0; ctl[6] = h; gem2(115, &pb); }
    t1 = Supexec(hz);
    lg("13 %ld VDI vqt_attributes via svc #2: %ld ticks of 5 ms\n", n, t1 - t0);
    t0 = Supexec(hz);
    for (k = 0; k < n; k++) { Sversion(); }
    t1 = Supexec(hz);
    lg("14 %ld Sversion (plain GEMDOS trap): %ld ticks of 5 ms\n", n, t1 - t0);
    /* side finding of Q3a: a name longer than 8.3 */
    r = Fcreate("C:\\OUT_SCREEN.PGM", 0);
    lg("15 Fcreate(C:\\OUT_SCREEN.PGM) -> %ld\n", r);
    if (r >= 0) Fclose((short)r);
    v_clsvwk(h);
    appl_exit();
    return 0;
}
