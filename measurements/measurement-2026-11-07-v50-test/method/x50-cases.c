/* ---- x50 (V50 tester, extra case, not in vdprobe.c): text with every effect in all four write modes at an odd x,
 * in two heights (the plain glyph path and the scaled/effect cell path), rotated text, v_justified; and vrt_cpyfm
 * from a 1-plane memory form to the screen in all four modes at odd and even destination x and source columns
 * 0,1,7,15,16,17,31, plus the cases the NEON adapter refuses (source column/row negative, source shorter than the
 * block, clip window).  Window 220x70 / 220x50 over a two-colour backdrop. */
static void x50_backdrop(int h)
{
	i1(23, 1); i1(25, 3); bar(20, 20, 239, 20 + h - 1);
	i1(25, 6); recfl(70, 24, 150, 20 + h - 5);
}

static void x50_text(void)
{
	static const int16_t eff[11] = { 0, 1, 2, 4, 8, 16, 1 | 8, 4 | 8, 16 | 1, 31, 2 | 4 };
	static const int hts[2] = { 0, 26 };
	static const char *mn[4] = { "rp", "tr", "xr", "rv" };
	char nm[9];
	int e, hh, m, i;

	for (hh = 0; hh < 2; hh++)
		for (e = 0; e < 11; e++)
			for (m = 0; m < 4; m++) {
				nm[0] = 'Y'; nm[1] = (char)('a' + e); nm[2] = (char)('0' + hh); nm[3] = (char)('0' + m + 1); nm[4] = 0;
				tbegin(nm, "v50 extra: text effects");
				emit("   effects %d height %d mode %s\n", eff[e], hts[hh] ? hts[hh] : 0, mn[m]);
				x50_backdrop(70);
				if (hts[hh])
					p1(12, 0, hts[hh]);
				i1(106, eff[e]); i1(22, 1); i1(32, m + 1);
				gtext(31, hts[hh] ? 62 : 52, "Wg jQ@ix");
				grab(nm, 20, 20, 220, 70);
			}
	for (m = 0; m < 4; m++)
		for (i = 0; i < 2; i++) {
			nm[0] = 'Z'; nm[1] = (char)('0' + i); nm[2] = (char)('0' + m + 1); nm[3] = 0;
			tbegin(nm, "v50 extra: rotated text / v_justified");
			emit("   %s mode %s\n", i ? "v_justified effects 5" : "rotation 900 effects 1", mn[m]);
			x50_backdrop(70);
			i1(22, 1); i1(32, m + 1);
			if (!i) {
				i1(13, 900); i1(106, 1);
				gtext(111, 80, "Wg jQ@ix");
			} else {
				int k;
				const char *s = "Just ified";

				i1(106, 5);
				for (k = 0; s[k]; k++)
					intin[2 + k] = (int16_t)(uint8_t)s[k];
				ptsin[0] = 31; ptsin[1] = 52; ptsin[2] = 177; ptsin[3] = 0;
				intin[0] = 1; intin[1] = 1;
				vdi(11, 10, 2, 2 + (int16_t)strlen(s));	/* v_justified = opcode 11, sub 10 */
			}
			grab(nm, 20, 20, 220, 70);
		}
}

static void x50_cpyfm_run(const char *nm, int mode, int fg, int bg, int fdh, int sx, int sy, int w, int h, int dx, int dy,
                          int obytes, int doclip)
{
	static uint16_t raster[70];
	static uint8_t odd[2 * 70 + 2];
	MFDB src, dst;
	uint16_t *r = raster;
	int i;
	uint32_t seed = 0x2545F491UL;

	for (i = 0; i < 70; i++) {
		seed = seed * 1103515245UL + 12345UL;
		raster[i] = (uint16_t)(seed >> 15);
	}
	if (obytes) {		/* the same words at an odd address */
		memcpy(odd + 1, raster, sizeof raster);
		r = (uint16_t *)(void *)(odd + 1);
	}
	begin(nm, "v50 extra: vrt_cpyfm");
	emit("   mode %d fg %d bg %d src x %d y %d w %d h %d fd_h %d dst %d,%d odd %d clip %d\n", mode, fg, bg, sx, sy, w, h,
	     fdh, dx, dy, obytes, doclip);
	x50_backdrop(50);
	memset(&src, 0, sizeof src);
	memset(&dst, 0, sizeof dst);
	src.fd_addr = r; src.fd_w = 48; src.fd_h = (int16_t)fdh; src.fd_wdwidth = 3; src.fd_nplanes = 1;
	if (doclip)
		clip(1, 60, 25, 150, 55);
	memset(contrl, 0, sizeof contrl);
	contrl[0] = 121; contrl[1] = 4; contrl[3] = 3; contrl[6] = handle;
	set_mfdb_ptr(0, &src);
	set_mfdb_ptr(1, &dst);
	intin[0] = (int16_t)mode; intin[1] = (int16_t)fg; intin[2] = (int16_t)bg;
	ptsin[0] = (int16_t)sx; ptsin[1] = (int16_t)sy; ptsin[2] = (int16_t)(sx + w - 1); ptsin[3] = (int16_t)(sy + h - 1);
	ptsin[4] = (int16_t)dx; ptsin[5] = (int16_t)dy; ptsin[6] = (int16_t)(dx + w - 1); ptsin[7] = (int16_t)(dy + h - 1);
	td_abi_vdi(contrl, intin, ptsin, intout, ptsout);
	grab(nm, 20, 20, 220, 50);
}

static void x50_cpyfm(void)
{
	static const int sxs[7] = { 0, 1, 7, 15, 16, 17, 31 };
	static const int dxs[2] = { 23, 40 };
	char nm[9];
	int m, s, d, k;

	for (m = 0; m < 4; m++)
		for (s = 0; s < 7; s++)
			for (d = 0; d < 2; d++) {
				nm[0] = 'W'; nm[1] = (char)('a' + s); nm[2] = (char)('0' + d); nm[3] = (char)('0' + m + 1); nm[4] = 0;
				x50_cpyfm_run(nm, m + 1, 1, 6, 20, sxs[s], 2, 17, 9, dxs[d], 30, 0, 0);
			}
	/* refused by the NEON adapter: negative source column, negative row, source shorter than the block (fd_h 8,
	 * rows 5..16), odd raster address, clip window; the scalar loop draws them as before */
	for (m = 0; m < 4; m++) {
		for (k = 0; k < 6; k++) {
			nm[0] = 'V'; nm[1] = (char)('a' + k); nm[2] = (char)('0' + m + 1); nm[3] = 0;
			switch (k) {
			case 0: x50_cpyfm_run(nm, m + 1, 1, 6, 20, -3, 2, 20, 9, 31, 30, 0, 0); break;
			case 1: x50_cpyfm_run(nm, m + 1, 1, 6, 20, 4, -2, 20, 9, 31, 30, 0, 0); break;
			case 2: x50_cpyfm_run(nm, m + 1, 1, 6, 8, 3, 5, 40, 12, 31, 30, 0, 0); break;
			case 3: x50_cpyfm_run(nm, m + 1, 1, 6, 20, 5, 1, 33, 9, 31, 30, 1, 0); break;
			case 4: x50_cpyfm_run(nm, m + 1, 1, 6, 20, 3, 1, 40, 12, 31, 30, 0, 1); break;
			default: x50_cpyfm_run(nm, m + 1, 1, 6, 20, 40, 1, 20, 9, 31, 30, 0, 0); break;	/* columns past fd_w */
			}
		}
	}
}

static void x50(void)
{
	x50_text();
	x50_cpyfm();
}

