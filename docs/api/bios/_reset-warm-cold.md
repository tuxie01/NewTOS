# Warm- und Kaltstart: `warm_reset`, `cold_reset`, Tastenkombination, Reset-Einsprung im OSHEADER

> Spec zu TASK/RST01 und TASK/X04 (Schritt 1, nur Spec; kein Code).
> Quelle: EmuTOS `references/emutos` @ `1a7484ab` (gelesen 2026-10-17).
> Jede Aussage traegt `file:line`; **[gelesen]** = unmittelbar im Quelltext
> gesehen, **[Schluss]** = von mir daraus gefolgert (nicht lasttragend ohne
> Gegenpruefung). Der Abschnitt zur Pi-Hardware steht getrennt in
> `docs/hardware/pm-watchdog-reset.md` (Track B).

Abgrenzung nach `docs/COMPATIBILITY.md`: Was ein Programm beobachtet
(Tastenkombination loest Neustart aus, `OSHEADER`-Reset-Feld ist ein
sprungfaehiger Einstieg, `memvalid`/`resvalid` steuern, ob RAM-Zustand den
Neustart ueberlebt) ist **Vertrag**. Wie der Neustart auf der Pi technisch
ablaeuft, ist **frei**.

## 1. `warm_reset` und `cold_reset`

Beide sind reine Assembler-Routinen in `bios/panicasm.S`; Deklaration
`bios/bios.h:45,48` (`void warm_reset(void) NORETURN;`,
`void cold_reset(void) NORETURN;`). Aufrufer im ganzen Baum (`bios/*.c`,
`bdos`, `aes`, `desk`, `vdi`): ausser `kbd_int` (`bios/ikbd.c:740,744`)
ruft nur `kill_program` per Durchfall (s. u.) `warm_reset`. [gelesen, Grep]

### 1.1 `warm_reset` (`panicasm.S:102-107`) [gelesen]

```
_warm_reset:
        jsr     _flush_data_cache
        jmp     _main
```

* Schreibt **nichts** in Sysvars, loescht nichts. Nur Datencache leeren,
  dann Sprung auf `_main`.
* `_main` (`bios/startup.S:115-116`) ist `bra.w _realmain` — der normale
  Einsprung des Kaltstarts; EmuTOS durchlaeuft also **denselben Startcode**
  wie nach dem Einschalten, und der Unterschied zwischen warm und kalt ergibt
  sich allein aus dem **Zustand der Magic-Werte im RAM** (s. 1.3).
* Einziger weiterer Eintritt: `kill_program` (`panicasm.S:93-100`) macht
  `Pterm(-1)` und faellt, falls das zurueckkehrt, in `_warm_reset`
  („should not happen!", `:100`). Siehe `_exception-crash.md`.

### 1.2 `cold_reset` (`panicasm.S:109-140`) [gelesen]

Reihenfolge:

1. `clr.l resvalid.w` und `clr.l resvector.w` (`:115-116`) — kein Sprung auf
   einen residenten Reset-Vektor mehr.
2. `#if CONF_DETECT_FIRST_BOOT_WITHOUT_MEMCONF`: `clr.l _warm_magic.w`
   (`:117-118`) — naechster Boot zaehlt als erster Boot.
3. Dann je Maschine (`#ifdef`-Kette):
   * `MACHINE_AMIGA` (`:120-126`), `MACHINE_FIREBEE` (`:127-130`): hier
     nicht relevant.
   * `CONF_ATARI_HARDWARE` (`:131-135`): `clr.l memvalid.w` („Invalidate
     memvalid"), `jsr _flush_data_cache`, `move.l 0x004,a0` („The 2 first
     longs are mapped to the ROM"), `jmp (a0)`.
   * sonst (`:136-139`): `jsr _flush_data_cache`, Kommentar „We don't know how
     to cold reset, so just warm reset this OS", `jmp _main`.
4. Auffaellig [gelesen]: `cold_reset` loescht **nur `memvalid`** (nicht
   `memval2`/`memval3`); das genuegt, weil `memchk` alle drei prueft
   (1.3). `cold_reset` setzt **nichts** auf 0 ausser den genannten Longs; die
   eigentliche Sysvar-Loeschung macht erst `memconf` beim naechsten Boot.
5. [Schluss] Auf einer Maschine ohne Atari-Hardware und ohne eigenen Zweig
   (also NewTOS, wenn man den Code 1:1 uebertraegt) ist `cold_reset` in
   EmuTOS ein `warm_reset` mit gestutzten Magic-Werten; ein echter
   Hardware-Neustart ist nur ueber den Sprung durch die ROM-Vektoren (`0x004`)
   bzw. die plattformeigenen Zweige modelliert. Fuer die Pi gibt es in EmuTOS
   keinen Zweig.

### 1.3 Was ein Warmstart uebersteht, was ein Kaltstart loescht

Alles unter `_realmain` (`bios/startup.S:335`) laeuft in beiden Faellen:

1. `bios/startup.S:335-345` `move #0x2700,sr`; (nur TT-MMU) ACSI-DMA-Toggle `:347-348`.
2. VBR-Reset `:403-407`; `reset`-Befehl (Hardware-Reset) `:408-432`, die Befehle selbst `:424`/`:431`, nur
   `#if CONF_WITH_RESET && !MACHINE_AMIGA`. Default `CONF_WITH_RESET` = 1 fuer
   ROM, 0 bei `EMUTOS_LIVES_IN_RAM` oder ColdFire (`include/config.h:1195-1208`).
3. Cartridge-Pruefung (`#if CONF_WITH_CARTRIDGE`, `bios/startup.S:442-462`).
4. Caches aus (`:465-472`), MMU aus (`:475-495`).
5. **Reset-Vektor-Pruefung** `resetvec:` (`bios/startup.S:502-520`):
   `resvalid == 0x31415926` **und** `resvector != 0` **und** gerade
   (`btst #0`), dann `a6 = resetvec`, `jmp (resvector)`. Rueckkehr nach `a6`
   fuehrt wieder zur Pruefung — der Handler muss `resvalid` selbst loeschen,
   sonst Endlosschleife (Kommentar `:516-518`). Sonst `noreset:`.
6. `jmp meminit` (`bios/startup.S:527`) → `memchk` (`bios/memory.S:117`):
   * `#if !EMUTOS_LIVES_IN_RAM` (ROM-Bau): `memvalid == 0x752019f3` **und**
     `memval2 == 0x237698aa` **und** `memval3 == 0x5555aaaa`
     (`memory.S:125-140`); eine Abweichung → `memconf` (RAM neu vermessen,
     `:528 memconf_done:` setzt `MEMINIT_BIT_FIRST_BOOT`, `:530`).
   * `EMUTOS_LIVES_IN_RAM`: die Pruefung wird **uebersprungen** („always
     assume that settings of previous OS are correct", `memory.S:119-123`).
   * alle gueltig: `memctrl`/`phystop` (und bei `CONF_WITH_TTRAM`
     `ramvalid`/`ramtop`) werden aus den Sysvars uebernommen
     (`memory.S:146-152`); `memory_reused:` (`:173`).
   * `#if CONF_DETECT_FIRST_BOOT_WITHOUT_MEMCONF` (`:174-184`): erster Boot
     auch ohne `memconf`, wenn `_warm_magic != 0x5741524D` („WARM_MAGIC").
     Default 0 fuer ROM, 1 fuer `EMUTOS_LIVES_IN_RAM`
     (`include/config.h:1408-1420`).
7. `stram_detected:` (`memory.S:553`):
   * **Erster Boot** (`MEMINIT_BIT_FIRST_BOOT`, `:559`): `bzero` des
     gesamten Sysvar-Bereichs `sysvars_start..sysvars_end` (`:567-569`),
     `memctrl`/`phystop` neu schreiben (`:572-573`), `memvalid`/`memval2`/
     `memval3` setzen (`:577-586`), ggf. `_warm_magic = 0x5741524D`
     (`:589-592`), TT-RAM-Werte restaurieren (`:596-600`).
   * **Alle Boots** (`any_boot:` `memory.S:605`): BSS (`__bss..__ebss`) und
     `.low_stram`-Variablen werden geloescht (`:606-616`),
     `meminit_flags` ins BSS gerettet (`:619`).
8. `jmp _biosmain` (`bios/startup.S:535`).

Daraus [gelesen, aus 5-7]:

| Zustand | Warmstart (Magics gueltig, ROM-Bau) | Kaltstart (`cold_reset`: `memvalid`=0, `resvalid`=`resvector`=0) |
|---|---|---|
| Sysvars `$400..sysvars_end` | **unveraendert** (inkl. `resvalid`, `resvector`, `memval*`, `phystop`, `bootdev`, `_hz_200` …) | `bzero` (`memory.S:567-569`), danach von `bios_init` neu gefuellt |
| `phystop`, `memctrl` | aus Sysvars uebernommen (`:146-147`) | neu ermittelt (`memconf`) |
| `memvalid/2/3` | bleiben | neu gesetzt (`:577-586`) |
| BSS / `.low_stram` | geloescht (`:606-616`) | geloescht |
| Crash-Dump `$380..$3FF` | bleibt (liegt ausserhalb des bzero; `_boot-init.md` §2) | bleibt ebenfalls |
| Reset-Vektor `resvalid==0x31415926` | **wird angesprungen**, falls gesetzt (`bios/startup.S:502-519`) | nie (von `cold_reset` geloescht, `panicasm.S:115-116`) |
| Benutzervektoren | nur die ersten 64 neu (`bios/vectors.S:120-137`, „avoid overwriting crash dumps") | alle 192 (`init_user_vec(FIRST_BOOT)`, `bios/bios.c:144`) |
| `FIRST_BOOT`-Zweige in `bios.c` | uebersprungen: Speichertest (`:482`, `CONF_WITH_MEMORY_TEST`), Boot-Delay (`:492`), `bootdev` bleibt (`:798-799`), Willkommensbild/`initinfo` nur wenn `FIRST_BOOT` (`:806`) | ausgefuehrt |

Der Reset-Vektor-Pfad (5) ist ein **Vertrag fuer residente Programme**
(`resvalid`/`resvector`); EmuTOS selbst setzt beides nie (`_boot-init.md` §5).

## 2. Tastenkombination (`bios/ikbd.c`)

`kbd_int(UBYTE scancode)` (`ikbd.c:725`) prueft **als Erstes** (`:734-747`)
[gelesen]:

```c
if ((scancode == KEY_DELETE)
    && ((shifty & (MODE_ALT|MODE_CTRL|MODE_LSHIFT)) == (MODE_ALT|MODE_CTRL))) {
    if (shifty & MODE_RSHIFT) cold_reset();   /* :738-741 */
    else                      warm_reset();   /* :742-745 */
}
```

Exakte Bedingungen:

* `scancode == KEY_DELETE` (`0x53`, `ikbd.c:86`) — der **Make-Code**; der
  Break-Code ist `0xD3` (`KEY_RELEASED 0x80`, `:48`) und passt nicht. Der
  Vergleich nutzt das Roh-Byte, nicht `scancode_only`.
* `shifty` (der Modifier-Zustand aus `kbd_int`, `:827-845` setzen,
  `:777-806` loeschen; Bitwerte `biosdefs.h:90-95`: `MODE_RSHIFT 0x01`,
  `MODE_LSHIFT 0x02`, `MODE_CTRL 0x04`, `MODE_ALT 0x08`): **Ctrl und Alt
  gesetzt, LShift nicht gesetzt**. Die Maske `(ALT|CTRL|LSHIFT)` betrachtet
  RShift, CapsLock, Home, Insert nicht.
* `MODE_RSHIFT` gesetzt → `cold_reset()` (Ctrl+Alt+RShift+Del), sonst
  `warm_reset()` (Ctrl+Alt+Del). LShift zusaetzlich gedrueckt → **weder
  noch**; die Taste wird dann normal weiterverarbeitet.
* `shifty` wird beim Booten in `kbd_init` auf 0 gesetzt (`ikbd.c:1102`).
* Die Pruefung steht **vor** der Auswertung von Break/Make und **vor** dem
  Mausemulations-Zweig; es gibt keine Entprellung, keine Anwendungsabfrage,
  keinen Dateiabschluss — EmuTOS springt sofort (s. 1.1: kein
  Flush ausser dem Datencache).
* Einziger Aufrufer von `kbd_int` ist der Standard-`kbdvec`
  (`aciavecs.S:372-376`, `docs/api/xbios/22-kbdvbase.md`); ein anderer
  Tastaturpfad (USB) existiert in EmuTOS' `bios/` nicht [gelesen, Grep].
  Ersetzt ein Programm `kbdvec`, ist die Kombination weg.

### 2.1 „Taste nach dem Neustart noch gehalten“ (`ikbd.c:1009-1056`) [gelesen]

Kommentar `:1009-1030`: Wird per Ctrl+Alt+Del neu gestartet und die Tasten
bleiben gehalten, sendet ein echter IKBD nach dem Reset-Befehl binnen ~100 ms
`0x9d` (Ctrl los), `0xf1` (Version), `0x1d` (Ctrl), `0x38` (Alt), `0x53`
(Del). EmuTOS wuerde darin einen neuen Neustart sehen und ewig neu starten;
Atari-TOS umgeht das mit einer Pause von ~312 ms nach dem Reset.

Abhilfe in `ikbd_reset()` (`:1032-1056`), aufgerufen aus `kbd_init`
(`:1088`):

1. Reset-Befehl `ikbd_writew(0x8001)` (`:1036`).
2. Schleife: Bytes lesen (`ikbd_readb(300)`, 300-ms-Timeout, `:1041`);
   Timeout (0) → aufgeben, `return` (`:1042-1043`); Byte mit
   `(b & 0xf0) == 0xf0` ist die Versionskennung → Schleife verlassen
   (`:1045-1049`). Die vorangehende `0x9d` wird dabei verworfen.
3. Danach alle noch anstehenden Bytes verwerfen: `while(ikbd_readb(5));`
   (`:1053-1054`) — das schluckt `0x1d 0x38 0x53`.
4. Das passiert **vor** `shifty = 0` (`:1102`) und bevor der Tastatur-
   Interrupt Tasten verarbeiten kann; deshalb ist `kbd_int` fuer die
   verworfenen Bytes nie aufgerufen worden.

[Schluss] Die Schutzmassnahme haengt an einer **IKBD-spezifischen**
Eigenheit (der Controller sendet die gehaltenen Tasten nach seinem Reset
neu). Ob eine NewTOS-Tastatur (UART-IKBD oder USB) dasselbe tut, folgt
nicht aus dem EmuTOS-Quelltext; bei USB ist zudem offen, wann der Zustand
„Taste gehalten“ nach dem Neustart erkannt wird. Entscheidung D.

## 3. OSHEADER-Reset-Einsprung (`bios/startup.S:73-116`)

Der Kopf (`_os_header`, `bios/startup.S:77`) [gelesen]:

| Off | Label | Wert | Zeile |
|---|---|---|---|
| 0x00 | `os_entry` | `bra.s _main` | `bios/startup.S:78-79` |
| 0x02 | `os_version` | `.dc.w TOS_VERSION` | `:80-81` |
| 0x04 | `reseth` | `.dc.l _main` | `:82-83` |
| 0x08 | `os_beg` | `.dc.l _os_header` | `:88-89` |
| 0x0C | `os_end` | `.dc.l __endvdibss` | `:90-91` |
| **0x10** | **`os_res1`** (typedef-Name `os_rsvl`, `biosdefs.h`) | **`.dc.l _main`** („reserved") | **`:92-93`** |
| 0x14 | `os_magic` | `.dc.l _ui_mupb` | `:94-95` |
| 0x18.. | `os_date` … `os_dummy` | s. `docs/api/structures/osheader.md` | `:96-113` |

* `_main:` (`bios/startup.S:115-116`) ist `bra.w _realmain`, „MUST be 4 bytes
  long“ — Kommentar `:116` (Erweiterungskopf `OSXH` liegt
  dazwischen, s. Text vor `:_realmain`).
* **Wer an `os_entry` (+0x00), `reseth` (+0x04) oder `os_res1` (+0x10)
  springt, landet in `_main` und damit im vollen Neustart** (Abschnitt 1.3):
  `reset`-Befehl, Reset-Vektor-Pruefung, `memchk`, `biosmain`. Es ist ein
  Sprung ohne Rueckkehr; es gibt keinen Parameter.
* Der **Zustand der Magics entscheidet warm/kalt**: ein Aufrufer, der vorher
  `memvalid`/`memval2` (und `resvalid`/`resvector`) nullt, erzwingt den
  Kaltstart mit Sysvar-Loeschung (`memory.S:125-139` → `memconf`). Genau das
  tut TeraDesk (`sdk/apps/teradesk/main.c:2097-2100`) vor dem Sprung.
* Die Kommentar-Aussage `bios/startup.S:85-87` („os_end and os_res1 below are never
  used“) betrifft nur den Fall, dass `os_magic` ungueltig ist; sie sagt
  nicht, dass der Wert 0 waere.
* `swv_vec = os_header.reseth` (`bios/bios.c:552`) nutzt denselben Einsprung fuer
  den Monitorwechsel (`screen.c:1329`): ein Aufruf von `(*swv_vec)()` ist ein
  Neustart [gelesen].

**Berichtigung (Auftrag RST01/X04):** `docs/api/structures/osheader.md`
§`os_rsvl` behauptete „EmuTOS sets `os_rsvl = 0`“. Das ist **falsch**:
`bios/startup.S:92-93` legt `_main` ab. Korrigiert; siehe dort.
`docs/api/bios/_boot-init.md:191` hatte die Zeile schon richtig.

## 4. CONF-Gating fuer das Falcon/TOS4-Profil (`make 512`, `TARGET_512`)

Tabelle: `docs/api/_emutos-config.md` (erzeugt; „1“ = fuer alle Maschinen
mitkompiliert, nicht „im Falcon-Profil an“). Relevante Schalter, jeweils
Definition im Quelltext [gelesen]:

| Schalter | `TARGET_512` | Wirkung hier | Beleg |
|---|---|---|---|
| `EMUTOS_LIVES_IN_RAM` | 0 (nur `TARGET_PRG/FLOPPY/..` setzen 1) | ROM-Bau: `memchk` prueft die Magics wirklich (`memory.S:119-140`) | `include/config.h:36-40` |
| `CONF_WITH_RESET` | 1 | `reset`-Befehl in `_realmain` | `include/config.h:1195-1208`; `_emutos-config.md:586` |
| `CONF_DETECT_FIRST_BOOT_WITHOUT_MEMCONF` | 0 | `_warm_magic` wird **nicht** ausgewertet/geloescht; warm/kalt allein ueber `memvalid*` | `include/config.h:1408-1420`; `_emutos-config.md:621` |
| `CONF_WITH_CARTRIDGE` | 1 | Modulport-Pruefung vor dem Reset-Vektor (`bios/startup.S:442-462`) | `include/config.h:1013-1023`; `_emutos-config.md:561` |
| `CONF_ATARI_HARDWARE` | (Atari) | waehlt den `cold_reset`-Zweig `panicasm.S:131-135` (Sprung ueber `0x004`) | `panicasm.S:131` |
| `CONF_WITH_MEMORY_TEST` | siehe Tabelle | Speichertest nur bei `FIRST_BOOT` (`bios/bios.c:478-487`) | `bios/bios.c:478` |
| `CONF_WITH_IKBD_ACIA` | 1 | 6850-ACIA; fuer NewTOS UART-IKBD, kein ACIA (`_emutos-config.md:575`) | `ikbd.c:1064-1070` |

[Schluss] Fuer NewTOS entfallen `reset`-Befehl, Cartridge, ACIA, `memconf`
(keine RAM-Vermessung per Busfehler, `_boot-init.md` §2) — das sind
Hardware-Ebene, nicht Vertrag. Vertrag bleibt: Tastenkombination (2),
OSHEADER-Einsprung (3), `memvalid*`/`resvalid`/`resvector`-Konvention (1.3).

## 5. Was der Kern heute tut (nur Befund, kein Auftrag)

* Der Lader schreibt die klassischen Sysvars **je Prozess** in Seite 0
  (`tos-api/src/bdos/process/loader.rs:1123-1260`, `populate_sysvars`):
  `memvalid/memval2/memval3` gesetzt (`:1176-1180`), `resvalid` und
  `resvector` = 0 (`:1181-1184`), `os_entry`/`reseth` = 0 (`:1127-1131`),
  `os_rsvl` (+0x10) = 0 (`:1150`, Kommentar dort falsch), `os_root`/`os_kbshift` = 0
  (`:1161-1162`). [gelesen] Daraus [Schluss]: Was ein Programm in seine
  Seite-0-Kopie schreibt (TeraDesk: `resvalid = 0` …), erreicht den Kern
  nicht; der Kern muss einen Neustart ueber **eigene** Zustaende fuehren.
* TeraDesk-Zugriffe `memval`, `memval2`, `resvector`, `resvalid`
  (`sdk/apps/teradesk/main.c:57-60`) sind als `long`/Zeiger-Schreibzugriffe auf LP64 **8 Byte**
  breit [gelesen: Makros; Breite Schluss aus LP64-`long`]: `memval`@0x420
  ueberdeckt 0x424..0x427, `resvector`@0x42A ueberdeckt 0x42E..0x431
  (= `phystop` der Prozesskopie), `memval2`@0x43A ueberdeckt 0x43E..0x441.
  Nur relevant, wenn der Kern die Kopie je liest.

## 6. Umsetzung (TASK/RST01, 2026-10-17; Stand §5 damit ueberholt)

* **Tastenkombination:** `tos_api::bios::reset::reset_key` ist die Pruefung aus
  §2 (Make-Code `0x53`, `shifty & (ALT|CTRL|LSHIFT) == ALT|CTRL`, RShift →
  kalt); der Standard-`kbdvec` des Kerns (`kernel/src/ikbd.rs`, `decode_key`)
  ruft sie als Erstes, fuer IKBD-Bytes aus jeder Quelle (USB-HID-Uebersetzer,
  Injektor). Dafuer fuehrt der Decoder LShift und RShift jetzt als zwei Bits
  (`docs/api/bios/0b-kbshift.md`).
* **Neustart:** `kernel/src/reset.rs` — warm = kalt (Anwenderentscheidung 2):
  Datencache per Set/Way leeren (ARM ARM D7.5.9.16.1), dann PM-Watchdog
  (`kernel/src/drivers/bcm2711/pm.rs`, `docs/hardware/pm-watchdog-reset.md`).
  `board-qemu-virt` hat keinen Mechanismus: Meldung, keine Wirkung.
* **OSHEADER:** `reseth` (+0x04) und `os_res1` (+0x10) zeigen auf einen
  Reset-Stub in Seite 0 (`0xA40`), der `Ssystem(S_NEWTOS_RESET = 2021, 0, 0)`
  ruft (NewTOS-intern, `docs/api/gemdos/154-ssystem.md`). Warm/kalt nach den
  Magics der Seite 0 des Aufrufers (`memchk`-Regel, §1.3). `os_entry` (+0x00)
  bleibt 0: ein AArch64-Befehl mit `os_version` in den oberen 16 Bit liegt im
  SVE-Kodierraum, kein Sprung moeglich (`tos-api/src/bios/reset.rs`).
* **TeraDesk:** LP64-Breiten in `sdk/overlays/teradesk/main.c` (Entscheidung 4).
* **„Taste nach dem Neustart noch gehalten“ (§2.1):** Der Watchdog-Neustart
  setzt den ganzen SoC zurueck; Firmware und Kern starten neu, der Decoder
  beginnt mit `shifty = 0`. Ein USB-Boot-Report meldet den *Zustand*: haelt der
  Anwender Ctrl+Alt+Del ueber den ganzen Firmware- und Kernstart hinweg, sieht
  der erste Report nach der Enumeration die Tasten als gedrueckt, und der Kern
  startet erneut — wie ein neuer Tastendruck. Die IKBD-Eigenheit, gegen die
  `ikbd_reset` sich wehrt (Controller sendet gehaltene Tasten ~100 ms nach
  seinem Reset neu), gibt es auf diesem Pfad nicht; keine Abhilfe gebaut.
* Belege: `measurements/measurement-2026-10-17-rst01/RUNINFO.TXT`.
