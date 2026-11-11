# PROC01 — `Pexec` Modi 5, 6 und 7 fehlen (EmuTOS hat sie)

Status: PARKED (2026-11-11) | Priority: P2 | Reason: legacy, evidence unverified; no receipt | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 17, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: no cited file of the ticket/receipt is touched by cc511c6fa (fix(libc): system() links, through mintlib's TOS b), named by body

Line 3 before the migration:
Status: **ERLEDIGT** (2026-10-05, mit ARCH03 Stufe 2, [Statusdatei](ARCH03-2-STATUS-DONE.MD)) · Prioritaet: **P2** (nach dem OFAM-Termin) · Typ: Fehlende EmuTOS-Funktion — aus [PORT10](PORT10.md)

Abhaengigkeiten: [MEM03](MEM03.md) (Umgebungsblock in EL0; Modus 5/7 legen ebenfalls eine Umgebung an)

## Sachverhalt

NewTOS beantwortet `PE_BASEPAGE` (5), `PE_GOTHENFREE` (6) und
`PE_BASEPAGEFLAGS` (7) mit `EINVFN` (`tos-api/src/bdos/process/pexec.rs:177-179`).
EmuTOS setzt alle drei um (`references/emutos/bdos/proc.c:210-247`): 5/7
legen Umgebung und Basepage an (`alloc_env`, `alloc_tpa`, `init_pd_fields`,
`p_flags`), 6 uebertraegt den Besitz an das Kind und startet es wie 4.

Das ist eine Luecke gegenueber EmuTOS, kein Sonderwunsch. Sichtbar wurde sie
in PORT10: mintlibs `system()` hat einen POSIX-Zweig mit `vfork`, und `vfork`
ist auf TOS `tfork` = `Pexec(5)` + `Pexec(4/6)` auf Code im eigenen
Adressraum. Bis PROC01 steht, linkt `system()` nur ueber ein SDK-Overlay ohne
diesen Zweig (Entscheidung Anwender 2026-10-02).

## Arbeit

1. Spec in `docs/api/gemdos/` fuer die drei Modi aus EmuTOS (Regel 5).
2. Umsetzen; die Basepage muss im EL0-Raum liegen (wie Malloc, ADR-021).
3. Danach mintlibs `vfork` (m68k-`.S`) nach aarch64 portieren und
   `mintlib/thread.c` (`__waitval`, `__waittime`) LP64-sauber uebernehmen;
   das `system.c`-Overlay wieder entfernen.

Abnahme: `system()` aus mintlib unveraendert; Hosttests und ein QEMU-Lauf mit
`Pexec(5)`+`Pexec(6)`.

## Erledigt (2026-10-05, ARCH03 Stufe 2)

- Spec: `docs/api/gemdos/process/4b-pexec.md` (NewTOS-Notizen 2b/2d).
- `Pexec` 5/7 legen Umgebung + Basepage an (Besitz beim Aufrufer, Slot wie
  `PE_LOAD`), 4/6 starten sie bei `p_tbase`, 6 gibt Basepage und Umgebung an
  das Kind (`tos-api/src/bdos/process/{loader,pexec}.rs`); die Basepage liegt
  im gemeinsamen Adressraum (Stufe 1 statt Fenster).
- mintlibs `vfork` nach AArch64 portiert (`sdk/libc/newtos-arch/vfork.S`,
  dazu `setstack.S`), `mintlib/thread.c` LP64-sauber ueber
  `sdk/libc/overlay/mintlib/thread.c` (nur Casts), das `system.c`-Overlay
  ist entfernt — `system()` ist mintlibs unveraendertes `posix/system.c`.
- Abnahme: Hosttests (`arch03_pexec_567_ptermres.rs`, `system-host.sh`);
  QEMU raspi4b `P567: PASS` (Pexec 5+4, 5+6, 7) und `VFORK: PASS` (vfork,
  waitpid, `system()` ueber den POSIX-Zweig), PORT10 unveraendert 42/44 —
  `measurements/measurement-2026-10-05-arch03-2/`.
- Nebenbefund (nicht behoben): mintlibs `<sys/wait.h>` liest den Status ueber
  big-endian-Bitfelder; `WEXITSTATUS` ergibt auf AArch64 immer 0.
