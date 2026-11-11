# DESK02 — Options → Set preferences: Einstellungsdialog

Status: PARKED (2026-11-11) | Priority: P2 | Reason: legacy, evidence unverified; receipt DESK02-STATUS-DONE.MD exists | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 14, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: no cited file of the ticket/receipt is touched by 43e257776 (feat(desk): copy and overwrite confirmations, and ), named by body

Line 3 before the migration:
Status: **ERLEDIGT (2026-09-24) — Pi-Sichtabnahme offen** · Prioritaet: **P2** · Typ: Funktionsluecke desk/

Abhaengigkeiten: [DESK08](DESK08.md), [DESK09](DESK09.md) (`cnx_put`/`cnx_get` speichert die Werte)

## Sachverhalt

`Item::SetPrefs` ohne Handler. EmuTOS: `do_optnmenu`/`PREFITEM` →
`inf_pref` (`references/emutos/desk/deskins.c`/`deskinf.c`, Baum `ADSETPRE`):
Doppelklickgeschwindigkeit, Bestaetigung fuer Loeschen/Kopieren/Ueberschreiben,
Zeitformat, Datumsformat, Sortierung. Die Werte landen im `#E`-Satz der
`.INF` (`INF_E1_*`/`INF_E2_*`, `desk/deskapp.c:86-115`).

## Arbeit

Dialog nachbauen, Werte an die vorhandenen Stellen binden
(`ev_dclick`/Doppelklickrate, Bestaetigungsflags in `desk/`), Persistenz
ueber DESK09. Hosttest je Feld, Screendump.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

## Abschluss

**ERLEDIGT (2026-09-24) — Pi-Sichtabnahme offen.** Quittung:
[DESK02-STATUS-DONE.MD](DESK02-STATUS-DONE.MD).

`kernel/src/desk/inf.rs::inf_pref` (1:1 zu `deskinf.c:721-818`, samt
`inf_which`/`inf_gindex`/`inf_what`), aufgerufen aus `main::do_optnmenu` als
`if inf_pref() { desk_all(...) }`; `men_update` gibt `Item::SetPrefs` frei
(EmuTOS listet `PREFITEM` in keiner `ILL_*`-Tabelle). Spec:
`docs/api/desktop/deskinf.md`, Abschnitt "NewTOS implementation notes —
`inf_pref` and the date/time formatters (DESK02)".

**Vier Felder, in EmuTOS' Reihenfolge, je unter einem eigenen `G_IBOX`:**
Loeschruecksprache (Yes/No), Doppelklickrate (1-5), Zeitformat (12H/24H),
Datumsformat (MDY/DMY). Der `G_IBOX` ist tragend, nicht schmueckend — der AES
waehlt Geschwister ueber den **Elternknoten** ab, zwei Gruppen unter einem
Elternknoten waeren fuer ihn eine. Neuer Hosttest
`tos-api/tests/aes_desk_prefs_radio.rs`, 3 Faelle; der dritte zeigt das
Gegenbeispiel.

**Drei Felder von `ADSETPRE` fehlen, mit Grund** (kein totes Bedienelement):

* *Confirm copies* — EmuTOS' Schalter schaltet den Kopierdialog in `d_doop`
  ein (`deskdir.c:939`); NewTOS' Kopierwerk hat keinen.
* *Confirm overwrites* — dasselbe eine Ebene tiefer (`deskdir.c:325`);
  NewTOS ueberschreibt immer (`Fcreate` kuerzt).
* *Default* bei beiden Formaten — das meint den `_IDT`-Keks, den EmuTOS mit
  `Supexec(get_idt_cookie)` liest (`desksupp.c:219`). XBIOS `Supexec` (0x26)
  gibt es nicht, und der Desktop darf seit DESK10 nur ueber den Trap an
  Kernelzustand.

Die `Idt`-Werte werden dennoch gelesen und unveraendert zurueckgeschrieben;
**aufgeloest** werden sie an einer Stelle (`model::TimeFmt::rendered` /
`DateFmt::rendered`), damit Dialog und Textansicht nicht auseinanderlaufen
koennen.

**Bindung.** Loeschruecksprache → `dir::confirm_deletes()` vor `fun_del` und
vor dem Fenster→Papierkorb-Abwurf (der Desktopsymbol→Papierkorb-Abwurf
bleibt **ungefiltert**, wie EmuTOS' `wants_to_delete_files`). Rate → der AES
ueber die neue `evnt_dclick`-Bindung (Opcode 26). Formate →
`inf::fmt_date`/`fmt_time`, aus `win.rs` dorthin gezogen, wo EmuTOS sie hat,
und von Textansicht **und** Show-Info gelesen. Persistenz ueber `CSave`
(`cs_confdel`/`cs_dblclick`/`cs_timefmt`/`cs_datefmt`) und
`cnx_put`/`cnx_get_prefs` in die `#E`-Zeile.

**Ausgefuehrte Pruefung.** Bau/Clippy/Hosttests siehe
[DESK02-STATUS-DONE.MD](DESK02-STATUS-DONE.MD). QEMU raspi4b: der Dialog misst
**(129, 143, 382, 212)** = `form_center`s Wert; `Date format → DMY` + `OK`
laesst den Schirm bytegleich zu dem vor dem Oeffnen; `Save desktop` schreibt
`#E 1A 86 00 00 60` (`0x86` = `BLITTER|DAYMONTH|24HCLOCK`, beide `IDT`-Bits
geloescht); nach einem **Kaltstart** von diesem Abbild steht `DMY` wieder
vorbelegt, und die Textansicht druckt `23-09-26` statt `09-23-26` — die
Einstellung erreicht also `format_fnode`, nicht nur den Dialog. `fsck_msdos`
ohne Beanstandung, null `[EXC]`/`[PANIC]`. Belege:
`measurements/measurement-2026-09-24-desk01-desk02/`.

**Offene Hardwareabnahme.** Pi 4, HW-Checkliste Punkt **37**, Schritte 3-7 —
die Doppelklickrate und die Loeschruecksprache brauchen eine echte Maus und
Tastatur, die 1920x1080-Optik den echten Schirm.
