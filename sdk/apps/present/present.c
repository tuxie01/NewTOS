/*
 * present.c -- PRESENT.NAP, the NewTOS slide show (TASK/PRESENT01).
 *
 * Our own GEM application (not a port). It reads a plain text slide file,
 * takes over the whole screen, shows one slide at a time in large text with
 * an optional PNG picture, and gives the screen back on exit.
 *
 * SLIDE FILE
 *   argv[1] if given, else SLIDES.TXT in the current directory, else
 *   C:\APPS\PRESENT\SLIDES.TXT. Plain text, LF or CRLF, UTF-8 or Atari/Latin-1
 *   (UTF-8 is converted to the Atari character set with the ISO-8859-1 table
 *   of EmuTOS tools/bug.c i2a[]; typographic quotes/dashes/ellipsis become
 *   ASCII). One directive or text line per line:
 *
 *     # Title            starts a new slide with this title
 *     - text             bullet            (also "* text")
 *       - text           sub-bullet        (indented by 2+ spaces or a tab)
 *     text               plain paragraph without a bullet
 *     (blank line)       vertical space
 *     @image NAME.PNG    picture for this slide (path relative to the slide
 *                        file's folder, or absolute "C:\..."); a missing or
 *                        unreadable picture shows a small note instead
 *     @center            centre this slide (title slide look)
 *     @note text         speaker note -- ignored by the viewer
 *     ; text             comment -- ignored
 *
 *   Lines before the first "# " line are a free header (ignored), except
 *   three global directives there:
 *     @footer text       small text bottom left on every slide
 *     @log               write render timings to PRESENT.LOG beside the
 *                        slide file on exit
 *     @spacing N         extra space after each bullet/line, in twelfths of
 *                        that item's character height (0..24, default 4);
 *                        lines wrapped inside one item stay tighter
 *   Long lines wrap at word boundaries. Limits: 64 slides, 40 lines per slide;
 *   more are dropped. Text that does not fit is first shrunk (bullet size steps
 *   down), then clipped at the bottom.
 *
 * KEYS (evnt_multi MU_KEYBD|MU_BUTTON)
 *   next      Right, Down, Space, Return, N, left mouse button
 *   previous  Left, Up, Backspace, P, right mouse button
 *   first     Home (ClrHome)        last   Shift+Home
 *   quit      Esc, Q
 *
 * HOW IT DRAWS
 *   The screen depth is read once (vq_extnd, intout[4]): PRESENT runs at 16
 *   planes (the NewTOS default) and at 32 planes (boot argument vdiplanes=32,
 *   ADR-038); any other count gives an alert and a clean exit. A NewTOS memory
 *   MFDB has the device's form: at 16 planes one 16-bit word per pixel
 *   (RRRRRGGGGG0BBBBB), at 32 planes one 32-bit word 0x00RRGGBB per pixel
 *   (ADR-038 point 5); a 16-plane form is refused at 32 planes.
 *   The system font (8x16 on NewTOS) is read once: the 224 printable cells are
 *   drawn with v_gtext and copied back with vro_cpyfm into a memory MFDB of
 *   the screen's depth. Larger sizes are made from those bitmaps with
 *   Scale2x/Scale3x (EPX), which rounds diagonals instead of producing blocks,
 *   and cached per size. A slide is composed into one off-screen 16-bit
 *   RRRRRGGGGG0BBBBB frame and put on the screen with a single vro_cpyfm
 *   (S_ONLY); at 32 planes the frame is first expanded to 0x00RRGGBB (5 -> 8
 *   bits per gun by bit replication, the expansion the 16-plane device itself
 *   applies), so the colours keep 16-bit quality at both depths. Pictures:
 *   libpng -> RGBA8 -> alpha-composited on the background, enlarged by the
 *   largest whole factor that fits (crisp pixels) or reduced with a box
 *   filter, then cached per slide.
 *
 * SCREEN OWNERSHIP
 *   wind_update(BEG_UPDATE) + wind_update(BEG_MCTRL), form_dial(FMD_START)
 *   over the whole screen, graf_mouse(M_OFF); on exit M_ON, form_dial
 *   (FMD_FINISH) -> the AES redraws what was under it, END_MCTRL, END_UPDATE.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <setjmp.h>
#include <png.h>
#include <gem.h>
#include <mint/osbind.h>
#include <mint/sysvars.h>

typedef unsigned char u8;
typedef unsigned short u16;
typedef unsigned int u32;

/* ------------------------------------------------------------------------ */
/* Slide model                                                              */

#define MAX_SLIDES  64
#define MAX_ITEMS   40
#define MAX_FILE    (256L * 1024L)

enum { IT_BULLET = 1, IT_SUB, IT_TEXT, IT_GAP };

typedef struct {
	int kind;
	char *text;
} Item;

typedef struct {
	char *title;
	char *image;
	int center;
	int nitems;
	Item item[MAX_ITEMS];
	/* picture cache (made for one picture area size) */
	int pic_state;          /* 0 not loaded, 1 ok, -1 failed */
	u16 *pic;
	int pw, ph;             /* picture size after scaling */
	int aw, ah;             /* area it was made for */
	char why[96];           /* failure text */
	/* timings (hz_200 ticks, @log): this showing and the maximum */
	long t_render, t_blit, t_image, shown;
	long m_render, m_blit, m_image;
} Slide;

static Slide slides[MAX_SLIDES];
static int nslides;
static int want_log;
static char *footer;
/* @spacing: extra space after an item in twelfths of its character height */
#define SPACING_DEFAULT 4
#define SPACING_MAX     24
static int item_space = SPACING_DEFAULT;
static char basedir[160];
static char slidepath[200];

/* ------------------------------------------------------------------------ */
/* Character set: UTF-8 / Latin-1 -> Atari ST                               */

/* ISO-8859-1 0x80..0xFF -> Atari ST, EmuTOS tools/bug.c i2a[] (0 = none). */
static const u8 i2a[128] = {
	   0,    0,    0,    0,    0,    0,    0,    0,
	   0,    0,    0,    0,    0,    0,    0,    0,
	   0,    0,    0,    0,    0,    0,    0,    0,
	   0,    0,    0,    0,    0,    0,    0,    0,
	   0, 0xad, 0x9b, 0x9c,    0, 0x9d,    0, 0xdd,
	0xb9, 0xbd, 0xa6, 0xae, 0xaa,    0, 0xbe, 0xff,
	0xf8, 0xf1, 0xfd, 0xfe, 0xba, 0xe6, 0xbc, 0xfa,
	   0,    0, 0xa7, 0xaf, 0xac, 0xab,    0, 0xa8,
	0xb6,    0,    0, 0xb7, 0x8e, 0x8f, 0x92, 0x80,
	   0, 0x90,    0,    0,    0,    0,    0,    0,
	   0, 0xa5,    0,    0,    0, 0xb8, 0x99,    0,
	0xb2,    0,    0,    0, 0x9a,    0,    0, 0x9e,
	0x85, 0xa0, 0x83, 0xb0, 0x84, 0x86, 0x91, 0x87,
	0x8a, 0x82, 0x88, 0x89, 0x8d, 0xa1, 0x8c, 0x8b,
	   0, 0xa4, 0x95, 0xa2, 0x93, 0xb1, 0x94, 0xf6,
	0xb3, 0x97, 0xa3, 0x96, 0x81,    0,    0, 0x98,
};

/* EmuTOS tools/bug.c replacements[] for Latin-1 without an Atari glyph. */
static int latin1_fallback(unsigned c)
{
	switch (c) {
	case 0xa0: return ' ';
	case 0xa6: return '|';
	case 0xad: return '-';
	case 0xc1: case 0xc2: return 'A';
	case 0xc8: case 0xca: case 0xcb: return 'E';
	case 0xcc: case 0xcd: case 0xce: case 0xcf: return 'I';
	case 0xd2: case 0xd3: case 0xd4: return 'O';
	case 0xd7: return 'x';
	case 0xd9: case 0xda: case 0xdb: return 'U';
	case 0xdd: return 'Y';
	case 0xfd: return 'y';
	default: return '?';
	}
}

/* Append the Atari form of code point cp to out; returns bytes written. */
static int put_cp(unsigned cp, char *out)
{
	const char *s = NULL;

	if (cp < 0x80) {
		out[0] = (char)cp;
		return 1;
	}
	if (cp >= 0xa0 && cp <= 0xff) {
		u8 a = i2a[cp - 0x80];
		out[0] = (char)(a ? a : latin1_fallback(cp));
		return 1;
	}
	switch (cp) {
	case 0x2018: case 0x2019: case 0x201a: case 0x2032: s = "'"; break;
	case 0x201c: case 0x201d: case 0x201e: case 0x2033: s = "\""; break;
	case 0x2010: case 0x2011: case 0x2012: case 0x2013:
	case 0x2014: case 0x2015: case 0x2212: s = "-"; break;
	case 0x2026: s = "..."; break;
	case 0x2022: case 0x00b7: case 0x2027: s = "\xfa"; break; /* middle dot */
	case 0x2192: s = "->"; break;
	case 0x2190: s = "<-"; break;
	case 0x20ac: s = "EUR"; break;
	case 0x2122: s = "TM"; break;
	case 0x00a0: case 0x2009: case 0x200a: case 0x202f: s = " "; break;
	default: s = "?"; break;
	}
	{
		int n = (int)strlen(s);
		memcpy(out, s, n);
		return n;
	}
}

/* Convert in place. Every replacement is no longer than its UTF-8 source
 * (3-byte sequences give at most 3 bytes), so the buffer suffices. Bytes that
 * do not form valid UTF-8 are taken as Latin-1. Control characters drop out,
 * tabs become one space. */
static void to_atari(char *line)
{
	u8 *r = (u8 *)line;
	char *w = line;

	while (*r) {
		unsigned c = *r, cp = 0;
		int len = 0;

		if (c == '\t') { *w++ = ' '; r++; continue; }
		if (c < 0x20 || c == 0x7f) { r++; continue; }
		if (c < 0x80) { *w++ = (char)c; r++; continue; }
		if ((c & 0xe0) == 0xc0) { cp = c & 0x1f; len = 1; }
		else if ((c & 0xf0) == 0xe0) { cp = c & 0x0f; len = 2; }
		else if ((c & 0xf8) == 0xf0) { cp = c & 0x07; len = 3; }
		if (len) {
			int i, ok = 1;
			for (i = 1; i <= len; i++) {
				if ((r[i] & 0xc0) != 0x80) { ok = 0; break; }
				cp = (cp << 6) | (r[i] & 0x3f);
			}
			if (ok && cp >= 0x80) {
				w += put_cp(cp, w);
				r += len + 1;
				continue;
			}
		}
		/* not UTF-8: Latin-1 byte */
		if (c >= 0xa0)
			w += put_cp(c, w);
		else
			*w++ = '?';
		r++;
	}
	*w = 0;
}

/* ------------------------------------------------------------------------ */
/* Slide file parser                                                        */

static char *skip_blanks(char *s)
{
	while (*s == ' ' || *s == '\t')
		s++;
	return s;
}

static void rtrim(char *s)
{
	size_t n = strlen(s);
	while (n > 0 && (s[n - 1] == ' ' || s[n - 1] == '\t' || s[n - 1] == '\r'))
		s[--n] = 0;
}

static void add_item(Slide *sl, int kind, char *text)
{
	if (kind == IT_GAP) {
		/* no gap at the top, none twice in a row */
		if (sl->nitems == 0 || sl->item[sl->nitems - 1].kind == IT_GAP)
			return;
	}
	if (sl->nitems >= MAX_ITEMS)
		return;
	sl->item[sl->nitems].kind = kind;
	sl->item[sl->nitems].text = text;
	sl->nitems++;
}

static int keyword(char *s, const char *kw, char **arg)
{
	size_t n = strlen(kw);
	if (strncmp(s, kw, n) != 0)
		return 0;
	if (s[n] != 0 && s[n] != ' ' && s[n] != '\t')
		return 0;
	*arg = skip_blanks(s + n);
	return 1;
}

/* Parse the whole file text (modified in place; the slide strings point
 * into it). */
static void parse_slides(char *text)
{
	char *p = text;
	Slide *cur = NULL;

	while (*p) {
		char *line = p, *nl = strchr(p, '\n');
		char *s, *arg;
		int indent = 0;

		if (nl) { *nl = 0; p = nl + 1; } else p += strlen(p);
		rtrim(line);

		/* indentation (tab = 4) */
		for (s = line; *s == ' ' || *s == '\t'; s++)
			indent += (*s == '\t') ? 4 : 1;

		if (s[0] == '#' && (s[1] == ' ' || s[1] == 0) && indent == 0) {
			if (nslides >= MAX_SLIDES) { cur = NULL; continue; }
			cur = &slides[nslides++];
			memset(cur, 0, sizeof(*cur));
			cur->title = skip_blanks(s + 1);
			to_atari(cur->title);
			continue;
		}
		if (s[0] == ';')
			continue;
		if (s[0] == '@' && indent == 0) {
			if (keyword(s, "@note", &arg))
				continue;
			if (cur == NULL) {
				if (keyword(s, "@footer", &arg)) {
					to_atari(arg);
					footer = arg;
				} else if (keyword(s, "@log", &arg)) {
					want_log = 1;
				} else if (keyword(s, "@spacing", &arg)) {
					if (*arg >= '0' && *arg <= '9') {
						int n = atoi(arg);
						item_space = n > SPACING_MAX ? SPACING_MAX : n;
					}
				}
				continue;
			}
			if (keyword(s, "@image", &arg)) {
				cur->image = *arg ? arg : NULL;
			} else if (keyword(s, "@center", &arg)) {
				cur->center = 1;
			}
			continue;              /* unknown directives: ignored */
		}
		if (cur == NULL)
			continue;              /* free header before the first slide */
		if (*s == 0) {
			add_item(cur, IT_GAP, NULL);
			continue;
		}
		if ((s[0] == '-' || s[0] == '*') && (s[1] == ' ' || s[1] == 0)) {
			char *t = skip_blanks(s + 1);
			to_atari(t);
			add_item(cur, indent >= 2 ? IT_SUB : IT_BULLET, t);
			continue;
		}
		to_atari(s);
		add_item(cur, IT_TEXT, s);
	}
	/* no gap at the end of a slide */
	{
		int i;
		for (i = 0; i < nslides; i++)
			while (slides[i].nitems > 0 && slides[i].item[slides[i].nitems - 1].kind == IT_GAP)
				slides[i].nitems--;
	}
}

static char *read_file(const char *path, long max, long *outlen)
{
	FILE *f = fopen(path, "rb");
	char *buf;
	long n;

	if (!f)
		return NULL;
	buf = malloc(max + 1);
	if (!buf) { fclose(f); return NULL; }
	n = (long)fread(buf, 1, max, f);
	fclose(f);
	if (n < 0) { free(buf); return NULL; }
	buf[n] = 0;
	if (outlen) *outlen = n;
	return buf;
}

/* ------------------------------------------------------------------------ */
/* Frame buffer (16-bit RRRRRGGGGG0BBBBB, the 16-plane memory-MFDB format)  */

static short vh;
static int SW, SH;              /* screen size */
static int FBW;                 /* frame stride in pixels (multiple of 16) */
static int planes;              /* screen depth from vq_extnd: 16 or 32 */
static u16 *fb;
static u32 *fb32;               /* 32 planes: fb expanded to 0x00RRGGBB */

#define RGB(r, g, b) ((u16)((((r) >> 3) << 11) | (((g) >> 3) << 6) | ((b) >> 3)))

/* Colour scheme: dark blue background, white titles, Atari-orange accent. */
#define C_BG_R 16
#define C_BG_G 24
#define C_BG_B 48
static const u16 C_BG     = RGB(C_BG_R, C_BG_G, C_BG_B);
static const u16 C_TITLE  = RGB(255, 255, 255);
static const u16 C_ACCENT = RGB(240, 128, 48);
static const u16 C_TEXT   = RGB(232, 236, 244);
static const u16 C_SUB    = RGB(184, 196, 216);
static const u16 C_SUBMK  = RGB(144, 164, 192);
static const u16 C_DIM    = RGB(128, 144, 176);

static void fill_rect(int x, int y, int w, int h, u16 c)
{
	int x1 = x + w, y1 = y + h, yy;
	if (x < 0) x = 0;
	if (y < 0) y = 0;
	if (x1 > SW) x1 = SW;
	if (y1 > SH) y1 = SH;
	for (yy = y; yy < y1; yy++) {
		u16 *p = fb + (long)yy * FBW;
		int xx;
		for (xx = x; xx < x1; xx++)
			p[xx] = c;
	}
}

static void fill_disc(int cx, int cy, int r, u16 c)
{
	int dy;
	for (dy = -r; dy <= r; dy++) {
		int dx = 0;
		while ((dx + 1) * (dx + 1) + dy * dy <= r * r + r)
			dx++;
		fill_rect(cx - dx, cy + dy, 2 * dx + 1, 1, c);
	}
}

/* ------------------------------------------------------------------------ */
/* Glyphs                                                                    */

#define MAX_CW 16
#define MAX_CH 32
#define MAX_SCALE 8

static int cw, ch;                          /* system font cell */
static u8 base_glyph[256][MAX_CH][MAX_CW];  /* 1 = ink */
static u8 *gcache[MAX_SCALE + 1][256];

/* Scale2x / EPX: w x h -> 2w x 2h, outside the cell counts as background. */
static void scale2x(const u8 *s, int w, int h, u8 *d)
{
	int x, y, W = 2 * w;
	for (y = 0; y < h; y++) {
		for (x = 0; x < w; x++) {
			u8 E = s[y * w + x];
			u8 B = y > 0 ? s[(y - 1) * w + x] : 0;
			u8 H = y < h - 1 ? s[(y + 1) * w + x] : 0;
			u8 D = x > 0 ? s[y * w + x - 1] : 0;
			u8 F = x < w - 1 ? s[y * w + x + 1] : 0;
			u8 e0 = E, e1 = E, e2 = E, e3 = E;
			if (B != H && D != F) {
				e0 = D == B ? D : E;
				e1 = B == F ? F : E;
				e2 = D == H ? D : E;
				e3 = H == F ? F : E;
			}
			d[(2 * y) * W + 2 * x] = e0;
			d[(2 * y) * W + 2 * x + 1] = e1;
			d[(2 * y + 1) * W + 2 * x] = e2;
			d[(2 * y + 1) * W + 2 * x + 1] = e3;
		}
	}
}

/* Scale3x (AdvMAME3x): w x h -> 3w x 3h. */
static void scale3x(const u8 *s, int w, int h, u8 *d)
{
	int x, y, W = 3 * w;
#define PX(xx, yy) (((xx) < 0 || (yy) < 0 || (xx) >= w || (yy) >= h) ? 0 : s[(yy) * w + (xx)])
	for (y = 0; y < h; y++) {
		for (x = 0; x < w; x++) {
			u8 A = PX(x - 1, y - 1), B = PX(x, y - 1), C = PX(x + 1, y - 1);
			u8 D = PX(x - 1, y),     E = PX(x, y),     F = PX(x + 1, y);
			u8 G = PX(x - 1, y + 1), H = PX(x, y + 1), I = PX(x + 1, y + 1);
			u8 e[9];
			int i;
			for (i = 0; i < 9; i++)
				e[i] = E;
			if (B != H && D != F) {
				e[0] = D == B ? D : E;
				e[1] = ((D == B && E != C) || (B == F && E != A)) ? B : E;
				e[2] = B == F ? F : E;
				e[3] = ((D == B && E != G) || (D == H && E != A)) ? D : E;
				e[5] = ((B == F && E != I) || (H == F && E != C)) ? F : E;
				e[6] = D == H ? D : E;
				e[7] = ((D == H && E != I) || (H == F && E != G)) ? H : E;
				e[8] = H == F ? F : E;
			}
			for (i = 0; i < 9; i++)
				d[(3 * y + i / 3) * W + 3 * x + i % 3] = e[i];
		}
	}
#undef PX
}

static void nearest(const u8 *s, int w, int h, int k, u8 *d)
{
	int x, y;
	for (y = 0; y < h * k; y++)
		for (x = 0; x < w * k; x++)
			d[y * w * k + x] = s[(y / k) * w + x / k];
}

/* The glyph of c at scale k (1..MAX_SCALE): (cw*k) x (ch*k) bytes, or NULL
 * if memory ran out (the character is then left out). */
static const u8 *glyph(int c, int k)
{
	u8 *g, *t1 = NULL, *t2 = NULL;
	u8 b[MAX_CH * MAX_CW];
	int x, y;

	c &= 0xff;
	if (k < 1) k = 1;
	if (k > MAX_SCALE) k = MAX_SCALE;
	if (gcache[k][c])
		return gcache[k][c];
	for (y = 0; y < ch; y++)
		for (x = 0; x < cw; x++)
			b[y * cw + x] = base_glyph[c][y][x];
	g = malloc((size_t)cw * k * ch * k);
	if (!g)
		return NULL;
	switch (k) {
	case 1: memcpy(g, b, cw * ch); break;
	case 2: scale2x(b, cw, ch, g); break;
	case 3: scale3x(b, cw, ch, g); break;
	case 4:
		t1 = malloc(cw * 2 * ch * 2);
		if (!t1) { free(g); return NULL; }
		scale2x(b, cw, ch, t1);
		scale2x(t1, cw * 2, ch * 2, g);
		break;
	case 6:
		t1 = malloc(cw * 3 * ch * 3);
		if (!t1) { free(g); return NULL; }
		scale3x(b, cw, ch, t1);
		scale2x(t1, cw * 3, ch * 3, g);
		break;
	case 8:
		t1 = malloc(cw * 2 * ch * 2);
		t2 = malloc(cw * 4 * ch * 4);
		if (!t1 || !t2) { free(t1); free(t2); free(g); return NULL; }
		scale2x(b, cw, ch, t1);
		scale2x(t1, cw * 2, ch * 2, t2);
		scale2x(t2, cw * 4, ch * 4, g);
		break;
	default: nearest(b, cw, ch, k, g); break;
	}
	free(t1);
	free(t2);
	gcache[k][c] = g;
	return g;
}

static void draw_glyph(int x, int y, int c, int k, u16 col)
{
	const u8 *g = glyph(c, k);
	int gw = cw * k, gh = ch * k, yy;
	if (!g)
		return;
	for (yy = 0; yy < gh; yy++) {
		int sy = y + yy, xx;
		const u8 *row;
		u16 *p;
		if (sy < 0 || sy >= SH)
			continue;
		row = g + yy * gw;
		p = fb + (long)sy * FBW;
		for (xx = 0; xx < gw; xx++) {
			int sx = x + xx;
			if (row[xx] && sx >= 0 && sx < SW)
				p[sx] = col;
		}
	}
}

static void draw_str(int x, int y, int k, u16 col, const char *s, int n)
{
	int i;
	for (i = 0; i < n; i++, x += cw * k)
		if ((u8)s[i] != ' ')
			draw_glyph(x, y, (u8)s[i], k, col);
}

/* Pixel i of a capture buffer in the screen's memory-MFDB form. At 32 planes
 * the X byte is masked off (ADR-038 point 4: it carries no colour). */
static u32 cap_pixel(const void *buf, long i)
{
	if (planes == 32)
		return ((const u32 *)buf)[i] & 0x00FFFFFFu;
	return ((const u16 *)buf)[i];
}

/* Read the system font: draw the cells 32..255 with v_gtext (black on
 * white, top-aligned) in the top-left corner and copy them back. */
static int capture_font(void)
{
	short attr[10], d, pxy[4], p8[8];
	int capw, caph, r, k, c, x, y, inked = 0;
	void *buf;
	u32 bgv;
	MFDB mem, scr;
	char s[33];

	vqt_attributes(vh, attr);
	cw = attr[8];
	ch = attr[9];
	if (cw < 4 || cw > MAX_CW || ch < 6 || ch > MAX_CH)
		return 0;
	capw = 32 * cw;
	caph = 7 * ch;
	/* a row is fd_wdwidth * 16 pixels of planes / 8 bytes */
	buf = malloc((size_t)((capw + 15) & ~15) * caph * (planes / 8));
	if (!buf)
		return 0;

	vswr_mode(vh, MD_REPLACE);
	vst_alignment(vh, 0, 5, &d, &d);
	vst_effects(vh, 0);
	vst_rotation(vh, 0);
	vst_color(vh, 1);
	pxy[0] = 0; pxy[1] = 0; pxy[2] = capw - 1; pxy[3] = caph - 1;
	vs_clip(vh, 0, pxy);
	vsf_interior(vh, 1);
	vsf_color(vh, 0);
	vsf_perimeter(vh, 0);
	v_bar(vh, pxy);
	for (r = 0; r < 7; r++) {
		for (k = 0; k < 32; k++)
			s[k] = (char)(32 + r * 32 + k);
		s[32] = 0;
		v_gtext(vh, 0, r * ch, s);
	}
	memset(&mem, 0, sizeof(mem));
	memset(&scr, 0, sizeof(scr));
	mem.fd_addr = buf;
	mem.fd_w = capw;
	mem.fd_h = caph;
	mem.fd_wdwidth = (capw + 15) / 16;
	mem.fd_stand = 0;
	mem.fd_nplanes = planes;
	p8[0] = 0; p8[1] = 0; p8[2] = capw - 1; p8[3] = caph - 1;
	p8[4] = 0; p8[5] = 0; p8[6] = capw - 1; p8[7] = caph - 1;
	vro_cpyfm(vh, S_ONLY, p8, &scr, &mem);

	bgv = cap_pixel(buf, 0);      /* top-left pixel of the ' ' cell */
	memset(base_glyph, 0, sizeof(base_glyph));
	for (c = 32; c < 256; c++) {
		int row = (c - 32) / 32, col = (c - 32) % 32;
		for (y = 0; y < ch; y++)
			for (x = 0; x < cw; x++) {
				u32 v = cap_pixel(buf, (long)(row * ch + y) * ((capw + 15) & ~15) + col * cw + x);
				if (v != bgv) {
					base_glyph[c][y][x] = 1;
					inked++;
				}
			}
	}
	free(buf);
	/* sanity: something was drawn, and 'A' has ink in its middle row */
	if (inked == 0)
		return 0;
	for (x = 0; x < cw; x++)
		if (base_glyph['A'][ch / 2][x])
			return 1;
	return 0;
}

/* ------------------------------------------------------------------------ */
/* Text layout                                                               */

/* Next line of t (n chars) at most maxc chars wide: returns its length
 * (without trailing blanks), *adv = chars consumed incl. the break blanks. */
static int wrap_line(const char *t, int n, int maxc, int *adv)
{
	int i, e, a;
	if (maxc < 1) maxc = 1;
	if (n <= maxc) { *adv = n; return n; }
	for (i = maxc; i > 0; i--)
		if (t[i] == ' ')
			break;
	if (i == 0) {            /* one word longer than the line: hard break */
		*adv = maxc;
		return maxc;
	}
	e = i;
	while (e > 0 && t[e - 1] == ' ') e--;
	a = i;
	while (a < n && t[a] == ' ') a++;
	*adv = a;
	return e;
}

/* Lay out (draw != 0: also draw) text in [x0, x1) from y with scale k.
 * align 0 = left, 1 = centred. Returns the height used. */
static int text_block(const char *t, int k, int x0, int x1, int y, int lh,
                      int align, u16 col, int draw, int ylimit)
{
	int n = (int)strlen(t), h = 0;
	int maxc = (x1 - x0) / (cw * k);
	while (n > 0) {
		int adv, len = wrap_line(t, n, maxc, &adv);
		if (draw && y + h + ch * k <= ylimit) {
			int x = x0;
			if (align == 1)
				x = x0 + ((x1 - x0) - len * cw * k) / 2;
			draw_str(x, y + h, k, col, t, len);
		}
		h += lh;
		t += adv;
		n -= adv;
		if (adv == 0)
			break;
	}
	return h;
}

static int lineh(int k) { return ch * k + (ch * k) / 8; }

/* Is this an allowed scale (Scale2x/3x family, or 5/7 nearest as last
 * resort -- skipped while shrinking). */
static int next_smaller(int k)
{
	switch (k) {
	case 8: return 6;
	case 6: return 4;
	default: return k - 1;
	}
}

static int clampi(int v, int lo, int hi) { return v < lo ? lo : v > hi ? hi : v; }

/* Body items of a slide; returns the height. */
static int items_block(Slide *sl, int B, int x0, int x1, int y0, int draw,
                       int center, int ylimit)
{
	/* sub-bullets one step smaller from size 3 up; below that the same size
	 * (indent, dash and colour mark them) -- size 1 would be too small */
	int S = B >= 3 ? B - 1 : B;
	int y = y0, i;
	for (i = 0; i < sl->nitems; i++) {
		Item *it = &sl->item[i];
		int k = it->kind == IT_SUB ? S : B;
		int lh = lineh(k);
		if (it->kind == IT_GAP) {
			y += (ch * B) / 2;
			continue;
		}
		if (center) {
			u16 col = it->kind == IT_SUB ? C_SUB : C_TEXT;
			y += text_block(it->text, k, x0, x1, y, lh, 1, col, draw, ylimit);
		} else if (it->kind == IT_BULLET) {
			int tx = x0 + cw * B * 2;
			if (draw && y + ch * B <= ylimit) {
				int r = (B * 5) / 2 + 1;
				fill_disc(x0 + cw * B * 3 / 4, y + (ch * B * 19) / 32, r, C_ACCENT);
			}
			y += text_block(it->text, B, tx, x1, y, lh, 0, C_TEXT, draw, ylimit);
		} else if (it->kind == IT_SUB) {
			int mx = x0 + cw * B * 2;
			int tx = mx + cw * S * 2;
			if (draw && y + ch * S <= ylimit)
				fill_rect(mx + cw * S / 4, y + (ch * S * 19) / 32 - S / 2 - 1,
				          cw * S, S + 1 > 2 ? S + 1 : 2, C_SUBMK);
			y += text_block(it->text, S, tx, x1, y, lh, 0, C_SUB, draw, ylimit);
		} else {
			y += text_block(it->text, B, x0, x1, y, lh, 0, C_TEXT, draw, ylimit);
		}
		y += (ch * k * item_space) / 12;   /* space after an item (@spacing) */
	}
	return y - y0;
}

/* ------------------------------------------------------------------------ */
/* Pictures                                                                  */

static void png_err(png_structp png, png_const_charp msg)
{
	(void)msg;
	longjmp(png_jmpbuf(png), 1);
}

static void png_warn(png_structp png, png_const_charp msg)
{
	(void)png;
	(void)msg;
}

static void make_path(const char *name, char *out, size_t n)
{
	char *p;
	if (strchr(name, ':') || name[0] == '\\' || name[0] == '/')
		snprintf(out, n, "%s", name);
	else
		snprintf(out, n, "%s%s", basedir, name);
	for (p = out; *p; p++)
		if (*p == '/')
			*p = '\\';
}

/* Decode a PNG to RGBA8. NULL + why on failure. */
static u8 *load_png(const char *path, int *pw, int *ph, char *why, size_t wl)
{
	FILE *f;
	u8 sig[8];
	png_structp png;
	png_infop info;
	u8 *volatile pix = NULL;
	png_bytep *volatile rows = NULL;
	png_uint_32 w, h;
	int ct, bd, y;
	size_t rb;

	f = fopen(path, "rb");
	if (!f) {
		snprintf(why, wl, "picture not found");
		return NULL;
	}
	if (fread(sig, 1, 8, f) != 8 || png_sig_cmp(sig, 0, 8)) {
		fclose(f);
		snprintf(why, wl, "not a PNG picture");
		return NULL;
	}
	png = png_create_read_struct(PNG_LIBPNG_VER_STRING, NULL, png_err, png_warn);
	if (!png) { fclose(f); snprintf(why, wl, "out of memory"); return NULL; }
	info = png_create_info_struct(png);
	if (!info) {
		png_destroy_read_struct(&png, NULL, NULL);
		fclose(f);
		snprintf(why, wl, "out of memory");
		return NULL;
	}
	if (setjmp(png_jmpbuf(png))) {
		free(rows);
		free(pix);
		png_destroy_read_struct(&png, &info, NULL);
		fclose(f);
		snprintf(why, wl, "cannot decode picture");
		return NULL;
	}
	png_init_io(png, f);
	png_set_sig_bytes(png, 8);
	png_set_user_limits(png, 8192, 8192);
	png_read_info(png, info);
	w = png_get_image_width(png, info);
	h = png_get_image_height(png, info);
	ct = png_get_color_type(png, info);
	bd = png_get_bit_depth(png, info);
	if ((unsigned long)w * h > 16UL * 1024 * 1024)
		png_error(png, "too large");
	if (bd == 16)
		png_set_strip_16(png);
	png_set_expand(png);
	if (ct == PNG_COLOR_TYPE_GRAY || ct == PNG_COLOR_TYPE_GRAY_ALPHA)
		png_set_gray_to_rgb(png);
	if (!(ct & PNG_COLOR_MASK_ALPHA) && !png_get_valid(png, info, PNG_INFO_tRNS))
		png_set_filler(png, 0xff, PNG_FILLER_AFTER);
	png_set_interlace_handling(png);
	png_read_update_info(png, info);
	rb = png_get_rowbytes(png, info);
	if (rb != (size_t)w * 4)
		png_error(png, "unexpected row format");
	pix = malloc(rb * h);
	rows = malloc(sizeof(png_bytep) * h);
	if (!pix || !rows)
		png_error(png, "out of memory");
	for (y = 0; y < (int)h; y++)
		rows[y] = pix + rb * y;
	png_read_image(png, rows);
	png_read_end(png, NULL);
	png_destroy_read_struct(&png, &info, NULL);
	fclose(f);
	free(rows);
	*pw = (int)w;
	*ph = (int)h;
	return pix;
}

static u16 blend565(const u8 *p)
{
	unsigned a = p[3];
	unsigned r = (p[0] * a + C_BG_R * (255 - a)) / 255;
	unsigned g = (p[1] * a + C_BG_G * (255 - a)) / 255;
	unsigned b = (p[2] * a + C_BG_B * (255 - a)) / 255;
	return RGB(r, g, b);
}

/* Fit w x h RGBA into aw x ah: whole-factor enlargement (nearest) or box
 * filter reduction. */
static u16 *fit_picture(const u8 *src, int w, int h, int aw, int ah, int *ow, int *oh)
{
	int kx = aw / w, ky = ah / h, k = kx < ky ? kx : ky;
	u16 *out;
	int x, y;

	if (k >= 1) {
		*ow = w * k;
		*oh = h * k;
		out = malloc((size_t)*ow * *oh * 2);
		if (!out) return NULL;
		for (y = 0; y < *oh; y++)
			for (x = 0; x < *ow; x++)
				out[(long)y * *ow + x] = blend565(src + ((long)(y / k) * w + x / k) * 4);
		return out;
	}
	/* reduce: keep the aspect ratio */
	if ((long)aw * h <= (long)ah * w) {
		*ow = aw;
		*oh = (int)((long)h * aw / w);
	} else {
		*oh = ah;
		*ow = (int)((long)w * ah / h);
	}
	if (*ow < 1) *ow = 1;
	if (*oh < 1) *oh = 1;
	out = malloc((size_t)*ow * *oh * 2);
	if (!out) return NULL;
	for (y = 0; y < *oh; y++) {
		int y0 = (int)((long)y * h / *oh), y1 = (int)((long)(y + 1) * h / *oh);
		if (y1 <= y0) y1 = y0 + 1;
		for (x = 0; x < *ow; x++) {
			int x0 = (int)((long)x * w / *ow), x1 = (int)((long)(x + 1) * w / *ow);
			unsigned long sr = 0, sg = 0, sb = 0, sa = 0, n = 0;
			int yy, xx;
			u8 px[4];
			if (x1 <= x0) x1 = x0 + 1;
			for (yy = y0; yy < y1; yy++)
				for (xx = x0; xx < x1; xx++) {
					const u8 *p = src + ((long)yy * w + xx) * 4;
					sr += p[0] * p[3];
					sg += p[1] * p[3];
					sb += p[2] * p[3];
					sa += p[3];
					n++;
				}
			if (sa) {
				px[0] = (u8)(sr / sa);
				px[1] = (u8)(sg / sa);
				px[2] = (u8)(sb / sa);
			} else {
				px[0] = px[1] = px[2] = 0;
			}
			px[3] = (u8)(sa / n);
			out[(long)y * *ow + x] = blend565(px);
		}
	}
	return out;
}

static void prepare_picture(Slide *sl, int aw, int ah)
{
	char path[256];
	u8 *rgba;
	int w = 0, h = 0;
	long t0 = get_sysvar(_hz_200);

	if (sl->pic_state != 0 && sl->aw == aw && sl->ah == ah)
		return;
	free(sl->pic);
	sl->pic = NULL;
	sl->aw = aw;
	sl->ah = ah;
	sl->pic_state = -1;
	make_path(sl->image, path, sizeof(path));
	rgba = load_png(path, &w, &h, sl->why, sizeof(sl->why));
	if (rgba) {
		sl->pic = fit_picture(rgba, w, h, aw, ah, &sl->pw, &sl->ph);
		free(rgba);
		if (sl->pic)
			sl->pic_state = 1;
		else
			snprintf(sl->why, sizeof(sl->why), "out of memory");
	}
	sl->t_image = get_sysvar(_hz_200) - t0;
}

static void draw_picture(Slide *sl, int ax, int ay, int aw, int ah, int fs)
{
	if (aw < 16 || ah < 16)
		return;
	prepare_picture(sl, aw, ah);
	if (sl->pic_state == 1) {
		int ox = ax + (aw - sl->pw) / 2, oy = ay + (ah - sl->ph) / 2, y;
		for (y = 0; y < sl->ph; y++) {
			int sy = oy + y;
			if (sy < 0 || sy >= SH)
				continue;
			memcpy(fb + (long)sy * FBW + ox, sl->pic + (long)y * sl->pw, (size_t)sl->pw * 2);
		}
	} else {
		char msg[200];
		int hgt;
		snprintf(msg, sizeof(msg), "[%s: %s]", sl->why, sl->image);
		to_atari(msg);
		hgt = text_block(msg, fs, ax, ax + aw, 0, lineh(fs), 1, C_DIM, 0, SH);
		text_block(msg, fs, ax, ax + aw, ay + (ah - hgt) / 2, lineh(fs), 1, C_DIM, 1, ay + ah);
	}
}

/* ------------------------------------------------------------------------ */
/* Slide rendering                                                           */

static int title_lines(const char *t, int k, int w)
{
	int n = (int)strlen(t), c = 0, maxc = w / (cw * k);
	while (n > 0) {
		int adv;
		wrap_line(t, n, maxc, &adv);
		c++;
		t += adv;
		n -= adv;
		if (adv == 0) break;
	}
	return c;
}

static void render_slide(int idx)
{
	Slide *sl = &slides[idx];
	int mx = SW / 20, my = SH / 18;
	int x0 = mx, x1 = SW - mx;
	int T = clampi((SH * 4 + 540) / 1080, 1, MAX_SCALE);
	int Bpref = clampi((SH * 3 + 540) / 1080, 2, MAX_SCALE);
	int fs = clampi((SH * 3 / 2 + 540) / 1080, 1, 4);
	int bt = SH / 360 > 2 ? SH / 360 : 2;
	int fy = SH - my / 2 - ch * fs;           /* footer text top */
	int cy1 = fy - SH / 60;                   /* content bottom */
	int has_text = sl->nitems > 0;
	int B, y;
	char num[32];
	long t0 = get_sysvar(_hz_200);

	fill_rect(0, 0, SW, SH, C_BG);

	if (sl->center) {
		int Tc = clampi(T + 2, 1, MAX_SCALE), th, ih, block, ytop, pic_h;
		if (Tc == 5) Tc = 4;
		if (Tc == 7) Tc = 6;
		while (Tc > 1 && (title_lines(sl->title, Tc, x1 - x0) > 2))
			Tc = next_smaller(Tc);
		th = sl->title[0] ? title_lines(sl->title, Tc, x1 - x0) * lineh(Tc) : 0;
		B = Bpref;
		for (;;) {
			ih = items_block(sl, B, x0, x1, 0, 0, 1, SH);
			block = th + (th ? SH / 30 + bt + SH / 30 : 0) + ih;
			if (block <= (cy1 - my) * (sl->image ? 1 : 2) / 2 || B == 1)
				break;
			B = next_smaller(B);
		}
		if (sl->image) {
			ytop = my;
			pic_h = cy1 - (ytop + block) - SH / 30;
		} else {
			ytop = my + ((cy1 - my) - block) / 2;
			pic_h = 0;
		}
		y = ytop;
		if (th) {
			text_block(sl->title, Tc, x0, x1, y, lineh(Tc), 1, C_TITLE, 1, SH);
			y += th + SH / 30;
			fill_rect(SW / 2 - SW / 10, y, SW / 5, bt, C_ACCENT);
			y += bt + SH / 30;
		}
		items_block(sl, B, x0, x1, y, 1, 1, cy1);
		if (sl->image && pic_h > 0)
			draw_picture(sl, x0, y + ih + SH / 30, x1 - x0, pic_h, fs);
	} else {
		int tw = x1 - x0, picx = 0, picw = 0, cy0, txt_x1 = x1;
		y = my;
		if (sl->title[0]) {
			while (T > 1 && title_lines(sl->title, T, tw) > 2)
				T = next_smaller(T);
			y += text_block(sl->title, T, x0, x1, y, lineh(T), 0, C_TITLE, 1, SH);
			y += (ch * T) / 4;
			fill_rect(x0, y, tw, bt, C_ACCENT);
			y += bt;
		}
		cy0 = y + SH / 25;
		if (sl->image && has_text) {
			int textw = tw * 56 / 100, gap = tw * 4 / 100;
			txt_x1 = x0 + textw;
			picx = x0 + textw + gap;
			picw = tw - textw - gap;
		} else if (sl->image) {
			picx = x0;
			picw = tw;
		}
		B = Bpref;
		while (B > 1 && items_block(sl, B, x0, txt_x1, cy0, 0, 0, SH) > cy1 - cy0)
			B = next_smaller(B);
		items_block(sl, B, x0, txt_x1, cy0, 1, 0, cy1);
		if (sl->image)
			draw_picture(sl, picx, cy0, picw, cy1 - cy0, fs);
	}

	/* footer: slide number right, optional text left */
	snprintf(num, sizeof(num), "%d / %d", idx + 1, nslides);
	draw_str(x1 - (int)strlen(num) * cw * fs, fy, fs, C_DIM, num, (int)strlen(num));
	if (footer && *footer) {
		int maxc = (x1 - x0) / (cw * fs) - (int)strlen(num) - 4;
		int n = (int)strlen(footer);
		if (maxc > 0)
			draw_str(x0, fy, fs, C_DIM, footer, n < maxc ? n : maxc);
	}
	sl->t_render = get_sysvar(_hz_200) - t0 - sl->t_image;
}

/* 5 -> 8 bits by bit replication: the expansion the 16-plane device applies
 * to a memory word (kernel/src/vdi_backend.rs word_to_device, expand5to8),
 * so a frame shows the same store pixels at 32 planes as at 16. */
#define EXPAND5(c) ((u32)(((c) << 3) | ((c) >> 2)))

/* 32 planes: fb (RRRRRGGGGG0BBBBB) -> fb32 (0x00RRGGBB), the visible
 * columns of every row; bit 5 of the word is never set by RGB(). */
static void expand_frame(void)
{
	int x, y;
	for (y = 0; y < SH; y++) {
		const u16 *s = fb + (long)y * FBW;
		u32 *d = fb32 + (long)y * FBW;
		for (x = 0; x < SW; x++) {
			u32 v = s[x];
			d[x] = (EXPAND5((v >> 11) & 31) << 16) |
			       (EXPAND5((v >> 6) & 31) << 8) |
			       EXPAND5(v & 31);
		}
	}
}

static void blit_frame(void)
{
	MFDB src, dst;
	short p[8];
	memset(&src, 0, sizeof(src));
	memset(&dst, 0, sizeof(dst));
	if (planes == 32) {
		expand_frame();
		src.fd_addr = fb32;
	} else {
		src.fd_addr = fb;
	}
	src.fd_w = SW;
	src.fd_h = SH;
	src.fd_wdwidth = FBW / 16;
	src.fd_stand = 0;
	src.fd_nplanes = planes;
	p[0] = 0; p[1] = 0; p[2] = SW - 1; p[3] = SH - 1;
	p[4] = 0; p[5] = 0; p[6] = SW - 1; p[7] = SH - 1;
	vro_cpyfm(vh, S_ONLY, p, &src, &dst);
}

static void show(int idx)
{
	long t0;
	slides[idx].t_image = 0;
	render_slide(idx);
	t0 = get_sysvar(_hz_200);
	blit_frame();
	slides[idx].t_blit = get_sysvar(_hz_200) - t0;
	slides[idx].shown++;
	if (slides[idx].t_render > slides[idx].m_render) slides[idx].m_render = slides[idx].t_render;
	if (slides[idx].t_image > slides[idx].m_image) slides[idx].m_image = slides[idx].t_image;
	if (slides[idx].t_blit > slides[idx].m_blit) slides[idx].m_blit = slides[idx].t_blit;
}

static void write_log(void)
{
	char path[256];
	FILE *f;
	int i;
	snprintf(path, sizeof(path), "%sPRESENT.LOG", basedir);
	f = fopen(path, "w");
	if (!f)
		return;
	fprintf(f, "PRESENT %dx%d, font cell %dx%d, %d slides from %s\r\n",
	        SW, SH, cw, ch, nslides, slidepath);
	fprintf(f, "longest time over all showings, ms (200 Hz clock, 5 ms steps; the picture\r\n"
	           "is decoded and scaled once, later showings use the cached copy)\r\n");
	for (i = 0; i < nslides; i++)
		fprintf(f, "slide %d: shown %ld x, render %ld ms, picture %ld ms, blit %ld ms%s%s\r\n",
		        i + 1, slides[i].shown, slides[i].m_render * 5, slides[i].m_image * 5,
		        slides[i].m_blit * 5, slides[i].image ? ", picture: " : "",
		        slides[i].image ? (slides[i].pic_state == 1 ? "ok" : slides[i].why) : "");
	fclose(f);
}

/* ------------------------------------------------------------------------ */

static int load_slides(int argc, char **argv)
{
	static const char *cand[] = { "SLIDES.TXT", "C:\\APPS\\PRESENT\\SLIDES.TXT" };
	char *text = NULL;
	char *p;
	int i;

	if (argc > 1 && argv[1] && argv[1][0]) {
		snprintf(slidepath, sizeof(slidepath), "%s", argv[1]);
		text = read_file(slidepath, MAX_FILE, NULL);
	}
	for (i = 0; !text && i < 2; i++) {
		snprintf(slidepath, sizeof(slidepath), "%s", cand[i]);
		text = read_file(slidepath, MAX_FILE, NULL);
	}
	if (!text)
		return -1;
	snprintf(basedir, sizeof(basedir), "%s", slidepath);
	p = strrchr(basedir, '\\');
	if (!p) p = strrchr(basedir, ':');
	if (p) p[1] = 0; else basedir[0] = 0;
	parse_slides(text);
	return nslides;
}

int main(int argc, char **argv)
{
	short work_in[11] = { 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2 };
	short work_out[57];
	short wc, hc, wb, hb;
	int cur = 0, quit = 0, btn_down = 0, n;

	if (appl_init() < 0)
		return 1;
	vh = graf_handle(&wc, &hc, &wb, &hb);
	v_opnvwk(work_in, &vh, work_out);
	if (vh <= 0) {
		form_alert(1, "[3][PRESENT: cannot open|a VDI workstation.][ Quit ]");
		appl_exit();
		return 1;
	}
	SW = work_out[0] + 1;
	SH = work_out[1] + 1;
	FBW = (SW + 15) & ~15;
	vq_extnd(vh, 1, work_out);
	planes = work_out[4];
	if (planes != 16 && planes != 32) {
		form_alert(1, "[3][PRESENT needs a screen|of 16 or 32 planes.][ Quit ]");
		goto out_vdi;
	}

	n = load_slides(argc, argv);
	if (n < 0) {
		form_alert(1, "[3][PRESENT: no slide file.|Looked for SLIDES.TXT here|and C:\\APPS\\PRESENT\\SLIDES.TXT][ Quit ]");
		goto out_vdi;
	}
	if (n == 0) {
		form_alert(1, "[3][PRESENT: the slide file has|no slides (lines \"# Title\").][ Quit ]");
		goto out_vdi;
	}
	fb = malloc((size_t)FBW * SH * 2);
	if (fb && planes == 32) {
		fb32 = malloc((size_t)FBW * SH * 4);
		if (!fb32) {
			free(fb);
			fb = NULL;
		}
	}
	if (!fb) {
		form_alert(1, "[3][PRESENT: not enough memory|for the screen buffer.][ Quit ]");
		goto out_vdi;
	}

	wind_update(BEG_UPDATE);
	wind_update(BEG_MCTRL);
	form_dial(FMD_START, 0, 0, 0, 0, 0, 0, SW, SH);
	graf_mouse(M_OFF, NULL);

	if (!capture_font()) {
		graf_mouse(M_ON, NULL);
		form_dial(FMD_FINISH, 0, 0, 0, 0, 0, 0, SW, SH);
		wind_update(END_MCTRL);
		wind_update(END_UPDATE);
		form_alert(1, "[3][PRESENT: cannot read|the system font.][ Quit ]");
		goto out_vdi;
	}

	show(cur);
	while (!quit) {
		short mx, my, mb, ks, kr, br, msg[8];
		int next = cur;
		short ev = evnt_multi(MU_KEYBD | MU_BUTTON | MU_MESAG,
		                      btn_down ? 1 : 0x101, 3, 0,
		                      0, 0, 0, 0, 0,
		                      0, 0, 0, 0, 0,
		                      msg, 0L,
		                      &mx, &my, &mb, &ks, &kr, &br);
		if (ev & MU_MESAG) {
			if (msg[0] == AP_TERM)
				quit = 1;
		}
		if (ev & MU_BUTTON) {
			if (btn_down) {
				btn_down = 0;           /* released */
			} else {
				btn_down = 1;
				if (mb & 2)
					next = cur - 1;
				else
					next = cur + 1;
			}
		}
		if (ev & MU_KEYBD) {
			int scan = (kr >> 8) & 0xff, asc = kr & 0xff;
			switch (scan) {
			case 0x4d: case 0x50: case 0x39: case 0x1c: case 0x72:
				next = cur + 1; break;           /* Right Down Space Return Enter */
			case 0x4b: case 0x48: case 0x0e:
				next = cur - 1; break;           /* Left Up Backspace */
			case 0x47:                           /* ClrHome, Shift = end */
				next = (ks & 3) ? nslides - 1 : 0; break;
			case 0x01:
				quit = 1; break;                 /* Esc */
			default:
				if (asc == 'q' || asc == 'Q') quit = 1;
				else if (asc == 'n' || asc == 'N') next = cur + 1;
				else if (asc == 'p' || asc == 'P') next = cur - 1;
				break;
			}
		}
		if (quit)
			break;
		next = clampi(next, 0, nslides - 1);
		if (next != cur) {
			cur = next;
			show(cur);
		}
	}

	graf_mouse(M_ON, NULL);
	form_dial(FMD_FINISH, 0, 0, 0, 0, 0, 0, SW, SH);
	wind_update(END_MCTRL);
	wind_update(END_UPDATE);
	if (want_log)
		write_log();

out_vdi:
	v_clsvwk(vh);
	appl_exit();
	return 0;
}
