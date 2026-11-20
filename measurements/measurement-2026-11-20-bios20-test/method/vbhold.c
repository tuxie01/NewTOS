/* TEMPORARY tester probe (BIOS20 test run, own tree only, never committed).
 * MODE_HOLD / MODE_CTRL: hold _vblsem at 0 for 6 s (HOLD) or not (CTRL) while
 * the injector moves the mouse; MODE_VBQ: put a routine into _vbl_list slot 0. */
#include <osbind.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#define CELL(a) (*(volatile uint32_t *)(uintptr_t)(a))
#define CELLW(a) (*(volatile int16_t *)(uintptr_t)(a))
static volatile uint32_t gv[3];
static volatile uint32_t called;
static char out[1024];
static int olen;
static long rd(void) { gv[0] = CELL(0x4BA); gv[1] = CELL(0x462); gv[2] = CELL(0x466); return 0; }
static __attribute__((unused)) long sem0(void) { CELLW(0x452) = 0; return 0; }
static __attribute__((unused)) long sem1(void) { CELLW(0x452) = 1; return 0; }
static __attribute__((unused)) void routine(void) { called++; }
static __attribute__((unused)) long slot_set(void) { CELL(0x4CE) = (uint32_t)(uintptr_t)routine; return 0; }
static __attribute__((unused)) long slot_clr(void) { CELL(0x4CE) = 0; return 0; }
static __attribute__((unused)) void until(uint32_t h) { do { (void)Bconstat(2); Supexec(rd); } while (gv[0] < h); }
static void emit(const char *tag) {
	olen += snprintf(out + olen, sizeof(out) - olen, "%s hz=%lu vbclock=%lu frclock=%lu\r\n", tag,
	    (unsigned long)gv[0], (unsigned long)gv[1], (unsigned long)gv[2]);
}
int main(void) {
	long fh; uint32_t h0 = 0; int i = 0; (void)h0;
#ifdef MODE_VBQ
	Supexec(rd); emit("start");
	Supexec(slot_set);
	for (i = 0; i < 30; i++) Vsync();
	Supexec(rd); emit("after30vsync");
	olen += snprintf(out + olen, sizeof(out) - olen, "routine called %lu times\r\n", (unsigned long)called);
	Supexec(slot_clr);
	for (i = 0; i < 5; i++) Vsync();
#else
	Supexec(rd); h0 = gv[0]; emit("start");
	until(h0 + 200); emit("t1s");
#ifdef MODE_HOLD
	Supexec(sem0);
#endif
	until(h0 + 1400); emit("t7s");
	Supexec(sem1);
	until(h0 + 2000); emit("t10s");
	(void)i;
#endif
	fh = Fcreate("VBHOLD.LOG", 0);
	if (fh < 0) return 1;
	Fwrite((short)fh, (long)olen, out);
	Fclose((short)fh);
	return 0;
}
