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


## BIOS, trap #13 -- `bios_vecs[]`, `bios/bios.c:1252-1265` (12 slots)

| # | Name | EmuTOS handler | Status | Cases / reason |
|---|---|---|---|---|
| 0x00 | Getmpb | `bios/biosmem.c:134` | excluded | rewrites `themd`, the memory descriptor GEMDOS built its pool on (`biosmem.c:143-150`); its capture would be three addresses |
| 0x01 | Bconstat | `bios/bios.c:914` | **round 1** | 9 cases: handles 0-6, 10, -1 (0..7 `bconstat_vec`, 6.. Bconmap, `bios/bios.c:923-933`) |
| 0x02 | Bconin | `bios/bios.c:958` | **round 1** | 4 cases: only handles that return at once on both machines -- 4, 5 (`char_dummy`, `chardev.c:34-36`), 10, -1. Handles 0-3 and the Bconmap devices 6-9 wait for input: never |
| 0x03 | Bconout | `bios/bios.c:991` | **round 1** | 4 cases: a printable byte to CON (2) and raw CON (5), handles 10 and -1. PRT (0) waits up to 30 s for a printer (`parport.c:150-168`), IKBD (4) and MIDI (3) are commands to a device, AUX (1) is NewTOS' serial console: later, with their own reasons |
| 0x04 | Rwabs | `bios/bios.c:1045` | later | write: never (media). Read: needs a BIOS drive on the oracle -- its C: is a Hatari GEMDOS drive, not a block device; a disk image is an oracle change (`docs/analysis/ORACLE.md` §7) |
| 0x05 | Setexc | `bios/bios.c:1070` | **round 1** | 5 cases, the inquiry `Setexc(n,-1)` (`bios/bios.c:1076-1078`), NONZERO: a vector address differs per machine. Setting a vector: excluded |
| 0x06 | Tickcal | `bios/bios.c:1096` | **round 1** | 1 case, EmuTOS answers 20 |
| 0x07 | Getbpb | `bios/bios.c:1120` | later | reads the boot sector of a BIOS drive: same oracle precondition as Rwabs; A:/B: would touch the floppy |
| 0x08 | Bcostat | `bios/bios.c:1143` | **round 1** | 9 cases: handles 0-7, -1 (IKBD/MIDI slots swapped, `chardev.c:38-42`) |
| 0x09 | Mediach | `bios/bios.c:1184` | later | same oracle precondition as Getbpb |
| 0x0A | Drvmap | `bios/bios.c:1206` | **round 1** | 1 case, NONZERO: the drive set is the machine's (`blkdev.c:833`) |
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
| 0x0E | Iorec | `bios/xbios.c:336` | **round 1** | 5 cases: devices 0-2 PTR_RAM, 3 and -1 EXACT (-1) |
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
| 0x26 | Supexec | `bios/vectors.S:1376` | later | runs code; a table can name buffers, not code -- needs a dumper-owned function |
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

| Table | Slots | round 1 | later | excluded |
|---|---|---|---|---|
| BIOS | 12 | 8 | 3 | 1 |
| XBIOS | 142 | 81 | 43 | 18 |

Plus the probes past each table (3 BIOS, 1 XBIOS). `bios-r1.tsv` holds 147 cases; every round-1 row above names how many are its own. 35 of the 81 XBIOS round-1 slots are empty (`xbios_unimpl`) and answer their own number.

