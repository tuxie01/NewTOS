# DESK12 — Baudatum: es gibt keine Konstante, der About-Kasten kann es nicht zeigen

Status: PARKED (2026-11-11) | Priority: P3 | Reason: legacy, evidence unverified; receipt DESK12-STATUS-DONE.MD exists | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 18, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: no cited file of the ticket/receipt is touched by 8a32d0302 (feat(tos-api): stamp the build, and give the OS he), named by body

Line 3 before the migration:
Status: **ERLEDIGT (2026-09-23)** · Prioritaet: **P3** · Typ: Werkzeugkette — Nebenbefund DESK01

Abhaengigkeiten: [DESK01](DESK01.md) (erledigt)

## Sachverhalt

Der About-Kasten (`kernel/src/desk/inf.rs` `do_aboutdesk`) zeigt Name,
Version, Maschine und den Unabhaengigkeitshinweis, **kein Baudatum** — es
gibt im ganzen Baum keine Konstante dafuer. `os_date` ist ein ausdruecklicher
1980-01-01-Platzhalter (`tos-api/src/bdos/process/loader.rs:806-809`), die
`Boot time`-Zeile des Startschirms ist die GEMDOS-Uhr. Eine davon als
Baudatum zu drucken waere unwahr.

EmuTOS hat das Datum im BIOS-`version[]` (`deskmain.c:1881` zeigt `DEVERSN`
darauf).

## Arbeit

Eine Baukonstante erzeugen (`kernel/build.rs`, z. B. Datum + kurzer
Git-Hash), mit Blick auf Reproduzierbarkeit (`SOURCE_DATE_EPOCH`
beruecksichtigen); `initinfo.rs` und der About-Kasten lesen sie aus derselben
Quelle wie Version und Hinweis. Danach die Zeile im Kasten und ggf. im
Startschirm.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

## Abschluss

**ERLEDIGT (2026-09-23).** Quittung: [DESK12-STATUS-DONE.MD](DESK12-STATUS-DONE.MD),
Messung: `measurements/measurement-2026-09-23-desk12/`.

**Die Quelle hat den Auftrag korrigiert.** Das Ticket nahm an, der
About-Kasten solle das Baudatum zeigen. EmuTOS zeigt dort **keins**:
`ADDINFO` traegt `DECOPYRT`, und das ist ein *Jahr* (`COPYRIGHT_YEAR`,
`deskmain.c:1892`). Das Baudatum geht bei EmuTOS **allein in den
OS-Kopf** — `tools/mkheader.awk` erzeugt `obj/header.h` mit `OS_DATE`
(BCD `MMDDYYYY`, `0x08292026`) und `OS_DOSDATE` (GEMDOS-gepackt, `23837`);
der Startschirm druckt nur `EmuTOS Version %s` (`bios/initinfo.c:321`).

Damit war die eigentliche Luecke nicht der Kasten, sondern `os_date`/
`os_dosdate`: beide standen als 1980-01-01-Platzhalter in
`tos-api/src/bdos/process/loader.rs` und sind Teil des OSHEADER, den
Programme lesen.

**Gebaut:**

* `tos-api/build.rs` (neu) — NewTOS' Gegenstueck zu `mkheader.awk`. Gibt
  `NEWTOS_BUILD_DATE` (ISO) und `NEWTOS_BUILD_REV` (kurzer Commit) aus.
  `SOURCE_DATE_EPOCH` schlaegt die Wanduhr, damit ein reproduzierbarer Bau
  reproduzierbar bleibt; ohne sie ist es der Lauf des Skripts, den ein neuer
  Commit ueber `.git/HEAD` erzwingt. Datumsrechnung: Hinnants
  `civil_from_days`, ohne Fremdkiste.
* `tos-api/src/build.rs` (neu) — packt daraus `OS_DATE` und `OS_DOSDATE` zur
  Uebersetzungszeit und exportiert `BUILD_DATE`/`BUILD_REV`/`BUILD_STAMP`/
  `BUILD_YEAR`. Zwei Hosttests: einer prueft die Packung gegen den einzigen
  erzeugten Kopf, den wir haben (`obj/header.h`: `0x08292026` / `23837`), der
  andere, dass dieser Bau wirklich gestempelt ist.
* `loader.rs` liest beide Werte von dort; die Platzhalterzeile ist weg.
* `initinfo.rs` zeigt `Built : <Datum> (<Commit>)` — **eigene Zugabe**, im
  Kommentar so benannt: EmuTOS' Startschirm druckt kein Datum.

**Warum die Konstante in `tos-api` liegt und nicht in `kernel`:** beide
brauchen sie, und die Umgebung eines Bauskripts erreicht nur die eigene
Kiste. Eine Quelle, zwei Leser — dieselbe Anordnung wie bei EmuTOS, wo ein
`obj/header.h` entsteht und alle davon lesen.

**Der About-Kasten bleibt ohne Datumszeile**, jetzt mit dem richtigen Grund
im Quelltext: EmuTOS hat dort keine, und der Kasten ist bei 640x480 mit
436 px schon 10 px unter der Grenze — eine Zeile mehr waeren 20. Die
Jahreszahl in der EmuTOS-Kreditzeile bleibt ebenfalls ein Literal: EmuTOS'
Jahr kommt aus *dessen* Bau, unsere Quelle ist ein festgenagelter Stand, und
unser Baujahr dort einzusetzen wuerde deren Urheberrecht per Kalender
verlaengern.

**Geprueft:** Hostsuite **934/0/1** (932 + 2 neue), beide Bretter frisch
gebaut tos-api 64 / kernel 94 (Delta 0), QEMU raspi4b-Startschirm mit der
Zeile `Built : 2026-09-23 (786cd80a)`, und `SOURCE_DATE_EPOCH=1700000000`
ergibt nachweislich `NEWTOS_BUILD_DATE=2023-11-14`.
