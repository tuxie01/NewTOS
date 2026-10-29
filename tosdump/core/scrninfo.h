/*
 * scrninfo.h -- the target services the SCRNINFO probe needs beyond the VDI
 * and AES traps.
 *
 * SCRNINFO.TTP records what a real EdDI implementation reports
 * (docs/api/vdi/eddi-vq_scrninfo.md, sections 2, 3 and 7/E3).  Everything it
 * asks the VDI goes through abi.h like every other tosdump program; the
 * cookie jar, the EdDI dispatcher and the date are target services, so they
 * live in their own backend (abi/m68k_scrninfo.c for TOS,
 * abi/newtos_scrninfo.c for NewTOS) and nothing in core/ touches a
 * supervisor address or a foreign function pointer directly.
 */
#ifndef TOSDUMP_SCRNINFO_H
#define TOSDUMP_SCRNINFO_H

#include <stdint.h>

/* 'EdDI', mintlib include/mint/cookie.h:125, spec section 2. */
#define TD_COOKIE_EDDI 0x45644449UL

/* More than any real jar holds; a jar that is longer is reported as cut. */
#define TD_JAR_MAX 128

struct td_cookie {
	uint32_t tag;
	uint32_t value;
};

struct td_jar {
	uint32_t jar_addr;              /* contents of _p_cookies (0x5A0)      */
	uint16_t count;                 /* entries copied, terminator excluded */
	uint16_t cut;                   /* 1: more than TD_JAR_MAX entries     */
	uint32_t end_value;             /* value of the 0 entry = jar size     */
	struct td_cookie e[TD_JAR_MAX];
};

/* Copies the cookie jar (read in supervisor mode via Supexec, as the jar
 * pointer lives at 0x5A0).  Nothing is written outside *jar. */
void td_jar_read(struct td_jar *jar);
/* DUMPBIOS links the same backend for its jar cases (TASK/TOOL18,
 * core/runner_call.c run_jar_case): one copy of the jar read. */

/*
 * Calls the EdDI dispatcher at `fn` with function number 0 per the
 * documented convention (spec section 2: "Opcode in Register d0", Pure C
 * register convention, d0-d2/a0-a1 scratch; NV250 section 3).  Returns d0
 * as the dispatcher left it, all 32 bits; Pure C's int is 16 bits, so the
 * documented result ($100 for 1.00) is the low word.
 */
uint32_t td_eddi_call0(uint32_t fn);

struct td_clock {
	uint16_t date;                  /* Tgetdate, GEMDOS 0x2A               */
	uint16_t time;                  /* Tgettime, GEMDOS 0x2C               */
	int16_t  rez;                   /* Getrez,   XBIOS 4                   */
};

void td_clock_read(struct td_clock *c);

#endif /* TOSDUMP_SCRNINFO_H */
