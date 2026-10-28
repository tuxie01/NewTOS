/*
 * newtos_svc.c -- ABI backend for NewTOS on the Raspberry Pi 4 (AArch64).
 *
 * Spec section 2: marshal, execute, return.  No logic.  The symmetric
 * counterpart of abi/m68k_trap.c -- same two functions, same parameter blocks,
 * only the instruction and the pointer width differ.  Section 8 rule 5: any
 * logic that appears in one backend and not the other is a bug, however well
 * justified it looks, so nothing beyond the block setup lives here either.
 *
 * The register ABI is the one the kernel dispatcher defines, taken from the
 * already-retargeted gemlib primitives rather than re-derived:
 *
 *   sdk/gemlib/overlay/_gc_vdi.c   x0 = 115, x1 = &VDIPB, svc #2
 *   sdk/gemlib/overlay/_gc_aes.c   x0 = 200, x1 = &AESPB, svc #2
 *
 * verified there against kernel/src/arch/aarch64/exceptions.rs (the GEM branch
 * of handle_svc dispatches on x0 and passes x1) and tos-api/src/vdi/mod.rs /
 * aes/mod.rs (Vdipb::from_raw / Aespb::from_raw read consecutive native
 * pointers, i.e. the LP64 struct layout below).  The SVC entry preserves
 * x1..x29 and only overwrites x0, so this clobbers x0 and memory and nothing
 * else, and x0 on return is what the kernel left there -- the counterpart of
 * the m68k trap's d0.  Whether NewTOS produces a flip_y equivalent at all is
 * an open question in docs/api/vdi/_working-list.md (D-080); the dumper does
 * not assume an answer, it records what is there.
 */
#include <stdint.h>

#include "abi.h"

#ifndef __aarch64__
# error "newtos_svc.c is the NewTOS backend; build it for aarch64"
#endif

/* The classic parameter blocks.  Same fields as on the m68k; the pointers are
 * 64-bit here, which is the one genuine difference between the two builds. */
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
	register long x0 __asm__("x0") = 115;      /* VDI selector */
	register long x1 __asm__("x1");

	pb.contrl = contrl;
	pb.intin  = intin;
	pb.ptsin  = ptsin;
	pb.intout = intout;
	pb.ptsout = ptsout;
	x1 = (long)&pb;

	__asm__ __volatile__ (
		"svc	#2"
		: "+r"(x0)
		: "r"(x1)
		: "memory", "cc"
	);
	return (int32_t)x0;
}

int32_t td_abi_aes(int16_t *control, int16_t *global, int16_t *int_in,
                   int16_t *int_out, void **addr_in, void **addr_out)
{
	AESPB pb;
	register long x0 __asm__("x0") = 200;      /* AES selector */
	register long x1 __asm__("x1");

	pb.control  = control;
	pb.global   = global;
	pb.int_in   = int_in;
	pb.int_out  = int_out;
	pb.addr_in  = addr_in;
	pb.addr_out = addr_out;
	x1 = (long)&pb;

	__asm__ __volatile__ (
		"svc	#2"
		: "+r"(x0)
		: "r"(x1)
		: "memory", "cc"
	);
	return (int32_t)x0;
}

/*
 * The register traps (abi.h): GEMDOS svc #1, BIOS svc #13, XBIOS svc #14.
 * The register ABI is the one sdk/libc/mintlib/include/mint/osbind.h's
 * NewTOS trap layer uses (verified there against tos_api::trap): the
 * function number in x8, the arguments in x0..x7 left to right, the result
 * in x0; the SVC entry preserves x1..x29.  `value` already holds every
 * argument at register width -- a WORD sign-narrowed, a LONG sign-extended,
 * a pointer whole, an unused slot poison -- so all eight registers are
 * loaded as they stand.  `kind` says how a WORD travels on the m68k stack;
 * a register has one width, so it has nothing to say here.
 */
#define TD_SVC_TRAP(insn)                                                   \
	__asm__ __volatile__ (                                                  \
		insn                                                                \
		: "+r"(x0)                                                          \
		: "r"(x1), "r"(x2), "r"(x3), "r"(x4), "r"(x5), "r"(x6), "r"(x7),     \
		  "r"(x8)                                                           \
		: "memory", "cc")

long td_abi_trap(int16_t trap, int16_t opcode, uint16_t nargs,
                 const uint8_t *kind, const long *value)
{
	register long x0 __asm__("x0") = value[0];
	register long x1 __asm__("x1") = value[1];
	register long x2 __asm__("x2") = value[2];
	register long x3 __asm__("x3") = value[3];
	register long x4 __asm__("x4") = value[4];
	register long x5 __asm__("x5") = value[5];
	register long x6 __asm__("x6") = value[6];
	register long x7 __asm__("x7") = value[7];
	register long x8 __asm__("x8") = (long)opcode;

	(void)nargs;
	(void)kind;
	switch (trap) {
	case 1:  TD_SVC_TRAP("svc	#1");  break;
	case 13: TD_SVC_TRAP("svc	#13"); break;
	case 14: TD_SVC_TRAP("svc	#14"); break;
	default: return TD_ABI_NO_TRAP;
	}
	return x0;
}

/* abi.h: an EL0 program shares the API's address space -- a returned
 * address is the C pointer itself. */
const void *td_abi_addr(long addr)
{
	return (const void *)addr;
}
