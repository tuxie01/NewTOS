# ACC01 — Quellenanalyse: der vollstaendige `.ACC`-Lebenszyklus in EmuTOS

Status: DONE (2026-11-11) | Priority: P2 | Commit: aace85f98 | Receipt: ACC01-STATUS-DONE.MD

Migrated (owner batch 20, 2026-11-11, to DONE), evidence: receipt TASK/ACC01-STATUS-DONE.MD exists, commit aace85f98 (docs(aes): extract the .ACC lifecycle from EmuTOS), VERIFIED: aace85f98 (docs(aes): extract the .ACC lifecycle from EmuTOS) added the receipt TASK/ACC01-STATUS-DONE.MD itself, the report of this research or measurement ticket (4 files), legacy line: ERLEDIGT (2026-09-24)

Line 3 before the migration:
Status: **ERLEDIGT** (2026-09-24) · Prioritaet: **P2** · Typ: Quellenanalyse (Researcher) — Voraussetzung fuer ACC02–ACC05

Abhaengigkeiten: —

## Warum

`.ACC`-Unterstuetzung fehlt bei uns fast vollstaendig (Anwender 2026-09-22).
Vorhanden ist nur `menu_register` (35), das Titel und `ap_id` in einer
6er-Tabelle ablegt (`tos-api/src/aes/menu.rs`), sowie die Beschreibung von
`mn_cleanup`/`AC_CLOSE` in der Spec. Es fehlen: Laden, Prozessmodell,
Desk-Menue-Anbindung, `AC_OPEN`/`AC_CLOSE`-Protokoll, Lebenszyklus ueber
Programmstarts hinweg.

## Auftrag

Den Lebenszyklus aus der gepinnten Quelle extrahieren, je Schritt mit
`datei:zeile`, nach `docs/SOURCE-ANALYSIS-PROTOCOL.md`:

1. **Zaehlen und Laden**: `count_accs` (`aes/geminit.c:265-284`, `\*.ACC`
   ueber `dos_sfirst`/`dos_snext`, `FA_RO`, `NUM_ACCS = 6`
   `include/sysconf.h:37`), `load_one_acc` (`:236-257`: `dos_open`, `pgmld`,
   `pstart(gotopgm, name, addr)`), `load_accs` (`:307-315`), `free_accs`
   (`:288-303`), Startreihenfolge in `run_accs_and_desktop` (`:654-745`:
   `load_accs` **vor** `sh_tographic`, `dsptch()`, dann
   `wait_for_accs(AP_MESAG)` `:737`, `sh_main`, danach `free_accs`).
2. **Bootflag**: `BOOTFLAG_SKIP_AUTO_ACC` (`:268-270`) — woher es kommt
   (`bios/`), welche Taste, was es sonst noch ueberspringt.
3. **Prozessmodell**: `pstart`/`AESPD`, wie ein Accessory zwischen
   `appl_init` und seinem ersten `evnt_multi` steht; was `wait_for_accs`
   genau abwartet (`:588-630`, `AP_MESAG`/`AP_ACCLOSE`, `WAIT_TIMEOUT 500`).
4. **Menueanbindung**: wie die per `menu_register` gemeldeten Titel in
   `THEDESK` erscheinen (`aes/gemmnlib.c` `mn_bar`/`mn_register`
   `gemgrlib`?), wer `AC_OPEN` postet (`gemctrl.c`/`gemmnlib.c` — Fundstelle
   belegen), mit welchen `msg`-Woertern (`ap_gaccs`-Index?), und wie
   `AC_CLOSE` beim Programmstart/Ende laeuft (`mn_cleanup`
   `gemmnlib.c:740-760`, Aufrufer `ap_exit` `gemaplib.c:254`, `sh_ldapp`
   `gemshlib.c:652`).
5. **Was ein Accessory selbst tun muss** (fuer unsere spaeteren Tests):
   `appl_init`, `menu_register`, `evnt_multi`-Schleife, Reaktion auf
   `AC_OPEN`/`AC_CLOSE`, **kein** `appl_exit`/`Pterm`. Beispiel aus
   EmuTOS/Fremdquelle benennen (z. B. ein minimales `.ACC` aus dem
   FreeMiNT-/Atari-Umfeld, nur als Referenz zitiert).

## Abnahme

`docs/api/aes/accessories.md` (neu): Lebenszyklus, Zustandsdiagramm in Worten,
Vertrag je AES-Funktion, alle Zitate mit `datei:zeile`; Liste der NewTOS-Luecken
mit Verweis auf ACC02–ACC05. Keine Codeaenderung.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

## Abschluss

**2026-09-24.** Deliverable: `docs/api/aes/accessories.md` (neu, ~470 Zeilen).

**Ticket-Zitate geprueft (Abschnitt 0 der Spec).** Kein Fehltreffer (keine
Zitat-Adresse zeigt auf die falsche Funktion), aber sieben Bereiche sind
off-by-1..5 gegen die exakte Funktionsgrenze (Doc-Kommentar- oder
Leerzeilen-Versatz), u. a. `count_accs` (`:264` statt `:265`),
`run_accs_and_desktop` (endet `:750`, nicht `:745`), `wait_for_accs`
(Signatur `:599`, nicht `:588` — `:588` liegt im Doc-Kommentar). Korrigierte
Zeilen durchgehend in der neuen Spec verwendet.

**Gating (Abschnitt 6).** Kein `CONF_WITH_*` schaltet die `.ACC`-
Lebenszyklus-Funktionen selbst — sie sind in jedem EmuTOS-Build
unconditional. `TARGET_192` (`config.h:210-401`) hat keinen Effekt auf
`TARGET_512` (kein eigener `#ifdef TARGET_512`-Block existiert in
`config.h`); alle inhaltlich benachbarten Flags (`CONF_WITH_MENU_EXTENSION`,
`_3D_OBJECTS`, `_BACKGROUNDS`, `_LOADABLE_CURSORS`, `_PCGEM`) fallen für
`TARGET_512` auf ihren generischen TOS4-Default (alle `1`). **Voll an in
unserem Referenzbuild.**

**Erweiterungsfrage (Abschnitt 7).** Waehrend der Bearbeitung kam per
Zwischennachricht die Lead-Entscheidung `.NAC`, dann die Zwei-Orte-Suche
(`\ACC\` zuerst, dann Root, Dedupe nach Dateiname) — beide als Vorgabe
übernommen, nicht selbst empfohlen. Ausgearbeitet: `count_accs`'s Muster hat
in NewTOS **keine** bestehende Zeile zum Aendern (kein Scan existiert bisher
— ACC02 schreibt ihn neu); Downstream (Laden, `menu_register`,
`AC_OPEN`/`AC_CLOSE`) ist erweiterungsblind. `NUM_ACCS=6` ist laut
`sysconf.h:37-40` bereits in EmuTOS ein gemeinsamer Lade- **und**
Registrierungs-Pool — die Zwei-Orte-Suche muss ihn als **einen** Pool mit
stillem Abbruch bei 6 behandeln (EmuTOS-Beleg fuer den Abbruch-Mechanismus,
NewTOS-eigene Folgerung fuer den geteilten Pool ueber zwei Orte). Ein
klassisches `.ACC` waere so oder so nicht lauffaehig — belegt ueber
`gotopgm` (`gemasm.S:73-131`, springt in m68k-Maschinencode), nicht nur
erwartet. Die leere `ACC\`-Testordner-Attrappe auf der Karte
(`tools/make-sd-image.sh:59`) verwechselt den heutigen EmuTOS-Root-Scan
nicht (Name- und Attribut-Mismatch nachvollzogen, `fsdir.c:737-796`/
`:1802-1845`). `kernel/src/desk/ftype.rs:59-72` und `docs/MEMORY.md`
ADR-018 (`:1004-1061`) enthalten **noch kein** `.NAC` — als Luecke fuer
ACC02 benannt, nicht als Diskrepanz dieser Extraktion.

**`msg[3]`/`msg[4]`-Frage (Abschnitt 3).** Bestaetigt, unabhaengig aus der
Quelle hergeleitet (nicht nur aus `sysacc.c` uebernommen): `AC_OPEN` traegt
die Menu-ID in `msg[4]` (ueber den `ct_msgup`/`MN_SELECTED`-gemeinsamen Weg),
`AC_CLOSE` in `msg[3]` (direkter `ap_sendmsg`-Aufruf in `mn_cleanup`, kein
Title-Wort). Beide Herleitungen trafen sich mit `sysacc.c`s eigenen Zitaten.

**`sysacc.c`-Review (Abschnitt 5).** Der `AC_OPEN`/`AC_CLOSE`-Vertrag ist
**richtig** implementiert, inklusive der Asymmetrie; Lebenszyklus-Form
(kein `appl_exit`/`Pterm`) korrekt. Keine Korrektur an der Datei noetig.

**NewTOS-Luecken (Abschnitt 8) → ACC02–ACC05**, fuenf Punkte: kein
Discovery/Load-Pfad, keine Accessory-Slots in der AESPD-Tabelle,
`AC_OPEN` nie gepostet, `AC_CLOSE` nie gepostet, `sysacc.c` blockiert heute
fuer immer in `evnt_mesag` (bestaetigt als ehrlicher, dokumentierter
Zustand, kein Defekt).

Keine Produktcode-Aenderung. `docs/STATUS.md` Recent-Decisions-Eintrag
2026-09-24 gesetzt.
