# REQUIREMENTS — NewTOS

## Functional Requirements

### FR-1: TOS API Compatibility

The system shall implement the GEMDOS, BIOS, and XBIOS APIs as documented
in the Atari Compendium, with bit-exact semantics for trap numbers,
function IDs, parameter ABIs (m68k stack-based and AArch64 PCS), and
return/error codes.

### FR-2: Native AArch64 Execution

The system shall execute native AArch64 binaries built against the TOS
API, with no m68k emulation overhead for native programs.

### FR-3: Source-Level Compatibility

C source code written for TOS (using `<tos.h>`, `<gemdos.h>` etc. headers)
shall compile for NewTOS with no functional changes, given a NewTOS-aware
header set and toolchain.

### FR-4: Optional m68k Binary Compatibility

Original Atari TOS .PRG binaries shall execute via an integrated m68k
user-space emulator, sharing the native TOS API layer through ABI
marshaling.

### FR-5: Storage

The system shall read and write FAT16 / FAT32 filesystems on SD cards
(RPi4) and virtio-blk devices (QEMU virt).

### FR-6: Display

The system shall provide TOS-compatible screen modes via software
rendering composited to an HDMI framebuffer.

### FR-7: Input

The system shall accept keyboard input via UART (initial) and USB HID
(later phase).

## Non-Functional Requirements

### NFR-1: Codebase

100% Rust + AArch64 assembly. No C, no C++. The only exception is
TOS-source-port test binaries built externally with a C compiler.

### NFR-2: Build Reproducibility

The kernel shall build deterministically from a clean checkout with a
documented Rust toolchain version, producing byte-identical
`kernel8.img` artifacts.

### NFR-3: No Stubs in Production

Production code shall not contain function stubs that ignore arguments
or return hardcoded values. Functions either implement their spec or
do not exist.

### NFR-4: Documented Decisions

All architectural decisions where the spec is silent or implementation-
defined shall be recorded as ADRs in `docs/MEMORY.md`.

### NFR-5: Hardware Targets

- Primary development target: QEMU `aarch64-virt` machine
- Primary deployment target: Raspberry Pi 4 Model B
- Future targets are explicitly out of scope until the primary targets
  are stable

### NFR-6: License

NewTOS is released under the GNU General Public License, version 2 or (at
your option) any later version (`LICENSE`, `NOTICE`; commit `aec5b730`).
EmuTOS source is read for reference; beyond that, a few data tables and
bitmaps are taken from it verbatim and attributed in `NOTICE`.
RuSTari is integrated under its own license (MIT, per RuSTari project).

## Constraints

### C-1: Standalone Project

NewTOS does not depend on any host operating system at runtime. It boots
directly from RPi firmware on metal.

### C-2: Single-User

Multi-user support is not a goal. TOS was always single-user; NewTOS
inherits this.

### C-3: Cooperative Multitasking

Preemptive multitasking is not a goal of the initial release.
Cooperative multitasking (à la MultiTOS) is acceptable when GEM lands.
