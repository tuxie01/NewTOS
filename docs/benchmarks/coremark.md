# CoreMark 1.0 on NewTOS

EEMBC CoreMark 1.0, ported to NewTOS/AArch64 in `3d54e3f`. Sources live in
`sdk/apps/coremark/` (upstream `github.com/czietz/coremark` @ `b546802`,
pinned by `sdk/apps/coremark-sources.md5`); the benchmark's own translation
units are compiled **unmodified**. Only `core_portme.{c,h}` is ours, in
`sdk/overlays/coremark/newtos/` — a normal CoreMark port directory, the same
role the upstream tree's `atari/` plays.

## Result — real Raspberry Pi 4 hardware, 2026-09-04

Measured on the machine, launched by double-click from the built-in desktop.

```
CoreMark 1.0 : 7380.085101 / clang 21.0.0 (clang-2100.1.1.101)
  -target aarch64-unknown-none-elf -O2 -fomit-frame-pointer / Code and data in RAM
```

| field | value |
|---|---|
| Iterations/Sec | 7380.085101 |
| Iterations | 110000 |
| Total time | 14.905215 s (over the required 10 s minimum) |
| Total ticks | 2981 (200 Hz tick, `04aaf16`) |
| CoreMark Size | 666 |
| seedcrc | 0xe9f5 |
| crclist / crcmatrix / crcstate | 0xe714 / 0x1fd7 / 0x8e3a |
| crcfinal | 0x33ff |

`Correct operation validated.`

The four fields CoreMark requires for a reportable result — score, compiler,
compiler flags, memory location — are in the quoted line above.

## Validation run — same hardware, same session

CoreMark requires a second run with the other seed set. Done, `COREVAL.NCP`:

| field | value |
|---|---|
| Iterations/Sec | 2942.266366 |
| Iterations | 40000 |
| Total time | 13.595029 s |
| Total ticks | 2719 |
| seedcrc | 0x18f2 |
| crclist / crcmatrix / crcstate | 0xe3c1 / 0x0747 / 0x8d84 |
| crcfinal | 0xbe81 |

`Correct operation validated.`

The first four CRCs are byte-identical to the QEMU control run of the same
binary (`crcfinal` differs there only because that run did 110000 iterations
against 40000 here). **With this, the result is complete under CoreMark's
reporting rules** — a performance run and a validation run, both validated.

## Cross-check: the algorithm CRCs match other machines

`seedcrc`, `crclist`, `crcmatrix` and `crcstate` are **identical** on every
machine below. Two compilers, three unrelated instruction sets, the same bits.
This is the real evidence that the port is correct; the speed is secondary.
(`crcfinal` legitimately differs — it depends on the iteration count.)

| machine | CoreMark | iterations | time | seedcrc/crclist/crcmatrix/crcstate |
|---|---:|---:|---:|---|
| **NewTOS, Raspberry Pi 4 (Cortex-A72)** | **7380.09** | 110000 | 14.91 s | e9f5 / e714 / 1fd7 / 8e3a |
| FireBee (ColdFire V4e, 264 MHz) | 27.35 | 400 | 14.63 s | e9f5 / e714 / 1fd7 / 8e3a |
| PAK68/3 (68020 + 68881, 32 MHz) | 13.03 | 200 | 15.35 s | e9f5 / e714 / 1fd7 / 8e3a |

**Do not quote the 270x against that FireBee number.** Those two 68k-family
runs were built `-O2 -mcpu=68000` — plain 68000 code on a ColdFire, not its
own instruction set. The published list has a properly built FireBee at
**466.74**, seventeen times higher. Use that one.

## Placement in the published Atari list

`github.com/czietz/coremark/wiki/Results` collects 50+ machines measured with
the same port, from an 8 MHz stock ST to the FireBee.

| reference | CoreMark | NewTOS is |
|---|---:|---:|
| FireBee — fastest entry on the list | 466.74 | **16x** |
| Falcon CT60 @ 85 MHz | 192.74 | 38x |
| TT 68030 @ 16 MHz | 7.80 | 946x |
| ST with 68020 @ 32 MHz | 4.80 | 1538x |
| Stock ST @ 8 MHz | 1.92 | 3844x |

**16x against the FireBee** is the claim that survives a follow-up question.

## How to reproduce

1. `sdk/apps/build-coremark.sh` → `CORE.NCP` (performance) and `COREVAL.NCP`
   (validation) in `sdk/apps/coremark/build/`. The script verifies the source
   pinning first and refuses to build on a mismatch.
2. `cargo xtask build --board rpi4 --release`.
3. Copy both `.NCP` to `C:\COREMARK\` and `kernel8.img` to the boot partition.
4. Boot, open `C:`, folder `COREMARK`, double-click `CORE.NCP`. Banner, then
   ~30 s of deliberately blank screen (calibration + the timed run), then the
   result and `Press any key to exit.` The result stays up until a key is
   pressed. The desktop itself does not wait after a program ends (as on
   EmuTOS); the port's `portable_fini` does, after the timed region
   (`sdk/overlays/coremark/newtos/core_portme.c`; TASK/DESK70,
   `measurements/measurement-2026-11-07-desk70-build/RUNINFO.TXT`).
5. Repeat with `COREVAL.NCP` — the mandatory validation run with the second
   seed set. It prints no score, only `Correct operation validated`.

Not via `\AUTO\`: an AUTO run finishes before the AES starts and the desktop
paints over the result. From `\AUTO\` the port does not wait for a key: it
waits only when its current directory is not the root, and an AUTO program
starts in the root of the boot drive. So measurement images that boot both
programs from `C:\AUTO\` run through to the desktop; the report is on the
serial log.

## Notes on the measurement

- **Clock:** `boot/rpi4/config.txt` keeps `force_turbo=0` and NewTOS has no
  clock management, so the SoC clock during the run is not asserted here.
  User's decision, 2026-09-04: the system is benchmarked as it is used, not
  configured up for the benchmark. CoreMark does not require the clock among
  its reported fields; it is only needed for a CoreMark/MHz figure.
- **The QEMU score is meaningless** as a CPU figure — it measures the host
  doing the emulating. QEMU runs only prove that the run completes, the CRCs
  match and the output is complete.
- `make check` in the upstream tree is unusable: its bundled md5 list is stale
  and fails even on a pristine checkout. `sdk/apps/coremark-sources.md5`
  replaces it and records the one upstream difference (`core_list_join.c`,
  `list_data info;` vs `= {0};`, with no reachable effect).

## Still open

- Console programs launched by double-click **from TeraDesk** print nothing
  (the built-in desktop and `\AUTO\` are fine). Under investigation; that is
  why the run above went through the built-in desktop.
