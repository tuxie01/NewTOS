# CLAUDE.md – NewTOS (TOS-API-compatible OS for AArch64)

<!-- This file is read automatically by Claude Code at the project root.
     It applies to all sub-crates unless overridden by a more specific CLAUDE.md. -->

## Project

**NewTOS** is a clean-room reimplementation of the Atari TOS API
(GEMDOS / BIOS / XBIOS / AES / VDI) as a native bare-metal OS for AArch64
(Raspberry Pi 4).

Goals:
- Native ARM64 OS, no emulation in the kernel core.
- Bit-exact TOS API compatibility (trap numbers, parameter ABI, error codes).
- Source-level portability for C programs written against TOS headers.
- Optional m68k userland emulator (later phase) for original Atari binaries.

Non-goals:
- Hardware-level Atari emulation (no Shifter, MFP, YM2149 in the kernel).
- Linux compatibility, POSIX compliance in the kernel, or any non-TOS API
  other than FreeMiNT-compatible extensions (e.g. `Ssystem`,
  `tos-api/src/gemdos/ssystem.rs`; ADR-027).
- x86 or x86_64 support — AArch64 only.

**Primary specification source:** EmuTOS source code, analyzed and extracted
into our own spec format during Phase 00. EmuTOS is the most accurate,
testable, complete reference for what TOS actually does. The extracted spec
lives in `docs/api/` and is what later phases implement against.

**Measured reference:** whether NewTOS behaves like EmuTOS is decided by
measurement against the pinned EmuTOS oracle — tosdump cases run in Hatari
and on NewTOS, compared with tosdiff (`docs/analysis/ORACLE.md`).

**FreeMiNT** (`references/freemint/`) is the sanctioned source for
FreeMiNT-compatible extensions and for kernel memory management (ADR-027).

**Secondary references** (used to cross-check the EmuTOS extraction):
Atari Compendium, Atari Profibuch, TOS Release Notes, Atari ST Internals.
None of them is in `references/` (see `references/README.md`).

Companion documents (read in this order at session start):
- `docs/SOURCE-ANALYSIS-PROTOCOL.md` — **mandatory protocol** for extracting
  the TOS API spec from EmuTOS source
- `docs/MEMORY.md` — project memory: ADRs with rationale, learned quirks,
  rejected alternatives, hard-won bug solutions
- `docs/REQUIREMENTS.md` — functional and non-functional requirements
- `docs/COMPATIBILITY.md` — **the compatibility contract**: what must match
  TOS exactly (syscalls, variables, structs, error codes) vs. what NewTOS
  is free to implement better. The measuring stick for "port vs. reimplement"
- `docs/ARCHITECTURE.md` — layer diagram, module boundaries, ABI rules
- `docs/STATUS.md` — current implementation state, open issues
- `docs/analysis/ORACLE.md` — the pinned EmuTOS oracle (commit, ROM, Hatari)
- `docs/api/` — **the extracted TOS API specification** (produced in Phase 00)
- `TASK/` — tickets, the unit of work: `<ID>.md` plus its receipt
- `phases/` — phase-by-phase implementation guidelines (PHASE-00 …
  PHASE-16; finished phases are history)

---

## CRITICAL RULES — read before every task

### 1. EmuTOS source is the TOS specification.

**The TOS API specification used by NewTOS is extracted from the EmuTOS
source code, function by function, in Phase 00.** This extraction lives
in `docs/api/` and is the authoritative reference for every later phase.

For any TOS API work:

- Implement against the extracted spec in `docs/api/`. If the spec entry is
  missing, incomplete or incorrect, fix the spec first from the EmuTOS
  source per `docs/SOURCE-ANALYSIS-PROTOCOL.md` (cite `file:line`), then
  implement.
- Reading source gives hypotheses; equality with EmuTOS is established only
  by measurement against the oracle (`docs/analysis/ORACLE.md`).

Compendium and Profibuch are **cross-references**, not primary sources.
EmuTOS code is what TOS actually does; documentation is approximate.

For hardware (RPi4 / AArch64), the BCM2711 datasheet and ARM ARM are
primary; see `SOURCE-ANALYSIS-PROTOCOL.md` Track B.

Writing code or specs from memory of "what TOS does" or "how ARM works"
is forbidden. The TOS API has a working reference implementation
(EmuTOS); the BCM2711 has a datasheet. There is no excuse to guess.

### 2. No stubs, ever.

A "stub" is a function that returns a hardcoded value, ignores its arguments,
or contains `todo!()` / `unimplemented!()` to make the build pass.

**Forbidden:**
```rust
pub fn fopen(_path: *const u8, _mode: u16) -> i32 {
    0  // stub – fix later
}
```

**Required when a function cannot be implemented yet:**
1. Stop work on the current task.
2. Document the missing dependency in `docs/STATUS.md` under "Blocked".
3. Ask the user whether to proceed with a different task or implement the dependency first.

`todo!()` is acceptable **only** in test scaffolding that is gated behind
`#[cfg(test)]` and never reachable in a release build.

### 3. STOP on uncertainty.

If you are unsure about:
- The TOS API semantics of a syscall
- The m68k ABI mapping for a parameter
- Whether existing code already implements something
- Which architectural layer a piece of code belongs in
- A hardware register address or bit layout for the BCM2711

→ **STOP and ask.** Do not guess. Do not invent values. Do not "make it look reasonable".

The TOS API is finite, fully documented, and bit-exact. There is no need to guess.

### 4. Debug depth limit.

When debugging a failure, you have **three attempts** to fix it. After the third
unsuccessful attempt:

1. Stop modifying code.
2. Write a brief diagnosis: what you tried, what you observed, what you suspect.
3. Hand the diagnosis to the user.

Do not enter loops of "let me try one more thing". Do not start refactoring
unrelated code. Do not delete failing tests.

### 5. Spec before code.

For any TOS API function, the specification in `docs/api/` (per-function
files such as `docs/api/gemdos/console/01-cconin.md`, or a group file) must exist
**before** implementation begins. The spec is produced via the source analysis
protocol (Rule 1), not from memory.

If the spec is missing:

1. Stop.
2. Read EmuTOS source for that function.
3. Read Compendium / Profibuch entries.
4. Draft the spec entry.
5. Get spec approval from the user.
6. Then implement.

### 6. No invented hardware.

Hardware register addresses, bit layouts, peripheral behavior, and BCM2711
quirks are sourced **only** from the hierarchy defined in
`docs/SOURCE-ANALYSIS-PROTOCOL.md` Track B.

Citing "I think this is how it works on ARM" is not acceptable.
Linux source code is a cross-reference, not an authority.

---

## Language & Communication

- Respond in **German** for design discussions, explanations, and decisions.
- Code, identifiers, comments, commit messages, doc-comments: **English**.
- File names and paths: **English**, lowercase, hyphenated. Exceptions: the
  upper-case top-level docs (`docs/STATUS.md`, `docs/analysis/ORACLE.md`, …),
  ticket files and receipts (`TASK/<ID>.md`, `<ID>-STATUS-DONE.MD`),
  `RUNINFO.TXT`, and TOS-side names (`NEWDESK.INF`, `*.NAP`).
- Be direct. No filler. No "great question!". No re-explaining what was just said.

---

## Tech Stack (non-negotiable)

- **Rust nightly** — required for the unstable cargo feature
  `per-package-target` (`kernel/Cargo.toml`). Pinned in `rust-toolchain.toml`,
  which also states the update policy. Naked functions and the inline asm in
  use are stable. Do not suggest stable until `per-package-target` stabilizes.
- **`#![no_std]`** — kernel and userland runtime. No `std`.
- **`alloc`** — yes, but only after the kernel allocator is up.
- **Cargo workspaces** — the kernel workspace (kernel, tos-api, tos-types,
  tools/xtask) with sub-crates per architectural layer, and `programs/` as a
  separate workspace for native EL0 programs (excluded from the root one).
- **MMIO** — register addresses as constants behind `mmio_va()` in
  `kernel::drivers` (the planned `cortex-a` / `tock-registers` crates were
  never adopted).
- **`spin`** or hand-rolled — no `parking_lot`, no `std::sync` primitives.
- **QEMU `raspi4b`** (`--board rpi4`) as primary test target; `aarch64-virt`
  where USB input is needed. Real RPi4 is the acceptance gate.
- **GDB** for debugging via QEMU's gdbstub.

Do **not** suggest, for the kernel and tos-api:
- C, C++, or any language that is not Rust or AArch64 assembly
- Linux kernel APIs, POSIX, libc

(The SDK is different: it ports real Atari C libraries — mintlib, gemlib,
cflib — for applications; ADR-019.)

Do **not** suggest anywhere:
- `std`, `tokio`, `async-std`, or any heap-using crate before allocator is up
- Bootloaders other than the RPi firmware's `kernel8.img` mechanism (stage 1)
- `crossbeam`, `parking_lot`, `lazy_static` — use `OnceCell` or `spin`
- x86 or x86_64 build targets
- `bindgen` to wrap EmuTOS C code — we are doing a clean reimplementation

---

## Source Tree Conventions

```
PiRusTOS/
├── CLAUDE.md                    # this file
├── Cargo.toml                   # workspace: kernel, tos-api, tos-types, tools/xtask
├── kernel/                      # crate: bare-metal kernel (arch/aarch64, drivers/bcm2711,
│                                #   hal, bios, selftest_*.rs)
├── tos-api/                     # crate: TOS syscall layer (gemdos, bdos, bios, xbios,
│   ├── src/                     #   aes, vdi, trap.rs = trap dispatcher)
│   └── tests/                   # host-side integration tests
├── tos-types/                   # crate: shared types (MD, BASEPAGE, AES/GEMDOS structs,
│                                #   error codes)
├── programs/                    # separate workspace: native EL0 programs — desktop/
│                                #   (EmuDESK rebuild, ADR-026), newdesk/, newtos-gem/, hello-gem/
├── tools/                       # host tools: xtask, make-sd-image.sh, trace/probe scripts
├── sdk/                         # app toolchain: libc, gemlib, cflib, crt0, overlays/, apps/
├── tosdump/                     # TOS reference dumper (cases/, reference/, deploy-*)
├── boot/rpi4/                   # config.txt for the boot partition
├── keyboards/                   # keyboard layout sources (.kbd)
├── docs/
│   ├── STATUS.md, MEMORY.md, REQUIREMENTS.md, ARCHITECTURE.md, COMPATIBILITY.md, …
│   ├── api/                     # extracted TOS API spec, one dir/file per group
│   ├── analysis/                # dated analyses and design notes (<topic>-YYYY-MM-DD.md)
│   ├── hardware/, sdk/, benchmarks/, presentation/ (slide decks + images)
│   ├── archive/                 # history moved out of STATUS.md etc. — never evidence
├── phases/                      # PHASE-XX-*.md implementation guidelines
├── TASK/                        # tickets: <ID>.md + <ID>-STATUS-DONE.MD / -STATUS-UNFINISHED.MD
│                                #   (also -STATUS-BLOCKED.MD, -PLAN-STATUS-DONE.MD, …)
├── measurements/                # evidence runs, one dir per run (see below)
└── references/                  # gitignored external sources (EmuTOS checkout, …);
                                 #   only references/README.md is tracked
```

Local, gitignored, never committed: `target/`, `Applications/`, `hardware/`,
`graphify-out/`, `.venv/`, `sd-card-backup-*/`.

**Placement rules:**

- **Nothing new in the repository root.** A new file belongs in one of the
  directories above; if none fits, ask.
- **Measurement / evidence runs** go to
  `measurements/measurement-YYYY-MM-DD-<ticket-ids>/` (lowercase ticket IDs,
  joined by `-`). Typical content: `RUNINFO.TXT` (what, why, how, result),
  `<run>/method/` (scripts, patches, action files), `<run>/serial/`,
  `<run>/screendumps/`.
  Per-run SD images and kernel binaries are gitignored; record their sha256.
- **Runs are immutable once committed.** Never edit files inside an existing
  run directory — several carry sha256 manifests over their own files. A
  correction is a new run or a note in the ticket.
- Cite runs from tickets/docs/code repo-relative:
  `measurements/measurement-…/RUNINFO.TXT`.
- **`docs/archive/` is history, never evidence.** Archived text records what
  was believed or done at the time; cite code, measurement runs, the oracle
  or a live doc instead.
- **Scratch** (temporary images, scratch kernels, probes) goes to
  `$CLAUDE_JOB_DIR/tmp` or the session scratchpad, never the repo or shared `/tmp`.
- Built `kernel8.img` for the card is not kept in the root; build with
  `cargo xtask build --release` when needed.

---

## Naming Conventions

- **TOS API functions**: lowercase Rust name matching Compendium spelling.
  `Fopen` → `pub fn fopen(...)`. Trap dispatch table maps function-IDs to these.
- **TOS error codes**: `pub const E_OK: i32 = 0; pub const EFILNF: i32 = -33;` etc.
  Defined in `tos-types::error`. Match Compendium values exactly.
- **Hardware registers**: `BCM2711_<PERIPHERAL>_<REGISTER>` in `kernel::drivers::bcm2711`.
- **Trap numbers**: named, not magic numbers — the dispatcher decodes them into
  `TrapClass` (`Gemdos` = 1, `Gem` = 2, `Bios` = 13, `Xbios` = 14) in
  `tos-api/src/trap.rs`.

---

## Before Suggesting Code

1. **Have you completed the source analysis per `SOURCE-ANALYSIS-PROTOCOL.md`?**
   No → STOP and do that first. Yes → continue.
2. Read the **ticket** in `TASK/` you are working on. Work outside the ticket
   list requires explicit user approval.
3. Read the **TOS API spec** for the function in `docs/api/`. If missing → see Rule 5.
4. Check `docs/MEMORY.md` for related ADRs and learned-the-hard-way knowledge.
5. Check `docs/STATUS.md` for related work in progress and open issues.
6. Reference the ticket ID, and any spec section or ADR, in commit messages
   (e.g., `fix(desk): … (DESK43 U8b)`).

---

## Commit Discipline

- Conventional Commits: `feat:`, `fix:`, `refactor:`, `docs:`, `test:`, `chore:`.
- Scope is the crate or subsystem: `feat(gemdos): ...`, `fix(mmu): ...`.
- Subject line ≤ 72 chars, imperative mood, English.
- Body explains **why**, not **what**. The diff shows the what.
- Reference ticket IDs: `(A118)`, `(DESK43 U8b)`.
- Never commit broken builds. Never commit failing tests without `#[ignore]` + reason.
- History was rewritten on 2026-10-19 (`git filter-repo`). A commit hash quoted
  in TASK/, docs/ or measurements/ before that date is an **old** hash:
  translate it with `tools/oldhash.sh <prefix>` (map: `docs/commit-map.txt`)
  and cite the current hash in new text.
- **Keep `docs/STATUS.md` in lockstep — required, after every feature, not only
  at phase ends.** For each landed extension: update the current-state section
  if it changed **and** add a dated entry at the top of *Recent Decisions*
  (newest first; what + why + the commit hashes), in the **same** work unit —
  not "later". *Recent Decisions* holds at most 10 entries: move the oldest to
  the log in `docs/archive/` in the same commit. A foreign uncommitted change already in
  `STATUS.md` is **not** an excuse to skip it: `git stash push -- docs/STATUS.md`,
  add your entry on the clean base, commit, then `git stash pop`.
- After editing `CLAUDE.md`, `docs/STATUS.md` or `docs/MEMORY.md`, run
  `cargo xtask doc-check`: every path and `file:line` cite must resolve.

---

## Testing

- Unit tests in-crate, `#[cfg(test)]`, runnable on host where logic permits.
- Host tests: `cargo xtask test` (tos-api, tos-types, xtask).
- Integration runs in QEMU via `cargo xtask run --board rpi4|qemu-virt`, driven
  by the scripts in the run's `measurements/*/method/`.
- Conformance with EmuTOS: tosdump cases against the oracle
  (`docs/analysis/ORACLE.md`).
- Hardware tests on RPi4 are manual, documented in `docs/HW-TEST-LOG.md`.
- Every TOS API function gets at least one test that verifies the documented
  return value for documented input, and one that verifies the documented
  error code for documented error input.

---

## What to do when you don't know

In order:

1. Re-read `docs/SOURCE-ANALYSIS-PROTOCOL.md`. Did you follow it?
2. Read the ticket, and the phase document if the ticket names one.
3. Read the relevant `docs/api/*.md` entry.
4. Consult the primary sources per Track A or Track B.
5. Consult the secondary sources.
6. **Ask the user.** Do not guess. Do not synthesize from "what seems reasonable".