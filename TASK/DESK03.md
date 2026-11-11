# DESK03 — Options → Install icon / Remove icon

Status: PARKED (2026-11-11) | Priority: P3 | Reason: legacy, evidence unverified; receipt DESK03-STATUS-DONE.MD exists | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 15, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: no cited file of the ticket/receipt is touched by 7dd743e1f (feat(desk): Read .INF file asks, checks and report), named by body

Line 3 before the migration:
Status: **ERLEDIGT (2026-09-24) — Pi-Sichtabnahme offen** · Prioritaet: **P3** · Typ: Funktionsluecke desk/

Abhaengigkeiten: [DESK08](DESK08.md), [DESK09](DESK09.md)

## Sachverhalt

`Item::InstallIcon` und `Item::RemoveIcon` ohne Handler. EmuTOS:
`ins_devices`/`ins_icon` (`references/emutos/desk/deskins.c`, Baeume
`ADINSDSK`/`ADINSAPP`): ein Symbol auf dem Desktop einrichten (Laufwerk,
Ordner, Papierkorb, Drucker), Typ und Beschriftung waehlen; Entfernen
loescht den `#M`/`#T`-Eintrag. Persistenz in der `.INF`.

## Arbeit

Beide Dialoge, Bindung an unsere Desktop-Symboltabelle
(`kernel/src/desk/global.rs`, `icon.rs`), Persistenz ueber DESK09.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

## Abschluss

**ERLEDIGT (2026-09-24) — Pi-Sichtabnahme offen.** Quittung:
[DESK03-STATUS-DONE.MD](DESK03-STATUS-DONE.MD).

Beide Handler stehen in `desk::main::do_optnmenu` (`IICNITEM`/`RICNITEM`,
`deskmain.c:862-881`); der Motor ist das neue Modul
[`kernel/src/desk/ins.rs`](../kernel/src/desk/ins.rs) (`deskins.c`:
`grid_free`, `ins_posdisk`, `install_desktop_icon` mit dem `ADINSDSK`-Baum,
`ins_icon`, `rmv_icon`). Die Voraussetzung dafuer war strukturell und ist
mitgeliefert: die Desktop-Symboltabelle ist jetzt **Zustand** (`G.g_ahead`,
`global::Anode`), aus der `app_blddesk` baut, statt bei jedem Aufruf aus
`dos_sdrv(dos_gdrv())` abgeleitet zu werden. Persistenz ueber DESK09
(`#M`/`#T`/`#O`, jetzt mit echter Bildnummer). `men_update` schaltet
*Remove desktop icon* ab, solange kein Desktop-Symbol ausgewaehlt ist
(`deskmain.c:540-542`) — ein graues Menueelement ist EmuTOS' ganze Antwort
darauf, es gibt keinen Alert.

**Geprueft:** `measurements/measurement-2026-09-24-desk03-desk04/` (QEMU raspi4b, 640x480,
IKBD-Injektor): Menuezustand in beiden Auswahlfaellen, Entfernen (Symbol weg,
rueckstandsfrei), der Dialog mit Schrittwerk und Typwahl, `Save desktop` +
Kaltstart in beide Richtungen, `fsck_msdos` sauber, null `[EXC]`/`[PANIC]`.
`desk::ins::selftest()` 14/14. Hostsuite 932/0/1, beide Boards
tos-api 64 / kernel 94, Clippy 80/125, Delta 0.

**Offen:** Pi 4, HW-Checkliste Punkt **38** — vier Laufwerke, die
Tastatureingabe (Laufwerkskennung, Beschriftung), das Ziehen und die
1920x1080-Optik des Kastens.

**Nicht enthalten, gemeldet:** Fenstersymbole (`ADINSWIN`,
`CONF_WITH_WINDOW_ICONS`) und Desktop-Verknuepfungen
(`CONF_WITH_DESKTOP_SHORTCUTS`) — beide haengen einen `ANODE` an einen
Dateinamen, und NewTOS' `win_bldview` nimmt das Symbol aus der Endung.
