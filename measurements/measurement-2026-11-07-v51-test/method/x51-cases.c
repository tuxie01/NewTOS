/* ---- x51 (V51 tester, extra case, not in vdprobe.c): screen->screen vro_cpyfm over a pseudo-random 16-bit memory
 * form (drawn by memory->screen S_ONLY first), the destination offset against the source in all 8 directions plus the
 * same row / same column (ox in -19,-3,0,3,19; oy in -3,0,3), ops 3, 6, 7, 10 (and 1, 2, 4, 8, 9, 11..14 on a subset),
 * source x odd (21), widths 16/17/64/65, 7 rows; then full-width scrolls (600 px, by one row and by 3 px, odd x).
 * The grabbed window covers source and destination with a 2 px margin.  */
#define X51_BW 640
#define X51_BH 32
static uint16_t x51_buf[WK_GUARD + X51_BW * X51_BH + WK_GUARD];

static void x51_one(const char *name, int op, int sx, int sy, int bw, int bh, int ox, int oy, int seed, int gx, int gy, int gw, int gh)
{
	MFDB s, d;
	int fw = gw; /* pseudo-random backdrop covers the grab window: (gx,gy) gw x gh */

	begin(name, "v51 extra: screen->screen overlapping copy");
	emit("   op %d src (%d,%d) %dx%d offset (%d,%d)\n", op, sx, sy, bw, bh, ox, oy);
	/* backdrop: memory form fw x gh, 16 planes, to the screen at (gx,gy), S_ONLY */
	{
		int wd = (fw + 15) / 16, n = wd * 16 * gh;

		wk_fill(x51_buf, n, 0x51ABCDEFUL + (uint32_t)seed * 977UL);
		memset(&s, 0, sizeof s);
		memset(&d, 0, sizeof d);
		s.fd_addr = x51_buf + WK_GUARD;
		s.fd_w = (int16_t)(16 * wd);
		s.fd_h = (int16_t)gh;
		s.fd_wdwidth = (int16_t)wd;
		s.fd_nplanes = 16;
		memset(contrl, 0, sizeof contrl);
		contrl[0] = 109; contrl[1] = 4; contrl[3] = 1; contrl[6] = handle;
		set_mfdb_ptr(0, &s);
		set_mfdb_ptr(1, &d);
		intin[0] = 3;
		ptsin[0] = 0; ptsin[1] = 0; ptsin[2] = (int16_t)(fw - 1); ptsin[3] = (int16_t)(gh - 1);
		ptsin[4] = (int16_t)gx; ptsin[5] = (int16_t)gy; ptsin[6] = (int16_t)(gx + fw - 1); ptsin[7] = (int16_t)(gy + gh - 1);
		td_abi_vdi(contrl, intin, ptsin, intout, ptsout);
	}
	memset(&s, 0, sizeof s);
	memset(&d, 0, sizeof d);
	memset(contrl, 0, sizeof contrl);
	contrl[0] = 109; contrl[1] = 4; contrl[3] = 1; contrl[6] = handle;
	set_mfdb_ptr(0, &s);
	set_mfdb_ptr(1, &d);
	intin[0] = (int16_t)op;
	ptsin[0] = (int16_t)sx; ptsin[1] = (int16_t)sy;
	ptsin[2] = (int16_t)(sx + bw - 1); ptsin[3] = (int16_t)(sy + bh - 1);
	ptsin[4] = (int16_t)(sx + ox); ptsin[5] = (int16_t)(sy + oy);
	ptsin[6] = (int16_t)(sx + ox + bw - 1); ptsin[7] = (int16_t)(sy + oy + bh - 1);
	td_abi_vdi(contrl, intin, ptsin, intout, ptsout);
	grab(name, gx, gy, gw, gh);
}

static void x51(void)
{
	static const int ops[4] = { 3, 6, 7, 10 };
	static const int wds[4] = { 16, 17, 64, 65 };
	static const int oxs[5] = { -19, -3, 0, 3, 19 };
	static const int oys[3] = { -3, 0, 3 };
	static const int more[9] = { 1, 2, 4, 8, 9, 11, 12, 13, 14 };
	char nm[9];
	int o, w, a, b, m, n = 0;

	for (o = 0; o < 4; o++)
		for (w = 0; w < 4; w++)
			for (a = 0; a < 5; a++)
				for (b = 0; b < 3; b++) {
					if (oxs[a] == 0 && oys[b] == 0)
						continue;
					nm[0] = 'X'; nm[1] = (char)('a' + o); nm[2] = (char)('a' + w); nm[3] = (char)('a' + a);
					nm[4] = (char)('a' + b); nm[5] = 0;
					x51_one(nm, ops[o], 21, 12, wds[w], 7, oxs[a], oys[b], n++, 0, 0, 21 + 65 + 19 + 4, 12 + 7 + 3 + 4);
				}
	/* the other ops, width 17 and 65, four offsets */
	for (m = 0; m < 9; m++)
		for (w = 1; w < 4; w += 2)
			for (a = 0; a < 4; a++) {
				static const int ex[4] = { 3, -3, 19, -19 }, ey[4] = { 3, -3, 0, 0 };

				nm[0] = 'Y'; nm[1] = (char)('a' + m); nm[2] = (char)('a' + w); nm[3] = (char)('a' + a); nm[4] = 0;
				x51_one(nm, more[m], 21, 12, wds[w], 7, ex[a], ey[a], n++, 0, 0, 21 + 65 + 19 + 4, 12 + 7 + 3 + 4);
			}
	/* wide scrolls: 600 px (odd start 11), by one row up/down, by 3 px left/right, ops 3 and 6 */
	for (o = 0; o < 2; o++)
		for (a = 0; a < 4; a++) {
			static const int sx_[4] = { 0, 0, 3, -3 }, sy_[4] = { 1, -1, 0, 0 };

			nm[0] = 'W'; nm[1] = (char)('a' + o); nm[2] = (char)('a' + a); nm[3] = 0;
			x51_one(nm, ops[o], 11, 20, 600, 9, sx_[a], sy_[a], n++, 4, 12, 620, 30);
		}
}

