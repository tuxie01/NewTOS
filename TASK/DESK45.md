# DESK45 — Die vier Selbsttests in `kernel/src/desk/` haengen in keinem Startpfad

Status: PARKED (2026-11-11) | Priority: P3 | Reason: legacy, evidence unverified; no receipt | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 15, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: no cited file of the ticket/receipt is touched by 87436d523 (test(desk43): U10 tester run on 22e01597 - baselin), named by status-line-named

Line 3 before the migration:
Status: **ERLEDIGT als Form A** (DESK43 U8a `09eaca14`: Feature `selftest-desk`, ein Kommando, Gegenprobe Karte unberuehrt; Tester-Lauf nach U10 `87436d52`: 64 PASS / 1 FAIL vorbestehend); offen/optional: Form B (EL0-Testprogramm) · Prioritaet: **P3** · Typ: Verrottungsrisiko — aus DESK43 U4, 2026-09-26

Abhaengigkeiten: —

## Der Befund

Vier `pub fn selftest()` liegen im Desktop und werden von **niemandem**
gerufen:

| Funktion | Was sie prueft | Was sie veraendert |
|---|---|---|
| `kernel/src/desk/dir.rs:2557` | Dateioperationen (`Dcreate`, Kopie, Umbenennen, rekursives Loeschen) gegen ein lebendes Laufwerk | **die SD-Karte** (`C:\DESKT9\`) |
| `kernel/src/desk/inf.rs:4381` | `NEWDESK.INF` Save→Read-Rundlauf, im Speicher und ueber die Platte | **die SD-Karte** (`C:\NEWDESK.INF`) |
| `kernel/src/desk/inf.rs:4491` (`selftest_install`) | Install-Application-Zuordnung (`#G`/`#P`-Zeilen) | **die SD-Karte** |
| `kernel/src/desk/ins.rs:1130` | `ins_devices`/`rmv_icon` am Modell | nur `G.g_ahead`, stellt den Zustand wieder her |

`/usr/bin/grep -rn 'selftest()' kernel/src tools` findet ausserhalb von
`kernel/src/desk/` **null** Treffer — es gibt keinen Aufrufer. Sie sind uebersetzt (kein `#[cfg(test)]`), also faengt der Compiler
Signaturbrueche — aber **kein Lauf** faengt Verhaltensbrueche.
[[unwired-selftests-rot-silently]]: genau dieser Zustand hat schon einmal eine
ABI-Aenderung durchgelassen.

## Warum DESK43 U4 sie nicht verdrahtet hat

1. **Drei von vier schreiben auf die Karte.** Ein Aufruf im normalen Bootpfad
   wuerde bei jedem Start Benutzerdaten anfassen. Das Repo hat dafuer die
   Konvention „nicht im Boot, bei Bedarf in `lib.rs` einschalten"
   (`kernel/src/lib.rs:373-382`) bzw. ein eigenes Cargo-Feature
   (`#[cfg(feature = "selftest-emmc")]`, `lib.rs:323`).
2. **Ihre Ausgabe haengt am selben Block wie der Rest von U4**: sie drucken
   ueber `crate::kprintln`, und der Ersatz `Cconws` steht dem Desktop heute
   nicht offen (zwei Gruende, im Kopf von `kernel/src/desk/mod.rs` belegt).
   Wer sie verdrahtet, entscheidet zugleich, wohin ihre Zeilen gehen.

## Auftrag (eine der beiden Formen, nicht beide)

* **A — Feature-Schalter.** `selftest-desk` in `kernel/Cargo.toml`, ein
  `#[cfg(feature = "selftest-desk")]`-Block in `kernel/src/lib.rs` neben dem
  `selftest-emmc`-Aufruf, der die vier in fester Reihenfolge fahrt, und eine
  Zeile in der Testanleitung. Billig, heute machbar, laeuft aber nur, wenn ihn
  jemand einschaltet.
* **B — EL0-Testprogramm.** Der Weg, den der Plan fuer U8 ohnehin vorsieht
  (`docs/analysis/desktop-userspace-plan-v2.md` §5, U8: *„Die vier Selftests
  werden ein EL0-Testprogramm (Vorbild `sdk/apps/apitest`)"*). Dann ist die
  Ausgabefrage von selbst geloest — ein EL0-Programm darf `Cconws` — und der
  Lauf ist Teil der Testrunde. Teurer, aber der Endzustand.

**Wenn B, dann gehoert dieses Ticket an U8 gehaengt und A entfaellt.** Wenn der
Termin dazwischenkommt, ist A der Platzhalter, der verhindert, dass die vier
beim Umzug still verschwinden.

## Abnahme

Die vier Funktionen laufen in **einem** benannten, wiederholbaren Kommando und
ihre PASS/FAIL-Zeilen stehen in einem Messordner. Gegenprobe: der normale Boot
fasst die Karte nicht an.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.
