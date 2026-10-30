# TOOL01 — `cargo xtask doctor`: Hostwerkzeuge gegen gepinnte Erwartungen pruefen

Status: **GEPARKT** 2026-10-31 (Anwender, Vor-Start-Pruefung: blockiert keine Abnahme) · Prioritaet: **P2** · Typ: Werkzeug (Reproduzierbarkeit) — Anwenderentscheidung 2026-10-27

## Sachverhalt

Ob ein Messlauf oder ein Abbild reproduzierbar ist, haengt an Hostwerkzeugen,
die heute niemand vorab prueft:

- zView `SAMPLES` fehlte im Abbild, weil `gen-samples.py` ohne Pillow lief
  (`sdk/zview/build-zview.sh` ruft `sdk/zview/samples/gen-samples.py`); das
  Abbild meldete es nur als `optional_missing` (`tools/make-sd-image.sh`).
- Pillow wurde am 2026-10-27 per `pip install --user` in die Homebrew-Python
  3.15 gelegt (`~/Library/Python/3.15`); `brew install pillow` liess sich nur
  fuer 3.14 einhaengen. Ein Homebrew-Python-Upgrade wirft das still weg.
- `ld.lld`, `llvm-nm` und `gtimeout` liegen nicht auf `PATH` (Stand
  2026-10-27); die SDK-Skripte finden ihre Werkzeuge je auf eigene Weise.
- Der Orakelpin (`tools/oracle/oracle.env`) nennt Hatari, Kreuzcompiler und
  ROM mit sha256; geprueft wird er nur von `tools/hatari-oracle.sh` beim Lauf.

## Arbeit

1. **`cargo xtask doctor`** prueft Vorhandensein und Version gegen gepinnte
   Erwartungen, Pfade wo moeglich aus `tools/oracle/oracle.env`:
   - Hatari 2.6.1 (`ORACLE_HATARI`);
   - `m68k-atari-mint-gcc` 7.5.0 (`ORACLE_CROSS_MINT`);
   - `clang`, `ld.lld`, `llvm-nm` — so aufgeloest, wie die Bauskripte sie
     aufloesen (die Skripte lesen, nicht raten), Version gepinnt;
   - QEMU (`qemu-system-aarch64`), `cflow`, `gtimeout`, `fsck_msdos`;
   - gepinntes ROM vorhanden und sha256 == `ORACLE_ROM_SHA256`.
   Wo es heute keine gepinnte Version gibt: die gemessene Version als
   Erwartung festschreiben (eine Stelle, wie `oracle.env`) und das im Bericht
   nennen.
2. **Python-Werkzeuge in einer Projekt-venv** statt User-Site: z. B.
   `tools/.venv` (gitignored) plus eine requirements.txt unter `tools/` mit Pillow (Version
   gepinnt). `doctor` prueft die venv und die Pakete gegen die Datei; ein
   Erzeugungsbefehl steht in der Fehlermeldung.
   `tools/make-sd-image.sh` und die von ihm gerufenen Python-Schritte
   (`mk-hello-ncp.py`, `sdk/zview/build-zview.sh` → `gen-samples.py`, das
   eingebettete Python am Ende) laufen mit dieser venv.
3. **Exit 0 nur, wenn alles passt**; jede Abweichung als eine Zeile mit
   Werkzeug, erwartet, gefunden.
4. **Aufrufer:** `tools/make-sd-image.sh` ruft `doctor` vor dem Bau und bricht
   bei exit != 0 ab; der Auditor ruft es als Pre-flight
   (`.claude/agents/auditor.md`). Sollte ein Aufrufer nur einen Teil brauchen
   (das Abbild braucht kein Hatari), eine Teilpruefung per Option — aber kein
   stilles Ueberspringen.
5. Hosttest fuer die Vergleichslogik (Versionsauswertung, sha256, fehlendes
   Werkzeug), ohne die echten Werkzeuge vorauszusetzen.
6. Doku: `docs/sdk/06-image-and-testing.md` (Bauvoraussetzungen) und
   `docs/analysis/ORACLE.md` (Pre-flight) verweisen auf `doctor`.

## Abnahme

- `cargo xtask doctor` exit 0 auf diesem Host; mit absichtlich falschem
  ROM-sha256 bzw. fehlender venv exit != 0 mit benannter Ursache.
- `tools/make-sd-image.sh` baut mit der venv, zView `SAMPLES` ist im Abbild,
  `optional_missing` im Manifest lautet `none`.
- `cargo xtask test` gruen, doc-check der geaenderten Doku 0 Fehler.
