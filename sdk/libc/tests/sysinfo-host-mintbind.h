/*
 * sysinfo-host-mintbind.h -- forced into the host build of mintlib's
 * unix/sysinfo.c (the overlay sdk/libc/overlay/unix/sysinfo.c) and of
 * sdk/libc/newtos-arch/getcookie.c by sysinfo-host.sh (TASK/SDK03).
 * <mint/mintbind.h> and <mint/osbind.h> are held back there
 * (-D_MINT_MINTBIND_H -D_MINT_OSBIND_H: their macros are AArch64 `svc' traps);
 * the four calls the two files make go to sysinfo-host.c instead, with
 * mintbind.h's argument order. sysinfo-host.c's Ssystem plays the kernel's
 * S_GETCOOKIE (tos-api/src/gemdos/ssystem.rs, s_getcookie): pointer form
 * writes four bytes and returns 0, value form returns the value as a 32-bit
 * signed long, a missing tag returns -1 (ERROR).
 */
#ifndef SYSINFO_HOST_MINTBIND_H
#define SYSINFO_HOST_MINTBIND_H
long sh_Ssystem (int, long, long);
long sh_Super (long);
long sh_SuperToUser (void *);
long sh_Sversion (void);
#define Ssystem(m, a, b)	sh_Ssystem ((int) (m), (long) (a), (long) (b))
#define Super(s)		sh_Super ((long) (s))
#define SuperToUser(s)		sh_SuperToUser ((void *) (s))
#define Sversion()		sh_Sversion ()
#endif
