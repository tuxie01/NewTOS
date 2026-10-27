# TOOL04 — Testlaufzeit: schnelle Stufe fuer jede Aenderung, volle Stufe fuer Meilensteine

Status: **OFFEN** · Prioritaet: **P2** · Typ: Werkzeug (Testbarkeit) — Anwenderentscheidung 2026-10-27

## Sachverhalt

- Ein voller `cargo xtask test` (tos-api, tos-types, xtask; 163 Testbinaries,
  rund 1700 Tests) dauerte am 2026-10-27 auf dem Entwicklungsrechner etwa
  2,5 h. Damit laeuft er selten.
- Beobachtung der Worker (nicht gemessen): jedes Testbinary haengt beim Start
  rund 60 s in `_dyld_start`, vermutlich eine macOS-Pruefung frisch
  gebauter Programme; 163 × 60 s ≈ 2,7 h. Die Tests selbst melden
  Laufzeiten unter einer Sekunde (`finished in 0.00s`).
- Eine Suche nach `thread::sleep`/`Duration::from_secs` in `tos-api/tests`
  und `tools/xtask/src` fand nichts; `qemu` kommt dort nur in Kommentaren
  vor. `cargo-nextest` ist nicht installiert.
- Anwenderentscheidung 2026-10-27: „Measure per-test durations (cargo nextest
  shows the slow ones), split into a fast tier (< ~10 min, every change) and
  a full tier (milestones/nightly). Look for fixed sleeps/timeouts in
  QEMU-based tests first.“

## Vorbefund des Leads (2026-10-27, waehrend eines laufenden Testlaufs)

- Ein Testbinary, das in diesem Lauf schon einmal lief
  (`target/debug/deps/tos_api-*`), startet zweimal hintereinander in 0,12 s
  und 0,10 s. Eine Kopie davon (gleiche Codesignatur) 0,25 s / 0,10 s, ein
  frisch mit clang gebautes Mini-Programm 0,13 s / 0,00 s.
- Das gerade vom Lauf gestartete Binary
  (`aes_t04_intin_long_contract-*`, 1,9 MB) stand 40 s lang bei 0 % CPU im
  Zustand `S`; gleichzeitig lief `XprotectService` mit 10,6 % CPU als
  zweitgroesster Verbraucher.
- Lesart (Hypothese, nicht bewiesen): der erste Start jedes **neu gelinkten**
  Binarys wartet auf einen XProtect-Scan; die Tests selbst sind schnell.
  Warum das Mini-Programm nicht wartet, klaert die Messung (Groesse,
  Signatur des Linkers, Pfad unter `target/`).

## Anwenderhinweise 2026-10-27

- Schnelldiagnose: dasselbe Testbinary zweimal starten; ist der zweite Start
  schnell, ist es der Erstscan neuer Binaries (Gatekeeper/XProtect oder ein
  EDR), nicht die Tests.
- Abhilfen fuer den Bericht:
  1. **Strukturell, ohne Sicherheitsaenderung:** die Integrationstests zu
     wenigen Binaries zusammenfassen (ein `tests/`-Crate mit Modulen statt
     163 `tests/*.rs`). Spart die Kosten je Binary und Linkzeit.
  2. **Konfiguration, nur Anwenderentscheidung:** Terminal/VS Code, die
     cargo ausfuehren, unter Datenschutz & Sicherheit → Entwicklertools
     zulassen (`spctl developer-mode enable-terminal`); bei einem EDR eine
     Ausnahme fuer `target/`. Abwaegung berichten, nicht anwenden.
- Groesse variieren (Anwender 2026-10-27): dasselbe Testbinary frisch
  gebaut mit voller Debuginfo gegen `[profile.test] debug =
  "line-tables-only"` (oder gestrippt), je erster Start gemessen. Waechst die
  Wartezeit mit der Groesse, sind kleinere Testbinaries eine **dritte
  Abhilfe ohne Sicherheitsaenderung**, neben dem Zusammenfassen der
  Integrationstests.

## Arbeit

1. **Messen, bevor etwas geaendert wird:** Laufzeit je Testbinary und je
   Test (`cargo nextest` oder gleichwertig), getrennt nach Startzeit des
   Binarys und Laufzeit der Tests. Feste Sleeps und Timeouts suchen, zuerst
   in QEMU-gestuetzten Tests (auch in `tools/`- und `measurements/`-Skripten,
   die Tests treiben), dann im Rest.
2. **Ursache des Start-Haengers** belegen (z. B. `log stream` /
   `syspolicyd`, Codesignatur, Spotlight/XProtect-Scan von `target/`) und,
   wenn sie am Host liegt, die Abhilfe beschreiben und messen. Eingriffe in
   Sicherheitseinstellungen des Hosts schlaegt der Bericht vor, fuehrt sie
   aber nicht aus.
3. **Zwei Stufen:** eine schnelle Stufe (Ziel < ~10 min, nach jeder Aenderung)
   und eine volle Stufe (Meilensteine, naechtlich). Auswahl begruendet aus
   den Messwerten; kein Test verschwindet, jeder ist in genau einer Stufe.
   Aufruf z. B. `cargo xtask test --fast` / `cargo xtask test` (oder
   `--full`); die schnelle Stufe nennt am Ende, was sie ausgelassen hat.
4. Falls viele kleine Testbinaries die Startkosten treiben: pruefen, ob sie
   sich ohne Verlust an Isolation zu weniger Binaries zusammenfassen lassen
   (Messwert vorher/nachher).
5. Doku: CLAUDE.md-Abschnitt „Testing“ und die Rollen-Dateien
   (`.claude/agents/developer.md`, `tester.md`) sagen, welche Stufe wann
   laeuft.

## Abnahme

- Messbericht mit den langsamsten Binaries/Tests und der Ursache des
  Start-Haengers.
- Schnelle Stufe < ~10 min auf diesem Host, volle Stufe weiter vollstaendig;
  beide gruen.
- doc-check der geaenderten Doku 0 Fehler.
