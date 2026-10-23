# 06 — Getting a program onto the image, and testing it

## The SD image: `tools/make-sd-image.sh`

```
tools/make-sd-image.sh [image-path] [size-MiB]
    image-path   defaults to /tmp/sd.img (what `cargo xtask run --sd` expects)
    size-MiB     defaults to 64
```
(`tools/make-sd-image.sh:17-26`). It creates an MBR with one FAT16 (type
`0x06`) partition, attaches it as `/Volumes/NEWTOS`, populates it, unmounts,
and verifies the MBR (`:36-44`, `:294-310`). macOS only (`hdiutil`,
`diskutil`, `dot_clean`; header comment `tools/make-sd-image.sh:13`). **There
is no Linux (or Windows) way to build the image yet** — it is a known gap
(TASK/SDK01 package G, planned); until then a Linux user has to build a FAT16
image by other means. To run only your own program on a ready-made release image without
building the kernel, follow [00-quickstart](00-quickstart.md) §4 (release
`kernel8.img` in QEMU, or the v0.6 card on a Pi). This page covers the full
repository workflow with `make-sd-image.sh` and `cargo xtask run`.

**This table and the line numbers in it date from 2026-09 and are
incomplete.** The script has since gained more install blocks (for example
`C:\Apps\PRESENT\`, the zView block `C:\ZVIEW\` at
`tools/make-sd-image.sh:414`, the MXPROBE/FPPEXEC/LDGTEST test programs) and `cicons.rsc` for TeraDesk; read the script itself
for the current list.

What it installs, and from where (nothing is built here except the
freestanding examples):

| On the image | Source | Built by | Lines |
|--------------|--------|----------|-------|
| `C:\HELLO.NCP` | `tools/mk-hello-ncp.py` (hand-crafted ELF) | the script | `:71-83` |
| `C:\HELLO2.NCP`, `BIG.NCP`, `PHYSBASE.NCP`, `SCRFLIP.NCP`, `DTACHILD.NCP` | `sdk/examples/*.c` | `sdk/build.sh`, run **by the script** | `:78-136` |
| `C:\PTERMCH.NCP`, `C:\CTRLCPAR.NCP` (PROC06: the `Pexec` exit-code child for APITEST T33, and the Ctrl-C'd-child proof, which waits for a key) | `sdk/examples/{ptermch,ctrlcpar}.c` | `sdk/build.sh`, run **by the script** | DTACHILD block |
| `C:\USBSNAP.NCP` (USB01: the USB device recorder, writes one text file per device to `C:\USB\`; spec `docs/api/usb/usb-snapshot.md` §4) | `sdk/examples/usbsnap.c` | `sdk/build.sh`, run **by the script** | USBSNAP block |
| `C:\LIBTEST.NCP`, `C:\CLOCKTST.NCP` | `sdk/libc/build/{libtest,clocktest}.NCP` | you: `sdk/libc/build-libc.sh [sdk/libc/tests/clocktest.c]` | `:143-166` |
| `C:\SETJMPT.NCP`, `C:\MATHTST.NCP` (T07) | `sdk/libc/build/setjmptst.NCP`, `sdk/libm/build/mathtst.NCP` | you: `sdk/libc/build-libc.sh sdk/examples/setjmptst.c sdk/libc/build/setjmptst.NCP`, `sdk/libm/build-libm.sh` | `:169-200` |
| `C:\COREMARK\{CORE,COREVAL}.NCP` | `sdk/apps/coremark/build/` | you: `sdk/apps/build-coremark.sh` | `:189-201` |
| `C:\DUMPVDI.NAP`, `C:\VDI.TSV`, `C:\VDIDRAW.NAP` | `tosdump/build/` | you: `make -C tosdump all` | `:224-242` |
| `C:\Apps\QED\{QED.NAP,qed.rsc}` | `sdk/apps/qed/build/` | you: `sdk/apps/build-qed.sh` **and** the manual `mkrsc.py convert` ([04](04-porting-anatomy.md)) | `:259-267` |
| `C:\Apps\TERADESK\{DESKTOP.NAP,desktop.rsc,icons.rsc}` | `sdk/apps/teradesk/build/dist/` | you: `sdk/apps/build-teradesk.sh` | `:277-286` |
| `C:\NEWDESK.INF` | the literal text `newdesk placeholder` | — | `:54` |

Missing inputs are skipped with a `note:` line, not an error.

**`\AUTO\` is empty on the default image** (since 2026-09-30): only
`\AUTO\READ.ME` is there, saying what the folder is for. The `HELLO.NCP` copy
that used to auto-run at boot was dropped. Nine environment switches copy a
program into `\AUTO\` on request (`grep -o 'SD_[A-Z0-9_]*_AUTO'
tools/make-sd-image.sh`, 2026-10-20): `SD_SCRFLIP_AUTO=1`, `SD_SYSINFO_AUTO=1`,
`SD_CLOCKTEST_AUTO=1`, `SD_SETJMPTST_AUTO=1`, `SD_MATHTST_AUTO=1`,
`SD_COREMARK_AUTO=1`, `SD_FPPEXEC_AUTO=1`, `SD_MXPROBE_AUTO=1`,
`SD_LDGTEST_AUTO=1`. Every one of them names a program that ends by itself —
a program that waits for a key would stop the boot from there.

**Copying with `mtools` instead.** The quick start's `mcopy -i sd.img@@1M`
assumes its own image (partition at sector 2048). `make-sd-image.sh` puts
the partition at **sector 63**, so for its images the offset is
`63 × 512 = 32256`: `mcopy -i sd.img@@32256 -o MYAPP.NAP ::/APPS/MYAPP/`
(check with `mdir -i sd.img@@32256 ::/`; the script prints `start_lba=` at
the end). No mounting, no AppleDouble files. Used for every run of TASK/SDK01 H.

**Adding your own program.** The project rule is that the script is the
single source of truth for the base image — add an install block to it
rather than hand-editing `/tmp/sd.img` (memory note *auto-folder-console-only*,
"Image discipline"). For a one-off test on a **scratch** image the manual
route is what the tester recipe and this document used:

```sh
DEV=$(hdiutil attach -imagekey diskimage-class=CRawDiskImage -nomount "$IMG" | head -1 | awk '{print $1}')
diskutil mount "${DEV}s1"                       # → /Volumes/NEWTOS
cp MYAPP.NAP /Volumes/NEWTOS/
printf '#R 02\r\n#Z 01 C:\\MYAPP.NAP@\r\n' > /Volumes/NEWTOS/NEWDESK.INF
dot_clean -m /Volumes/NEWTOS                    # see the warning below
diskutil unmount "${DEV}s1" && hdiutil detach "$DEV"
```

**Warning — AppleDouble files.** Copying with Finder or `cp` onto the FAT
volume creates `._NAME.NCP` side files. Their names *end in `.NCP`*, so
`run_auto_folder` would try to load them (it classifies by the last four
bytes, `programs/desktop/src/ftype.rs:50-70`) and the desktop would list them as
programs. `make-sd-image.sh` strips them with `dot_clean -m` (`:291`); do the
same after any manual copy (observed while writing this page: a
`._HELLOLIB.NCP` appeared in `\AUTO\` and was removed before the run).

## Three ways a program starts

### `C:\AUTO\` — console programs, before the desktop

`kernel/src/launch.rs:199-248` (`run_auto_folder`): every file in
`C:\AUTO\*.*` whose extension classifies as **`.NCP`** is run in directory
order, each to its `Pterm`, before the AES and the desktop exist. `.NAP` and
`.NTP` are skipped (`:216-226`); a `.NAP` needs `appl_init`, which is not
available yet. Holding **Control** on the boot screen skips the folder
(`:204-207`). Output goes to the framebuffer console and the serial line and
is painted over when the desktop comes up.

Serial log of the run made for this document (two `.NCP` in `\AUTO\`):

```
[auto] running C:\AUTO\HELLOLIB.NCP
[launch] EL0: slot 0 entry 0x1000
Hello from mintlib on NewTOS (argc=1, argv[0]="")
…
[launch] returned (program Pterm'd)
[auto] running C:\AUTO\HELLO.NCP
…
[auto] 2 AUTO program(s) run
```

### `NEWDESK.INF` `#Z` — a GEM application instead of the desktop

The AES reads `C:\NEWDESK.INF` at start (`tos-api/src/aes/init.rs:355-410`,
EmuTOS `process_inf2`). A line

```
#Z 01 C:\MYAPP.NAP@
```

is parsed as: tag `Z`, skip the blank, two hex digits = `isgem`, skip three
bytes, the path up to `@` (`init.rs:386-399`). The path is split into
directory and program name and handed to `sh_wdef`; `isauto` becomes true
(`:400-409`). The shell loop then starts **that program instead of the
desktop**, and enters the desktop only when it ends
(`kernel/src/lib.rs:401-408`, `shell_loop::sh_main`). `isgem = 01` means a
GEM application; `00` would start it in character mode (`init.rs:414-415`,
EmuTOS `(*isauto && !isgem) ? FALSE : TRUE`).

This is how every headless QEMU measurement launches a `.NAP` — QEMU
`raspi4b` has no input device, so double-clicking is impossible there
(memory note *qemu-dump-runs-via-newdesk-inf-autorun*). The desktop's own
`NEWDESK.INF` parser (`programs/desktop/src/inf.rs:373-379`) reads the same line
and writes it back on save (`:497-506`), so the autorun survives a desktop
"Save Desktop".

Serial log of the run made for this document:

```
[shell] sh_ldapp: starting C:\MINIGEM.NAP, sh_nextapp=2, sh_isgem=1
[launch] EL0: slot 0 entry 0x1000
```

`make-sd-image.sh` writes **no** `#Z` line (`:54`); add it by hand as above.

### Double-click on the desktop (real hardware only)

`programs/desktop/src/main.rs:467-500` (`launch_native`): `.NAP`/`.NTP` go
through `run_gem_app` (desktop relinquish → run → resume); `.NCP` clears the
screen, runs, waits for a key (`console_pause_for_key`, `:610`), and redraws
the desktop. Needs a USB keyboard/mouse, which QEMU `raspi4b` cannot provide.

## Running in QEMU (`raspi4b`)

```
cargo xtask run --board rpi4 --sd /tmp/sd.img            # headless, serial on stdout
cargo xtask run --board rpi4 --sd /tmp/sd.img --window   # with the emulated HDMI window
cargo xtask run --board rpi4 --sd /tmp/sd.img --release  # release-profile kernel
cargo xtask run --board rpi4 --sd /tmp/sd.img --debug    # halted, gdbstub on :1234
```

`cmd_run_rpi4` (`tools/xtask/src/main.rs:525-563`) builds the kernel for
`rpi4`, produces `kernel8.img` in the workspace root by `objcopy -O binary`
(`:355-366`), and runs

```
qemu-system-aarch64 -M raspi4b -no-reboot -display none|cocoa -serial stdio \
    -kernel kernel8.img -append serial=on -drive file=<sd>,if=sd,format=raw [-S -s]
```

Facts about that machine that matter for a test:

* **Serial output is off unless the boot arguments say `serial=on`**
  (SER01, 2026-10-02). On a real Pi the switch is `cmdline.txt` on the boot
  partition; on `raspi4b` QEMU's firmware model answers the kernel's "get
  command line" mailbox query with the `-append` string, so every hand-made
  run that wants a serial log needs `-append serial=on`. Without it the log
  stays empty (a panic or fatal exception still prints). An older measurement
  script that lacks the option gets it back without being edited by running it
  as `PATH="$PWD/tools/qemu-serial-on:$PATH" sh <script>` — that directory's
  `qemu-system-aarch64` adds `-append serial=on` to `-M raspi4b` runs and
  starts the real binary.

* The display is **640×480** (`[fb] VideoCore display geometry detected:
  640x480` in the serial log), the same profile as an Atari TT — which is why
  QEMU is the preferred oracle for conformance dumps.
* **No input**: `[pcie] RC not present (no response at RC_BASE); USB disabled`
  — the Pi's USB sits behind PCIe/xHCI, which `raspi4b` does not emulate. A
  `.NAP` parked in `evnt_multi` stays there; only autorun-able runs are
  possible — unless you build a **scratch kernel** with a scripted IKBD
  mouse injector, which feeds mouse packets from the AES input pump on a
  timetable. That is how the SDK01 H run clicked a link, a toolbar button,
  a scroll arrow and the close box of HypView:
  `measurements/measurement-2026-10-21-sdk01h/method/scratch-kernel-injector.diff`
  (against `kernel/src/host_env.rs` of 2026-10-21; the older H03 diff,
  `measurements/measurement-2026-09-17-h03/method/`, no longer applies — it
  calls a `tos_api::vdi::mouse_position` that is gone). Build it in a copy of
  the workspace, never in the shared tree; it is test scaffolding, not a
  substitute for the hardware check.
* The SD controller **is** emulated, so `C:` works.
* Boot to `[desk] desktop up` (or to the `#Z` program) takes roughly 25 s,
  15 s of which are the boot info screen (tester recipe, memory note
  *qemu-dump-runs-via-newdesk-inf-autorun*; not timed separately here — the
  run for this document had the `#Z` program on screen at the first
  screendump, 45 s after launch).

### Screendumps via QMP

`xtask run` opens no QMP socket. For a screendump, start QEMU yourself with
the same arguments plus `-qmp unix:<sock>,server,nowait` and `-serial
file:<log>`, then use `tools/qmpshot.c` (compile with `cc -o qmpshot
tools/qmpshot.c`; it exists because `nc -U` is blocked by the agent sandbox,
`qmpshot.c:8-24`):

```sh
cc -o qmpshot tools/qmpshot.c
qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:serial.log \
    -qmp unix:/tmp/claude-501/<job>-qmp.sock,server,nowait \
    -kernel kernel8.img -append serial=on -drive file=sd.img,if=sd,format=raw &
sleep 45 && ./qmpshot /tmp/claude-501/<job>-qmp.sock shot.ppm
sips -s format png shot.ppm --out shot.png        # QEMU writes PPM
```

The UNIX socket path must be **under 104 bytes** (QEMU refuses longer ones —
observed: the scratchpad path of this job was 107). The screendump in
`docs/sdk/examples/minigem-qemu-raspi4b-2026-09-17.png` was taken this way.

### The board footgun

`cargo xtask build --board rpi4` and `--board qemu-virt` write the **same**
`target/aarch64-unknown-none-softfloat/{debug,release}/kernel`. After a build
for the other board, cargo may report "up to date" and hand you the wrong
ELF (symptom: empty serial log / hang). `cargo clean -p kernel` does not
reliably help. Build fresh for the board you are about to boot and check the
ELF's mtime (memory note *cargo-xtask-board-target-footgun*; `cargo xtask run`
always rebuilds).

## Running in QEMU (`virt`) with more USB devices

`raspi4b` has the card but no USB; `virt` has the xHCI controller but **no
drive**. `cargo xtask run` (board `qemu-virt`, the default) attaches a
`usb-kbd` and a `usb-mouse`; two options add more (USB01 K11,
`qemu_usb_args` in `tools/xtask/src/main.rs`):

```
cargo xtask run --usb-extra                   # + usb-tablet (root port 3, report protocol only)
                                              #   + usb-hub (root port 4) with usb-mouse 4.1, usb-kbd 4.2
cargo xtask run --usb-dev usb-tablet          # one more QEMU -device spec (repeatable)
cargo xtask run --usb-dev usb-hub,bus=usb.0,port=4 --usb-dev usb-mouse,bus=usb.0,port=4.1
```

With either option the controller gets `id=usb`, so a spec can name its
bus and port (`bus=usb.0,port=<root>[.<hub port>]`); without them the
command line is unchanged. Both are refused with `--board rpi4`. The boot
log line `[usb] enumeration done: N devices (K keyboard/mouse, O observed
only)` shows what the kernel made of them. To see what a program like
`USBSNAP` reads on this board, a measurement build has to replace the
embedded desktop program in a scratch copy of the workspace (no drive, so no
program can be started from `C:`):
`measurements/measurement-2026-10-23-usb01-snap-program/method/run-all.sh`.

## Real Raspberry Pi 4

`docs/HW-TEST-LOG.md:55-80`: the boot partition is FAT, holds `start4.elf`,
`fixup4.dat`, `boot/rpi4/config.txt` (with `init_uart_clock=48000000`,
`gpu_mem=256`, `arm_64bit=1`, `kernel=kernel8.img`) and the `kernel8.img`
from

```sh
cargo xtask build --board rpi4 --release
```

Build **release** for hardware and performance runs — the default profile is
the slow dev build (memory note *flash-release-builds-to-real-hw*). The test
card's layout is four 500 MB FAT16 partitions: `BOOT` = `C:` (firmware,
`kernel8.img`, and the programs), `NEWTOS1..3` = `D:`–`F:` (memory note
*sd-card-real-hw-layout*). Flashing the card is a user/lead action, not
something an agent does on its own (memory note *workers-no-hardware-or-outward-actions*).

What only the hardware can test: anything interactive (double-click, menus,
typing, closing a window), 1920×1080 geometry, and the cache/DMA behaviour of
the framebuffer (`docs/HW-TEST-LOG.md:122`). Serial output is available on
the Pi's UART at 115200 8N1 (`HW-TEST-LOG.md:29-36`).

## Host tests: `sdk/run-host-tests.sh`

Not everything needs a kernel. The gemlib bindings and the libc arch shims
have **host tests** in `sdk/gemlib/tests/` and `sdk/libc/tests/`: small C
programs the host `cc` compiles together with the overlaid gemlib source
(or `newtos-arch/setjmp.S`), which call the binding, capture the parameter
block it built and check the words — no trap, no QEMU. They run on any LP64
little-endian host (macOS/arm64 here) and refuse to certify from one whose
word width or byte order differs from the AArch64 target.

`sdk/run-host-tests.sh` builds and runs all of them in one pass and exits
with the number of failures:

| Test | What it pins | Ticket |
|------|--------------|--------|
| `obspec_bits` | the `BFOBSPEC` bit-field overlay matches the GEM `ob_spec` layout | — |
| `evnt_timer_words` | `mt_evnt_timer` hands `int_in[0]` = low, `int_in[1]` = high | H03 |
| `wind_set_addrin` | `wind_set_str/_ptr/_ptr_int` pass the pointer in `addr_in[0]`, AIN_LEN 1 | A81 |
| `intin_long_words` | `*_intin_long`/`*_intout_long` are 32-bit `[lo, hi]` in place; `wind_get_ptr(WF_SCREEN)` reads the LONG (under AddressSanitizer) | T04 |
| `ptr_word_count` | the six pointer-in-`intin` bindings declare `N_PTRINTS` WORDs | T04 |
| `aesglobal_fields` | TeraDesk's `library.h` + overlay address `ap_ptree`/`ap_rscmem` as 4-byte fields at byte 10/14 (skipped without the vendored TeraDesk source) | A60 |
| `setjmp-host` (`sdk/libc/tests/setjmp-host.sh`) | `newtos-arch/setjmp.S` round-trips on an AArch64 host (skipped elsewhere) | T07 |

Each test's own header carries its build line, so a single one can still be
run by hand. `sdk/gemlib/tests/gemtest_verify.c` is not in the runner: it is a
historical ILP32 *target* program without a builder (see its header), not a
host test. Neither is this runner a substitute for `cargo run -p xtask -- test`
(the kernel/tos-api host tests) or for the QEMU runs above.

## Reading the serial log

Lines a port will look for:

| Line | Meaning | Source |
|------|---------|--------|
| `[auto] running C:\AUTO\X.NCP` / `[auto] N AUTO program(s) run` | AUTO folder pass | `kernel/src/launch.rs:233-243` |
| `[launch] EL0: slot N entry 0x1000` | the loader dropped to EL0 at `e_entry` | `kernel/src/launch.rs:117` |
| `[launch] returned (program Pterm'd)` | clean exit | `kernel/src/launch.rs:134` |
| `[shell] sh_ldapp: starting …, sh_isgem=1` / `… exited with rc=N` | `#Z` autorun or `shel_write` launch, and its exit | `kernel/src/shell_loop.rs:222`, `:316` |
| `[EXC] Data Abort (lower EL): ESR=… ELR=… FAR=…` | the program faulted at EL0; `ELR` is the faulting PC in the program's own image (`llvm-objdump -d` it), `FAR` the address | `kernel/src/arch/aarch64/exceptions.rs:133`, `:1571` |
| `[PANIC] …` | kernel panic (debug builds check slice alignment etc. — see [05](05-abi-pitfalls.md#b8)) | — |
| `[desk] desktop up (N drives)` | the desktop program `NEWDESK` has reached its first `evnt_multi` (ready for input); printed by the kernel, the desktop itself prints nothing (DESK43 D4) | `kernel/src/shell_loop.rs` (`observe_desktop_gem_call`) |
| `[desk-selftest] begin` … `[desk-selftest] done` | the desktop's four self-tests (`--features selftest-desk`, writes to C:) | `programs/desktop/src/selftest.rs` |

A GEMDOS trap trace is available with `--features gemdos-trace`
(`docs/analysis/q07-…md:29`); it is a debug feature, not part of a normal run.
