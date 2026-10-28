# TOOL19 — `DISKUNIT.TXT`: Inhalt im RUNINFO, Abweisung bei fehlender oder falscher Einheit

Status: **OFFEN** · Prioritaet: **P2** · Typ: Developer (Werkzeug) — Anwenderentscheidung 2026-10-28, aus [TOOL07](TOOL07.md)

## Sachverhalt

- TOOL07 loest das Einheiten-Token `u:0` ueber `D:\DISKUNIT.TXT` auf, die die
  Profildefinition schreibt (`tools/hatari-oracle.sh:150@DISKUNIT`: 18;
  `tools/xtask/src/tosdump.rs:974@DISKUNIT`: 26). DUMPBIOS laeuft eine Tabelle
  mit `u:0` nicht, wenn die Datei fehlt (`tosdump/core/dumpbios.c:51@unit_path`).
- Anwender 2026-10-28: in Ordnung, **wenn** (a) die Datei immer aus der
  Profildefinition erzeugt wird, nie von Hand; (b) ihr Inhalt im RUNINFO steht;
  (c) der Runner abweist, wenn sie fehlt oder nicht zum Profil passt — kein
  Standardwert.

## Arbeit

1. Pruefen und belegen, dass (a) gilt (keine handgeschriebene Kopie in
   `measurements/`-Methoden oder Tabellen).
2. (b): die Einheit steht im RUNINFO bzw. Manifest beider Seiten (Orakel-
   Skript und `cargo xtask tosdump`) und im LOG des Dumpers.
3. (c): `cargo xtask tosdump` und `tools/hatari-oracle.sh` weisen eine Tabelle
   mit `u:0` ab, wenn die Datei fehlt oder ihr Wert nicht der Einheit des
   Profils entspricht; tosdiff vergleicht nur Laeufe, deren aufgezeichnete
   Einheit zum jeweiligen Profil passt. Kein Standardwert.
4. Tests fuer die Abweisungen.

## Abnahme

- Abweisungen gemessen (fehlend, falscher Wert); Hosttests gruen
  (`cargo xtask test`, `make -C tosdump host-test`); doc-check 0 Fehler.
