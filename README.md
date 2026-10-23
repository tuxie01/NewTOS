# NewTOS

**The Atari TOS API, rewritten as a native operating system for 64-bit ARM.**

NewTOS boots on a Raspberry Pi 4 and gives Atari software what it expects:
GEMDOS, BIOS, XBIOS, VDI and AES with the same trap numbers, parameters,
error codes and behaviour as TOS. It is not an emulator. There is no 68000,
no Shifter, no MFP in the kernel — the TOS API itself is implemented in Rust,
running natively on the Cortex-A72.

Atari programs written in C are recompiled from their original sources with
the NewTOS SDK and run as native AArch64 programs. The upstream sources are
kept unchanged as archives. What a 64-bit, little-endian target needs in
general is fixed in the SDK (libc, headers, start-up code); what a single
program needs is kept as a separate patch file next to its sources
(`sdk/overlays/<program>/`).

The specification is EmuTOS: every TOS function is extracted from the EmuTOS
source into [`docs/api/`](docs/api/) and implemented against that, then
checked against a real Atari TT and against Hatari (TT and Falcon profiles),
down to the pixel.

> **Status:** alpha — **version 0.6** (October 2026) is out for testers.
> NewTOS runs a full GEM desktop on a real Raspberry Pi 4, runs ported
> applications and games, and was first shown in public at the OFAM Atari
> meeting in October 2026. It is not finished: one CPU core, one program at a
> time (plus desk accessories), no 68000 emulation yet. The dated state is in
> [`docs/STATUS.md`](docs/STATUS.md).

## Try it / report a bug

1. Download the latest release ZIP from the
   [Releases](../../releases) page and read its `LIESMICH.TXT`.
2. The SD card needs an **MBR** partition table and a first partition with
   **FAT16, at most 2 GB** (FAT32 and exFAT are not supported yet). Further
   FAT16 partitions (primary or logical) become drives `D:` … `P:`.
3. Copy the contents of the ZIP's `KARTE` folder onto that partition and boot.

Found something that behaves differently from a real Atari, a crash, or a
hang? **[Open an issue](../../issues/new/choose)** — the bug form asks for
what we need. A photo of the crash report helps a lot.

## What runs

**Hardware**

- Raspberry Pi 4 (BCM2711); QEMU `raspi4b` and `virt` for development
- HDMI framebuffer up to Full HD, true colour
- SD card as the hard disk: every FAT16 partition (primary or logical, up to
  2 GB each) becomes a drive, `C:` upward to `P:`; programs see the 8.3
  names; ~10 MB/s with a 32 MB read cache
- USB keyboard and mouse (incl. wheel) through the VL805 xHCI controller.
  Known limit: only devices that offer the USB HID boot protocol work, one
  interface per device. Wireless receivers such as Logitech Unifying (e.g. MX
  Master 2S, MX Ergo) and other report-protocol-only mice do not work yet
  ([#3](https://github.com/tuxie01/NewTOS/issues/3), TASK/USB01)
- US and German keyboard layouts built in, or any `C:\KEYBOARD.KBD`
- Per-program memory protection after the FreeMiNT model: a crashing program
  ends, the system keeps running
- CPU clock pinned to maximum by default (`cpuclk=firmware` to opt out)

**TOS**

- GEMDOS, BIOS, XBIOS: files and directories, memory (`Malloc`/`Mxalloc`),
  processes (`Pexec` incl. LDG module loading), console, clock, cookie jar,
  system variables — profiled after a Falcon with TOS 4
- VDI: all ~70 functional opcodes, in true colour
- AES: all eleven managers — windows, menus, dialogs with the TOS 4 3D look,
  events, resources, file selector, alerts, desk accessories
- Desktop: EmuDESK, rebuilt function by function against EmuTOS; TeraDesk
  runs on top as an alternative
- A few NewTOS extensions via `Ssystem` (CPU clock, temperature, system
  information), in the FreeMiNT style

**Ported software** (recompiled; per-program patches in `sdk/overlays/`)

- QED (text editor), TeraDesk (desktop), zView (image viewer with ten codec
  modules), EmuCON
- Doom (doomgeneric), CoreMark
- Libraries: mintlib, gemlib, cflib, WinDom, LDG, libshare, zlib, libpng,
  IJG libjpeg, giflib, libexif, OpenLibm
- Our own tools: System Info / settings accessory, `PRESENT.NAP` slide show,
  test and benchmark programs

Native programs use their own extensions — `.NAP` (GEM application), `.NCP`
(console), `.NTP` (takes parameters), `.NAC` (accessory), `.NLD` (LDG
module) — so that `.PRG`, `.TOS`, `.TTP` and `.ACC` stay free for a future
68000 emulator that runs original Atari binaries.

## Building

Requires the pinned Rust nightly (installed via `rust-toolchain.toml`) and,
for QEMU runs, `qemu-system-aarch64`.

```sh
# Kernel for the Raspberry Pi 4 -> kernel8.img
cargo xtask build --board rpi4 --release

# Kernel for QEMU virt
cargo xtask build --board qemu-virt --release

# Host-side unit tests
cargo xtask test
```

Use `--release` for anything that goes onto real hardware or is measured;
the default dev profile is much slower.

## Running

```sh
# QEMU virt, serial console on stdout
cargo xtask run --board qemu-virt --release

# QEMU raspi4b with the framebuffer window (the GEM desktop)
cargo xtask run --board rpi4 --release --window
```

[`tools/make-sd-image.sh`](tools/make-sd-image.sh) builds a test SD image for
QEMU with the desktop, the ported applications and test programs.

On a real Pi 4: put `kernel8.img`, the Raspberry Pi firmware (`start4.elf`,
`fixup4.dat`, `bcm2711-rpi-4-b.dtb`) and [`boot/rpi4/config.txt`](boot/rpi4/config.txt)
on the FAT16 boot partition (MBR, at most 2 GB). Kernel options go into `cmdline.txt`
(for example `serial=on` for the UART log, `memprot=`, `bcache=`, `cpuclk=`);
the System Info accessory shows and edits them in plain words.

## Writing programs for NewTOS

The SDK under [`sdk/`](sdk/) is a clang/LLVM cross toolchain with ported
mintlib, gemlib and cflib. Start with the developer documentation in
[`docs/sdk/`](docs/sdk/README.md): first programs, porting an existing
TOS/GEM application, the LP64 pitfalls, LDG modules, and getting a program
onto the card. New to it? The [quick start](docs/sdk/00-quickstart.md) goes
from a fresh Mac (Linux: untested) to your own program on the release card,
without building the kernel.

## Repository layout

```
kernel/        bare-metal kernel: boot, MMU, exceptions, drivers, scheduler
programs/      EL0 programs embedded in the kernel image: NEWDESK (EmuDESK rebuild)
tos-api/       the TOS API: gemdos, bios, xbios, vdi, aes, trap dispatcher
tos-types/     shared on-wire types (DTA, BPB, MD, error codes)
sdk/           app toolchain: libc, gemlib, cflib, codecs, overlays, apps
tools/         host tools: xtask, make-sd-image.sh, mkkbd, mkrsc, trace scripts
tosdump/       TOS reference dumper (reference tables from TT / Hatari)
keyboards/     keyboard layout files (.KBD)
boot/rpi4/     boot configuration for the Pi 4
docs/          architecture, requirements, compatibility contract, the
               extracted TOS API spec (api/), SDK docs (sdk/), STATUS
phases/        phase-by-phase implementation guidelines
TASK/          tickets (internal work log; bug reports go to GitHub issues)
measurements/  evidence runs: serial logs, screendumps, reference diffs
```

Commit hashes quoted in `TASK/`, `docs/` and `measurements/` before
2026-10-19 refer to the history before a one-time rewrite; translate them with
`tools/oldhash.sh <hash>` ([`docs/commit-map.txt`](docs/commit-map.txt)).

Start reading at [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) and
[`docs/COMPATIBILITY.md`](docs/COMPATIBILITY.md) — the latter is the contract
for what must match TOS exactly and where NewTOS is free to do better.

## Roadmap

- A 68000 user-mode emulator, so original `.PRG` binaries run
- Shared libraries (SLB) and multitasking the FreeMiNT way, then more cores
- Networking
- Faster drawing: the VDI raster code in hand-written AArch64 assembly
- More ported software

## References

The TOS behaviour comes from **EmuTOS**; **FreeMiNT** is the model for
extensions and memory protection. Raspberry Pi hardware beyond the BCM2711
datasheet (PCIe, the VL805 xHCI) is cross-checked against **Circle**. None of
these sources is redistributed here (see
[`references/README.md`](references/README.md)). Third-party libraries used
by the SDK are pinned as upstream archives; versions and licences are listed
in [`docs/sdk/09-port-material.md`](docs/sdk/09-port-material.md).

NewTOS was built by Ingo Uhlemann with Claude Code (Anthropic).

## License

NewTOS is licensed under the **GNU General Public License, version 2 or (at
your option) any later version** — see [`LICENSE`](LICENSE).

Some data tables and bitmaps (keyboard layouts, mouse cursor forms, alert
icons, dither patterns) are taken verbatim from EmuTOS (GPL v2+); see
[`NOTICE`](NOTICE) for the full attribution.
