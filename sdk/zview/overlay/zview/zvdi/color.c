/*
 * NewTOS/AArch64 overlay (TASK/SDK04 probe S3, patch site SDK04-S3;
 * finding 8 of docs/analysis/byte-order-survey-2026-10-29.md). The pristine
 * file is z-tools-04805b7/zview/zvdi/color.c (z-tools commit 04805b7).
 * Changed: remap_color() and color_lookup(); marked NEWTOS.
 *
 * Why (endianness): a colour is an int32 0x00RRGGBB (cnvpal_4_8, vdi.c
 * cube216/graymap), and screen_colortab[] holds SRGB {satur, red, green,
 * blue}. Upstream takes R, G, B from the colour's bytes 1..3 and compares
 * and assembles colours through `*(int32 *)&screen_colortab[i]` -- both
 * read memory big-endian, as the 68k does. On little-endian AArch64 byte 1
 * is green and byte 3 the unused top byte, and an SRGB read as int32 is
 * 0xBBGGRRSS: red matched to green, blue to black, the exact-match test
 * never hits, color_lookup() returned 0xGGRRSS. Measured with the host
 * harness sdk/zview/tests/color-host.sh (the real color.c, a 16-colour
 * palette): 38 of 41 checks fail, e.g. remap_color(0xFF0000) = the green
 * entry. The patch takes the guns from the value by shifts and compares /
 * assembles the SRGB fields by name: the same results as the 68k.
 *
 * Also (LP64): color_lookup()'s "0xFFFFFFnn is index nn" test compared
 * against ~0xFFuL, 64-bit on LP64, so it never matched a 32-bit colour; it
 * is a 32-bit mask here. (Upstream's `(int32)satur << 24`, which overflows
 * for satur >= 128, goes away with the int32 compare.)
 *
 * Reach on NewTOS: only on a <= 8-plane screen (vdi.c `depth == 4 ||
 * depth == 8`, cnvpal_4_8); the VDI is 16-plane today (ADR-024), so the
 * path is latent (docs/sdk/13-zview.md). Every other line is upstream.
 */
#include "..//general.h"
#include "..//pic_load.h"
#include "vdi.h"

typedef struct 
{
	uint8 satur;
	uint8 red, green, blue;
} SRGB;

static SRGB screen_colortab[256];


/* prototype */
void save_colors(void);
int16 remap_color (int32 value);
uint32 color_lookup ( uint32 rgb, int16 *trans);
void cnvpal_mono ( IMGINFO info, DECDATA data);
void cnvpal_1_2 (IMGINFO info, DECDATA data);
void cnvpal_4_8( IMGINFO info, DECDATA data);
void cnvpal_15( IMGINFO info, DECDATA data);
void cnvpal_high( IMGINFO info, DECDATA data);
void cnvpal_true( IMGINFO info, DECDATA data);


static uint8 saturation (uint8 * rgb)
{
	uint8 satr;

	if (rgb[0] >= rgb[1]) 
	{
		if (rgb[1] >= rgb[2])   
			satr = rgb[0]   - rgb[2];
		else if (rgb[0] > rgb[2]) 
			satr = rgb[0]   - rgb[1];
		else
			satr = rgb[2]  - rgb[1];
	} 
	else if (rgb[1] >= rgb[2]) 
	{
		if (rgb[0] >= rgb[2])
	    	satr = rgb[1] - rgb[2];
		else
			satr = rgb[1] - rgb[0];
	} 
	else 
		satr = rgb[2]  - rgb[0];

	return satr;
}


void save_colors(void)
{
	register int16  i;

	for( i = 0; i < app.color; i++)
	{
		screen_colortab[i].red   = ((((int32)app.palette[i][0] << 8 ) - app.palette[i][0]) + 500) / 1000;
		screen_colortab[i].green = ((((int32)app.palette[i][1] << 8 ) - app.palette[i][1]) + 500) / 1000;
		screen_colortab[i].blue  = ((((int32)app.palette[i][2] << 8 ) - app.palette[i][2]) + 500) / 1000;
		screen_colortab[i].satur = saturation ( &screen_colortab[i].red);
	}
}

int16 remap_color (int32 value)
{
	uint8 rgb[3] = { (uint8)(value >> 16), (uint8)(value >> 8), (uint8)value };	/* NEWTOS: was ((uint8*)&value)[1..3] */
	int16 red   = rgb[0];
	int16 green = rgb[1];
	int16 blue  = rgb[2];
	int16 satur = saturation (rgb);	/* NEWTOS */
	
	int16  best_fit  = 0;
	uint16 best_err  = 0xFFFFu;
	int16  max_color = 1 << app.nplanes;
	
	int16 i = 0;
	
	/* NEWTOS: upstream folded satur into value's top byte here for the
	 * int32 compare below, which now compares the four fields by name */
	
	do 
	{
		if ( screen_colortab[i].satur == satur && screen_colortab[i].red == red	/* NEWTOS: was *( int32*)&screen_colortab[i] == value */
		  && screen_colortab[i].green == green && screen_colortab[i].blue == blue) 
		{
			/* gotcha! */
			best_fit = i;
			break;
		} 
		else 
		{
			uint16 err = 
			( red > screen_colortab[i].red ? red - screen_colortab[i].red : screen_colortab[i].red - red)
			+ (green > screen_colortab[i].green ? green - screen_colortab[i].green : screen_colortab[i].green - green)
			+ (blue  > screen_colortab[i].blue  ? blue  - screen_colortab[i].blue  : screen_colortab[i].blue - blue)
			+ (satur > screen_colortab[i].satur ? satur - screen_colortab[i].satur : screen_colortab[i].satur - satur);
			
			if (err <= best_err) 
			{
				best_err = err;
				best_fit = i;
			}
		}
	} while (++i < max_color);
	
	return best_fit;
}


uint32 color_lookup ( uint32 rgb, int16 *trans)
{
	uint8 idx = ((rgb & 0xFFFFFF00uL) == 0xFFFFFF00uL ? rgb : remap_color (rgb));	/* NEWTOS: was ~0xFFuL (64-bit) */
	return ( (( uint32)(trans ? trans[idx] : idx) << 24) | (( uint32)screen_colortab[idx].red << 16)	/* NEWTOS: was *(int32*)&screen_colortab[idx] & 0x00FFFFFFuL */
		| (( uint32)screen_colortab[idx].green << 8) | screen_colortab[idx].blue);
}


void cnvpal_mono ( IMGINFO info, DECDATA data)
{
/*	if ( info->indexed_color) 
	{
		uint32  bgnd, fgnd;
		
		bgnd = (((( int32)info->palette[0].red <<8) | info->palette[0].green) <<8) | info->palette[0].blue;
		fgnd = (((( int32)info->palette[1].red <<8) | info->palette[1].green) <<8) | info->palette[1].blue;

		data->Pixel[0] = remap_color ( bgnd);
		data->Pixel[1] = remap_color ( fgnd);
		
	}
	else 
*/	{
		data->Pixel[0] = G_WHITE;
		data->Pixel[1] = G_BLACK;
	}
}


void cnvpal_1_2 (IMGINFO info, DECDATA data)
{
	register int32 i;

	for( i = 0; i < info->colors; i++)
	{
		data->Pixel[i] = ( int16)info->palette[i].red * 5 + ( int16)info->palette[i].green * 9 + (( int16)info->palette[i].blue << 1);
	}
}


void cnvpal_4_8( IMGINFO info, DECDATA data)
{
	register int32 i;

	for( i = 0; i < info->colors; i++)
	{
	    uint32 rgb = (((( int32)info->palette[i].red << 8) | info->palette[i].green) << 8) | info->palette[i].blue;
		data->Pixel[i] = rgb | (( int32)pixel_val[remap_color ( rgb)] << 24);
	}
}


void cnvpal_15( IMGINFO info, DECDATA data)
{
	register int32 i;

	for( i = 0; i < info->colors; i++)
	{
		data->Pixel[i] = (( int16)( info->palette[i].red & 0xF8) << 7) | (( int16)( info->palette[i].green & 0xF8) << 2) | ( info->palette[i].blue >> 3);
	}
}


void cnvpal_high( IMGINFO info, DECDATA data)
{
	register int32 i;

	for( i = 0; i < info->colors; i++)
	{
		data->Pixel[i] = (( int16)( info->palette[i].red & 0xF8) << 8) | (( int16)( info->palette[i].green & 0xFC) << 3) | ( info->palette[i].blue >> 3);
	}
}


void cnvpal_true ( IMGINFO info, DECDATA data)
{
	register int32 i;
	
	for( i = 0; i < info->colors; i++)
	{
		data->Pixel[i] = (((( int32)info->palette[i].red << 8) | info->palette[i].green) << 8) | info->palette[i].blue;
	}
}
