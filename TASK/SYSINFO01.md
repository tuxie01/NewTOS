# SYSINFO01 — SYSACC wird ein kleines Systeminformations-Werkzeug (Version 1)

Status: DONE (2026-11-11) | Priority: P2 | Commit: 8521acc2d | Receipt: SYSINFO01-STATUS-DONE.MD

Migrated (owner batch 5, 2026-11-11), evidence: receipt TASK/SYSINFO01-STATUS-DONE.MD exists, commit 8521acc2d, TASK/SYSINFO01-STATUS-DONE.MD, commit 8521acc2d (feat(sysacc): read-only System Info dialog, Ssystem 2003-2019 (SYSINFO), fix/feat commits naming the id: 8521acc2d (n=1), measurements/measurement-2026-10-07-acctitle/, the receipt names no hash for this commit, the Migrated line names it

Line 3 before the migration:
Status: **ERLEDIGT (QEMU), 2026-10-07** — [Quittung](SYSINFO01-STATUS-DONE.MD) · Prioritaet: **P2** · Typ: NewTOS-Erweiterung (Accessory + Ssystem 2000+) — Anwender 2026-10-07

Abhaengigkeiten: [ACC06](ACC06.md) (Ssystem 2000/2001, SYSACC als `form_alert`),
ACC02/ACC03 (Accessory-Geruest), ACCTITLE (Accessory-Titel lesbar), FS08/FS09
(SD-Modus, Block-Cache), SER01 (serielle Konsole), ARCH03 (memprot, ASIDs)

## Anforderungen des Anwenders (2026-10-07)

* **Keine** Anzeige von `cmdline.txt`/`config.txt` als Text — menschenlesbare
  Bezeichnungen und Werte, wie Schalter/Auswahlen, die ein Mensch versteht.
* Version 1 **zeigt nur**, kein Bearbeiten. Bearbeiten mit Bedienelementen
  kommt spaeter — der Dialog ist so anzulegen, dass die Werte spaeter
  auswaehlbare Bedienelemente werden koennen.
* Gezeigt wird der **wirksame** Zustand, mit dem der Kernel laeuft, nicht nur,
  was in einer Datei steht.

### Inhalt

* **System:** CPU-Takt (Rate, die die Firmware meldet — als angeforderte/
  aktuelle Rate bezeichnet — und die gemessene Rate, falls verfuegbar),
  SoC-Temperatur, Kernel-Build/Versionszeichenkette, Groesse von ST-RAM und
  TT-RAM.
* **Speicherschutz:** Aus / Ein / Streng (`memprot=`); Adressraum-IDs:
  Aus / Ein / Test (`asid=`, `asid=tiny`).
* **SD-Karte:** Schneller Modus (4-Bit-Bus + Mehrblock) / Kompatibilitaetsmodus
  (`emmc=compat`) und die tatsaechlich ausgehandelte Busbreite; Block-Cache:
  Groesse in MB (oder Aus) und Anteil je Lesevorgang (`bcache=`, `bcachepct=`).
* **CPU-Taktpolitik:** Firmware-gesteuert / Fest auf Maximum (`cpuclk=`).
* **Anzeige:** wo Kernel/Firmware es wissen — bevorzugt der tatsaechliche
  Framebuffer-Modus aus dem Kernel: Aufloesung und Farbtiefe; GPU-Speicher,
  falls per Mailbox erhaeltlich.
* **Tastaturbelegung:** Deutsch / US / aus `KEYBOARD.KBD`.
* **Serielle Konsole:** Ein / Aus.

### Mechanismus

Das Accessory laeuft in EL0 und muss die Werte vom Kernel bekommen: neue
NewTOS-`Ssystem`-Unterfunktionen im Bereich 2000+ neben `S_NEWTOS_CPUCLOCK`
(2000) / `S_NEWTOS_CPUTEMP` (2001). **Spezifikation zuerst**
(`docs/api/gemdos/154-ssystem.md`: Nummer, Argumente, Rueckgabewerte, Fehler
fuer Unbekanntes je Unterfunktion), Host-Grenze wie bei den bestehenden.
Keine Stubs: jeder Wert ist echt oder ein ehrliches „unbekannt".

### Oberflaeche

Ein richtiger GEM-Dialog (Objektbaum im Code oder `.rsc` ueber
`tools/mkrsc.py` — was zu den SDK-Konventionen passt), 3D-Optik wie der Rest
der AES, geoeffnet ueber den Desk-Menueeintrag (kurz halten), OK schliesst.
Beschriftung englisch wie der Rest der NewTOS-Oberflaeche. Passt in 640x480.
Accessory-Regeln: endet nie, `AC_OPEN`/`AC_CLOSE` (`docs/api/aes/accessories.md`).

## Pruefung

* Host-Tests fuer die neuen Ssystem-Unterfunktionen.
* QEMU raspi4b (`-append serial=on ...`) Screendumps des Dialogs mit
  Standardargumenten und mit z. B. `memprot=strict bcache=0 emmc=compat
  cpuclk=max` (Werte aendern sich sichtbar); oeffnen aus EmuDESK, TeraDesk und
  QED (Desk-Menue) — Screendumps genau ansehen.
* Regression: APITEST, der Lauf des Accessory-Titel-Fixes
  (`measurements/measurement-2026-10-07-acctitle/method`), LDGDEMO kurz.
* Statisch: Host-Tests (1227/0/1), Release-Builds beider Bretter (64/94),
  Clippy (76/125), Drift (31), SDK-Host-Tests (18/0/0).

## Kartenpaket (nicht kopiert)

Neues `SYSACC.NAC` + Release-Kernel rpi4 unter
`measurements/measurement-2026-10-07-sysinfo/bin/` mit sha256 und kurzer
deutscher `LIESMICH.TXT`. Laufverzeichnis
`measurements/measurement-2026-10-07-sysinfo/` (RUNINFO.TXT, method/, serial/,
screendumps/, checks/). Quittung `TASK/SYSINFO01-STATUS-DONE.MD`.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten. Keine erfundene Hardware-Semantik
(Mailbox-Tags aus `references/rpi4-boot/` bzw. `docs/hardware/`).
