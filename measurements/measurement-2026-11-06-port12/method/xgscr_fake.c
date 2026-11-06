/* xgscr_fake.c - the Mac side of xgscr.c: a fake screen VDI (upstream host/hostvdi.c cut to what xgscr.c
 * passes on, 640x480, device parameters as the NewTOS VDI answers them in QEMU raspi4b). */
#include <stdio.h>
#include <string.h>
#include "vdi.h"
#ifndef FAKE_PW
#define FAKE_PW 278
#endif
#ifndef FAKE_PH
#define FAKE_PH 278
#endif
#ifndef FAKE_COL
#define FAKE_COL 256
#endif
#define FW 640
#define FH 480
static unsigned char fb[FH][FW];
static int nexth = 1;
int xgscr_main(void);
int nt_phys_handle(void) { return 1; }
void nt_finish_screen(void)
{
    FILE *f = fopen("XGSCR.PGM", "wb");
    int x, y;
    fprintf(f, "P5 %d %d 255\n", FW, FH);
    for (y = 0; y < FH; y++) for (x = 0; x < FW; x++) fputc(fb[y][x] == 1 ? 0 : 255, f);
    fclose(f);
}
int drv_open(VDIPB *pb, const char *drv) { (void)drv; pb->contrl[6] = 0; return 0; }
void drv_call(Wk *w, VDIPB *pb) { (void)w; (void)pb; }
void rom_vdi(VDIPB *pb)
{
    s16 *c = pb->contrl;
    int i;
    switch (c[0]) {
    case 1: case 100: case 102:
        for (i = 0; i < 45; i++) pb->intout[i] = 0;
        pb->intout[0] = FW - 1; pb->intout[1] = FH - 1; pb->intout[3] = FAKE_PW; pb->intout[4] = FAKE_PH;
        pb->intout[10] = 1; pb->intout[13] = FAKE_COL;
        if (c[0] != 102) c[6] = (s16)++nexth;
        break;
    case 38: pb->intout[0] = 1; pb->intout[1] = 1; pb->intout[2] = 0; pb->intout[3] = 0; pb->intout[4] = 0; pb->intout[5] = 1; break;
    case 121: {
        MFDB *s = (MFDB *)GETPTR(c, XG_CP(0));
        s16 *p = pb->ptsin;
        int mode = pb->intin[0], x, y;
        int w = p[2] - p[0] + 1, h = p[3] - p[1] + 1;
        for (y = 0; y < h; y++) for (x = 0; x < w; x++) {
            /* like tos-api/src/vdi/raster.rs: a form is native WORDs, bit 15 = leftmost */
            const u16 *row = (const u16 *)s->fd_addr + (y + p[1]) * s->fd_wdwidth;
            int bit = (row[(x + p[0]) >> 4] >> (15 - ((x + p[0]) & 15))) & 1;
            int X = p[4] + x, Y = p[5] + y;
            if (X < 0 || Y < 0 || X >= FW || Y >= FH) continue;
            if (mode == 1) fb[Y][X] = bit ? 1 : 2;
            else if (mode == 2) { if (bit) fb[Y][X] = 1; }
            else if (mode == 3) { if (bit) fb[Y][X] ^= 1; }
        }
        break;
    }
    /* upstream host/hostvdi.c answers, unchanged */
    case 12: pb->ptsout[0] = 7; pb->ptsout[1] = 13; pb->ptsout[2] = 8; pb->ptsout[3] = 16; break;
    case 107: pb->intout[0] = pb->intin[0]; pb->ptsout[0] = 8; pb->ptsout[1] = 13; pb->ptsout[2] = 8; pb->ptsout[3] = 16; break;
    case 21: pb->intout[0] = 1; break;
    case 22: case 32: pb->intout[0] = pb->intin[0]; break;
    case 39: pb->intout[0] = pb->intin[0]; pb->intout[1] = pb->intin[1]; break;
    case 130: pb->intout[0] = 1; memset(pb->intout + 1, 0, 64); { const char *n = "6x6 system font"; for (i = 0; n[i]; i++) pb->intout[1 + i] = n[i]; } break;
    default: break;
    }
}
int main(void) { return xgscr_main(); }
