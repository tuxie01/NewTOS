/*
 * osbind.h	bindings for OS traps
 *
 *		++jrb bammi@cadence.com
 */

/*
 * majorly re-hacked for gcc-1.36 and probably beyond
 * all inlines changed to #defines, beacuse gcc is not
 * handling clobbered reggies correctly when -mshort.
 * We now use the Statement Exprs feature of GnuC
 *
 * 10/12/89
 *	changed all "g" constraints to "r" that will force
 *	all operands to be evaluated (or lea calculated)
 *	before the __asm__. This is necessary because
 *	if we had the (looser) "g" constraint, then sometimes
 *	we are in the situation where stuff is in the stack,
 *	and we are modifying the stack under Gcc (but eventually
 *	restoring it before the end of the __asm__), and it does
 *	not know about it (i believe there is no way to tell it
 *	this either, but you can hardly expect that). by forcing
 *	the stricter "r" constraint, we force the eval before using
 *	the val (or lea as the case may be) and we dont get into
 *	trouble.
 *	(thanks to ers for finding this problem!)
 *	[one side effect of this is that we may(depending on the
 *	  situation) actually end up with better code when the
 *	values are already in reggies, or that value is used
 *	later on (note that Gnu's reggie allocation notices the
 *	clobbered reggie, and does'nt put the value/or uses
 *	them from those reggies, nice huh!)
 *
 *  28/2/90
 *	another major re-hack:
 *	-- the basic reason: there was just no reliable
 *	way to get the definitions (inline or not does'nt matter) to
 *	fully evaluate the args before we changed the sp from under it.
 *	(if -fomit-frame-pointer is *not* used, then most of the time
 *	 we dont need to do this, as things will just reference off of
 *	 a6, but this is not true all of the time).
 *	my solution was to use local vars in the body of the statement
 *	exprs, and initialize them from the args of the statement expr block.
 *	to force the evaluation of the args before we change sp from
 *	under gcc's feet, we make the local vars volatile. we use a
 *	slight code optimization heuristic: if there are more than 4
 *	args, only then we make the local volatile, and relax
 *	the "r" constraint to "g". otherwise, we dont put the volatile
 *	and force the evaluation by putting the "r" constaint. this
 *	produces better code in most sitiations (when !__NO_INLINE__
 *	especially), as either the args are already in a register or
 *	there is good chance they will soon be reused, and in that
 *	case it will already be in a register.
 *      it may (the local vars, especially when no volatile)
 *	look like overhead, but in 99% of the situations gcc will just
 *	optimize that assignment right out. besides, this makes
 *	these defines totally safe (from re-evaluation of the macro args).
 *
 *	-- as suggested by andreas schwab (thanks!)
 *	 (schwab@ls5.informatik.uni-dortmund.de) all the retvalues are now
 *	 local register vals (see the extentions section in the info file)
 *	 this really worked out great as all the silly "movl d0,%0" at
 *	 the end of each def can now be removed, and the value of
 *	 retvalue ends up in the correct register. it avoids all the
 *	 silly "mov d0,[d|a]n" type sequences from being generated. a real win.
 *	 (note in the outputs "=r"(__retvalue) still has to be specified,
 *	 otherwise in certain situations you end up loosing the return
 *	 value in d0, as gcc sees no output, and correctly assumes that the
 *	 asm returns no value).
 *
 *	-- all the n's (the function #'s for the traps) are now given
 *	the more relaxed "g". This again results in better code, as
 *	it is always a constant, and gcc turns the movw %1,sp@- into
 *	a movw #n,sp@-. we could have given them a "i" constraint too,
 *	but "g" gives gcc more breathing room, and it does the right
 *	thing. note: the n's still need to have "r" constraints in the
 *	non-inline form (function form), as they are no longer constants
 *	in the function, but a normal arg on the stack frame, and hence
 *	we need to force evaluation before we change sp. (see osbind.c)
 *
 *	-- straps.cpp and traps.c are history. we dont need no stinking
 *	non-reentrant bindings (straps) or incorrect ones (traps.c :-)
 *
 * 03/15/92 ++jrb
 *	-- another re-hack needed for gcc-2.0: the optimization that we
 *      used earlier for traps with more than 4 args, making them volatile
 *	and using "g" constraints no longer works, because gcc has become
 *	so smart! we now remove the volatile, and give "r" constraints
 *	(just like traps with <= 4 args). that way the args are evaled
 *	before we change the stack under gcc, and at appropriate times
 *	put into reggies and pushed (or as in most cases, they are evaled
 *	straight into reggies and pushed -- and in even more common cases
 *	they are already in reggies, and they are just pushed). not doing
 *	this with -fomit-frame-pointer was causing the temps (from evaluing
 *	the args) to be created on the stack, but when we changed sp
 *	from under gccs feet, the offsets  to the temps ended up being wrong.
 *
 * 10/28/93 ++jrb
 *	relax the constraints on the inputs of trap_14_wwwwwww (only
 *	Rsconf maps to this)  to "g" from "r", as these many "r" 's
 *	give gcc 2.>3.X heartaches (understandably). note this is ok
 *	since these args will never be expressions, and we never
 *	have to constrain hard enough to force eval before we change
 *	sp from underneath gcc.
 *
 */

#ifndef _MINT_OSBIND_H
#define _MINT_OSBIND_H	1

#ifndef _FEATURES_H
# include <features.h>
#endif

#ifndef _MINT_OSTRUCT_H
# include <mint/ostruct.h>
#endif

__BEGIN_DECLS


/*
 * ---------------------------------------------------------------------------
 * NewTOS (AArch64) trap layer -- retargeted from the m68k stack-trap form.
 * ---------------------------------------------------------------------------
 *
 * The original mintlib turned every GEMDOS/BIOS/XBIOS call into a `trap #N`
 * that pushed its arguments right-to-left on the m68k stack (`movw`/`movl`
 * ... %%sp@-) and read the result from d0. NewTOS uses a REGISTER SVC ABI,
 * verified from the kernel dispatcher (kernel/src/arch/aarch64/exceptions.rs,
 * tos-api/src/trap.rs):
 *
 *   - trap class = the SVC immediate: GEMDOS = svc #1, BIOS = svc #13,
 *     XBIOS = svc #14 (matching the m68k trap numbers).
 *   - the function id goes in x8 (kernel reads its low 16 bits).
 *   - integer / pointer arguments go in x0, x1, x2, ... x7 (LEFT TO RIGHT in
 *     declaration order). The kernel captures exactly EIGHT argument
 *     registers x0..x7 (tos_api::trap::TrapArgs::regs is `[u64; 8]`).
 *   - the result comes back in x0 (a sign-extended TOS LONG; BIOS Getbpb
 *     returns a full 64-bit pointer).
 *
 * The kernel SVC entry stub saves and restores x1..x29 across the trap and
 * only overwrites x0 with the result, so -- exactly like sdk/include/newtos.h
 * -- these helpers clobber nothing but memory and the x0 result register; the
 * argument registers are inputs that survive the trap.
 *
 * Consequences for the width chars in the macro names (`w` = 16-bit word,
 * `l` = 32-bit long on m68k): with a register ABI the width no longer selects
 * a stack slot -- EACH argument occupies exactly one 64-bit register and the
 * kernel reads the width it needs (arg_i16 / arg_u16 / arg_i32 / arg_u64).
 * We keep the original per-position `(short)` / `(long)` casts so callers see
 * identical C-visible value semantics: a `w` position is sign-narrowed to 16
 * bits, an `l` position is passed as a full 64-bit long/pointer (the LP64
 * widening of the old ILP32 `long` is deliberate -- pointers are 64-bit here).
 *
 * The macro NAMES and SIGNATURES are unchanged, so all ~200 osbind/mintbind
 * bindings below (and in mintbind.h / falcon.h / metados.h) compile against
 * these helpers with no edits.
 */

/*
 * Arity helpers: `sv` is the raw SVC immediate (1 / 13 / 14), stringized into
 * the instruction; `fn` is the function id (-> x8); a0..a7 are the already
 * width-cast arguments (-> x0..x7). x0 doubles as arg0 input and result
 * output ("+r"); with zero args it is a pure "=r" output. Modelled on the
 * `register long __rN __asm__("xN")` pattern in sdk/include/newtos.h.
 */
#define __newtos_trap0(sv, fn)						\
__extension__								\
({									\
	register long __r8 __asm__("x8") = (long)(fn);			\
	register long __retvalue __asm__("x0");				\
	__asm__ volatile("svc #" #sv					\
		: "=r"(__retvalue)					\
		: "r"(__r8)						\
		: "memory");						\
	__retvalue;							\
})

#define __newtos_trap1(sv, fn, a0)					\
__extension__								\
({									\
	register long __r8 __asm__("x8") = (long)(fn);			\
	register long __retvalue __asm__("x0") = (a0);			\
	__asm__ volatile("svc #" #sv					\
		: "+r"(__retvalue)					\
		: "r"(__r8)						\
		: "memory");						\
	__retvalue;							\
})

#define __newtos_trap2(sv, fn, a0, a1)					\
__extension__								\
({									\
	register long __r8 __asm__("x8") = (long)(fn);			\
	register long __retvalue __asm__("x0") = (a0);			\
	register long __r1 __asm__("x1") = (a1);			\
	__asm__ volatile("svc #" #sv					\
		: "+r"(__retvalue)					\
		: "r"(__r8), "r"(__r1)					\
		: "memory");						\
	__retvalue;							\
})

#define __newtos_trap3(sv, fn, a0, a1, a2)				\
__extension__								\
({									\
	register long __r8 __asm__("x8") = (long)(fn);			\
	register long __retvalue __asm__("x0") = (a0);			\
	register long __r1 __asm__("x1") = (a1);			\
	register long __r2 __asm__("x2") = (a2);			\
	__asm__ volatile("svc #" #sv					\
		: "+r"(__retvalue)					\
		: "r"(__r8), "r"(__r1), "r"(__r2)			\
		: "memory");						\
	__retvalue;							\
})

#define __newtos_trap4(sv, fn, a0, a1, a2, a3)				\
__extension__								\
({									\
	register long __r8 __asm__("x8") = (long)(fn);			\
	register long __retvalue __asm__("x0") = (a0);			\
	register long __r1 __asm__("x1") = (a1);			\
	register long __r2 __asm__("x2") = (a2);			\
	register long __r3 __asm__("x3") = (a3);			\
	__asm__ volatile("svc #" #sv					\
		: "+r"(__retvalue)					\
		: "r"(__r8), "r"(__r1), "r"(__r2), "r"(__r3)		\
		: "memory");						\
	__retvalue;							\
})

#define __newtos_trap5(sv, fn, a0, a1, a2, a3, a4)			\
__extension__								\
({									\
	register long __r8 __asm__("x8") = (long)(fn);			\
	register long __retvalue __asm__("x0") = (a0);			\
	register long __r1 __asm__("x1") = (a1);			\
	register long __r2 __asm__("x2") = (a2);			\
	register long __r3 __asm__("x3") = (a3);			\
	register long __r4 __asm__("x4") = (a4);			\
	__asm__ volatile("svc #" #sv					\
		: "+r"(__retvalue)					\
		: "r"(__r8), "r"(__r1), "r"(__r2), "r"(__r3),		\
		  "r"(__r4)						\
		: "memory");						\
	__retvalue;							\
})

#define __newtos_trap6(sv, fn, a0, a1, a2, a3, a4, a5)			\
__extension__								\
({									\
	register long __r8 __asm__("x8") = (long)(fn);			\
	register long __retvalue __asm__("x0") = (a0);			\
	register long __r1 __asm__("x1") = (a1);			\
	register long __r2 __asm__("x2") = (a2);			\
	register long __r3 __asm__("x3") = (a3);			\
	register long __r4 __asm__("x4") = (a4);			\
	register long __r5 __asm__("x5") = (a5);			\
	__asm__ volatile("svc #" #sv					\
		: "+r"(__retvalue)					\
		: "r"(__r8), "r"(__r1), "r"(__r2), "r"(__r3),		\
		  "r"(__r4), "r"(__r5)					\
		: "memory");						\
	__retvalue;							\
})

#define __newtos_trap7(sv, fn, a0, a1, a2, a3, a4, a5, a6)		\
__extension__								\
({									\
	register long __r8 __asm__("x8") = (long)(fn);			\
	register long __retvalue __asm__("x0") = (a0);			\
	register long __r1 __asm__("x1") = (a1);			\
	register long __r2 __asm__("x2") = (a2);			\
	register long __r3 __asm__("x3") = (a3);			\
	register long __r4 __asm__("x4") = (a4);			\
	register long __r5 __asm__("x5") = (a5);			\
	register long __r6 __asm__("x6") = (a6);			\
	__asm__ volatile("svc #" #sv					\
		: "+r"(__retvalue)					\
		: "r"(__r8), "r"(__r1), "r"(__r2), "r"(__r3),		\
		  "r"(__r4), "r"(__r5), "r"(__r6)			\
		: "memory");						\
	__retvalue;							\
})

#define __newtos_trap8(sv, fn, a0, a1, a2, a3, a4, a5, a6, a7)		\
__extension__								\
({									\
	register long __r8 __asm__("x8") = (long)(fn);			\
	register long __retvalue __asm__("x0") = (a0);			\
	register long __r1 __asm__("x1") = (a1);			\
	register long __r2 __asm__("x2") = (a2);			\
	register long __r3 __asm__("x3") = (a3);			\
	register long __r4 __asm__("x4") = (a4);			\
	register long __r5 __asm__("x5") = (a5);			\
	register long __r6 __asm__("x6") = (a6);			\
	register long __r7 __asm__("x7") = (a7);			\
	__asm__ volatile("svc #" #sv					\
		: "+r"(__retvalue)					\
		: "r"(__r8), "r"(__r1), "r"(__r2), "r"(__r3),		\
		  "r"(__r4), "r"(__r5), "r"(__r6), "r"(__r7)		\
		: "memory");						\
	__retvalue;							\
})

/*
 *     GEMDOS  (trap #1  ->  svc #1)
 */
#define trap_1_w(n)		__newtos_trap0(1, (n))
#define trap_1_ww(n, a)		__newtos_trap1(1, (n), (short)(a))
#define trap_1_wl(n, a)		__newtos_trap1(1, (n), (long)(a))
#define trap_1_wlw(n, a, b)	__newtos_trap2(1, (n), (long)(a), (short)(b))
#define trap_1_www(n, a, b)	__newtos_trap2(1, (n), (short)(a), (short)(b))
#define trap_1_wll(n, a, b)	__newtos_trap2(1, (n), (long)(a), (long)(b))
#define trap_1_wwll(n, a, b, c)	__newtos_trap3(1, (n), (short)(a), (long)(b), (long)(c))
#define trap_1_wlww(n, a, b, c)	__newtos_trap3(1, (n), (long)(a), (short)(b), (short)(c))
#define trap_1_wlwww(n, a, b, c, d)					\
	__newtos_trap4(1, (n), (long)(a), (short)(b), (short)(c), (short)(d))
#define trap_1_wwlll(n, a, b, c, d)					\
	__newtos_trap4(1, (n), (short)(a), (long)(b), (long)(c), (long)(d))
#define trap_1_wwwll(n, a, b, c, d)					\
	__newtos_trap4(1, (n), (short)(a), (short)(b), (long)(c), (long)(d))
#define trap_1_wwllww(n, a, b, c, d, e)					\
	__newtos_trap5(1, (n), (short)(a), (long)(b), (long)(c),	\
		       (short)(d), (short)(e))

/*
 *     BIOS    (trap #13  ->  svc #13)
 */
#define trap_13_w(n)		__newtos_trap0(13, (n))
#define trap_13_ww(n, a)	__newtos_trap1(13, (n), (short)(a))
#define trap_13_wl(n, a)	__newtos_trap1(13, (n), (long)(a))
#define trap_13_www(n, a, b)	__newtos_trap2(13, (n), (short)(a), (short)(b))
#define trap_13_wwl(n, a, b)	__newtos_trap2(13, (n), (short)(a), (long)(b))
#define trap_13_wwlwww(n, a, b, c, d, e)				\
	__newtos_trap5(13, (n), (short)(a), (long)(b), (short)(c),	\
		       (short)(d), (short)(e))
#define trap_13_wwlwwwl(n, a, b, c, d, e, f)				\
	__newtos_trap6(13, (n), (short)(a), (long)(b), (short)(c),	\
		       (short)(d), (short)(e), (long)(f))

/*
 *     XBIOS   (trap #14  ->  svc #14)
 */
#define trap_14_w(n)		__newtos_trap0(14, (n))
#define trap_14_ww(n, a)	__newtos_trap1(14, (n), (short)(a))
#define trap_14_wl(n, a)	__newtos_trap1(14, (n), (long)(a))
#define trap_14_www(n, a, b)	__newtos_trap2(14, (n), (short)(a), (short)(b))
#define trap_14_wwl(n, a, b)	__newtos_trap2(14, (n), (short)(a), (long)(b))
#define trap_14_wwll(n, a, b, c)					\
	__newtos_trap3(14, (n), (short)(a), (long)(b), (long)(c))
#define trap_14_wllw(n, a, b, c)					\
	__newtos_trap3(14, (n), (long)(a), (long)(b), (short)(c))
#define trap_14_wlll(n, a, b, c)					\
	__newtos_trap3(14, (n), (long)(a), (long)(b), (long)(c))
#define trap_14_wwwl(n, a, b, c)					\
	__newtos_trap3(14, (n), (short)(a), (short)(b), (long)(c))
#define trap_14_wllww(n, a, b, c, d)					\
	__newtos_trap4(14, (n), (long)(a), (long)(b), (short)(c), (short)(d))
#define trap_14_wwwwl(n, a, b, c, d)					\
	__newtos_trap4(14, (n), (short)(a), (short)(b), (short)(c), (long)(d))
#define trap_14_wlwlw(n, a, b, c, d)					\
	__newtos_trap4(14, (n), (long)(a), (short)(b), (long)(c), (short)(d))
#define trap_14_wwwwwww(n, a, b, c, d, e, f)				\
	__newtos_trap6(14, (n), (short)(a), (short)(b), (short)(c),	\
		       (short)(d), (short)(e), (short)(f))
#define trap_14_wllwwwww(n, a, b, c, d, e, f, g)			\
	__newtos_trap7(14, (n), (long)(a), (long)(b), (short)(c),	\
		       (short)(d), (short)(e), (short)(f), (short)(g))
#define trap_14_wllwwwwlw(n, a, b, c, d, e, f, g, h)			\
	__newtos_trap8(14, (n), (long)(a), (long)(b), (short)(c),	\
		       (short)(d), (short)(e), (short)(f), (long)(g),	\
		       (short)(h))

/*
 * trap_14_wllwwwwwlw -- 9-argument XBIOS (Flopfmt, XBIOS 0x0A). NewTOS reads
 * only EIGHT argument registers (x0..x7; x8 holds the function id), so a 9th
 * argument has no register slot -- and, more fundamentally, NewTOS has NO
 * floppy hardware. Per the floppy/DMA stub decision (user 2026-07-23, spec
 * docs/api/xbios/_floppy-stubs.md), the whole floppy/DMA XBIOS group returns
 * the contract value EUNDEV (-15) WITHOUT reading its arguments in the kernel,
 * and Flopfmt is deliberately given NO 9th trap-ABI register slot. So this
 * binding must NOT issue an svc: a normal 8-register svc would silently
 * truncate / mis-pass the 9th argument. Instead it evaluates every argument
 * for its C side-effects (each is a caller expression) and yields EUNDEV (-15)
 * directly as a long. The macro signature is kept so Flopfmt() at osbind.h:633
 * still expands correctly through it.
 */
#define trap_14_wllwwwwwlw(n, a, b, c, d, e, f, g, h, i)		\
__extension__								\
({									\
	(void)(n); (void)(a); (void)(b); (void)(c); (void)(d);		\
	(void)(e); (void)(f); (void)(g); (void)(h); (void)(i);		\
	(long)(-15);		/* EUNDEV -- no floppy device on NewTOS */\
})


/* DEFINITIONS FOR OS FUNCTIONS */

/*
 *     GEMDOS  (trap1)
 */
#define	       Pterm0()					       	       \
       (void)trap_1_w((short)(0x00))
#define	       Cconin()						       \
       (long)trap_1_w((short)(0x01))
#define	       Cconout(c)					       \
       (void)trap_1_ww((short)(0x02),(short)(c))
#define	       Cauxin()						       \
       (short)trap_1_w((short)(0x03))
#define	       Cauxout(c)					       \
       (void)trap_1_ww((short)(0x04),(short)(c))
#define	       Cprnout(c)					       \
       (short)trap_1_ww((short)(0x05),(short)(c))
#define	       Crawio(data)					       \
       (long)trap_1_ww((short)(0x06),(short)(data))
#define	       Crawcin()					       \
       (long)trap_1_w((short)(0x07))
#define	       Cnecin()						       \
       (long)trap_1_w((short)(0x08))
#define	       Cconws(s)					       \
       (short)trap_1_wl((short)(0x09),(long)(s))
#define	       Cconrs(buf)					       \
       trap_1_wl((short)(0x0A),(long)(buf))
#define	       Cconis()						       \
       (short)trap_1_w((short)(0x0B))
#define	       Dsetdrv(d)					       \
       (long)trap_1_ww((short)(0x0E),(short)(d))
#define	       Cconos()						       \
       (short)trap_1_w((short)(0x10))
#define	       Cprnos()						       \
       (short)trap_1_w((short)(0x11))
#define	       Cauxis()						       \
       (short)trap_1_w((short)(0x12))
#define	       Cauxos()						       \
       (short)trap_1_w((short)(0x13))
#define	       Dgetdrv()					       \
       (short)trap_1_w((short)(0x19))
#define	       Fsetdta(dta)					       \
       (void)trap_1_wl((short)(0x1A),(long)(dta))

/*
 * ---------------------------------------------------------------------------
 * Super (GEMDOS 0x20) -- NewTOS/AArch64: a real trap again (TASK/MEM12).
 * ---------------------------------------------------------------------------
 *
 * The m68k binding is `trap_1_wl(0x20, ptr)`; so is this one. EmuTOS answers
 * the call in the TRAP #1 front end (bdos/rwa.S:159-184): x20_inq for 1L,
 * x20_usr from user mode, x20_sup from supervisor mode (:280-346).
 *
 * On NewTOS the kernel keeps a per-process SUPER STATE
 * (docs/api/_super-state.md, approved 2026-10-27). It grants exactly one
 * thing: the process's page 0 -- the classic system variables -- is
 * writable from EL0 while it is on; in user state page 0 is read-only and a
 * write there ends the program like a bus error on the Atari. The code keeps
 * running at EL0 either way; nothing else becomes privileged.
 *
 *   Super(1L)          the inquiry: 0 in user state, -1 in super state;
 *   Super(x), user     enters the super state and returns a token (the
 *                      caller's stack pointer); the stack does NOT change,
 *                      an explicit x is not used as a stack (S4);
 *   Super(x), super    leaves it (SuperToUser); returns the stack pointer.
 *
 * Until MEM12 (TASK/X01) this macro was an EL0 no-op, because page 0 was
 * writable anyway. A program built with that macro still READS its system
 * variables, but its writes fault until it is rebuilt with this header.
 */
#define	       Super(ptr)					       \
       (long)trap_1_wl((short)(0x20),(long)(ptr))

/*
 * SuperToUser -- mintlib's stack-safe way to close the bracket. On NewTOS no
 * stack is switched, so it is Super's closing half with the result discarded.
 */
#define SuperToUser(ptr)						\
	((void)Super(ptr))

#define	       Tgetdate()					       \
       (short)trap_1_w((short)(0x2A))
#define	       Tsetdate(date)					       \
       (long)trap_1_ww((short)(0x2B),(short)(date))
#define	       Tgettime()					       \
       (short)trap_1_w((short)(0x2C))
#define	       Tsettime(time)					       \
       (long)trap_1_ww((short)(0x2D),(short)(time))
#define	       Fgetdta()					       \
       (_DTA *)trap_1_w((short)(0x2F))
#define	       Sversion()					       \
       (short)trap_1_w((short)(0x30))
#define	       Ptermres(save,rv)				       \
       (void)trap_1_wlw((short)(0x31),(long)(save),(short)(rv))
#define	       Dfree(buf,d)					       \
       (long)trap_1_wlw((short)(0x36),(long)(buf),(short)(d))
#define	       Dcreate(path)					       \
       (short)trap_1_wl((short)(0x39),(long)(path))
#define	       Ddelete(path)					       \
       (long)trap_1_wl((short)(0x3A),(long)(path))
#define	       Dsetpath(path)					       \
       (long)trap_1_wl((short)(0x3B),(long)(path))
#define	       Fcreate(fn,mode)					       \
       (long)trap_1_wlw((short)(0x3C),(long)(fn),(short)(mode))
#define	       Fopen(fn,mode)					       \
       (long)trap_1_wlw((short)(0x3D),(long)(fn),(short)(mode))
#define	       Fclose(handle)					       \
       (long)trap_1_ww((short)(0x3E),(short)(handle))
#define	       Fread(handle,cnt,buf)				       \
       (long)trap_1_wwll((short)(0x3F),(short)(handle),	       \
			 (long)(cnt),(long)(buf))
#define	       Fwrite(handle,cnt,buf)				       \
       (long)trap_1_wwll((short)(0x40),(short)(handle),	       \
			 (long)(cnt),(long)(buf))
#define	       Fdelete(fn)					       \
       (long)trap_1_wl((short)(0x41),(long)(fn))
#define	       Fseek(where,handle,how)				       \
       (long)trap_1_wlww((short)(0x42),(long)(where),	       \
			 (short)(handle),(short)(how))
#define	       Fattrib(fn,rwflag,attr)				       \
       (short)trap_1_wlww((short)(0x43),(long)(fn),	       \
			  (short)(rwflag),(short)(attr))
#define	       Fdup(handle)					       \
       (long)trap_1_ww((short)(0x45),(short)(handle))
#define	       Fforce(Hstd,Hnew)				       \
       (long)trap_1_www((short)(0x46),(short)(Hstd),(short)(Hnew))
#define	       Dgetpath(buf,d)					       \
       (long)trap_1_wlw((short)(0x47),(long)(buf),(short)(d))
#define	       Malloc(size)					       \
       (long)trap_1_wl((short)(0x48),(long)(size))
#define	       Mfree(ptr)					       \
       (long)trap_1_wl((short)(0x49),(long)(ptr))
#define	       Mshrink(ptr,size)				       \
       (long)trap_1_wwll((short)(0x4A),(short)0,(long)(ptr),(long)(size))
#define	       Pexec(mode,prog,tail,env)		       \
       (long)trap_1_wwlll((short)(0x4B),(short)(mode),(long)(prog),   \
			   (long)(tail),(long)(env))
#define	       Pterm(rv)					       \
       (void)trap_1_ww((short)(0x4C),(short)(rv))
#define	       Fsfirst(filespec,attr)				       \
       (long)trap_1_wlw((short)(0x4E),(long)(filespec),(short)(attr))
#define	       Fsnext()						       \
       (long)trap_1_w((short)(0x4F))
#define	       Frename(zero,old,_new)				       \
       (short)trap_1_wwll((short)(0x56),(short)(zero),	       \
			  (long)(old),(long)(_new))
#define	       Fdatime(timeptr,handle,rwflag)			       \
       (long)trap_1_wlww((short)(0x57),(long)(timeptr),	       \
			 (short)(handle),(short)(rwflag))
#define	       Flock(handle,mode,start,length)			       \
       (long)trap_1_wwwll((short)(0x5C),(short)(handle),       \
			  (short)(mode),(long)(start),(long)(length))

/*
 *     BIOS    (trap13)
 */
#define Getmpb(ptr)					       \
       (void)trap_13_wl((short)(0x00),(long)(ptr))
#define	       Bconstat(dev)					       \
       (short)trap_13_ww((short)(0x01),(short)(dev))
#define	       Bconin(dev)					       \
       (long)trap_13_ww((short)(0x02),(short)(dev))
#define	       Bconout(dev,c)					       \
       (long)trap_13_www((short)(0x03),(short)(dev),(short)((c) & 0xFF))
/* since AHDI 3.1 there is a new call to Rwabs with one more parameter */
#define	       Rwabs(rwflag,buf,n,sector,d)			\
       (long)trap_13_wwlwww((short)(0x04),(short)(rwflag),(long)(buf), \
			     (short)(n),(short)(sector),(short)(d))
#define	       Lrwabs(rwflag,buf,n,sector,d)			\
       (long)trap_13_wwlwwwl((short)(0x04),(short)(rwflag),(long)(buf),\
			     (short)(n),(short)(-1),(short)(d),(long)sector)
#define	       Setexc(vnum,vptr) 				      \
       (void (*) (void))trap_13_wwl((short)(0x05),(short)(vnum),(long)(vptr))
#define	       Tickcal()					       \
       (long)trap_13_w((short)(0x06))
#define	       Getbpb(d)					       \
       (void *)trap_13_ww((short)(0x07),(short)(d))
#define	       Bcostat(dev)					       \
       (short)trap_13_ww((short)(0x08),(short)(dev))
#define	       Mediach(dev)					       \
       (short)trap_13_ww((short)(0x09),(short)(dev))
#define	       Drvmap()						       \
       (long)trap_13_w((short)(0x0A))
#define	       Kbshift(data)					       \
       (long)trap_13_ww((short)(0x0B),(short)(data))
#define	       Getshift()					       \
	Kbshift(-1)


/*
 *     XBIOS   (trap14)
 */

#define	       Initmous(type,param,vptr)			       \
       (void)trap_14_wwll((short)(0x00),(short)(type),	       \
			  (long)(param),(long)(vptr))
#define Ssbrk(size)					       \
       (void *)trap_14_ww((short)(0x01),(short)(size))
#define	       Physbase()					       \
       (void *)trap_14_w((short)(0x02))
#define	       Logbase()					       \
       (void *)trap_14_w((short)(0x03))
#define	       Getrez()						       \
       (short)trap_14_w((short)(0x04))
#define	       Setscreen(lscrn,pscrn,rez)			       \
       (void)trap_14_wllw((short)(0x05),(long)(lscrn),(long)(pscrn), \
			  (short)(rez))
#define	       Setpalette(palptr)				       \
       (void)trap_14_wl((short)(0x06),(long)(palptr))
#define	       Setcolor(colornum,mixture)			       \
       (short)trap_14_www((short)(0x07),(short)(colornum),(short)(mixture))
#define	       Floprd(buf,x,d,sect,trk,side,n)			       \
       (short)trap_14_wllwwwww((short)(0x08),(long)(buf),(long)(x), \
	 (short)(d),(short)(sect),(short)(trk),(short)(side),(short)(n))
#define	       Flopwr(buf,x,d,sect,trk,side,n)			       \
       (short)trap_14_wllwwwww((short)(0x09),(long)(buf),(long)(x), \
	       (short)(d),(short)(sect),(short)(trk),(short)(side),(short)(n))
#define	       Flopfmt(buf,x,d,spt,t,sd,i,m,v)		       \
       (short)trap_14_wllwwwwwlw((short)(0x0A),(long)(buf),(long)(x), \
	  (short)(d),(short)(spt),(short)(t),(short)(sd),(short)(i),  \
	  (long)(m),(short)(v))
#define	       Midiws(cnt,ptr)					       \
       (void)trap_14_wwl((short)(0x0C),(short)(cnt),(long)(ptr))
#define	       Mfpint(vnum,vptr)				       \
       (void)trap_14_wwl((short)(0x0D),(short)(vnum),(long)(vptr))
#define	       Iorec(ioDEV)					       \
       (void *)trap_14_ww((short)(0x0E),(short)(ioDEV))
#define	       Rsconf(baud,flow,uc,rs,ts,sc)			       \
       (long)trap_14_wwwwwww((short)(0x0F),(short)(baud),(short)(flow), \
			  (short)(uc),(short)(rs),(short)(ts),(short)(sc))
	/* ret old val: MSB -> ucr:8, rsr:8, tsr:8, scr:8 <- LSB */
#define	       Keytbl(nrml,shft,caps)				       \
       (void *)trap_14_wlll((short)(0x10),(long)(nrml), \
			    (long)(shft),(long)(caps))
#define	       Random()						       \
       (long)trap_14_w((short)(0x11))
#define	       Protobt(buf,serial,dsktyp,exec)			       \
       (void)trap_14_wllww((short)(0x12),(long)(buf),(long)(serial), \
			   (short)(dsktyp),(short)(exec))
#define	       Flopver(buf,x,d,sect,trk,sd,n)			       \
       (short)trap_14_wllwwwww((short)(0x13),(long)(buf),(long)(x),(short)(d),\
	       (short)(sect),(short)(trk),(short)(sd),(short)(n))
#define	       Scrdmp()						       \
       (void)trap_14_w((short)(0x14))
#define	       Cursconf(rate,attr)				       \
       (short)trap_14_www((short)(0x15),(short)(rate),(short)(attr))
#define	       Settime(time)					       \
       (void)trap_14_wl((short)(0x16),(long)(time))
#define	       Gettime()					       \
       (long)trap_14_w((short)(0x17))
#define	       Bioskeys()					       \
       (void)trap_14_w((short)(0x18))
#define	       Ikbdws(len_minus1,ptr)				       \
       (void)trap_14_wwl((short)(0x19),(short)(len_minus1),(long)(ptr))
#define	       Jdisint(vnum)					       \
       (void)trap_14_ww((short)(0x1A),(short)(vnum))
#define	       Jenabint(vnum)					       \
       (void)trap_14_ww((short)(0x1B),(short)(vnum))
#define	       Giaccess(data,reg)				       \
       (short)trap_14_www((short)(0x1C),(short)(data),(short)(reg))
#define	       Offgibit(ormask)					       \
       (void)trap_14_ww((short)(0x1D),(short)(ormask))
#define	       Ongibit(andmask)					       \
       (void)trap_14_ww((short)(0x1E),(short)(andmask))
#define	       Xbtimer(timer,ctrl,data,vptr)			       \
       (void)trap_14_wwwwl((short)(0x1F),(short)(timer),(short)(ctrl), \
			   (short)(data),(long)(vptr))
#define	       Dosound(ptr)					       \
       trap_14_wl((short)(0x20),(long)(ptr))
#define	       Setprt(config)					       \
       (short)trap_14_ww((short)(0x21),(short)(config))
#define	       Kbdvbase()					       \
       (_KBDVECS*)trap_14_w((short)(0x22))
#define	       Kbrate(delay,reprate)				       \
       (short)trap_14_www((short)(0x23),(short)(delay),(short)(reprate))
#define	       Prtblk(pblkptr)					       \
       (void)trap_14_wl((short)(0x24),(long)(pblkptr)) /* obsolete ? */
#define	       Vsync()						       \
       (void)trap_14_w((short)(0x25))
/*
 * ---------------------------------------------------------------------------
 * Supexec (XBIOS 0x26) -- NewTOS/AArch64: a real trap again (TASK/MEM12).
 * ---------------------------------------------------------------------------
 *
 * EmuTOS: `movea.l 4(a7),a0; jmp (a0)` (bios/vectors.S:1376-1378) -- the
 * callback runs in the trap's supervisor mode and its `rts` returns through
 * the dispatcher with its d0. NewTOS (docs/api/_super-state.md S6): the
 * kernel turns the super state on, runs `funcptr()` at EL0 as an upcall, and
 * on its return gives back its x0 as Supexec's result with the state of
 * before. The callback may make system calls.
 *
 * The kernel restores every general register of the caller except x0. The
 * FP/SIMD registers are not saved by an upcall, so the callback -- ordinary C
 * code -- may clobber the caller-saved ones (v0-v7, v16-v31) as any call
 * may; the asm says so.
 */
#define	       Supexec(funcptr)					       \
__extension__								\
({									\
	register long __r8 __asm__("x8") = 0x26L;			\
	register long __retvalue __asm__("x0") = (long)(funcptr);	\
	__asm__ volatile("svc #14"					\
		: "+r"(__retvalue)					\
		: "r"(__r8)						\
		: "memory", "v0", "v1", "v2", "v3", "v4", "v5", "v6",	\
		  "v7", "v16", "v17", "v18", "v19", "v20", "v21", "v22",	\
		  "v23", "v24", "v25", "v26", "v27", "v28", "v29", "v30",	\
		  "v31");						\
	__retvalue;							\
})
#define			 Puntaes() \
		 (void)trap_14_w((short)(0x27))
#define	       Floprate(drive,rate)				       \
       (short)trap_14_www((short)(0x29),(short)(drive),(short)(rate))
#define	       Blitmode(flag)					       \
       (short)trap_14_ww((short)(0x40),(short)(flag))
/*
 * Flag:
 *  -1: get config
 * !-1: set config	previous config returned
 *	bit
 *	 0	0 blit mode soft	1 blit mode hardware
 *	 1	0 no blitter		1 blitter present
 *	2..14   reserved
 *	 15	must be zero on set/returned as zero
 * blitmode (bit 0) forced to soft if no blitter(bit 1 == 0).
 */

/*
 * extensions for TT TOS
 */

#define         Mxalloc(amt,flag)					\
	(long)trap_1_wlw((short)(0x44),(long)(amt),(short)(flag))
#define		Maddalt(start,size)					\
	(long)trap_1_wll((short)(0x14),(long)(start),(long)(size))

#define         EsetShift(mode)						\
	(void)trap_14_ww((short)(80),(short)mode)
#define         EgetShift()						\
	(short)trap_14_w((short)(81))
#define         EsetBank(bank)						\
	(short)trap_14_ww((short)(82),(short)bank)
#define         EsetColor(num,val)					\
	(short)trap_14_www((short)(83),(short)num,(short)val)
#define         EsetPalette(start,count,ptr)				\
	(void)trap_14_wwwl((short)(84),(short)start,(short)count,(long)ptr)
#define         EgetPalette(start,count,ptr)				\
	(void)trap_14_wwwl((short)(85),(short)start,(short)count,(long)ptr)
#define         EsetGray(mode)						\
	(short)trap_14_ww((short)(86),(short)mode)
#define         EsetSmear(mode)						\
	(short)trap_14_ww((short)(87),(short)mode)

#define		DMAread(sector,count,buffer,devno)			\
	(long)trap_14_wlwlw((short)0x2a,(long)sector,(short)count,(long)buffer, \
			    (short)devno)
#define		DMAwrite(sector,count,buffer,devno)			\
	(long)trap_14_wlwlw((short)0x2b,(long)sector,(short)count,(long)buffer, \
			(short)devno)
#define		Bconmap(dev)						\
	(long)trap_14_ww((short)0x2c,(short)(dev))
#define		NVMaccess(op,start,count,buf)				\
	(short)trap_14_wwwwl((short)0x2e,(short)op,(short)start,(short)count, \
			(long)buf)

/*  Wake-up call for ST BOOK -- takes date/time pair in DOS format. */

#define	       Waketime(w_date, w_time)					\
       (void)trap_14_www((short)(0x2f),(unsigned short)(w_date),	\
				       (unsigned short)(w_time))

__END_DECLS


#endif /* _MINT_OSBIND_H */
