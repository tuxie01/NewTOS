/* K14 (TASK/V21 acceptance): the seed queue of v_contourfill overflows.  EmuTOS ends the whole fill when its
 * 1024-entry queue is full (vdi/vdi_fill.c get_seed), leaving a partial fill.  Dots come from a fixed LCG so
 * both sides draw the same field. */
static uint32_t k14_seed;

static int k14_rand(void)
{
	k14_seed = k14_seed * 1103515245UL + 12345UL;
	return (int)((k14_seed >> 16) & 0x7fff);
}

/* dots on rows y0..y1 over the full width; pct = density in percent, or (step > 0) every step-th column (x % step == 1) */
static void k14_dots(int y0, int y1, int pct, int step)
{
	int16_t pts[2000];
	int n = 0, y, x, j;

	k14_seed = 12345UL;
	for (y = y0; y <= y1; y++)
		for (x = 0; x < 640; x++) {
			int dot = step ? (x % step == 1) : (k14_rand() % 100 < pct);
			if (!dot)
				continue;
			pts[2 * n] = (int16_t)x;
			pts[2 * n + 1] = (int16_t)y;
			if (++n == 1000) {
				for (j = 0; j < 2 * n; j++) ptsin[j] = pts[j];
				vdi(7, 0, (int16_t)n, 0);
				n = 0;
			}
		}
	if (n) {
		for (j = 0; j < 2 * n; j++) ptsin[j] = pts[j];
		vdi(7, 0, (int16_t)n, 0);
	}
}

static void k14(void)
{
	begin("K14A", "queue overflow: 25% random dots on rows 100..104, contourfill from (0,50), solid colour 2, boundary 1");
	k14_dots(100, 104, 25, 0);
	i1(25, 2); contour(0, 50, 1); grab("K14A", 0, 40, 640, 80);
	begin("K14B", "queue overflow: 10% random dots on rows 100..139, contourfill from below (0,300), solid colour 2, boundary 1");
	k14_dots(100, 139, 10, 0);
	i1(25, 2); contour(0, 300, 1); grab("K14B", 0, 0, 640, 320);
	begin("K14C", "control, no overflow: dot on every 2nd column of rows 100..299 (queue 321), contourfill from (0,50)");
	k14_dots(100, 299, 0, 2);
	i1(25, 2); contour(0, 50, 1); grab("K14C", 0, 40, 640, 280);
}

