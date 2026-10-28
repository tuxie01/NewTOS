# TOOL11 — `PTR_RAM` laesst die Funktionsnummer als Zeiger durch

Status: **OFFEN** · Prioritaet: **P1** · Typ: Werkzeug (Vergleichsregel) — Befund des Leads 2026-10-28

## Sachverhalt

- DUMPBIOS gegen das Orakel: am 2026-10-27 47 (`tt`) / 51 (`falcontc`)
  abweichende Faelle (`measurements/measurement-2026-10-27-dumpbios/RUNINFO.TXT`),
  am 2026-10-28 42 / 46
  (`measurements/measurement-2026-10-28-bios-storage-audit/RUNINFO.TXT`).
- Die fuenf Faelle, die weggefallen sind, sind in beiden Profilen dieselben:
  98–100 (`Iorec(0..2)`), 109 (`Keytbl(-1,-1,-1)`), 184 (`Bconmap(-2)`), alle
  Klasse `PTR_RAM`. Kein Fall kam hinzu.
- Die NewTOS-Werte sind in beiden Laeufen gleich (je Lauf NEWTOS.BIN im cmp-Ordner der Laeufe,
  dekodiert): `Iorec` → `0x0E`, `Keytbl` → `0x768`, `Bconmap(-2)` → `0x2C`.
  Gekippt hat also kein NewTOS-Commit, sondern die Regel: `ff1da029`
  verlangt statt eines Vielfachen von 4 nur noch eine gerade Adresse
  (`tosdump/host/tosdiff.c:531@TD_RET_PTR_RAM`). Damit bestanden auch die
  Orakel-Werte 109/184 (`0x40AE`, `0x4A76`), die vorher die Regel selbst
  verletzten — beabsichtigt.
- Nebenwirkung: `0x0E` und `0x2C` sind die XBIOS-Funktionsnummern von
  `Iorec` und `Bconmap`, keine Zeiger. `Iorec` ist nicht verdrahtet und
  faellt in den Zweig, der die Funktionsnummer zurueckgibt
  (`tos-api/src/xbios/mod.rs`, Kommentar bei den Funktionsnummern);
  `Bconmap` gibt `0x2C` als „nicht verfuegbar“ zurueck
  (`docs/api/xbios/2c-bconmap.md`). Beide bestehen `PTR_RAM` jetzt (ungleich
  null, gerade, unter `0x80000000`). Die Regel verdeckt damit einen Befund,
  den der Lauf vom 2026-10-27 noch zeigte („`Iorec` fehlt“); ob `Bconmap(-2)`
  → `0x2C` eine akzeptierte Abweichung ist, entscheidet die Triage, nicht
  die Regel.
- Nach der Rolle ist eine Vergleichsregel, die falsch aussieht, ein Befund
  fuer den Researcher; tosdiff aendert der Developer.

## Arbeit

1. Researcher: Regelvorschlag, der am Orakel besteht (`tosdiff
   --selfcheck`, alle 18 gepinnten Referenzen) und eine Funktionsnummer
   oder sonst eine Adresse, hinter der keine Struktur liegt, nicht
   durchlaesst — z. B. Untergrenze, Ausschluss der Aufrufnummer, oder
   `capture` der Zielstruktur (Groesse und Felder je Fall) statt einer reinen
   Adresspruefung. Begruendung aus EmuTOS-Quelle und Referenzwerten.
   Ob `Keytbl` → `0x768` (Seite 0, fuer EL0 lesbar) eine gueltige Antwort
   ist, gehoert zur Begruendung.
2. Developer: Regel in tosdiff bzw. der Falltabelle umsetzen,
   `tosdump/tests/test_selfcheck.sh` erweitern um einen Fall, der die
   Funktionsnummer als Zeiger ablehnt.
3. DUMPBIOS-Vergleich neu: `bios-r1.tsv` gegen beide Profile, Ergebnis im
   RUNINFO; jede neu abweichende Zelle ist ein Befund fuer die Triage, kein
   Fix.

## Abnahme

- `tosdiff --selfcheck` besteht fuer alle gepinnten Referenzen.
- Die Faelle 98–100 und 184 weichen mit den Werten vom 2026-10-28 ab; jeder
  andere `PTR_RAM`-Fall ist im RUNINFO begruendet.
- Hosttests gruen (`cargo xtask test` und `tosdump/tests`); doc-check der
  geaenderten Doku 0 Fehler.

## Stand 2026-10-28 (Arbeitspunkt 1 erledigt)

- Regelvorschlag `docs/analysis/ptr-ram-rule-2026-10-28.md`, Empfehlung
  Option A: feste Untergrenze `v >= 0x400` (`sysvars_start`,
  `references/emutos/tosvars.ld:31`); kleinster Orakelwert `0x30D4`, NewTOS
  `Keytbl` `0x768` und `Setexc(0x168)` `0x5F0` bleiben gueltig; neu
  abweichend nur `Iorec(0..2)` und `Bconmap(-2)`. Erkennt nur kleine
  Konstanten.
- Spaeter fuer `Iorec`/`Bconmap`: Capture durch den Rueckgabezeiger
  (`r:<len>` aus TOOL07, Option B), sobald der LP64-Vertrag des IOREC-Abbilds
  geklaert ist.
- Developer-Schritt zusaetzlich: der Laeufer folgt dem Rueckgabezeiger ohne
  die PTR_RAM-Grenzen (`tosdump/core/runner_call.c:257@TD_CALL_CAP_RET`); er
  muss dieselben Grenzen einhalten, bevor `r:` fuer weitere Faelle genutzt
  wird.
