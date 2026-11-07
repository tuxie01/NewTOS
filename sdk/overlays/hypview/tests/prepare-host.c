/*
 * prepare-host.c -- host harness for HypView's picture loader (hyp/prepare.c:
 * LoadPicture), TASK/A142. Built and run by prepare-host.sh beside this file.
 *
 * This file is the translation unit: it replaces the GEMDOS block calls with
 * host malloc/free, fakes the VDI calls and the rest of HypView, and #includes
 * the prepare.c under test (PREPARE_C, a quoted path) unchanged.
 *
 * A .HYP file holds picture planes as big-endian words. NewTOS' vr_trnfm and
 * vrt_cpyfm read an MFDB as native words with bit 15 the leftmost pixel, so a
 * plane has to be native by the time it reaches either of them, exactly once:
 * the colour path through mono_bitmap() decodes the file bytes itself. Each
 * case hands LoadPicture a picture with asymmetric words and looks at what the
 * VDI receives.
 *
 * The host must be LP64 little-endian like the target (macOS arm64 is);
 * otherwise the harness refuses (exit 77). Exit status: number of failed
 * checks.
 */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include <osbind.h>

#undef Malloc
#undef Mfree
#undef Mshrink
#define Malloc(n)	malloc((size_t)(n))
#define Mfree(p)	free((void *)(p))
#define Mshrink(p, n)	((void)(p), (void)(n), 0L)

#ifndef PREPARE_C
#error "PREPARE_C (quoted path of the prepare.c under test) not defined"
#endif
#include PREPARE_C

/* ---- what prepare.c takes from the rest of HypView ----------------------- */
short vdi_handle = 1;
short ext_workout[57];
short background_col;
short transparent_pics;
short font_cw = 8, font_ch = 16;
short font_id, font_pt;
short link_effect;

static unsigned char *entry;
static long entry_size;

void Debug(const char *fmt, ...) { (void)fmt; }
void *LoadData(HYP_DOCUMENT *h, long n, long *loaded)
{
	void *p = malloc((size_t)entry_size);

	(void)h; (void)n;
	memcpy(p, entry, (size_t)entry_size);
	*loaded = entry_size;
	return p;
}
long GetDataSize(HYP_DOCUMENT *h, long n) { (void)h; (void)n; return entry_size; }
void GetEntryBytes(HYP_DOCUMENT *h, long n, void *src, void *dst, long bytes)
{
	(void)h; (void)n;
	memcpy(dst, src, (size_t)bytes);
}
void *AskCache(HYP_DOCUMENT *h, long n) { (void)h; (void)n; return NULL; }
void TellCache(HYP_DOCUMENT *h, long n, void *p) { (void)h; (void)n; (void)p; }

/* PrepareNode's text layout; the pictures go through LoadPicture directly */
char *skip_esc(char *pos) { return pos; }
short vst_effects(short h, short e) { (void)h; return e; }
void vqt_extent(short h, const char *s, short *pts) { (void)h; (void)s; memset(pts, 0, 16); }

/* ---- the VDI: record what it reads, as the kernel reads it ---------------- */
#define MAX_CALLS 32
#define MAX_WORDS 64
struct call {
	short stand, nplanes;
	unsigned short words[MAX_WORDS];
	int nwords;
};
static struct call trnfm[MAX_CALLS], cpyfm[MAX_CALLS];
static int ntrnfm, ncpyfm;

static void record(struct call *c, const MFDB *m)
{
	long total = (long)m->fd_wdwidth * m->fd_h * m->fd_nplanes;
	long i;

	c->stand = m->fd_stand;
	c->nplanes = m->fd_nplanes;
	c->nwords = total > MAX_WORDS ? MAX_WORDS : (int)total;
	for (i = 0; i < c->nwords; i++)
		memcpy(&c->words[i], (const char *)m->fd_addr + 2 * i, 2);
}

void vr_trnfm(short h, MFDB *src, MFDB *dst)
{
	(void)h;
	if (ntrnfm < MAX_CALLS)
		record(&trnfm[ntrnfm++], src);
	memcpy(dst->fd_addr, src->fd_addr, (size_t)((long)src->fd_wdwidth * src->fd_h * src->fd_nplanes * 2));
}

void vrt_cpyfm(short h, short mode, short *pxy, MFDB *src, MFDB *dst, short *col)
{
	(void)h; (void)mode; (void)pxy; (void)dst; (void)col;
	if (ncpyfm < MAX_CALLS)
		record(&cpyfm[ncpyfm++], src);
}

/* ---- the cases ------------------------------------------------------------ */
static int failed, checks;

static void check(int ok, const char *name, const char *what)
{
	checks++;
	if (!ok)
	{
		failed++;
		printf("FAIL  %s: %s\n", name, what);
	}
}

/* 32 x 2 pixels: plane p, word w of 4; every word differs in its two bytes */
static unsigned short plane_word(int p, int w)
{
	return (unsigned short)(0x1200 + (p << 12) + (w << 4) + 0x0F - w);
}

static LOADED_PICTURE *load(int planes, int screen_planes)
{
	long plane_size = 8;
	int p, w;

	entry_size = (long)sizeof(HYP_PICTURE) + plane_size * planes;
	entry = malloc((size_t)entry_size);
	entry[0] = 0; entry[1] = 32;
	entry[2] = 0; entry[3] = 2;
	entry[4] = (unsigned char)planes;
	entry[5] = (unsigned char)((1 << planes) - 1);
	entry[6] = 0;
	entry[7] = 0;
	for (p = 0; p < planes; p++)
		for (w = 0; w < 4; w++)
		{
			entry[8 + p * 8 + w * 2] = (unsigned char)(plane_word(p, w) >> 8);
			entry[8 + p * 8 + w * 2 + 1] = (unsigned char)plane_word(p, w);
		}
	ext_workout[4] = (short)screen_planes;
	ntrnfm = ncpyfm = 0;
	return LoadPicture((HYP_DOCUMENT *)entry, 1);
}

static void done(LOADED_PICTURE *pic)
{
	free(pic);
	free(entry);
}

static void case_mono(int screen_planes, const char *name)
{
	LOADED_PICTURE *pic = load(1, screen_planes);
	int w, same = 1;

	check(pic != NULL, name, "loaded");
	if (pic == NULL)
		return;
	check(ntrnfm == 1 && ncpyfm == 0, name, "one vr_trnfm, no blit");
	check(trnfm[0].stand == 1 && trnfm[0].nplanes == 1, name, "standard form, one plane");
	for (w = 0; w < 4; w++)
		if (trnfm[0].words[w] != plane_word(0, w))
			same = 0;
	check(same, name, "vr_trnfm reads the file's big-endian words as native words");
	same = 1;
	for (w = 0; w < 4; w++)
	{
		unsigned short got;

		memcpy(&got, (const char *)pic->mfdb.fd_addr + 2 * w, 2);
		if (got != plane_word(0, w))
			same = 0;
	}
	check(same, name, "the MFDB display.c blits holds native words");
	check(pic->mfdb.fd_nplanes == 1 && pic->mfdb.fd_stand == 0, name, "device form, one plane");
	done(pic);
}

/* the colour path on a truecolor screen: mono_bitmap() decodes the bytes, no second swap */
static void case_truecolor(void)
{
	static const char name[] = "4 planes on 16";
	LOADED_PICTURE *pic;
	int p, w, same = 1;

	/* every plane the same word: colour 15 where it is set, else 0 */
	entry_size = (long)sizeof(HYP_PICTURE) + 8 * 4;
	entry = malloc((size_t)entry_size);
	memset(entry, 0, (size_t)entry_size);
	entry[1] = 32; entry[3] = 2; entry[4] = 4; entry[5] = 0x0F;
	for (p = 0; p < 4; p++)
		for (w = 0; w < 4; w++)
		{
			entry[8 + p * 8 + w * 2] = (unsigned char)(plane_word(0, w) >> 8);
			entry[8 + p * 8 + w * 2 + 1] = (unsigned char)plane_word(0, w);
		}
	ext_workout[4] = 16;
	ntrnfm = ncpyfm = 0;
	pic = LoadPicture((HYP_DOCUMENT *)entry, 1);
	check(pic != NULL, name, "loaded");
	if (pic == NULL)
		return;
	check(ntrnfm == 0, name, "no vr_trnfm");
	check(ncpyfm == 15, name, "15 blits, colour 15 then 1..14");
	for (w = 0; w < 4; w++)
		if (cpyfm[0].words[w] != plane_word(0, w))
			same = 0;
	check(same, name, "the colour-15 mask is the file's word, swapped exactly once");
	done(pic);
}

/* the palette path, not reachable while the VDI has 16 planes: same statement */
static void case_palette(int planes, int screen_planes, const char *name)
{
	LOADED_PICTURE *pic = load(planes, screen_planes);
	int p, w, same = 1;

	check(pic != NULL, name, "loaded");
	if (pic == NULL)
		return;
	check(ntrnfm == 1, name, "one vr_trnfm");
	check(trnfm[0].nplanes == screen_planes, name, "the screen's plane count");
	for (p = 0; p < planes; p++)
		for (w = 0; w < 4; w++)
			if (trnfm[0].words[p * 4 + w] != plane_word(p, w))
				same = 0;
	check(same, name, "vr_trnfm reads every file plane as native words");
	done(pic);
}

/* 4 planes on one: reduce() first, in the file's byte order, then the same */
static void case_reduce(void)
{
	static const char name[] = "4 planes on 1";
	LOADED_PICTURE *pic = load(4, 1);
	int w, bit, same = 1;

	check(pic != NULL, name, "loaded");
	if (pic == NULL)
		return;
	check(ntrnfm == 1 && trnfm[0].nplanes == 1, name, "one vr_trnfm, one plane");
	for (w = 0; w < 4; w++)
	{
		unsigned short out = 0;

		for (bit = 15; bit >= 0; bit--)
		{
			int px = ((plane_word(0, w) >> bit) & 1) | (((plane_word(1, w) >> bit) & 1) << 1) |
				(((plane_word(2, w) >> bit) & 1) << 2) | (((plane_word(3, w) >> bit) & 1) << 3);

			out = (unsigned short)((out << 1) | ((0xFF7E >> px) & 1));
		}
		if (trnfm[0].words[w] != out)
			same = 0;
	}
	check(same, name, "vr_trnfm reads reduce()'s result as native words");
	done(pic);
}

int main(void)
{
	const uint16_t probe = 0x0102;

	if (sizeof(long) != 8 || *(const unsigned char *)&probe != 0x02)
	{
		printf("skipped -- host is not LP64 little-endian\n");
		return 77;
	}
	case_mono(16, "1 plane on 16");
	case_mono(4, "1 plane on 4");
	case_truecolor();
	case_palette(4, 4, "4 planes on 4");
	case_palette(2, 4, "2 planes on 4");
	case_reduce();
	printf("%d checks, %d failed\n", checks, failed);
	return failed;
}
