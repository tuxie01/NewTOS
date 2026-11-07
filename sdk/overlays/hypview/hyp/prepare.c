/*
 * NewTOS patch of HypView hyp/prepare.c (freemint/hypview @ 6f3eeeb).
 *
 * Why: upstream implements reduce() and mono_bitmap() as m68k inline
 * assembler under `#ifdef __GNUC__` (clang defines __GNUC__ too, and the
 * AArch64 assembler rejects `a0`), or as the Pure C module hyp/gfx.s.
 * This file is upstream verbatim except that block, which is replaced by
 * the same two functions in C (m68k asm retarget):
 *
 *  - reduce(): 4 planes -> 1 plane in place, per pixel bit
 *    (_16to2 >> pixelvalue) & 1, the loop running over the whole first
 *    plane as gfx.s does (`cmpa.l A0,A4 / bne`; the GNU inline version's
 *    `blt` after `cmpa.l a0,a4` would stop after the first word).
 *  - mono_bitmap(): one 1-plane mask of the pixels equal to `color`.
 *
 * Byte order: the source planes are the picture data as read from the
 * .HYP file (standard format, big-endian 16-bit words), so they are read
 * byte-wise as big-endian words. reduce() writes its result back into that
 * standard-format plane, so big-endian as well. mono_bitmap() fills a
 * 1-plane device-format MFDB (fd_stand = 0, then vrt_cpyfm), whose words
 * the NewTOS VDI reads as native 16-bit words with bit 15 the leftmost
 * pixel (docs/sdk/13-zview.md, custom_font.c), so it stores native words.
 *
 * Also (big-endian file format): LoadPicture() takes HYP_PICTURE.width and
 * .height from the file as big-endian words; they are swapped after
 * GetEntryBytes (marked NEWTOS).
 *
 * TASK/A142: what goes to vr_trnfm is native words too. The VDI reads a
 * standard-form MFDB as native words like any other, so a picture copied straight
 * from the file had the two 8-pixel halves of every word traded (the QED.HYP
 * logo). be_planes_to_native() converts in place right before vr_trnfm (marked
 * NEWTOS). The truecolor path stays on the file bytes, mono_bitmap() decodes them.
 * Host test: sdk/overlays/hypview/tests/prepare-host.sh.
 */
/*
 * HypView - (c)      - 2006 Philipp Donze
 *               2006 -      Philipp Donze & Odd Skancke
 *
 * A replacement hypertext viewer
 *
 * This file is part of HypView.
 *
 * HypView is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 2 of the License, or
 * (at your option) any later version.
 *
 * HypView is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with HypView; if not, write to the Free Software
 * Foundation, Inc., 59 Temple Place, Suite 330, Boston, MA  02111-1307  USA
 */

#ifdef __GNUC__
#include <osbind.h>
#else
#include <tos.h>
#endif
#include <string.h>
#include <gem.h>
#include "../diallib.h"
#include "../defs.h"
#include "../hyp.h"
static unsigned short
be_word(const unsigned char *p)
{
	return (unsigned short)((p[0] << 8) | p[1]);
}

/* the file's big-endian plane words -> native words, in place */
static void
be_planes_to_native(void *addr, long size)
{
	unsigned char *p = (unsigned char *)addr;
	long i;

	for (i = 0; i + 1 < size; i += 2)
	{
		unsigned short w = be_word(p + i);

		memcpy(p + i, &w, sizeof(w));
	}
}

/* pixel value (0..15) of pixel `bit` (15 = leftmost) of four plane words */
static unsigned short
pixel4(unsigned short w0, unsigned short w1, unsigned short w2, unsigned short w3, int bit)
{
	return (unsigned short)((((w3 >> bit) & 1) << 3) | (((w2 >> bit) & 1) << 2) |
		(((w1 >> bit) & 1) << 1) | ((w0 >> bit) & 1));
}

static void
reduce(char *addr, long plane_size, short _16to2)
{
	unsigned char *p0 = (unsigned char *)addr;
	unsigned char *p1 = p0 + plane_size;
	unsigned char *p2 = p1 + plane_size;
	unsigned char *p3 = p2 + plane_size;
	unsigned short dither = (unsigned short)_16to2;
	long i;

	for (i = 0; i < plane_size; i += 2)
	{
		unsigned short w0 = be_word(p0 + i), w1 = be_word(p1 + i);
		unsigned short w2 = be_word(p2 + i), w3 = be_word(p3 + i);
		unsigned short out = 0;
		int bit;

		for (bit = 15; bit >= 0; bit--)
			out = (unsigned short)((out << 1) | ((dither >> pixel4(w0, w1, w2, w3, bit)) & 1));
		p0[i] = (unsigned char)(out >> 8);
		p0[i + 1] = (unsigned char)out;
	}
}

static void
mono_bitmap(void *src, void *dst, long planesize, short color)
{
	const unsigned char *p0 = (const unsigned char *)src;
	const unsigned char *p1 = p0 + planesize;
	const unsigned char *p2 = p1 + planesize;
	const unsigned char *p3 = p2 + planesize;
	unsigned short *out = (unsigned short *)dst;
	long i;

	for (i = 0; i < planesize; i += 2)
	{
		unsigned short w0 = be_word(p0 + i), w1 = be_word(p1 + i);
		unsigned short w2 = be_word(p2 + i), w3 = be_word(p3 + i);
		unsigned short mask = 0;
		int bit;

		for (bit = 15; bit >= 0; bit--)
			mask = (unsigned short)((mask << 1) | (pixel4(w0, w1, w2, w3, bit) == (unsigned short)color));
		*out++ = mask;
	}
}
static void
TrueColors(MFDB *src, MFDB *dst, short planes)
{
        short colors[2] = {G_BLACK,G_WHITE};
        
        colors[0] = G_BLACK;
        
        if (transparent_pics)
                colors[1] = background_col;

        if (planes == 4)
        {
                long plane_size=((long)dst->fd_wdwidth*dst->fd_h)<<1;
                MFDB mono;
                void *mono_plane;
                short pxy[8],i;
                short hardware2vdi[]={0,2,3,6,4,7,5,8,9,10,11,14,12,15,13,1};
                mono_plane = (void *)Malloc(plane_size);
                
                if(!mono_plane)
                {
                        Debug("Not enough memory to adapt picture to true color.");
                        Debug("Planesize is %ld",plane_size);
                        return;
                }
        
                memset(mono_plane,0,plane_size);
        
                mono.fd_addr = mono_plane;
                mono.fd_w = dst->fd_w;
                mono.fd_h = dst->fd_h;
                mono.fd_wdwidth = dst->fd_wdwidth;
                mono.fd_stand = 0;
                mono.fd_nplanes = 1;
                mono.fd_r1 = 0;
                mono.fd_r2 = 0;
                mono.fd_r3 = 0;
                
                pxy[0] = 0;
                pxy[1] = 0;
                pxy[4] = 0;
                pxy[5] = 0;

                pxy[2] = pxy[6] = dst->fd_w - 1;
                pxy[3] = pxy[7] = dst->fd_h - 1;
        
                mono_bitmap(src->fd_addr, mono_plane, plane_size, 15);
                vrt_cpyfm(vdi_handle,MD_REPLACE,pxy,&mono,dst,colors);
                
                for(i = 1; i < 15; i++)
                {
                        mono_bitmap(src->fd_addr, mono_plane, plane_size, i);
                        colors[0] = hardware2vdi[i];
                        vrt_cpyfm(vdi_handle, MD_TRANS, pxy, &mono, dst, colors);
                }
                Mfree(mono_plane);
        }
        else if(planes == 1)
        {
                short pxy[8];
                
                src->fd_nplanes = 1;
                pxy[0] = 0;
                pxy[1] = 0;

                pxy[4] = 0;
                pxy[5] = 0;

                pxy[2] = pxy[6] = dst->fd_w - 1;
                pxy[3] = pxy[7] = dst->fd_h - 1;
                vrt_cpyfm(vdi_handle,MD_REPLACE,pxy,src,dst,colors);
        }
}
static LOADED_PICTURE *
LoadPicture(HYP_DOCUMENT *hyp_doc, short num)
{
        LOADED_PICTURE *pic = NULL;
        HYP_PICTURE hyp_pic;
        long mem_size;
        long plane_size;
        short line_size;
        long size, loaded;
        void *data;

        data = LoadData(hyp_doc, num, &loaded);         /*      Lade Eintrag    */
        
        if(!data)
                return(NULL);

        /*      HYP_PICTURE-Header entpacken    */
        GetEntryBytes(hyp_doc, num, data, &hyp_pic, sizeof(HYP_PICTURE));
        /* NEWTOS: width and height are big-endian words in the file */
        hyp_pic.width = be_word((const unsigned char *)&hyp_pic.width);
        hyp_pic.height = be_word((const unsigned char *)&hyp_pic.height);

        line_size = ((hyp_pic.width + 15) / 16) * 2;
        plane_size = line_size*(long)hyp_pic.height;

        /*      Volle Laenge der Daten ermitteln        */
        size = GetDataSize(hyp_doc, num);
        if(size != plane_size * hyp_pic.planes + sizeof(HYP_PICTURE))
        {
                Debug("Strange picture format: %ld bytes found, but %ld bytes expected",
                        size, plane_size * hyp_pic.planes + sizeof(HYP_PICTURE));
                Mfree(data);
                return(NULL);
        }
        
/*      Debug("W: %d H: %d Planes: %d pic: %x on: %x",
                        hyp_pic.width,hyp_pic.height,hyp_pic.planes,
                        hyp_pic.plane_pic,hyp_pic.plane_on_off);
*/
   hyp_pic.plane_pic &= ((1 << hyp_pic.planes) - 1);
        hyp_pic.plane_on_off &= ((1 << hyp_pic.planes) - 1);

        if (hyp_pic.plane_on_off)
        {
                Debug("Unsupported picture format! The result may look strange.");
        }
        
        mem_size = sizeof(LOADED_PICTURE) + sizeof(HYP_PICTURE);
        if(hyp_pic.planes > 1)
        {
                mem_size += plane_size * max(hyp_pic.planes, ext_workout[4]);
                mem_size += plane_size * ext_workout[4];
        }
        else
                mem_size += 2 * plane_size;

/*      Debug("Plane size %ld. Allocating %ld bytes",plane_size,mem_size);*/
        pic = (LOADED_PICTURE *)Malloc(mem_size);
        if(pic)
        {
                void *pic_start = (void *)((long)pic + sizeof(LOADED_PICTURE));
                char *src;
                MFDB std_pic;

#ifdef __GNUC__
                bzero(pic, sizeof(LOADED_PICTURE));
#else
                memset(pic, 0, sizeof(LOADED_PICTURE));
#endif

                if (hyp_pic.planes > 1)
                        src = (char *)((long)pic_start + plane_size * ext_workout[4]);
                else
                        src = (char *)((long)pic_start + plane_size);

                GetEntryBytes(hyp_doc, num, data, src, size);
                src += sizeof(HYP_PICTURE);                             /*      HYP_PICTURE-Header ueberspringen        */

                pic->number = num;

                pic->mfdb.fd_addr = pic_start;
                pic->mfdb.fd_w = hyp_pic.width;
                pic->mfdb.fd_h = hyp_pic.height;
                pic->mfdb.fd_wdwidth = line_size / 2;
                
                if (hyp_pic.planes > 1)
                {                       
                        /*      Color image!    */
                        pic->mfdb.fd_stand = 0;
                        pic->mfdb.fd_nplanes = ext_workout[4];
                        pic->mfdb.fd_r1 = pic->mfdb.fd_r2 = pic->mfdb.fd_r3 = 0;
                        
                        std_pic = pic->mfdb;
                        std_pic.fd_stand = 1;
                        std_pic.fd_addr = src;
        
                        if (ext_workout[4] <= 8)                                /*      Kein True-Colour?       */
                        {
                                if (hyp_pic.planes < ext_workout[4])    /*      Bild hat weniger Planes?        */
                                {
                                        char *unused_plane = &src[hyp_pic.planes * plane_size];
                                        short j;
                                        long i;
        
                                        /*      Schwarze Pixel extrahieren...   */
                                        for (i = 0; i < plane_size; i++)
                                        {
                                                unused_plane[i] = src[i];
                                                for(j = 1; j < hyp_pic.planes; j++)
                                                        unused_plane[i] &= src[i + j * plane_size];
                                        }
                                        
                                        /*      ... und in noch leere Planes kopieren   */
                                        for (j = 1; j < (ext_workout[4] - hyp_pic.planes); j++)
                                                memcpy(unused_plane + j * plane_size, unused_plane, plane_size);
                                }
                                else if (hyp_pic.planes > ext_workout[4])
                                {
                                        short _16to2 = 0xFF7EU;
                                        
                                        pic->mfdb.fd_nplanes = 1;
                                        std_pic.fd_nplanes = 1;
                                
                                        reduce(std_pic.fd_addr, plane_size, _16to2);
                                }
        
                                /* NEWTOS: every plane the file brought, reduce()'s result included */
                                be_planes_to_native(src, plane_size * max(hyp_pic.planes, std_pic.fd_nplanes));
                                vr_trnfm(vdi_handle,&std_pic,&pic->mfdb);
        
                                if (background_col && transparent_pics)
                                {
                                        short pxy[8];
                                        short colors[2];
                                        short j;
                                        long i;
                                        colors[1] = background_col;
        
                                        pxy[0] = 0;
                                        pxy[1] = 0;
                                        pxy[4] = 0;
                                        pxy[5] = 0;

                                        pxy[2] = pxy[6] = pic->mfdb.fd_w - 1;
                                        pxy[3] = pxy[7] = pic->mfdb.fd_h - 1;
        
                                        /*      Farbige Pixel extrahieren...    */
                                        for (i =0 ; i < plane_size; i++)
                                        {
                                                for (j = 1; j < hyp_pic.planes; j++)
                                                        src[i] |= src[i + j * plane_size];
                                        }
        
                                        std_pic.fd_nplanes = 1;
                                        vrt_cpyfm(vdi_handle, MD_ERASE, pxy, &std_pic, &pic->mfdb, colors);
                                }
                        }
                        else
                        {
                                TrueColors(&std_pic,&pic->mfdb,hyp_pic.planes);
                        }
                }
                else
                {
                        /*      Monochrome image!       */
                        pic->mfdb.fd_stand = 0;
                        pic->mfdb.fd_nplanes = 1;
                        pic->mfdb.fd_r1 = pic->mfdb.fd_r2 = pic->mfdb.fd_r3 = 0;
                        
                        std_pic = pic->mfdb;
                        std_pic.fd_stand = 1;
                        std_pic.fd_addr = src;

                        be_planes_to_native(src, plane_size);   /* NEWTOS */
                        vr_trnfm(vdi_handle, &std_pic, &pic->mfdb);
                }
                Mshrink(pic,sizeof(LOADED_PICTURE) + plane_size * pic->mfdb.fd_nplanes);
        }
        Mfree(data);

        return(pic);
}

#if 0
static void
d8b(char *s)
{
        int i;

        for (i = 0; i < 8; i++)
        {
                NDIAG(("%03d ", s[i]));
        }
        NDIAG(("1/2 = %d, 4/5 = %d", dec_from_chars(&s[1]), dec_from_chars(&s[4])));
        DIAG((""));
}
#endif

static void
set_start(struct prepnode *p, short empty)
{
        int i;
        if (p->pic_count)
        {
                long limag_yadd = 0;
                short limage_y = -1;
                short align;
                short inside_idx = -1;
                
                for (i = 0; i < p->pic_count; i++)
                {
                        short ly2 = p->pic_adm[i].orig.g_y + p->pic_adm[i].orig.g_h;
                        
                        if (p->line >= p->pic_adm[i].orig.g_y && p->line < ly2)
                        {
                                DIAG((" 0 inside type=%d, y=%d, h=%d", p->pic_adm[i].type, p->pic_adm[i].orig.g_y, p->pic_adm[i].orig.g_h));
                                
                                if (p->pic_adm[i].type != -1 && p->line == p->pic_adm[i].orig.g_y)
                                {
                                        char *psrc = p->pic_adm[i].src;
                                        short py2 = 0, ny, type = p->pic_adm[i].type;
                                        short islimage;
                                        LOADED_PICTURE *pic;
                                        
                                        if (type == LINE || type == BOX || type == RBOX)
                                        {
                                                DIAG((" set line(idx=%d)to %ld", i, p->y));
                                                dec_to_chars((short)p->y, (unsigned char *)&psrc[2]);
                                                p->pic_adm[i].trans.g_y = p->y;
                                                p->pic_adm[i].trans.g_h = p->pic_adm[i].orig.g_h * font_ch;
                                        }
                                        else
                                        {
                                                if (i)
                                                {
                                                        py2 = (p->pic_adm[i - 1].trans.g_y + p->pic_adm[i - 1].trans.g_h);
                                                        align = py2 % font_ch;
                                                        if (align)
                                                                py2 += (font_ch - align);
                                                }
                                                
                                                if ((islimage = (p->hyp->comp_vers >= 3 && psrc[6] == 1) ? 1 : 0) || !i || p->y >= py2)
                                                {
                                                        ny = p->y;
                                                        DIAG((" -- idx %d placed at line %d(py2=%d)(%d)", i, ny, py2, islimage));
                                                        if (islimage)
                                                        {
                                                                pic = p->pic_adm[i].data;
                                                                
                                                                DIAG(("limage! add %d to y", p->pic_adm[i].trans.g_h));
                                                                if (limage_y == -1)
                                                                        limage_y = ny;
                                                                DIAG(("limage_y %d", limage_y));
                                                                
                                                                if (limag_yadd < pic->mfdb.fd_h)
                                                                {
                                                                        p->line_ptr[p->line].y += pic->mfdb.fd_h - limag_yadd;
                                                                        p->y += pic->mfdb.fd_h - limag_yadd;
                                                                        align = p->y % font_ch;
                                                                        if (align)
                                                                        {
                                                                                align = font_ch - align;
                                                                                p->line_ptr[p->line].y += align;
                                                                                p->y += align;
                                                                        }
                                                                        limag_yadd = pic->mfdb.fd_h;
                                                                
                                                                        p->last_line = p->line;
                                                                        p->last_y = p->y;
                                                                        p->last_width = p->width;
                                                                }
                                                                p->pic_adm[i].trans.g_y = ny = limage_y;
                                                                p->pic_adm[i].type = -1;
                                                                empty = 1;
                                                        }
                                                        else
                                                                p->pic_adm[i].trans.g_y = ny;
                                                }
                                                else
                                                {
                                                        ny = p->y;
                                                        p->pic_adm[i].trans.g_y = ny;
                                                        if (inside_idx == -1 || p->pic_adm[i].trans.g_h > p->pic_adm[inside_idx].trans.g_h)
                                                        {
                                                                if (inside_idx >= 0)
                                                                        p->pic_adm[inside_idx].type = -1;
                                                                inside_idx = i;
                                                        }
                                                        else
                                                                p->pic_adm[i].type = -1;
                                                        DIAG((" -- idx %d largest height", inside_idx));
                                                }
                                                DIAG((" -- pict y=%d, liney=%ld", ny * font_ch, p->y));
                                                if (type == PIC)
                                                        dec_to_chars(ny, (unsigned char *)&psrc[4]);
                                                else
                                                        dec_to_chars(ny, (unsigned char *)&psrc[2]);
                                                if (!empty)
                                                {
                                                        p->start_y = p->line;
                                                        p->start_line = -1;
                                                        p->y_start = p->y;
                                                        p->start_idx = i;
                                                        p->between = 1;
                                                        p->last_line = -1;
                                                }
                                        }
                                }
                        }
                }
                if (inside_idx >= 0 && !empty)
                {
                        p->start_y = p->line;
                        p->start_line = -1;
                        p->y_start = p->y;
                        p->start_idx = inside_idx;
                        p->between = 1;
                        p->last_line = -1;
                }
        }
}

static short
check_end(struct prepnode *p, short force)
{
        short ret = 0;
        

        DIAG(("between %d", p->between));
        switch (p->between)
        {
                case 1:
                {
                        short h1, h2;
                        if (force || (p->line >= (p->pic_adm[p->start_idx].orig.g_y + p->pic_adm[p->start_idx].orig.g_h)) )
                        {
                                long py2 = p->pic_adm[p->start_idx].trans.g_y + p->pic_adm[p->start_idx].trans.g_h;
                                if (p->pic_adm[p->start_idx].type != -1)
                                {
                                        short start_line = p->start_line;

                                        DIAG((" lines between %d(real=%d) and %d(real=%d) belong in rect (idx=%d) %d/%d/%d/%d",
                                                p->start_y, p->start_line, p->line - 1, p->last_line, p->start_idx,
                                                p->pic_adm[p->start_idx].orig.g_x, p->pic_adm[p->start_idx].orig. g_y,
                                                p->pic_adm[p->start_idx].orig.g_w, p->pic_adm[p->start_idx].orig.g_h));
                        
                                        if (start_line == -1)
                                                start_line = p->start_y;
                                        
                                        if (start_line != -1)
                                        {
                                                DIAG((" -- last_line = %d, y = %ld", p->line, p->y));
                                                h2 = p->pic_adm[p->start_idx].trans.g_h;
                                                h1 = p->y - p->y_start;
                                        
                                                DIAG((" -- h1=%d, h2=%d", h1, h2));
                                                if (h2 > h1)
                                                {
                                                        long diff = ((long)p->pic_adm[p->start_idx].trans.g_y + (h2 - h1)) - p->y_start;
                                                        
                                                        DIAG(("transy=%d, y2=%d", p->pic_adm[p->start_idx].trans.g_y, p->pic_adm[p->start_idx].trans.g_y + p->pic_adm[p->start_idx].trans.g_h));
                                        
                                                        p->line_ptr[start_line].y += diff;
                                                        p->y += diff;
                                                        diff = (p->y % font_ch);
                                                        if (diff)
                                                        {
                                                                diff = font_ch - diff;
                                                                p->line_ptr[start_line].y += diff;
                                                                p->y += diff;
                                                        }
                                                        DIAG((" -- font_ch = %d, diff=%ld, set y=%d on line %d (newly=%ld)(newy %ld)",
                                                                font_ch, diff, p->line_ptr[start_line].y, start_line, p->y_start + p->line_ptr[start_line].y, p->y));
                                                }
                                                p->pic_adm[p->start_idx].type = -1;
                                        }
                                #if 1
                                        else if (p->y < py2)
                                        {
                                                long diff = p->y - py2;
                                                
                                                p->line_ptr[p->line].y += diff;
                                                p->y += diff;
                                                diff = (diff % font_ch);
                                                if (diff)
                                                {
                                                        diff = font_ch - diff;
                                                        p->line_ptr[p->line].y += diff;
                                                        p->y += diff;
                                                }
                                        }
                                #endif
                                        p->last_y = p->y;
                                        p->last_line = p->line;
                                        p->last_width = p->width;
                                }
                                p->start_y = -1;
                                ret = 1;
                        }
                        break;
                }
        }
        return ret;
}

short
PrepareNode(HYP_DOCUMENT *hyp, LOADED_NODE *node)
{
        char *src = node->start;
        char *end = node->end;
        char *line_start;
        short gfx_bloc = TRUE;
        long real_height = 0;
        struct prepnode p;

        memset(&p, 0, sizeof(p));
        p.hyp = hyp;
        p.node = node;

        DIAG(("PrepareNode:"));

        while (*src == 27)
        {
                switch (src[1])
                {
                        case PIC:
                        case LINE:
                        case BOX:
                        case RBOX:
                        {
                                p.pic_count++;
                                break;
                        }
                        default:;
                }
                src = skip_esc(src);
        }

        if (p.pic_count)
        {
                p.pic_adm = (struct pic_adm *)Malloc(sizeof(struct pic_adm) * p.pic_count);
                if (!p.pic_adm)
                        goto error;
        }
                
        /*      Zuerst wird der Grafik-/Info-Block bearbeitet (=alles ESC Daten)        */
        src = node->start;
        while (gfx_bloc)                /*      ESC ?   */
        {
                if (*src == 27)
                {
                        src++;
                        switch (*src)
                        {
                                case WINDOWTITLE:
                                        src++;                                  /*      Nummer ueberspringen    */
                                        node->window_title = src;       /*      Fenstertitel merken     */
                                        src += strlen(src) + 1; /*      Daten ueberspringen     */
                        /*****  Abfrage ob ungerade Adresse noetig???   ******/
                                        break;
                                case EXTERNAL_REFS:             /*      bis zu 12 Querverweis-Datenbloecke      */
                                {
/*                                      short node = DEC_255(&src[2]);
                                        printf("ext REF: Text <%s> ",src+4);
                                        printf("Node %lx <%s>, ",hyp->indextable[node],&hyp->indextable[node]->name);
*/
                                        src += src[1] - 1;                      /*      Daten ueberspringen     */
                                        break;
                                }
                                case 40:                                                /*      weitere Datenbloecke    */
                                case 41:                                                /*      weitere Datenbloecke    */
                                case 42:                                                /*      weitere Datenbloecke    */
                                case 43:                                                /*      weitere Datenbloecke    */
                                case 44:                                                /*      weitere Datenbloecke    */
                                case 45:                                                /*      weitere Datenbloecke    */
                                case 46:                                                /*      weitere Datenbloecke    */
                                case 47:                                                /*      weitere Datenbloecke    */
                                        DIAG(("tag %d, size %d", *src, src[1]));
                                        src += src[1] - 1;                      /*      Daten ueberspringen     */
                                        break;                          
                                case OBJTABLE:                          /*      Tabelle mit Objekten und Seiten */
                                {
#if 0
                                        short line_nr, tree_nr,obj_nr,index_nr;
                                        line_nr=DEC_255(&src[1]);       /*      Zeilennummer in der Zielseite   */
                                        tree_nr=DEC_255(&src[3]);       /*      Nummer des Baumes       */
                                        obj_nr=DEC_255(&src[5]);        /*      Objekt in diesem Baum   */
                                        index_nr=DEC_255(&src[7]);      /*      Index der Seite */
                                        
                                        Debug("Objtable: Line: %d  Tree: %d  Obj: %d  Index: %d",
                                                        line_nr, tree_nr,obj_nr,index_nr);

#endif
                                        src += 9;                                       /*      Daten berspringen       */
                                        break;
                                }
                                case PIC:       /*      Bild    */
                                {
                                        int i;
                                        short num;
                                        long tmp;
                                        LOADED_PICTURE *pic;
                                        
                                        num = dec_from_chars((unsigned char *)&src[1]);
                                        
                                        pic = AskCache(hyp, num);
                                        
                                        if(!pic)
                                        {
                                                pic = LoadPicture(hyp,num);
                                                TellCache(hyp,num,pic);
                                        }

                                        if (pic)
                                        {
                                                short islimage = 0;
                                
                                                DIAG((" -- pic_idx %d", p.pic_idx));
                                                
                                                if (hyp->comp_vers >= 3 && src[6] == 1)
                                                        islimage = 1;
                                                
                                                src[7] = (pic->mfdb.fd_h + 15) >> 4;
                                                
                                                
                                                p.pic_adm[p.pic_idx].type = PIC;
                                                p.pic_adm[p.pic_idx].src = src;
                                                p.pic_adm[p.pic_idx].data = pic;

                                                for (i = 0; i < 8; i++)
                                                        p.pic_adm[p.pic_idx].osrc[i] = src[i];

                                                p.pic_adm[p.pic_idx].osrc[7] = (pic->mfdb.fd_h + 15) >> 4;

                                                if (!src[3])
                                                {
                                                        short val = (hyp->line_width - (pic->mfdb.fd_w / font_cw)) >> 1;
                                                        
                                                        val = val > 0 ? val : 0;
                                                        
                                                        p.pic_adm[p.pic_idx].orig.g_x = val;
                                                        src[3] = val;
                                                }
                                                else
                                                {
                                                        src[3]--;
                                                        p.pic_adm[p.pic_idx].orig.g_x = src[3];
                                                }
                                                
                                                p.pic_adm[p.pic_idx].orig.g_w = pic->mfdb.fd_w / font_cw;
                                                
                                                p.pic_adm[p.pic_idx].orig.g_y = dec_from_chars((unsigned char *)&src[4]);
                                                p.pic_adm[p.pic_idx].orig.g_h = src[7];
                                                
                                                tmp = pic->mfdb.fd_h;
                                                if ((tmp % font_ch))
                                                        tmp += font_ch - (tmp % font_ch);
                                                p.pic_adm[p.pic_idx].trans.g_h = tmp;
                                                
                                                tmp = (long)((unsigned short)dec_from_chars((unsigned char *)&src[4])) * 16;
                                                tmp += ((tmp % font_ch) ? font_ch - (tmp % font_ch) : 0);
                                                p.pic_adm[p.pic_idx].trans.g_y = tmp;
                                                
                                                if (islimage)
                                                {
                                                        p.line += p.pic_adm[p.pic_idx].orig.g_h;
                                                        p.limage_add += p.pic_adm[p.pic_idx].orig.g_h;
                                                        p.pic_adm[p.pic_idx].flags = 1;
                                                }
                                                DIAG((" IDX %d(PIC ): limage? %s, org %d/%d/%d/%d - %d/%d/%d/%d",
                                                        p.pic_idx,
                                                        islimage ? "Yes":"No",
                                                        p.pic_adm[p.pic_idx].orig.g_x,  p.pic_adm[p.pic_idx].orig. g_y,
                                                        p.pic_adm[p.pic_idx].orig.g_w,  p.pic_adm[p.pic_idx].orig.g_h,
                                                        p.pic_adm[p.pic_idx].trans.g_x, p.pic_adm[p.pic_idx].trans.g_y,
                                                        p.pic_adm[p.pic_idx].trans.g_w, p.pic_adm[p.pic_idx].trans.g_h));

                                                /*      @limage ?       */
                                                if (islimage)
                                                {
                                                        p.line += src[7];
                                                        p.limage_add += src[7];
                                                }
                                                else
                                                {                                                       
                                                        p.min_lines = max(p.min_lines, (short)(p.limage_add + dec_from_chars((unsigned char *)&src[4])) + src[7]);
                                                }
                                                p.width = max(p.width, src[3] * font_cw + pic->mfdb.fd_w);
                                        }
                                        src += 8;
                                        p.pic_idx++;
                                        break;
                                }
                                case LINE:
                                {
                                        GRECT r;
                                        
                                        r.g_x = src[1];
                                        r.g_y = dec_from_chars((unsigned char *)&src[2]);
                                        r.g_w = (src[4] - 128);
                                        r.g_h = src[5];
                                        
                                        p.min_lines = max(p.min_lines, r.g_y + r.g_h);
                                        p.width = max(p.width, r.g_x + r.g_w);
                                        
                                        p.pic_adm[p.pic_idx].src = src;
                                        p.pic_adm[p.pic_idx].type = *src;
                                        p.pic_adm[p.pic_idx].orig = r;
                                        
                                        p.pic_adm[p.pic_idx].trans.g_y = dec_from_chars((unsigned char *)&src[2]) * 16;
                                        p.pic_adm[p.pic_idx].trans.g_h = src[5];

                                        DIAG((" IDX %d(LINE): org %d/%d/%d/%d - %d/%d/%d/%d",
                                                p.pic_idx,
                                                p.pic_adm[p.pic_idx].orig.g_x,  p.pic_adm[p.pic_idx].orig. g_y,
                                                p.pic_adm[p.pic_idx].orig.g_w,  p.pic_adm[p.pic_idx].orig.g_h,
                                                p.pic_adm[p.pic_idx].trans.g_x, p.pic_adm[p.pic_idx].trans.g_y,
                                                p.pic_adm[p.pic_idx].trans.g_w, p.pic_adm[p.pic_idx].trans.g_h));
                                        
                                        src += 7;
                                        p.pic_idx++;
                                        break;
                                }
                                case BOX:
                                case RBOX:
                                {
                                        GRECT r;
                                        
                                        r.g_x = src[1];
                                        r.g_y = p.limage_add + dec_from_chars((unsigned char *)&src[2]);
                                        r.g_w = (src[4] - 128);
                                        r.g_h = src[5];
                                        
                                        p.min_lines = max(p.min_lines, r.g_y + r.g_h);
                                        p.width = max(p.width, r.g_x + r.g_w);
                                        
                                        p.pic_adm[p.pic_idx].src = src;
                                        p.pic_adm[p.pic_idx].type = *src;
                                        p.pic_adm[p.pic_idx].orig = r;
                                        
                                        p.pic_adm[p.pic_idx].trans.g_y = dec_from_chars((unsigned char *)&src[2]) * 16;
                                        p.pic_adm[p.pic_idx].trans.g_h = src[5];
                                        
                                        DIAG((" IDX %d(rBOX): org %d/%d/%d/%d - %d/%d/%d/%d",
                                                p.pic_idx,
                                                p.pic_adm[p.pic_idx].orig.g_x,  p.pic_adm[p.pic_idx].orig. g_y,
                                                p.pic_adm[p.pic_idx].orig.g_w,  p.pic_adm[p.pic_idx].orig.g_h,
                                                p.pic_adm[p.pic_idx].trans.g_x, p.pic_adm[p.pic_idx].trans.g_y,
                                                p.pic_adm[p.pic_idx].trans.g_w, p.pic_adm[p.pic_idx].trans.g_h));
                                        src += 7;
                                        p.pic_idx++;
                                        break;
                                }
                                default:
                                        gfx_bloc = FALSE;
                                        src--;
                                        break;
                        }
                }
                else
                        gfx_bloc = FALSE;
        }

        /*      Beginn der ersten Zeile merken  */      
        line_start = src;

        /*      Anzahl Zeilen Anhand der Nullbytes zaehlen      */
        while (src < end)
        {
                if (!*src++)            /*      Nullbyte? => Zeilenende */
                        p.line++;
        }
        p.line += 1;

        node->lines = max(p.line, p.min_lines);
        
        /*      Speicher fuer die Zeilenanfang-Tabelle anlegen  */
        p.line_ptr = (LINEPTR *)Malloc(sizeof(LINEPTR) * node->lines);
        
        if (!(node->line_ptr = p.line_ptr))
        {
                Debug("ERROR: Out of memory while creating line buffer");
                goto error;
        }
        
        /*      Tabelle leeren  */
        memset(p.line_ptr, 0, sizeof(LINEPTR) * node->lines);
        
        /*      <src> auf den Anfang der Textdaten zuruecksetzen        */
        src = line_start;

        p.line = 0;
        p.limage_add = 0;
        p.last_line = -1;

        {
                short ext[8], x = 0, curr_txt_effect = 0;
                char line_buffer[1024];
                char *dst = line_buffer;
                
                p.start_y = -1;
                p.start_idx = -1;
                p.h = font_ch;
/* ****************************************************************************************** */
                p.line_ptr[0].txt = src;

                /*      Standard Text-Effekt    */
                vst_effects(vdi_handle, curr_txt_effect);

                while (p.line < node->lines)
                {
                        if (src && src >= end)
                                src = NULL;

                        if (src && *src == 27)                                  /*      ESC-Codes       */
                        {
                                src++;                                  /*      ESC ueberspringen       */

                                *dst = 0;                               /*      Pufferende schreiben    */
                                dst = line_buffer;                      /*      ... und zuruecksetzen   */
        
                                /*      Zeilenpuffer enthaelt Daten?    */
                                if (*line_buffer)
                                {
                                        vqt_extent(vdi_handle,line_buffer,ext);
                                        x += (ext[2] + ext[4]) >> 1;
                                }
        
                                switch (*src)                           /*      Was fuer ein Code?      */
                                {
                                        case 27:                        /*      ESC Zeichen     */
                                                *dst++=27;
                                                src++;
                                                break;
                                        case 37:
                                        case 39:
                                        case LINK:
                                        case ALINK:
                                        {
                                                unsigned short dest_page;
                                                if(*src & 1)            /*      Zeilennummer ueberspringen      */
                                                        src += 2;
        
                                                dest_page = DEC_255(&src[1]);
                                                src += 3;
        
                                                /*      Verknuepfungstext ermitteln     */
                                                if(*src == 32)
                                                        dst = &hyp->indextable[dest_page]->name;
                                                else
                                                {
                                                        strncpy(dst, src + 1,(*(unsigned char *)src) - 32L);
                                                        dst[(*(unsigned char *)src) - 32] = 0;
                                                        src += (*(unsigned char *)src) - 32 ;
                                                }
        
                                                /*      Verknuepfungstext mit entsprechendem Texteffekt ausgeben        */
                                                vst_effects(vdi_handle,link_effect);
        
                                                vqt_extent(vdi_handle, dst, ext);
                                                x += (ext[2] + ext[4]) >> 1;
        
                                                vst_effects(vdi_handle, curr_txt_effect);
        
                                                dst = line_buffer;
                                                *dst = 0;
                                                src++;
        
                                                break;
                                        }
                                        default:
                                        {
                                                if (*(unsigned char *)src >= 100)       /*      Text-Attribute  */
                                                {
                                                        curr_txt_effect = *((unsigned char *)src) - 100;
                                                        vst_effects(vdi_handle,curr_txt_effect);
                                                        src++;
                                                }
                                                else
                                                {
                                                        DIAG(("unknown tag: %d",*src));
                                                        Debug("unknown Tag: %d,",*(unsigned char *)src);
                                                }
                                        }
                                }
                        }
                        else if (src && *src)                                   /*      Beliebiger Text */
                        {
                                *dst++ = *src++;
                        }
                        else                                                    /*      Zeilenende      */
                        {
                                NDIAG(("NOW ON LINE %d - y=%ld(%ld)", p.line, p.y, real_height));

                                if (src)
                                {
                                        *dst = 0;                                       /*      Pufferende schreiben    */
                                        dst = line_buffer;
                                }                               /*      ... und zuruecksetzen   */
                                if (!src || src == line_start)
                                {
                                        DIAG((" -- EMPTY "));
                        again_e:
                                        if (p.start_y == -1)
                                                set_start(&p, 0);
                                        else
                                        {
                                                if (check_end(&p, 0))
                                                        goto again_e;
                                        }
                                        line_start = NULL;
                                        if (!p.empty_lines)
                                                p.empty_start = p.line;
                                        p.empty_lines++;        
                                }
                                else
                                {                                       
                                        DIAG((" "));
                        again:
                                        if (p.start_y == -1)
                                                set_start(&p, 0);
                                        else
                                        {
                                                if (check_end(&p, 0))
                                                        goto again;
                                        }
                                        if (p.start_line == -1)
                                        {
                                                p.start_line = p.line;
                                                p.start_y = p.line;
                                                p.y_start = p.y;
                                        }
                                        p.empty_lines = 0;
                                        vqt_extent(vdi_handle, line_buffer, ext);
                                        x += (ext[2] + ext[4]) >> 1;
                                        p.width = max(p.width, x);
                                        x = 0;
                                }
                                if (line_start || p.start_y != -1)
                                {
                                        p.last_y = p.y;
                                        p.last_line = p.line;
                                        p.last_width = p.width;
                                }
                                p.y += p.h;

                                p.line_ptr[p.line].w = p.width;
                                p.line_ptr[p.line].h = p.h;
                                p.line_ptr[p.line].txt = line_start;
                                
                                real_height += (p.line_ptr[p.line].y + p.line_ptr[p.line].h);
                                
                                p.line++;
                                
                                line_start = src ? ++src : NULL;
                        }
                }
        }

        DIAG(("last line %d, height %ld", p.last_line, p.last_y + p.h));
        node->columns = p.last_width / font_cw + 2;
        node->height = p.last_y + p.h;
        node->lines = p.line;
        
        if (p.pic_adm)
                Mfree(p.pic_adm);

        DIAG(("leaving ok, width=%d, height=%ld(%ld)\r\n", p.width, p.y, real_height));
        return(FALSE);

error:
        DIAG(("leaving error\r\n"));
        if (node->line_ptr)
                Mfree(node->line_ptr);
        if (p.pic_adm)
                Mfree(p.pic_adm);
        return(TRUE);
}
