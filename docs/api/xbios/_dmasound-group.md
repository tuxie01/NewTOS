# XBIOS DMA-Sound-Gruppe — 0x80…0x8D

**Stand:** 2026-09-29 · **Quelle (primär):** `references/emutos` @
`1a7484aba1bfba23e2c0899e243b6d8177fe41df` · **Quelle (sekundär, für das,
was EmuTOS nicht ausführt):** `references/freemint` @
`f906416a9387ed68876c738cfc5c30037b294fc0` · **Zielprofil:** `make 512`,
Falcon030/TOS4 · **Anlass:** `TASK/SND01.md`, Schritt 1 (Quellenanalyse,
kein Code).

Formvorlage: `docs/api/xbios/_psg-group.md` (Gruppenspec) und
`docs/api/xbios/2c-bconmap.md` (Vollvertrag). Zitierregel wie dort:
kein C-Rumpf abgeschrieben, Konstanten/Registertabellen als Daten,
jede Aussage mit `file:line`.

> Diese Datei schließt die in
> `docs/api/xbios/_full-function-inventory.md` (Kopfzeile und §2.9)
> ausdrücklich offen gelassene Unterfunktionsebene für den Bereich
> **0x80–0x8D**. Die DSP-Gruppe 0x60–0x7F bleibt weiterhin offen.

---

## 0. Das Ergebnis vorab

1. **EmuTOS implementiert alle 14 Funktionen 0x80–0x8D vollständig**
   (`bios/dmasound.c:215-803`) — es ist keine Stub-Gruppe.
2. **EmuTOS spielt selbst nie einen Puffer ab.** Die einzigen
   EmuTOS-internen Aufrufer der Gruppe sind `falcon_dmasound_init()`
   (`bios/dmasound.c:182-198`, ruft `devconnect`/`setsndmode`/`soundcmd`)
   und die XBIOS-Vektortabelle (`bios/xbios.c:1500-1513`). `setbuffer`,
   `buffoper`, `buffptr`, `setinterrupt`, `settracks`, `setmontracks`,
   `locksnd`, `unlocksnd`, `dsptristate`, `gpio`, `sndstatus` haben
   **keinen** EmuTOS-internen Aufrufer (geprüft per Baum-Sweep über alle
   `.c`/`.h`, siehe §10.1). Das heißt: der **Abspielweg selbst ist aus
   EmuTOS nicht beobachtbar** — EmuTOS schreibt nur Register, die es nie
   benutzt. Für die Puffersemantik (§4) ist deshalb FreeMiNT die
   tragende Quelle, protokollkonform (Track A, Punkt 2b).
3. Im `make 512`-Referenzbau sind **`CONF_WITH_DMASOUND = 1` und
   `CONF_WITH_XBIOS_SOUND = 1`** (`docs/api/_emutos-config.md:564,604`).
   Beide sind Laufzeit-torgesteuert; ohne Chip ist alles aus (§1).
4. **NewTOS verhält sich heute schon konform** — jeder Aufruf 0x80…0x8D
   liefert die Funktionsnummer zurück, exakt wie EmuTOS auf einer
   Maschine ohne DMA-Sound-Chip (§1.4). Ton einzubauen heißt, diese
   konforme Antwort bewusst zu verlassen.
5. Der teure Teil ist **nicht** die API, sondern der
   **Ende-des-Puffers-Interrupt**: er läuft über MFP-Timer-A, und NewTOS
   hat weder MFP noch `Xbtimer`/`Mfpint`/`Jenabint` (§9.1).

---

## 1. Gatter — was gebaut wird und was davon zur Laufzeit lebt

### 1.1 Übersetzungszeit

| Schalter | `make 512` | Wirkung |
|---|:--:|---|
| `CONF_WITH_DMASOUND` | **1** | `bios/dmasound.c` wird überhaupt übersetzt (`bios/dmasound.c:23`, `bios/dmasound.h:17`); `LAST_ENTRY = 0x8d`, Tabelle 142 Plätze (`bios/xbios.c:1273-1283`) |
| `CONF_WITH_XBIOS_SOUND` | **1** | wählt die Verfügbarkeitsregel (§1.3); setzt `CONF_WITH_DMASOUND` zwingend voraus — sonst `#error` (`include/config.h:2180-2183`) |

Werte aus der erzeugten Tabelle `docs/api/_emutos-config.md:564,604` (Audittabelle) und `:181,221` (Spalte `TARGET_512` der Divergenztabelle, Kopf `:162-165`),
nicht aus `config.h` nachgelesen (Protokoll Schritt 3.7). Die Standardwerte
stehen in `include/config.h:1129-1134` bzw. `1514-1522`; die
Nicht-Atari-Ziele schalten beide ab (`include/config.h:274-279`, `515-520`,
`755-760`).

**Stehende Warnung:** `1` in der Tabelle heißt „für alle Maschinen
mitübersetzt", nicht „im Falcon-Profil aktiv". Genau das ist hier der
entscheidende Punkt.

### 1.2 Laufzeit-Erkennung (`bios/dmasound.c:128-150`)

`detect_dmasound()` (gerufen aus `machine_detect()`,
`bios/machine.c:588-590`) setzt drei globale Flags
(`bios/dmasound.c:113-115`, exportiert über `include/has.h:124-130`):

| Flag | Wie ermittelt |
|---|---|
| `has_dmasound` | Busfehler-Probe `check_read_byte()` auf das Control-Register `$FFFF8901` (`bios/dmasound.c:131`) |
| `has_falcon_dmasound` | nur wenn `has_dmasound`: Probe auf `gpx_data_port` `$FFFF8943` (`bios/dmasound.c:139`) |
| `has_microwire` | `has_dmasound && !has_falcon_dmasound` — der Falcon hat kein Microwire, und das ist *nicht* per Busfehler feststellbar (`bios/dmasound.c:146-149`) |

Nebenwirkung der Erkennung: wenn `has_dmasound`, wird `control = 0`
geschrieben — „Stop all sound" (`bios/dmasound.c:141-142`).

### 1.3 `SOUND_IS_AVAILABLE` — das eine Makro, das alles torsteuert

`bios/dmasound.c:122-126`:

* mit `CONF_WITH_XBIOS_SOUND` (unser Profil): `SOUND_IS_AVAILABLE = has_dmasound`
* ohne: `SOUND_IS_AVAILABLE = has_falcon_dmasound`

**Jede** der 14 Funktionen beginnt mit `if (!SOUND_IS_AVAILABLE) return
<Funktionsnummer>;` (`bios/dmasound.c:217, 232, 344, 371, 405, 426, 443,
461, 501, 523, 547, 723, 753, 782`). Das ist dasselbe Signal wie
`xbios_unimpl` — die positive Funktionsnummer, **kein** `EINVFN`
(`docs/api/xbios/_dispatcher.md §3`, D-008).

Daraus die drei Maschinenklassen:

| Maschine | `has_dmasound` | `has_falcon_dmasound` | Verhalten der Gruppe |
|---|:--:|:--:|---|
| Falcon | 1 | 1 | voller Funktionsumfang |
| STe / TT | 1 | 0 | ST-Teilmenge; Falcon-only-Funktionen → `EBADRQ` (-5), siehe §3 |
| ohne Chip (**RPi4**) | 0 | 0 | alle 14 geben ihre Funktionsnummer zurück |

### 1.4 Was NewTOS heute tut — und warum das bereits konform ist

`tos-api/src/xbios/mod.rs:127-136`: jede nicht verdrahtete ID — also auch
0x80…0x8D — fällt auf `other => other as i64`, gibt also die
Funktionsnummer zurück. Das ist **bitgleich** mit EmuTOS auf einer
Maschine ohne DMA-Sound-Chip (§1.3, Zeile 3). NewTOS hat hier keinen
Konformitätsfehler, sondern eine ehrliche „nicht vorhanden"-Antwort.

Zwei Folgen, die im Ticket festzuhalten sind:

* Die Begründung „no PSG" in `tos-api/src/xbios/system.rs:145-161`
  (PSG-Gruppe 0x1C/0x1D/0x1E/0x20) bleibt **richtig** — der PSG ist ein
  anderer Chip als der DMA-Sound. Sobald DMA-Sound existiert, ist sie
  aber **unvollständig**: `docs/api/xbios/_psg-group.md`
  (Abschnitt „NewTOS implementation notes") sagt heute „A future NewTOS
  audio extension (`Dosound` stays a no-op) can expose PCM playback …
  at a reserved XBIOS slot" — das ist durch diese Spec überholt: der
  Platz ist nicht zu reservieren, er ist mit 0x80–0x8D belegt. Die
  Präzisierung der Begründung („kein PSG, aber DMA-Sound vorhanden")
  gehört in den Code — **das macht der Developer, nicht diese Spec.**
* `docs/api/xbios/_dispatcher.md §4` legt für NewTOS `LAST_ENTRY = 0x57`
  fest (TT-Shifter-Profil, 88 Plätze). Mit DMA-Sound wächst die Tabelle
  auf `0x8D` / 142 Plätze — und damit ändert sich auch, was `Xbios(N)`
  für **0x58…0x7F** (DSP-Bereich) liefert: nach EmuTOS bleibt es die
  Funktionsnummer (`xbios_unimpl`-Platz statt Bereichsprüfung — gleiche
  Antwort, `docs/api/xbios/_dispatcher.md §3`), also von außen
  ununterscheidbar. **Kein Vertragsbruch, aber eine bewusste
  Profilentscheidung**, die der Dispatcher-Spec-Eintrag §4 nachgeführt
  bekommen muss (Widerspruch zum dort notierten TT-Profil, siehe §10.4).

---

## 2. Opcode-Tabelle und ABI

Tabelle: `bios/xbios.c:1500-1513`. Prototypen: `bios/dmasound.h:26-39`.
Die m68k-Stapelform ist unabhängig bestätigt durch FreeMiNTs
Trap-Bindungen `references/freemint/sys/mint/falcon.h:356-386` — EmuTOS
selbst definiert für diese Gruppe **keine** Bindemakros
(`include/xbiosbind.h` kennt nur `Dosound`, Zeile 50).

| ID | TOS-Name | EmuTOS-Funktion | Signatur (`bios/dmasound.h`) | Bytes auf dem Stapel | FreeMiNT-Bindung |
|---:|---|---|---|:--:|---|
| 0x80 | `Locksnd` | `locksnd` | `LONG (void)` | 2 | `sys/mint/falcon.h:356` |
| 0x81 | `Unlocksnd` | `unlocksnd` | `LONG (void)` | 2 | `sys/mint/falcon.h:358` |
| 0x82 | `Soundcmd` | `soundcmd` | `LONG (WORD mode, WORD data)` | 6 | `sys/mint/falcon.h:362` |
| 0x83 | `Setbuffer` | `setbuffer` | `LONG (UWORD mode, ULONG start, ULONG end)` | 12 | `sys/mint/falcon.h:360` |
| 0x84 | `Setmode` | `setsndmode` | `LONG (UWORD mode)` | 4 | `sys/mint/falcon.h:364` |
| 0x85 | `Settracks` | `settracks` | `LONG (UWORD play, UWORD rec)` | 6 | `sys/mint/falcon.h:366` |
| 0x86 | `Setmontracks` | `setmontracks` | `LONG (UWORD track)` | 4 | `sys/mint/falcon.h:368` |
| 0x87 | `Setinterrupt` | `setinterrupt` | `LONG (UWORD mode, WORD cause)` | 6 | `sys/mint/falcon.h:370` |
| 0x88 | `Buffoper` | `buffoper` | `LONG (WORD mode)` | 4 | `sys/mint/falcon.h:373` |
| 0x89 | `Dsptristate` | `dsptristate` | `LONG (WORD dspxmit, WORD dsprec)` | 6 | `sys/mint/falcon.h:375` |
| 0x8A | `Gpio` | `gpio` | `LONG (UWORD mode, UWORD data)` | 6 | `sys/mint/falcon.h:378` |
| 0x8B | `Devconnect` | `devconnect` | `LONG (WORD src, WORD dst, WORD clk, WORD prescale, WORD protocol)` | 12 | `sys/mint/falcon.h:380` |
| 0x8C | `Sndstatus` | `sndstatus` | `LONG (WORD reset)` | 4 | `sys/mint/falcon.h:383` |
| 0x8D | `Buffptr` | `buffptr` | `LONG (LONG ptr)` | 6 | `sys/mint/falcon.h:385` |

**Korrektur zum Ticket:** `TASK/SND01.md:25` nennt `Buffoper` mit 0x89.
Richtig ist **0x88** (`bios/xbios.c:1508`); 0x89 ist `Dsptristate`
(`:1509`). Ebenso fehlt im Ticket `Setmode` 0x84 — EmuTOS nennt die
C-Funktion `setsndmode`, der API-Name ist `Setmode`
(`bios/xbios.c:1193` Debug-Text und `references/freemint/sys/mint/falcon.h:364`).

**AArch64-Abbildung** (NewTOS-Seite): ID in `x8`, Argumente ab `x0`,
Ergebnis in `x0` (`tos-api/src/xbios/mod.rs:1-8`). Wortargumente wie
üblich über `arg_i16`/`arg_u16` (`tos-api/src/trap.rs:75-86`). Für
`Setbuffer` und `Buffptr` siehe §9.2 — dort liegt eine echte
ABI-Entscheidung (32-Bit-`LONG` gegen 64-Bit-Zeiger), die **nicht** aus
EmuTOS ableitbar ist.

---

## 3. Die Funktionen einzeln

Alle Zeilenangaben in `references/emutos/bios/dmasound.c`, wenn nicht
anders vermerkt. Fehlercodes: `EBADRQ = -5` (`include/gemerror.h:24`),
`SNDNOTLOCK = -128`, `SNDLOCKED = -129` (`bios/dmasound.h:19-20`).

### 0x80 `Locksnd()` / 0x81 `Unlocksnd()` — `:215-240`

* `Locksnd`: bereits gesperrt → `SNDLOCKED` (-129); sonst sperren und
  **`1`** zurückgeben (`:220-224`). Nicht `0`.
* `Unlocksnd`: nicht gesperrt → `SNDNOTLOCK` (-128); sonst entsperren
  und `0` (`:235-239`).
* Zustand: `static int sound_locked` (`:111`), genullt in
  `dmasound_init()` (`:202`).
* **Die Sperre ist rein beratend.** `sound_locked` wird von **keiner**
  der anderen zwölf Funktionen gelesen (einzige Fundstellen: `:111,
  202, 220, 223, 236, 238`). Ein Programm, das `Locksnd` überspringt,
  kann ungehindert abspielen. Das ist EmuTOS-Verhalten und damit für uns
  das Soll.

### 0x82 `Soundcmd(mode, data)` — `:342-364`, Helfer `:246-303` / `:308-337`

`data == -1` ist durchgängig die Abfrage; jeder andere Wert setzt und
gibt den neuen Wert zurück. **`Soundcmd` liefert nie einen Fehlercode** —
unbekannte `mode`-Werte fallen auf `return 0` (`:302`, `:336`).

`mode == 6` (`SETPRESCALE`) wird **vor** der Falcon/STe-Verzweigung
behandelt und gilt auf beiden (`:347-356`): schreibt `data` (0…3) in
`mode_control` Bits 0-1, gibt `mode_control & 3` zurück. Werte außerhalb
0…3 setzen nichts und liefern den Istwert.

Falcon-Zweig (`sndcmd_falcon`, `:246-303`):

| mode | Name (FreeMiNT `sys/mint/falcon.h:231-241`) | Register | Lage des Wertes |
|---:|---|---|---|
| 0 | `LTATTEN` | `channel_attenuation` `$FFFF893A` | API-Bits 4-7 ↔ Registerbits 8-11 (`:256-260`) |
| 1 | `RTATTEN` | dito | API-Bits 4-7 ↔ Registerbits 4-7 (`:263-269`) |
| 2 | `LTGAIN` | `channel_amplification` `$FFFF8939` | API-Bits 4-7 ↔ Registerbits 4-7 (`:272-278`) |
| 3 | `RTGAIN` | dito | API-Bits 4-7 ↔ Registerbits 0-3 (`:281-287`) |
| 4 | `ADDERIN` | `codec_16bit_source` `$FFFF8937` | 2 Bits (`:288-293`) |
| 5 | `ADCINPUT` | `codec_adc_source` `$FFFF8938` | 2 Bits (`:294-299`) |

Die Einheit „1,5 dB je Schritt, Form `0x00V0`" steht **nicht** in EmuTOS,
sondern in FreeMiNT (`sys/mint/falcon.h:235`) — Sekundärquelle, so
gekennzeichnet.

STe-Zweig (`sndcmd_ste`, `:308-337`): nur `mode` 0/1. Der Wert wird
**anders interpretiert als auf dem Falcon**: `atten = (data & 0xff) / 6`,
gedeckelt auf 40, daraus ein LMC1992-Fader-Kommando über Microwire
(`:317-322`); Rückgabe ist `atten * 6`.

> **Kante (Quelle: Rechnung über `:317-323`):** der Wert läuft auf dem
> STe-Zweig **nicht rund** — `Soundcmd(LTATTEN, 0x0010)` liefert `12`
> zurück, nicht `0x10`. Der Istwert kommt außerdem aus zwei
> `static UWORD` (`:310`), nicht aus der Hardware (Microwire ist nur
> schreibbar).

### 0x83 `Setbuffer(mode, start, end)` — `:369-396`

Siehe §4. Prüfungen: `mode > 1` → `EBADRQ`; `mode == 1` (Aufnahme) ohne
Falcon → `EBADRQ` (`:374-375`). Auf dem Falcon wird vor dem Schreiben
Bit 7 des Control-Registers gesetzt (Aufnahme) bzw. gelöscht
(Wiedergabe) (`:377-383`). Danach werden Start- und Endadresse als je
drei Bytes geschrieben (`:386-393`) — **24 Bit**, die oberen 8 Bit einer
`ULONG` fallen stillschweigend weg. Rückgabe immer `0`.

### 0x84 `Setmode(mode)` — `:401-419`

`mode > 2` → `EBADRQ`. Sonst `mode_control &= 0x3f`, dann `mode == 2` →
Bit 7 (mono), `mode == 1` → Bit 6 (16 Bit) (`:411-416`). Rückgabe `0`.
Werte: `STEREO8 = 0`, `STEREO16 = 1`, `MONO8 = 2`
(FreeMiNT `sys/mint/falcon.h:251-253`).

> **Kante:** die Prüfung fragt `has_falcon_dmasound` **nicht** ab.
> `Setmode(STEREO16)` liefert deshalb auch auf STe/TT `0` (Erfolg),
> obwohl die Hardware kein 16-Bit kann (`:405-418`).

### 0x85 `Settracks(play, rec)` / 0x86 `Setmontracks(track)` — `:424-452`

Beide: ohne Falcon → `EBADRQ`; Wert > 3 → `EBADRQ`. `Settracks` schreibt
`play` in `track_control` Bits 0-1 und `rec` nach `record_tracks`
`$FFFF8936` (`:432-433`); `Setmontracks` schreibt `track` in
`track_control` Bits 4-5 (`:449`). Rückgabe `0`.

### 0x87 `Setinterrupt(mode, cause)` — `:457-494`

* `mode > 1` → `EBADRQ` (`:464-465`).
* **Ohne Falcon** (STe/TT): keinerlei Registerzugriff. `cause == 1` → `0`,
  alles andere → `EBADRQ`, mit der Begründung im Quelltext: „On STE and TT,
  interrupts are always enabled" (`:467-475`).
* **Mit Falcon:** `cause &= 3`; `mode == 0` schreibt `cause` in Bits 2-3
  des Interrupt-Registers `$FFFF8900`, `mode == 1` in Bits 0-1
  (`:477-491`). Rückgabe `0`.

Bedeutung der Werte — EmuTOS nennt sie nur in der Funktionsüberschrift
(„Timer-A or MFP-i7", `:455`); die vollständige Belegung steht in
FreeMiNT (`sys/mint/falcon.h:259-268`): `SI_TIMERA = 0`, `SI_MFPI7 = 1`;
`SI_NONE = 0`, `SI_PLAY = 1`, `SI_RECORD = 2`, `SI_BOTH = 3`. Der
FreeMiNT-Treiber bestätigt die Zuordnung *durch Benutzung*: er schreibt
`FalInt = 0x4` (also `cause = 1` in Bits 2-3) und hängt sich gleichzeitig
an Timer A (`sys/xdd/audio/falcon.c:82,187`). Damit ist
`mode 0 → Bits 2-3 → Timer A` aus zwei Quellen belegt.

### 0x88 `Buffoper(mode)` — `:499-516`

* `mode < 0` (**jeder** negative Wert, nicht nur -1) ist die Abfrage:
  liefert `control & 3`, auf dem Falcon zusätzlich die Aufnahmebits als
  `((control >> 2) & 0xc)` (`:504-511`).
* Sonst: `control = ((mode & 0xc) << 2) | (mode & 3)` (`:514`).
  API-Bits 0-1 → Controlbits 0-1, API-Bits 2-3 → Controlbits 4-5.
  Rückgabe `0`.

Bedeutung (FreeMiNT `sys/mint/falcon.h:272-275`): `SB_PLA_ENA = 1`,
`SB_PLA_RPT = 2`, `SB_REC_ENA = 4`, `SB_REC_RPT = 8`.

> **Zwei Kanten, beide aus `:504-514` abgelesen:**
> 1. Der **Schreibweg ist nicht** `has_falcon_dmasound`-abhängig, der
>    **Leseweg schon**. Auf STe/TT setzt `Buffoper(SB_REC_ENA)` also
>    Controlbits 4-5, die die Abfrage anschließend nicht zurückmeldet.
> 2. Der Schreibzugriff schreibt das **ganze Byte** — Bit 7 (die
>    Aufnahme/Wiedergabe-Registerumschaltung, §4.6) wird bei **jedem**
>    `Buffoper`-Setzaufruf auf 0 (Wiedergabe) zurückgesetzt.

### 0x89 `Dsptristate(dspxmit, dsprec)` — `:521-540`

Ohne Falcon → `EBADRQ`. Sonst Bit 7 in `crossbar_src` bzw. `crossbar_dest`
setzen/löschen (`:529-537`); Rückgabe `0`. „0 = tristate, 1 = enable"
(FreeMiNT `sys/mint/falcon.h:277`).

### 0x8A `Gpio(mode, data)` — `:545-568`

Ohne Falcon → `EBADRQ`. `mode 0` = Richtung setzen (`data & 7` nach
`gpx_data_direction` `$FFFF8941`), `mode 1` = Pins lesen (`gpx_data_port
& 7` **als Rückgabewert**), `mode 2` = Pins schreiben; alles andere →
`EBADRQ` (`:553-565`). Drei Bits, `GPIO_SET/READ/WRITE = 0/1/2`
(FreeMiNT `sys/mint/falcon.h:281-283`).

### 0x8B `Devconnect(src, dst, clk, prescale, protocol)` — `:721-730`

Falcon: `devconnect_falcon` (`:590-686`), sonst `devconnect_ste`
(`:691-719`).

`src` (FreeMiNT `sys/mint/falcon.h:287-290`): 0 `DMAPLAY`, 1 `DSPXMIT`,
2 `EXTINP`, 3 `ADC`. `dst` ist eine **Bitmaske** (`:294-297`): 1 `DMAREC`,
2 `DSPRECV`, 4 `EXTOUT`, 8 `DAC`. `clk` (`:301-303`): 0 = intern
25,175 MHz, 1 = extern, 2 = intern 32 MHz.

Falcon-Zweig: schreibt die Quellenbits in `crossbar_src` (`:597-635`),
danach für jedes gesetzte Ziel-Bit das zugehörige Feld in
`crossbar_dest` (`:640-675`), zuletzt `prescale` nach `freq_ext`
(externer Takt) oder `freq_int` (interner Takt) (`:680-683`).
Rückgabe: **`0` bei ungültigem `src`** (`:633`, bewusst wie TOS 4.04),
sonst **`1`** (`:685`).

> EmuTOS dokumentiert an `:573-589` ausdrücklich drei Mängel des
> TOS-4.04-Originals (kaum Fehlerprüfung, „ill-defined" Rückgabewert,
> Müll in `crossbar_dest` bei externem Takt) und weicht bewusst ab: nur
> `src` wird geprüft, Rückgabe `1` statt Müll. Der Autor vermerkt
> zugleich: „I do not know if `crossbar_dest` is set correctly, having
> no way of testing this" (`:586-588`). **Das ist ungeprüfter
> EmuTOS-Code** — für uns ein Warnhinweis, kein Beleg.

STe-Zweig (`:691-719`): akzeptiert **nur** `src == 0 && dst == 8 &&
clk == 0`, sonst `EBADRQ` (`:696-697`). `prescale == 0` = „STE compatible
mode", kehrt sofort mit `0` zurück (`:702-703`); sonst wird `prescale`
auf die vier STe-Raten abgebildet (§4.7). Rückgabe `0` — **nicht `1`**
wie im Falcon-Zweig.

### 0x8C `Sndstatus(reset)` — `:751-767`

* Ohne Falcon: **immer `0`**, `reset` wirkungslos (`:756-757`).
* Mit Falcon und `reset != 0`: `channel_attenuation = 0`, dann Bit 3 in
  `codec_16bit_source` `$FFFF8937` setzen und wieder löschen (`:759-764`).
  Welche Register dieser Umschlag nullt, listet der Quelltextkommentar
  `:735-750` auf: `$FF8920`, `$FF8921`, `$FF8930-33`, `$FF8934-39`,
  `$FF893A-3B`; `$FF893C-3D` wird auf `$003F` gesetzt.
* Rückgabe immer `(codec_status >> 4) & 0x3f` (`:766`).

Feldbelegung der Rückgabe — aus FreeMiNT (`sys/mint/falcon.h:338-344`):
Bits 0-1 sind ein Fehlercode (`SS_OK 0`, `SS_CTRL 1`, `SS_SYNC 2`,
`SS_SCLK 3`), Bit 4 `SS_RTCLIP`, Bit 5 `SS_LTCLIP`. Beides zusammen ist
in sich stimmig: **Schlussfolgerung (Inferenz, nicht zitiert)** — direkt
nach einem Reset ergibt der im Kommentar genannte Registerwert `$003F`
genau `3` = `SS_SCLK` („serial clock out of valid range"), was zu einem
eben erst genullten Taktkreuz passt. Ein Programm sollte also nach
`Sndstatus(1)` erst `Devconnect` aufrufen und dann erneut prüfen.

### 0x8D `Buffptr(ptr)` — `:772-803`

`ptr` zeigt auf eine 16-Byte-Struktur `SndBufPtr` — vier `LONG`:
`play`, `record`, `res1`, `res2` (`:774-779`; identisch in FreeMiNT
`sys/mint/falcon.h:348-353`).

Ablauf: auf dem Falcon zuerst Control-Bit 7 setzen, die drei
Zählerbytes lesen → `record`, Bit 7 wieder löschen (`:787-795`); dann
die Zählerbytes lesen → `play` (`:797-800`). Rückgabe `0`.

> **Kanten:** (a) **keine Nullzeigerprüfung** — `ptr == 0` schreibt nach
> Adresse 0 (`:785`). (b) `res1`/`res2` werden **nicht** beschrieben; der
> Aufrufer sieht dort seinen eigenen alten Inhalt. (c) Auf STe/TT wird
> `record` **gar nicht** beschrieben (`:787` ist Falcon-gated), also
> ebenfalls Altinhalt. (d) Die Funktion hinterlässt Control-Bit 7
> **gelöscht**, auch wenn der Aufrufer es zuvor gesetzt hatte (§4.6).

---

## 4. Der Puffermechanismus — das, was ein Treiber bedienen muss

> **Quellenlage, ehrlich:** EmuTOS *programmiert* die Register, *benutzt*
> sie aber nie (§0.2). Die Bedeutung von „Endadresse", „Schleife" und
> „Ende-Signal" ist deshalb aus EmuTOS allein **nicht** belegbar. Was
> folgt, ist zweiquellig: EmuTOS für das Registerbild und die
> API-Wirkung, FreeMiNTs Falcon-Audiotreiber
> (`references/freemint/sys/xdd/audio/falcon.c`, `references/freemint/sys/xdd/audio/dmasnd.h`) für den
> tatsächlichen Abspielablauf. Jede Zeile unten sagt, aus welcher.

### 4.1 Das Registerbild (EmuTOS `bios/dmasound.c:25-72`)

Basis `0xFFFF8900` (`:72`). Aus den Strukturversätzen (`:28-69`) ergibt
sich:

| Adresse | Feld | Rolle |
|---|---|---|
| `$FFFF8900` | `interrupt` | Interrupt-Freigabe (nur Falcon), §3/0x87 |
| `$FFFF8901` | `control` | Bit 0/1 Wiedergabe an/Schleife, Bit 4/5 Aufnahme an/Schleife, **Bit 7 Registerbank** |
| `$FFFF8903/05/07` | `frame_start_{high,mid,low}` | Startadresse, 24 Bit |
| `$FFFF8909/0B/0D` | `frame_counter_{high,mid,low}` | laufender Zeiger, 24 Bit, **nur lesbar sinnvoll** |
| `$FFFF890F/11/13` | `frame_end_{high,mid,low}` | Endadresse, 24 Bit |
| `$FFFF8920` | `track_control` | Spuren (Falcon) |
| `$FFFF8921` | `mode_control` | Bit 7 mono, Bit 6 16 Bit, Bits 0-1 STe-Prescaler |
| `$FFFF8922/24` | `microwire_data`/`_mask` | LMC1992 (nur STe/TT) |
| `$FFFF8930/32` | `crossbar_src`/`_dest` | Verbindungsmatrix (Falcon) |
| `$FFFF8934/35` | `freq_ext`/`freq_int` | Prescaler (Falcon) |
| `$FFFF8936…3D` | Codec-Register | siehe §3 |
| `$FFFF8941/43` | `gpx_data_direction`/`_port` | DSP-Steckverbinder |

Die Adressen `$FF8903/05/07`, `$FF8909/0B/0D`, `$FF890F/11/13` und
`$FF8920` sind unabhängig durch FreeMiNT bestätigt
(`sys/xdd/audio/dmasnd.h:13-30`).

**Für NewTOS ist dieses Registerbild Interna, kein Vertrag** — siehe §8.

### 4.2 Übergabe eines Puffers

1. `Setbuffer(SR_PLAY=0, start, end)` schreibt Start und Ende in die
   Wiedergabe-Registerbank (EmuTOS `:386-393`).
2. `Buffoper(SB_PLA_ENA)` startet (EmuTOS `:514`; FreeMiNT schreibt an
   derselben Stelle `DMActl = DMA_PLAY = 0x01`,
   `sys/xdd/audio/dmasnd.h:10`, `falcon.c:181`).
3. Der Baustein liest ab `start` aufwärts und erhöht
   `frame_counter`; erreicht er `end`, ist der Rahmen fertig.

**Die Endadresse ist exklusiv** — belegt aus FreeMiNT, nicht aus EmuTOS:
der Treiber füllt `dmab[i].used` Bytes ab `dmab[i].buf` und setzt
`DMASetEnd(dmab[i].buf + dmab[i].used)`
(`sys/xdd/audio/falcon.c:179-180, 184-185`; Makro
`sys/xdd/audio/dmasnd.h:39-44`). Ende = erste Adresse **hinter** den
Daten; die Rahmenlänge ist `end - start`.

**Ausrichtung:** derselbe Treiber sorgt dafür, dass die Länge **gerade**
ist, und verdoppelt notfalls den letzten Abtastwert — Kommentar im
Quelltext: „DMA wants even number of samples"
(`sys/xdd/audio/falcon.c:168-173`). Das ist eine
Hardware-Anforderung an die Rahmenlänge, keine API-Prüfung: **weder
EmuTOS noch irgendeine XBIOS-Funktion prüft das.**

### 4.3 Ende und Schleife

* `SB_PLA_ENA` allein: ein Rahmen, danach stoppt der Baustein. Der
  FreeMiNT-Treiber nutzt genau das für den Einzelpuffer
  (`falcon.c:178-181`).
* `SB_PLA_ENA | SB_PLA_RPT` (= 3, `DMA_PLAY|DMA_LOOP`): „Play repeat
  (continuous loop mode)" — so der Name in FreeMiNT
  (`sys/mint/falcon.h:273`, Konstante `sys/xdd/audio/dmasnd.h:10-11`,
  benutzt `falcon.c:186`). **Inferenz (nicht zitiert):** dass dabei
  Start- und Endadresse aus den Rahmenregistern neu geladen werden,
  folgt aus dem Doppelpufferverfahren unten, steht aber in keiner der
  beiden Quellen wörtlich.
* Anhalten: `Buffoper(0)` (`DMA_OFF`, `falcon.c:112,134`).

Das Doppelpuffer-Verfahren des FreeMiNT-Treibers ist die Blaupause für
lückenlose Wiedergabe (`falcon.c:121-145` zusammen mit `:174-188`):
Puffer A läuft in Schleife, im Ende-Interrupt wird auf Puffer B
umgeschaltet, indem `Setbuffer`-Register neu gesetzt und `control`
neu geschrieben wird; bleibt kein Nachschub, wird auf `DMA_OFF`
geschaltet.

### 4.4 Der Interrupt-Vertrag

**Das ist der Kern und die teuerste Stelle.** Zwei Schichten:

1. **Chipseite:** `Setinterrupt(mode, cause)` gibt im Sound-Baustein
   frei, *dass* am Rahmenende ein Signal erzeugt wird — Timer A
   (`mode 0`) oder MFP-i7 (`mode 1`) (EmuTOS `:477-491`).
2. **Vektorseite:** den Handler hängt das **Programm selbst** ein, über
   den MFP: `Xbtimer(0, 8, 1, handler)` plus `Jenabint(13)`
   (`references/freemint/sys/xdd/audio/falcon.c:81-83`). **EmuTOS
   installiert keinen einzigen Sound-Interrupt-Handler** — in
   `bios/dmasound.c` gibt es keine Handler-Funktion, und `setinterrupt`
   berührt keinen Vektor.

Die Nummer ist EmuTOS-belegt: `MFP_TIMERA = 13`, und EmuTOS definiert
dafür sogar das Synonym **`MFP_DMASOUND`**
(`include/biosdefs.h:192-193`). `Xbtimer` → `setup_timer` +
`mfpint` → Vektor bei `(0x40 + 13) * 4` (`bios/mfp.c:114-120, 163-169`).

**Konsequenz für NewTOS:** ohne `Xbtimer`/`Mfpint`/`Jenabint` (0x1F,
0x0D, 0x1B — in `tos-api/src/xbios/mod.rs` **nicht verdrahtet**) gibt es
keinen Weg, dem Programm das Pufferende zu melden. Siehe §9.1.

### 4.5 Polling als Ersatz

`Buffptr` liefert den laufenden Zähler (§3/0x8D). Ein Programm kann
damit ohne Interrupt erkennen, wie weit die Wiedergabe ist. Der
FreeMiNT-Treiber tut das nicht (er ist interruptgetrieben), aber die
Funktion ist genau dafür da. Für NewTOS ist das der **realistische
erste Abspielweg** (§9.1).

### 4.6 Der Bit-7-Zustandsautomat (leicht zu übersehen)

Control-Bit 7 schaltet zwischen Aufnahme- und Wiedergabe-Registerbank
um. Wer es setzt und löscht:

| Aufruf | Wirkung auf Bit 7 | Zeile |
|---|---|---|
| `Setbuffer(0, …)` | löscht | `:382` |
| `Setbuffer(1, …)` | **setzt — und lässt es gesetzt** | `:380` |
| `Buffptr(…)` | setzt, liest, **löscht** | `:789, 794` |
| `Buffoper(mode ≥ 0)` | löscht (Vollbyte-Schreibzugriff) | `:514` |
| `Buffoper(mode < 0)` | unverändert (nur Lesen) | `:507` |

Nach `Setbuffer(SR_RECORD, …)` bleibt die Aufnahmebank ausgewählt, bis
eine der anderen drei Operationen sie zurückschaltet. Das ist
**beobachtbarer Zustand über Aufrufgrenzen hinweg** — ein NewTOS-Backend
muss ihn nachbilden oder die Abweichung begründen.

### 4.7 Abtastraten

**STe-kompatibler Weg** (`Devconnect(…, prescale)` mit `clk = 0` und
`prescale != 0`, bzw. `Soundcmd(SETPRESCALE, n)`), EmuTOS
`devconnect_ste:700-716` samt Quelltextkommentaren:

| `mode_control` Bits 0-1 | EmuTOS-Kommentar | `prescale`-Werte, die dorthin abgebildet werden |
|:--:|---|---|
| 3 | „50 kHz" (`:705`) | 1 |
| 2 | „25 kHz" (`:709`) | 2, 3, 4 |
| 1 | „12 kHz" (`:712`) | 5, 6, 7, 8 |
| 0 | „leave it at 6 kHz" (`:714`) | alle übrigen |

`prescale == 0` bedeutet „STE compatible mode" und kehrt **vor** jeder
Änderung zurück (`:702-703`). Die Teilerbezeichnungen `PRE160/320/640`
(= Bits 3/2/1) und `PREMUTE` (= 0, „was /1280, now is invalid, mutes")
stehen in FreeMiNT (`sys/mint/falcon.h:243-247`); die *gemessenen*
Frequenzen des ST-Wegs — 50066, 25033, 12517, 6285 Hz — ebenfalls dort
(`sys/xdd/audio/falcon.c:209-250`).

**Falcon-Weg** (interner 25,175-MHz-Takt): EmuTOS schreibt `prescale`
**roh** nach `freq_int` (`:683`) und sagt über die resultierende Rate
nichts. FreeMiNT nennt die Tabelle (`sys/mint/falcon.h:309-320`):

| `prescale` | Hz | | `prescale` | Hz |
|:--:|---:|---|:--:|---:|
| 1 | 49170 | | 7 | 12292 |
| 2 | 32780 | | 9 | 9834 |
| 3 | 24585 | | 11 | 8195 |
| 4 | 19668 | | 5 | 16390 |

FreeMiNT merkt selbst an, die offizielle Dokumentation nenne für viele
Werte falsche Hz-Zahlen (`sys/mint/falcon.h:307`); die ungeraden
Zwischenwerte sind „invalid for CODEC". **Inferenz (nicht zitiert, aber
nachgerechnet):** die Tabelle ist exakt
`25 175 000 / (256 × (prescale + 1))` — für 1, 2, 3, 4 ergibt das
49170, 32780, 24585, 19668.

### 4.8 Sampleformat

Aus FreeMiNT (`sys/mint/falcon.h:213-222`), in EmuTOS **nirgends**
beschrieben: alle Werte sind **vorzeichenbehaftet**; 8-Bit-Stereo ist
`L R L R …` (je 1 Byte), 16-Bit-Stereo ist `LL RR LL RR …` (je 2 Bytes),
bei mehreren Spuren verschachtelt `L0 R0 L1 R1 …`. Die Bytefolge der
16-Bit-Werte ist die der m68k-Seite (big-endian) — das steht so nicht
da, folgt aber aus der Zielmaschine. **Für NewTOS eine offene
Vertragsfrage, siehe §9.3.**

---

## 5. Zustand nach dem Systemstart

`dmasound_init()` (`bios/dmasound.c:200-208`, gerufen aus
`bios/bios.c:440-444`) tut dreierlei: `sound_locked = 0`,
`lmc1992_init()` (nur wenn Microwire, also STe/TT: Lautstärke 0 dB,
Fader 0 dB, Höhen/Bässe 0 dB, „Mix GI sound chip output" —
`:167-180`) und auf dem Falcon `falcon_dmasound_init()` (`:182-198`):

| Aufruf | Bedeutung laut Quelltextkommentar `:184-189` / `:192-197` |
|---|---|
| `devconnect(0, 8, 0, 0, 1)` | DMA-Wiedergabe → DAC (Kopfhörer/Lautsprecher), interner 25,175-MHz-Takt, TT/STe-Kompatibilitätsmodus, „disable handshaking" (`protocol = 1`) |
| `setsndmode(0)` | 8-Bit-Stereo |
| `soundcmd(2, 0x0080)` / `(3, 0x0080)` | linke/rechte Verstärkung = 8 |
| `soundcmd(4, 0x0003)` | ADDERIN = ADC + Matrix |
| `soundcmd(5, 0x0003)` | ADC-Quelle links+rechts = PSG |
| `soundcmd(6, 0x0003)` | TT-kompatibler Prescaler /160 (≙ 50 kHz) |

Das ist der Zustand, den ein Programm auf einem EmuTOS-Falcon **vor
seinem ersten eigenen Aufruf** vorfindet — also Teil dessen, was
abfragende Programme sehen. Für NewTOS gehört genau diese Anfangslage
nachgebildet, damit `Soundcmd(…, -1)`-Abfragen plausibel antworten.

---

## 6. Der `_SND`-Cookie — die Entdeckungsseite des Vertrags

`setvalue_snd()` (`bios/machine.c:409-447`), eingehängt in
`fill_cookie_jar()` (`bios/machine.c:707-719`), Tag `COOKIE_SND`
= `0x5F534E44` („_SND", `include/cookie.h:30`).

| Bit | Name (`include/cookie.h:81-86`) | Wann gesetzt |
|---:|---|---|
| 0x01 | `SND_PSG` | `CONF_WITH_YM2149` (`machine.c:413-415`) |
| 0x02 | `SND_8BIT` | `HAS_DMASOUND` (`:417-420`) |
| 0x04 | `SND_16BIT` | `HAS_FALCON_DMASOUND` (`:422-425`) |
| 0x10 | `SND_MATRIX` | dito, gleiche Zeile |
| 0x08 | `SND_DSP` | `HAS_DSP` (`:427-430`) |
| 0x20 | `SND_XBIOS` | **nie** — definiert, aber im ganzen Baum nirgends gesetzt (Sweep über den Baum, einzige Fundstelle `include/cookie.h:86`) |

Zusätzlich: ist DIP-Schalter 8 an (Bit 7 in `cookie_swi` aus), wird
`SND_8BIT` wieder gelöscht — „just like TOS3/TOS4 do"
(`bios/machine.c:432-444`).

> **Kommentar-gegen-Code (klassischer Fall):** der Kommentarblock an
> `bios/machine.c:707-716` nennt eine **andere** Bitbelegung
> (0x04 = „DMA Record (w/XBIOS)", 0x08 = „16-bit CODEC", 0x10 = „DSP")
> als die Konstanten, die der Code eine Zeile später tatsächlich
> verwendet (`include/cookie.h:81-86`). Der Code gilt. Zahlenmäßig
> fällt es nicht auf, weil auf dem Falcon ohnehin alle fünf Bits
> gesetzt werden (0x1F) und auf dem STe nur 0x03 — aber wer die
> Belegung aus dem Kommentar abschreibt, baut den Cookie für
> Zwischenfälle falsch. **Eintrag für `docs/api/_discrepancies.md`
> (noch nicht vergeben), siehe §10.**

**Warum das für SND01 zählt:** der FreeMiNT-Falcon-Audiotreiber
verweigert den Dienst, wenn `_SND` fehlt oder keines der Bits
`SND_16BIT|SND_DSP|SND_MATRIX` gesetzt ist
(`sys/xdd/audio/falcon.c:47-53`). NewTOS setzt **heute gar kein `_SND`**
— `tos-api/src/cookie.rs:189-193` lässt `_VDO`/`_SND`/`_FDC`/`_FPU`
bewusst weg. Ohne `_SND` wird ein portiertes Klangprogramm die
XBIOS-Funktionen gar nicht erst aufrufen. **Der Cookie ist Teil der
Arbeit, nicht Beiwerk.**

---

## 7. Fehlercodes der Gruppe

| Wert | Name | Woher | Wann |
|---:|---|---|---|
| Funktionsnummer (0x80…0x8D) | — | `bios/dmasound.c:218` usw. | kein Sound-Chip vorhanden |
| `-5` | `EBADRQ` | `include/gemerror.h:24` | ungültiger Modus/Bereich; Falcon-only-Funktion auf STe/TT |
| `-128` | `SNDNOTLOCK` | `bios/dmasound.h:19` | `Unlocksnd` ohne vorheriges `Locksnd` |
| `-129` | `SNDLOCKED` | `bios/dmasound.h:20` | `Locksnd` bei bestehender Sperre |
| `0` | E_OK | überall | Erfolg |
| `1` | — | `:224` bzw. `:685` | `Locksnd` erfolgreich; `Devconnect` (Falcon) erfolgreich |

> **Abweichung zwischen den Quellen:** FreeMiNT führt dieselben beiden
> Sound-Fehler als `ESNDLOCKED = -129` und **`ESNDNOTLOCK = -130`**
> (`sys/mint/errno.h:561-562`), nicht -128. EmuTOS ist primär, also
> gilt **-128**. **Inferenz zur Ursache:** -128 ist in FreeMiNTs
> errno-Tabelle bereits durch `EINTR` belegt (`sys/mint/errno.h:558`),
> was die Verschiebung erklären würde; belegt ist das nicht.
> Kandidat für den Abweichungslog (§10).

---

## 8. Vertrag oder Interna — die Linie nach `docs/COMPATIBILITY.md`

Nach `docs/COMPATIBILITY.md:35-47` (Vertragsfläche) und `:49-64`
(freie Umsetzung), Faustregel `:63-64`: „if a program can read it, time
it, or depend on it through a documented interface, it is contract."

**Vertrag — muss stimmen:**

* Trap 14, Funktions-IDs 0x80…0x8D und ihre Argumentzahl/-reihenfolge
  (§2).
* Alle Rückgabewerte inklusive der Sonderfälle: `Locksnd → 1`,
  `Devconnect → 1` (Falcon) bzw. `0` (STe-Zweig und ungültiges `src`),
  `EBADRQ`, `SNDLOCKED`/`SNDNOTLOCK`, und die
  Funktionsnummer-als-„nicht vorhanden".
* Die Abfrageformen: `Soundcmd(mode, -1)`, `Buffoper(mode < 0)`,
  `Gpio(1, …)`, `Sndstatus(0)`.
* `SndBufPtr` (4 × LONG, `play` zuerst) und die Tatsache, dass
  `res1`/`res2` unberührt bleiben (§3/0x8D).
* Die Bedeutung von Start/Ende (Ende exklusiv), die
  Schleifenbits, und dass ein Rahmenende ein Programm erreichen kann.
* Der `_SND`-Cookie (§6) — `docs/COMPATIBILITY.md:46` nennt den
  Cookie-Jar ausdrücklich als Vertragsfläche.
* Der Zustand nach dem Start (§5), soweit ihn Abfragen sichtbar machen.

**Interna — NewTOS darf anders:**

* Das gesamte Registerbild `$FFFF8900…$FFFF8943` (§4.1). Auf der Pi
  existiert diese Adresse nicht; ein Programm, das dorthin schreibt,
  kann auf NewTOS ohnehin nicht funktionieren — dieselbe bewusste
  „ABI-Stille" wie bei der PSG-Gruppe
  (`docs/api/xbios/_psg-group.md`, Kante 2).
* Die Verbindungsmatrix als *Mechanismus* (Kreuzschienenbits). Die
  *Wirkung* — „DMA-Wiedergabe landet am DAC" — ist Vertrag, die
  Bitkodierung nicht.
* Die 24-Bit-Adressgrenze der Rahmenregister (§4.2) — eine Eigenheit des
  Falcon-DMA, kein API-Versprechen. **Aber:** die daraus folgende
  Frage, was NewTOS mit einem Zeiger oberhalb 16 MB tut, ist sehr wohl
  vertragsrelevant (§9.2).
* Ob intern PWM, HDMI oder etwas anderes klingt — Geräteparameter.

---

## 9. Lücken — was NewTOS ehrlich nicht liefern kann

Jede Lücke hier ist als **dokumentiertes „nicht vorhanden"** zu führen,
nicht als stiller Erfolg (CLAUDE.md Regel 2, `TASK/SND01.md:101-106`).

### 9.1 Der Ende-des-Puffers-Interrupt (die größte Lücke)

Der übliche Weg — Timer A über `Xbtimer(0, 8, 1, handler)` +
`Jenabint(13)` (§4.4) — setzt MFP-Funktionen voraus, die NewTOS nicht
hat: 0x0D `Mfpint`, 0x1A `Jdisint`, 0x1B `Jenabint`, 0x1F `Xbtimer` sind
in `tos-api/src/xbios/mod.rs` **nicht verdrahtet** und liefern die
Funktionsnummer. Folgen:

* `Setinterrupt` kann gebaut werden (es speichert nur eine Freigabe),
  bewirkt aber ohne Vektorseite **nichts** — das wäre genau die Art
  stiller Erfolg, die Regel 2 verbietet. Ehrliche Varianten: (a)
  `Setinterrupt` ebenfalls als „nicht vorhanden" führen, oder (b)
  zuerst einen MFP-Ersatz für Timer A bauen.
* Ohne Interrupt bleiben zwei belastbare Abspielwege: **eine einzelne
  Schleife** (`Buffoper(SB_PLA_ENA|SB_PLA_RPT)`, läuft ewig, braucht
  niemanden) und **Polling über `Buffptr`**.
* **Entscheidung nötig (Lead/Developer, nicht hier):** MFP-Timer-A-Ersatz
  bauen, oder FreeMiNTs GSXB-Erweiterung übernehmen, die den Handler
  direkt an `Setinterrupt` übergibt (dritter Parameter `void *inth`,
  `references/freemint/sys/gsxb.c:294`) — Achtung, siehe §10.5:
  `sys/gsxb.c` ist im gepinnten Baum ein **leeres Gerüst**.

### 9.2 Zeigerbreite bei `Setbuffer`/`Buffptr` — echte ABI-Entscheidung

EmuTOS nimmt zwei `ULONG` und schneidet auf 24 Bit (`:386-393`). NewTOS
läuft LP64; eine Pufferadresse im EL0-Raum passt nicht in 32 Bit, erst
recht nicht in 24. Die Stellschrauben stehen bereit
(`tos-api/src/trap.rs:93-99`: `arg_u64`, `arg_ptr`), aber **welche Form
der Vertrag hat, ist aus EmuTOS nicht ableitbar** — dort gibt es keine
64-Bit-Welt. Gleiches gilt für den `SndBufPtr`-Zeiger von `Buffptr` und
für die Breite der Felder `play`/`record` in dieser Struktur (LONG auf
m68k; auf LP64 offen — vgl. `docs/COMPATIBILITY.md:623-637`, „Native vs
m68k struct variants").
**STOP — Entscheidung des Leads, keine Erfindung hier.**

### 9.3 Sampleformat und Bytefolge

§4.8: vorzeichenbehaftet und verschachtelt ist belegt, die **Bytefolge
der 16-Bit-Werte** ist es nicht. Auf m68k ist sie big-endian; ein
nativer AArch64-LP64-Anwender erzeugt little-endian. **Ungelöste
Vertragsfrage** — ein falsch geratenes Byteorder klingt nach Rauschen,
nicht nach einem Fehler, deshalb hier ausdrücklich markiert.

### 9.4 Aufnahme

Kein Eingang auf der Pi: `Setbuffer(SR_RECORD, …)`, die Aufnahmebits in
`Buffoper`, `SndBufPtr.record`, `Settracks(·, rec)` und
`Soundcmd(ADCINPUT)` haben nichts zu bedienen. EmuTOS' eigene Antwort
für „kein Falcon" ist hier `EBADRQ` (`:374-375`, `:429-430`) — das ist
die ehrliche, präzedenzgestützte Wahl.

### 9.5 DSP-Anbindung

`Dsptristate` (0x89) und `Gpio` (0x8A) konfigurieren den
DSP-Steckverbinder. Ohne DSP gibt es nichts zu schalten; EmuTOS'
Nicht-Falcon-Antwort ist `EBADRQ` (`:526-527`, `:550-551`). Passend
dazu darf `SND_DSP` (0x08) im `_SND`-Cookie **nicht** gesetzt werden —
sonst suchen Programme einen DSP.

### 9.6 Raten

Die Falcon-Ratentabelle (§4.7) ist an einen 25,175-MHz-Takt gebunden.
Welche Raten ein PWM- oder HDMI-Ausgang auf der Pi trifft, ist **Track
B** und steht ausdrücklich nicht in dieser Spec
(`TASK/SND01.md:91-99`, Schritt 2). Ein Backend muss entweder exakt
treffen oder umtasten; welches von beidem, entscheidet die
Hardwarerecherche.

### 9.7 `Locksnd` ist beratend

§3/0x80: EmuTOS erzwingt nichts. Ein NewTOS, das die Sperre *erzwingt*,
wäre „besser" und trotzdem eine Abweichung — Programme, die ohne
`Locksnd` losspielen, funktionieren auf echtem TOS. **Nicht erzwingen,
ohne es als bewusste Abweichung zu notieren.**

---

## 10. Kreuzverweise, Korrekturen, Abweichungskandidaten

### 10.1 Methode der „kein Aufrufer"-Aussage

Sweep über den gesamten EmuTOS-Baum (`.c`/`.h`, ohne `doc/` und ohne
Objektdateien) nach allen 14 Funktionsnamen: Treffer ausschließlich in
`bios/dmasound.c` (Definition + `falcon_dmasound_init`),
`bios/dmasound.h`, `bios/xbios.c:1171-1240` (Debug-Hüllen) und
`bios/xbios.c:1500-1513` (Tabelle).

### 10.2 Korrektur am Ticket

`TASK/SND01.md:25` — `Buffoper` ist 0x88, nicht 0x89 (§2). Das Ticket
gehört dem Lead; hier nur vermerkt.

### 10.3 Präzisierung an `_psg-group.md`

Der dortige Schlusssatz („a future NewTOS audio extension … at a
reserved XBIOS slot") ist überholt: die Wiedergabe hat ihren
vorgesehenen Platz bei 0x80–0x8D. Die PSG-Aussage selbst („no YM2149")
bleibt richtig.

### 10.4 Widerspruch zum Dispatcher-Profil

`docs/api/xbios/_dispatcher.md §4` legt NewTOS auf `LAST_ENTRY = 0x57`
fest; das Zielprofil Falcon030/TOS4 und diese Gruppe verlangen `0x8D`.
Von außen ist der Unterschied nicht beobachtbar (§1.4), die Spec sollte
aber nachziehen, sobald die Gruppe gebaut wird. **Nicht hier geändert**
— fremder Spec-Eintrag, gehört in eine eigene Entscheidung.

### 10.5 FreeMiNTs GSXB ist kein benutzbarer Beleg

`references/freemint/sys/gsxb.c` enthält für 0x82…0x8D **leere Rümpfe**
(`:263-334`) und in `sys_gsxb_unlocksnd` eine Zuweisung statt eines
Vergleichs (`if (lock = ENOLOCK)`, `:252`). Verwertbar sind daraus nur
zwei Struktur-Aussagen: GSXB erweitert `Soundcmd` um ein drittes
`long`-Argument (`:263`) und `Setinterrupt` um einen Handler-Zeiger
(`:294`), und es gibt ein `Waveplay` auf 0xA5 (`:336`).
**Als Semantikquelle unbrauchbar.**

### 10.6 Abweichungskandidaten für `docs/api/_discrepancies.md`

Hier bewusst **nicht** eingetragen (Nummernvergabe gehört dem Lead;
`_discrepancies.md` ist gemeinsam genutzt):

1. `_SND`-Kommentar gegen `_SND`-Konstanten in EmuTOS (§6).
2. `SNDNOTLOCK` -128 (EmuTOS) gegen `ESNDNOTLOCK` -130 (FreeMiNT) (§7).
3. `Buffoper`: Schreibweg nicht Falcon-gated, Leseweg schon (§3/0x88).
4. `Setmode(STEREO16)` meldet auf STe/TT Erfolg für eine unmögliche
   Betriebsart (§3/0x84).
5. `Soundcmd(LTATTEN)` läuft auf dem STe-Zweig nicht rund (§3/0x82).
6. `Devconnect` gibt im Falcon-Zweig `1`, im STe-Zweig `0` zurück —
   derselbe Erfolg, zwei Werte (§3/0x8B).

### 10.7 Compendium / Profibuch

Beide liegen in `references/` **nicht** vor (`references/README.md:24-25`
listet sie als separat zu beschaffen; vorhanden sind nur `emutos`,
`freemint`, `datasheets`, `sd-spec`, `circle`). Der in Schritt 3.8 des
Protokolls vorgesehene Abgleich ist damit **aufgeschoben**, wie schon
bei `_psg-group.md`. Alle „Bedeutungs"-Angaben, die nicht aus EmuTOS
stammen, kommen in dieser Spec aus FreeMiNT und sind dort einzeln
zitiert.

---

## 11. NewTOS-Umsetzungsnotizen

(Leer nach Protokoll — wird beim Implementierungsschritt gefüllt. Die
offenen Entscheidungen stehen gesammelt in §9; sie sind Entscheidungen,
keine Umsetzungsnotizen.)
