/* xgscr_nt.c - the NewTOS side of xgscr.c: the GEM calls (gem.h clashes with XGDOS's vdi.h), the VDI
 * trap XGDOS passes calls on to, and harness answers for what stage 1 does not exercise. */
#include <gem.h>
#include <osbind.h>

int xgscr_main(void);
long _stksize = 512 * 1024;   /* room for the stack paint of xgscr.c */

/* upstream: atari.S rom_vdi, the previous trap #2 handler. Here: the kernel's VDI, the same trap gemlib uses. */
void rom_vdi(void *pb)
{
    register long x0 __asm__("x0") = 115;
    register long x1 __asm__("x1") = (long)pb;
    __asm__ __volatile__("svc #2" : "+r"(x0) : "r"(x1) : "memory", "cc");
    (void)x0;
}

/* external .SYS drivers are m68k code: not part of stage 1 (upstream drv.c needs atari.S drv_jump). */
int drv_open(void *pb, const char *drv) { (void)drv; ((short **)pb)[0][6] = 0; return 0; }
void drv_call(void *w, void *pb) { (void)w; (void)pb; }

int nt_phys_handle(void)
{
    short cw, ch, bw, bh;
    return graf_handle(&cw, &ch, &bw, &bh);
}

/* leave the picture on the screen for the screendumps; the marker file tells the host script */
void nt_finish_screen(void)
{
    long h = Fcreate("C:\\XGSCR.RDY", 0);
    if (h >= 0) Fclose((short)h);
    evnt_timer(25000);
}

int main(void)
{
    int rc;
    appl_init();
    rc = xgscr_main();
    appl_exit();
    return rc;
}
