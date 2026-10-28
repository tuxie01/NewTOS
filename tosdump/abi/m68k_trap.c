/*
 * m68k_trap.c -- ABI backend for TOS 3.06 on the TT.
 *
 * Spec section 2: marshal, execute, return.  No logic.  Everything this file
 * does is build the classic parameter block around the caller's arrays and
 * issue trap #2.
 *
 * The trap sequences are the ones gemlib issues on the m68k, taken from the
 * pristine tree rather than written from memory:
 *
 *   VDI  gem_vdiP.h:47-62   d1 = &VDIPB, d0 = 115, trap #2
 *   AES  gem_aesP.h:38-45   d1 = &AESPB, d0 = 200, trap #2
 *
 * with the same clobber list (d0/d1/d2, a0/a1/a2, memory, cc).  gemlib also
 * fills contrl[] inside its trap macro; this backend must not -- contrl is
 * already laid out by core/, and duplicating that here would be logic in a
 * place where the two builds are required to agree.
 */
#include <stdint.h>

#include "abi.h"

#if !defined(__mc68000__) && !defined(__M68K__)
# error "m68k_trap.c is the TOS 3.06 backend; build it with m68k-atari-mint-gcc"
#endif

/* The classic parameter blocks: five and six 32-bit pointers. */
typedef struct {
	int16_t *contrl;
	int16_t *intin;
	int16_t *ptsin;
	int16_t *intout;
	int16_t *ptsout;
} VDIPB;

typedef struct {
	int16_t *control;
	int16_t *global;
	int16_t *int_in;
	int16_t *int_out;
	void   **addr_in;
	void   **addr_out;
} AESPB;

int32_t td_abi_vdi(int16_t *contrl, int16_t *intin, int16_t *ptsin,
                   int16_t *intout, int16_t *ptsout)
{
	VDIPB pb;
	long ret;

	pb.contrl = contrl;
	pb.intin  = intin;
	pb.ptsin  = ptsin;
	pb.intout = intout;
	pb.ptsout = ptsout;

	/* d0 on return is flip_y (vdi_asm.S:124-125); it is an output of the
	 * call, so it is moved out before the clobber takes effect. */
	__asm__ volatile (
		"move.l %1,%%d1\n\t"      /* &VDIPB          */
		"move.w #115,%%d0\n\t"    /* VDI selector    */
		"trap   #2\n\t"
		"move.l %%d0,%0"
		: "=d"(ret)
		: "g"(&pb)
		: "d0", "d1", "d2", "a0", "a1", "a2", "memory", "cc"
	);
	return (int32_t)ret;
}

int32_t td_abi_aes(int16_t *control, int16_t *global, int16_t *int_in,
                   int16_t *int_out, void **addr_in, void **addr_out)
{
	AESPB pb;
	long ret;

	pb.control  = control;
	pb.global   = global;
	pb.int_in   = int_in;
	pb.int_out  = int_out;
	pb.addr_in  = addr_in;
	pb.addr_out = addr_out;

	__asm__ volatile (
		"move.l %1,%%d1\n\t"      /* &AESPB          */
		"move.w #200,%%d0\n\t"    /* AES selector    */
		"trap   #2\n\t"
		"move.l %%d0,%0"
		: "=d"(ret)
		: "g"(&pb)
		: "d0", "d1", "d2", "a0", "a1", "a2", "memory", "cc"
	);
	return (int32_t)ret;
}

/*
 * The register / stack traps (abi.h): GEMDOS trap #1, BIOS trap #13, XBIOS
 * trap #14.  The frame is what mintlib's trap_13_wwl & friends push
 * (sdk/libc/mintlib/include/mint/osbind.h, the m68k original): the
 * arguments right to left, a WORD as one WORD and a LONG as two, most
 * significant first, then the function number on top.  It is laid here in
 * a WORD array in exactly that memory order and copied onto the stack in
 * one loop, so the frame does not depend on how many arguments a case has.
 *
 * Every operand the asm still needs after it has moved the stack pointer is
 * in a register ("a"/"d"), never a stack-relative memory operand, and none
 * of them is in d0-d2/a0-a2, which the trap may clobber and which the asm
 * uses itself -- the clobber list keeps the compiler out of them.  The
 * trap returns with d3-d7/a3-a6 intact (bios/vectors.S, biosxbios saves
 * them), so the frame size survives it.
 */
#define TD_M68K_FRAME_WORDS (1 + 2 * TD_ABI_ARGS_MAX)

#define TD_M68K_TRAP(insn)                                                  \
	__asm__ volatile (                                                      \
		"move.l %1,%%a0\n\t"      /* a0 = end of the frame          */      \
		"move.l %2,%%d1\n\t"      /* d1 = WORDs - 1, for dbra       */      \
		"1:\n\t"                                                            \
		"move.w -(%%a0),-(%%sp)\n\t"                                        \
		"dbra   %%d1,1b\n\t"                                                \
		insn "\n\t"                                                         \
		"add.l  %3,%%sp\n\t"      /* pop the frame                  */      \
		"move.l %%d0,%0"                                                    \
		: "=&d"(ret)                                                        \
		: "a"(end), "d"(count), "d"(bytes)                                  \
		: "d0", "d1", "d2", "a0", "a1", "a2", "memory", "cc")

long td_abi_trap(int16_t trap, int16_t opcode, uint16_t nargs,
                 const uint8_t *kind, const long *value)
{
	uint16_t frame[TD_M68K_FRAME_WORDS];
	uint16_t *end;
	long ret = TD_ABI_NO_TRAP, count, bytes;
	int n = 0, i;

	frame[n++] = (uint16_t)opcode;
	for (i = 0; i < (int)nargs && i < TD_ABI_ARGS_MAX; i++) {
		if (kind[i] == TD_ABI_ARG_WORD) {
			frame[n++] = (uint16_t)value[i];
		} else {
			frame[n++] = (uint16_t)((unsigned long)value[i] >> 16);
			frame[n++] = (uint16_t)value[i];
		}
	}
	end   = frame + n;
	count = n - 1;
	bytes = 2L * n;

	switch (trap) {
	case 1:  TD_M68K_TRAP("trap   #1");  break;
	case 13: TD_M68K_TRAP("trap   #13"); break;
	case 14: TD_M68K_TRAP("trap   #14"); break;
	default: break;
	}
	return ret;
}

/* abi.h: the dumper shares TOS's flat 24/32-bit address space -- a returned
 * address is the C pointer itself. */
const void *td_abi_addr(long addr)
{
	return (const void *)addr;
}
