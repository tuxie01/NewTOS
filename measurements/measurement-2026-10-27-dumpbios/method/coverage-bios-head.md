# BIOS + XBIOS case coverage ledger

What `bios-r1.tsv` covers, what is still to come, and what is excluded with
the reason -- the counterpart of `COVERAGE.md` (VDI) and `COVERAGE-AES.md`
(AES) for the program `DUMPBIOS` (`docs/TOS-REFERENCE-DUMPER.md` sections 6
and 7, task T7).

The rows are the complete EmuTOS dispatch tables at the pin `1a7484ab`
(`references/emutos`, `docs/analysis/ORACLE.md`): `bios_vecs[]`, 12 slots,
and `xbios_vecs[]`, 142 slots (`LAST_ENTRY = 0x8D` because
`CONF_WITH_DMASOUND` is on in `make 512`, `bios/xbios.c:1273-1283`). The
handler cited is the function the slot reaches (`DBG_BIOS`/`DBG_XBIOS` are
0, so `VEC(wrapper, direct)` is the direct one). Specs: `docs/api/bios/`,
`docs/api/xbios/` (`_full-function-inventory.md` in each).

**Statuses.**

- **round 1** -- measured by `bios-r1.tsv`. Only queries: the call reads,
  or its effect does not outlive it. A setter appears only in its inquiry
  arm (`Kbshift(-1)`, `Setcolor(n,-1)`, `Rsconf(-1,...)`); the reason
  column says which arm, and which arms stay out.
- **later** -- safe to measure once something exists that round 1 does not
  have: a prelude that restores what the call changes, a BIOS drive on the
  oracle, a dumper-owned function, a DSP program.
- **excluded** -- never in an unattended table: writes media, resets or
  reboots a real TOS, installs vectors or interrupt handlers, or reprograms
  hardware the run depends on (the screen, AUX = NewTOS' serial console).

Safety is judged for **both** machines and for a real TOS the table might be
carried to, like the AES guard ("the union of what either machine would find
fatal", `docs/TOS-REFERENCE-DUMPER.md` section 6): `Ssbrk`, `Prtblk`,
`Puntaes` are empty slots on the pin and still excluded. The loader's
whitelist (`call_policy`, `tosdump/core/cases.c`) admits exactly the round-1
rows below and refuses everything else.

**Profile.** The reference is the oracle's TT profile, like every other
area; the same table was also recorded on the `falcontc` profile, because a
third of the XBIOS answers is the machine's: TT shifter calls answer only on
a TT, Videl, DSP and Falcon DMA-sound calls only on a Falcon, and `Getrez`,
the palette, PSG port A, `Rsconf`, `Bconmap` differ between the two
(`measurements/measurement-2026-10-27-dumpbios/RUNINFO.TXT`).

**One table per round**, as for the other areas: the case id is the
physical line number and the dump header carries the table's CRC
(`583C11E0` for `bios-r1.tsv`).

