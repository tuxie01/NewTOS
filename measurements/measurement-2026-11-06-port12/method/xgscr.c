/* xgscr.c - PORT12 stage 1, question 3: the XGDOS portable modules as a library inside one program,
 * no kernel change. VDI calls go to xg_vdi() directly; what XGDOS passes on (rom_vdi) goes through the
 * real VDI trap on NewTOS (svc #2) and to a fake framebuffer on the Mac (xgscr_fake.c).
 * Same source, same patched XGDOS, both sides: LOG, PDF and text pixels are compared. */
#include <stdio.h>
#include <string.h>
#include "vdi.h"
#include "heap.h"
#include "xos.h"
#include "xstd.h"
#include "gcache.h"

static s16 contrl[16], intin[1024], ptsin[1024], intout[512], ptsout[64];
static char lb[256];
static s32 logh = -1;

void nt_hold_ms(long ms);
void nt_mark_ready(void);
int  nt_phys_handle(void);
void nt_finish_screen(void);

static void lg(const char *s) { if (logh >= 0) xos_write(logh, (s32)strlen(s), s); }
#define LOG(...) do { snprintf(lb, sizeof lb, __VA_ARGS__); lg(lb); } while (0)

static void vdi(int op, int sub, int h, int np, int ni)
{
    VDIPB pb = { contrl, intin, ptsin, intout, ptsout };
    memset(contrl, 0, sizeof contrl);
    contrl[0] = (s16)op; contrl[1] = (s16)np; contrl[3] = (s16)ni; contrl[5] = (s16)sub; contrl[6] = (s16)h;
    if (xg_vdi_need(&pb)) xg_vdi(&pb); else rom_vdi(&pb);
}
static int str(const char *s, s16 *d) { int n = 0; while (*s) d[n++] = (u8)*s++; return n; }
static void gtext(int h, int x, int y, const char *s) { int n = str(s, intin); ptsin[0] = x; ptsin[1] = y; vdi(8, 0, h, 1, n); }
static void i1(int op, int h, int v) { intin[0] = v; vdi(op, 0, h, 0, 1); }

static void cpyfm_img(int op, int h, int dx, int dy, int mode, int ni)
{
    static u16 img[32 * 2];
    MFDB src, dst;
    VDIPB pb = { contrl, intin, ptsin, intout, ptsout };
    int y;
    for (y = 0; y < 32; y++) { img[y * 2] = (u16)(y & 1 ? 0xAAAA : 0x5555); img[y * 2 + 1] = (u16)(0xffff >> (y / 2)); }
    /* an application form on NewTOS: native WORDs (no byte swap as in upstream test_vdi.c) */
    src.fd_addr = img; src.fd_w = 32; src.fd_h = 32; src.fd_wdwidth = 2; src.fd_stand = 0; src.fd_nplanes = 1;
    src.fd_r1 = src.fd_r2 = src.fd_r3 = 0;
    memset(&dst, 0, sizeof dst);
    memset(contrl, 0, sizeof contrl);
    ptsin[0] = 0; ptsin[1] = 0; ptsin[2] = 31; ptsin[3] = 31; ptsin[4] = dx; ptsin[5] = dy; ptsin[6] = dx + 31; ptsin[7] = dy + 31;
    intin[0] = (s16)mode; intin[1] = 1; intin[2] = 0;
    contrl[0] = (s16)op; contrl[1] = 4; contrl[3] = (s16)ni; contrl[6] = (s16)h;
    SETPTR(contrl, XG_CP(0), &src); SETPTR(contrl, XG_CP(1), &dst);
    if (xg_vdi_need(&pb)) xg_vdi(&pb); else rom_vdi(&pb);
}

static void screen_part(int phys)
{
    int h, i, nf, ids[16], y;
    static const int sizes[] = { 8, 10, 12, 14, 18, 24 };
    for (i = 0; i < 10; i++) intin[i] = 1;
    intin[10] = 2;
    vdi(100, 0, phys, 0, 11); h = contrl[6];
    LOG("v_opnvwk phys %d -> %d: %dx%d pixel %dx%d um fonts %d colours %d\n", phys, h > 0, intout[0] + 1, intout[1] + 1, intout[3], intout[4], intout[10], intout[13]);
    vdi(123, 0, h, 0, 0);                       /* v_hide_c */
    vdi(3, 0, h, 0, 0);                         /* v_clrwk */
    i1(32, h, 2);                               /* transparent */
    vdi(119, 0, h, 0, 0); nf = intout[0];
    LOG("vst_load_fonts -> %d\n", nf);
    for (i = 1; i <= nf + 1 && i < 16; i++) {
        char nm[33]; int k;
        intin[0] = i; vdi(130, 0, h, 0, 1);
        for (k = 0; k < 32; k++) nm[k] = (char)intout[1 + k];
        nm[32] = 0; ids[i] = intout[0];
        LOG("vqt_name %d: id %d '%s' outline=%d\n", i, intout[0], nm, intout[33]);
    }
    i1(21, h, ids[2]); LOG("vst_font -> %d\n", intout[0]);
    intin[0] = 0; intin[1] = 5; vdi(39, 0, h, 0, 2);
    y = 4;
    for (i = 0; i < 6; i++) {
        i1(107, h, sizes[i]);
        LOG("vst_point %d -> %d cw %d ch %d cellw %d cellh %d\n", sizes[i], intout[0], ptsout[0], ptsout[1], ptsout[2], ptsout[3]);
        gtext(h, 4, y, "XGDOS Speedo: Hamburgefonstiv \x84\x94\x81\x9e \x82\x8a\x87");
        y += ptsout[3] + 2;
    }
    intin[0] = 0; intin[1] = 0; vdi(39, 0, h, 0, 2);
    i1(21, h, ids[3]); i1(107, h, 18);
    i1(106, h, 0);  gtext(h, 4, 240, "Italic");
    i1(106, h, 1);  gtext(h, 90, 240, "Bold");
    i1(106, h, 8);  gtext(h, 170, 240, "Under");
    i1(106, h, 16); gtext(h, 260, 240, "Outline");
    i1(106, h, 2);  gtext(h, 380, 240, "Light");
    i1(106, h, 0);
    i1(21, h, ids[6]); i1(107, h, 14); gtext(h, 4, 270, "Courier 10 Pitch 14 pt 0123456789");
    i1(21, h, ids[4]); i1(107, h, 18);
    intin[0] = str("Extent test", intin); vdi(116, 0, h, 0, intin[0]);
    LOG("vqt_extent: %d,%d %d,%d %d,%d %d,%d\n", ptsout[0], ptsout[1], ptsout[2], ptsout[3], ptsout[4], ptsout[5], ptsout[6], ptsout[7]);
    i1(13, h, 900); gtext(h, 620, 300, "Rotated 90");
    i1(13, h, 450); gtext(h, 480, 400, "Rotated 45");
    i1(13, h, 0);
    /* replace mode: the background cell goes through vrt_cpyfm mode 1 */
    i1(32, h, 1); i1(107, h, 14); gtext(h, 4, 300, "Replace mode background"); i1(32, h, 2);
    /* clipping */
    intin[0] = 1; ptsin[0] = 0; ptsin[1] = 310; ptsin[2] = 200; ptsin[3] = 370; vdi(129, 0, h, 2, 1);
    i1(107, h, 36); gtext(h, 4, 355, "Clipped at x=200");
    intin[0] = 0; vdi(129, 0, h, 0, 1);
    /* pair kerning off, on */
    i1(107, h, 18);
    intin[0] = 0; intin[1] = 0; vdi(237, 0, h, 0, 2); gtext(h, 240, 330, "AVAWAY To");
    intin[0] = 0; intin[1] = 1; vdi(237, 0, h, 0, 2); LOG("vst_kern pairs=%d\n", intout[1]); gtext(h, 240, 352, "AVAWAY To");
    /* the memory form path of the VDI itself, unrelated to fonts */
    cpyfm_img(121, h, 240, 370, 2, 3);
    /* system font: XGDOS must pass it on to the VDI (fake side draws nothing: below the compared area) */
    i1(21, h, 1); i1(32, h, 1); i1(107, h, 10);
    gtext(h, 4, 470, "System font through the VDI (pass-through) \x84\x94\x81");
    LOG("heap free %d largest %d\n", heap_free_total(), heap_largest());
    nt_finish_screen();
    vdi(101, 0, h, 0, 0);
}

static void print_part(void)
{
    int h, i, nf, ids[5];
    for (i = 0; i < 11; i++) intin[i] = 1;
    intin[0] = 21; intin[10] = 2;
    vdi(1, 0, 0, 0, 11); h = contrl[6];
    LOG("printer handle %d: %dx%d px, pixel %dx%d um, colours %d\n", h, intout[0] + 1, intout[1] + 1, intout[3], intout[4], intout[13]);
    vdi(119, 0, h, 0, 0); nf = intout[0];
    LOG("printer fonts +%d\n", nf);
    intin[0] = 2; vdi(130, 0, h, 0, 1); ids[2] = intout[0];
    intin[0] = 3; vdi(130, 0, h, 0, 1); ids[3] = intout[0];
    i1(21, h, ids[2]); i1(107, h, 36); gtext(h, 300, 400, "XGDOS prints to PDF on NewTOS");
    i1(107, h, 12);
    for (i = 0; i < 6; i++) {
        char b[80];
        snprintf(b, sizeof b, "Line %d: The quick brown fox jumps over the lazy dog. \x84\x94\x81\x9e", i + 1);
        gtext(h, 300, 600 + i * 120, b);
    }
    i1(21, h, ids[3]); i1(107, h, 24); i1(106, h, 8); gtext(h, 300, 1500, "Italic underlined, 24 pt"); i1(106, h, 0);
    i1(21, h, 1); i1(107, h, 12); gtext(h, 300, 1900, "System font -> Courier (\x84\x94\x81)");
    i1(25, h, 2); i1(23, h, 1);
    ptsin[0] = 300; ptsin[1] = 2100; ptsin[2] = 1300; ptsin[3] = 2500; vdi(11, 1, h, 2, 0);
    i1(25, h, 4); ptsin[0] = 2000; ptsin[1] = 2300; ptsin[4] = 300; vdi(11, 4, h, 3, 0);
    ptsin[0] = 300; ptsin[1] = 2700; ptsin[2] = 4600; ptsin[3] = 2700; ptsin[4] = 4600; ptsin[5] = 3200; vdi(6, 0, h, 3, 0);
    i1(21, h, ids[2]); i1(107, h, 20); i1(13, h, 300); gtext(h, 1500, 6800, "Rotated 30 degrees"); i1(13, h, 0);
    cpyfm_img(109, h, 4200, 4400, 3, 1);        /* image through vro_cpyfm: pointers from CONTRL */
    vdi(4, 0, h, 0, 0); vdi(3, 0, h, 0, 0);     /* page break */
    i1(21, h, ids[2]); i1(107, h, 48); intin[0] = 1; intin[1] = 5; vdi(39, 0, h, 0, 2); gtext(h, 2480, 300, "Page two, centred");
    vdi(2, 0, h, 0, 0);
    LOG("heap free %d largest %d\n", heap_free_total(), heap_largest());
}

/* stack bound for upstream SLOT_STACK (src/main.c:17, 8192 on m68k): paint, run, scan. */
#define PAINT 180000
static char *paint_low;
static __attribute__((noinline)) void paint_stack(void)
{
    volatile char a[PAINT];
    int i;
    for (i = 0; i < PAINT; i++) a[i] = 0x5a;
    paint_low = (char *)a;
}
static __attribute__((noinline)) long stack_used(void)
{
    int i = 0;
    while (i < PAINT && paint_low[i] == 0x5a) i++;
    return PAINT - i;
}

/* a workstation XGDOS first sees at a GDOS call (lazy_wk): the caller's vst_load_fonts without its v_opnvwk */
static void lazy_part(int phys)
{
    int i, h;
    VDIPB pb = { contrl, intin, ptsin, intout, ptsout };
    for (i = 0; i < 10; i++) intin[i] = 1;
    intin[10] = 2;
    memset(contrl, 0, sizeof contrl);
    contrl[0] = 100; contrl[3] = 11; contrl[6] = (s16)phys;
    rom_vdi(&pb);
    h = contrl[6];
    vdi(119, 0, h, 0, 0);
    LOG("lazy workstation: vst_load_fonts -> %d, tracked %d\n", intout[0], wk_find(h) != NULL);
    vdi(101, 0, h, 0, 0);
}

int xgscr_main(void)
{
    static char assign[4096];
    s32 fh, n;
    void *hp;
    logh = xos_create("C:\\XGSCR.LOG");
    cfg_defaults(&xg_cfg, 2);
    fh = xos_open("C:\\ASSIGN.SYS", 0);
    n = fh >= 0 ? xos_read(fh, sizeof assign, assign) : -1;
    xos_close(fh);
    LOG("assign %d bytes\n", n > 0);
    if (n > 0) cfg_parse(&xg_cfg, assign, n);
    hp = xos_alloc((s32)xg_cfg.heap_kb * 1024);
    heap_init(hp, (s32)xg_cfg.heap_kb * 1024);
    if (xg_cfg.cache_kb * 1024L > xg_cfg.heap_kb * 512L) xg_cfg.cache_kb = (s16)(xg_cfg.heap_kb / 2);
    xg_scratch_size = 24 * 1024; xg_scratch = xos_alloc(xg_scratch_size);
    LOG("sizeof void* %d XFont %d Glyph %d Wk %d MFDB %d VDIPB %d\n", (int)sizeof(void *), (int)sizeof(XFont), (int)sizeof(struct Glyph), (int)sizeof(Wk), (int)sizeof(MFDB), (int)sizeof(VDIPB));
    LOG("fonts: %d\n", fm_init());
    fnt_load_all();
    paint_stack();
    screen_part(nt_phys_handle());
    print_part();
    lazy_part(nt_phys_handle());
    xos_close(logh);
    {
        s32 sh = xos_create("C:\\XGSCR.STK");
        snprintf(lb, sizeof lb, "stack used by the whole driver, screen and PDF: %ld bytes\n", stack_used());
        xos_write(sh, (s32)strlen(lb), lb);
        xos_close(sh);
    }
    return 0;
}
