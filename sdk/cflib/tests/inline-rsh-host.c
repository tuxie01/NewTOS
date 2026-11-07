/*
 * inline-rsh-host.c -- host test for cflib's built-in images (TASK/A142).
 *
 * cflib's inline.rsh holds the images of the check box, the radio button and
 * the alert icons. Upstream writes them as tables of BYTES in m68k order and
 * casts them to short *. VDI and AES read an MFDB or a BITBLK as native 16-bit
 * words with bit 15 the leftmost pixel, so on AArch64 the two 8-pixel halves
 * of every word traded places. The overlay holds them as tables of WORDS.
 *
 * The same file is compiled three times by inline-rsh-host.sh:
 *   -DSIDE=side_pristine  around the upstream inline.rsh, from the tarball
 *   -DSIDE=side_port      around the file under test
 *   no SIDE               the comparison, main()
 *
 * Each side walks the tables through the resource's own structures, the way
 * cflib reaches them: rs_bitblk[].bi_pdata, and for every rs_ciconblk[] the
 * mono mask and data plus each CICON of the mainlist chain. The word the VDI
 * sees in the port (native read) has to equal the word m68k saw in the upstream
 * bytes (big-endian read), in every table, and the table has to be word-aligned.
 *
 * Exit status: the number of failed checks (at most 76); 77 = skipped.
 */
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

struct raster {
	const char *name;
	const unsigned char *data;
	size_t len;
};

#define MAX_RASTERS 128

#ifdef SIDE

#include <mt_gem.h>
#include INLINE_RSH

static struct raster tab[MAX_RASTERS];
static size_t n;

static void add(const char *name, const void *data, size_t len)
{
	if (n < MAX_RASTERS)
	{
		tab[n].name = name;
		tab[n].data = data;
		tab[n].len = len;
		n++;
	}
}

size_t SIDE(const struct raster **out)
{
	size_t i;

	n = 0;
	for (i = 0; i < NUM_BB; i++)
		add("bitblk", rs_bitblk[i].bi_pdata, (size_t)rs_bitblk[i].bi_wb * rs_bitblk[i].bi_hl);
	for (i = 0; i < NUM_CIB; i++)
	{
		const CICONBLK *cib = &rs_ciconblk[i];
		const CICON *c;
		size_t len = (size_t)(cib->monoblk.ib_wicon / 8) * cib->monoblk.ib_hicon;

		add("mono mask", cib->monoblk.ib_pmask, len);
		add("mono data", cib->monoblk.ib_pdata, len);
		for (c = cib->mainlist; c != NULL; c = c->next_res)
		{
			add("col_data", c->col_data, len * c->num_planes);
			add("col_mask", c->col_mask, len);
			add("sel_data", c->sel_data, len * c->num_planes);
			add("sel_mask", c->sel_mask, len);
		}
	}
	*out = tab;
	return n;
}

#else /* SIDE */

size_t side_pristine(const struct raster **out);
size_t side_port(const struct raster **out);

static int failed, checks;

static void check(int ok, const char *what, size_t index, const char *name)
{
	checks++;
	if (!ok)
	{
		failed++;
		printf("FAIL  table %zu (%s): %s\n", index, name, what);
	}
}

static uint16_t native_word(const unsigned char *p)
{
	uint16_t w;

	memcpy(&w, p, sizeof(w));
	return w;
}

int main(void)
{
	const struct raster *up, *port;
	size_t nup = side_pristine(&up);
	size_t nport = side_port(&port);
	size_t i, j;
	const uint16_t probe = 0x0102;

	if (sizeof(long) != 8 || *(const unsigned char *)&probe != 0x02)
	{
		printf("skipped -- host is not LP64 little-endian\n");
		return 77;
	}

	/* 4 BITBLK, 4 CICONBLK with mono mask + data + 2 CICON of 4 tables: 44 tables */
	check(nup == 44, "the upstream resource has 44 tables", nup, "count");
	check(nport == nup, "the port has as many tables as upstream", nport, "count");

	for (i = 0; i < nup && i < nport; i++)
	{
		int same = 1;

		check(port[i].len == up[i].len && up[i].len != 0 && (up[i].len & 1) == 0,
		      "same even length", i, up[i].name);
		check(((uintptr_t)port[i].data & 1) == 0, "word-aligned", i, up[i].name);
		if (port[i].len != up[i].len)
			continue;
		for (j = 0; j + 1 < up[i].len; j += 2)
		{
			unsigned expect = ((unsigned)up[i].data[j] << 8) | up[i].data[j + 1];

			if (native_word(port[i].data + j) != expect)
				same = 0;
		}
		check(same, "the native word is upstream's big-endian word", i, up[i].name);
	}

	/*
	 * The same statement without the upstream table: rows of the "!" alert
	 * icon (second BITBLK) and of the radio button's mask (first CICONBLK),
	 * as words with bit 15 the leftmost pixel, read off the ORCS output.
	 */
	if (nport > 12)
	{
		check(native_word(port[1].data + 4) == 0x3FFF, "\"!\" icon row 1, left word", 1, port[1].name);
		check(native_word(port[1].data + 6) == 0xFFFC, "\"!\" icon row 1, right word", 1, port[1].name);
		check(native_word(port[4].data + 2) == 0x03C0, "radio mask row 1", 4, port[4].name);
	}

	printf("%d checks, %d failed\n", checks, failed);
	return failed > 76 ? 76 : failed;
}

#endif /* SIDE */
