# GEMDOS — vollständiges Funktions- und Unterfunktionsinventar

**Stand:** 2026-09-14 · **Quelle:** `references/emutos` @
`1a7484aba1bfba23e2c0899e243b6d8177fe41df` (mit `git -C references/emutos log -1`
bestätigt: „Update GitHub Actions to use Node 24") · **Zielprofil:** `make 512`,
Falcon030/TOS4 · **NewTOS-Seite:** HEAD `19871c0`.

**Zweck.** Die Liste, die sagt „das sind alle". Grundlage für ein
Laufzeit-Prüfprogramm, dessen Vollständigkeit ohne sie nicht überprüfbar ist.
Formvorlage: `docs/api/aes/_full-function-inventory.md`.

---

## 0. Methode und Abgrenzung

**Die Liste kommt aus der Dispatcher-Tabelle, nicht aus Kopfdateien.**
Maßgeblich ist `bdos/bdosmain.c`:

* die Funktionstabelle `funcs[]` (`:142-290`),
* `#define MAX_FNCALL (ARRAY_SIZE(funcs) - 1)` (`:291`),
* der Verteiler `osif()` (`:431-705`), Bereichsprüfung `:444-445`.

Ein nicht belegter Platz trägt `NI` = `F(ni)` (`:145`), und `ni()` ist
`return EINVFN;` (`bdos/bdosmain.c:307-310`). **Ein `NI`-Platz ist also kein
Loch, sondern ein definierter Fehlerweg.** Ebenso eine Nummer > `MAX_FNCALL`:
`osif` gibt vor jedem Tabellenzugriff `EINVFN` zurück (`:444-445`).

Die Tabelle hat **88 Plätze** (0x00…0x57). Davon sind **53 belegt** und
**35 `NI`**.

### Profilgatter — nachgeprüft für `make 512`

`TARGET_512` wird in `include/config.h` nirgends ausgewertet (das Makefile setzt
nur `-DTARGET_512`, `Makefile:561`); die Abmagerungsblöcke `#ifdef TARGET_192`
usw. greifen daher **nicht**. Mit dem Präprozessor gegen `include/config.h`
(`cc -E -dM -Iinclude -DTARGET_512`) nachgemessen:

| Schalter | Wert | GEMDOS-Folge |
|---|---|---|
| `CONF_WITH_ALT_RAM` | **1** | 0x14 `Maddalt` existiert (`bdosmain.c:194-198`) |
| `CONF_WITH_VIDEL` | **1** | 0x15 `Srealloc` existiert (`bdosmain.c:200-204`) |
| `GEMDOS_VERSION` | **0x2000** | Rückgabe von 0x30 `Sversion` |

Beide gatterbehafteten Funktionen sind im Zielprofil also **vorhanden**; es gibt
in GEMDOS **keine** Funktion, die im Zielprofil abgeschaltet wäre.

### Was „verdrahtet" heißt

Der **behandelnde Zweig** zählt, nicht der Name. Spalte **N**:

* **ja** — NewTOS führt eine echte Implementierung aus.
* **HW-Adapter** — der Arm existiert, liefert aber einen festen Wert, weil die
  Atari-Peripherie fehlt (AUX/ACIA, Drucker). Faktisch festgehalten, **nicht**
  bewertet.
* **Konstante** — fester Wert ohne Hardware-Begründung (Regel-2-Kandidat).
* **nein** — kein Arm im NewTOS-Verteiler; fällt auf `EINVFN`.

---

## 1. Opcode-Ebene: 88 Tabellenplätze, 53 belegt

NewTOS-Verteiler: `tos-api/src/gemdos/mod.rs::build_table` (`:45-112`, 48 Arme)
plus die **rahmenverändernden** Funktionen, die der Kernel vor der Tabelle
abfängt: `kernel/src/arch/aarch64/exceptions.rs:400` (0x4C/0x00/0x31), `:404`
(0x4B), `:408` (0x4D), `:417` (0x11A).

### 1.1 Belegte Plätze (53)

| Nr | Name | EmuTOS Tabelle | EmuTOS Rumpf | N | NewTOS |
|---|---|---|---|---|---|
| 0x00 | `Pterm0` | `bdosmain.c:147` | `bdos/proc.c:588` | ja | `exceptions.rs:400` → `handle_pterm` |
| 0x01 | `Cconin` | `:156` | `bdos/console.c:396` | ja | `gemdos/mod.rs:47` |
| 0x02 | `Cconout` | `:157` | `bdos/console.c:279` | ja | `:48` |
| 0x03 | `Cauxin` | `:158` | `bdos/console.c:432` | HW-Adapter | `:49` → `bios::chardev::bconin_handle(1)` |
| 0x04 | `Cauxout` | `:159` | `bdos/console.c:331` | HW-Adapter | `:50` |
| 0x05 | `Cprnout` | `:160` | `bdos/console.c:340` | Konstante `0` | `:51`, (cite stale 2026-10-26: console.rs:144-146, a constant-returning stub) |
| 0x06 | `Crawio` | `:161` | `bdos/console.c:441` | ja | `:52` |
| 0x07 | `Crawcin` | `:162` | `bdos/console.c:375` | ja | `:53` |
| 0x08 | `Cnecin` | `:163` | `bdos/console.c:414` | ja | `:54` |
| 0x09 | `Cconws` | `:164` | `bdos/console.c:459` | ja | `:55` |
| 0x0A | `Cconrs` | `:165` | `bdos/console.c:533` | ja | `:56` |
| 0x0B | `Cconis` | `:166` | `bdos/console.c:159` | ja | `:57` |
| 0x0E | `Dsetdrv` | `:179` | `fsmain.c:420` | ja | `:102` |
| 0x10 | `Cconos` | `:189` | `bdos/console.c:168` | Konstante `-1` | `:58`, (cite stale 2026-10-26: console.rs:574-576, a constant-returning stub) |
| 0x11 | `Cprnos` | `:190` | `bdos/console.c:177` | Konstante `0` | `:59`, (cite stale 2026-10-26: console.rs:583-585, a constant-returning stub) |
| 0x12 | `Cauxis` | `:191` | `bdos/console.c:186` | Konstante `0` | `:60`, (cite stale 2026-10-26: console.rs:592-594, a constant-returning stub) |
| 0x13 | `Cauxos` | `:192` | `bdos/console.c:195` | Konstante `0` | `:61`, (cite stale 2026-10-26: console.rs:601-603, a constant-returning stub) |
| 0x14 | `Maddalt` | `:195` (`CONF_WITH_ALT_RAM`) | `umem.c:455` | ja | `:66` |
| 0x15 | `Srealloc` | `:201` (`CONF_WITH_VIDEL`) | `umem.c:358` | **nein** | — |
| 0x19 | `Dgetdrv` | `:210` | `fsmain.c:438` | ja | `:103` |
| 0x1A | `Fsetdta` | `:211` | `fsmain.c:403` | ja | `:96` |
| 0x2A | `Tgetdate` | `:232` | `bdos/time.c:99` | ja | `:73` |
| 0x2B | `Tsetdate` | `:233` | `bdos/time.c:108` | ja | `:74` |
| 0x2C | `Tgettime` | `:234` | `bdos/time.c:132` | ja | `:75` |
| 0x2D | `Tsettime` | `:235` | `bdos/time.c:142` | ja | `:76` |
| 0x2F | `Fgetdta` | `:239` | `fsmain.c:394` | ja | `:97` |
| 0x30 | `Sversion` | `:240` | `bdosmain.c:298` | ja (anderer Wert) | `:77`, `version.rs:14-19` |
| 0x31 | `Ptermres` | `:241` | `bdos/proc.c:624` | ja | `exceptions.rs:400` |
| 0x36 | `Dfree` | `:248` | `fsfat.c:337` | ja | `:104` |
| 0x39 | `Dcreate` | `:253` | `fsdir.c:201` | ja | `:107` |
| 0x3A | `Ddelete` | `:254` | `fsdir.c:297` | ja | `:108` |
| 0x3B | `Dsetpath` | `:255` | `fsdir.c:1113` | ja | `:109` |
| 0x3C | `Fcreate` | `:256` | `fsopnclo.c:81` | ja | `:88` |
| 0x3D | `Fopen` | `:257` | `fsopnclo.c:221` | ja | `:80` |
| 0x3E | `Fclose` | `:258` | `fsopnclo.c:426` | ja | `:81` |
| 0x3F | `Fread` | `:259` | `fsio.c:440` | ja | `:82` |
| 0x40 | `Fwrite` | `:260` | `fsio.c:528` | ja | `:89` |
| 0x41 | `Fdelete` | `:261` | `fsopnclo.c:623` | ja | `:90` |
| 0x42 | `Fseek` | `:262` | `fsio.c:321` | ja | `:83` |
| 0x43 | `Fattrib` | `:263` | `fsdir.c:402` | ja | `:91` |
| 0x44 | `Mxalloc` | `:264` | `umem.c:234` | ja | `:67` |
| 0x45 | `Fdup` | `:265` | `fshand.c:93` | ja | `:84` |
| 0x46 | `Fforce` | `:266` | `fshand.c:32` | ja | `:85` |
| 0x47 | `Dgetpath` | `:267` | `fsdir.c:1229` | ja | `:110` |
| 0x48 | `Malloc` | `:268` | `umem.c:116` | ja | `:68` |
| 0x49 | `Mfree` | `:269` | `umem.c:135` | ja | `:69` |
| 0x4A | `Mshrink` | `:270` | `umem.c:170` | ja | `:70` |
| 0x4B | `Pexec` | `:271` | `bdos/proc.c:179` | teilweise (s. §2.6) | `exceptions.rs:404` |
| 0x4C | `Pterm` | `:272` | `bdos/proc.c:601` | ja | `exceptions.rs:400` |
| 0x4E | `Fsfirst` | `:276` | `fsdir.c:523` | ja | `:98` |
| 0x4F | `Fsnext` | `:277` | `fsdir.c:654` | ja | `:99` |
| 0x56 | `Frename` | `:286` | `fsdir.c:895` | ja | `:92` |
| 0x57 | `Fdatime` | `:287` | `fsdir.c:684` | ja | `:93` |

### 1.2 Die 35 `NI`-Plätze (Fehlerweg `EINVFN`)

`0x0C, 0x0D, 0x0F, 0x16, 0x17, 0x18, 0x1B–0x1F, 0x20–0x29, 0x2E, 0x32–0x35,
0x37, 0x38, 0x4D, 0x50–0x55`

Alle mit `{ NI, 0, 0 }` in `bdosmain.c:176-285`. Alle geben `EINVFN` (−32)
zurück. NewTOS: `TABLE[id] == None` → `EINVFN` (`gemdos/mod.rs:152-153`) —
**mit einer Ausnahme:** 0x4D (siehe §1.4).

### 1.3 Nummern > `MAX_FNCALL` (0x58 und höher)

EmuTOS: `if (fn > MAX_FNCALL) return EINVFN;` (`bdosmain.c:444-445`).
NewTOS: `if id > MAX_FN { return EINVFN }` (`gemdos/mod.rs:149-151`) — nach den
Sonderfällen unten.

### 1.4 NewTOS-Erweiterungen jenseits der EmuTOS-Tabelle

| Nr | Name | EmuTOS | NewTOS | Beleg |
|---|---|---|---|---|
| 0x4D | `Pwait` | **`NI` → `EINVFN`** (`bdosmain.c:274`) | eigener rahmenverändernder Zweig | `exceptions.rs:309` |
| 0x11A | `Psigreturn` | existiert nicht | EL0-Upcall-Rücksprung | `exceptions.rs:318` |
| 0x154 | `Ssystem` | **existiert im gepinnten Baum nicht** (kein Treffer für `Ssystem`/`ssystem` in `references/emutos`) | Kekssubfunktionen 8/9/26 | `gemdos/mod.rs:145-147`, `ssystem.rs:16` |

> **Quelle mehrdeutig.** `docs/api/gemdos/154-ssystem.md` ist **nicht** aus
> EmuTOS ableitbar; Ssystem ist eine FreeMiNT-Erweiterung. Der Beleg müsste aus
> FreeMiNT kommen — `references/freemint` ist im Baum nicht vorhanden.

### 1.5 Summenzeile

| | Plätze | belegt | `NI` |
|---|---|---|---|
| EmuTOS `funcs[]` (0x00–0x57) | **88** | **53** | **35** |

| | Anzahl |
|---|---|
| EmuTOS belegt | **53** |
| davon in NewTOS verdrahtet | **52** |
| davon konstanter Rückgabewert (EmuTOS rechnet über `run->p_uft[]`) | **5** (0x05 `Cprnout`, 0x10 `Cconos`, 0x11 `Cprnos`, 0x12 `Cauxis`, 0x13 `Cauxos`) |
| davon Durchreichung auf BIOS-Handle 1 (AUX) | **2** (0x03 `Cauxin`, 0x04 `Cauxout`) |
| in NewTOS **nicht** verdrahtet | **1** (0x15 `Srealloc`) |
| unter einem bei uns abgeschalteten Schalter | **0** |
| NewTOS-Erweiterungen über EmuTOS hinaus | **3** (0x4D, 0x11A, 0x154) |

**Gegengezählt:** `build_table` hat 48 `t[…] = Some(…)`-Zeilen
(`gemdos/mod.rs:45-108`), der Kernel 4 EmuTOS-Funktionen als
rahmenverändernde Zweige (0x00, 0x31, 0x4B, 0x4C) = **52**.

---

## 2. Unterfunktionen

**Zählregel** (wie im AES-Inventar): als Unterfunktion zählt ein diskreter Wert
eines *Modus- oder Auswahlparameters*, auf den die Implementierung verzweigt —
nicht ein Datenwert (Länge, Zeiger, Koordinate). Jede Zeile steht auf dem
tatsächlich behandelnden Zweig.

### 2.1 `Fopen` (0x3D) — `mod`

`xopen` maskiert **zuerst**: `ixopen(name, mod & VALID_FOPEN_BITS)`
(`fsopnclo.c:225`), `VALID_FOPEN_BITS = MODE_FAC = 0x07` (`bdos/fs.h:169,173`).

| Wert | Name | Beleg |
|---|---|---|
| 0 | `RO_MODE` | `bdos/fs.h:170` |
| 1 | `WO_MODE` | `bdos/fs.h:171` |
| 2 | `RW_MODE` | `bdos/fs.h:172` |
| 3–7 | nicht benannt, **werden trotzdem gespeichert** (`makopn`: `p->o_mod = mod`, `fsopnclo.c:282`) | |
| Bits 4–6 (`MODE_FSM`) | file sharing — **weggeworfen** durch die Maske | `bdos/fs.h:168` |
| Bit 7 (`INH_MODE`) | inheritance — **weggeworfen** | `bdos/fs.h:167` |

Eigener Zweig: `if ((fcb->f_attrib & FA_RO) && (mod != 0)) return EACCDN;`
(`fsopnclo.c:260-261`) — **jeder** Modus ≠ 0 auf einer schreibgeschützten Datei.

→ 3 benannte + 5 unbenannte Werte + 1 Maskierungsklasse.

### 2.2 `Fseek` (0x42) — `flg`

`xlseek`, `fsio.c:329-334`:

| Wert | Bedeutung | Zweig |
|---|---|---|
| 0 | vom Dateianfang | (kein Zweig, Vorgabe) |
| 1 | von der aktuellen Position | `:331-332` |
| 2 | vom Dateiende | `:329-330` |
| sonst (auch negativ) | **`EINVFN`** | `:333-334` |

→ 3 Werte + 1 Rückfallklasse. Dazu `EIHNDL` bei ungültigem Handle (`:326-327`)
und `ERANGE` für `n < 0 || n > Dateilänge` (`ixlseek`, `fsio.c:360-361`).

### 2.3 `Fattrib` (0x43) — `wrt` × Attributbits

`xchmod`, `fsdir.c:402-435`:

| `wrt` | Bedeutung | Zweig |
|---|---|---|
| 0 | lesen (`ixread`) | `:427-428` |
| ≠ 0 | schreiben (`ixwrite` + `ixclose(CL_DIR)`) | `:429-433` |

Attributbits `mod` (`include/bdosdefs.h:36-41`):

| Bit | Name | in `FA_NORM`? |
|---|---|---|
| 0x01 | `FA_RO` | ja |
| 0x02 | `FA_HIDDEN` | ja |
| 0x04 | `FA_SYSTEM` | ja |
| 0x08 | `FA_VOL` | **nein** |
| 0x10 | `FA_SUBDIR` | **nein** |
| 0x20 | `FA_ARCHIVE` | ja |

`FA_NORM = FA_ARCHIVE|FA_SYSTEM|FA_HIDDEN|FA_RO = 0x27` (`bdos/fs.h:195`).
Beim Setzen: `if (wrt && (mod & ~FA_NORM)) return EACCDN;` (`:421-422`).

> **Anmerkung zur Version, nicht zur Bewertung.** Der Quellkommentar
> (`fsdir.c:391-401`) hält ausdrücklich fest: EmuTOS verhält sich wie
> **TOS 1–3**, nicht wie TOS4 — Ordner liefern `EFILNF`. Das Zielprofil ist
> laut `docs/MEMORY.md` TOS4. Das ist ein **Widerspruch in der Vorgabe**, kein
> Befund dieses Inventars.

→ 2 `wrt`-Werte × 6 Attributbits + 1 Fehlerklasse.

### 2.4 `Fdatime` (0x57) — `wrt`

`xgsdtof`, `fsdir.c:684-707`: `wrt ≠ 0` = setzen (`:694-698`), `0` = lesen
(`:700-704`). Kein `default`, immer `E_OK`; `EIHNDL` bei ungültigem Handle
(`:689-690`). → 2 Werte.

### 2.5 `Mxalloc` (0x44) — `mode`

`xmxalloc`, `umem.c:234-...`; **`mode &= MX_MODEMASK`** (`:241`),
`MX_MODEMASK = 0x03` (`include/bdosdefs.h:22`). Es gibt **zwei** Schalter: einen
für `amount == -1` (Größenabfrage, `:247-279`) und einen für die echte
Belegung (`:291-317`).

| Wert | Name | `amount == -1` | `amount > 0` |
|---|---|---|---|
| 0 | `MX_STRAM` | `ffit(-1,&pmd)` `:249-251` | `ffit(amount,&pmd)` `:293-295` |
| 1 | `MX_TTRAM` | `ffit(-1,&pmdalt)` `:253-255` (`CONF_WITH_ALT_RAM`) | `:297-299` |
| 2 | `MX_PREFSTRAM` | Maximum beider Pools `:257-273` | ST zuerst, dann TT `:301-307` |
| 3 | `MX_PREFTTRAM` | dito | TT zuerst, dann ST `:308-315` |
| sonst | — | **unerreichbar** (Maske 0x03) | **unerreichbar** |

**Der `default`-Zweig ist toter Code**: nach `mode &= 0x03` kann `mode` nur
0–3 sein. Die Bits 4–5 (`MX_SUPER`, `vdi_control.c:35`) werden von EmuTOS
ausdrücklich ignoriert. → 4 Werte × 2 Betriebsarten (`amount == -1` /
`amount > 0`) + die Klasse `amount <= 0` (`:284-287`, → `NULL`).

NewTOS: gleiche Maske und dieselben vier Zweige
(`tos-api/src/bdos/mem/traps.rs:263,282-284,321-331`).

### 2.6 `Pexec` (0x4B) — `flag`

`xexec`, `bdos/proc.c:179-254`. **Zwei Schalter**: der erste (`:192-254`) für alles,
was keine Datei lädt.

| Wert | Name (`include/bdosdefs.h:25-32`) | EmuTOS | NewTOS |
|---|---|---|---|
| 0 | `PE_LOADGO` | `:249-251` → Ladepfad | ja (`pexec.rs:172`) |
| 3 | `PE_LOAD` | `:250-251` → Ladepfad | ja (`pexec.rs:173`) |
| 4 | `PE_GO` | `:244-248` | ja (`pexec.rs:176`) |
| 5 | `PE_BASEPAGE` | `:210-237` | **nein** → `UnsupportedMode` (`pexec.rs:177-179`) |
| 6 | `PE_GOTHENFREE` | `:238-243` | **nein** → `UnsupportedMode` |
| 7 | `PE_BASEPAGEFLAGS` | `:213-237` | **nein** → `UnsupportedMode` |
| 50 | `PE_RELOCATE` | `:194-209`, `#if DETECT_NATIVE_FEATURES` — im Zielprofil **1**, also übersetzt | **nein** → `UnknownMode` |
| sonst | — | `EINVFN` (`:252-253`) | `UnknownMode` (`pexec.rs:180`) |

→ 7 Werte + 1 Rückfallklasse. **Drei der sieben sind in NewTOS ehrlich
unsupported, einer (50) fehlt.**

### 2.7 `Fsfirst` (0x4E) — `att`

`ixsfirst`, `fsdir.c:452-...`. Ein Sonderfall vor der Suche:
`if (att != FA_VOL) att |= (FA_ARCHIVE|FA_RO);` (`:459-460`).

| Klasse | Wirkung |
|---|---|
| `att == FA_VOL` (0x08) | Suche **nur** nach dem Datenträgernamen, ohne Zwangs-ODER |
| jeder andere Wert | `FA_ARCHIVE|FA_RO` (0x21) wird **erzwungen** hinzugefügt |

→ 2 Klassen + die 6 Attributbits aus §2.3 als Eingabe.

### 2.8 `Fdup` (0x45) / `Fforce` (0x46) — Standardhandles

`NUMSTD = 6` (`include/bdosdefs.h:61`).

| Funktion | gültiger Bereich | Fehlerweg |
|---|---|---|
| `Fdup(h)` | `0 ≤ h < 6` (`fshand.c:97-98`) | sonst `EIHNDL` |
| `Fforce(std,h)` | `0 ≤ std < 6` (`fshand.c:50-51`) | sonst `EIHNDL` |
| `Fforce` `h < 0` | BIOS-Handle, wird roh in `p_uft[std]` abgelegt (`:53-54`) | — |

→ 6 Standardhandles × 2 Funktionen, plus die Klasse „negatives Handle".

### 2.9 Die BIOS-Handles −1/−2/−3 in `Fread`/`Fwrite`

Kein Parameter der Funktion, aber ein **eigener Verteilerzweig** in `osif`
(`bdosmain.c:571-640`):

| Handle | Gerät | Beleg |
|---|---|---|
| −1 | CON | `bdosmain.c:595-596` (Kommentar „prn, aux, con / -3 -2 -1") |
| −2 | AUX | dito |
| −3 | PRN | dito |
| < −3 | **`EIHNDL`** | `:601-602` |

Für `Fread` zusätzlich: `if (pw[2]) return 0;` — eine Länge ≥ 64 KiB auf einem
Zeichengerät gibt 0 zurück (`:610-611`); `pw[3] == 1` nimmt `conin()`
(`:613-617`), sonst `cgets()`.

→ 3 Gerätehandles + 1 Fehlerklasse + 2 Längenklassen.

### 2.10 Die Standardhandle-Umleitung der Konsolfunktionen

`osif` (`bdosmain.c:498-569`) fängt **vor** dem Tabellensprung ab, wenn
`f->stdio_typ` gesetzt ist **und** `fn < 12` oder `16 ≤ fn ≤ 19` **und** das
`p_uft`-Handle > 0 ist (die Funktion ist also auf eine Datei umgeleitet).
Behandelte Fälle: 1, 2, 3, 4, 5, 6 (mit `pw[1] != 0xFF`-Sonderfall, `:505-507`),
7, 8, 9, 10, 11, 16, 17, 18, 19.

**Das ist eine echte Unterfunktionsebene** — jede dieser 15 Funktionen hat zwei
Verhalten je nachdem, ob `Fforce` sie umgeleitet hat.

NewTOS kennt diese Ebene nicht: `Cconos`/`Cprnos`/`Cauxis`/`Cauxos` sind feste
Werte (§1.1), und die übrigen Konsolfunktionen lesen `p_uft` nicht.

### 2.11 `Crawio` (0x06) — `w`

`xrawio`, `console.c:441-...`: `w == 0xFF` = lesen, sonst schreiben. Zusätzlich
der Umleitungszweig `case 6:` in `osif` (`:505-507`).
→ 2 Werte.

### 2.12 `Fcreate` (0x3C) — `attr`

`xcreat`: `ixcreat(name, attr & ~FA_SUBDIR)` (`fsopnclo.c:85`) — Bit 0x10 wird
**gelöscht**, ein Verzeichnis lässt sich so nicht anlegen. → die 6 Attributbits
aus §2.3, davon eines zwangsweise 0.

### 2.13 `Ssystem` (0x154, NewTOS-Erweiterung) — `mode`

Kein EmuTOS-Beleg. NewTOS (`tos-api/src/gemdos/ssystem.rs:19-21,31-41`):

| Wert | Name | NewTOS |
|---|---|---|
| −1 | Vorhandenseinsprobe | `E_OK` (`:32-34`) |
| 8 | `S_GETCOOKIE` | ja |
| 9 | `S_SETCOOKIE` | ja |
| 26 | `S_DELCOOKIE` | ja |
| sonst | — | `EINVFN` (`:40`) |

### 2.14 Funktionen **ohne** Unterfunktion

Für diese 40 belegten Nummern ist jeder Parameter ein Datenwert:

`Pterm0` 0x00 · `Cconin` 0x01 · `Cconout` 0x02 · `Cauxin` 0x03 ·
`Cauxout` 0x04 · `Cprnout` 0x05 · `Crawcin` 0x07 · `Cnecin` 0x08 ·
`Cconws` 0x09 · `Cconrs` 0x0A · `Cconis` 0x0B · `Dsetdrv` 0x0E ·
`Cconos` 0x10 · `Cprnos` 0x11 · `Cauxis` 0x12 · `Cauxos` 0x13 ·
`Maddalt` 0x14 · `Srealloc` 0x15 · `Dgetdrv` 0x19 · `Fsetdta` 0x1A ·
`Tgetdate` 0x2A · `Tsetdate` 0x2B · `Tgettime` 0x2C · `Tsettime` 0x2D ·
`Fgetdta` 0x2F · `Sversion` 0x30 · `Ptermres` 0x31 · `Dfree` 0x36 ·
`Dcreate` 0x39 · `Ddelete` 0x3A · `Dsetpath` 0x3B · `Fclose` 0x3E ·
`Fread` 0x3F · `Fwrite` 0x40 · `Fdelete` 0x41 · `Dgetpath` 0x47 ·
`Malloc` 0x48 · `Mfree` 0x49 · `Mshrink` 0x4A · `Pterm` 0x4C ·
`Fsnext` 0x4F · `Frename` 0x56.

(`Fread`/`Fwrite` tragen die Handle-Klassen aus §2.9, `Cconin`&Co. die
Umleitungsklasse aus §2.10 — beides sind Zustands-, keine Parameterebenen.)

### 2.15 Summe der Unterfunktionswerte

| Träger | Werte | Rückfallklassen |
|---|---|---|
| `Fopen` `mod` (3 benannt + 5 unbenannt) | 8 | 1 (Maske) |
| `Fseek` `flg` | 3 | 1 (`EINVFN`) |
| `Fattrib` `wrt` × Attributbits (2 + 6) | 8 | 1 (`EACCDN`) |
| `Fdatime` `wrt` | 2 | — |
| `Mxalloc` `mode` × 2 Betriebsarten | 8 | 1 (`amount ≤ 0`) |
| `Pexec` `flag` | 7 | 1 (`EINVFN`) |
| `Fsfirst` `att` (Klassen) | 2 | — |
| `Fdup`/`Fforce` Standardhandles (6 × 2) | 12 | 1 (`EIHNDL`) |
| `Fread`/`Fwrite` BIOS-Handles | 3 | 1 (`< −3`) |
| Konsolen-Umleitung (15 Funktionen × 2) | 30 | — |
| `Crawio` `w` | 2 | — |
| `Fcreate` `attr` | 6 | — |
| `Ssystem` `mode` (nur NewTOS) | 4 | 1 |
| **Summe** | **95** | **8** |

**Also: 53 belegte Nummern, davon 13 mit Unterfunktionen und 40 ohne, zusammen
95 diskrete Unterfunktionswerte + 8 Rückfallklassen + 2 Fehlerklassen der
Opcode-Ebene (`NI` → `EINVFN`; > `MAX_FNCALL` → `EINVFN`).**

---

## 3. Was eine vollständige Falltabelle abdecken muss

1. **Jede der 53 belegten Nummern** mindestens einmal im Gutfall.
2. **Jeden der 35 `NI`-Plätze** — alle müssen `EINVFN` liefern.
3. **Mindestens eine Nummer > 0x57** — ebenfalls `EINVFN`. Achtung: 0x4D,
   0x11A und 0x154 sind in NewTOS belegt und liefern dort **nicht** `EINVFN`.
4. **Jede Unterfunktion** aus Abschnitt 2.
5. **Die Fehlerwege:** ungültige Handles (`EIHNDL`), ungültige Laufwerke,
   `EACCDN` auf schreibgeschützten Dateien, `ERANGE` beim Suchen.

### Abschnitte für die Messtabelle

**A — frei messbar.** `Cconis`/`Cconos`/`Cprnos`/`Cauxis`/`Cauxos`,
`Dgetdrv`, `Dsetdrv`, `Dfree`, `Fgetdta`/`Fsetdta`, `Sversion`,
`Tgetdate`/`Tgettime`, `Fopen`…`Fdatime` (mit vorbereiteten Dateien),
`Fsfirst`/`Fsnext`, `Dgetpath`, `Malloc`/`Mfree`/`Mshrink`/`Mxalloc`,
alle `NI`-Nummern, Nummern > 0x57.

**B — blockiert auf Eingabe.** `Cconin` 0x01, `Cauxin` 0x03, `Crawcin` 0x07,
`Cnecin` 0x08, `Cconrs` 0x0A, `Crawio(0xFF)`.

**C — beendet oder verändert den Zustand irreversibel.** `Pterm0` 0x00,
`Ptermres` 0x31, `Pterm` 0x4C, `Pexec` 0x4B, `Fdelete` 0x41, `Ddelete` 0x3A,
`Frename` 0x56, `Tsetdate`/`Tsettime`, `Maddalt` 0x14, `Srealloc` 0x15
(Bildschirmspeicher!).

---

## 4. Belege

* Funktionstabelle: `bdos/bdosmain.c:142-290`, `MAX_FNCALL` `:291`
* `NI`/`ni()`: `bdosmain.c:145,307-310`
* Verteiler: `bdos/bdosmain.c:431-705`, Bereichsprüfung `:444-445`,
  Standardhandle-Umleitung `:498-569`, BIOS-Handles `:571-640`
* Trap-#2-Abzweig für `xterm`: `bdosmain.c:45-46,326`
* Modus-Konstanten: `include/bdosdefs.h:18-32,36-41,61`; `bdos/fs.h:167-173,195`
* Profilgatter: `include/config.h` (mit `cc -E -dM -Iinclude -DTARGET_512`
  ausgewertet); `Makefile:558-564`
* NewTOS-Verteiler: `tos-api/src/gemdos/mod.rs:43-156`
* NewTOS-Rahmenzweige: `kernel/src/arch/aarch64/exceptions.rs:283-321`
