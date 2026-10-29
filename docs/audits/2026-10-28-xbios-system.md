# Audit: XBIOS-Systemaufrufe ohne Plattenzugriff (2026-10-28)

## 1. Kopf

| | |
|---|---|
| Scope | XBIOS `Random` (0x11), `Protobt` (0x12), `Cursconf` (0x15), `Settime` (0x16), `Gettime` (0x17), `Setprt` (0x21), `NVMaccess` (0x2E) — Nummern nach `tosdump/cases/COVERAGE-BIOS.md`. Nicht im Scope: `Supexec`, die PSG-Gruppe, `Puntaes`. Slug `xbios-system` |
| Datum | 2026-10-28 |
| NewTOS | `3d74514b24e6f274fed6a71d9e305d348da047b3`, sauber (losgeloester Worktree, `git status` leer vor und nach dem Lauf) |
| EmuTOS | `1a7484aba1bfba23e2c0899e243b6d8177fe41df` |
| ROM | `tools/oracle/etos512de-1a7484ab.img`, sha256 `323c87bb…2d2750c` |
| Hatari | 2.6.1 (Binary sha256 `953f4e78…dffcd6f6`), `tools/oracle/hatari.cfg` sha256 `c8f4998a…38f8ae46` |
| Kanarienvogel | `tt` OK, `falcontc` OK (`docs/analysis/ORACLE.md` §6); kein Plattenprofil (kein Plattenaufruf im Scope) |
| Kernel | `cargo xtask build --board rpi4 --release` im Worktree: `kernel8.img` `bc724fea…a85f6a` |
| Abbilder | `tools/mkcard.sh`-Karten je Lauf, `optional_missing none`; Hashes in den Laufverzeichnissen |
| Board | QEMU 11.0.2 `raspi4b`, `-append serial=on` |
| Belege | `measurements/measurement-2026-10-28-xbios-system-audit/RUNINFO.TXT` |

Messweg: `bios-r1.tsv` (Vorflug), die neue Tabelle `tosdump/cases/xbios-system-r1.tsv`
(43 Faelle, Referenzen `EMUTOS-1a7484ab-XBIOS-system{,-falcontc}`) und drei Sonden
(`measurements/measurement-2026-10-28-xbios-system-audit/method/xsprobe.c`, `measurements/measurement-2026-10-28-xbios-system-audit/method/xsseed.c`, `measurements/measurement-2026-10-28-xbios-system-audit/method/xshi.c`) fuer das, was der
DUMPBIOS-Waechter verweigert oder was Zustand braucht, den eine Falltabelle nicht
herstellen kann.

## 2. Befunde

### XBIOS-SYS-001 — S3 — MEASURED

**`Settime` speichert unmoegliche Datums-/Zeitfelder nicht roh, sondern normalisiert bzw. klemmt sie; EmuTOS uebernimmt sie unveraendert in die GEMDOS-Uhr und zaehlt sie roh weiter.**

- LOCALIZED: `tos-api/src/gemdos/time.rs:205-216@xbios_settime` (Monat/Tag
  geklemmt, `compose` rechnet Stunde/Minute/Sekunde in Sekunden um). `deps
  settime --down` hat auf NewTOS-Seite keine Abhaengigkeit (die EmuTOS-Treiber
  `nsetdt`, `isetdt` … sind „no counterpart“); beobachtet ueber
  `Tgetdate`/`Tgettime` (GEMDOS) und `Gettime`.
- Faelle: Sonde T10–T22 (sofortiges Zuruecklesen), T23–T28 (nach dem naechsten
  Uhrtakt). Belege: `measurements/measurement-2026-10-28-xbios-system-audit/runs/PROBE-EMUTOS-tt/XSPROBE.LOG`
  (dazu `-tt-repeat`, `-falcontc`, gleich) gegen
  `measurements/measurement-2026-10-28-xbios-system-audit/runs/PROBE-NEWTOS-raspi4b/XSPROBE.LOG`
  (`-repeat` gleich); Gegenueberstellung
  `measurements/measurement-2026-10-28-xbios-system-audit/probe-diff-emutos-tt-vs-newtos.txt`.
  Beispiele (gesetzt → gelesen, EmuTOS / NewTOS):
  - `2001-02-03 25:00:00` → `25:00:00` / `2001-02-04 01:00:00`
  - `2001-02-03 04:05:62` → `04:05:62` / `04:06:02`; nach einem Takt `04:06:00` / `04:06:04`
  - `2001-13-03` → `2001-13-03` / `2001-12-03`; `2001-02-30` → `2001-02-30` / `2001-02-28`
  - `0x00000000` → `1980-00-00 00:00:00` / `1980-01-01 00:00:00`
  - `2001-02-03 23:59:62` nach einem Takt → `23:60:00` / `2001-02-04 00:00:02`
  - `2001-02-30 23:59:58` nach einem Takt → `2001-03-01 00:00:00` / `2001-02-28 23:59:58`
  Gueltige Werte (T01–T08: Tag-, Monats-, Jahreswechsel, Schaltjahr 2000, 2100,
  Ueberlauf 2107→1980) sind auf beiden Seiten gleich.
- Input-Check: EmuTOS — die 27 `Settime`-Argumente im Hatari-Trace
  (`measurements/measurement-2026-10-28-xbios-system-audit/runs/PROBE-EMUTOS-tt/trace-program.txt`) sind in Reihenfolge die `set=`-Werte
  des Protokolls. NewTOS — kein Tracer bei `3d74514b`; indirekt: T01 liest den
  gesetzten Wert exakt zurueck, jede Abweichung ist eine deterministische
  Funktion des uebergebenen Werts.
- EmuTOS: `references/emutos/bios/clock.c:1218-1222` (`current_time`/`current_date`
  roh gesetzt), `references/emutos/bdos/time.c:175-230` (`tikfrk` zaehlt die rohen
  Felder weiter), `references/emutos/bdos/time.c:98-135` (`Tgetdate`/`Tgettime`
  geben den Cache zurueck).
- Spec: `docs/api/xbios/16-settime.md` vermerkt das Klemmen von Monat/Tag als
  „documented deviation“, behauptet aber, Stunde/Minute/Sekunde „pass through
  unvalidated“ — gemessen werden auch sie normalisiert (siehe XBIOS-SYS-006).
  Kein Eintrag in `docs/COMPATIBILITY.md`.
- Fehlerbild: ein Programm, das mit `Settime` einen Rohwert schreibt und ueber
  `Tgettime`/`Gettime` zurueckliest (Uhr-Stellprogramme, Testprogramme), sieht
  einen anderen Wert; normale Programme setzen gueltige Zeiten (deshalb S3).
- Retest: `deps settime --up` = `xbiostrap`, `xsetdate`, `xsettime`
  (`Tsetdate`/`Tsettime` committen ueber `Settime`, `bdos/time.c:122,150`);
  Sonde T01–T33.
- Route: Developer (rohe Felder halten wie EmuTOS) oder Researcher (Abweichung
  mit Begruendung in COMPATIBILITY eintragen, Spec korrigieren).

### XBIOS-SYS-002 — S3 — MEASURED

**`Gettime` liefert den gepackten LONG nullerweitert in `x0`: ab Jahr 2044 (Bit 31 gesetzt) ist das Ergebnis fuer ein LP64-Programm positiv, der TOS-LONG aber negativ.**

- LOCALIZED: `tos-api/src/xbios/mod.rs:119@GETTIME` (`xbios_gettime() as i64`
  auf einem `u32`).
- Fall: Sonde `xshi.c` H01, `Settime(0x8CCF6000)` (2050-06-15 12:00:00):
  `measurements/measurement-2026-10-28-xbios-system-audit/runs/HIGH-NEWTOS-raspi4b/XSHI.LOG`
  (und `-repeat`): `ret=8ccf6000 hi=00000000 negative-as-long=no`.
- EmuTOS-Seite: `LONG gettime(void)` (`references/emutos/bios/clock.c:1266`); auf dem
  m68k ist `d0` der LONG, die Vergleichsregel definiert die obere Haelfte als
  dessen Vorzeichenerweiterung (`docs/TOS-REFERENCE-DUMPER.md` §3, `ret_hi`).
  **Am Orakel nicht messbar**: Hatari nimmt keinen RTC-Schreibzugriff an und
  liefert die Wirtsuhr (2026), also nie Bit 31
  (`measurements/measurement-2026-10-28-xbios-system-audit/runs/HIGH-EMUTOS-tt/XSHI.LOG`, `measurements/measurement-2026-10-28-xbios-system-audit/runs/PROBE-EMUTOS-tt-nvtrace/trace.txt`,
  siehe XBIOS-SYS-007).
- Spec: `docs/api/xbios/17-gettime.md` sagt „LONG result **sign-extended** into
  `x0`“, ebenso der Bindungsvertrag in `sdk/libc/mintlib/include/mint/osbind.h`
  („a sign-extended TOS LONG“); der Kommentar `tos-api/src/xbios/mod.rs:109-111@SETTIME`
  sagt „zero-extended“. Gleiche Klasse wie XBIOS-CHARDEV-006 (`Rsconf`).
- Fehlerbild: `long t = Gettime(); if (t < 0) …` oder ein Vergleich mit einem
  aus `Tgetdate`/`Tgettime` gebauten LONG faellt ab 2044 anders aus.
- Retest: `deps gettime --up` = `cprint_asctime`, `time_init`, `xbiostrap`
  (NewTOS hat fuer `time_init` kein Gegenstueck); Sonde H01.
- Route: Developer.

### XBIOS-SYS-003 — S3 — CONFIRMED

**`NVMaccess` ist nicht verdrahtet: jede Operation liefert die Funktionsnummer `0x2E`; der Referenzbau beantwortet Lesen mit 0 und Daten, Fehlerarme mit -5.**

- LOCALIZED: `tos-api/src/xbios/mod.rs:138@other` (Auffangarm); `mapping.tsv`
  „no counterpart: TT/Falcon NVRAM: not implemented“. `deps nvmaccess --down`:
  nur `compute_sum`/`get_sum`/`set_sum`, ohne NewTOS-Gegenstueck.
- Faelle: `bios-r1` 189–194 und `xbios-system-r1` 68–76, alle EXACT-DIFF
  (EmuTOS `0x0000` bzw. `0xFFFB`, NewTOS `0x002E`; beim Lesen bleibt der Puffer
  vergiftet). tosdiff:
  `measurements/measurement-2026-10-28-xbios-system-audit/runs/NEWTOS-bios-r1/tosdiff-newtos-vs-emutos.txt`,
  `measurements/measurement-2026-10-28-xbios-system-audit/runs/NEWTOS-xbios-system-r1/tosdiff-newtos-vs-emutos.txt`
  (falcontc: dieselben Fall-IDs).
- Input-Check: EmuTOS — Hatari zeigt `NVMaccess` ohne Argumente (9 Aufrufe, die
  Anzahl der Tabelle; `measurements/measurement-2026-10-28-xbios-system-audit/runs/TRACE-EMUTOS-xs/trace-program.txt`), die Ergebnisse
  passen zu den Armen der Tabelle (Lesen 0, Bereichs- und Op-Fehler -5).
  NewTOS — kein Tracer; die Antwort haengt von keinem Argument ab.
- EmuTOS: `references/emutos/bios/nvram.c:155-214`. Auffaellig, **nicht gemessen**:
  `0x2E` ist genau EmuTOS' Antwort auf einer Maschine ohne NVRAM
  (`references/emutos/bios/nvram.c:161-162`, `!has_nvram`); ein gepinntes Profil ohne
  NVRAM gibt es nicht.
- Spec: widerspruechlich — `docs/api/xbios/_triage.md:80` begruendet „unimpl“
  mit „`CONF_WITH_NVRAM` off“, `docs/api/xbios/_full-function-inventory.md`
  stellt fuer `make 512` `CONF_WITH_NVRAM` = 1 fest. `docs/COMPATIBILITY.md` sagt
  nur im Tastaturabschnitt „NewTOS has no NVRAM“, ohne Eintrag fuer das
  API-Verhalten von `NVMaccess`.
- Fehlerbild: Falcon-/TT-Kontrollfelder und Einstellprogramme sehen „kein
  NVRAM“ (`0x2E`) und fallen zurueck; Daten gehen nicht verloren. S3, weil die
  Antwort EmuTOS' eigener Arm fuer fehlende Hardware ist; entscheidet der
  Researcher, dass das Falcon-Profil NVRAM zeigen muss, ist es S2.
- Retest: `deps nvmaccess --up` = `detect_akp`, `detect_idt`,
  `screen_init_mode`, `scsi_init`, `xbiostrap` (EmuTOS-intern); Faelle oben.
- Route: Researcher (Entscheidung „kein NVRAM wie EmuTOS ohne Chip“ mit
  COMPATIBILITY-Eintrag, oder ein emuliertes 50-Byte-NVRAM → Developer).

### XBIOS-SYS-004 — S4 — MEASURED

**`Random` holt den ersten Seed aus der 60-Hz-Bildfrequenzuhr statt aus `_hz_200`, obwohl NewTOS `_hz_200` fuehrt.**

- LOCALIZED: `tos-api/src/xbios/system.rs:14-28@random` (`h.frame_clock()`; dazu
  `s = 1` bei Seed 0, wo EmuTOS mit 0 weiterrechnet — gelesen, unerreichbar in
  der Praxis).
- Fall: Sonde `xsseed.c` X01 (erster `Random()` nach dem Start, `_hz_200` davor
  und danach): EmuTOS `yes` (tt, tt-repeat, falcontc), NewTOS `no` (beide
  Laeufe, `hz_200` 3024, Wert `91cc67` = Seed aus `frame_clock` 908).
  `measurements/measurement-2026-10-28-xbios-system-audit/runs/SEED-EMUTOS-tt/XSSEED.LOG`,
  `measurements/measurement-2026-10-28-xbios-system-audit/runs/SEED-NEWTOS-raspi4b/XSSEED.LOG`.
  Der Generator selbst ist gleich (R01–R03, P08: 1000 Werte ohne Bit 24–31,
  zehn Folgewerte aus einem LCG-Seed, `Protobt`-Zufallsseriennummer ist der
  naechste `Random`-Wert — auf beiden Seiten).
- EmuTOS: `references/emutos/bios/xbios.c:421-430`.
- Spec: `docs/api/xbios/11-random.md` fuehrt den Auto-Seed als „Reference“, nicht
  als Vertrag, und beschreibt die Bildfrequenzuhr; kein COMPATIBILITY-Eintrag.
- Fehlerbild: nur ein Programm, das `_hz_200` liest und den ersten
  `Random`-Wert vorhersagt, merkt es. S4.
- Retest: `deps random --up` = `protobt`, `xbiostrap`, Desktop-`write_boot`;
  Sonde X01, P08, R01–R03.
- Route: Developer (Seed aus `_hz_200`, wie EmuTOS) oder Researcher
  (COMPATIBILITY-Eintrag).

### XBIOS-SYS-005 — S4 — PLAUSIBLE

**Startuhr ohne RTC: NewTOS beginnt bei 1980-01-01 00:00 (oder beim FAT-Datum von `C:\KERNEL8.IMG`), EmuTOS ohne Uhr beim Baudatum des ROMs um Mitternacht.**

- NOT LOCALIZED (nicht gegen das Orakel gemessen). NewTOS-Seite gemessen:
  `[clock] no kernel8.img FAT date on C: — clock stays at 1980-01-01` und T00
  `1980-01-01 00:00:14` (`measurements/measurement-2026-10-28-xbios-system-audit/runs/PROBE-NEWTOS-raspi4b/BOOT-SERIAL.txt`, `XSPROBE.LOG`);
  Quelle `kernel/src/lib.rs:1315@seed_clock_from_kernel_file`.
- EmuTOS: `DEFAULT_DATETIME` = `MAKE_ULONG(os_header.os_dosdate, 0)`
  (`references/emutos/bios/ikbd.h:95`), Rueckfall in `references/emutos/bios/clock.c:1319`
  und Initialisierung der IKBD-Uhr `references/emutos/bios/clock.c:1211`.
- Fall, der es entscheidet: ein Orakelprofil ohne RTC — gibt es nicht (Hatari
  liefert fuer NVRAM- und IKBD-Uhr die Wirtsuhr). Ersatz: echte TT/ST-Hardware
  ohne Batterie, `Gettime` direkt nach dem Start `== os_dosdate << 16`.
- Spec: `docs/api/xbios/17-gettime.md` nennt den Unterschied (1980-01-01 statt
  Baudatum), verschweigt aber die Saat aus `KERNEL8.IMG`; kein
  COMPATIBILITY-Eintrag. `TASK/RTC01.md` (offen) ersetzt die Quelle spaeter.
- Route: Researcher.

### XBIOS-SYS-006 — S4 — MEASURED (Doku widerspricht Messung oder Code)

Route: Researcher.

1. `docs/api/xbios/16-settime.md` (Pi binding): „Hour/minute/second pass through
   unvalidated (they only roll the clock forward)“ — gemessen normalisiert
   (T10–T14, XBIOS-SYS-001); ebenso der Kommentar `tos-api/src/gemdos/time.rs:211@hour`
   „(EmuTOS parity)“.
2. `tos-api/src/xbios/mod.rs:109-111@SETTIME` „Gettime returns the packed LONG
   (zero-extended)“ gegen `docs/api/xbios/17-gettime.md` „sign-extended“
   (XBIOS-SYS-002).
3. `docs/api/xbios/_triage.md:80` „`CONF_WITH_NVRAM` off“ und
   `docs/api/xbios/_dispatcher.md:171` („NVRAM (0x2E) entries are `xbios_unimpl`
   in the profile“) gegen `docs/api/xbios/_full-function-inventory.md` (=1 im
   Referenzbau).
4. `tosdump/cases/bios-r1.tsv` Kommentare zitieren `bios/nvram.c:180-181`,
   `:206-208`, `bios/floppy.c:703-717`, `:725-735`, `:711` — am Pin stehen die
   Stellen bei `references/emutos/bios/nvram.c:178-179`, `:208-210`,
   `references/emutos/bios/floppy.c:698-713`, `:727-735`, `:706`. Tabelle ist
   eingefroren (CRC); Kandidat fuer `tosdump/cases/ERRATA.md`.
5. `docs/api/xbios/17-gettime.md` „fallback = 1980-01-01 if the clock was never
   set“ — ohne die Saat aus `KERNEL8.IMG` (XBIOS-SYS-005).
6. Ledger: `tosdump/cases/COVERAGE-BIOS.md` braucht einen Abschnitt fuer
   `xbios-system-r1.tsv` (der Auditor aendert es nicht).

### XBIOS-SYS-007 — S4 — MEASURED (Orakelgrenze)

**Hatari 2.6.1 nimmt keinen Schreibzugriff auf die Zeitregister der TT/Falcon-RTC an: `Gettime` liefert nach `Settime` die Wirtsuhr.**

- Beleg: `measurements/measurement-2026-10-28-xbios-system-audit/runs/PROBE-EMUTOS-tt-nvtrace/trace.txt`
  — nach `Settime(0x2A4320A3)` schreibt EmuTOS (`nvram_setregs`,
  `references/emutos/bios/clock.c:893-907`) Jahr 33, Monat 2, Tag 3, 4:05:06 in die
  Register 9…0; das folgende `Gettime` liest 58/10/28 12:20:12 (Wirtsuhr).
  In allen EmuTOS-Laeufen bleibt `Gettime` die Wirtsuhr; nach dem Lauf ist sie
  hinter der aus `_hz_200` gerechneten Zeit zurueck (Wirtszeit statt
  emulierter Zeit unter `--fast-forward`).
- Folge: `Gettime` nach `Settime` ist am Orakel nicht vergleichbar; dieser
  Audit vergleicht `Settime` deshalb ueber den GEMDOS-Cache
  (`Tgetdate`/`Tgettime`), den `Settime` auf EmuTOS immer schreibt
  (`references/emutos/bios/clock.c:1221-1222`).
- Route: Researcher (in `docs/analysis/ORACLE.md` als Grenze vermerken).

## 3. Offene Tickets im Scope

`TASK/RTC01.md` (offen, P3) ist ein Hardware-Vorhaben (DS3231), kein
Fehlerticket; es gibt keinen Fall, der es als EQUAL/DIFF entscheidet. Die hier
gemessene Uhr (XBIOS-SYS-001, -002, -005) ist das, was RTC01 an der Quelle
aendert, nicht am API-Verhalten. Sonst kein offenes Ticket zu den sieben
Funktionen.

## 4. Coverage-Matrix

| EmuTOS | NewTOS (mapping) | Schicht | Status | Faelle |
|---|---|---|---|---|
| `random` `bios/xbios.c:421` | `tos-api/src/xbios/system.rs:14@random` | bios | **DIFF** (Seed-Quelle, XBIOS-SYS-004; Generator EQUAL) | `bios-r1` 112; Sonde R01–R03, P08, X01 |
| `protobt` `bios/floppy.c:690` | `tos-api/src/xbios/floppy.rs:126@protobt` | bios | **EQUAL** | `bios-r1` 115–119; `xbios-system-r1` 26–60 (31 Faelle); Sonde P01–P08 |
| `cursconf` `bios/vt52.c:893` | `tos-api/src/xbios/screen.rs:380@cursconf`, `tos-api/src/vdi/escape.rs:761@cursconf` | bios | **EQUAL** (Rueckgaben; Bildschirmwirkung 0–3 offen, §6) | `bios-r1` 122; `xbios-system-r1` 63–65; Sonde C01–C17 |
| `settime` `bios/clock.c:1218` | `tos-api/src/gemdos/time.rs:205@xbios_settime` | bios | **DIFF** (XBIOS-SYS-001) | Sonde T01–T33 |
| `gettime` `bios/clock.c:1266` | `tos-api/src/gemdos/time.rs:222@xbios_gettime` | bios | **DIFF** (XBIOS-SYS-002) | `bios-r1` 125; Sonde T00, H01 |
| `setprt` `bios/parport.c:49` | `tos-api/src/xbios/system.rs:210@setprt` | bios | **EQUAL** | `bios-r1` 135; Sonde S01–S09 |
| `nvmaccess` `bios/nvram.c:155` | — (Auffangarm `tos-api/src/xbios/mod.rs:138@other`) | bios | **DIFF** (XBIOS-SYS-003) | `bios-r1` 189–194; `xbios-system-r1` 68–76 |

equal 3, accepted 0, diff 4, unmeasured 0 von 7. Bug-Dichte: CONFIRMED 1 ÷ 7
gemessene Funktionen = 0,14; mit den MEASURED-Befunden des API-Verhaltens
(001, 002, 004) 4 ÷ 7.

Abhaengigkeiten (`cargo xtask deps <fn> --down`, Ausgaben in
`measurements/measurement-2026-10-28-xbios-system-audit/deps/`): `random` und
`setprt` haben keine; `protobt` → `compute_cksum` (gleich gemessen ueber jede
Pruefsumme) und `random` ueber Trap 14 (P08: gleiche Kette); `cursconf` →
`cursor_on`/`cursor_off` (Rueckgaben gemessen, Bildwirkung offen);
`settime`/`gettime` → nur EmuTOS-RTC-Treiber ohne NewTOS-Gegenstueck;
`nvmaccess` → Pruefsummenhelfer ohne Gegenstueck. Die Zuordnung der Befunde
haengt damit an keiner ungemessenen unteren Schicht.

## 5. Neue Faelle

`tosdump/cases/xbios-system-r1.tsv` — 43 Faelle, CRC `3944A677`, sha256
`22c32ade…6b59241d`, ganz im Round-1-Waechter:

- `Protobt` (31): jeder `disktype` 0–6 (BPB 0:32 und Pruefsumme @510),
  `disktype` 0x7FFF/-32768/-1, die unberuehrten Bytes 32–509, Seriennummer
  0xABCDEF/0xFFFFFF/0/-2/0x80000000, Zufallsarm 0x1000000/0x7FFFFFFF (nur die
  unberuehrten Bytes), `exec` 0/2/0x7FFF/-32768.
- `Cursconf(5,x)` (3): Operand 1, -1, 0x7FFF.
- `NVMaccess` (9): ein Byte, letztes Nutzerbyte, Block bei 8, Start 48,
  Anzahl 49, Start -32768, Op -1/0x7FFF, Op 3 mit NULL.

Referenzen `tosdump/reference/EMUTOS-1a7484ab-XBIOS-system.{BIN,LOG}` (tt) und
`-falcontc`; tt-Wiederholung bitgleich, tt gegen falcontc 0 von 43
verschieden, `tosdiff --selfcheck` auf beiden bestanden.

Vorgeschlagene Zeilen, die der Waechter heute verweigert (Sondenbereich):

```
# Settime/Gettime (braucht ein Orakel, dessen RTC Schreibzugriffe annimmt, XBIOS-SYS-007)
+14:0x16	l:0x8CCF6000	IGNORE	-	Settime(2050-06-15 12:00) prelude
14:0x17	-	EXACT	-	Gettime() bit 31 -> high half
# Cursconf-Setzarme mit Wiederherstellung
+14:0x15	w:4,w:0x1FF	IGNORE	-	prelude
14:0x15	w:5,w:0	EXACT.W	-	Cursconf(5) after 0x1FF -> 0xFF
# Setprt-Setzarm mit Wiederherstellung
+14:0x21	w:-2	EXACT.W	-	prelude
14:0x21	w:-1	EXACT.W	-	Setprt(-1) after -2
```

## 6. Gemessen vs. offen

Gemessen (tosdiff oder reproduzierte Sonde, beide Seiten):

- `Protobt` vollstaendig ueber die Argumentarme, inklusive ausfuehrbarem
  Ausgangssektor, ungerader Pufferadresse (68030 und AArch64) und
  Zufallsseriennummer als naechster `Random`-Wert.
- `Random`: Bereich, Generator, Seed-Quelle.
- `Cursconf`: alle Rueckgaben, 4/5-Rundlauf mit `LOBYTE` (0x1FF → 255, -1 → 255,
  0x80 → 128), unbekannte Funktionen → 0 ohne Wirkung.
- `Setprt`: Setzen/Abfragen, nur -1 ist Abfrage (-2 und -32768 werden
  gespeichert), Wiederherstellung.
- `Settime` → GEMDOS-Uhr fuer gueltige und unmoegliche Werte, Takt danach;
  `Tsettime`/`Tsetdate` Rueckgaben inkl. Fehlerarme (T30–T33 Rueckgaben gleich).
- `NVMaccess` alle Lese- und Fehlerarme.

Offen:

- `Gettime` nach `Settime` am Orakel (XBIOS-SYS-007) — die NewTOS-Seite ist
  gemessen (gleiche Uhr wie GEMDOS, T01–T33), die EmuTOS-Seite auf echter
  Hardware liest die RTC.
- `Gettime`-Wert mit Bit 31 am Orakel (XBIOS-SYS-002 stuetzt sich dort auf die
  ABI-Regel).
- EmuTOS ohne RTC/NVRAM (XBIOS-SYS-003, -005): kein gepinntes Profil.
- `Cursconf(0..3)` auf dem Bildschirm (Sichtbarkeit, Blinken, Blinkrate):
  Rueckgaben gleich, Bildwirkung nicht gemessen (Screendump-Lauf oder Pi).
- `Protobt` mit NULL- oder ungemapptem Puffer: EmuTOS schreibt in den unteren
  Speicher (`references/emutos/bios/floppy.c:690-736` prueft nicht), am Orakel nicht
  gefahrlos; NewTOS gibt bei NULL 0 zurueck (`tos-api/src/xbios/floppy.rs:131@buf_ptr`).
- NewTOS-Eingangspruefung: kein Tracer bei `3d74514b`.
- `NVMaccess` Schreiben/Ruecksetzen am Orakel (Waechter; NewTOS antwortet ohnehin
  `0x2E`).

Eine Lehre aus der Sondenentwicklung: `Tgetdate` und `Tgettime` sind zwei
Aufrufe; faellt ein Uhrtakt dazwischen, paart sich altes Datum mit neuer Zeit
(erste Sondenfassung zeigte so „2000-02-28 00:00:00“). Die Sonde liest
deshalb Datum–Zeit–Datum bis zur Gleichheit.

## 7. Real-Pi4-Liste

- `Cursconf(0..3)` und `Cursconf(4,n)`: Cursor sichtbar/unsichtbar, Blinken
  an/aus, Blinkrate am HDMI-Bild (COMPATIBILITY Q5: 1/60 s je Einheit auf
  `_hz_200`).
- Startuhr auf der echten Karte: Saat aus dem FAT-Datum von `KERNEL8.IMG`
  (`kernel/src/lib.rs:1315@seed_clock_from_kernel_file`), `Gettime` direkt nach dem
  Start.

## 8. Akzeptable Abweichungen

Keine. Die Abweichungen 001–005 haben keinen passenden Eintrag in
`docs/COMPATIBILITY.md` und sind ueber das API beobachtbar; die
Spec-Vermerke („documented deviation“ in `16-settime.md`, „Reference“ in
`11-random.md`) ersetzen den COMPATIBILITY-Eintrag nicht.

## 9. Out-of-scope observations

- GEMDOS `Tsetdate`: EmuTOS prueft Monat und Tag nicht nach unten
  (`references/emutos/bdos/time.c:108-125`, Monat 0/Tag 0 werden angenommen), NewTOS
  lehnt sie laut eigenem Kommentar ab — nicht gemessen.
- Die Hauptlinie steht inzwischen auf `c686cd7d` mit dem BIOS/XBIOS-Argumenttracer
  (TOOL08, `61f39de2`); die NewTOS-Eingangspruefung dieses Audits ist damit
  nachholbar.

## 10. Cleanup

- Keine temporaere Verdrahtung: kein Produktcode beruehrt, keine
  `AUDIT-TEMP`-Zeile gesetzt; Suche im Worktree ohne Treffer.
- Worktree `3d74514b`: `git status` leer.
- Neue Dateien im Hauptbaum (nicht committet): dieser Bericht,
  `tosdump/cases/xbios-system-r1.tsv`,
  `tosdump/reference/EMUTOS-1a7484ab-XBIOS-system{,-falcontc}.{BIN,LOG}`,
  `measurements/measurement-2026-10-28-xbios-system-audit/`.
- Fremde Aenderungen im Hauptbaum am Ende des Laufs (nicht angefasst, nicht
  verwendet): `tools/xtask/src/main.rs` (geaendert), `tools/xtask/src/tosdump.rs`
  und `measurements/measurement-2026-10-28-tool09/` (neu) — TOOL09-Arbeit eines
  anderen Agenten; die TOOL08-Arbeit wurde waehrend des Laufs committet.
- Kein eigener QEMU- oder Hatari-Prozess laeuft mehr; ein Hatari des
  TOOL08-Entwicklers und ein `cargo xtask tosdump`-Lauf von TOOL09 liefen
  waehrend des Audits und wurden nicht angefasst.

## 11. Was am Verfahren hinderte oder unklar war

- `couplings.md` (im Verzeichnis `docs/analysis/deps/`) existiert nicht; das Verfahren verlangt,
  dessen Szenarien zu fahren. Die Kopplung `Settime` ↔ `Tsetdate`/`Tsettime`
  steht dafuer im Graphen (`deps settime --up`) und wurde gemessen.
- Das Orakel kann die Uhr nicht stellen (XBIOS-SYS-007); der Hinweis im Brief
  („relationales Design“) traf zu, aber erst ein NVRAM-Trace zeigte, dass
  `Gettime` dabei ganz ausfaellt.
- Hatari druckt `Protobt` und `NVMaccess` ohne Argumente; die EmuTOS-Eingangspruefung
  bleibt fuer diese beiden eine Zaehlung plus Plausibilitaet der Ausgabe.
- `casecheck` kennt keine Aufruftabellen (nur VDI/AES/Raster); eine neue
  Tabelle wird erst vom Lader in Hatari geprueft.
- Eine korrigierte Kommentarzeile aendert die Tabellen-CRC; Referenzen muessen
  danach neu aufgenommen werden (hier geschehen, die erste Aufnahme verworfen).
