
/* ---- TY (V44 tester scratch): cells around the 64 x 64 bound of the narrow rows --------------------------------
 * loaded monospace font id 15, codes 'A','B', cell W x H, effects, exact double / DDA up, rotation. */
static void ty_case(const char *tag, int W, int H, int eff, int scale, int rot)
{
	int fw = ((2 * W + 15) / 16) * 2, bottom = H / 4, top = H - 1 - bottom, sh, gw, gh, m, th;
	char what[96];

	tx_off[0] = 0; tx_off[1] = (uint16_t)W; tx_off[2] = (uint16_t)(2 * W);
	tx_pattern(tx_dat, fw, H);
	tx_head(&tx_f[0], 15, 12, "AUDIT TY", top, bottom, W, 65, 66, F_STD | F_MONO, tx_off, tx_dat, fw, H, NULL);
	snprintf(what, sizeof what, "cell %dx%d eff %d scale %d rot %d", W, H, eff, scale, rot);
	tbegin(tag, what);
	tx_load(&tx_f[0]);
	i1(21, 15);
	th = scale == 1 ? top * 2 : scale == 2 ? top * 3 / 2 : top;
	tx_height(th);
	m = scale == 1 ? 2 : 1;
	if (eff)
		i1(106, (int16_t)eff);
	if (rot)
		i1(13, (int16_t)rot);
	sh = th + 10;
	if (rot) {
		gtext(30, 60 + 2 * W * m, "AB");
		gw = H * m + 60; gh = 2 * W * m + 80;
	} else {
		gtext(10, sh, "AB");
		gw = 2 * W * m + 40; gh = H * m + sh + 20;
	}
	if (gw > 640) gw = 640;
	if (gh > 480) gh = 480;
	grab(tag, 0, 0, gw, gh);
	tx_end();
}

static void ty(void)
{
	static const int fonts[][2] = { {63,16},{64,16},{65,16},{16,63},{16,64},{16,65},{31,16},{32,16},{33,16},{16,31},{16,32},{16,33},
					{32,32},{33,33},{62,62} };
	static const int effs[] = { 0, 1, 4, 16, 31 };
	static const int dda[][2] = { {42,16},{43,16},{16,42},{16,43} };
	char tag[9];
	int i, e, s, n = 0;

	for (i = 0; i < (int)(sizeof fonts / sizeof fonts[0]); i++)
		for (e = 0; e < 5; e++)
			for (s = 0; s < 2; s++) {
				snprintf(tag, sizeof tag, "Y%03d", n++);
				ty_case(tag, fonts[i][0], fonts[i][1], effs[e], s, 0);
			}
	for (i = 0; i < 4; i++)
		for (e = 0; e < 5; e += 2) {
			snprintf(tag, sizeof tag, "Y%03d", n++);
			ty_case(tag, dda[i][0], dda[i][1], effs[e], 2, 0);
		}
	for (i = 0; i < 6; i++)
		for (e = 0; e < 2; e++) {
			snprintf(tag, sizeof tag, "Y%03d", n++);
			ty_case(tag, fonts[i][0], fonts[i][1], effs[e], 0, 900);
		}
	emit("   TY cases %d\n", n);
}
