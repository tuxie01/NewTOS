# MOD01 — Quellenanalyse: FreeMiNTs Modulsystem als Vorlage fuer nachladbare NewTOS-Module

Status: PARKED (2026-11-11) | Priority: P3 | Reason: owner decision 2026-11-11 at the legacy migration: not on the work queue | Reopen: a card program or port is affected, or the owner schedules it

Migrated (owner batch 11, 2026-11-11), evidence: TASK/MOD01.md status line: OFFEN, no DONE receipt, no fix/feat commit names the id

Line 3 before the migration:
Status: **OFFEN** · Prioritaet: **P3** · Typ: Quellenanalyse (Researcher) — Entwurfsgrundlage, kein Code

Abhaengigkeiten: —

## Warum

Treiber (Netz, spaeter VDI, Dateisysteme) sollen nachladbar sein. FreeMiNT
hat das Muster; sein Bootstrap („MINT.PRG im AUTO-Ordner") ist dagegen ein
Notbehelf fuer ROM-TOS und **entfaellt bei uns** — NewTOS ist der Kernel
(`kernel8.img` von der Firmware). Uebernehmenswert ist die Schicht darunter.

## Auftrag

Aus `references/freemint` (Pin `f906416`) extrahieren, je mit `datei:zeile`:

1. **Ladepfad**: `load_all_modules` (`sys/module.c:170-200`), `load_modules`,
   `load_module` (`:223`), Klassen `.xdd`/`.xfs` (`:161-169`), `.km`
   (`:605-680`), `.xif` ueber das Socket-Modul (`sys/sockets/inet4/ifload.c:92-118`).
2. **Verzeichnisse**: `sysdir` = `\mint\<version>\` (`sys/init.c:265-274`),
   `mchdir` (maschinenabhaengig) — wie ermittelt, Reihenfolge.
3. **Der Vertrag**: Modul = Programmfile mit Basepage; Einsprung = Textbasis,
   aufgerufen mit `(struct kerinfo *, <klassenspezifisch>)`
   (`ifload.c:59-88`). `struct kerinfo` (`sys/mint/kerinfo.h:59+`) vollstaendig
   auflisten: Versionsfelder, `bios_tab`/`dos_tab`, Speicher, Debug,
   Medienwechsel, Callbacks — und wie ein Modul sich registriert
   (`netinfo`, `if_register`).
4. **Klassenspezifische Strukturen**: `netinfo` (`.xif`), `DEVDRV` (`.xdd`),
   `FILESYS` (`.xfs`) — Felder und wer sie ausfuellt.
5. **Versionierung/Ablehnung**: wie ein Modul einen inkompatiblen Kernel
   erkennt und umgekehrt.

## Zusaetzlich: was NewTOS anders entscheiden muss (benennen, nicht entscheiden)

- **Privileg**: MiNT-Module laufen im Supervisor ohne MMU; unsere laufen auf
  EL1 mit Vollzugriff — geladener Fremdcode ist vertrauenswuerdiger Kernelcode.
- **Abbildung**: unser Lader legt Programme ins EL0-Fenster (`usermem`);
  ein Kernmodul braucht RX im Kernelbereich.
- **IRQ und DMA**: ein Netztreiber braucht Interrupt-Registrierung und
  kohaerenten Speicher — beides gibt es heute nicht (`kernel/src/arch/aarch64/`).
- **Symbolform**: MiNT ruft die Textbasis; wir haben ELF und koennten einen
  benannten Einsprung (`nt_module_init`) oder einen Header am Dateianfang
  verlangen.
- **Ort**: `C:\GEMSYS\` legen wir bereits an (`kernel/src/shell.rs`,
  `tools/make-sd-image.sh`) — Versionsunterverzeichnis wie MiNT?

## Abnahme

`docs/analysis/mod01-freemint-module-system-*.md`: Ladepfad, Vertrag,
Strukturen, Versionierung, dazu die NewTOS-Entscheidungspunkte als Liste
fuer den Lead. Keine Codeaenderung, keine ABI-Festlegung.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten. **Reihenfolge-Entscheid des Anwenders:**
NET01–NET05 werden **statisch** gebaut; die Modul-ABI wird erst nach dem
ersten echten Treiber festgelegt (MiNTs `kerinfo` ist genau so gewachsen).
