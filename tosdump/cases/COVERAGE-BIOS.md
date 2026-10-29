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
- **disk round** -- measured by `bios-r2.tsv` only, a `!disk` table on a
  test image (section "Disk round" at the end); refused in every other table.
- **pointer round** -- measured by `xbios-ptr-r1.tsv` only, under the rule
  that table states (section "Structures behind a pointer" at the end);
  refused in every other table.
- **GEMDOS media-change round** -- the GEMDOS calls of `gemdos-mc-*.tsv`
  (TOOL20), in a destructive `!disk` table only, under the rules of the
  section "GEMDOS after a media change" at the end; refused in every other
  table. Their cases go to `DUMPGDOS` once that exists.
- **excluded** -- never in an unattended table: writes media, resets or
  reboots a real TOS, installs vectors or interrupt handlers, or reprograms
  hardware the run depends on (the screen, AUX = NewTOS' serial console).

Safety is judged for **both** machines and for a real TOS the table might be
carried to, like the AES guard ("the union of what either machine would find
fatal", `docs/TOS-REFERENCE-DUMPER.md` section 6): `Ssbrk`, `Prtblk`,
`Puntaes` are empty slots on the pin and still excluded. The loader's
whitelist (`call_policy`, `tosdump/core/cases.c`) admits exactly the round-1
rows below -- and, in a `!disk` table only, the disk-round rows -- and
refuses everything else.

**Profile.** The reference is the oracle's TT profile, like every other
area; the same table was also recorded on the `falcontc` profile, because a
third of the XBIOS answers is the machine's: TT shifter calls answer only on
a TT, Videl, DSP and Falcon DMA-sound calls only on a Falcon, and `Getrez`,
the palette, PSG port A, `Rsconf`, `Bconmap` differ between the two
(`measurements/measurement-2026-10-27-dumpbios/RUNINFO.TXT`).

**One table per round**, as for the other areas: the case id is the
physical line number and the dump header carries the table's CRC
(`583C11E0` for `bios-r1.tsv`).


## BIOS, trap #13 -- `bios_vecs[]`, `bios/bios.c:1252-1265` (12 slots)

| # | Name | EmuTOS handler | Status | Cases / reason |
|---|---|---|---|---|
| 0x00 | Getmpb | `bios/biosmem.c:134` | pointer round | `xbios-ptr-r1.tsv` only (TOOL16, section "Structures behind a pointer" below): a `b:` buffer, inside a `Super` bracket, last -- it rewrites `themd`, the memory descriptor GEMDOS built its pool on (`biosmem.c:143-150`), and the MD lies in protected memory. 1 case, the MPB and the MD through `mp_free` (`0:mpb`). Every other table: refused |
| 0x01 | Bconstat | `bios/bios.c:914` | **round 1** | 9 cases: handles 0-6, 10, -1 (0..7 `bconstat_vec`, 6.. Bconmap, `bios/bios.c:923-933`) |
| 0x02 | Bconin | `bios/bios.c:958` | **round 1** | 4 cases: only handles that return at once on both machines -- 4, 5 (`char_dummy`, `chardev.c:34-36`), 10, -1. Handle 0 returns at once with 0 on EmuTOS (`bconin0`: "not implemented", `bios/parport.c:121-125`; probe B09, `docs/audits/2026-10-28-bios-core.md`); handles 1-3 and the Bconmap devices 6-9 wait for input: never [BIOS13] |
| 0x03 | Bconout | `bios/bios.c:991` | **round 1** | 4 cases: a printable byte to CON (2) and raw CON (5), handles 10 and -1. PRT (0) waits up to 30 s for a printer (`parport.c:150-168`), IKBD (4) and MIDI (3) are commands to a device, AUX (1) is NewTOS' serial console: later, with their own reasons |
| 0x04 | Rwabs | `bios/bios.c:1045` | disk round | `!disk` tables only, on a test image. `bios-r2.tsv`: 31 cases -- logical reads, the FastCopy NULL arm, write-back of sectors a prelude read; the closing tables: `RW_NOTRANSLATE` with the unit token `u:0` (`bios-r2-phys.tsv`, 12 cases) and writes of patched boot sectors (`bios-r2-x004.tsv`, `-x005.tsv`, `-x006.tsv`); see "Disk round" below. Round 1: refused |
| 0x05 | Setexc | `bios/bios.c:1070` | **round 1** | 5 cases, the inquiry `Setexc(n,-1)` (`bios/bios.c:1076-1078`), NONZERO: a vector address differs per machine. Setting a vector: excluded |
| 0x06 | Tickcal | `bios/bios.c:1096` | **round 1** | 1 case, EmuTOS answers 20 |
| 0x07 | Getbpb | `bios/bios.c:1120` | disk round | `!disk` tables only. `bios-r2.tsv`: 8 cases (C: -- three times, around the forcechange handshake --, absent and out-of-range drives; the BPB captured through the return); the closing tables `bios-r2-x004.tsv`, `-x005.tsv`, `-x006.tsv`: C: around patched boot sectors. A:/B: excluded (a floppy without a disk). Round 1: refused |
| 0x08 | Bcostat | `bios/bios.c:1143` | **round 1** | 9 cases: handles 0-7, -1 (IKBD/MIDI slots swapped, `chardev.c:38-42`) |
| 0x09 | Mediach | `bios/bios.c:1184` | disk round | `!disk` tables only. `bios-r2.tsv`: 11 cases (C: around the forcechange handshake, absent drives); the closing tables: C: around patched boot sectors. A:/B: excluded. Round 1: refused |
| 0x0A | Drvmap | `bios/bios.c:1206` | **round 1** | 1 case, NONZERO: the drive set is the machine's (`blkdev.c:833`). `bios-r2.tsv`: EXACT, both disk setups are C: + D: (`0xC`) |
| 0x0B | Kbshift | `bios/ikbd.c:145` | **round 1** | 1 case, the inquiry -1; setting the shift state: later, with a restoring prelude |
| 0x0C-0xFF | past `bios_ent` | `bios/vectors.S:1325-1326` | **round 1** | 3 cases (12, 127, 255): the dispatcher leaves the number in d0 |

## XBIOS, trap #14 -- `xbios_vecs[]`, `bios/xbios.c:1285-1515` (142 slots, `LAST_ENTRY` 0x8D)

| # | Name | EmuTOS handler | Status | Cases / reason |
|---|---|---|---|---|
| 0x00 | Initmous | `bios/mouse.c:40` | excluded | installs a mouse packet handler |
| 0x01 | Ssbrk | `xbios_unimpl, bios/xbios.c:1287` | excluded | empty on the pin, but a real TOS reserves memory at the top of RAM; the table may be carried to one |
| 0x02 | Physbase | `bios/screen.c:1042` | **round 1** | 1 case, PTR_RAM |
| 0x03 | Logbase | `bios/screen.c:1071` | **round 1** | 1 case, PTR_RAM |
| 0x04 | Getrez | `bios/screen.c:1076` | **round 1** | 1 case, EXACT.W (`WORD getrez`); profile dependent (TT-Medium 4, Falcon 2) |
| 0x05 | Setscreen | `bios/screen.c:1102` | **round 1** | 1 case, `Setscreen(-1,-1,-1)` returns 0 before anything is read (`screen.c:1115-1117`). Any other value moves or reprograms the screen the run draws on: excluded |
| 0x06 | Setpalette | `bios/screen.c:1168` | later | reprograms the palette at the next VBL; needs a prelude that restores it |
| 0x07 | Setcolor | `bios/screen.c:1192` | **round 1** | 3 cases, the inquiry `Setcolor(n,-1)`; setting: later (restoring prelude) |
| 0x08 | Floprd | `bios/floppy.c:742` | later | reads a floppy; the answer is the drive's state on the oracle, an environment decision |
| 0x09 | Flopwr | `bios/floppy.c:749` | excluded | writes media |
| 0x0A | Flopfmt | `bios/floppy.c:898` | excluded | formats media; nine arguments, one more than the record and the NewTOS ABI carry |
| 0x0B | (used by BIOS) | `xbios_unimpl, bios/xbios.c:1297` | later | empty on the pin; what a real TOS has there is not established |
| 0x0C | Midiws | `bios/midi.c:104` | later | sends bytes to the MIDI port |
| 0x0D | Mfpint | `bios/mfp.c:114` | excluded | installs an interrupt vector |
| 0x0E | Iorec | `bios/xbios.c:336` | **round 1** | 5 cases: devices 0-2 PTR_RAM, 3 and -1 EXACT (-1). The records behind the pointer: `xbios-ptr-r1.tsv` (TOOL16), section "Structures behind a pointer" below |
| 0x0F | Rsconf | `bios/xbios.c:376, bios/serport.c:217` | **round 1** | 2 cases, all -1 and -2 (baud), `serport.c:220-226`; a real setting reprograms AUX, NewTOS' serial console: excluded |
| 0x10 | Keytbl | `bios/ikbd.c:122` | **round 1** | 1 case, `Keytbl(-1,-1,-1)` PTR_RAM; installing tables: later |
| 0x11 | Random | `bios/xbios.c:421` | **round 1** | 1 case, IGNORE |
| 0x12 | Protobt | `bios/floppy.c:690` | **round 1** | 5 cases: a boot sector built in buffer 0, captured as bytes (serial + BPB, checksum at 510) |
| 0x13 | Flopver | `bios/floppy.c:797` | later | same as Floprd |
| 0x14 | Scrdmp | `bios/xbios.c:490` | later | runs `dump_vec`; a real TOS prints the screen |
| 0x15 | Cursconf | `bios/vt52.c:893` | **round 1** | 1 case, function 5 (blink-rate inquiry); 0-4 change the cursor: later |
| 0x16 | Settime | `bios/clock.c:1218` | excluded | sets the machine clock |
| 0x17 | Gettime | `bios/clock.c:1266` | **round 1** | 1 case, NONZERO: the date is the machine's |
| 0x18 | Bioskeys | `bios/ikbd.c:136` | later | restores the key tables; measurable only after a Keytbl set |
| 0x19 | Ikbdws | `bios/ikbd.c:948` | excluded | commands the keyboard processor |
| 0x1A | Jdisint | `bios/mfp.c:122` | excluded | switches an MFP interrupt |
| 0x1B | Jenabint | `bios/mfp.c:127` | excluded | switches an MFP interrupt |
| 0x1C | Giaccess | `bios/sound.c:94` | **round 1** | 2 cases, register reads 7 and 14 (no write bit 0x80); a write: excluded |
| 0x1D | Offgibit | `bios/sound.c:130` | excluded | writes PSG port A (drive select, strobe) |
| 0x1E | Ongibit | `bios/sound.c:115` | excluded | writes PSG port A |
| 0x1F | Xbtimer | `bios/mfp.c:163` | excluded | programs an MFP timer and its vector |
| 0x20 | Dosound | `bios/sound.c:145` | **round 1** | 1 case, the inquiry -1 (`sound.c:150`); playing: later |
| 0x21 | Setprt | `bios/parport.c:49` | **round 1** | 1 case, -1; setting: later |
| 0x22 | Kbdvbase | `bios/xbios.c:716` | **round 1** | 1 case, PTR_RAM |
| 0x23 | Kbrate | `bios/ikbd.c:439` | **round 1** | 1 case, (-1,-1); setting: later |
| 0x24 | Prtblk | `xbios_unimpl, bios/xbios.c:1339` | excluded | empty on the pin; a real TOS prints a block |
| 0x25 | Vsync | `bios/screen.c:1205` | **round 1** | 1 case, IGNORE (void) |
| 0x26 | Supexec | `bios/vectors.S:1376` | super-r1 | only with a dumper-owned callback `f:<n>` (MEM12, 2026-10-27): `tosdump/cases/super-r1.tsv`, see the section at the end |
| 0x27 | Puntaes | `xbios_unimpl, bios/xbios.c:1342` | excluded | empty on the pin; a real TOS discards the AES and reboots |
| 0x28 | (empty) | `xbios_unimpl, bios/xbios.c:1251-1257, vectors.S:1366` | **round 1** | 1 case: answers its own number |
| 0x29 | Floprate | `bios/floppy.c:1039` | **round 1** | 3 cases: rate -1 and -4 (the inquiries) on drive 0, -1 on drive 2; setting: later |
| 0x2A | DMAread | `bios/disk.c:1241` | later | reads an ACSI/SCSI unit: environment |
| 0x2B | DMAwrite | `bios/disk.c:1254` | excluded | writes media |
| 0x2C | Bconmap | `bios/serport.c:1082` | **round 1** | 4 cases: -1, -2 (PTR_RAM), 0 and 10 (map nothing); remapping AUX: later |
| 0x2D | (empty) | `xbios_unimpl, bios/xbios.c:1251-1257, vectors.S:1366` | **round 1** | 1 case: answers its own number |
| 0x2E | NVMaccess | `bios/nvram.c:155` | **round 1** | 6 cases: a 48-byte read (`NVRAM_USER_SIZE`, `nvram.c:30`) captured, five error arms; write and reset: excluded |
| 0x2F | (empty) | `xbios_unimpl, bios/xbios.c:1251-1257, vectors.S:1366` | **round 1** | 1 case: answers its own number |
| 0x30 | (empty) | `xbios_unimpl, bios/xbios.c:1251-1257, vectors.S:1366` | **round 1** | 1 case: answers its own number |
| 0x31 | (empty) | `xbios_unimpl, bios/xbios.c:1251-1257, vectors.S:1366` | **round 1** | 1 case: answers its own number |
| 0x32 | (empty) | `xbios_unimpl, bios/xbios.c:1251-1257, vectors.S:1366` | **round 1** | 1 case: answers its own number |
| 0x33 | (empty) | `xbios_unimpl, bios/xbios.c:1251-1257, vectors.S:1366` | **round 1** | 1 case: answers its own number |
| 0x34 | (empty) | `xbios_unimpl, bios/xbios.c:1251-1257, vectors.S:1366` | **round 1** | 1 case: answers its own number |
| 0x35 | (empty) | `xbios_unimpl, bios/xbios.c:1251-1257, vectors.S:1366` | **round 1** | 1 case: answers its own number |
| 0x36 | (empty) | `xbios_unimpl, bios/xbios.c:1251-1257, vectors.S:1366` | **round 1** | 1 case: answers its own number |
| 0x37 | (empty) | `xbios_unimpl, bios/xbios.c:1251-1257, vectors.S:1366` | **round 1** | 1 case: answers its own number |
| 0x38 | (empty) | `xbios_unimpl, bios/xbios.c:1251-1257, vectors.S:1366` | **round 1** | 1 case: answers its own number |
| 0x39 | (empty) | `xbios_unimpl, bios/xbios.c:1251-1257, vectors.S:1366` | **round 1** | 1 case: answers its own number |
| 0x3A | (empty) | `xbios_unimpl, bios/xbios.c:1251-1257, vectors.S:1366` | **round 1** | 1 case: answers its own number |
| 0x3B | (empty) | `xbios_unimpl, bios/xbios.c:1251-1257, vectors.S:1366` | **round 1** | 1 case: answers its own number |
| 0x3C | (empty) | `xbios_unimpl, bios/xbios.c:1251-1257, vectors.S:1366` | **round 1** | 1 case: answers its own number |
| 0x3D | (empty) | `xbios_unimpl, bios/xbios.c:1251-1257, vectors.S:1366` | **round 1** | 1 case: answers its own number |
| 0x3E | (empty) | `xbios_unimpl, bios/xbios.c:1251-1257, vectors.S:1366` | **round 1** | 1 case: answers its own number |
| 0x3F | (empty) | `xbios_unimpl, bios/xbios.c:1251-1257, vectors.S:1366` | **round 1** | 1 case: answers its own number |
| 0x40 | Blitmode | `bios/xbios.c:886` | **round 1** | 1 case, -1; setting: later |
| 0x41 | (empty) | `xbios_unimpl, bios/xbios.c:1251-1257, vectors.S:1366` | **round 1** | 1 case: answers its own number |
| 0x42 | (empty) | `xbios_unimpl, bios/xbios.c:1251-1257, vectors.S:1366` | **round 1** | 1 case: answers its own number |
| 0x43 | (empty) | `xbios_unimpl, bios/xbios.c:1251-1257, vectors.S:1366` | **round 1** | 1 case: answers its own number |
| 0x44 | (empty) | `xbios_unimpl, bios/xbios.c:1251-1257, vectors.S:1366` | **round 1** | 1 case: answers its own number |
| 0x45 | (empty) | `xbios_unimpl, bios/xbios.c:1251-1257, vectors.S:1366` | **round 1** | 1 case: answers its own number |
| 0x46 | (empty) | `xbios_unimpl, bios/xbios.c:1251-1257, vectors.S:1366` | **round 1** | 1 case: answers its own number |
| 0x47 | (empty) | `xbios_unimpl, bios/xbios.c:1251-1257, vectors.S:1366` | **round 1** | 1 case: answers its own number |
| 0x48 | (empty) | `xbios_unimpl, bios/xbios.c:1251-1257, vectors.S:1366` | **round 1** | 1 case: answers its own number |
| 0x49 | (empty) | `xbios_unimpl, bios/xbios.c:1251-1257, vectors.S:1366` | **round 1** | 1 case: answers its own number |
| 0x4A | (empty) | `xbios_unimpl, bios/xbios.c:1251-1257, vectors.S:1366` | **round 1** | 1 case: answers its own number |
| 0x4B | (empty) | `xbios_unimpl, bios/xbios.c:1251-1257, vectors.S:1366` | **round 1** | 1 case: answers its own number |
| 0x4C | (empty) | `xbios_unimpl, bios/xbios.c:1251-1257, vectors.S:1366` | **round 1** | 1 case: answers its own number |
| 0x4D | (empty) | `xbios_unimpl, bios/xbios.c:1251-1257, vectors.S:1366` | **round 1** | 1 case: answers its own number |
| 0x4E | (empty) | `xbios_unimpl, bios/xbios.c:1251-1257, vectors.S:1366` | **round 1** | 1 case: answers its own number |
| 0x4F | (empty) | `xbios_unimpl, bios/xbios.c:1251-1257, vectors.S:1366` | **round 1** | 1 case: answers its own number |
| 0x50 | EsetShift | `bios/screen.c:180` | excluded | changes the resolution the run draws in |
| 0x51 | EgetShift | `bios/screen.c:210` | **round 1** | 1 case; TT only (profile) |
| 0x52 | EsetBank | `bios/screen.c:222` | **round 1** | 1 case, -1; TT only |
| 0x53 | EsetColor | `bios/screen.c:241` | **round 1** | 3 cases, (n,-1); TT only |
| 0x54 | EsetPalette | `bios/screen.c:265` | later | reprograms the palette |
| 0x55 | EgetPalette | `bios/screen.c:290` | **round 1** | 2 cases, 16 WORDs captured; TT only |
| 0x56 | EsetGray | `bios/screen.c:313` | **round 1** | 1 case, -1; TT only |
| 0x57 | EsetSmear | `bios/screen.c:334` | **round 1** | 1 case, -1; TT only |
| 0x58 | VsetMode | `bios/videl.c:619` | **round 1** | 1 case, -1; Falcon only. A mode: excluded (resolution) |
| 0x59 | VgetMonitor | `bios/videl.c:664` | **round 1** | 1 case; Falcon only |
| 0x5A | VsetSync | `bios/videl.c:675` | excluded | changes the video sync |
| 0x5B | VgetSize | `bios/videl.c:724` | **round 1** | 1 case, mode 0x1C; Falcon only |
| 0x5C | (empty) | `xbios_unimpl, bios/xbios.c:1251-1257, vectors.S:1366` | **round 1** | 1 case: answers its own number |
| 0x5D | VsetRGB | `bios/videl.c:806` | later | reprograms the palette |
| 0x5E | VgetRGB | `bios/videl.c:862` | **round 1** | 1 case, 16 LONGs captured; Falcon only |
| 0x5F | Vfixmode | `bios/videl.c:917` | **round 1** | 1 case, mode 0x1C; Falcon only |
| 0x60 | Dsp_DoBlock | `bios/dsp.c:222` | later | transfers data to or runs code on the DSP; needs a DSP program, and a handshake can wait forever on a Falcon without one |
| 0x61 | Dsp_BlkHandShake | `bios/dsp.c:259` | later | transfers data to or runs code on the DSP; needs a DSP program, and a handshake can wait forever on a Falcon without one |
| 0x62 | Dsp_BlkUnpacked | `bios/dsp.c:296` | later | transfers data to or runs code on the DSP; needs a DSP program, and a handshake can wait forever on a Falcon without one |
| 0x63 | Dsp_InStream | `bios/dsp.c:326` | later | transfers data to or runs code on the DSP; needs a DSP program, and a handshake can wait forever on a Falcon without one |
| 0x64 | Dsp_OutStream | `bios/dsp.c:350` | later | transfers data to or runs code on the DSP; needs a DSP program, and a handshake can wait forever on a Falcon without one |
| 0x65 | Dsp_IOStream | `bios/dsp.c:425` | later | transfers data to or runs code on the DSP; needs a DSP program, and a handshake can wait forever on a Falcon without one |
| 0x66 | Dsp_RemoveInterrupts | `bios/dsp.c:494` | later | changes DSP bookkeeping (lock, reservation, abilities, interrupts); needs a paired restoring call |
| 0x67 | Dsp_GetWordSize | `bios/dsp.c:508` | **round 1** | 1 case; Falcon only (TT: the function number) |
| 0x68 | Dsp_Lock | `bios/dsp.c:599` | later | changes DSP bookkeeping (lock, reservation, abilities, interrupts); needs a paired restoring call |
| 0x69 | Dsp_Unlock | `bios/dsp.c:611` | later | changes DSP bookkeeping (lock, reservation, abilities, interrupts); needs a paired restoring call |
| 0x6A | Dsp_Available | `bios/dsp.c:619` | **round 1** | 2 cases (x, y captured as LONGs); writes even without a DSP; Falcon only (TT: the function number) |
| 0x6B | Dsp_Reserve | `bios/dsp.c:628` | later | changes DSP bookkeeping (lock, reservation, abilities, interrupts); needs a paired restoring call |
| 0x6C | Dsp_LoadProg | `bios/dsp.c:650` | later | transfers data to or runs code on the DSP; needs a DSP program, and a handshake can wait forever on a Falcon without one |
| 0x6D | Dsp_ExecProg | `bios/dsp.c:670` | later | transfers data to or runs code on the DSP; needs a DSP program, and a handshake can wait forever on a Falcon without one |
| 0x6E | Dsp_ExecBoot | `bios/dsp.c:693` | later | transfers data to or runs code on the DSP; needs a DSP program, and a handshake can wait forever on a Falcon without one |
| 0x6F | Dsp_LodToBinary | `bios/dsp.c:922` | later | transfers data to or runs code on the DSP; needs a DSP program, and a handshake can wait forever on a Falcon without one |
| 0x70 | Dsp_TriggerHC | `bios/dsp.c:954` | later | transfers data to or runs code on the DSP; needs a DSP program, and a handshake can wait forever on a Falcon without one |
| 0x71 | Dsp_RequestUniqueAbility | `bios/dsp.c:965` | later | changes DSP bookkeeping (lock, reservation, abilities, interrupts); needs a paired restoring call |
| 0x72 | Dsp_GetProgAbility | `bios/dsp.c:976` | **round 1** | 1 case; Falcon only (TT: the function number) |
| 0x73 | Dsp_FlushSubroutines | `bios/dsp.c:987` | later | changes DSP bookkeeping (lock, reservation, abilities, interrupts); needs a paired restoring call |
| 0x74 | Dsp_LoadSubroutine | `bios/dsp.c:1083` | later | transfers data to or runs code on the DSP; needs a DSP program, and a handshake can wait forever on a Falcon without one |
| 0x75 | Dsp_InqSubrAbility | `bios/dsp.c:1146` | **round 1** | 1 case, ability 0; Falcon only (TT: the function number) |
| 0x76 | Dsp_RunSubroutine | `bios/dsp.c:1166` | later | transfers data to or runs code on the DSP; needs a DSP program, and a handshake can wait forever on a Falcon without one |
| 0x77 | Dsp_Hf0 | `bios/dsp.c:1200` | **round 1** | 1 case, -1 (read); Falcon only (TT: the function number) |
| 0x78 | Dsp_Hf1 | `bios/dsp.c:1222` | **round 1** | 1 case, -1 (read); Falcon only (TT: the function number) |
| 0x79 | Dsp_Hf2 | `bios/dsp.c:1247` | **round 1** | 1 case; Falcon only (TT: the function number) |
| 0x7A | Dsp_Hf3 | `bios/dsp.c:1257` | **round 1** | 1 case; Falcon only (TT: the function number) |
| 0x7B | Dsp_BlkWords | `bios/dsp.c:522` | later | transfers data to or runs code on the DSP; needs a DSP program, and a handshake can wait forever on a Falcon without one |
| 0x7C | Dsp_BlkBytes | `bios/dsp.c:558` | later | transfers data to or runs code on the DSP; needs a DSP program, and a handshake can wait forever on a Falcon without one |
| 0x7D | Dsp_HStat | `bios/dsp.c:1273` | **round 1** | 1 case, EXACT.B (`UBYTE`); Falcon only (TT: the function number) |
| 0x7E | Dsp_SetVectors | `bios/dsp.c:1286` | excluded | installs interrupt handlers |
| 0x7F | Dsp_MultBlocks | `bios/dsp.c:1338` | later | transfers data to or runs code on the DSP; needs a DSP program, and a handshake can wait forever on a Falcon without one |
| 0x80 | Locksnd | `bios/dmasound.c:215` | later | locks the sound system; needs the paired Unlocksnd |
| 0x81 | Unlocksnd | `bios/dmasound.c:230` | later | pairs with Locksnd |
| 0x82 | Soundcmd | `bios/dmasound.c:342` | **round 1** | 3 cases, modes 0, 1, 6 with -1 (the inquiries); setting: later |
| 0x83 | Setbuffer | `bios/dmasound.c:369` | later | sets DMA frame pointers |
| 0x84 | Setsndmode | `bios/dmasound.c:401` | later | setter |
| 0x85 | Settracks | `bios/dmasound.c:424` | later | setter |
| 0x86 | Setmontracks | `bios/dmasound.c:441` | later | setter |
| 0x87 | Setinterrupt | `bios/dmasound.c:457` | later | arms an end-of-frame interrupt |
| 0x88 | Buffoper | `bios/dmasound.c:499` | **round 1** | 1 case, -1 (inquiry); starting playback: later |
| 0x89 | Dsptristate | `bios/dmasound.c:521` | later | setter |
| 0x8A | Gpio | `bios/dmasound.c:545` | later | drives the DSP connector GPIO |
| 0x8B | Devconnect | `bios/dmasound.c:721` | later | rewires the sound matrix |
| 0x8C | Sndstatus | `bios/dmasound.c:751` | **round 1** | 1 case, 0 (no reset) |
| 0x8D | Buffptr | `bios/dmasound.c:772` | **round 1** | 1 case, 16 bytes captured as LONGs |
| 0x8E-0xFF | past `xbios_ent` | `bios/vectors.S:1325-1326` | **round 1** | 1 case (0x8E). Not 0x96: TOS 4 has `VsetMask` there |

## Summary

| Table | Slots | round 1 | disk round | pointer round | later | excluded |
|---|---|---|---|---|---|---|
| BIOS | 12 | 8 | 3 | 1 | 0 | 0 |
| XBIOS | 142 | 81 | 0 | 0 | 43 | 18 |

Plus the probes past each table (3 BIOS, 1 XBIOS). `bios-r1.tsv` holds 147 cases; every round-1 row above names how many are its own. 35 of the 81 XBIOS round-1 slots are empty (`xbios_unimpl`) and answer their own number.

## Super and Supexec -- `super-r1.tsv` (MEM12, 2026-10-27)

A second table for the processor-state calls behind the NewTOS super state
(`docs/api/_super-state.md` S3-S6, `TASK/MEM12.md`). Separate from
`bios-r1.tsv`, so that table, its CRC and its references stay as they are.
Form and guard: `docs/TOS-REFERENCE-DUMPER.md` section 6, "Super and Supexec
in a BIOS-area table".

| trap:fn | call | admitted as | why safe |
|---|---|---|---|
| `1:0x20` | `Super` (GEMDOS; the only GEMDOS call a BIOS table may issue) | `l:1` inquiry; `l:0` ... `t:0` bracket, closed in the same table | the inquiry changes nothing (`x20_inq`, `bdos/rwa.S:340-346`); the bracket returns to user mode on the stack it left (`x20_usr`/`x20_sup`, `:280-336`); an explicit stack address is refused |
| `14:0x26` | `Supexec` | `f:0`, `f:1`, `f:2` -- the dumper's own callbacks (`tosdump/core/runner_call.c`) | the code run is the dumper's: two return constants, one asks `Super(1L)`, which the TRAP #1 front end answers before the BDOS (`bios/xbios.c:767-773` forbids BDOS calls from a callback) |

References: `tosdump/reference/EMUTOS-1a7484ab-SUPER` (TT) and
`-SUPER-falcontc`, `measurements/measurement-2026-10-27-mem12-super-oracle/RUNINFO.TXT`.

## Disk round -- `bios-r2.tsv` (TOOL07, 2026-10-28)

`Rwabs`, `Getbpb`, `Mediach` and an exact `Drvmap`, on a BIOS block drive
that is a test image: the oracle's profile `disk` (`docs/analysis/ORACLE.md`
§2) and, on NewTOS, a card from `tools/mkcard.sh` with `MKCARD_TESTDISK` --
the same image as C:, the dumper on D:. Separate from `bios-r1.tsv`, whose
table, CRC and references stay as they are. Form and guard:
`docs/TOS-REFERENCE-DUMPER.md` section 6, "The disk round" (`!disk`,
Rwabs-read preludes, write-back only, `r:` capture). The cases come from the
audit's probe (`docs/audits/2026-10-28-bios-storage.md` section 5;
`measurements/measurement-2026-10-28-bios-storage-audit/method/bstprobe.c`,
same ids in the comments).

| Group | Cases | What | Finding |
|---|---|---|---|
| `Drvmap` | 1 | EXACT, `0x0000000C` on both setups | -- |
| `Getbpb` | 6 | C: (PTR_RAM, the BPB as nine WORDs via `r:18.W`), F:, P:, 16, -1, 0x7FFF | FS12 (16) |
| `Mediach` | 5 | C:, F:, P:, 16, -1 | FS12 (16) |
| `Rwabs` reads R01-R20 | 21 | sector 0 (two halves), two sectors, `recno`/`lrecno`, the last sector, past the end, negative, count 0, count 0 past the end, absent drives, the flags 2/4/0x80, `recno` 0x8000 and -2 | BIOS-STOR-003 (R10) |
| FastCopy F03 | 2 | `Rwabs(0,NULL,2,0,A:)` and back to 0: EmuTOS sets A:'s media-change state and returns 0 before any I/O (`bios/blkdev.c:425-428`) | BIOS-STOR-007 |
| forcechange W01-W11, X12-X16 | 16 | write-back of sector 0 (without and with `RW_NOMEDIACH`) and of the last sector, with `Mediach`, reads and `Getbpb` around it (`bios/blkdev.c:450-453`, `:473-477`, `:589-590`, `:799-803`) | BIOS-STOR-001 |

51 cases, 3 preludes, CRC `09C71B80`. Why each call is safe:

| Call | Admitted as | Why safe |
|---|---|---|
| `Getbpb`, `Mediach` | any drive but A:/B: | they read a boot sector or ask the device; a floppy without a disk ends in the critical-error handler on a real TOS (`bios/blkdev.c:623-629`) |
| `Rwabs` read | flags 0/2/4/0x80, a dumper buffer, count 0..2, not A:/B: | it reads at most two sectors into a 1024-byte buffer |
| `Rwabs` NULL arm | read, A:/B:, count 0..2 | EmuTOS returns before any I/O; NewTOS has no floppy (EUNDEV) |
| `Rwabs` write | write-back only: the last prelude read the same sectors into the same buffer; test drive only | the disk gets the bytes it already holds, or nothing (a failed prelude stops the run) |

Not in this round (later), with the reason:

- `RW_NOTRANSLATE` (physical unit; BIOS-STOR-002): the unit number is the
  machine's -- EmuTOS' IDE master is unit 18 (`bios/disk.h:30`), the Pi's
  card has none fixed yet (a spec decision).
- Patched boot sectors (BIOS-STOR-004 bps 1024, -005 tot16 = tot32 = 0,
  -006 logical `Rwabs` after a failed `Getbpb`): a write that changes the
  image needs a patch step the dumper does not have, and after a failed
  `Getbpb` EmuTOS refuses every logical access to the drive
  (`bios/blkdev.c:443-444`, `:601`), so the restore would have to go through
  the physical path -- the open unit number again.
- D: (drive 3): on the oracle Hatari's GEMDOS drive, on NewTOS the program
  partition -- a setup difference, not behaviour.

Reference: `tosdump/reference/EMUTOS-1a7484ab-BIOS-r2-disk` (profile
`disk`), `measurements/measurement-2026-10-28-tool07/RUNINFO.TXT`. The two
"later" items above were built as the closing tables below (TOOL07,
2026-10-28); `bios-r2.tsv` itself is frozen.

### The closing tables (TOOL07, 2026-10-28)

User decision 2026-10-28 (variant B): one table per sequence that changes the
image, which ends with it changed; the form (`!disk <drive> destructive`,
`+set`, the unit token `u:0`) and the runner rule (a destructive table only
on a fresh clone of the test image) are `docs/TOS-REFERENCE-DUMPER.md`
section 6, "The closing tables". Cases from the probe's groups P and X
(`bstprobe.c`, same ids in the comments).

| Table | Cases | What | Finding | End state |
|---|---|---|---|---|
| `bios-r2-phys.tsv` (not destructive) | 12 | `Rwabs(RW_NOTRANSLATE, …, u:0, …)`: MBR entry 1 and signature, C:'s boot sector at LBA 63 (two halves) and the same sector logically, two sectors, flags 2 and 4, `recno` 0x8000, count 0, negative `lrecno` and far past the device (ESECNF) | BIOS-STOR-002 | image unchanged |
| `bios-r2-x004.tsv` | 10 | boot sector bps 1024 written logically (`+set 0x0C=0x04`, `RW_NOMEDIACH`), `Mediach`, `Getbpb` (EmuTOS 0), reads after it (EmuTOS ESECNF) | BIOS-STOR-004 (and -001) | boot sector bps 1024 |
| `bios-r2-x005.tsv` | 13 | tot16 = tot32 = 0, `Getbpb` (EmuTOS numcl 0, no `B_16`), reads; then tot32 restored logically and `Getbpb` again | BIOS-STOR-005 (and -001) | image restored |
| `bios-r2-x006.tsv` | 11 | spf 0, `Getbpb` (EmuTOS 0, recsiz marked 0), logical reads and count 0 after it (EmuTOS ESECNF) | BIOS-STOR-006 (and -001) | boot sector spf 0 |
| `bios-r2-x001.tsv` (BIOS01) | 8 | a CHANGED sector 0 (`+set 3=0x58`, OEM name) written without `RW_NOMEDIACH` (EmuTOS E_CHNG, nothing written), `Mediach`, a read while armed, `Getbpb`, sector 0 read back | BIOS-STOR-001 | image unchanged |

Why each new form is safe: `+set` changes only a dumper buffer, and only
before the write-back of the read it follows, on the test drive; the image
it changes is a clone made for the run (both runners). The physical read
(`u:0`) reads at most two sectors into a 1024-byte buffer from the unit the
run's profile names (`D:\DISKUNIT.TXT`, oracle 18, NewTOS 26); a literal unit
number, a physical write and `RW_NOBYTESWAP` stay refused. Left out: the last
device sector (probe P03/P04) -- the device sizes differ between the image
(131072 sectors) and the card (262144), a setup difference.

References `tosdump/reference/EMUTOS-1a7484ab-BIOS-r2-<phys|x004|x005|x006>-disk`,
`measurements/measurement-2026-10-28-tool07b/RUNINFO.TXT`.
`bios-r2-x001.tsv`: reference `EMUTOS-1a7484ab-BIOS-r2-x001-disk`,
`measurements/measurement-2026-10-28-bios01-s2/RUNINFO.TXT` (TASK/BIOS01).

## Audit tables -- `bios-core-r1.tsv`, `xbios-chardev-r1.tsv`, `xbios-system-r1.tsv` (2026-10-28) [BIOS13]

Three further round-1 tables, each separate from `bios-r1.tsv` (whose table,
CRC `583C11E0` and references stay as they are). They hold only calls the
round-1 guard admits; every row is a query or a call whose effect does not
outlive it. References: `tosdump/reference/EMUTOS-1a7484ab-<BIOS-core|XBIOS-chardev|XBIOS-system>[-falcontc].{BIN,LOG}`.

| Table | Cases | CRC | Audit | What |
|---|---|---|---|---|
| `bios-core-r1.tsv` | 44 | `C056A3E0` | `docs/audits/2026-10-28-bios-core.md` section 5 | `Bconstat` 7, 8, 9, 11, 0x7FFF, -2, -32768; `Bconin` 11, 0x7FFF, -2, -32768; `Bconout` CON/raw-CON at the printable limits and non-devices with ESC/0xFF/`'A'`; `Setexc` inquiries (nine CPU vectors, `etv_*` with PTR_RAM, `$40C`, `$420`, `$5A8` EXACT, `$5A0`, `$5A4` PTR_RAM); `Bcostat` 8, 9, 10, 11, 0x7FFF, -2, -32768 |
| `xbios-chardev-r1.tsv` | 23 | `5C3A5F7A` | `docs/audits/2026-10-28-xbios-chardev.md` section 5 | XBIOS character-device queries: `Iorec` (0x0E, 7 rows), `Rsconf` (0x0F), `Kbrate` (0x23), `Bconmap` (0x2C, 11 rows) and the neighbouring slots 0x10, 0x22; build `b81d` |
| `xbios-system-r1.tsv` | 43 | `3944A677` | `docs/audits/2026-10-28-xbios-system.md` section 5 | `Protobt` 31 (disktype 0-6 and out-of-range, serial numbers, random arm, `exec`), `Cursconf(5,x)` 3, `NVMaccess` 9 |

Rows the guard still refuses and the audits propose (the setting arms
`Kbshift`/`Setexc`, `Bconout` to devices, `Midiws`, `Bioskeys`,
`Ikbdws`): status **later** -- they need a restoring prelude. Cite for the
guard: audits BIOS-CORE section 11.4, XBIOS-CHARDEV section 11.3. `Getmpb`
and the capture "through the return pointer" came with TOOL16 (next
section).

## Structures behind a pointer -- `xbios-ptr-r1.tsv` (TOOL16, 2026-10-29)

The structures behind `Iorec`'s pointer and in `Getmpb`'s MPB, field by
field, with the layouts of `docs/api/structures/iorec.md` and
`docs/api/structures/md.md` (copied lines, `layoutcheck`). Form and rules:
`docs/TOS-REFERENCE-DUMPER.md` section 6, "Structures behind a pointer"
(`!layout`, `r:<layout>` / `<n>:<layout>`, the follow rule, `!write return`,
`+put`). Separate from every earlier table, whose CRCs and references stay
as they are. 9 cases, 1 prelude, CRC `87DF757E`.

| Line | Call | Capture / rule | What |
|---|---|---|---|
| 24 | `Iorec(0)` | `r:extiorec` | RS-232 `EXT_IOREC`: `in`, `out`, baud, flow, UCR, data mask (`wr5` IGNORE) |
| 25, 26 | `Iorec(1)`, `Iorec(2)` | `r:iorec` | the IKBD and MIDI `IOREC`: size, head, tail, water marks; `ibuf` PTR_RAM |
| 28, +29, 30, 31 | `Iorec(1)`, `+put ibufhd=ibuftl`, `Bconstat(2)`, `Iorec(1)` | `!write return` | the keyboard flush of XBIOS01 in user state, `Bconstat(2)` after it, the record read back |
| 33-35 | `Super(0L)`, `Getmpb(b:0)`, `Super(t:0)` | `0:mpb` | the MPB in buffer 0 and the MD behind `mp_free`, read in supervisor state |

Why each new form is safe:

| Form | Admitted as | Why safe |
|---|---|---|
| layout read | through a pointer only when it passes `PTR_RAM` and the structure ends at or below `0x80000000` (`td_layout_follow_refusal`) | the address rule of the comparator; nothing below `0x400`, nothing odd, nothing past 2 GiB is read |
| `+put` | `!write return` tables only; right after the `r:<layout>` case; one non-pointer field copied onto another of the same type, through a pointer the follow rule admits | it writes inside the structure the call returned, a value of the same structure: the flush `ibufhd=ibuftl` sets the ring's read index to its write index, which `Bconin(2)` itself does when it drains the ring (`bios/ikbd.c:269-273`); measured in user state on the oracle by XCIOFL (`measurements/measurement-2026-10-28-xbios-chardev-audit/runs/XCIOFL-EMUTOS-tt`) |
| `Getmpb` | a `b:` buffer; inside a `Super(0L)` ... `Super(t:0)` bracket; last (only `Getmpb` and the closing `Super(t:0)` after it) | as the audit's probe ran it (group G, last, MD read in super state; `measurements/measurement-2026-10-28-bios-core-audit/RUNINFO.TXT`): the machine wrote its log afterwards on both profiles. It rewrites `themd` (`bios/biosmem.c:143-150`), so nothing that allocates follows it in the table |

References: `tosdump/reference/EMUTOS-1a7484ab-XBIOS-ptr` (TT) and
`-XBIOS-ptr-falcontc`, two runs each byte-identical,
`measurements/measurement-2026-10-29-tool16/RUNINFO.TXT`.

## The flush with keys in the ring -- `xbios-flush-r1.tsv` (XBIOS01, 2026-10-29)

`xbios-ptr-r1.tsv` flushes an empty ring. This table has three keys (a, b, c)
in the IKBD ring before the flush: they are typed from outside while the table
waits in 200 `Vsync()` cases (`Bconin(2)` waits for input and stays refused) --
on the oracle through Hatari's `--cmd-fifo` (`hatari-event keydown/keyup`,
`--fast-forward off`), on NewTOS through the scratch IKBD injector
(`tools/inject-arm.py`, `launch:DUMPBIOS`), so it is **not** a plain
`tools/hatari-oracle.sh` / `cargo xtask tosdump` table: method and key
scripts in `measurements/measurement-2026-10-29-xbios01/method/`. 207 cases,
1 prelude, CRC `965AE625`.

| Line | Call | What |
|---|---|---|
| 23 | `Bconstat(2)` EXACT | 0 before the keys (on NewTOS the input pump that arms the injector) |
| 25-224 | `Vsync()` x 200, IGNORE | the wait (about 3 s of frames) |
| 225, 226 | `Bconstat(2)` IGNORE | input pumps: the NewTOS injector feeds the keys here |
| 228 | `Bconstat(2)` EXACT | -1: keys waiting -- the precondition |
| 229 | `Iorec(1)` `r:iorec` | head 0, tail 12: three 4-byte entries |
| +230, 231, 232 | `+put ibufhd=ibuftl`, `Bconstat(2)`, `Iorec(1)` | the flush, `Bconstat(2)` 0, head == tail == 12 |

References: `tosdump/reference/EMUTOS-1a7484ab-XBIOS-flush` (TT) and
`-XBIOS-flush-falcontc`, two runs each byte-identical.

## GEMDOS after a media change -- `gemdos-mc-*.tsv` (TOOL20, 2026-10-29)

What the permanent probes `tosdump/probes/gcprobe.c` (FS20) and
`tosdump/probes/pxprobe.c` (FS22) measure, and the further FS22 paths after a
media change as far as a user program reaches them (`TASK/FS22.md`,
"Nachtrag Lead 2026-10-29"), as case tables of `DUMPBIOS`: a logical write of
sector 0 arms forcechange, the next GEMDOS access meets it, osif logs the
drive in again and restarts the call (`bdos/bdosmain.c:449-487`;
`docs/api/gemdos/_state.md` section 4a). Form and rules:
`docs/TOS-REFERENCE-DUMPER.md` section 6, "GEMDOS after a media change". The
three tables share one test image (`tools/mktestdisk.sh` with
`C:\NEWDESK.INF`, `C:\PXPROBE.PRG`, `C:\PXPROBE.NAP`, `C:\DELME.TXT`,
`C:\SUB\INNER.TXT`; `measurements/measurement-2026-10-29-tool20/RUNINFO.TXT`).

| Table | Cases | CRC | What |
|---|---|---|---|
| `gemdos-mc-fs20.tsv` | 17 (3 preludes) | `55B17174` | gcprobe A01-A09, B01-B08: `Dfree` meets the change; a changed boot sector (512 -> 256 root entries) and the geometry `Dfree` and an `Fcreate`/`Fwrite`/`Fclose` use after it; end state: the changed boot sector and `C:\GCPROBE.DAT` |
| `gemdos-mc-fs22.tsv` | 13 (2 preludes) | `26353FB1` | pxprobe P01-P07, F01-F07: `Pexec(0)` of the machine's own child on C: after a change (42), the caller's DTA around it, `Fgetdta`; `Fsfirst`/`Fsnext` after a change; end state = the image |
| `gemdos-mc-paths.tsv` | 49 (6 preludes) | `39FDDD95` | the file selector's listing after a change (`Fsfirst(<dir>\*.*, FA_SUBDIR)` and `Fsnext` to the end, root and a folder, `aes/gemfslib.c:285-310`); `Dsetpath`, `Dgetpath` (EmuTOS checks no media change, `bdos/fsdir.c:1223-1224`), `Dsetdrv`/`Dgetdrv` and `Fdelete` after a change; end state: `C:\DELME.TXT` deleted |

What the guard admits, in a table that says `!disk <drive> destructive`
only -- each a refusal lifted from "of GEMDOS a BIOS table issues Super only"
(`call_policy`, `gdos_refusal`, `tosdump/core/cases.c`):

| Call | Admitted as | Why safe |
|---|---|---|
| `Dgetdrv` (0x19), `Fgetdta` (0x2F) | no argument | queries (`bdos/fsmain.c:394-397`, `:438-441`) |
| `Fsetdta` (0x1A) | a `b:` buffer, which is the DTA from then on; no other line may name it | it stores a pointer (`bdos/fsmain.c:403-406`); the DTA is a dumper buffer, and nothing else writes into the search state |
| `Fsfirst` (0x4E), `Fsnext` (0x4F) | `s:` a path on the test drive, any attribute; the DTA set | they read a directory of the test image and write 44 bytes into the DTA buffer |
| `Dfree` (0x36), `Dgetpath` (0x47) | a `b:` buffer (not the DTA), the test drive as `<drive>+1` | they write 16 bytes / at most `LEN_ZPATH` (114) into a 1024-byte buffer (`bdos/fsfat.c:337`, `bdos/fsdir.c:1229`) |
| `Dsetpath` (0x3B) | `s:` a path on the test drive | it moves C:'s current directory only, never the dumper's (D:) |
| `Fcreate` (0x3C), `Fwrite` (0x40), `Fclose` (0x3E) | `Fcreate(s:<path on the test drive>, 0)` opens a bracket, `Fclose(t:1)` closes it; `Fwrite(t:1, l:0..1024, b:<n>)` inside it; one file at a time, a table that ends with it open is refused | they create one file on the test image, a clone made for the run; the write writes a dumper buffer |
| `Fdelete` (0x41) | `s:` a path on the test drive, no file open | it deletes on the test image only (`bdos/fsopnclo.c:623`) |
| `Dsetdrv` (0x0E) | `w:<test drive>` opens a bracket that `Dsetdrv(t:2)` closes -- t:2 is what a `Dgetdrv` before it returned; a table that ends with it open is refused | the dumper writes its BIN and LOG to its current drive; the bracket gives it back before the end (`bdos/fsmain.c:420-426`) |
| `Pexec` (0x4B) | mode 0, `x:` a program path on the test drive (the run's profile adds `.PRG` / `.NAP`), a `c:` command tail, env `l:0` | it runs the probe build on the test image, which with the tail `child` returns 42 and does nothing else (`tosdump/probes/pxprobe.c`) |
| any of them | not inside a `Super` bracket, never as a prelude | the file calls run in user state, as a program makes them |

What stays probe-only, with the reason:

- pxprobe P07 "the DTA is unchanged across the `Pexec`, all 44 bytes": the 21
  reserved bytes are each system's own (`docs/api/structures/dta.md`), so a
  record carries only the public fields (`d_reserved` is `x21`, not
  captured); the identity `Fgetdta() == &dta` is an address, which a record
  never carries. The table measures the public fields after the `Pexec` and
  behind `Fgetdta`'s pointer.
- gcprobe B06's marker: the table writes buffer 0 as the poison left it
  (`DE AD`), not the text marker; where it lands is the end-state comparison.
- The input trace of the BIOS calls (`gcprobe-trace-check.py`,
  `pxprobe-trace-check.py`): `tools/biostrace-check.py` reads neither `+set`
  nor the string arguments, and GEMDOS calls are not traced.
- The accessory loader and the AUTO folder: they run at boot, before any
  program; no user program reaches them after a media change. The AES
  shell's own `Dsetpath`/`Dgetpath`/`Dsetdrv`/`Fdelete` (`dos_call`) are the
  kernel's, not a program's: the tables measure the same GEMDOS calls from a
  program.

References `tosdump/reference/EMUTOS-1a7484ab-BIOS-mc-<fs20|fs22|paths>-disk`,
two runs each byte-identical, `measurements/measurement-2026-10-29-tool20/RUNINFO.TXT`.
