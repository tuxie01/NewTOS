# `NVMaccess` (XBIOS 0x2E) [BIOS19]

> Researcher output for TASK/BIOS19 (work item 2, document part). Source:
> EmuTOS @ `1a7484ab`. Consistent with TASK/XBIOS10.md (ACCEPTED 2026-11-12,
> ADR-012) and `docs/COMPATIBILITY.md` ("XBIOS for absent Atari hardware:
> NVRAM, TT shifter, DSP", `:1953`).

> **Contract vs. reference** — `docs/COMPATIBILITY.md`. Contract on NewTOS:
> trap ID/ABI and the one answer `0x2E` (below). Reference (free): everything
> EmuTOS does with the NVRAM chip.

## NewTOS status: not wired, answers `0x2E` by decision

NewTOS has no NVRAM. `NVMaccess` takes the dispatcher's catch-all arm and
returns its own function number, `0x002E`, whatever the arguments
(`tos-api/src/xbios/mod.rs:162@other`; the arm's comment names "NVRAM 0x2E",
`:154`). `0x2E` is also what EmuTOS returns on a machine without an NVRAM chip
(`references/emutos/bios/nvram.c:161-162`, `!has_nvram`). Owner decision
2026-11-12 (TASK/XBIOS10.md, "Owner decision 2026-11-12: accepted"):
independent of `_MCH`, no NVRAM is emulated; reopen when a port needs it.
The buffer is not touched (measured, below).

## EmuTOS source

**Primary location:** `bios/nvram.c:155-214` — `WORD nvmaccess(WORD type, WORD
start, WORD count, UBYTE *buffer)`; slot 0x2E of `xbios_vecs[]`
(`bios/xbios.c:1349-1353`: `VEC(xbios_2e, nvmaccess)` with `CONF_WITH_NVRAM`,
else `xbios_unimpl`). The wrapper `xbios_2e` (`bios/xbios.c:871-878`) exists
only for the `DBG_XBIOS` debug build.

**Build configuration:** the whole of `nvram.c` is under `CONF_WITH_NVRAM`
(`bios/nvram.c:22`), 1 for `make 512` (`docs/api/_emutos-config.md:104,197`); the oracle ROM links
`_nvmaccess` (`references/emutos-oracle/1a7484ab/emutos.map:688`).
That macro only compiles the code in; whether it runs is the runtime
variable `has_nvram` (`include/has.h:107-108`, `bios/nvram.c:44`), set once at
boot by `detect_nvram` = `check_read_byte(NVRAM_ADDR_REG)`
(`bios/nvram.c:49-54`, called at `bios/machine.c:586`). A machine without the
chip (an ST) has `has_nvram` 0. The oracle profiles `tt` and `falcontc` have
the chip; no oracle profile without it exists (XBIOS10 dropped the ST-profile
measurement, owner 2026-11-12).

**Hardware:** address register `0xFFFF8961`, data register `0xFFFF8963`
(`bios/nvram.c:24-25`). Layout: 14 RTC registers, then 50 bytes of RAM of
which the user may access 48 (`NVRAM_USER_SIZE`), the last 2 are the checksum
(`bios/nvram.c:27-31`).

## Behaviour on EmuTOS (decision order, as coded)

| Step | Condition | Result | Cite |
|---|---|---|---|
| 1 | `!has_nvram` | return `0x2E`, nothing else touched, for **every** `type` and argument | `bios/nvram.c:161-162` |
| 2 | `type == 2` (reset) | zero the 48 user bytes; on a TT (`cookie_mch == MCH_TT`) store the checksum of the zeroes, otherwise write the 6-byte block `{0x00,0x2F,0x20,0xFF,0xFF,0xFF}` at user offset 8 through a recursive `nvmaccess(1, …)`; return 0. **No argument check before it** | `bios/nvram.c:164-176`, block `:41-42` |
| 3 | `buffer == NULL`, `start < 0`, `count < 1` or `start + count > 48` | return `-5` (`EBADRQ`, `include/gemerror.h:24`) | `bios/nvram.c:178-179` |
| 4a | `type == 0` (read) | recompute the checksum; if it differs from the stored one return `-12` (`EGENRL`, `include/gemerror.h:31`) and read nothing; else copy `count` bytes from `start` into `buffer`; return 0 | `bios/nvram.c:182-199`, `compute_sum` `:98-111`, `get_sum` `:117-` |
| 4b | `type == 1` (write) | write `count` bytes from `buffer`, store the new checksum, return 0 | `bios/nvram.c:200-207` |
| 4c | any other `type` | return `-5` | `bios/nvram.c:208-210` |

The checksum word is `MAKE_UWORD(~sum, sum)` of the byte sum of the 48 user
bytes (`bios/nvram.c:111`). Arguments are WORDs (`type`, `start`, `count`)
and a byte pointer; the return is a WORD (the tosdump table row reads
`EXACT.W`, `tosdump/cases/bios-r1.tsv:189@NVMaccess`).

**Callers inside EmuTOS and their fallback** (read): `detect_akp` reads user
bytes 6-7 (`bios/country.c:150`, error: keep the ROM country), `detect_idt`
bytes 8-9 (`bios/country.c:171`, error: country default), the boot video mode
bytes 14-15 (`bios/screen.c:529`, error: default mode), the SCSI host id byte
16 (`bios/scsi.c:305`, error: default id), and the RTC registers through
`get_nvram_rtc`/`set_nvram_rtc` (`bios/clock.c:870-907`, only `if (has_nvram)`).
A program that gets a non-zero return takes the same "no usable NVRAM" path.

## Trap interface

- **Trap number:** 14 (XBIOS), **Function ID:** `0x2E`.
- **Arguments (order and width from the C signature and the dumper rows,
  `bios/nvram.c:155`, `tosdump/cases/bios-r1.tsv:189@NVMaccess`):** `type` WORD, `start`
  WORD, `count` WORD, `buffer` pointer. m68k stack offsets follow the usual
  XBIOS rule (WORDs 2 bytes, pointer 4 bytes after the function word; inferred,
  not read from `xbiosbind.h`). NewTOS reads none of them.
- **Return:** WORD in `D0`/`x0`. NewTOS: `0x002E`, the function number
  zero-extended (`tos-api/src/xbios/mod.rs:162@other`).

## Outputs and side effects

| | EmuTOS with NVRAM chip | EmuTOS without chip | NewTOS |
|---|---|---|---|
| Return | `0`, `-5`, `-12` as the table | `0x2E` | `0x2E` |
| `buffer` | filled on a successful read | untouched | untouched (poison stays) |
| Hardware | NVRAM registers | none | none |
| Global state | none (the checksum is in the chip) | none | none |

## Measured (tosdiff, EmuTOS oracle against NewTOS)

Run `measurements/measurement-2026-10-28-xbios-system-audit/RUNINFO.TXT`
(cases `bios-r1` 189-194 and `xbios-system-r1` 68-76; `tt` and `falcontc`
alike). EmuTOS with the chip: reads in range `0`, bad range / bad `type` /
NULL buffer `-5` (`0xFFFB`); NewTOS: `0x002E` in every row and the buffer keeps
its poison bytes (`0xDE 0xAD …`)
(`measurements/measurement-2026-10-28-xbios-system-audit/runs/NEWTOS-bios-r1/tosdiff-newtos-vs-emutos.txt`,
case 189 at its line 57; `…/runs/NEWTOS-xbios-system-r1/tosdiff-newtos-vs-emutos.txt`,
cases 68-76). All 15 rows are ACCEPTED differences with ADR-012 and
`docs/COMPATIBILITY.md:1953` as evidence. **Not measured:** the answer of
EmuTOS without a chip (`0x2E`): read from `bios/nvram.c:161-162`, no profile
exists to measure it.

## Verification criteria (NewTOS)

1. `NVMaccess(any type, any start, any count, any buffer)` returns `0x002E`.
2. After a read call the buffer is byte-for-byte what the caller left.
3. No other XBIOS answer changes.

## Related

`docs/api/xbios/_dispatcher.md` (catch-all and range check, section 3),
`docs/api/xbios/_full-function-inventory.md` (the 0x2E row: `CONF_WITH_NVRAM`
1, function number), `docs/api/xbios/_triage.md:91` (the 0x2E row, already consistent with this file), cookie
`_AKP` (`docs/COMPATIBILITY.md:1660`: NewTOS has no NVRAM, `_AKP` low byte 0).

## Open points

- No STOP-block: the NewTOS behaviour and the EmuTOS arms are read from code;
  the chip-less answer is code-read only (see "Measured").
- Inferred: the m68k stack offsets of the four arguments (not read from a
  binding header); that the oracle profiles `tt` and `falcontc` both have
  `has_nvram` set (the XBIOS10 facts say the cases differ on both, which
  implies it).
- `docs/api/xbios/_full-function-inventory.md:357` still carries the German
  "type 2 = löschen (nicht gegen die Quelle geprüft)": type 2 is the *reset*
  (zero, plus the Falcon init block), and it runs before the argument check.
  Not edited here (German file, outside this scope; the lead may hand it to a
  spec-maintenance ticket). `docs/api/xbios/_triage.md` was already corrected
  (XBIOS13) and needs nothing.
