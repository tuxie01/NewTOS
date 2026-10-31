# RTC01 — Battery-backed clock: DS3231 on I2C as a BIOS clock driver

Status: **OFFEN** (after the Atari meeting) · Priority: P3 · Type: new hardware driver

## Background

NewTOS has no RTC. The boot clock starts at the kernel's build date,
00:00:00, like EmuTOS's `DEFAULT_DATETIME` (since 2026-10-31, TASK/XBIOS12;
`tos-api/src/gemdos/time.rs:247@time_init`; before that the FAT timestamp of
`C:\KERNEL8.IMG`), then advanced by the ARM
generic timer; `docs/HARDWARE-PROFILE.md:53` records "none on-board". The
code already anticipates swapping the clock *source* without touching the
TOS API (`Tgetdate`/`Tgettime`/`Tsetdate`/`Tsettime`, XBIOS `Gettime`/`Settime`).

The user has a module (photos 2026-10-08): "MH" board with **DS3231SN** RTC
and **AT24C32** EEPROM on one I2C bus, 4.7 kΩ pull-up arrays on board, pins
GND/VCC/SDA/SCL/SQW/32K, address jumpers A0–A2 (EEPROM).

## Decision (user, 2026-10-08)

BIOS-level driver, not an AUTO program or accessory: EL0 programs cannot
reach the I2C controller under memory protection, and an accessory would
only run after the AES is up (AUTO programs and boot logs would keep the
wrong time). Setting the clock from the System Info accessory comes later.

## Scope

1. I2C master driver for BSC1 (GPIO 2/3, Pi header pins 3/5) — register
   layout from `references/datasheets/RP-008248-DS-1-bcm2711-peripherals.pdf`
   only (CLAUDE.md rule 6).
2. DS3231 read/write of the time-keeping registers — from the DS3231
   datasheet (to be acquired into `references/datasheets/`; not there yet).
3. Boot: seed the wall clock from the DS3231 if present; otherwise keep the
   build-date start (`DEFAULT_DATETIME`, TASK/XBIOS12). Never block or panic boot.
4. `Settime`/`Tsettime`/`Tsetdate` write back to the DS3231, following
   EmuTOS `bios/clock.c` / `bios/nvram.c` (how EmuTOS seeds the boot clock and
   writes back) — cite file:line.
5. `Ssystem` info mode / System Info: "Clock: DS3231 found" (human-readable).

## Hardware notes to verify before the first power-on

- Power from **3.3 V (pin 1)**, never 5 V: the on-board pull-ups would put
  SDA/SCL at 5 V on the Pi's 3.3 V GPIOs.
- Modules of this type carry a coin-cell charging path (diode + 200 Ω next
  to the DS3231). With a non-rechargeable CR2032 that path must be removed,
  or an LIR2032 used. Verify against the board's schematic/datasheet.

## Acceptance

QEMU raspi4b has no DS3231: host tests for BCD conversion and the
register/packed-time mapping; boot without module unchanged in QEMU; on the
real Pi: time survives a power cycle, `Settime` persists.
