/*
 * abi.h -- the ONLY thing that differs between the two builds.
 *
 * Spec section 2: "two thin ABI backends, and nothing else may differ ...
 * marshal registers, execute, return.  No logic."  If a difference in results
 * can be explained by the backends rather than by the API implementations,
 * the whole exercise is worthless.
 *
 * A backend implements exactly these two functions:
 *
 *   abi/m68k_trap.c    TOS 3.06 on the TT   -- trap #2, d0 = 115 / 200
 *   abi/newtos_svc.c   NewTOS on the Pi 4   -- svc #2, x0 = 115 / 200
 *
 * The arrays are the caller's; a backend builds the target's parameter block
 * around them, executes the trap and returns.  The parameter block layout is
 * the one place where the two targets genuinely differ (32-bit pointers on
 * m68k, 64-bit on AArch64), which is why it lives inside the backend.
 */
#ifndef TOSDUMP_ABI_H
#define TOSDUMP_ABI_H

#include <stdint.h>

/*
 * Both return what the trap left in its return register -- d0 on the m68k, x0
 * on AArch64.  For the VDI that is flip_y (docs/api/vdi/_dispatcher.md); it is
 * an output of the call, so the backend hands it back rather than discarding
 * it, and the runner records it.
 */

/* contrl[] must already carry opcode, counts, sub-opcode and handle. */
int32_t td_abi_vdi(int16_t *contrl, int16_t *intin, int16_t *ptsin,
                   int16_t *intout, int16_t *ptsout);

/* control[] must already carry opcode and the four parameter counts. */
int32_t td_abi_aes(int16_t *control, int16_t *global, int16_t *int_in,
                   int16_t *int_out, void **addr_in, void **addr_out);

/*
 * The register / stack traps: GEMDOS (1), BIOS (13), XBIOS (14).
 *
 * `kind[i]` says how argument i travels -- the one thing the two ABIs
 * disagree about:
 *
 *   TD_ABI_ARG_WORD  one WORD on the m68k stack (the low 16 bits of
 *                    value[i]); on AArch64 the register gets value[i],
 *                    which the caller has already sign-narrowed.
 *   TD_ABI_ARG_LONG  a LONG or a pointer: two WORDs on the m68k stack, most
 *                    significant first; on AArch64 the whole register.
 *
 * `value` always holds TD_ABI_ARGS_MAX entries.  The m68k backend pushes
 * the first `nargs` right to left under the function number, issues the
 * trap and pops the frame; the AArch64 backend loads all eight into x0..x7
 * (the caller fills the unused ones with poison) and the function number
 * into x8, as sdk/libc/mintlib/include/mint/osbind.h's NewTOS trap layer
 * does.  Marshalling only, no logic (spec section 2, section 8 rule 5).
 *
 * Returns the return register at its native width -- d0 (32 bits) on the
 * m68k, x0 (64 bits) on AArch64 -- as a `long`, which is exactly that width
 * on both.  An unknown trap number issues nothing and returns
 * TD_ABI_NO_TRAP; the case loader never lets one through.
 */
#define TD_ABI_ARG_WORD  1
#define TD_ABI_ARG_LONG  2
#define TD_ABI_ARGS_MAX  8
#define TD_ABI_NO_TRAP   ((long)(int32_t)0xDEADDEADuL)

long td_abi_trap(int16_t trap, int16_t opcode, uint16_t nargs,
                 const uint8_t *kind, const long *value);

#endif /* TOSDUMP_ABI_H */
