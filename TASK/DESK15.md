# DESK15 — Fenstersymbole und Desktop-Verknuepfungen (`ADINSWIN`)

Status: DONE (2026-11-11) | Priority: P3 | Commit: 0364966cd | Receipt: DESK15-STATUS-DONE.MD

Migrated (owner batch 18, 2026-11-11, to DONE), evidence: receipt TASK/DESK15-STATUS-DONE.MD exists, commit 0364966cd (feat(desk): window icons - a file's icon can come from its name), VERIFIED: 0364966cd (feat(desk): window icons - a file's icon can come from its n) names the id by body, its message names the ticket-title identifier adinswin, touches code (12 files), legacy line: ERLEDIGT (2026-09-23)

Line 3 before the migration:
Status: **ERLEDIGT** (2026-09-23) · Prioritaet: **P3** · Typ: Funktionsluecke — gemeldet in DESK03

Abhaengigkeiten: [DESK03](DESK03.md) (erledigt)

## Sachverhalt

`kernel/src/desk/ins.rs` hat die Form eines `CONF_WITH_WINDOW_ICONS = 0`-Baus:
`ins_icon(0)` geht direkt auf `install_desktop_icon(NULL)`
(`references/emutos/desk/deskins.c:1262-1263`, der `#else`-Zweig), statt mit
`STICNTYP` zu fragen, ob ein Desktop- oder ein Fenstersymbol gemeint ist. Der
Grund ist kein Zeitmangel, sondern eine fehlende Ebene: beide Merkmale haengen
einen `ANODE` an einen **Dateinamen**, und NewTOS' `win_bldview` nimmt das
Symbol eines Fensterobjekts aus der Endung (`ftype::classify`), nie aus einer
Zuordnung. `ADINSWIN` waere ohne diesen Leser ein Bedienelement ohne Wirkung.

Betroffen sind damit auch `CONF_WITH_DESKTOP_SHORTCUTS` (ein Desktop-Symbol,
das auf eine Datei oder einen Ordner zeigt) — dieselbe fehlende Ebene.

## Arbeit

Reihenfolge ist vorgegeben: **zuerst** der Leser. `win_bldview` muss das Symbol
eines Fensterobjekts aus der `ANODE`-Liste beziehen koennen (EmuTOS
`deskwin.c`/`deskapp.c` `app_afind_by_name`), mit der Endungsregel als
Rueckfall. Erst danach `ADINSWIN` (`desk_rsc.c`) und der `STICNTYP`-Zweig in
`ins_icon`.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

---

## Abschluss

Erledigt am 2026-09-23, Branch `desk-rebuild` (Basis `f206a2a`), Aenderungen
**uncommitted** im Baum. Messung:
[`measurements/measurement-2026-09-23-desk15/`](../measurements/measurement-2026-09-23-desk15/).

**Die Analyse des Tickets stimmte, mit einer Einschraenkung.** Die
Reihenfolge — erst der Leser, dann `ADINSWIN` — war richtig, und
`CONF_WITH_WINDOW_ICONS` ist im Profil Falcon030/TOS4 (`make 512`) tatsaechlich
`1` (`include/config.h:1866-1868`; die `0` bei `:322-324` steht in
`#ifdef TARGET_192`, und `make 512` definiert `TARGET_512`,
`Makefile:560-562`). Falsch war nur, die beiden Haelften als *dieselbe*
fehlende Ebene zu fuehren: ein Fenstersymbol ist ein `#I`/`#N`/`#D`-Knoten der
**Zuordnungs**liste (`desk::inf`s Session-Modell), eine Desktop-Verknuepfung
ein `#X`/`#V`-Knoten der **Desktop**liste `g_ahead`. Sie sind trennbar, EmuTOS
schaltet sie getrennt, und gebaut ist hier nur die erste.

### Gebaut

1. **Der Leser.** `DeskState::win_icalc` (`kernel/src/desk/win.rs`) loest jedes
   Fensterobjekt nach Typ und Name gegen die `ANODE`-Liste auf
   (`deskwin.c:377-384`), `build_item` nimmt `f_isap ? a_aicon : a_dicon`
   (`:462-474`). Die Endungsregel `ftype::classify` bleibt der Rueckfall.
2. **Die Listenprimitive.** `app_afind_by_name`, `allocate_window_anode`,
   `set_anode_data`, `set_icon`, `app_free` in `kernel/src/desk/inf.rs` — dort,
   wo der Speicher liegt, den sie bearbeiten.
3. **`default_type_anodes()`** steht jetzt in EmuTOS' *Live*-Reihenfolge (sonst
   verschluckt `#F FF 07 @ *.*@` jede Programmendung) und hat drei eigene
   Zeilen `#G *.NAP` / `#F *.NCP` / `#P *.NTP` — ADR-018s Programmfamilie an
   genau der Stelle, an der EmuTOS seine deklariert. `build_inf()` legt das
   synthetisierte Modell in die Session, wie `app_start` seinen `infbuf` in
   `G.g_ahead` parst; ohne das ist die Liste auf einer frischen Karte leer und
   EmuTOS' „kann nicht passieren"-Fall (`deskins.c:1157-1161`) passiert.
4. **Der Kasten.** `ins::install_window_icon` (`ADINSWIN`),
   `ins::is_executable`, die drei Zweige von `ins::ins_icon` (`STICNTYP`,
   Desktop-Symbole, Fensterauswahl) und der `rebld < 0`-Arm von `IICNITEM`
   (`win_bdall(); win_shwall();`) in `kernel/src/desk/main.rs`.

### Offen, als eigenes Ticket

[DESK30](DESK30.md) — `CONF_WITH_DESKTOP_SHORTCUTS`. Braucht eine
`AT_ISFILE`/`AT_ISFOLD`-Variante von `global::Anode` plus `fun_win2win`s
Ablegen-auf-Programm; ohne die ist es kein halber Kasten, sondern ein anderes
Stueck Arbeit. Der DESK18-Nebenbefund (*Default dir* wird erst mit
Verknuepfungen sichtbar) haengt dort.
