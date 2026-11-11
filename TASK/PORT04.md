# PORT04 — mintlib und gemlib gegen ihren gepflegten Stand halten

Status: PARKED (2026-11-11) | Priority: P3 | Reason: owner decision 2026-11-11 at the legacy migration: not on the work queue | Reopen: a card program or port is affected, or the owner schedules it

Migrated (owner batch 11, 2026-11-11), evidence: TASK/PORT04.md status line: OFFEN, no DONE receipt, no fix/feat commit names the id

Line 3 before the migration:
Status: **OFFEN** · Prioritaet: **P3** · Typ: Quellenpflege (Researcher, dann ggf. Developer)

Abhaengigkeiten: [PORT03](PORT03.md) (dort ist das Verfahren durchgespielt)

## Warum

PORT03 hat an cflib gemessen, was ein eingefrorener Fremdstand kostet: der
Fehler, an dem QED eine gezogene Datei als `README.TX` oeffnete, war in der
Bibliothek seit 2024-06-12 behoben. Gefunden wurde er bei uns 2026-09.

`sdk/libc` (mintlib 0.60.1) und `sdk/gemlib` (gemlib 0.44.0) stammen aus
derselben Ecke und aus derselben Zeit und sind **nie** gegen ihren gepflegten
Stand gehalten worden. Ob dort dasselbe liegt, ist unbekannt — nicht
vermutet, sondern ungemessen.

Dazu kommt ein Mechanismus-Loch, das PORT03 nur fuer cflib geschlossen hat:
ein neuer Tarball **kommt bei mintlib und gemlib gar nicht an**. `build-*.sh`
extrahiert nur bei fehlendem Baum, `sdk/lib-stale.sh` beobachtet ihre
Tarballs nicht. Ein Pin-Wechsel dort waere heute still wirkungslos.

## Auftrag

1. **Messen, nicht schaetzen.** Fuer mintlib und gemlib je: woher stammt der
   eingelagerte Tarball (Hash gegen `/Volumes/Daten/tho-otto-backu` und gegen
   `freemint/mintlib` bzw. `freemint/gemlib`), welchem Commit entspricht er,
   wieviele Commits liegen seither.
2. **Auf unsere Uebersetzung eingrenzen.** Von den seitherigen Commits nur
   die behalten, die eine von uns uebersetzte Datei beruehren. Je Datei:
   Commit, Datum, was sich aendert. Das ist die Liste, ueber die entschieden
   wird — nicht die Gesamtzahl.
3. **Overlays pruefen.** Jedes Voll-Datei-Overlay unter `sdk/libc/overlay/`
   und `sdk/gemlib/overlay/` gegen den neuen Stand halten: was hat
   stromaufwaerts in genau dieser Datei zugelegt.
4. **Die Paarung nachziehen.** `build-mintlib.sh`/`build-gemlib.sh`
   extrahieren neu, wenn der Tarball juenger als der Baum ist; `lib-stale.sh`
   beobachtet beide Tarballs per Glob — genau wie cflib seit PORT03.

## Abnahme

* Zwei Tabellen (mintlib, gemlib) mit dem gemessenen Abstand und der auf
  uebersetzte Dateien eingegrenzten Aenderungsliste.
* Entscheidung je Bibliothek — pinnen oder begruendet stehen lassen —, mit
  der Liste als Beleg.
* Wird gepinnt: Basislinien unveraendert (`cargo run -p xtask -- test`,
  `sdk/run-host-tests.sh`), QED und TeraDesk bauen und kommen hoch,
  `expected-failures.txt` unveraendert oder je Eintrag begruendet.
* `docs/sdk/09-port-material.md` §3 fortgeschrieben.
* Wird nicht gepinnt: Punkt 4 (die Paarung) **trotzdem** erledigt.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.
