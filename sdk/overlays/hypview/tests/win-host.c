/*
 * win-host.c -- host harness for HypView's window list (dl_win.c: OpenWindow,
 * RemoveWindow), TASK/A140 (GitHub #22). Built and run by win-host.sh beside
 * this file.
 *
 * This file is the translation unit: it pulls in mintlib's <osbind.h>,
 * replaces the GEMDOS block calls dl_win.c makes (Mxalloc, Mfree -- svc traps
 * in the SDK) with host malloc/free, fakes the AES calls it makes, and then
 * #includes the dl_win.c under test (DLWIN_C, a quoted path) unchanged.
 *
 * What is checked: the global `Win` ("current window", global.c) is either
 * NULL or a live window after every path that frees a window. HelpWindow
 * (window.c) and PopupWindow (hyp/popup.c) store their window in `Win` on every
 * call, so `Win` holds the window being freed when RemoveWindow frees it.
 * find_file (file.c) reads `Win->data` on the next OpenFileNW. The fake window
 * procs below do what those two do with `Win`, and nothing else.
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

#undef Mxalloc
#undef Mfree
static int live_blocks;
static void *host_alloc(long n)
{
	live_blocks++;
	return malloc((size_t)n);
}
static long host_free(void *p)
{
	live_blocks--;
	free(p);
	return 0;
}
#define Mxalloc(n, mode)	host_alloc((long)(n))
#define Mfree(p)		host_free((void *)(p))

#ifndef DLWIN_C
#error "DLWIN_C (quoted path of the dl_win.c under test) not defined"
#endif
#include DLWIN_C

/* ---- what dl_win.c takes from the rest of HypView ------------------------ */
WINDOW_DATA *Win;		/* global.c */
CHAIN_DATA *all_list;		/* dl_items.c */
short modal_items = -1;
short ap_id;
short has_iconify = 1;
short vdi_handle = 1;
short iconified_count;
CHAIN_DATA *iconified_list[MAX_ICONIFY_PLACE];
OBJECT **tree_addr;
short aes_global[16];

void add_item(CHAIN_DATA *item)
{
	item->next = all_list;
	all_list = item;
}
void remove_item(CHAIN_DATA *item)
{
	CHAIN_DATA **p = &all_list;

	while (*p && *p != item)
		p = &(*p)->next;
	if (*p)
		*p = item->next;
}
void AllIconify(short handle, GRECT *r) { (void)handle; (void)r; }
void ConvertKeypress(short *key, short *kstate) { (void)key; (void)kstate; }
void CycleItems(void) {}
void DoEventDispatch(EVNT *event) { (void)event; }
short rc_intersect_my(GRECT *r1, GRECT *r2) { (void)r1; (void)r2; return 1; }

/* ---- the AES as far as dl_win.c reaches it: all calls succeed ------------- */
static short next_handle = 1;
static short open_result = 1;

short mt_appl_write(short id, short n, void *m, short *g) { (void)id; (void)n; (void)m; (void)g; return 1; }
short mt_evnt_timer(unsigned long ms, short *g) { (void)ms; (void)g; return 1; }
short mt_form_alert(short b, const char *s, short *g) { (void)b; (void)s; (void)g; return 1; }
short mt_graf_growbox_grect(const GRECT *a, const GRECT *b, short *g) { (void)a; (void)b; (void)g; return 1; }
short mt_graf_mouse(short m, const MFORM *f, short *g) { (void)m; (void)f; (void)g; return 1; }
short mt_graf_shrinkbox_grect(const GRECT *a, const GRECT *b, short *g) { (void)a; (void)b; (void)g; return 1; }
short mt_graf_watchbox(OBJECT *t, short o, short a, short b, short *g) { (void)t; (void)o; (void)a; (void)b; (void)g; return 1; }
short mt_objc_draw_grect(OBJECT *t, short o, short d, const GRECT *r, short *g) { (void)t; (void)o; (void)d; (void)r; (void)g; return 1; }
short mt_objc_find(OBJECT *t, short o, short d, short x, short y, short *g) { (void)t; (void)o; (void)d; (void)x; (void)y; (void)g; return -1; }
short mt_wind_calc_grect(short t, short k, const GRECT *i, GRECT *o, short *g) { (void)t; (void)k; (void)g; *o = *i; return 1; }
short mt_wind_close(short h, short *g) { (void)h; (void)g; return 1; }
short mt_wind_create_grect(short k, const GRECT *r, short *g) { (void)k; (void)r; (void)g; return next_handle++; }
short mt_wind_delete(short h, short *g) { (void)h; (void)g; return 1; }
short mt_wind_get_grect(short h, short f, GRECT *r, short *g)
{
	(void)h; (void)f; (void)g;
	r->g_x = 0; r->g_y = 19; r->g_w = 640; r->g_h = 461;
	return 1;
}
short mt_wind_open_grect(short h, const GRECT *r, short *g) { (void)h; (void)r; (void)g; return open_result; }
short mt_wind_set(short h, short f, short a, short b, short c, short d, short *g)
{
	(void)h; (void)f; (void)a; (void)b; (void)c; (void)d; (void)g;
	return 1;
}
short mt_wind_set_grect(short h, short f, const GRECT *r, short *g) { (void)h; (void)f; (void)r; (void)g; return 1; }
short mt_wind_set_str(short h, short f, const char *s, short *g) { (void)h; (void)f; (void)s; (void)g; return 1; }
short mt_wind_update(short m, short *g) { (void)m; (void)g; return 1; }
void vro_cpyfm(short h, short m, short *p, MFDB *a, MFDB *b) { (void)h; (void)m; (void)p; (void)a; (void)b; }

/* ---- the fake window procs: only the `Win` behaviour of the real ones ----- */
static WINDOW_DATA *pending_popup;	/* what the main window removes at WIND_EXIT */

static short main_proc(WINDOW_DATA *p, short obj, void *data)
{
	(void)data;
	Win = p;			/* window.c HelpWindow: Win = win = p; */
	if (obj == WIND_EXIT)
	{
		if (pending_popup)	/* window.c: RemoveWindow(doc->popup) */
		{
			WINDOW_DATA *popup = pending_popup;

			pending_popup = NULL;
			RemoveWindow(popup);
		}
	}
	return 1;
}

static short popup_proc(WINDOW_DATA *p, short obj, void *data)
{
	(void)data;
	Win = p;			/* hyp/popup.c PopupWindow: Win = win = ptr; */
	return 1;
}

static short quiet_proc(WINDOW_DATA *p, short obj, void *data)
{
	(void)p; (void)obj; (void)data;
	return 1;
}

/* ---- the checks ----------------------------------------------------------- */
static int failed, checks;
static void check(const char *what, int ok)
{
	checks++;
	if (!ok)
		failed++;
	printf("%s  %s\n", ok ? "ok  " : "FAIL", what);
}

int main(void)
{
	static char t1[] = "doc1", t2[] = "doc2", t3[] = "popup";
	WINDOW_DATA *a, *b, *p;
	int i;

	if (sizeof(long) != 8 || sizeof(void *) != 8)
	{
		printf("win-host: skipped -- host not LP64\n");
		return 77;
	}

	/* The A140 sequence: open, close the last window, open again, repeatedly. */
	for (i = 0; i < 3; i++)
	{
		a = OpenWindow(main_proc, NAME | CLOSER, t1, 10, 10, t1);
		check("open: Win is the new window", a != NULL && Win == a);
		RemoveWindow(a);
		check("last window removed: Win is NULL", Win == NULL);
		check("last window removed: no block left", live_blocks == 0);
		check("last window removed: list empty", all_list == NULL);
	}

	/* Two document windows. RemoveWindow(a) calls a's proc, which stores a in
	 * `Win` as the real one does, so afterwards `Win` must not be a; `b` stays
	 * listed. The freed pointer is only compared, never read. */
	a = OpenWindow(main_proc, NAME | CLOSER, t1, 10, 10, t1);
	b = OpenWindow(main_proc, NAME | CLOSER, t2, 10, 10, t2);
	check("second open: Win is the second window", b != NULL && Win == b);
	{
		WINDOW_DATA *freed = a;

		RemoveWindow(a);
		check("first of two removed: Win is not the freed window", Win != freed);
	}
	check("first of two removed: other window alive", all_list == (CHAIN_DATA *)b);
	RemoveWindow(b);
	check("second of two removed: Win is NULL", Win == NULL);

	/* `Win` on another window is left alone by RemoveWindow's own clearing:
	 * a proc that does not store its window models a window whose proc did not
	 * run last. */
	a = OpenWindow(quiet_proc, NAME | CLOSER, t1, 10, 10, t1);
	b = OpenWindow(main_proc, NAME | CLOSER, t2, 10, 10, t2);
	check("quiet window + document: Win is the document", Win == b);
	RemoveWindow(a);
	check("other window removed: Win unchanged", Win == b);
	RemoveWindow(b);
	check("last removed: Win is NULL", Win == NULL);

	/* A popup closed on its own: the document window stays. */
	a = OpenWindow(main_proc, NAME | CLOSER, t1, 10, 10, t1);
	p = OpenWindow(popup_proc, 0, t3, 10, 10, t3);
	check("popup open: Win is the popup", p != NULL && Win == p);
	RemoveWindow(p);
	check("popup removed: Win is not the freed popup", Win == NULL);
	check("popup removed: document window alive", all_list == (CHAIN_DATA *)a);

	/* The document window closed with its popup open (window.c WIND_EXIT). */
	p = OpenWindow(popup_proc, 0, t3, 10, 10, t3);
	pending_popup = p;
	RemoveWindow(a);
	check("window with popup removed: Win is NULL", Win == NULL);
	check("window with popup removed: no block left", live_blocks == 0);

	/* wind_open fails: OpenWindow frees the block it just made. */
	open_result = 0;
	a = OpenWindow(main_proc, NAME | CLOSER, t1, 10, 10, t1);
	open_result = 1;
	check("wind_open refused: OpenWindow returns NULL", a == NULL);
	check("wind_open refused: Win is NULL", Win == NULL);
	check("wind_open refused: no block left", live_blocks == 0);

	printf("win-host: %d checks, %d failed\n", checks, failed);
	return failed;
}
