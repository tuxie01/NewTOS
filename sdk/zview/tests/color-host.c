/*
 * color-host.c -- host harness for zView's palette matcher (zview/zvdi/
 * color.c: save_colors, remap_color, color_lookup, cnvpal_4_8), TASK/SDK04
 * probe S3 (finding 8 of docs/analysis/byte-order-survey-2026-10-29.md).
 * Built and run by color-host.sh beside this file, which compiles the
 * color.c under test unchanged against color-host-stubs.h.
 *
 * The m68k contract, read from color.c: an `int32` colour is 0x00RRGGBB
 * (cnvpal_4_8 and vdi.c build it so); remap_color() returns the index of
 * the screen palette entry equal to it (red, green, blue and saturation),
 * else the best fit (sum of the four distances, the last of equal ones);
 * color_lookup(rgb, trans) returns (trans[idx] << 24) | the entry's RGB,
 * where a colour of the form 0xFFFFFFnn means index nn directly. The
 * palette is the VDI one (0..1000 per gun), scaled to 0..255 by
 * save_colors(). The expected values below are those of the m68k (big-
 * endian, 32-bit long) reading; every query colour is either a palette
 * entry or unambiguously nearest to one.
 *
 * The host must be little-endian like the target (macOS arm64 is);
 * otherwise the harness refuses (exit 77). Exit status: number of failed
 * checks.
 */
#include <stdio.h>
#include ZVIEW_COLOR_C

struct host_app app;
int16 pixel_val[256];

static int fails, checks;

static void check(int ok, const char *what)
{
	checks++;
	if (!ok)
		fails++;
	printf("%s  %s\n", ok ? "ok  " : "FAIL", what);
}

/* VDI palette, 16 entries, and the 8-bit values save_colors() makes of it
 * (((v << 8) - v + 500) / 1000: 1000 -> 255, 752 -> 192, 501 -> 128). */
static short vdi_pal[16][3] = {
	{ 1000, 1000, 1000 }, { 0, 0, 0 }, { 1000, 0, 0 }, { 0, 1000, 0 },
	{ 0, 0, 1000 }, { 0, 1000, 1000 }, { 1000, 1000, 0 }, { 1000, 0, 1000 },
	{ 752, 752, 752 }, { 501, 501, 501 }, { 501, 0, 0 }, { 0, 501, 0 },
	{ 0, 0, 501 }, { 0, 501, 501 }, { 501, 501, 0 }, { 501, 0, 501 },
};

static uint32 rgb8(int i)
{
	uint32 v[3];
	int k;

	for (k = 0; k < 3; k++)
		v[k] = (uint32)((((int32)vdi_pal[i][k] << 8) - vdi_pal[i][k] + 500) / 1000);
	return (v[0] << 16) | (v[1] << 8) | v[2];
}

int main(void)
{
	static const struct { uint32 rgb; int idx; const char *name; } near[] = {
		{ 0xFA0A0Au, 2, "near red" },
		{ 0x05FA05u, 3, "near green" },
		{ 0x0A0AF0u, 4, "near blue" },
		{ 0x820000u, 10, "near dark red" },
		{ 0xC8BEC3u, 8, "near light grey" },
	};
	int16 trans[256];
	char what[160];
	union { uint32 u; unsigned char c[4]; } bo = { 0x01020304u };
	struct _img_info info;
	struct _dec_data data;
	int i;

	if (bo.c[0] != 0x04) {
		printf("color-host: host is not little-endian -- refusing\n");
		return 77;
	}
	if (sizeof(int32) != 4) {
		printf("color-host: int32 is not 32-bit (prelude missing)\n");
		return 1;
	}

	app.nplanes = 4;
	app.color = 16;
	app.palette = vdi_pal;
	for (i = 0; i < 256; i++) {
		trans[i] = (int16)(100 + i);
		pixel_val[i] = (int16)(20 + i);	/* < 128: no signed overflow in << 24 */
	}
	save_colors();
	check(screen_colortab[2].red == 255 && screen_colortab[2].green == 0 && screen_colortab[2].satur == 255 &&
	      screen_colortab[8].red == 192 && screen_colortab[9].blue == 128 && screen_colortab[9].satur == 0 &&
	      rgb8(8) == 0xC0C0C0u && rgb8(9) == 0x808080u,
	      "save_colors: 1000 -> 255, 752 -> 192, 501 -> 128, saturation");

	/* every palette colour maps to its own index */
	for (i = 0; i < 16; i++) {
		int16 got = remap_color((int32)rgb8(i));

		snprintf(what, sizeof what, "remap_color(0x%06X) = %d (got %d)", (unsigned)rgb8(i), i, got);
		check(got == i, what);
	}
	/* color_lookup: index from the translation table, RGB of the entry */
	for (i = 0; i < 16; i++) {
		uint32 want = ((uint32)trans[i] << 24) | rgb8(i);
		uint32 got = color_lookup(rgb8(i), trans);

		snprintf(what, sizeof what, "color_lookup(0x%06X) = 0x%08X (got 0x%08X)",
			 (unsigned)rgb8(i), (unsigned)want, (unsigned)got);
		check(got == want, what);
	}
	/* best fit */
	for (i = 0; i < (int)(sizeof near / sizeof near[0]); i++) {
		int16 got = remap_color((int32)near[i].rgb);

		snprintf(what, sizeof what, "remap_color(0x%06X) %s = %d (got %d)",
			 (unsigned)near[i].rgb, near[i].name, near[i].idx, got);
		check(got == near[i].idx, what);
	}
	/* 0xFFFFFFnn: index nn directly */
	{
		uint32 want = ((uint32)trans[5] << 24) | rgb8(5);
		uint32 got = color_lookup(0xFFFFFF05u, trans);

		snprintf(what, sizeof what, "color_lookup(0xFFFFFF05) = index 5 -> 0x%08X (got 0x%08X)",
			 (unsigned)want, (unsigned)got);
		check(got == want, what);
	}
	/* cnvpal_4_8: a picture palette of red and blue */
	memset(&info, 0, sizeof info);
	memset(&data, 0, sizeof data);
	info.colors = 2;
	info.palette[0].red = 255;
	info.palette[1].blue = 255;
	cnvpal_4_8(&info, &data);
	snprintf(what, sizeof what, "cnvpal_4_8 red -> 0x%08X (got 0x%08X)",
		 (unsigned)(((uint32)pixel_val[2] << 24) | 0xFF0000u), (unsigned)data.Pixel[0]);
	check(data.Pixel[0] == (((uint32)pixel_val[2] << 24) | 0xFF0000u), what);
	snprintf(what, sizeof what, "cnvpal_4_8 blue -> 0x%08X (got 0x%08X)",
		 (unsigned)(((uint32)pixel_val[4] << 24) | 0x0000FFu), (unsigned)data.Pixel[1]);
	check(data.Pixel[1] == (((uint32)pixel_val[4] << 24) | 0x0000FFu), what);

	printf("color-host: %d checks, %d failed\n", checks, fails);
	return fails;
}
