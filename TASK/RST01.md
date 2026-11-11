# RST01 — Neustart: Warm- und Kaltstart, Ctrl+Alt+Del

Status: DONE (2026-11-11) | Priority: P2 | Commit: 95be2e0dd | Receipt: RST01-STATUS-DONE.MD

Migrated (owner batch 3, 2026-11-11), evidence: receipt TASK/RST01-STATUS-DONE.MD exists; commit 95be2e0dd; TASK/RST01-STATUS-DONE.MD, commit 95be2e0dd (feat(bios): software restart, Ctrl+Alt+Del and OSHEADER reset entry (R), fix/feat commits naming the id: 95be2e0dd (n=1)

Line 3 before the migration:
Status: **ERLEDIGT** 2026-10-17 (QEMU, `f63c39c1`; Pi abgenommen 2026-10-18, `docs/HW-TEST-LOG.md`) — [Quittung](RST01-STATUS-DONE.MD) · Prioritaet: **P2** · Typ: fehlende Funktion — Anwender 2026-10-17

## Anlass

Anwender 2026-10-17: *„reboot oder auch Reset haben wir noch nicht, auch
Strg+Alt+Entf geht noch nicht. Wäre aber manchmal sinnvoll.“* Ausloeser war
[X04](X04.md): TeraDesks „Reboot“ springt ueber den Reset-Einsprung aus dem
`OSHEADER` auf Adresse 0.

## Vorbild EmuTOS

- `bios/ikbd.c:743-754`: **Ctrl+Alt+Del** → `warm_reset()`,
  **Ctrl+Alt+RShift+Del** → `cold_reset()` (LShift darf nicht gedrueckt sein).
- `bios/ikbd.c:~1009`: Sonderfall, wenn Ctrl+Alt+Del nach dem Neustart noch
  gehalten wird.
- `warm_reset`/`cold_reset` und der Reset-Einsprung im `OSHEADER` (den
  TeraDesk liest): in EmuTOS nachlesen, was sie tun (Speicher/Sysvars,
  `resvalid`/`resvector`, `memvalid`), bevor etwas gebaut wird.

## Auftrag

1. **Spec zuerst** (Regel 5): EmuTOS `warm_reset`/`cold_reset`, die Tasten-
   pruefung, den `OSHEADER`-Reset-Einsprung und die Reset-Sysvars nach
   `docs/api/` extrahieren.
2. **Hardware:** wie die Pi 4 neu startet (BCM2711 PM-Watchdog o. ae.) —
   nur aus Datenblatt bzw. der Quellen-Hierarchie Track B, nicht geraten.
   QEMU raspi4b: pruefen, ob es den Mechanismus nachbildet.
3. Festlegen (Anwender), was warm und kalt auf NewTOS heissen: z. B. warm =
   Kern neu initialisieren ohne Firmware, kalt = echter Pi-Neustart; ob
   offene Dateien vorher geschlossen werden.
4. Bauen: Tastenkombination im Tastaturpfad (IKBD und USB), Reset-Einsprung
   im `OSHEADER` gueltig (behebt [X04](X04.md) ohne Aenderung an TeraDesk).

[Gemeinsame Regeln](README.md) gelten.

## Quellenbefund 2026-10-17 (Review `1d17eae8`)

`docs/api/structures/osheader.md:128-132` und der Kommentar in
`tos-api/src/bdos/process/loader.rs` (~1150) sagen „EmuTOS setzt `os_rsvl = 0`“ —
**falsch**: `references/emutos/bios/startup.S:92-93` legt dort
`os_res1: .dc.l _main` ab, und `reseth` (+4) ist ebenfalls `_main`. TeraDesk
liest `*((long *) os_start + 4)`: auf m68k Offset 0x10 = `os_res1` = Neustart;
auf LP64 (`long` = 8 Byte) Offset 0x20..0x27 (`os_root`/`os_kbshift`), daher
`pc=0`. Nur +0x10 zu setzen behebt es also **nicht** — Entscheidung noetig
(Toolchain-/Overlay-Weg wie bei anderen LP64-Struct-Fallen vs. Kern), bevor
RST01 den Einsprung baut. Spec und Kommentar zuerst berichtigen.

## Spec und Hardware (2026-10-17, `9b773120`)

`docs/api/bios/_reset-warm-cold.md` (EmuTOS `warm_reset`/`cold_reset`,
Tastenkombination `ikbd.c:734-747`, OSHEADER-Einspruenge alle → `_main`),
`docs/hardware/pm-watchdog-reset.md` (Pi-4-Neustart per PM-Watchdog: **nicht**
im BCM2711-Datenblatt, belegt nur durch Circle `lib/sysinit.cpp:231-241`,
`bcm2835.h:261-269`; QEMU `bcm2835_powermgt.c` bildet dieselben Register nach),
`docs/api/structures/osheader.md` berichtigt (`os_res1 = _main`).

## Anwenderentscheidungen (2026-10-17)

1. **Kalt** (Ctrl+Alt+RShift+Del, TeraDesk „Reboot“): echter Pi-Neustart ueber
   den PM-Watchdog. Restrisiko Quellenlage (Circle, nicht Datenblatt) bewusst
   getragen; zuerst Kurzlauf in QEMU, dann Pi.
2. **Warm** (Ctrl+Alt+Del): vorerst wie kalt. Der EmuTOS-treue Warmstart
   (Sysvars bleiben) → eigenes Ticket [RST02](RST02.md).
3. **Daten:** wie EmuTOS — nur Cache leeren, keine Puffer schreiben.
4. **TeraDesk:** LP64-Fix als Overlay (`sdk/overlays/teradesk/main.c`, feste
   32-Bit-Breiten fuer `os_start`, `+0x10` und die Magic-Pokes); Kern legt den
   Einsprung EmuTOS-treu bei `+0x00`/`+0x04`/`+0x10`.
5. Tastaturen (Standard, nicht gefragt): IKBD und USB im gemeinsamen Pfad,
   Bedingung exakt wie `ikbd.c:734-747` (Make-Code 0x53, Ctrl+Alt, nicht LShift).
