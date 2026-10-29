/*
 * color-host-stubs.h -- what zview/zvdi/color.c needs to compile on the host
 * (TASK/SDK04 probe S3, sdk/zview/tests/color-host.sh): the zView integer
 * names at their m68k widths (sdk/zview/include/zview-ints.h, the prelude
 * build-zview.sh passes with -DHAVE_INTS_DEFINED), WinDom's `app` with the
 * three fields color.c reads (color, nplanes, palette -- windom.h APPvar,
 * W_COLOR = INT16[3] in VDI units 0..1000), the picture types IMGINFO and
 * DECDATA reduced to the members color.c touches (pic_load.h), pixel_val
 * (zvdi/vdi.c) and the two pen names. color-host.sh makes general.h,
 * pic_load.h and zvdi/vdi.h of a scratch tree include this file, so the
 * color.c under test is compiled unchanged. The real headers pull in
 * WinDom, gemlib and mintlib.
 */
#ifndef ZVIEW_COLOR_HOST_STUBS_H
#define ZVIEW_COLOR_HOST_STUBS_H
#include <string.h>
#include ZVIEW_INTS_H

typedef short W_COLOR[3];
struct host_app { int color; int nplanes; W_COLOR *palette; };
extern struct host_app app;

typedef struct { uint8 red, green, blue; } COLOR_MAP;
typedef struct _img_info { uint32 colors; COLOR_MAP palette[256]; } img_info, *IMGINFO;
typedef struct _dec_data { uint32 Pixel[256]; } dec_data, *DECDATA;

extern int16 pixel_val[256];

#define G_WHITE 0
#define G_BLACK 1
#endif
