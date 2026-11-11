# DESK20 — `Install application` vergibt keine Funktionstaste (`APFUNKEY`)

Status: DONE (2026-11-11) | Priority: P4 | Commit: b402747df | Receipt: DESK20-STATUS-DONE.MD

Migrated (owner batch 19, 2026-11-11, to DONE), evidence: receipt TASK/DESK20-STATUS-DONE.MD exists, commit b402747df (feat(desk): dropping a file on a program launches it, Install applicat), VERIFIED: b402747df (feat(desk): dropping a file on a program launches it, Instal) names the id by body, its message names the ticket-title identifier apfunkey, touches code (18 files), legacy line: ERLEDIGT (2026-09-23)

Line 3 before the migration:
Status: **ERLEDIGT** (2026-09-23) · Prioritaet: **P4** · Typ: Funktionsluecke — Nebenbefund DESK05

Abhaengigkeiten: [DESK05](DESK05.md) (erledigt)

## Sachverhalt

EmuTOS' `ADINSAPP` hat ein Funktionstastenfeld (`APFUNKEY`,
`references/emutos/desk/desk_rsc.h:119`, Baum `desk_rsc.c:1059`), das
`install_application` liest und schreibt (`deskins.c:410`, `:477`, `:523`) —
so bekommt eine installierte Anwendung ihre Funktionstaste.

NewTOS' `ins_app_dialog` hat das Feld nicht. `a_funkey` wird gefuehrt,
gespeichert und in `Desktop configuration` **angezeigt** (DESK05), aber es
gibt keinen Weg, eine Taste zu vergeben — der Anzeiger bleibt darum leer.

## Arbeit

Das Feld in `ins_app_dialog` nachziehen, samt Lesen/Schreiben wie in
`deskins.c`; danach zeigt der Anzeiger in `inf_conf` etwas.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

---

## Abschluss (2026-09-23)

Die Praemisse haelt. `ADINSAPP`s Objekt 5 ist `APFUNKEY` (`desk_rsc.h:119`,
`desk_rsc.c:1059-1063`), ein `EDITABLE | FL3DBAK` `G_FBOXTEXT` ueber
`desk_rs_tedinfo[19]` (Vorlage „Install as: F__", `te_pvalid` „9",
`te_txtlen` 3 — also **zwei** Ziffern, die NUL mitgezaehlt).
`ins_app_dialog` fuellt es vor (`deskins.c:474-477`), `get_funkey` liest und
prueft es (`:404-444`), `APINSTAL` speichert das Ergebnis (`:520-531`).

Gebaut in `kernel/src/desk/insapp.rs`: das Feld eine Zeile unter
*Document type*, wie im Baum; `atow` (`:380-393`) und `get_funkey` samt
beider Meldungen; `Outcome::Install` traegt den Schluessel.
`kernel/src/desk/inf.rs` nimmt ihn: `install_application(…, funkey, …)`
schreibt `InfAnode::funkey`, `installed_doctype` gibt ihn (als `Installed`)
fuer die Vorbelegung zurueck, und `funkey_owner`/`release_funkey` sind der
Rundlauf ueber die anderen `ANODE`s (`:430-441`) — in zwei Haelften, weil die
Rueckfrage dazwischen sitzt und ein Trap nicht unter dem `SESSION`-Schloss
laufen darf.

Gemessen (Karte, QEMU raspi4b, Screendumps):

  * `7` getippt, Install → `[d20] get_funkey(fkey="7", …) -> Some(7)`, und
    *Desktop configuration* zeigt `F7  C:\CMDLINE.NCP______` in der
    *Function keys*-Anzeige, die vorher **immer** leer war.
  * `99` getippt → `STINVKEY` („Invalid function key value.",
    `desk_rsc.c:2525`), danach ist das Feld geleert und der Dialog wartet auf
    einen neuen Versuch (`deskins.c:522-529`).
  * Eine zweite Anwendung verlangt `7` → `STDUPKEY` („Function key already
    assigned.|Do you want to reassign it?", `:2526-2527`); nach OK steht
    `F7  C:\HELLO2.NCP` und die erste hat ihn verloren
    (`an->a_funkey = 0`, `:438`).

**Was DESK20 NICHT baut.** Ein Druck auf F7 startet weiterhin nichts:
`process_funkey` (`deskmain.c:1087-1106`) hat keine Entsprechung. Der
Vermerk in `main.rs::check_function_key` sagt das jetzt so, statt zu
behaupten, es koenne gar keine Anwendung eine Taste tragen. Als Folgeticket
gemeldet, hier nicht gemacht.

**Benannte Abweichung.** Die beiden Meldungen stehen woertlich als
Ressourcentext im Modul (`[  OK  ]` wie in `desk_rsc.c`), nicht ueber
`dir::fun_alert`, dessen Rahmen `[ OK ]` schreibt — ein alter, eigener
Unterschied, der hier nicht mitgeschleppt wird.

Messung: [`measurements/measurement-2026-09-23-desk19-desk20/`](../measurements/measurement-2026-09-23-desk19-desk20/).
Empfang: [`DESK20-STATUS-DONE.MD`](DESK20-STATUS-DONE.MD).
