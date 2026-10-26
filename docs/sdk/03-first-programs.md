# 03 — First programs: `.NCP` without libc, `.NCP` with mintlib, and a `.NAP`

All three programs below were **built and run** when this page was first
written (2026-09-17; kernel `kernel8.img` at revision `49b2c2c`, SHA-256
`a107a340…`, QEMU 11.0.2 `-M raspi4b`); the QEMU log lines, the screendump and
the SHA-256 hashes at the end are from that run and are **not** refreshed.
The three **build** recipes were re-run on 2026-10-20 at revision `a900ff0b`
(host: the toolchain from [02-prerequisites.md](02-prerequisites.md)); the file
sizes and `readobj` output quoted below are from that re-run. Sizes move
whenever a library is rebuilt — treat them as orders of magnitude, not as
reference values. The example sources are in `docs/sdk/examples/`.

Everything on this page assumes a checkout of this repository on a macOS
host. A way to do it without the repository (an installed sysroot) **does not
exist yet**; it is planned. The commands below are what the build scripts run,
spelled out; since TASK/SDK01 D the compiler wrapper `sdk/bin/newtos-cc` runs
the same commands in one step — `newtos-cc -bare`, `newtos-cc -ncp` and
`newtos-cc -nap` for the three steps below — see
[16-newtos-cc.md](16-newtos-cc.md). Linux is not described and has not been
tried.

Throughout, `$SDK` is the absolute path of `sdk/`, `$RES` is
`$(clang -print-resource-dir)/include`, `$BIN` is the Rust sysroot's
LLVM bin directory and `$MINT` is `$SDK/libc/mintlib/include` — exactly the
variables the build scripts defined when these transcripts were recorded
(`sdk/apps/gemtest/build-gemtest.sh:25-54`). *Since TASK/SDK01 K* the scripts
take `ld.lld` and `llvm-*` from the LLVM packages first and the Rust sysroot
only as fallback ([02](02-prerequisites.md#linker-and-binutils-llvms-from-the-packages-or-from-rust));
with the packages, write `ld.lld` where the transcripts below say
`"$BIN/rust-lld" -flavor gnu` or `rust-lld -flavor gnu` — same arguments,
no `-flavor`.

## Step 1 — Hello World, freestanding (`sdk/build.sh`)

No libc, no CRT beyond `sdk/crt0.S`; the program talks to the OS through the
clean-room inline-asm helpers in `sdk/include/newtos.h`.

Source (`sdk/examples/hello.c:8-14`):

```c
#include <newtos.h>

int main(void)
{
    Cconws("Hello from a C program built with clang!\r\n");
    return 0;
}
```

Command actually run, and its output:

```
$ sdk/build.sh sdk/examples/hello.c "$OUT/HELLO2.NCP"
clang : Apple clang version 21.0.0 (clang-2100.1.1.101)
lld   : rust-lld -flavor gnu

+ clang -target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 -fno-stack-protector -fPIE -fno-builtin -Wall -Wextra -I…/sdk/include -c …/sdk/crt0.S -o crt0.o
+ clang -target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 -fno-stack-protector -fPIE -fno-builtin -Wall -Wextra -I…/sdk/include -c sdk/examples/hello.c -o app.o
+ rust-lld -flavor gnu -T …/sdk/newtos.ld -pie --no-dynamic-linker -z text -z max-page-size=0x1000 --build-id=none crt0.o app.o -o …/HELLO2.NCP

wrote …/HELLO2.NCP (5968 bytes)
```

What came out (`llvm-readobj --file-headers --program-headers`):

```
Class: 64-bit (0x2)   DataEncoding: LittleEndian   Type: SharedObject (0x3)
Machine: EM_AARCH64 (0xB7)   Entry: 0x1000   ProgramHeaderEntrySize: 56
PT_LOAD  FileSize: 304  MemSize: 304  Flags: 0x5 (R-X)
PT_LOAD  FileSize: 208  MemSize: 208  Flags: 0x6 (RW-)      (.dynamic)
PT_DYNAMIC, PT_NOTE (24 bytes, the PRGFLAGS note)
```

`Type: SharedObject (0x3)` is `ET_DYN`: since ARCH03 stage 0d every program is
a position-independent executable (see "Which crt0" below), not `ET_EXEC`. That
is the shape `tos-api/src/bdos/process/elf.rs` and `loader.rs`
(`validate_segment`) accept: `PT_LOAD`s from `0x1000` (the linker script puts
`.text.boot`, i.e. `_start`, first: `sdk/newtos.ld`). The script emits **two**
segments — `text` (R-X: `.text` + dynamic metadata + `.rodata`) and `data`
(RW-: `.dynamic`, `.got`, `.data` with `.bss` as its memsz-only tail) — so the
loader can fill all six basepage segment fields; in this program the `data`
segment holds only `.dynamic`, a libc-linked one also has crt0's `argv`
in `.data`. The loader still maps every TPA
page RWX; the flags feed the basepage only.

What `crt0.S` does before `main` (`sdk/crt0.S`): read `p_hitpa` from the
basepage whose address the loader passes in `x0` (`ldr w9, [x0, #4]`; ARCH03
stage 0d — never a fixed address), set `SP` to it, `bl main`, then
`Pterm(main's return)` via `mov x8, #0x4C; svc #1`. The loader zeroes the
whole TPA, so there is no BSS clearing in crt0
(`tos-api/src/bdos/process/loader.rs:35-38`).

The `HELLO2.NCP` that `tools/make-sd-image.sh` builds from the same source is
this program; on the image it lands in `C:\`, beside the hand-crafted
`HELLO.NCP` from `tools/mk-hello-ncp.py` (a different program — a
by-hand-assembled ELF, not this C source). Both are reached by a double-click
and by nothing else: `\AUTO\` is **empty** on the default image, because the
second `HELLO.NCP` copy that used to auto-run there was dropped on 2026-09-30.

## Step 2 — Hello World with mintlib (`printf`, `malloc`, GEMDOS bindings)

Source: `docs/sdk/examples/hello-libc.c`

```c
#include <stdio.h>
#include <stdlib.h>
#include <mint/osbind.h>

int main(int argc, char **argv)
{
	char *p = malloc(32);

	printf("Hello from mintlib on NewTOS (argc=%d, argv[0]=\"%s\")\n",
	       argc, argv[0] ? argv[0] : "(null)");
	printf("Sversion() = 0x%04x, Tgettime() = 0x%04x\n",
	       (unsigned)Sversion(), (unsigned)Tgettime());
	free(p);
	return 0;
}
```

Prerequisite: `sdk/libc/libc.a` exists (build it once with
`sdk/libc/build-libc.sh`; on this host it was already current). The three
commands below are the compile/link recipe `sdk/apps/build-coremark.sh:136-170`
uses for a libc-only console program, run by hand so nothing but the output
directory is written:

```
$ clang -target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 -fomit-frame-pointer \
        -fPIE -fno-builtin -nostdinc -D_GNU_SOURCE -I"$MINT" -isystem "$RES" \
        -c hello-libc.c -o hello.o
$ clang -target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 -fPIE -fno-builtin -nostdinc \
        -I"$SDK/include" -I"$MINT" -c "$SDK/libc/crt0.S" -o crt0.o
$ "$BIN/rust-lld" -flavor gnu -T "$SDK/newtos.ld" -pie --no-dynamic-linker -z text -z max-page-size=0x1000 --build-id=none \
        crt0.o hello.o "$SDK/libc/libc.a" -o HELLOLIB.NCP
$ ls -l HELLOLIB.NCP ; llvm-nm -u HELLOLIB.NCP | grep -c .
101264 bytes ; 0 undefined symbols
```

Note the **libc crt0** (`sdk/libc/crt0.S`, not `sdk/crt0.S`): it first
calls `__libc_crtinit(basepage)` (`sdk/libc/newtos-arch/heap_init.c`,
mintlib's `_crtinit`/`_acc_main`), which sets mintlib's `_base`, builds
`environ` from the environment block and `argc`/`argv` from its `ARGV=` part
or else the basepage command tail (`__libc_parseargs()`,
`sdk/libc/newtos-arch/cmdargs.c`), applies the `_stksize` policy and shrinks
the TPA (an accessory gets a `Malloc`'ed stack instead), and returns the new
stack. Then it calls mintlib's own `_main` (`sdk/libc/mintlib/mintlib/main.c`), which sets up
the standard streams (stdout line-buffered, text mode on the console, stderr
unbuffered and, with an empty `argv[0]`, forced to the console) and ends in
`exit(main(argc, argv, environ))` — atexit handlers and the stdio flush run
on a `return` from `main` ([07-mintlib-limits.md](07-mintlib-limits.md) §5).
With no `ARGV=` and no tail — a plain double-click, or an `AUTO` run like the
one below — that is `argc = 1, argv = {"", NULL}`.

Run: the program was copied to `C:\AUTO\HELLOLIB.NCP` on a scratch image and
booted headless in QEMU. Serial log (`[auto]`/`[launch]` lines are the
kernel's, the two text lines are the program's):

```
[auto] running C:\AUTO\HELLOLIB.NCP
[launch] EL0: slot 0 entry 0x1000
Hello from mintlib on NewTOS (argc=1, argv[0]="")
Sversion() = 0x3000, Tgettime() = 0xffff92c7
[launch] returned (program Pterm'd)
```

Two things the log shows that a porter should know: `argc == 1` and
`argv[0] == ""` for a launch **without a command tail** — with one, `argv[1]…`
carry it (TASK/PORT01, see [01-overview.md](01-overview.md) and
[05 B6](05-abi-pitfalls.md#b6-argcargv-the-command-tail-and-the-environment)) —
and `Tgettime()` is a `short` in mintlib's binding
(`sdk/libc/overlay/include/mint/osbind.h:579-580`: `(short)trap_1_w((short)(0x2C))`),
so the `(unsigned)` cast above sign-extended it — mask with `& 0xFFFF`.

## Step 3 — A minimal GEM application (`.NAP`)

Source: `docs/sdk/examples/minigem.c`. It is derived from
`sdk/gemlib/tests/gemtest.c` (the `appl_init` / `graf_handle` / `v_opnvwk` /
`v_clsvwk` / `appl_exit` skeleton) and `sdk/apps/gemtest/gemtest.c`
(`wind_get(0, WF_WORKXYWH, …)`), plus one window and an `evnt_multi` loop:

```c
#include <gem.h>

static short vh;                       /* VDI virtual workstation handle */

static void draw_window(short wh)
{
	short x, y, w, h;
	short pxy[4];

	wind_get(wh, WF_WORKXYWH, &x, &y, &w, &h);
	pxy[0] = x; pxy[1] = y; pxy[2] = x + w - 1; pxy[3] = y + h - 1;
	vs_clip(vh, 1, pxy);
	vsf_color(vh, 0);                  /* white */
	vsf_interior(vh, 1);               /* solid */
	v_bar(vh, pxy);
	v_gtext(vh, x + 8, y + 24, "Hello from a NewTOS .NAP");
	vs_clip(vh, 0, pxy);
}

int main(void)
{
	short work_in[11] = { 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2 };
	short work_out[57];
	short wcell, hcell, wbox, hbox;
	short dx, dy, dw, dh;
	short wh, done = 0;
	short msg[8];
	short mx, my, mb, ks, kr, br;

	if (appl_init() < 0)
		return 1;

	vh = graf_handle(&wcell, &hcell, &wbox, &hbox);
	v_opnvwk(work_in, &vh, work_out);

	wind_get(0, WF_WORKXYWH, &dx, &dy, &dw, &dh);
	wh = wind_create(NAME | CLOSER | MOVER, dx, dy, dw, dh);
	if (wh > 0) {
		wind_set_str(wh, WF_NAME, "MINIGEM");
		wind_open(wh, dx + 32, dy + 32, 320, 160);
		draw_window(wh);
	}

	graf_mouse(ARROW, 0L);

	while (wh > 0 && !done) {
		short ev = evnt_multi(MU_MESAG | MU_KEYBD,
		                      0, 0, 0,
		                      0, 0, 0, 0, 0,
		                      0, 0, 0, 0, 0,
		                      msg, 0L,
		                      &mx, &my, &mb, &ks, &kr, &br);
		if (ev & MU_MESAG) {
			switch (msg[0]) {
			case WM_REDRAW:
				if (msg[3] == wh) {
					wind_update(BEG_UPDATE);
					draw_window(wh);
					wind_update(END_UPDATE);
				}
				break;
			case WM_MOVED:
				wind_set(wh, WF_CURRXYWH, msg[4], msg[5], msg[6], msg[7]);
				break;
			case WM_CLOSED:
				done = 1;
				break;
			}
		}
		if (ev & MU_KEYBD)
			done = 1;
	}

	if (wh > 0) {
		wind_close(wh);
		wind_delete(wh);
	}
	v_clsvwk(vh);
	appl_exit();
	return 0;
}
```

Two gemlib details that matter on LP64:

* The window title is set with `wind_set_str(wh, WF_NAME, "MINIGEM")`, **not**
  `wind_set(wh, WF_NAME, ptr_hi, ptr_lo, 0, 0)`. gemlib's plain `wind_set`
  takes four `short`s (`sdk/gemlib/gemlib-0.44.0/mt_gem.h:2456`); the string
  variant packs the 8-byte pointer at `int_in[2..5]`
  (`sdk/gemlib/overlay/a_wind_set_str.c:11-14`), which is where the kernel reads it for an LP64
  caller (`tos-api/src/aes/mod.rs:310-330`, `tos-api/src/aes/window.rs:3329-3348`).
* The `evnt_multi` prototype has 22 parameters
  (`sdk/gemlib/gemlib-0.44.0/mt_gem.h:2315-2319`); `Interval` is an `unsigned long`, so pass `0L`.

Built with the **same flags as `sdk/apps/gemtest/build-gemtest.sh:65-70`**
(and therefore as `build-teradesk.sh`), run by hand:

```
$ clang -target aarch64-unknown-none-elf -ffreestanding -nostdlib -Os -fomit-frame-pointer \
        -fPIE -fno-builtin -nostdinc -funsigned-char -D__NEWTOS__ -D_TOS_ONLY -Wno-macro-redefined \
        -I"$SDK/gemlib/gemlib-0.44.0" -I"$MINT" -isystem "$RES" -c minigem.c -o minigem.o
$ clang -target aarch64-unknown-none-elf -ffreestanding -nostdlib -Os -fomit-frame-pointer \
        -fPIE -fno-builtin -nostdinc -funsigned-char -D__NEWTOS__ -D_TOS_ONLY -Wno-macro-redefined \
        -I"$SDK/include" -I"$MINT" -c "$SDK/libc/crt0.S" -o crt0.o
$ "$BIN/rust-lld" -flavor gnu -T "$SDK/newtos.ld" -pie --no-dynamic-linker -z text -z max-page-size=0x1000 --build-id=none \
        crt0.o minigem.o --start-group "$SDK/gemlib/gemlib.a" "$SDK/libc/libc.a" --end-group -o MINIGEM.NAP
$ ls -l MINIGEM.NAP ; llvm-readobj --file-headers MINIGEM.NAP | grep -E 'Class|Type|Machine|Entry'
56280 bytes
Class: 64-bit   Type: SharedObject   Machine: EM_AARCH64   Entry: 0x1000
$ llvm-nm -u MINIGEM.NAP | grep -c .          → 0
$ llvm-objdump -d MINIGEM.NAP | grep -cE 'svc\s+#0x2'   → 2   (<aes> at 0x1400, <vdi> at 0x1a70)
```

`-D__NEWTOS__` selects the `svc #2` primitives in the gemlib overlay headers
(`sdk/gemlib/overlay/gem_aesP.h:21-31`); `-D_TOS_ONLY` is TeraDesk's
single-TOS switch (`build-teradesk.sh:87-94`) and is harmless for a program
that does not read `_MINT_`. The include order — gemlib top, then mintlib, then
clang's headers — is what the scripts use (`build-gemtest.sh:68-70`).

Run: `MINIGEM.NAP` was copied to `C:\` on the scratch image and started
through the `NEWDESK.INF` autorun line `#Z 01 C:\MINIGEM.NAP@`
(see [06-image-and-testing.md](06-image-and-testing.md)). Serial log:

```
[auto] 2 AUTO program(s) run
[shell] sh_ldapp: starting C:\MINIGEM.NAP, sh_nextapp=2, sh_isgem=1
[launch] EL0: slot 0 entry 0x1000
```

QMP screendump 60 s after boot (`docs/sdk/examples/minigem-qemu-raspi4b-2026-09-17.png`):
a 640×480 green desktop background, the menu bar reading `MINIGEM.NAP`, a
window titled `MINIGEM` with a close box at (32,50)–(352,210) showing the text
`Hello from a NewTOS .NAP`, and the arrow cursor. The program is parked in
`evnt_multi` — QEMU `raspi4b` has no input device, so it cannot be closed
there; on a real Pi 4 a key press or the close box ends it.

Artifacts of the 2026-09-17 run (SHA-256; later library rebuilds changed the binaries): `HELLO2.NCP` `c829c420…`,
`HELLOLIB.NCP` `266e6704…`, `MINIGEM.NAP` `aee28971…`.

## Which crt0, which linker script, which archives

| Program kind | crt0 | Link line | Archives |
|--------------|------|-----------|----------|
| freestanding `.NCP` | `sdk/crt0.S` | `sdk/build.sh:70-71` | none |
| libc `.NCP` | `sdk/libc/crt0.S` | `sdk/libc/build-libc.sh:232-234` | `libc.a` |
| gemlib `.NAP` | `sdk/libc/crt0.S` | `sdk/gemlib/build-gemlib.sh:182-184` | `gemlib.a libc.a` |
| cflib `.NAP` | `sdk/libc/crt0.S` | `sdk/cflib/build-cflib.sh:174-178` | `--start-group cflib.a gemlib.a libc.a --end-group` |

The linker script is always `sdk/newtos.ld`; every object is compiled
`-fPIE` and the program is linked as a position-independent executable
(`ET_DYN`, ARCH03 stage 0d) whose only dynamic relocations are
`R_AARCH64_RELATIVE` — the kernel loader applies them and refuses an `ET_EXEC`
image with `EPLFMT`. The link flags are always
`-pie --no-dynamic-linker -z text -z max-page-size=0x1000 --build-id=none`. The `.NCP`/`.NAP` name is
chosen by the caller — it is the same ELF either way; only the desktop's launch
behaviour differs ([01-overview.md](01-overview.md)).

**PRGFLAGS.** Both crt0 files emit an ELF note `.note.newtos.prgflags` (owner
`"NewTOS"`, type 1, one `u32`) that `sdk/newtos.ld` keeps under a `PT_NOTE`
header; the loader takes the program's `PRGFLAGS` (`p_flags`) from it and picks
the pool of the TPA and environment by `PF_TTRAMLOAD` (ARCH03 E8b, format in
`docs/api/gemdos/process/4b-pexec.md`). The default is `PF_STANDARD` (7). To
build a program with other flags, assemble crt0 with
`-DNEWTOS_PRGFLAGS=<value>` — `sdk/build.sh` does that from the environment
variable `PRGFLAGS` (`PRGFLAGS=0x4 sdk/build.sh prog.c PROG.NCP`) — or change a
linked file with the host tool, the counterpart of Atari's `PRGFLAGS.PRG`:

```sh
$ cargo xtask prgflags PROG.NCP           # show
PROG.NCP: PRGFLAGS 0x00000007 (PF_FASTLOAD | PF_TTRAMLOAD | PF_TTRAMMEM)
$ cargo xtask prgflags PROG.NCP 0x4       # set, in place
```

A program without the note (linked with a crt0 from before ARCH03 stage 2)
loads with `PF_STANDARD`, as before.
