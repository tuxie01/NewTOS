# BIOS Boot-Time Initialisation — Low-Memory Sysvars, OSHEADER, Cookie-Jar

> Researcher, 2026-07-23. Ausgelöst durch
> `docs/analysis/sysvar-osheader-lowmem-gap.md` (TeraDesk crasht bei
> direktem `_sysbase`/OSHEADER-Zugriff) und
> `docs/analysis/bios-audit/00-PLAN.md` BB1. Track A (EmuTOS-Quelle),
> Ziel-Profil Falcon030/TOS4 `make 512` (Standard-Voll-Build, kein
> `TARGET_192`/`TARGET_256`/`MACHINE_*`, siehe
> `docs/analysis/aes-audit/emutos-config-profile.md`).
>
> **Scope:** die **Boot-Zeit-Befüllung** der Low-Memory-Systemvariablen
> `$400`–`$5FF` (mit Kontext bis `$800`, dem dokumentierten Ende des
> Sysvar-Bereichs), der OSHEADER-Struktur (`_sysbase`@`$4F2`) und des
> Cookie-Jars (`$5A0`). Dies ist ein **Init-Dokument** (wer setzt was,
> wann, im Boot-Ablauf) — die Trap-13-Dispatcher-Funktionen, die diese
> Variablen zur Laufzeit lesen/schreiben, sind einzeln in
> `docs/api/bios/00-getmpb.md` … `0b-kbshift.md` dokumentiert; die
> Adress-Tabelle "was ein Programm beobachten kann" lebt in
> `docs/api/bios/_state.md` §1 (dieses Dokument ergänzt sie um die
> **vollständige** `tosvars.ld`-Deckung + den zeitlichen Ablauf).
>
> **NICHT im Scope:** das Speichermodell/ST-RAM-vs-TT-RAM-Sizing selbst
> (BB2, `docs/analysis/emutos-memory-model.md`) — hier wird nur
> dokumentiert, *dass* und *wann* `memvalid`/`memval2`/`memval3`/
> `phystop`/`membot`/`memtop` gesetzt werden, nicht *wie* EmuTOS die
> ST/TT-MMU-Bank-Größe per Bus-Error-Sondierung ermittelt
> (`bios/memory.S`, reine Atari-Hardware-Sondierung, für NewTOS/RPi4
> ohnehin nicht 1:1 nachbaubar).

## EmuTOS-Referenzen (Primärquellen dieses Dokuments)

- `references/emutos/tosvars.ld` — **die** Adress-Autorität für jede
  Low-Memory-Variable (vollständig gelesen, Zeilen 22–135).
- `references/emutos/include/tosvars.h` — welche Sysvars einen echten
  C-`extern`-Zugriff haben (Rest sind reine Linker-Reservierungen).
- `references/emutos/bios/startup.S:1-135` — OSHEADER-Assemblerdefinition
  (`_os_header`), früher Boot-Ablauf bis `jmp _biosmain`.
- `references/emutos/bios/memory.S:100-260,560-610` — `memchk`/`memconf`:
  Cold- vs. Warmboot-Unterscheidung, Sysvar-Bereich-Clear, `memvalid`/
  `memval2`/`memval3`/`memctrl`/`phystop` Setzen.
- `references/emutos/bios/bios.c:118-580,781-878` — `vecs_init`,
  `bios_init`, `biosmain` (voller Boot-Ablauf in C).
- `references/emutos/bios/biosmem.c:51-83` — `bmem_init` (`membot`/
  `memtop`/`end_os`).
- `references/emutos/bios/blkdev.c:60-110` — `blkdev_init` (`hdv_*`-Vektoren,
  `dskbufp`, `pun_ptr`).
- `references/emutos/util/cookie.c`, `references/emutos/bios/machine.c:665`
  (`fill_cookie_jar`) — Cookie-Jar-Boot-Sequenz (Details in
  `docs/api/bios/_cookie-jar.md`).
- `references/emutos/bdos/bdosmain.c:339`, `references/emutos/bdos/proc.c:575,609`
  — `run` (Ziel von `os_run`), GEMDOS-seitig, nicht BIOS.
- `references/emutos/bios/panicasm.S:115` — einziger Ort, der `resvalid`
  aktiv **löscht** (Panic-Handler).

---

## 1. Der Boot-Ablauf (zeitliche Reihenfolge, file:line)

```
startup.S _realmain
  → memory.S: memchk/memconf   (RAM-Größe ermitteln; Cold- vs. Warmboot, s. §2)
  → startup.S: jmp _biosmain
bios.c biosmain()
  → bios_init()
      → vecs_init()                         (bios.c:118)   Exception-Vektoren
      → processor_init(), machine_detect(), machine_init()
      → bmem_init()                         (biosmem.c:51) membot/memtop/end_os
      → cookie_init()                       (util/cookie.c) $5A0-Jar anlegen
      → fill_cookie_jar()                   (machine.c:665) Jar befüllen
      → screen_init_mode(), linea_init(), screen_init_address(), vt52_init()
      → "misc. variables" Block             (bios.c:356-373) s. §3 unten
      → init_system_timer()
      → chardev_init()                      (chardev.c)    bcon*_vec[8] + CHARDEV_AVAILABLE
      → init_serport(), kbd_init(), midi_init(), init_acia_vecs()
      → blkdev_init()                       (blkdev.c:60)  hdv_bpb/rw/mediach/init, dskbufp, pun_ptr
      → parport_init(), clock_init()
      → exec_os = os_header.os_magic->gm_init   (bios.c:534)
      → osinit_before_xmaddalt() / altram_init() / osinit_after_xmaddalt()
          (GEMDOS-Init — setzt u.a. `run = &initial_basepage;`, bdosmain.c:339)
      → swv_vec = os_header.reseth; vblsem = 1  (bios.c:552-553)
      → disk_try_dmaboot()
  → Drvmap()                                (bios.c:791)   einmaliger Aufruf (Steem-Kompatibilität)
  → bootdev = … (FIRST_BOOT-abhängig)        (bios.c:798-799)
  → initinfo() / kbshift(-1)                 (bios.c:809-812) Willkommensschirm bzw. Shift-Sondierung
  → blkdev_boot()                            (bios.c:825)
  → Dsetdrv(bootdev); init_default_environment()
  → autoexec()                               (bios.c:846)   AUTO-Ordner
  → Pexec(...) COMMAND.PRG oder exec_os      (bios.c:850-865) — hier wird `run`
      (und damit `*os_header.os_run`) auf die erste Shell/AES-PD gesetzt
```

Wichtig für den Gap: **jede** BIOS-C-Ebene-Initialisierung läuft, bevor
irgendein Anwendungsprogramm (Shell, AES, AUTO-Ordner-Programm) startet.
Ein natives EL0-Programm, das `_sysbase`/OSHEADER/Cookie-Jar liest, sieht
also — wenn NewTOS diese Ebene nachbildet — einen bereits vollständig
initialisierten Zustand; es gibt keinen Lauf-Zeit-Punkt, an dem eine App
läuft, während diese Init-Kette noch offen ist.

---

## 2. Cold-Boot vs. Warm-Boot: der Sysvar-Bereich wird nur bedingt gelöscht

**Zentraler, für den Gap sehr relevanter Befund** (`bios/memory.S:117-142,
560-610`):

- `memchk` (memory.S:117) prüft **vor** jeder anderen Sysvar-Initialisierung
  drei Magic-Werte an ihren festen Adressen:
  `memvalid`(`$420`)`==0x752019f3`, `memval2`(`$43a`)`==0x237698aa`,
  `memval3`(`$51a`)`==0x5555aaaa` (memory.S:125-140).
- **Alle drei gültig** (Warmboot / Reset mit bereits konfiguriertem RAM):
  → `memctrl`/`phystop`/(bei `CONF_WITH_TTRAM`) `ramvalid`/`ramtop` werden aus
  den bestehenden Sysvar-Werten **übernommen** (memory.S:146-152), der
  gesamte übrige Sysvar-Bereich (`$400`–`$800`) bleibt **unangetastet** —
  er wird **nicht** gelöscht. Programme, die vor dem Reset z. B.
  `resvalid`/`resvector` gesetzt hatten, sehen diese Werte nach dem Reset
  unverändert (das ist der ganze Sinn des `resvalid`/`resvector`-
  Mechanismus, s. §5).
- **Mindestens einer ungültig** (Cold-Boot / erster Boot / RAM-Config
  geändert): `memconf` (memory.S:201ff.) ermittelt die RAM-Größe per
  Bus-Error-Sondierung (reine Atari/ST-MMU-Hardware, für RPi4 ohne
  Äquivalent), dann (memory.S:566-587):
  1. `bzero($400 .. sysvars_end)` — **der komplette dokumentierte
     Sysvar-Bereich wird auf 0 gesetzt** (`sysvars_start=0x400`,
     `sysvars_end` = Ende des `tosvars.ld`-Blocks = `$800`,
     `emutos.ld:78`). Der **Crash-Dump-Bereich** `$380`–`$3FF`
     (`proc_lives` etc.) liegt **außerhalb** dieses Clears — er dient
     bewusst dazu, einen Crash über einen Reset hinweg zu überleben.
  2. `memctrl`(`$424`)/`phystop`(`$42e`) aus den soeben ermittelten
     Werten geschrieben.
  3. `memvalid`(`$420`)=`0x752019f3`, `memval2`(`$43a`)=`0x237698aa`,
     `memval3`(`$51a`)=`0x5555aaaa` gesetzt — das validiert die Konfiguration
     **für den nächsten (Warm-)Boot**.
  4. Bei `CONF_WITH_TTRAM`: `ramtop`(`$5a4`)/`ramvalid`(`$5a8`) aus dem
     *vor* dem Clear geretteten Zustand wiederhergestellt (memory.S:598-599).

**Konsequenz für die Spec:** "Init-Wert" für die meisten `$400`-`$7FF`-
Sysvars ist **0 auf Cold-Boot**, gefolgt von der C-seitigen Befüllung in
§1 (die einzelnen Werte werden erst im Lauf von `bios_init()`/`biosmain()`
gesetzt, in der Reihenfolge oben). Auf Warm-Boot ist der Init-Wert
**"was vorher da war"** — das ist die Eigenschaft, auf der der klassische
TOS-"resident code across reset"-Mechanismus beruht (§5).

---

## 3. Der "misc. variables"-Block (`bios/bios.c:356-373`) — die zentrale Sysvar-Sammelstelle

Ein einziger Codeblock in `bios_init()`, **nach** `cookie_init()`/
`fill_cookie_jar()`/Screen-Init und **vor** `chardev_init()`, setzt mehrere
der für Apps sichtbaren Sysvars auf einmal:

```c
dumpflg = -1;
sysbase = &os_header;                       /* $4f2 */
savptr = (LONG) trap_save_area;             /* $4a2 */
etv_timer = (ETV_TIMER_T) just_rts;         /* $400 */
etv_critic = default_etv_critic;            /* $404 */
etv_term = just_rts;                        /* $408 */
swv_vec = just_rts;                         /* $46e (später bei bios.c:552 überschrieben) */
nvbls = ARRAY_SIZE(vbl_list);               /* $454 */
vblqueue = vbl_list;                        /* $456 */
```

Das ist der Moment, an dem `_sysbase` (`$4f2`) erstmals einen gültigen
Zeiger auf die OSHEADER-Struktur erhält (`bios/bios.c:358`) — **davor** ist
`sysbase` auf Cold-Boot `0` (aus dem `bzero` in §2), auf Warm-Boot der
alte Wert. `etv_critic` bekommt hier seinen **Default-Handler**
(`default_etv_critic`, `bios/vectors.S:823-826`) — nicht `NULL` — noch
bevor AES/Desktop überhaupt existiert; siehe `docs/api/bios/04-rwabs.md`
für den vollständigen `etv_critic`-Pfad bei Disk-Fehlern.

---

## 4. OSHEADER — Definition, Ort, Boot-Zeit-Werte

Vollständiges Feldlayout: `docs/api/structures/osheader.md` (validiert/
korrigiert in dieser Pass, s. dortige Änderungen). Kurzfassung der
**Boot-Zeit-Herkunft** jedes Feldes:

Die OSHEADER-Struktur ist **statische ROM/Text-Daten**, definiert direkt
als erste Bytes des TEXT-Segments in `bios/startup.S:73-114`
(Label `_os_header`) — sie wird nicht zur Laufzeit "gebaut", sondern liegt
bereits beim Compile/Link fertig vor:

| Feld (Offset) | Wert bei EmuTOS (Ziel-Profil `make 512`) | Quelle |
|---|---|---|
| `os_entry` (0x00) | `bra.s _main` | `bios/startup.S:78-79` |
| `os_version` (0x02) | `TOS_VERSION` = **`0x206`** im EmuTOS-Standardprofil (`include/config.h:1367-1372` — **kein** `TARGET_192`/`256`/`MACHINE_*`-Block setzt es um; `make 512`/`1024`/`prg` fallen alle auf diesen Default durch, siehe `docs/analysis/aes-audit/emutos-config-profile.md` Teil 1l/2). **Für NewTOS ist der Zielwert `0x0404` NICHT verifiziert**, s. Abschnitt „OFFEN" unten. | `bios/startup.S:80-81`, `include/config.h:1367-1372` |
| `reseth` (0x04) | `.dc.l _main` | `bios/startup.S:82-83` |
| `os_beg` (0x08) | `.dc.l _os_header` (Selbstzeiger) | `bios/startup.S:88-89` |
| `os_end` (0x0C) | `.dc.l __endvdibss` | `bios/startup.S:90-91` |
| `os_rsvl` (0x10) | `.dc.l _main` (per Kommentar "reserved") | `bios/startup.S:92-93` |
| `os_magic` (0x14) | `.dc.l _ui_mupb` (Zeiger auf AES' `GEM_MUPB`) | `bios/startup.S:94-95` |
| `os_date` (0x18) | `OS_DATE` (Build-Timestamp, BCD) | `bios/startup.S:96-97` |
| `os_conf` (0x1C) | `OS_CONF_MULTILANG` oder `(OS_COUNTRY<<1)+OS_PAL`, je `CONF_MULTILANG` | `bios/startup.S:98-103` |
| `os_dosdate` (0x1E) | `OS_DOSDATE` | `bios/startup.S:104-105` |
| `os_root` (0x20) | `.dc.l _root` (BDOS-Pool) | `bios/startup.S:106-107` |
| `os_kbshift` (0x24) | `.dc.l _shifty` | `bios/startup.S:108-109` |
| `os_run` (0x28) | `.dc.l _run` (Zeiger **auf** die GEMDOS-Variable `run`) | `bios/startup.S:110-111` |
| `os_dummy` (0x2C) | `.ascii "ETOS"` = `0x45544F53` | `bios/startup.S:112-113` |

**Die einzige zur Laufzeit geschriebene "Instanz"-Verbindung** ist
`sysbase = &os_header;` (`bios/bios.c:358`, s. §3) — die Struktur selbst ändert
sich nach dem Link nie; nur der `_run`-Inhalt (über `os_run`-Doppel-
Indirektion) und `exec_os`/`swv_vec` (separate Sysvars, nicht Teil der
OSHEADER) ändern sich zur Laufzeit.

### `_run` (Ziel von `os_run`) — GEMDOS, nicht BIOS

`os_run` zeigt auf die GEMDOS-Variable `run` (ein `PD **`-Ziel), die
**außerhalb** der BIOS-Schicht lebt:

- `bdos/bdosmain.c:339` — `run = &initial_basepage;` (GEMDOS-Boot-Init,
  läuft innerhalb `osinit_before_xmaddalt()`/`osinit_after_xmaddalt()`
  aus §1).
- `bdos/proc.c:575` — `run = (PD *)p;` bei jedem `Pexec`.
- `bdos/proc.c:609` — `run = run->p_parent;` bei `Pterm`.

Ein Programm, das `*(osheader->os_run)` liest, sieht also immer die
**aktuell laufende** Basepage — für ein natives EL0-Programm wäre das
(wenn NewTOS diese Kette nachbildet) die eigene PD, exakt wie unter
klassischem TOS.

### OFFEN — `os_version`/`GEMDOS_VERSION`-Zielwert für NewTOS

Der 00-PLAN.md-Auftragstext dieser Phase nennt `os_version = 0x0404`
(TOS 4.04/Falcon) als scheinbar feststehend. Das ist **nicht** durch
EmuTOS-Quelle gedeckt (EmuTOS' eigener Default ist `0x206`, s. Tabelle
oben) und laut der bereits vorhandenen Analyse
`docs/analysis/aes-audit/emutos-config-profile.md` (Teil 2, "Entscheidung
… GEMDOS_VERSION") **explizit als noch zu verifizieren markiert**:
„Zu verifizieren, NICHT raten … der exakte Zahlenwert. Ein echter Atari
Falcon030 mit TOS 4.0x meldet GEMDOS 0.30 → `Sversion()` byte-getauscht
`0x3000` (plausibel, aber vor dem Setzen gegen Compendium/Profibuch bzw.
reales TOS4 gegenprüfen)." Dieselbe Unsicherheit gilt für den
`os_version`-Feldwert selbst (`0x0404`?). **Dieser Researcher übernimmt
diese offene Markierung unverändert** — kein neuer Wert wird hier
festgelegt; die Entscheidung braucht eine Cross-Referenz gegen
Compendium/Profibuch oder eine reale TOS4-Maschine, nicht eine Vermutung.

---

## 5. `resvalid`/`resvector`/`memvalid`/`memval2` — der Warm-Reset-Mechanismus (TeraDesk-Beleg)

> Ergaenzung 2026-10-17 (RST01): `warm_reset`/`cold_reset`, Tastenkombination und
> der OSHEADER-Einsprung `os_res1`/`reseth` -> `_main` stehen in
> `docs/api/bios/_reset-warm-cold.md`.

Das Gap-Dokument listet `$420` (`memvalid`), `$43a` (`memval2`), `$426`
(`resvalid`), `$42a` (`resvector`) als von TeraDesk bei **Shutdown/Reset**
gelesen. Quellbefund, was diese vier tatsächlich tun:

- **`memvalid`/`memval2`/`memval3`** (`$420`/`$43a`/`$51a`): reine
  RAM-Konfigurations-Validitätsmarker für `memchk` (§2) — ein Programm,
  das sie liest, prüft, ob die aktuelle RAM-Konfiguration nach einem
  Reset wiederverwendet würde (Warmboot-Fast-Path) oder ob der nächste
  Boot neu konfiguriert (`memconf`, inkl. **Sysvar-Bereich-Clear**).
- **`resvalid`/`resvector`** (`$426`/`$42a`): der **Warm-Reset-Sprung**-
  Mechanismus. `bios/startup.S:502-519` (`resetvec:`) prüft **vor jeder
  anderen Boot-Initialisierung** (noch vor `memchk`!), ob
  `resvalid == 0x31415926`; wenn ja, springt die CPU auf die Adresse in
  `resvector`, **ohne** die normale Boot-Sequenz zu durchlaufen. Das ist
  die dokumentierte TOS-Konvention "residenter Code überlebt einen
  `Reset`-Opcode": ein Programm pokt `resvector` = eigene Adresse,
  `resvalid` = `0x31415926`, ruft dann `Reset` (XBIOS oder direkt die
  CPU-Instruktion) auf — und landet nach dem Hardware-Reset direkt in
  seinem eigenen Code, statt dass EmuTOS normal bootet.
  - **EmuTOS selbst setzt `resvalid`/`resvector` nirgends** (kein Treffer
    außer der Prüfung in `startup.S` und dem expliziten Löschen in
    `bios/panicasm.S:115`, `clr.l resvalid.w` — der Panic-Handler
    verhindert damit, dass ein hängender resident-Handler eine
    Boot-Schleife erzeugt). Der Rückgabewert an eine App, die
    `resvalid` *liest* (ohne vorher selbst geschrieben zu haben), ist
    also **Cold-Boot: 0** (aus dem bzero, §2) **oder Warm-Boot: der
    zuletzt geschriebene Wert** (typischerweise 0, außer ein Programm hat
    ihn zuvor gesetzt und der eigene Reset-Handler ihn nicht wieder
    gelöscht).
  - **Für TeraDesk relevant:** ein Desktop-Ersatzprogramm, das eine
    eigene "Maschine neu starten"-Funktion anbietet, poked typischerweise
    genau dieses Paar (`resvector`+`resvalid`), um nach dem Reset direkt
    wieder eigenen Code (oder EmuTOS selbst über `swv_vec`/`os_header.
    reseth`, s. `bios/bios.c:552`) anzuspringen — daher der Zugriff "bei
    Shutdown/Reset" statt beim Start.

**Kein direkter Bezug zum aktuellen Boot-Crash** (der hängt an
`_sysbase`/OSHEADER, §4) — dieser Abschnitt dokumentiert die vier
Variablen für Vollständigkeit, weil sie explizit im Gap-Dokument als
TeraDesk-Fundstellen genannt sind.

---

## 6. Cookie-Jar-Boot-Sequenz (Kurzfassung — Details in `_cookie-jar.md`)

Reihenfolge innerhalb `bios_init()` (§1): `cookie_init()` legt den
20-Slot-Array an und veröffentlicht ihn über `p_cookies` (`$5a0`)
(`util/cookie.c:26-35`); **danach** füllt `fill_cookie_jar()`
(`bios/machine.c:665`) ihn mit den Hardware-Erkennungs-Cookies
(`_CPU`/`_VDO`/`_FPU`/`_MCH`/`_SND`/`_FDC`/`_AKP`/…). Beide laufen **vor**
`chardev_init()`, also lange bevor irgendein Programm startet. Der
klassische Lesezugriff (`Supexec` + `*(COOKIE**)0x5a0` + linearer Scan
bis `tag==0`) ist bereits vollständig in `docs/api/bios/_cookie-jar.md`
dokumentiert (inkl. ADR-012-Entscheidung "kernel-owned, kein `$5A0`").

**OFFEN — Konflikt mit dem Gap-Doc:** ADR-012 entschied den Jar als
kernel-owned ohne `$5A0`-Adresse, mit `Ssystem`-Zugriff statt
`Supexec`+Pointer-Scan. Das Gap-Dokument (`sysvar-osheader-lowmem-gap.md`
Prüfauftrag Punkt 4) stellt genau diese Entscheidung infrage: "Braucht
der direkte `$5A0`-Pfad + `Setexc(0x5A0/4,…)` trotzdem eine
Low-Memory-Sichtbarkeit für geportete Apps?" — TeraDesk selbst nutzt
exakt diesen Pfad (`sdk/apps/teradesk/library/utility/cookie.c:28,56`, per `_cookie-jar.md` bereits
zitiert). Diese Researcher-Pass **entscheidet das nicht** (das ist
Kern-Scope des BB1-Audits); siehe die entsprechende Korrekturnotiz in
`docs/api/bios/_cookie-jar.md`.

---

## 7. Vollständige Sysvar-Tabelle `$380`–`$5B4` (aus `tosvars.ld`, vollständig gelesen)

Jede Zeile zitiert `tosvars.ld` als Adress-Autorität. Spalte **"C-Backing"**:
`ja` = hat einen `extern`-Eintrag in `include/tosvars.h` (also normalen
C-Zugriff); `nur ASM` = wird nur direkt per `symbol.w`-Adressierung in
`.S`-Dateien gelesen/geschrieben (kein `tosvars.h`-Eintrag); `keins
gefunden` = im gesamten `references/emutos/`-Baum (außer der
Linker-Reservierung selbst) keine Referenz gefunden — reine
Adress-Reservierung ohne aktives EmuTOS-Verhalten. Spalte **"wer
schreibt (Boot)"** zitiert den in dieser Pass verifizierten Schreiber;
wo nicht tiefer verifiziert, ehrlich als solches markiert statt geraten.

### 7a. Crash-Dump-Bereich `$380`–`$3FF` (Kontext, außerhalb der Kernanfrage)

| Adresse | Symbol | Bedeutung (`tosvars.ld`) | Hinweis |
|---|---|---|---|
| `0x380` | `proc_lives` | validiert Crash-Seite, falls `0x12345678` | überlebt Reset (nicht im §2-Clear) |
| `0x384` | `proc_dregs` | gesicherte D0-D7 | dto. |
| `0x3a4` | `proc_aregs` | gesicherte A0-A7 | dto. |
| `0x3c4` | `proc_enum` | Vektornummer der Crash-Exception | dto. |
| `0x3c8` | `proc_usp` | gesicherter User-SP | dto. |
| `0x3cc` | `proc_stk` | 16 Words vom Exception-Stack | dto. |

Alle sechs haben `tosvars.h`-C-Backing; Details liegen außerhalb des
BIOS-Trap-13-Scopes (Crash-/Panic-Pfad, nicht dieser Auftrag).

### 7b. GEM-Event-Vektoren `$400`–`$40F` (bereits in `_state.md` §1 dokumentiert)

| Adresse | Symbol | `Setexc`-Nummer | Boot-Default | Quelle |
|---|---|---|---|---|
| `0x400` | `etv_timer` | `0x100` | `just_rts` (No-Op) | `bios/bios.c:360` |
| `0x404` | `etv_critic` | `0x101` | `default_etv_critic` (gibt Fehlercode unverändert zurück) | `bios/bios.c:361`, `bios/vectors.S:823-826` |
| `0x408` | `etv_term` | `0x102` | `just_rts` | `bios/bios.c:362` |
| `0x40c` | `etv_xtra` | `0x103` | — (nie geschrieben, reine Reservierung) | `tosvars.ld:36` |

### 7c. `$420`–`$5B4` (Kernbereich dieser Anfrage)

| Adresse | Symbol | Typ | C-Backing | Bedeutung (`tosvars.ld`) | Boot-Init | Wer liest (verifiziert) |
|---|---|---|---|---|---|---|
| `0x420` | `memvalid` | LONG | ja | RAM-Konfig valide, wenn `0x752019f3` | §2 (Cold: gesetzt am Ende von `memconf`; Warm: unverändert geprüft) | `memchk` (memory.S:126,135) |
| `0x424` | `memctrl` | BYTE | nur ASM | Kopie von `0xffff8001` (ST-MMU-Bank-Reg.) | §2 | `memchk`/`memconf` (memory.S:146,170,572) |
| `0x426` | `resvalid` | LONG | nein (nur ASM) | validiert `resvector`, wenn `0x31415926` | **nie von EmuTOS gesetzt**; nur `panicasm.S:115` löscht es | `bios/startup.S:504-508` (§5) |
| `0x42a` | `resvector` | LONG | nein (nur ASM) | Reset-Vektor (App-Konvention) | nie von EmuTOS gesetzt | `bios/startup.S:510-519` (§5) |
| `0x42e` | `phystop` | LONG(Ptr) | ja | physisches Top des ST-RAM | §2 | `bmem_init` (`memtop=phystop`, biosmem.c:72) |
| `0x432` | `membot` | LONG(Ptr) | ja | Start der TPA | `bmem_init`: `membot=end_os` (biosmem.c:67) | `Getmpb` (`00-getmpb.md`) |
| `0x436` | `memtop` | LONG(Ptr) | ja | Ende der TPA | `bmem_init` (biosmem.c:72), dann laufend durch `balloc_stram` | `Getmpb` |
| `0x43a` | `memval2` | LONG | ja | validiert memcntlr/memconf, wenn `0x237698aa` | §2 | `memchk` (memory.S:129,137) |
| `0x43e` | `flock` | WORD | ja | ≠0 sperrt VBL-Floppy-Routine | zur Laufzeit (`clock_init`/ACSI/SCSI-Zugriff, `clock.c:168,187`) | Floppy-VBL-Routine |
| `0x440` | `seekrate` | WORD | ja | Floppy-Seek-Rate | `floppy_init`: `DD_STEPRATE_3MS` (floppy.c:309) | `Floprate`-Familie (floppy.c:294) |
| `0x442` | `timer_ms` | WORD | ja | ms zwischen Timer-Aufrufen | `mfp_init`: `= 20` (mfp.c:196) | Timer-Interna |
| `0x444` | `fverify` | WORD | ja | ≠0: Floppy-Writes verifizieren | `floppy_init`: `= 0xff` (floppy.c:308) | Floppy-Write-Pfad (floppy.c:1200) |
| `0x446` | `bootdev` | WORD | ja | Standard-Bootlaufwerk | `biosmain` (bios.c:798-799, §1) | `Dsetdrv`, `init_default_environment` |
| `0x448` | `palmode` | WORD | **keins gefunden** | 0=NTSC, sonst PAL | reine Reservierung — kein aktiver Schreiber/Leser über die lokale `BOOL palmode`-Variable in `screen.c:607` hinaus (die ist ein **anderer**, lokaler C-Bezeichner, keine Sysvar-Referenz) | — |
| `0x44a` | `defshiftmod` | WORD | ja | Standard-Videoauflösung | screen-Init | Screen-Interna |
| `0x44c` | `sshiftmod` | WORD | ja | Kopie von `0xffff8260` (Shifter-Modus) | screen-Init (screen.c:556-580) | `vsetmode`/`Getrez` |
| `0x44e` | `v_bas_ad` | LONG(Ptr) | ja | Bildschirm-Basisadresse | `screen_init_address` (screen.c:656) | `conout.c` (Zeichenausgabe) |
| `0x452` | `vblsem` | WORD | ja | >0: VBL-Routine aktiv | `bios_init` Ende: `=1` (bios.c:553, §1) | VBL-Dispatcher |
| `0x454` | `nvbls` | WORD | ja | Anzahl VBL-Slots | `bios_init`: `=ARRAY_SIZE(vbl_list)` (bios.c:366, §3) | VBL-Dispatcher |
| `0x456` | `vblqueue` | LONG(Ptr) | ja | Zeiger auf VBL-Routinen-Array | `bios_init`: `=vbl_list` (bios.c:367, §3) | VBL-Dispatcher |
| `0x45a` | `colorptr` | LONG(Ptr) | ja | zu ladende Palette | Screen/Videl (screen.c:1182) | VBL-Palette-Handler |
| `0x45e` | `screenpt` | LONG(Ptr) | ja | Video-RAM für nächsten VBL | Screen-Umschaltung | VBL-Handler |
| `0x462` | `vbclock` | LONG | **keins gefunden** | Anzahl ausgeführter VBL-Routinen | reine Reservierung, kein Treffer über `tosvars.h`/`tosvars.ld` hinaus | — |
| `0x466` | `frclock` | LONG | ja | Anzahl verarbeiteter VBL-Routinen | VBL-Dispatcher | `Random` (XBIOS), Zeitmessung |
| `0x46a` | `hdv_init` | LONG(Vec) | ja | Vektor Festplatten-Init | `blkdev_init`: `=flop_hdv_init` (blkdev.c:98) | nicht von EmuTOS selbst aufgerufen (Kompatibilitäts-Reservierung, `blkdev.c:97` Kommentar) |
| `0x46e` | `swv_vec` | LONG(Vec) | ja | Auflösungswechsel-Vektor | `bios_init`: `=just_rts` (bios.c:363, §3), **überschrieben** am Ende von `bios_init`: `=os_header.reseth` (bios.c:552) | Monitor-Wechsel-ISR |
| `0x472` | `hdv_bpb` | LONG(Vec) | ja | Get-BPB-Vektor | `blkdev_init`: `=blkdev_getbpb` (blkdev.c:101) | `Getbpb` (`07-getbpb.md`) |
| `0x476` | `hdv_rw` | LONG(Vec) | ja | Read/Write-Vektor | `blkdev_init`: `=blkdev_rwabs` (blkdev.c:102) | `Rwabs` (`04-rwabs.md`) |
| `0x47a` | `hdv_boot` | LONG(Vec) | ja | Festplatten-Boot-Vektor | boot-abhängig gesetzt | `blkdev_boot` (bios.c:825) |
| `0x47e` | `hdv_mediach` | LONG(Vec) | ja | Media-Change-Vektor | `blkdev_init`: `=blkdev_mediach` (blkdev.c:103) | `Mediach` (`09-mediach.md`) |
| `0x482` | `cmdload` | WORD | ja | ≠0: COMMAND.PRG statt ROM-Shell laden | Boot-Konfiguration (0 im Standardfall) | `biosmain` (bios.c:850, §1) |
| `0x484` | `conterm` | BYTE | ja | Konsolen-Attribut-Bits (bit3 = Shift-Fold in `Bconin(2)`) | `= 7` (ikbd.c:1100) | `Bconin` (`02-bconin.md`) |
| `0x486` | `trp14ret` | LONG | **keins gefunden** | Rückkehradresse Trap#14 (laut Kommentar "unused") | reine, explizit als unbenutzt dokumentierte Reservierung | — |
| `0x48a` | `criticret` | LONG | **keins gefunden** | Rückkehradresse Critical-Error-Handler | reine Reservierung, kein aktiver Referenzpunkt gefunden | — |
| `0x48e` | `themd` | MD (16 B) | ja | erster Memory-Descriptor-Block | `Getmpb` (biosmem.c:144-147) | `Getmpb` (`00-getmpb.md`) |
| `0x49e` | `____md` | (16 B) | **keins gefunden** | Platz für weitere MDs (Reservierung) | nie beschrieben (EmuTOS nutzt nur `themd`) | — |
| `0x4a2` | `savptr` | LONG(Ptr) | ja | BIOS-Register-Sicherungsbereich-SP | `bios_init`: `=trap_save_area` (bios.c:359, §3) | Trap-Trampolin (`_dispatcher.md`) |
| `0x4a6` | `nflops` | WORD | ja | Anzahl erkannter Floppy-Laufwerke | `floppy_init` (floppy.c:319,386) | Floppy-Boot-Logik |
| `0x4a8` | `con_state` | LONG(Vec) | ja | Zustand der Konsolen-Ausgabe-Zustandsmaschine (VT52) | `vt52_init()`: `=normal_ascii` (vt52.c:921,950) | `bconout2`-VT52-Parser (vt52.c:153-264) |
| `0x4ac` | `save_row` | WORD | ja | temp. Zeilenposition (VT52-Escape-Y) | VT52-Parser | VT52-Parser |
| `0x4ae` | `sav_context` | LONG(Ptr) | **keins gefunden** | Zeiger auf Sicherungsbereich für Exception-Verarbeitung | reine Reservierung, kein Treffer | — |
| `0x4b2` | `bufl` | LONG[2] | ja | Zeiger auf GEMDOS-Puffer-Kontrollblöcke (FAT/Dir) | GEMDOS-Init (`bdos/fsbuf.c:67,71`) | GEMDOS-FAT-Cache (BDOS, außerhalb BIOS-Scope) |
| `0x4ba` | `hz_200` | LONG (volatile) | ja | 200-Hz-Systemuhr-Zähler | `init_system_timer` | `Tickcal`-Umfeld (`06-tickcal.md`) |
| `0x4be` | `the_env` | LONG(Ptr) | **keins gefunden** | Zeiger auf Standard-Environment-String | reine Reservierung, kein Treffer (EmuTOS nutzt stattdessen `default_env`, eine lokale Static in `bios/bios.c:80`, **nicht** diese Sysvar) | — |
| `0x4c2` | `drvbits` | LONG | ja | Bit-Zuordnung physischer Laufwerke | Boot-Partitions-Scan (`blkdev_init`) | `Drvmap` (`0a-drvmap.md`) |
| `0x4c6` | `dskbufp` | LONG(Ptr) | ja | Zeiger auf Disk-Puffer | `blkdev_init`: `=dskbuf` (blkdev.c:91) | `Getbpb`, `Rwabs`-Interna |
| `0x4ca` | `autopath` | LONG(Ptr) | **keins gefunden** | Zeiger auf AUTO-Ausführungspfad | reine Reservierung, kein Treffer (`autoexec()` in `bios/bios.c:846` iteriert das AUTO-Verzeichnis direkt, nicht über diese Sysvar) | — |
| `0x4ce` | `vbl_list` (8×LONG) | LONG[8] | ja (als `vbl_list[]`) | 8 VBL-Routinen-Zeiger | `bios_init`: alle `NULL` (bios.c:368-373, §3) | VBL-Dispatcher |
| `0x4ee` | `dumpflg` | WORD | ja | Screen-Dump-Flag | `bios_init`: `=-1` (bios.c:357, §3) | `Scrdmp` (XBIOS, außerhalb Scope) |
| `0x4f0` | `prtabt` | WORD | **keins gefunden** | Drucker-Abbruch-Flag | reine Reservierung, kein Treffer über `tosvars.h`/`.ld` hinaus | — |
| `0x4f2` | `sysbase` | LONG(Ptr) | ja | Zeiger auf OSHEADER-Start | `bios_init`: `=&os_header` (bios.c:358, §3/§4) | **jedes** Programm, das `_sysbase` liest (TeraDesk-Crash-Ursache) |
| `0x4f6` | `shell_p` | LONG(Ptr) | **keins gefunden** | Zeiger auf Shell | reine Reservierung, kein Treffer | — |
| `0x4fa` | `end_os` | LONG(Ptr) | ja | Ende des OS (statisch) | `bmem_init`: `=_end_os_stram`, dann bei jedem `balloc_stram`-Bottom-Alloc aktualisiert (biosmem.c:66,125) | `Getmpb`-Umfeld |
| `0x4fe` | `exec_os` | LONG(Ptr) | ja | Einstiegspunkt des OS (AES/Shell) | `bios_init`: `=os_header.os_magic->gm_init` (bios.c:534) | `biosmain` (bios.c:856-865, §1) |
| `0x502` | `dump_vec` | LONG(Vec) | ja | Screen-Dump-Routine | `chardev_init`: `=just_rts` | `Scrdmp` (außerhalb Scope) |
| `0x506` | `prt_stat` | LONG(Vec) | ja | Zeiger auf `prv_lsto` | `chardev_init`: `=just_rts` | Drucker-Status-Umleitung |
| `0x50a` | `prt_vec` | LONG(Vec) | ja | Zeiger auf `prv_lst` | `chardev_init`: `=just_rts` | Drucker-Ausgabe-Umleitung |
| `0x50e` | `aux_stat` | LONG(Vec) | ja | Zeiger auf `prv_auxo` | `chardev_init`: `=just_rts` | AUX-Status-Umleitung |
| `0x512` | `aux_vec` | LONG(Vec) | ja | Zeiger auf `prv_aux` | `chardev_init`: `=just_rts` | AUX-Ausgabe-Umleitung |
| `0x516` | `pun_ptr` | LONG(Ptr) | nein (kein `tosvars.h`-Eintrag, aber real referenziert) | falls AHDI: Zeiger auf `pun_info` | `blkdev_init`: `=&pun_info` (blkdev.c:161) | `Xhdi`/`ahdi_version` (xhdi.c:214) |
| `0x51a` | `memval3` | LONG | ja | validiert Speicherkonfig, wenn `0x5555aaaa` (TOS≥1.2) | §2 | `memchk` (memory.S:132,139) |
| `0x51e` | `bconstat_vec` (8×LONG) | LONG[8] | ja | Input-Status-Routinen | `chardev_init` (chardev.c) | `Bconstat` (`01-bconstat.md`) |
| `0x53e` | `bconin_vec` (8×LONG) | LONG[8] | ja | Input-Routinen | `chardev_init` | `Bconin` (`02-bconin.md`) |
| `0x55e` | `bcostat_vec` (8×LONG) | LONG[8] | ja | Output-Status-Routinen | `chardev_init` | `Bcostat` (`08-bcostat.md`) |
| `0x57e` | `bconout_vec` (8×LONG) | LONG[8] | ja | Output-Routinen | `chardev_init` | `Bconout` (`03-bconout.md`) |
| `0x59e` | `longframe` | LONG | ja | ≠0: nicht 68000, lange Stack-Frames | `processor_init` | Trap-Trampolin (`_dispatcher.md`) |
| `0x5a0` | `p_cookies` | LONG(Ptr) | ja | Zeiger auf Cookie-Jar | `cookie_init` (§6) | Cookie-Jar-Scan (`_cookie-jar.md`) |
| `0x5a4` | `ramtop` | LONG(Ptr) | ja | Top des TT-RAM, oder `NULL` | `memory2.c:detect_ttram` bzw. aus §2-Restore | Alt-RAM-Erkennung |
| `0x5a8` | `ramvalid` | LONG | ja | `RAMVALID_MAGIC`, falls `ramtop` gültig | dto. | dto. |
| `0x5ac` | `bell_hook` | LONG(Vec) | ja | Systemklingel-Routine | `snd_init`: `=do_bell` (sound.c:90) | `Bconout`-Klingel (`\a`-Zeichen) |
| `0x5b0` | `kcl_hook` | LONG(Vec) | ja | Tastenklick-Routine | `snd_init`: `=do_keyclick` (sound.c:91) | Tastatur-ISR |

**Vollständigkeits-Hinweis:** die als "keins gefunden" markierten Einträge
(`palmode`, `vbclock`, `trp14ret`, `criticret`, `____md`, `sav_context`,
`the_env`, `autopath`, `prtabt`, `shell_p`) sind ehrlich als **reine
Adress-Reservierungen ohne aktives EmuTOS-Backing** dokumentiert — per
Grep über den kompletten `references/emutos/`-Baum verifiziert (keine
Fundstelle außer `tosvars.ld`/vereinzelt `tosvars.h`). Das ist ein
legitimer Befund, kein offener Punkt: diese Adressen existieren nur, damit
das klassische TOS-Sysvar-Layout kompatibel bleibt, falls ein Programm sie
pokt/peekt — EmuTOS selbst tut nichts damit. Für NewTOS heißt das: ein
1:1-Nachbau dieser Adressen kann sie als "immer 0 / nie gelesen" behandeln,
**es sei denn** eine geportete App liest sie aktiv (dann ist der Wert `0`
der korrekte Cold-Boot-Wert nachzubilden).

---

## 8. Zusammenfassung für den nachfolgenden BB1-Auditor

1. **Zwei-Welten-Annahme widerlegt** (bereits im Gap-Doc festgestellt,
   hier mit vollständigem Boot-Ablauf-Beleg bestätigt): jede der oben
   dokumentierten Sysvars + die OSHEADER-Struktur existieren, bevor
   irgendein Programm (nativ oder m68k) startet — es gibt keinen
   architektonischen Grund, sie nur für den m68k-Emulator anzulegen.
2. **`_sysbase`/OSHEADER ist der Blocker** — ohne eine reale,
   kernel-befüllte OSHEADER-Instanz (Felder exakt wie §4) + eine
   `sysbase`-Sysvar, die ein natives EL0-Programm dereferenzieren kann,
   crasht jedes 1:1-portierte TOS-Programm, das den klassischen
   Versions-Check macht (TeraDesk `sdk/apps/teradesk/main.c:56`, vermutlich auch QED).
3. **`os_version`/`GEMDOS_VERSION`-Zielwert ist offen** (§4) — vor dem
   Festlegen gegen Compendium/Profibuch/reales TOS4 verifizieren, nicht
   den `00-PLAN.md`-Wert `0x0404` ungeprüft übernehmen.
4. **Cookie-Jar-Zugriffsmechanismus ist ein ungelöster Konflikt**
   (ADR-012 vs. Gap-Doc, §6) — braucht eine BB1-Entscheidung, kein
   Researcher-Entscheid.
5. **Die als "kein Backing" markierten Sysvars** (§7-Tabelle) sind für
   den 1:1-Nachbau niedrige Priorität — sie werden von EmuTOS selbst nie
   beschrieben/gelesen; nur relevant, falls eine konkrete geportete App
   sie direkt anfasst (dann jeweils einzeln nachprüfen).
6. **Layout-Frage aus dem Gap-Doc (Punkt 2, "wo liegt die Sysvar-Area in
   der EL0-VA") ist eine NewTOS-Architekturfrage**, keine EmuTOS-Spec-
   Frage — EmuTOS legt diese Adressen absolut in physischem Low-Memory
   ab, was auf AArch64+MMU keine direkte Entsprechung hat. Das ist
   Kernscope des BB1-Audits (Entwickler-Entscheidung), nicht dieser
   Spec-Extraktion.

## Cross-references

- `docs/analysis/sysvar-osheader-lowmem-gap.md` — Auslöser
- `docs/analysis/bios-audit/00-PLAN.md` — Auftrag (BB1)
- `docs/api/structures/osheader.md` — OSHEADER-Feldlayout (validiert)
- `docs/api/bios/_cookie-jar.md` — Cookie-Jar-Datenmodell + Zugriffspfad
- `docs/api/bios/_state.md` — Trap-Ebene "was ein Programm beobachten
  kann" (kuratierte Teilmenge; dieses Dokument ist die vollständige
  `tosvars.ld`-Deckung + der zeitliche Boot-Ablauf)
- `docs/api/bios/04-rwabs.md` — `etv_critic`-Laufzeitpfad bei Disk-Fehlern
- `docs/analysis/aes-audit/emutos-config-profile.md` — `TOS_VERSION`/
  `GEMDOS_VERSION`-Default-Werte + das offene NewTOS-Zielwert-Problem
