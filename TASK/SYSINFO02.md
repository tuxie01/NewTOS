# SYSINFO02 — System Info, Version 2: Starteinstellungen auswaehlen und speichern

Status: IMPLEMENTED (2026-11-11) | Priority: P2 | Commit: e78e43df7 | Missing: one open point (Esc, kernel AES)

Migrated (owner batch 15, 2026-11-11, to IMPLEMENTED), evidence: receipt TASK/SYSINFO02-STATUS-DONE.MD exists (legacy DONE receipt, IMPLEMENTED accepted by TOOL55), commit e78e43df7 (feat(sysacc): System Info edits the boot settings (SYSINFO02)), VERIFIED: diff of e78e43df7 (feat(sysacc): System Info edits the boot settings (SYSINFO02) touches sdk/examples/sysacc.c, sdk/examples/sysacc-cmdline.h, sdk/examples/tests/sysacc-cmdline-host.c (+1), names the id by subject, 123 files, legacy line: ERLEDIGT (QEMU), 2026-10-08, ein offener Punkt (Esc, Kernel-AES) — [Quittung](SY

Line 3 before the migration:
Status: **ERLEDIGT (QEMU), 2026-10-08, ein offener Punkt (Esc, Kernel-AES)** — [Quittung](SYSINFO02-STATUS-DONE.MD) · Prioritaet: **P2** · Typ: NewTOS-Erweiterung (Accessory, nur Werkzeugkette) — Anwender 2026-10-07/08

Abhaengigkeiten: [SYSINFO01](SYSINFO01.md) (Dialog, Ssystem 2003–2019,
[Quittung](SYSINFO01-STATUS-DONE.MD),
[Beleg](../measurements/measurement-2026-10-07-sysinfo/RUNINFO.TXT)),
[`docs/api/gemdos/154-ssystem.md`](../docs/api/gemdos/154-ssystem.md)
§„System-information modes (2003–2019)", `kernel/src/bootargs.rs`
(die Startargumente und ihre Voreinstellungen), INPUT01 (`input=aes`).

## Anforderungen des Anwenders (2026-10-07/08)

* Menschenlesbare Auswahlen, „Dropdown oder so etwas" — **nie** der rohe Text
  von `cmdline.txt`/`config.txt` (Gedaechtnisnotiz *settings-ui-human-readable*).
* Die Einstellungen aus `C:\cmdline.txt` werden auswaehlbar:

  | Zeile | Auswahl | Startargument (Voreinstellung fett) |
  |---|---|---|
  | Memory protection | Off / **On** / Strict | `memprot=off` / — / `memprot=strict` |
  | Address-space IDs | **Off** / On / Test | — / `asid=on` / `asid=tiny` |
  | SD card mode | **Fast** / Compatibility | — / `emmc=compat` |
  | CPU clock | **Fixed at maximum** / Firmware-controlled | — / `cpuclk=firmware` |
  | Block cache | Off / 8 / 16 / **32** / 64 MB | `bcache=0` / `bcache=8192` / … / — / `bcache=65536` |
  | Cache share | **5** / 10 / 25 / 50 % | — / `bcachepct=10` / … |
  | Mouse while busy | **On** / Off | — / `input=aes` |
  | Serial console | On / **Off** | `serial=on` / — |

  Je Zeile: der Wert, mit dem der Kernel **jetzt** laeuft (Ssystem), und der
  fuer den **naechsten Start** ausgewaehlte Wert.
* Auswahl: je Wert ein dropdown-artiges Aufklappfeld **ohne** AES
  `menu_popup` (laut Auftrag nicht implementiert, #59; im Baum inzwischen vorhanden, A59): Klick auf den Wert oeffnet eine
  kleine Auswahlbox (eigener Objektbaum, `objc_draw` + eigene kleine
  Verfolgungsschleife), 3D-Optik; Esc bricht ab. Alles im Accessory (nur
  Werkzeugkette, keine Kernel-Aenderung). Wenn eine AES-Aenderung im Kernel
  noetig waere: STOP und melden.
* Knoepfe **Save** (schreibt `C:\cmdline.txt`), **Cancel**, **OK**.
* Speichern: vorhandenes `cmdline.txt` lesen (darf fehlen); jedes Token, das
  das Werkzeug nicht verwaltet (Firmware-/Linux-Token, Unbekanntes), bleibt in
  seiner Reihenfolge erhalten; nur verwaltete Token werden ersetzt/entfernt.
  Vorher Sicherung `C:\cmdline.bak`; so atomar wie GEMDOS erlaubt (neue Datei
  schreiben, dann umbenennen); eine Zeile, durch Leerzeichen getrennt
  (`references/circle/doc/cmdline.txt:3-4`). Werte gleich der
  Kernel-Voreinstellung werden weggelassen. Deutlicher Hinweis
  „Changes take effect after restart". `config.txt` wird nie angefasst.
* Vor dem Speichern **eine** Bestaetigung (`form_alert`), die in Klartext
  auflistet, was sich aendert.

## Pruefung

* Host-Test der Token-Zusammenfuehrung (reine C-Funktion, fuer den Host
  uebersetzt, im SDK-Host-Test-Lauf): fremde Token bleiben, verwaltete werden
  ersetzt, Voreinstellungen entfallen, leere/fehlende Datei, CRLF/LF,
  Leerzeichen am Ende.
* QEMU raspi4b (`-append serial=on`): aus EmuDESK oeffnen, zwei Werte ueber
  die Aufklappfelder aendern, Save; `C:\cmdline.txt` und `cmdline.bak` auf dem
  Host pruefen; QEMU neu starten mit `-append` = gespeicherter Dateiinhalt
  (QEMU nimmt die Startargumente aus `-append`, nicht aus der Datei) — die
  Spalte „jetzt" zeigt die neuen Werte. Einmal aus TeraDesk/QED oeffnen.
  Screendumps genau ansehen.
* Regression: APITEST, Accessory-Titel-Lauf, LDGDEMO kurz.
* Statisch: SDK-Host-Tests (18/0/0 + neu); Rust-Host-Tests nur, wenn Rust
  angefasst wurde (soll nicht).

## Kartenpaket (nicht kopiert)

`SYSACC.NAC` mit sha256 + deutsche `LIESMICH.TXT` unter
`measurements/measurement-2026-10-08-sysinfo2/bin/`. Laufverzeichnis
`measurements/measurement-2026-10-08-sysinfo2/` (RUNINFO.TXT, method/,
serial/, screendumps/, checks/). Quittung `TASK/SYSINFO02-STATUS-DONE.MD`.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten. Keine Stubs; drei Versuche je Fehler.
