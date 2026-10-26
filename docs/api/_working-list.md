# Phase 00 — Working List

> Tracks per-function progress through the Phase 00 extraction.
> Updated at the start and end of every working session.
>
> **EmuTOS pinned commit:** `1a7484ab`

Legend: ✅ done · ⏳ in progress · 🔒 blocked · ⏸ deferred · ☐ not started

---

## D1 — Source layout map

- [✅] `docs/emutos-map.md`

## D2 — Trap dispatchers

- [✅] `docs/api/gemdos/_dispatcher.md` — Trap 1 (`bdos/bdosmain.c:osif`)
- [✅] `docs/api/bios/_dispatcher.md` — Trap 13 (`bios/vectors.S:_biostrap`)
- [✅] `docs/api/xbios/_dispatcher.md` — Trap 14 (`bios/vectors.S:_xbiostrap`)

## D3 — GEMDOS function specs

### Console group (`bdos/console.c`)

| ID    | Name        | EmuTOS func     | Status | Spec file                                 |
|------:|-------------|-----------------|:------:|-------------------------------------------|
| 0x01  | Cconin      | `xconin`        | ✅     | `gemdos/console/01-cconin.md`             |
| 0x02  | Cconout     | `xconout`       | ✅     | `gemdos/console/02-cconout.md`            |
| 0x03  | Cauxin      | `xauxin`        | ✅     | `gemdos/console/03-cauxin.md`             |
| 0x04  | Cauxout     | `xauxout`       | ✅     | `gemdos/console/04-cauxout.md`            |
| 0x05  | Cprnout     | `xprtout`       | ✅     | `gemdos/console/05-cprnout.md`            |
| 0x06  | Crawio      | `xrawio`        | ✅     | `gemdos/console/06-crawio.md`             |
| 0x07  | Crawcin     | `xrawcin`       | ✅     | `gemdos/console/07-crawcin.md`            |
| 0x08  | Cnecin      | `xnecin`        | ✅     | `gemdos/console/08-cnecin.md`             |
| 0x09  | Cconws      | `xconws`        | ✅     | `gemdos/console/09-cconws.md`             |
| 0x0A  | Cconrs      | `xconrs`        | ✅     | `gemdos/console/0a-cconrs.md`             |
| 0x0B  | Cconis      | `xconstat`      | ✅     | `gemdos/console/0b-cconis.md`             |
| 0x10  | Cconos      | `xconostat`     | ✅     | `gemdos/console/10-cconos.md`             |
| 0x11  | Cprnos      | `xprtostat`     | ✅     | `gemdos/console/11-cprnos.md`             |
| 0x12  | Cauxis      | `xauxistat`     | ✅     | `gemdos/console/12-cauxis.md`             |
| 0x13  | Cauxos      | `xauxostat`     | ✅     | `gemdos/console/13-cauxos.md`             |

### Process group (`bdos/proc.c`)

| ID    | Name        | EmuTOS func     | Status | Spec file                                 |
|------:|-------------|-----------------|:------:|-------------------------------------------|
| 0x00  | Pterm0      | `x0term`        | ✅     | `gemdos/process/00-pterm0.md`             |
| 0x4B  | Pexec       | `xexec`         | ✅     | `gemdos/process/4b-pexec.md`              |
| 0x4C  | Pterm       | `xterm`         | ✅     | `gemdos/process/4c-pterm.md`              |
| 0x31  | Ptermres    | `xtermres`      | ✅     | `gemdos/process/31-ptermres.md`           |
| 0x4D  | (Pwait)     | (not in funcs)  | —     | not implemented in EmuTOS                 |

### Memory group (`bdos/umem.c`, `iumem.c`)

| ID    | Name        | EmuTOS func     | Status |
|------:|-------------|-----------------|:------:|
| 0x14  | Maddalt     | `xmaddalt`      | ✅      |
| 0x44  | Mxalloc     | `xmxalloc`      | ✅      |
| 0x48  | Malloc      | `xmalloc`       | ✅      |
| 0x49  | Mfree       | `xmfree`        | ✅      |
| 0x4A  | Mshrink     | `xsetblk`       | ✅      |

### Date/time group (`bdos/time.c`)

| ID    | Name        | EmuTOS func     | Status |
|------:|-------------|-----------------|:------:|
| 0x2A  | Tgetdate    | `xgetdate`      | ✅      |
| 0x2B  | Tsetdate    | `xsetdate`      | ✅      |
| 0x2C  | Tgettime    | `xgettime`      | ✅      |
| 0x2D  | Tsettime    | `xsettime`      | ✅      |

### Drive/DTA/version group (`bdos/bdosmain.c`, `fsdrive.c`)

| ID    | Name        | EmuTOS func     | Status |
|------:|-------------|-----------------|:------:|
| 0x0E  | Dsetdrv     | `xsetdrv`       | ✅      |
| 0x19  | Dgetdrv     | `xgetdrv`       | ✅      |
| 0x1A  | Fsetdta     | `xsetdta`       | ✅      |
| 0x2F  | Fgetdta     | `xgetdta`       | ✅      |
| 0x30  | Sversion    | `xgetver`       | ✅      |
| 0x36  | Dfree       | `xgetfree`      | ✅      |

### File group (`bdos/fsopnclo.c`, `fsio.c`)

| ID    | Name        | EmuTOS func     | Status | Spec file                                 |
|------:|-------------|-----------------|:------:|-------------------------------------------|
| 0x3C  | Fcreate     | `xcreat`        | ✅     | `gemdos/file/3c-fcreate.md`               |
| 0x3D  | Fopen       | `xopen`         | ✅     | `gemdos/file/3d-fopen.md`                 |
| 0x3E  | Fclose      | `xclose`        | ✅     | `gemdos/file/3e-fclose.md`                |
| 0x3F  | Fread       | `xread`         | ✅     | `gemdos/file/3f-fread.md`                 |
| 0x40  | Fwrite      | `xwrite`        | ✅     | `gemdos/file/40-fwrite.md`                |
| 0x41  | Fdelete     | `xunlink`       | ✅     | `gemdos/file/41-fdelete.md`               |
| 0x42  | Fseek       | `xlseek`        | ✅     | `gemdos/file/42-fseek.md`                 |
| 0x43  | Fattrib     | `xchmod`        | ✅     | `gemdos/file/43-fattrib.md`               |
| 0x45  | Fdup        | `xdup`          | ✅     | `gemdos/file/45-fdup.md`                  |
| 0x46  | Fforce      | `xforce`        | ✅     | `gemdos/file/46-fforce.md`                |
| 0x4E  | Fsfirst     | `xsfirst`       | ✅     | `gemdos/file/4e-fsfirst.md`               |
| 0x4F  | Fsnext      | `xsnext`        | ✅     | `gemdos/file/4f-fsnext.md`                |
| 0x56  | Frename     | `xrename`       | ✅     | `gemdos/file/56-frename.md`               |
| 0x57  | Fdatime     | `xgsdtof`       | ✅     | `gemdos/file/57-fdatime.md`               |

### Directory group (`bdos/fsdir.c`)

| ID    | Name        | EmuTOS func     | Status | Spec file                                 |
|------:|-------------|-----------------|:------:|-------------------------------------------|
| 0x36  | Dfree       | `xgetfree`      | ✅     | `gemdos/directory/36-dfree.md`            |
| 0x39  | Dcreate     | `xmkdir`        | ✅     | `gemdos/directory/39-dcreate.md`          |
| 0x3A  | Ddelete     | `xrmdir`        | ✅     | `gemdos/directory/3a-ddelete.md`          |
| 0x3B  | Dsetpath    | `xchdir`        | ✅     | `gemdos/directory/3b-dsetpath.md`         |
| 0x47  | Dgetpath    | `xgetdir`       | ✅     | `gemdos/directory/47-dgetpath.md`         |

### Misc

| ID    | Name        | EmuTOS func     | Status | Spec file                                 |
|------:|-------------|-----------------|:------:|-------------------------------------------|
| 0x15  | Srealloc    | `srealloc`      | ✅     | `gemdos/15-srealloc.md`                   |

---

## D4 — BIOS function specs (`bios/bios.c`, `bios_vecs[]`)

| ID    | Name        | EmuTOS func   | Status | Spec file                  |
|------:|-------------|---------------|:------:|----------------------------|
| 0x00  | Getmpb      | `getmpb`      | ✅      |                            |
| 0x01  | Bconstat    | `bconstat`    | ✅     | `bios/01-bconstat.md`      |
| 0x02  | Bconin      | `bconin`      | ✅     | `bios/02-bconin.md`        |
| 0x03  | Bconout     | `bconout`     | ✅     | `bios/03-bconout.md`       |
| 0x04  | Rwabs       | `lrwabs`      | ✅      |                            |
| 0x05  | Setexc      | `setexc`      | ✅      |                            |
| 0x06  | Tickcal     | `tickcal`     | ✅      |                            |
| 0x07  | Getbpb      | `getbpb`      | ✅      |                            |
| 0x08  | Bcostat     | `bcostat`     | ✅     | `bios/08-bcostat.md`       |
| 0x09  | Mediach     | `mediach`     | ✅      |                            |
| 0x0A  | Drvmap      | `drvmap`      | ✅      |                            |
| 0x0B  | Kbshift     | `kbshift`     | ✅      |                            |

---

## D5 — XBIOS function specs (`bios/xbios.c`)

Triage doc: [✅] `docs/api/xbios/_triage.md`

| ID    | Name         | EmuTOS func   | Status | Spec file                                 |
|------:|--------------|---------------|:------:|-------------------------------------------|
| 0x00  | Initmous     | `Initmous`    | ✅     | `xbios/00-initmous.md`                    |
| 0x02  | Physbase     | `physbase`    | ✅     | `xbios/02-physbase.md`                    |
| 0x03  | Logbase      | `logbase`     | ✅     | `xbios/03-logbase.md`                     |
| 0x04  | Getrez       | `getrez`      | ✅     | `xbios/04-getrez.md`                      |
| 0x05  | Setscreen    | `setscreen`   | ✅     | `xbios/05-setscreen.md`                   |
| 0x06  | Setpalette   | `setpalette`  | ✅     | `xbios/06-setpalette.md`                  |
| 0x07  | Setcolor     | `setcolor`    | ✅     | `xbios/07-setcolor.md`                    |
| 0x08  | Floprd       | `floprd`      | ✅     | `xbios/_floppy-stubs.md` (group)            |
| 0x09  | Flopwr       | `flopwr`      | ✅     | `xbios/_floppy-stubs.md` (group)            |
| 0x0A  | Flopfmt      | `flopfmt`     | ✅     | `xbios/_floppy-stubs.md` (group)            |
| 0x0C  | Midiws       | `midiws`      | ✅     | `xbios/0c-midiws.md`                       |
| 0x0E  | Iorec        | `iorec`       | ✅     | `xbios/0e-iorec.md`                        |
| 0x0F  | Rsconf       | `rsconf`      | ✅     | `xbios/0f-rsconf.md`                       |
| 0x10  | Keytbl       | `keytbl`      | ✅     | `xbios/10-keytbl.md`                       |
| 0x11  | Random       | `random`      | ✅     | `xbios/11-random.md`                      |
| 0x12  | Protobt      | `protobt`     | ✅     | `xbios/_floppy-stubs.md` (group, PHASE-04)  |
| 0x13  | Flopver      | `flopver`     | ✅     | `xbios/_floppy-stubs.md` (group)            |
| 0x14  | Scrdmp       | `scrdmp`      | ✅     | `xbios/14-scrdmp.md`                       |
| 0x15  | Cursconf     | `cursconf`    | ✅     | `xbios/15-cursconf.md`                    |
| 0x16  | Settime      | `settime`     | ✅     | `xbios/16-settime.md`                     |
| 0x17  | Gettime      | `gettime`     | ✅     | `xbios/17-gettime.md`                     |
| 0x18  | Bioskeys     | `bioskeys`    | ✅     | `xbios/18-bioskeys.md`                    |
| 0x19  | Ikbdws       | `ikbdws`      | ✅     | `xbios/19-ikbdws.md`                       |
| 0x1C  | Giaccess     | `giaccess`    | ✅     | `xbios/_psg-group.md` (group)              |
| 0x1D  | Offgibit     | `offgibit`    | ✅     | `xbios/_psg-group.md` (group)              |
| 0x1E  | Ongibit      | `ongibit`     | ✅     | `xbios/_psg-group.md` (group)              |
| 0x20  | Dosound      | `dosound`     | ✅     | `xbios/_psg-group.md` (group)              |
| 0x22  | Kbdvbase     | `kbdvbase`    | ✅     | `xbios/22-kbdvbase.md`                    |
| 0x23  | Kbrate       | `kbrate`      | ✅     | `xbios/23-kbrate.md`                       |
| 0x25  | Vsync        | `vsync`       | ✅     | `xbios/25-vsync.md`                        |
| 0x26  | Supexec      | `supexec`     | ✅     | `xbios/26-supexec.md`                     |
| 0x29  | Floprate     | `floprate`    | ✅     | `xbios/_floppy-stubs.md` (group)            |
| 0x2A  | DMAread      | `DMAread`     | ✅     | `xbios/_floppy-stubs.md` (group)            |
| 0x2B  | DMAwrite     | `DMAwrite`    | ✅     | `xbios/_floppy-stubs.md` (group)            |
| 0x2C  | Bconmap      | `bconmap`     | ✅     | `xbios/2c-bconmap.md`                      |
| 0x40  | Blitmode     | `blitmode`    | ✅     | `xbios/40-blitmode.md`                     |
| 0x50..0x57 | Eset*    | (TT shifter)  | ✅     | `xbios/_tt-shifter.md` (group)              |

Unimplemented entries (`xbios_unimpl`-routed under NewTOS profile)
documented collectively in `docs/api/xbios/_triage.md`.

---

## D6 — Data structures

- [✅] `docs/api/structures/dta.md` (DTA)
- [✅] `docs/api/structures/basepage.md` (PD)
- [✅] `docs/api/structures/bpb.md` (BPB)
- [✅] `docs/api/structures/md.md` (MD + MPB)
- [✅] `docs/api/structures/ofd.md` (OFD + DFD + FCB)
- [✅] `docs/api/structures/dnd.md` (DND + DMD)
- [✅] `docs/api/structures/osheader.md` (OSHEADER + GEM_MUPB)
- [✅] `docs/api/structures/iorec.md` (IOREC)

## D7 — Subsystem state

- [☐] `docs/api/gemdos/_state.md`
- [✅] `docs/api/bios/_state.md` (chardev tables documented; rest TBD)
- [☐] `docs/api/xbios/_state.md`

## D8 — AES (deferred — likely PHASE-00b)

## D9 — VDI (deferred — likely PHASE-00b)

## D10 — Line-A

- [☐] `docs/api/_line-a.md`

## D11 — Discrepancy log

- [✅] `docs/api/_discrepancies.md` (file present, entries appended as found)

## D12 — Dependency graph

- [✅] `docs/api/_dependency-graph.md` (file present, entries appended)

## D13 — Completion report

- [☐] `docs/api/_phase00-completion.md` (final, end of phase)

