# AES — vollständiges Funktions- und Unterfunktionsinventar

**Stand:** 2026-09-12 · **Quelle:** `references/emutos` @ `1a7484aba1bfba23e2c0899e243b6d8177fe41df`
(mit `git -C references/emutos log -1` bestätigt) · **Zielprofil:** Falcon030/TOS4,
`make 512`

**Zweck.** Grundlage für eine Falltabelle, die *jede* AES-Funktion **und jede
Unterfunktion** misst. Das bestehende `TASK/COVERAGE.md` führt 79 Hauptfunktionen
und sagt selbst: „Subopcode-Vollständigkeit wird nicht behauptet." Genau die wird
hier nachgetragen.

**Abgrenzung.** Maßgeblich ist der Dispatcher `aes/gemsuper.c::crysbind`
(`:87-420`) und `include/aesdefs.h`. Was EmuTOS nicht kennt, landet im
`default`-Zweig (`:413-415`): `fm_show(ALNOFUNC, …)` für jeden Opcode ≠ 0.
**Opcode 0 wird stillschweigend ignoriert** („some PRGs are this call") — ein
eigener, messbarer Fall.

---

## 1. Opcode-Ebene: 79 Funktionen, beide Seiten vollständig

| Bereich | Opcodes | Anzahl | NewTOS |
|---|---|---|---|
| Application | 10–15, 17, 19 | 8 | alle verdrahtet |
| Event | 20–26 | 7 | alle |
| Menu | 30–39 | 10 | alle |
| Object | 40–48 | 9 | alle |
| Form | 50–56 | 7 | alle |
| Graf | 70–79 | 10 | alle |
| Scrap | 80–82 | 3 | alle |
| Fileselector | 90–91 | 2 | alle |
| Window | 100–109 | 10 | alle |
| Resource | 110–114 | 5 | alle |
| Shell | 120–127 | 8 | alle |
| **Summe** | | **79** | **79** |

Gegengezählt: `tos-api/src/aes/mod.rs::dispatch` hat 79 Arme, `crysbind` 79
`case`-Marken. **Auf Opcode-Ebene ist NewTOS vollständig.** Die Lücken liegen
eine Ebene tiefer.

### Freie Nummern im Bereich 10–127

`16, 18, 27, 28, 29, 49, 57–69, 83–89, 92–99, 115–119`

Für EmuTOS sämtlich unbehandelt. Der Fehlerweg wörtlich
(`aes/gemsuper.c:411-417`):

```c
if (unsupported)
{
    KDEBUG(("Bad AES function %d\n", opcode));
    if (opcode != 0)    /* Ignore zero since some PRGs are this call */
        fm_show(ALNOFUNC, &opcode, 1);
    ret = -1;
}
```

Also: **Rückgabe −1**, und für jeden Opcode ≠ 0 zusätzlich ein sichtbarer
Alarm. **Opcode 0 wird stillschweigend abgewiesen** — auch mit `ret = -1`, aber
ohne Alarm.

Damit sind zwei Klassen von Messfällen definiert: eine freie Nummer (−1 **plus**
Alarm) und Opcode 0 (−1 **ohne** Alarm). Der Alarm blockiert allerdings auf eine
Eingabe — in einer headless gefahrenen Tabelle gehört er deshalb in den
gesonderten Abschnitt, nicht in den Hauptlauf.

> **Zurückgenommen, 2026-09-12.** Eine frühere Fassung dieses Absatzes
> behauptete, echtes TOS belege einige dieser Nummern (`appl_search` 18,
> `appl_getinfo` 130). Das stammte aus dem Gedächtnis, nicht aus einer Quelle,
> und ist damit nach CLAUDE.md Regel 1 unzulässig. Nachgeprüft: beide Namen
> kommen im gepinnten EmuTOS-Baum **nirgends** vor. Ob echtes TOS sie kennt,
> ist damit **nicht** geklärt — es müsste aus Compendium/Profibuch belegt
> werden, bevor es hier wieder auftaucht.

---

## 2. Unterfunktionen — hier entscheidet sich die Vollständigkeit

### 2.1 `wind_get` (104) und `wind_set` (105) — `WF_*`

Konstanten: `include/aesdefs.h:533-552`. **13 ist nicht vergeben.**

**Achtung — die Konstantenliste ist nicht die Behandlungsliste.** Ein `#define`
im Header beweist nicht, dass die Funktion den Wert annimmt. Die folgende
Tabelle steht auf den tatsächlichen `switch`-Zweigen: `wm_get`
(`aes/gemwmlib.c:1505-1596`) und `wm_set` (`:1623-1740`). Beide haben ein
`default: return FALSE`.

| Wert | Name | `wind_get` | `wind_set` |
|---|---|---|---|
| 1 | `WF_KIND` | **nein** | **nein** |
| 2 | `WF_NAME` | **nein** | ja |
| 3 | `WF_INFO` | **nein** | ja |
| 4 | `WF_WXYWH` | ja | nein |
| 5 | `WF_CXYWH` | ja | ja |
| 6 | `WF_PXYWH` | ja | nein |
| 7 | `WF_FXYWH` | ja | nein |
| 8 | `WF_HSLIDE` | ja | ja |
| 9 | `WF_VSLIDE` | ja | ja |
| 10 | `WF_TOP` | ja ° | ja |
| 11 | `WF_FIRSTXYWH` | ja | nein |
| 12 | `WF_NEXTXYWH` | ja | nein |
| **13** | **nicht vergeben** | nein | nein |
| 14 | `WF_NEWDESK` | **nein** | ja |
| 15 | `WF_HSLSIZ` | ja | ja |
| 16 | `WF_VSLSIZ` | ja | ja |
| 17 | `WF_SCREEN` | ja ° | nein |
| 18 | `WF_COLOR` | ja † | ja † |
| 19 | `WF_DCOLOR` | ja °† | ja † |
| 20 | `WF_OWNER` | ja ‡ | nein |

° **kein gültiges Handle nötig** (`gemwmlib.c:1517-1519`). Alle anderen Felder
geben bei ungültigem Handle `FALSE` zurück, **bevor** der Hauptschalter greift.
Das ist ein eigener Messfall je Feld.
† nur bei `CONF_WITH_WINDOW_COLOURS` (im 512er-Zielprofil **an**).
‡ nur bei `AES_VERSION >= 0x330` — im gemessenen ROM (EmuTOS 1.4) **nicht
vorhanden**, siehe Anmerkung unten.

**Die Asymmetrie ist der interessante Teil dieser Tabelle:** `WF_NAME` und
`WF_INFO` sind **nur setzbar, nicht lesbar**; `WF_NEWDESK` ebenso. `WF_KIND` ist
definiert, aber **weder lesbar noch setzbar** — es fällt beidseitig in
`default`. Sieben der zwanzig Felder sind einseitig, eines gar nicht bedient.
Wer die Konstantenliste für die Behandlungsliste hält, misst diese Fälle als
Gutfall statt als Fehlerweg.

Nebenbefund für den Tabellenbau: `WF_COLOR`/`WF_DCOLOR` nehmen den
Gadget-Index in `pinwds[0]` und schreiben nach `poutwds[1..2]`, **nicht** nach
`poutwds[0]`.

**Anmerkung zu `WF_OWNER` (20):** kam erst mit EmuTOS-Commit `acb7f3b4`
(2025-08-23), **nach** dem `VERSION_1_4`-Tag. Das gemessene ROM
(`etos512de.img`, EmuTOS 1.4) kennt es nicht und liefert `RET_CODE = 0` plus
Stapelmüll. Bei einer Falltabelle gegen dieses ROM ist das ein
**Versionsdelta**, kein Defekt — siehe `docs/STATUS.md`, Recent Decisions
2026-09-06.

Zusätzlich je Feld messenswert: **ungültige** Werte (0, 21, 99) und ein
**ungültiges Handle** (−1, 99).

### 2.2 `wind_update` (107) — `include/aesdefs.h:558-562`

| Wert | Name |
|---|---|
| 0 | `END_UPDATE` |
| 1 | `BEG_UPDATE` |
| 2 | `END_MCTRL` |
| 3 | `BEG_MCTRL` |

Dazu die Verschachtelung (zweimal `BEG` hintereinander) und der
Freigabe-Sonderfall.

### 2.3 `wind_calc` (108) — `include/aesdefs.h:555-556`

| Wert | Name |
|---|---|
| 0 | `WC_BORDER` |
| 1 | `WC_WORK` |

Über alle `kind`-Bitmuster (`wind_create`-Gadgets, `:518-531`) — das ist die
eigentliche Kombinatorik dieser Funktion.

### 2.4 `graf_mouse` (78) — `include/aesdefs.h:423-438`

| Wert | Name | |
|---|---|---|
| 0 | `ARROW` | |
| 1 | `TEXT_CRSR` | |
| 2 | `HOURGLASS` | |
| 3 | `POINT_HAND` | |
| 4 | `FLAT_HAND` | |
| 5 | `THIN_CROSS` | |
| 6 | `THICK_CROSS` | |
| 7 | `OUTLN_CROSS` | |
| 255 | `USER_DEF` | eigene `MFORM` über `addr_in` |
| 256 | `M_OFF` | |
| 257 | `M_ON` | |
| 258 | `M_SAVE` | **AES 3.20** |
| 259 | `M_RESTORE` | **AES 3.20** |
| 260 | `M_PREVIOUS` | **AES 3.20** |

Vierzehn Unterfunktionen hinter einem Opcode.

### 2.5 `form_dial` (51) — `include/aesdefs.h:338-342`

| Wert | Name |
|---|---|
| 0 | `FMD_START` |
| 1 | `FMD_GROW` |
| 2 | `FMD_SHRINK` |
| 3 | `FMD_FINISH` |

### 2.6 `objc_edit` (46) — `include/obdefs.h:248-251`

| Wert | Name |
|---|---|
| 0 | `EDSTART` |
| 1 | `EDINIT` |
| 2 | `EDCHAR` |
| 3 | `EDEND` |

`EDCHAR` ist der Pfad, an dem der `te_txtlen`-Überlauf saß (A37) — hier gehört
ein Fall mit **vollem** Feld hin.

### 2.7 `objc_sysvar` (48) — `include/obdefs.h:129-134`

| Wert | Name |
|---|---|
| 1 | `LK3DIND` |
| 2 | `LK3DACT` |
| 3 | `INDBUTCOL` |
| 4 | `ACTBUTCOL` |
| 5 | `BACKGRCOL` |
| 6 | `AD3DVALUE` |

Jeweils **lesend und schreibend** (`ob_sysvar(mode, …)`, `aes/gemoblib.c`), dazu
der `default`-Zweig für einen ungültigen `which`-Wert.

### 2.8 `evnt_multi` (25) — Flagsmaske, `include/aesdefs.h:256-263`

| Bit | Name | |
|---|---|---|
| 0x0001 | `MU_KEYBD` | |
| 0x0002 | `MU_BUTTON` | |
| 0x0004 | `MU_M1` | |
| 0x0008 | `MU_M2` | |
| 0x0010 | `MU_MESAG` | |
| 0x0020 | `MU_TIMER` | |
| 0x003F | `MU_TOSVALID` | Maske der TOS-gültigen Bits |
| 0x0100 | `MU_M3` | **nur intern** — ein Programm darf es nicht setzen |

Eine Falltabelle kann nicht alle 64 Kombinationen blockierend fahren; messbar
sind die **nicht blockierenden**: Maske 0 und Masken, die sofort zurückkehren.
Blockierende Fälle brauchen ein Messdesign (siehe `TASK/Q04.md`).

### 2.9 `shel_write` (121) — `doex`-Modus, `include/aesdefs.h:617-621`

| Wert | Name | |
|---|---|---|
| 0 | `SHW_NOEXEC` | zurück zum Desktop |
| 1 | `SHW_EXEC` | anderes Programm starten |
| 4 | `SHW_SHUTDOWN` | System herunterfahren |
| 5 | `SHW_RESCHNG` | Auflösung wechseln |

2 und 3 sind nicht vergeben — eigener Fehlerweg-Fall. **4 und 5 sind in einer
Messtabelle gefährlich** (sie beenden bzw. schalten um) und gehören in einen
eigenen, letzten Abschnitt oder gar nicht hinein.

### 2.10 Weitere Unterfunktionsträger

| Opcode | Funktion | Unterfunktion |
|---|---|---|
| 30 | `menu_bar` | `show`-Flag 0/1 (+ AES-3.3-Erweiterungen) |
| 35 | `menu_register` | negativer `me_pid` = Deregistrierung |
| 37 | `menu_attach` | `flag`-Werte, `include/aesdefs.h:296` |
| 39 | `menu_settings` | lesend/schreibend |
| 45 | `objc_order` | `newpos` inkl. `0` (oben) und `-1` (unten) |
| 47 | `objc_change` | `redraw`-Flag 0/1 |
| 11/12 | `appl_read`/`write` | `MU_MESAG`/`MU_SDMSG` (`gemsuper.c:103`) |
| 14/15 | `appl_tplay`/`trecord` | Puffer und Zählwerte |
| 112 | `rsrc_gaddr` | **`R_*`-Typen 0–16** — siehe unten |
| 113 | `rsrc_saddr` | dieselben Typen |
| 122/123 | `shel_get`/`put` | Längen, Sonderfall Länge 0 |

### 2.11 `rsrc_gaddr` (112) / `rsrc_saddr` (113) — die `R_*`-Typen

Die größte einzelne Unterfunktionsgruppe. **Korrektur (A53, 2026-09-20):**
dieser Absatz gab noch den Task-#108-Stand wieder (6 Typen, `rsrc_saddr`
No-op) und widersprach damit bereits der Tabelle weiter unten in diesem
Dokument (§ „Der offene Widerspruch aus dem Auftrag ist entschieden"). NewTOS
deckt **alle 17** `R_*`-Typen ab (`get_addr`, `tos-api/src/aes/resource.rs:2589-2650`,
mit Zweigen für jeden Selektor `aesdefs.h:578-594`); `rsrc_saddr` ist
implementiert (`:2693-2719`), kein Stub.

**Jeder dieser Typen ist ein eigener Messfall** — und hier ist zu erwarten, dass
sich EmuTOS und NewTOS unterscheiden. Genau dafür ist die Tabelle da.

---

## 3. Was eine vollständige Falltabelle abdecken muss

1. **Jeden der 79 Opcodes** mindestens einmal im Gutfall.
2. **Jede Unterfunktion** aus Abschnitt 2 — das sind grob:
   20 `WF_*` × {get, set} + 4 `wind_update` + 2 `wind_calc` × `kind`-Muster +
   14 `graf_mouse` + 4 `form_dial` + 4 `objc_edit` + 6 `objc_sysvar` × {r, w} +
   4 `shel_write` + 15 `R_*` × {gaddr, saddr} + die Kleineren.
3. **Die Fehlerwege:** ungültige Unterfunktionsnummer, ungültiges Handle,
   Nullzeiger, freie Opcode-Nummern, Opcode 0.
4. **Reihenfolgeabhängige Fälle** — die R3-Erfahrung: `wind_get(WF_FIRSTXYWH)`
   und Verwandte hängen vom Zustand davor ab, nicht nur von den Argumenten.

### Zwei Fallen, aus den bisherigen Runden gelernt

**Nicht jeder Unterschied ist ein Defekt.** Drei Schubladen, jede mit Beleg:
NewTOS falsch · Versionsdelta EmuTOS 1.4 gegen die gepinnte Quelle ·
Geräteparameter oder kein Soll (EmuTOS' uninitialisierter Dispatcher-Stapel,
`aes/gemsuper.c:430-432,444-445`).

**Blockierende und zerstörende Aufrufe gehören getrennt.** Alles, was auf
Eingabe wartet (`evnt_*` blockierend, `form_do`, `fsel_input`, die
`graf_*box`-Verfolger) oder den Lauf beendet (`shel_write` 4/5, `appl_exit`),
darf eine Messtabelle nicht in der Mitte anhalten. Entweder eigener Abschnitt
am Ende oder eigene Tabelle.

---

## 4. Belege

* Dispatcher: `aes/gemsuper.c:87-420`, `default`-Zweig `:413-415`
* Opcode-Nummern: `include/aesdefs.h:17-115`
* Unterfunktionen: `include/aesdefs.h:256-263, 296, 338-342, 423-438, 518-562, 617-621`;
  `include/obdefs.h:129-134, 248-251`
* NewTOS-Dispatcher: `tos-api/src/aes/mod.rs::dispatch`
* Bisheriges Inventar (79 Hauptfunktionen, ohne Subopcode-Anspruch):
  `TASK/COVERAGE.md`

---

# Teil II — vollständiges Unterfunktionsinventar (Audit, 2026-09-12)

**Prüfer:** Auditor · **Quelle EmuTOS:** `references/emutos` @
`1a7484aba1bfba23e2c0899e243b6d8177fe41df` (mit `git -C references/emutos log -1`
bestätigt: „Update GitHub Actions to use Node 24", 2026-04-14) · **Zielprofil:**
`make 512`, Falcon030/TOS4 · **NewTOS-Seite:** `tos-api/src/aes/`.

**Belegregel dieses Teils.** Jede Zeile steht auf dem `switch`- bzw. `if`-Zweig,
der den Wert tatsächlich behandelt — nie auf einer `#define`-Liste. Wo die
NewTOS-Antwort nicht gelesen, sondern **gefahren** wurde, steht *gemessen* mit
dem Ergebnis.

## II.0 Profilgatter — nachgeprüft, gilt für `make 512`

`TARGET_512` wird in `include/config.h` **nirgends** ausgewertet (nur im
`Makefile:561` als `-DTARGET_512` gesetzt). Damit greift **keiner** der
Abmagerungsblöcke `#ifdef TARGET_192` (`:210-394`), `#ifdef TARGET_256`
(`:402-448`) oder `#if !CONF_ATARI_HARDWARE` (`:673-806`), und es gelten die
Vorgabewerte am Dateiende:

| Schalter | Wert | Beleg | AES-Folge |
|---|---|---|---|
| `CONF_WITH_3D_OBJECTS` | 1 | `include/config.h:1258-1260` | `objc_sysvar` (48) existiert |
| `CONF_WITH_MENU_EXTENSION` | 1 | `:1304-1306` | 36–39 existieren; `evnt_multi` maskiert auf `MU_TOSVALID` |
| `CONF_WITH_WINDOW_COLOURS` | 1 | `:1328-1330` | `WF_COLOR`/`WF_DCOLOR` |
| `CONF_WITH_GRAF_MOUSE_EXTENSION` | 1 | `:1282-1284` | `M_SAVE`/`M_RESTORE`/`M_PREVIOUS` |
| `CONF_WITH_COLOUR_ICONS` | 1 | `:1266-1268` | CICON-Fixup in `rsrc_load` |
| `CONF_WITH_PCGEM` | 1 | `:1319-1321` | 17, 82, 126, 127; `HOTCLOSE` |
| `CONF_WITH_NICELINES` | 1 | `:1312-1314` | Menü-Trennlinie in `objc_draw` |
| `CONF_WITH_SHUTDOWN` | **1** | `:2028-2034` × `DETECT_NATIVE_FEATURES = 1` (`:1908-1914`, kein ColdFire) | `SHW_SHUTDOWN`-Zweig **ist** übersetzt |
| `AES_VERSION` | **0x0340** | `:1346-1348` | `WF_OWNER`-Zweig aktiv (`#if AES_VERSION >= 0x330`) |

> **Korrektur zur Vorarbeit (2026-09-12).** Die Vorarbeit ließ
> `CONF_WITH_SHUTDOWN` offen. Es ist **1**. Der `SHW_SHUTDOWN`-Zweig existiert
> also im Referenzprofil — er ist aber durch `can_shutdown()` gedeckelt, das auf
> echter Atari-Hardware `FALSE` liefert (`bios/bios.c`, `#if
> DETECT_NATIVE_FEATURES` → `has_nf_shutdown()`, sonst `return FALSE`). Auf dem
> gemessenen TT/Falcon ist `shel_write(4,…)` daher **beobachtbar ein No-op mit
> Rückgabe 1** — genau das, was NewTOS tut. Kein Defekt.

## II.1 Versionsgatter — EmuTOS 1.4 (`etos512de.img`) gegen die gepinnte Quelle

`git log VERSION_1_4..HEAD -- aes/ include/aesdefs.h include/obdefs.h
include/gemdefs.h` liefert **genau zwei** Commits. Das ist die vollständige
Liste der AES-Versionsdeltas; alles andere in diesem Dokument ist in beiden
Ständen gleich.

| Commit | Datum | Wirkung | Messfolge |
|---|---|---|---|
| `acb7f3b4` | 2025-08-23 | `WF_OWNER` (20) in `wm_get` (`aes/gemwmlib.c:1578-1589`) | 1.4 antwortet `RET_CODE = 0` (fällt in `default`), gepinnte Quelle `1` + vier Wörter. **Versionsdelta, kein Defekt.** Bereits als `DELTAS` in `tos-api/tests/aes_reference/harness.rs:996+` geführt |
| `4b0d9518` | 2025-08-20 | „nicelines"-Heuristik: `… && (len > 3) && is_dashes(…)` statt `… && is_dashes(…)` (`aes/gemoblib.c:838`) | 1.4 zeichnet auch einen **1–3 Zeichen langen** DISABLED-`G_STRING` aus Bindestrichen als durchgezogene Linie, die gepinnte Quelle erst ab 4. Betrifft `objc_draw` (42), rein optisch |

> **Korrektur an einer bestehenden Quelle im Repo (2026-09-12).** Der
> Doc-Kommentar von `DELTAS` in `tos-api/tests/aes_reference/harness.rs:948-949`
> behauptet, `WF_OWNER` sei „absent from `include/aesdefs.h` (which ends at
> `WF_COLOR` 18)". Das ist **falsch**: `git show VERSION_1_4:include/aesdefs.h`
> führt die Liste bis **`WF_DCOLOR` 19**. Nur `WF_OWNER` (20) fehlt. Die
> `DELTAS`-Einträge selbst sind davon unberührt (sie betreffen ausschließlich
> `WF_OWNER`) — der Begründungstext gehört korrigiert.

## II.2 Zählregel und Gesamtzahl

Als **Unterfunktion** zählt hier ein diskreter Wert eines *Modus- oder
Auswahlparameters*, auf den die Implementierung verzweigt — nicht ein Datenwert
(Koordinate, Länge, Zeiger). Ein Boolean zählt als 2. Ein Bit einer Maske zählt
als 1. Der `default`-Zweig zählt gesondert als *Rückfallklasse*.

| Gruppe | Unterfunktionswerte | Rückfallklassen |
|---|---|---|
| `wind_get` `WF_*` (14 behandelt von 20 definiert) | 14 | 1 |
| `wind_set` `WF_*` (11 behandelt von 20 definiert) | 11 | 1 |
| `rsrc_gaddr` `R_*` | 17 | 1 |
| `rsrc_saddr` `R_*` | 17 | (geteilt mit `get_addr`) |
| `graf_mouse` | 14 | 1 (→ `ARROW`) |
| `objc_sysvar` (5 set + 6 get) | 11 | 2 |
| `objc_edit` `kind` (4) + `EDCHAR`-`in_char`-Klassen (6) | 10 | 1 |
| `wind_create`/`wind_calc` `kind`-Bits | 13 | — |
| `form_error` Fehlerklassen (6) + `n > 63`-Kurzschluss | 7 | — |
| `form_alert` Icon (4) + `defbut` (2) | 6 | — |
| `evnt_multi` `MU_*`-Bits (nutzbar) | 6 | — |
| `wind_update` | 4 | 2 (`<2` ≠ 0, `>3`) |
| `form_dial` | 4 | 1 |
| `shel_write` `doex` | 4 | 1 |
| `appl_tplay` `EVNTREC.ap_event` | 4 | 1 |
| `menu_attach` `ME_*` | 3 | 1 |
| `objc_order` `newpos`-Klassen (0 / `NIL` / n>0) | 3 | — |
| `evnt_button`/`evnt_multi` `bflgclks`-Hochbyte-Klassen | 3 | — |
| `menu_bar`, `menu_icheck`, `menu_tnormal`, `menu_istart`, `menu_settings`, `objc_change`, `graf_slidebox`, `evnt_dclick`, `wind_calc` `wtype`, `MOBLK.m_out` | 10 × 2 = 20 | 2 (`mn_istart`, `mn_settings`) |
| `menu_ienable` (`ENABLE_IT` 2 + Item-Bit 15 als `dodraw` 2) | 4 | — |
| **Summe** | **176** | **~15** |

Dazu die **zwei Fehlerklassen der Opcode-Ebene** (freie Nummer → −1 + Alarm;
Opcode 0 → −1 ohne Alarm) und **50 Funktionen ohne jede Unterfunktion** (II.4).

**Also: 79 Opcodes, davon 29 mit Unterfunktionen und 50 ohne, zusammen 176
diskrete Unterfunktionswerte + rund 15 Rückfallklassen + 2 Opcode-Fehlerklassen.**

---

## II.3 Die Funktionen mit Unterfunktionen — Zweig für Zweig

Spalten: **E** = EmuTOS behandelt (gepinnte Quelle) · **N** = NewTOS behandelt ·
**H** = headless in einer Falltabelle messbar (ohne Eingabe, ohne Abbruch) ·
**V** = Versionsgatter.

### 104 `wind_get` — `wm_get`, `aes/gemwmlib.c:1505-1600`

Handle-Vorprüfung: `WF_TOP`, `WF_SCREEN`, `WF_DCOLOR` brauchen **kein gültiges
Handle** (`:1516-1524`); alle anderen liefern vorher `FALSE`.

| Wert | Name | E | N | H | Anmerkung |
|---|---|---|---|---|---|
| 1 | `WF_KIND` | nein (`default`, `:1591-1592`) | nein | ja | gemessen: NewTOS `0` ✔ |
| 2 | `WF_NAME` | **nein** (`default`, `:1591-1592`) | **ja** | ja | **Abweichung A-1**, gemessen |
| 3 | `WF_INFO` | **nein** (`default`, `:1591-1592`) | **ja** | ja | **Abweichung A-1**, gemessen |
| 4 | `WF_WXYWH` | ja | ja | ja | |
| 5 | `WF_CXYWH` | ja | ja | ja | |
| 6 | `WF_PXYWH` | ja | ja | ja | |
| 7 | `WF_FXYWH` | ja | ja | ja | |
| 8 | `WF_HSLIDE` | ja | ja | ja | |
| 9 | `WF_VSLIDE` | ja | ja | ja | |
| 10 | `WF_TOP` | ja (ohne Handle) | ja | ja | |
| 11 | `WF_FIRSTXYWH` | ja | ja | ja | zustandsabhängig (Reihenfolge!) |
| 12 | `WF_NEXTXYWH` | ja | ja | ja | dito |
| 13 | — | nicht vergeben | — | ja | fällt in `default` |
| 14 | `WF_NEWDESK` | nein (`default`, `:1591-1592`) | nein | nein (Zeiger) | |
| 15 | `WF_HSLSIZ` | ja | ja | ja | |
| 16 | `WF_VSLSIZ` | ja | ja | ja | |
| 17 | `WF_SCREEN` | ja (ohne Handle) | nein (`ret=0`, A54) | ja | **Abweichung A-6**, sanktioniert (kein persistenter Puffer, `docs/COMPATIBILITY.md`) |
| 18 | `WF_COLOR` | ja | ja | nein (Index ungeprüft) | Gadget in `pinwds[0]`, Ausgabe `poutwds[1..2]` |
| 19 | `WF_DCOLOR` | ja (ohne Handle) | ja | nein (dito) | |
| 20 | `WF_OWNER` | ja (`AES_VERSION>=0x330`) | ja | ja | **V: fehlt in 1.4** |
| sonst | `default` | `FALSE` | `FALSE` | ja | |

`WF_COLOR`/`WF_DCOLOR`: EmuTOS indiziert `w_tcolor[gadget]` **ungeprüft**
(`:1568`); NewTOS klemmt auf 0..18 (`window.rs::gadget_index`). Für einen Index
außerhalb 0..18 hat EmuTOS **kein Soll** (Lesen außerhalb des Arrays) — deshalb
H = nein.

### 105 `wind_set` — `wm_set`, `aes/gemwmlib.c:1622-1757`

Handle-Vorprüfung: nur `WF_DCOLOR` kommt ohne gültiges Handle aus
(`:1631-1636`). `wm_update(BEG_UPDATE)`/`(END_UPDATE)` klammert **auch** den
`default`-Zweig (`:1637`, `:1753`) — in NewTOS fehlt die Klammer im
`default`-Arm; bei Einzelprozess-Betrieb unbeobachtbar (Zähler netto 0).

| Wert | Name | E | N | H |
|---|---|---|---|---|
| 1 `WF_KIND` | nein | nein | ja |
| 2 `WF_NAME` | ja | ja | nein (Zeiger) |
| 3 `WF_INFO` | ja | ja | nein (Zeiger) |
| 4 `WF_WXYWH` | nein | nein | ja |
| 5 `WF_CXYWH` | ja | ja | ja |
| 6 `WF_PXYWH` | nein | nein | ja |
| 7 `WF_FXYWH` | nein | nein | ja |
| 8 `WF_HSLIDE` | ja (Klemme 1..1000) | ja | ja |
| 9 `WF_VSLIDE` | ja (Klemme 1..1000) | ja | ja |
| 10 `WF_TOP` | ja | ja | ja |
| 11/12 `WF_*XYWH` | nein | nein | ja |
| 14 `WF_NEWDESK` | ja | ja | nein (Zeiger) |
| 15 `WF_HSLSIZ` | ja (−1 durchlassen) | ja | ja |
| 16 `WF_VSLSIZ` | ja (−1 durchlassen) | ja | ja |
| 17 `WF_SCREEN` | nein | nein | ja |
| 18 `WF_COLOR` | ja | ja | nein (Index) |
| 19 `WF_DCOLOR` | ja (ohne Handle) | ja | nein (Index) |
| 20 `WF_OWNER` | **nein** (kein `case`) | nein | ja |
| sonst | `FALSE` | `FALSE` | ja |

**Feinunterschied (kein Defekt):** EmuTOS zeichnet den Schieber nur neu, wenn
sich der Wert wirklich geändert hat (`:1688-1716`); NewTOS ruft
`redraw_?slider` unbedingt. Rein zeichnerisch, byte-gleiches Ergebnis.

### 107 `wind_update` — `wm_update`, `aes/gemwmlib.c:1778-1797`

```c
if (beg_update < 2) { if (beg_update) tak_flag/ev_block; else unsync; }
else { beg_update -= 2; fm_own(beg_update); }
```

**Es gibt keinen Fehlerweg.** Jeder Wert wird bedient.

| Wert | Name | E | N | H |
|---|---|---|---|---|
| 0 | `END_UPDATE` | ja | ja | ja (gemessen: 1) |
| 1 | `BEG_UPDATE` | ja | ja | ja (gemessen: 1) |
| 2 | `END_MCTRL` | ja → `fm_own(0)` | ja | ja (gemessen: 1) |
| 3 | `BEG_MCTRL` | ja → `fm_own(1)` | ja | ja (gemessen: 1) |
| <0 | — | wie `BEG_UPDATE` | ja (`m if m < 2`) | ja |
| >3 | — | `fm_own(m-2)` → *begin* | ja (`m => mctrl_own(m-2 != 0)`) | ja |

### 108 `wind_calc` — `wm_calc`, `aes/gemwmlib.c:1807-1867`

`wtype` verzweigt genau einmal: `if (wtype == WC_BORDER) { negieren }`. Also
**2 Klassen** — `0` und *alles andere* (WC_WORK inbegriffen). NewTOS
`window.rs::calc` ist zeichengleich. E = N.

`kind`-Bits, 3D-Zweig (`CONF_WITH_3D_OBJECTS = 1`):

| Bit | Name | fließt in `wm_calc` ein | N |
|---|---|---|---|
| 0x0001 `NAME` · 0x0002 `CLOSER` · 0x0004 `FULLER` · 0x0008 `MOVER` | `TGADGETS` → `tb` | ja | ja |
| 0x0010 `INFO` | `tb += gl_hbox` | ja | ja |
| 0x0020 `SIZER` | `havevbar`/`havehbar` (mit der Entweder-oder-Regel `:1828-1836`) | ja | ja |
| 0x0040/0x0080/0x0100 `VGADGETS` | `rb` | ja | ja |
| 0x0200/0x0400/0x0800 `HGADGETS` | `bb` | ja | ja |
| **0x1000 `HOTCLOSE`** | nein (nur Control Manager) | **in NewTOS gar nicht vorhanden** — **Abweichung A-7** |

### 78 `graf_mouse` — `gr_mouse`, `aes/gemgrlib.c:419-455` (Gesamt), `default:` `:426-429`

`gr_mouse` ist **`void`**; `crysbind` lässt `ret = TRUE`. Es gibt keinen
Fehlerweg. Der `default:`-Zweig fängt **alles** außerhalb
{255,256,257,258,259,260} ab und klemmt `mode < ARROW || mode > OUTLN_CROSS` auf
`ARROW`.

| Wert | Name | E | N | H | V |
|---|---|---|---|---|---|
| 0–7 | `ARROW`…`OUTLN_CROSS` | ja | ja | nur per Screendump | — |
| 255 | `USER_DEF` | ja (`MFORM` aus `addr_in[0]`) | ja, **ohne** `mf_nplanes`/`mf_fg`/`mf_bg` | nein | — |
| 256 | `M_OFF` | ja (zählend) | ja | nein | — |
| 257 | `M_ON` | ja (zählend) | ja | nein | — |
| 258 | `M_SAVE` | ja (`rlr->p_mouse`, je Prozess) | ja (global) | nein | — |
| 259 | `M_RESTORE` | ja | ja | nein | — |
| 260 | `M_PREVIOUS` | ja (`gl_prevmouse`) | ja | nein | — |
| 8…254, 261…, <0 | — | **ja: → `ARROW`** | **nein: Cursor bleibt** | Screendump | **Abweichung A-3** |

Gemessen: die Rückgabe ist auf beiden Seiten für 2/8/254/261/−1 gleich `1`; der
Unterschied liegt allein im Cursorbild und braucht einen Screendump.

### 51 `form_dial` — `fm_dial`, `aes/gemfmlib.c:360-383`

Kein `default`, immer `TRUE`.

| Wert | Name | E | N | H |
|---|---|---|---|---|
| 0 `FMD_START` | ja (Kommentar-No-op) | ja | ja |
| 1 `FMD_GROW` | ja (`gr_growbox`) | ja | ja (zeichnet) |
| 2 `FMD_SHRINK` | ja | ja | ja (zeichnet) |
| 3 `FMD_FINISH` | ja (`w_drawdesk` + `w_update`) | ja | ja (zeichnet) |
| sonst | nur `gsx_sclip`, `TRUE` | `_ => {}`, `1` | ja (gemessen: `form_dial(99)` → 1) |

### 46 `objc_edit` — `ob_edit`, `aes/gemobed.c:335-463`

Vorbedingung: `if ((kind == EDSTART) || (obj <= 0)) return TRUE;` — **ohne**
`*idx` zu ändern. NewTOS ergänzt `tree.is_null()` (EmuTOS würde dereferenzieren;
kein Soll).

| Wert | Name | E | N | H |
|---|---|---|---|---|
| 0 `EDSTART` | ja (sofort `TRUE`) | ja | ja |
| 1 `EDINIT` | ja (`*idx = strlen(rawstr)`) | ja | ja |
| 2 `EDCHAR` | ja | ja | ja |
| 3 `EDEND` | ja (nur Cursor löschen) | ja | ja |
| sonst | kein `case`, fällt durch, `TRUE` | `_ => {}` | ja |

`EDCHAR`-Innenschalter auf `in_char` (`:379-442`):

| Zeichen | E | N |
|---|---|---|
| `BACKSPACE` | ja | ja |
| `ESCAPE` (Feld leeren) | ja | ja |
| `DELETE` | ja | ja |
| `ARROW_LEFT` | ja | ja |
| `ARROW_RIGHT` | ja | ja |
| sonst (Einfügen mit `te_pvalid`-Prüfung, `te_txtlen`-Überlaufpfad, `scan_to_end`-Vorwärtsspringen) | ja | ja |

### 48 `objc_sysvar` — `ob_sysvar`, `aes/gemoblib.c:1350-1433`

`mode != 0` = setzen, `mode == 0` = lesen.

| `which` | Name | set E | set N | get E | get N | H |
|---|---|---|---|---|---|---|
| 1 | `LK3DIND` | ja (−1 = unverändert) | ja | ja (2 Werte) | ja | ja |
| 2 | `LK3DACT` | ja | ja | ja | ja | ja |
| 3 | `INDBUTCOL` | ja (`in1 >= ws_ncolors` → 0) | ja | ja (1 Wert) | ja | ja |
| 4 | `ACTBUTCOL` | ja | ja | ja | ja | ja |
| 5 | `BACKGRCOL` | ja | ja | ja | ja | ja |
| 6 | `AD3DVALUE` | **nein → 0** | **nein → 0** | ja (`ADJ3DSTD`, `ADJ3DSTD`) | ja | ja |
| sonst | | 0 | 0 | 0 | 0 | ja |

**Gemessen und deckungsgleich:** `set(6)=0`, `get(6)=1` mit `(2,2)`, `get(99)=0`.

*Nur-Lesen-Klasse:* EmuTOS schreibt bei `INDBUTCOL`/`ACTBUTCOL`/`BACKGRCOL` nur
`*out1`; `*out2` bleibt der uninitialisierte Stapel von `xif`
(`gemsuper.c:429`). NewTOS schreibt dort 0. **Kein Soll** — die dokumentierte
Klasse „uninitialisierter Dispatcher-Stapel".

*Negative Farbe:* `in1 = -1` besteht die Prüfung `in1 >= ws_ncolors` auf beiden
Seiten und wird gespeichert. Gleich.

### 25 `evnt_multi` — `ev_multi`, `aes/gemevlib.c:151-293`

`crysbind` übergibt bei `CONF_WITH_MENU_EXTENSION` **`MU_FLAGS & MU_TOSVALID`**
und `pmo3 = NULL` (`gemsuper.c:152-155`). `MU_M3` ist damit für eine Anwendung
**unerreichbar**. NewTOS maskiert gleich (`event.rs`: `& MU_TOSVALID`).

| Bit | Name | E | N | H |
|---|---|---|---|---|
| 0x0001 `MU_KEYBD` | ja | ja | nur wenn eine Taste schon anliegt |
| 0x0002 `MU_BUTTON` | ja | ja | ja, wenn `downorup` den Ist-Zustand trifft |
| 0x0004 `MU_M1` | ja | ja | ja, wenn die Maus schon drin/draußen ist |
| 0x0008 `MU_M2` | ja | ja | dito |
| 0x0010 `MU_MESAG` | ja | ja | ja, wenn eine Nachricht anliegt |
| 0x0020 `MU_TIMER` | ja | ja | **ja, garantiert, mit `tmcount == 0`** |
| 0x0100 `MU_M3` | wegmaskiert | wegmaskiert | — |

> **Korrektur zur Vorarbeit (2026-09-12).** Die Vorarbeit nennt „Maske 0" als
> messbaren Fall. Das ist **falsch und gefährlich**: bei `flags == 0` registriert
> `ev_multi` nichts und läuft in `mwait(0)`; `mwait` prüft `!(mask &
> rlr->p_evflg)` — mit `mask = 0` immer wahr — setzt `WAITIN` und ruft
> `dsptch()` (`aes/gemasync.c:117-127`). Der Prozess wird **nie** wieder geweckt.
> `evnt_multi(0,…)` hängt EmuTOS dauerhaft. NewTOS verhält sich gleich (Schleife
> ohne Austrittsbedingung). Der Fall gehört **nicht** in eine Falltabelle.

`bflgclks`-Hochbyte (`downorup`, `aes/geminput.c:348-358`): `flag = (buparm >>
24) & 0xff` ist das **Hochbyte von `bflgclks`**, und `((mask & (val^new)) == 0)
!= flag` vergleicht 0/1 gegen dieses ganze Byte. Drei Klassen: `0` = auf Eintritt
warten, `1` = auf Verlassen warten, **jeder andere Wert (auch das dokumentierte
`0x80`) = Bedingung immer wahr**. NewTOS (`event.rs::downorup`) rechnet
zeichengleich. E = N.

`MOBLK.m_out` (`in_mrect`, `aes/geminput.c:76-79`): 2 Werte, `m_out != inside`
(0 = warten, bis die Maus hineinkommt; 1 = bis sie herausgeht). NewTOS
`rect_event` gleich. Dieselben zwei Werte sind die **einzige** Unterfunktion
von **22 `evnt_mouse`** (`ev_mouse` -> `ev_block(MU_M1, pmo)`,
`aes/gemevlib.c:123-128`) und von `MU_M1`/`MU_M2` in `evnt_multi`.

### 121 `shel_write` — `sh_write`, `aes/gemshlib.c:157-196`

Kein `default`; jeder unbekannte `doex` ist ein No-op mit `TRUE`.

| Wert | Name | E | N | H | Anmerkung |
|---|---|---|---|---|---|
| 0 | `SHW_NOEXEC` | ja | ja | **nein — beendet zum Desktop** | |
| 1 | `SHW_EXEC` | ja | ja | **nein — startet ein Programm** | |
| 2, 3 | nicht vergeben | No-op, `TRUE` | No-op, `1` | ja (gemessen: 1) | **Korrektur:** kein Fehlerweg |
| 4 | `SHW_SHUTDOWN` | `case` vorhanden, aber `can_shutdown()` = `FALSE` auf Atari-HW → No-op, `TRUE` | No-op, `1` | ja | deckungsgleich auf dem Referenzprofil |
| 5 | `SHW_RESCHNG` | setzt `gl_changerez = 1+isover`, `gl_nextrez = isgem`, leert `s_cmd`/`ad_stail` | **nur** `s_cmd`/`s_tail` geleert | **nein — schaltet um** | **Abweichung A-5** |

### 112/113 `rsrc_gaddr` / `rsrc_saddr` — `get_addr`, `aes/gemrslib.c:112-176`

**Alle 17 Typen sind auf beiden Seiten verzweigt.** `rs_gaddr` gibt
`(*rsaddr != (void *)-1L)` zurück, `rs_saddr` schreibt in jeden auflösbaren Slot.

| Wert | Name | E | N | Identitätspaar |
|---|---|---|---|---|
| 0 | `R_TREE` | ja (`ap_ptree[i]`) | ja | |
| 1 | `R_OBJECT` | ja | ja | |
| 2 | `R_TEDINFO` | ja | ja | ≡ 8 |
| 3 | `R_ICONBLK` | ja | ja | ≡ 11 |
| 4 | `R_BITBLK` | ja | ja | ≡ 14 |
| 5 | `R_STRING` | ja (dereferenziert) | ja | |
| 6 | `R_IMAGEDATA` | ja (dereferenziert) | ja | |
| 7 | `R_OBSPEC` | ja (`&obj->ob_spec`) | ja | |
| 8 | `R_TEPTEXT` | ja | ja | ≡ 2 |
| 9 | `R_TEPTMPLT` | ja | ja | |
| 10 | `R_TEPVALID` | ja | ja | |
| 11 | `R_IBPMASK` | ja | ja | ≡ 3 |
| 12 | `R_IBPDATA` | ja | ja | |
| 13 | `R_IBPTEXT` | ja | ja | |
| 14 | `R_BIPDATA` | ja | ja | ≡ 4 |
| 15 | `R_FRSTR` | ja | ja | |
| 16 | `R_FRIMG` | ja | ja | |
| sonst | `(void*)-1L` → `FALSE` | `None` → `0` | |

> **Der offene Widerspruch aus dem Auftrag ist entschieden: `docs/STATUS.md` ist
> veraltet.** Zeile 244 behauptet, `rsrc_gaddr` decke nur
> `R_TREE`/`R_OBJECT`/`R_STRING`/`R_IMAGEDATA`/`R_FRSTR`/`R_FRIMG` ab und
> `rsrc_saddr` sei ein No-op-Stub. Belegt in
> `tos-api/src/aes/resource.rs:2589-2650` (`get_addr` mit Zweigen für alle 17)
> und `:2693-2719` (`rsrc_saddr` schreibt den Slot über
> `slot.access` in `ptr_size`-Breite). **Korrigiert (A53, 2026-09-20) — siehe
> `docs/STATUS.md` Zeile 244.** (Zeilenangaben hier stammen vom 2026-09-12 und
> waren seither verschoben; oben auf den heutigen Stand nachgezogen.)

Zwei echte Unterschiede in diesem Block, beide **Abweichung A-4**:

1. **Bereichsprüfung.** EmuTOS' `get_addr` prüft *keinen* Index; `base + size*i`
   wird immer gebildet und ist ≠ `-1`, also `TRUE`. NewTOS prüft gegen die
   geladene Tabellenlänge und liefert `0`/`FALSE`.
2. **Rückgabekriterium.** EmuTOS: `TRUE`, sofern die Adresse ≠ `-1` ist — ein
   **Null**-Slot (`R_STRING` auf einen `NULL`-Zeiger) ergibt `TRUE` mit
   `addr_out[0] = 0`. NewTOS: `TRUE`, sofern die Adresse ≠ `0` ist.

### 45 `objc_order` — `ob_order`, `aes/gemoblib.c:1201-1246`

| Wert | Bedeutung | E | N |
|---|---|---|---|
| `mov_obj == ROOT` | | `FALSE` | `false` |
| `new_pos == 0` | an den Kopf | ja | ja |
| `new_pos == NIL` (−1) | ans Ende | ja | ja |
| `new_pos > 0` | hinter das (n−1)-te Geschwister | ja, **ungeklemmt** | ja, **geklemmt** |

**Gemessen, Alleinkind + `new_pos = 0`:** NewTOS liefert `head=1, tail=1,
next=0`; EmuTOS liefert nach Quelllage `head=1, **tail=−1**, **next=−1**` (der
`ob_delete` hat `ob_tail` auf `NIL` gesetzt, der Kopfzweig schreibt
`ob_next = chg_obj = NIL`, und `if (movptr->ob_next == parent)` greift nicht).
**Abweichung A-8.**

**Gemessen, `new_pos = 99` auf drei Kindern:** NewTOS klemmt ans Ende
(`head=2, tail=1`). EmuTOS folgt 98-mal `ob_next`, läuft über den Elternknoten
hinaus und indiziert am Ende `tree[-1]` — **kein Soll**, NewTOS' Klemme ist
zulässig, gehört aber benannt.

### 39 `menu_settings` — `mn_settings`, `aes/gemmnext.c:596-616`

| Wert | Bedeutung | E | N | H |
|---|---|---|---|---|
| 0 | lesen | ja | ja | ja |
| 1 | schreiben (negative Felder ignorieren, `height` klemmen) | ja | ja | ja |
| sonst | **No-op** (kein `default`) | ja | **nein → schreibt** | ja |

**Gemessen:** nach `menu_settings(1, display=111)` und `menu_settings(2,
display=222)` liest NewTOS `222` zurück; EmuTOS müsste `111` behalten.
**Abweichung A-2.**

### 37 `menu_attach` / 38 `menu_istart`

`mn_attach` (`gemmnext.c:535-557`): Vorprüfung `(tree[item].ob_type & 0xff) !=
G_STRING` → `FALSE`, dann `ME_INQUIRE` 0 / `ME_ATTACH` 1 / `ME_REMOVE` 2, kein
`default` (`rc` bleibt `FALSE`). N = E, plus die Null-/Negativ-Wächter.

`mn_istart` (`gemmnext.c:562-591`): Vorprüfung „Submenü gefunden?" → `FALSE`,
dann `0` = abfragen, `1` = setzen (auf `ob_head`..`ob_tail` geklemmt),
`default: return FALSE`. **Rückgabe ist `submenu->s_start`, nicht `TRUE`.**
N = E (`menu.rs::menu_istart` gibt `s.sub_start` zurück).

### 30–35 Menü-Flags

| Opcode | Selektor | E | N |
|---|---|---|---|
| 30 `menu_bar` | `SHOW_IT` 0 / ≠0 | ja | ja |
| 31 `menu_icheck` | `CHECK_IT` 0/1 → `do_chg(…, CHECKED, …, dodraw=FALSE, chkdisabled=FALSE)` | ja | ja |
| 32 `menu_ienable` | `ENABLE_IT` 0/1 **und Item-Bit 15 = `dodraw`** (`gemsuper.c:175-176`) | ja | ja |
| 33 `menu_tnormal` | `NORMAL_IT` 0/1, `dodraw=TRUE`, **`chkdisabled=TRUE`** → `0` bei DISABLED | ja | ja |
| 34 `menu_text` | — | `strcpy` ohne Neuzeichnen | gleich |
| 35 `menu_register` | — | `pid < 0` **oder** `g_accreg >= NUM_ACCS` → `-1` | `pid < 0` **oder** kein freier Slot → `-1` |

> **Korrektur zur Vorarbeit (2026-09-12).** Die Vorarbeit führt für
> `menu_register` „negativer `me_pid` = Deregistrierung" und für `menu_bar`
> „+ AES-3.3-Erweiterungen". Beides steht **nicht** im gepinnten EmuTOS:
> `mn_register` gibt bei `pid < 0` schlicht `-1` zurück (`gemmnlib.c:789-818`, Prüfung `:794`),
> und `mn_bar` kennt nur `showit` wahr/falsch (`gemmnlib.c:703-732`). Ob echtes
> TOS ≥ 3.3 mehr kann, ist damit **unentschieden** — es müsste aus
> Compendium/Profibuch belegt werden.

### 53 `form_error` — `fm_error`, `aes/gemfmlib.c:436-472`

| `n` | Meldung | E | N | H |
|---|---|---|---|---|
| > 63 | **keine**, sofort `FALSE` | ja | ja | **ja** (einziger nicht blockierender Zweig) |
| 2, 3, 18 | `AL18ERR` | ja | ja | nein (Alarm blockiert) |
| 4 | `AL04ERR` | ja | ja | nein |
| 5 | `AL05ERR` | ja | ja | nein |
| 15 | `AL15ERR` | ja | ja | nein |
| 8, 10, 11 | `AL08ERR` | ja | ja | nein |
| sonst (auch negativ) | `ALXXERR` mit `%u` | ja | ja | nein |

Kleiner Rest: EmuTOS' `sprintf("%u", …)` auf ein negatives `n` druckt den
vorzeichenlosen Wert, NewTOS' `write_decimal` das Vorzeichen. Nur im Alarmtext
sichtbar, der ohnehin blockiert.

### 52 `form_alert` — `fm_alert`/`fm_build`, `aes/gemfmalt.c:163,260-270,319-396`

| Selektor | Werte | E | N | H |
|---|---|---|---|---|
| Icon (`alert[1] - '0'`) | 0 = keins · 1 = `NOTEBB` · 2 = `QUESTBB` · **alles andere** = `STOPBB` | ja | ja | nein |
| `defbut` | 0 = keiner · sonst `BUTOFF+defbut-1` bekommt `DEFAULT` | ja | ja | nein |

### 14 `appl_tplay` — `ap_tplay`, `aes/gemaplib.c:119-186`

Unterfunktionen sind die `EVNTREC.ap_event`-Codes (`aes/gemaplib.h:24-27`):
`TCHNG` 0, `BCHNG` 1, `MCHNG` 2, `KCHNG` 3; alles andere wird übersprungen.
**Alle vier: E = ja, N = nein** (siehe A-9).

### 21/26 `evnt_button`, `evnt_dclick`

`ev_dclick(rate, setit)` (`gemevlib.c:300-309`): `setit ≠ 0` setzt, Rückgabe ist
immer `gl_dcindex`. **Keine Bereichsprüfung** — `gl_dcrates[gl_dcindex]` mit
`rate > 4` oder `< 0` liest außerhalb des 5er-Arrays (`:33`). Kein Soll.
NewTOS speichert den Rohwert. 2 Unterfunktionen (setzen/lesen), E = N.

---

## II.4 Funktionen **ohne** Unterfunktion — ausdrücklich festgehalten

Für diese 50 Opcodes ist jeder Parameter ein Datenwert, kein Modus. Es gibt
nichts tiefer zu messen als den Opcode selbst (plus Grenzwerte/Fehlerwege).

`appl_init` 10 · `appl_read` 11 · `appl_write` 12 · `appl_find` 13 ·
`appl_trecord` 15 · `appl_yield` 17 · `appl_exit` 19 · `evnt_keybd` 20 ·
`evnt_mesag` 23 · `evnt_timer` 24 ·
`menu_text` 34 · `menu_register` 35 · `menu_popup` 36 · `objc_add` 40 ·
`objc_delete` 41 · `objc_draw` 42 · `objc_find` 43 · `objc_offset` 44 ·
`form_do` 50 · `form_center` 54 · `form_keybd` 55 · `form_button` 56 ·
`graf_rubberbox` 70 · `graf_dragbox` 71 · `graf_movebox` 72 · `graf_growbox` 73 ·
`graf_shrinkbox` 74 · `graf_watchbox` 75 · `graf_handle` 77 · `graf_mkstate` 79 ·
`scrp_read` 80 · `scrp_write` 81 · `scrp_clear` 82 · `fsel_input` 90 ·
`fsel_exinput` 91 · `wind_open` 101 · `wind_close` 102 · `wind_delete` 103 ·
`wind_find` 106 · `wind_new` 109 · `rsrc_load` 110 · `rsrc_free` 111 ·
`rsrc_obfix` 114 · `shel_read` 120 · `shel_get` 122 · `shel_put` 123 ·
`shel_find` 124 · `shel_envrn` 125 · `shel_rdef` 126 · `shel_wdef` 127.

> **Korrektur zur Vorarbeit (2026-09-12).** Die Vorarbeit führt `appl_read`/
> `appl_write` mit der Unterfunktion „`MU_MESAG`/`MU_SDMSG`". Das ist **keine**
> Unterfunktion: `crysbind` leitet den Code aus dem **Opcode** ab
> (`opcode == APPL_READ ? MU_MESAG : MU_SDMSG`, `gemsuper.c:103`), der Aufrufer
> kann ihn nicht wählen. Ebenso ist `graf_slidebox`' `isvert` in der Vorarbeit
> nicht geführt — es ist eine (2-wertige) Unterfunktion und steht jetzt oben.

Einziger nicht blockierender Zweig von 90/91: `pipath == NULL` → `FALSE`
(`gemfslib.c:629-631`). E = N, headless messbar.

---

## II.5 Messbarkeit — was in eine headless gefahrene Falltabelle darf

**Abschnitt A — frei messbar (kein Warten, kein Abbruch).**
Alle `wind_*` außer den zeigertragenden Feldern; `objc_sysvar` beide
Richtungen; `objc_add/delete/order/change/find/offset`; `objc_edit` alle vier
`kind`; `form_dial` alle vier + Fehlwert; `form_center`; `form_error(n > 63)`;
`graf_handle`; `graf_mkstate`; `menu_*` außer `menu_popup`; `rsrc_*` alle;
`scrp_read`/`scrp_write`; `shel_read/get/put/find/envrn/rdef/wdef`;
`shel_write(2|3|4)`; `appl_init/find/yield/read/write`; `evnt_dclick`;
`evnt_multi` **nur** mit `MU_TIMER` und `tmcount == 0`; `fsel_*` mit
`pipath == NULL`; Opcode 0.

> **Korrektur (A53, 2026-09-20).** Diese Zeile führte bis hierher auch
> „alle freien Opcodes" — im Widerspruch zu **Teil I** oben und zu
> Abschnitt B unten, die beide sagen, dass jede freie Opcode-Nummer **≠ 0**
> den blockierenden `ALNOFUNC`-Alarm auslöst (`aes/gemsuper.c:411-417`) und
> damit **nicht** headless/frei messbar ist. Nur **Opcode 0** ist frei
> messbar (`-1` ohne Alarm); Abschnitt B nennt den Alarm-Fall bereits
> korrekt. Die fehlerhafte Nennung ist aus der Aufzählung oben entfernt
> (`TASK/A53.md` „r4-Tabellenbau-Nachtrag").

**Abschnitt B — blockiert auf Eingabe, eigener Abschnitt oder eigene Tabelle.**
`evnt_keybd` 20, `evnt_button` 21, `evnt_mouse` 22, `evnt_mesag` 23,
`evnt_timer` 24, `evnt_multi` mit jeder anderen Maske (einschließlich **Maske 0
= hängt für immer**), `form_do` 50, `form_alert` 52, `form_error` mit `n ≤ 63`
(Alarm), `form_keybd`/`form_button` (nicht blockierend, aber zustandsabhängig),
`graf_rubberbox` 70, `graf_dragbox` 71, `graf_watchbox` 75, `graf_slidebox` 76,
`menu_popup` 36, `fsel_input` 90 / `fsel_exinput` 91 mit gültigem Pfad,
`appl_trecord` 15 (wartet auf ein Ereignis, `gemaplib.c:216-217`),
**der `ALNOFUNC`-Alarm** hinter jeder freien Opcode-Nummer ≠ 0.

**Abschnitt C — beendet oder schaltet um, gehört ans Ende oder gar nicht hinein.**
`shel_write(0)` (zurück zum Desktop), `shel_write(1)` (startet ein Programm),
`shel_write(5)` (Auflösungswechsel), `appl_exit` 19, `scrp_clear` 82 (löscht
Dateien), `rsrc_free` 111 (gibt den Speicher des Resources frei),
`wind_new` 109 (nur als deklarierter `!reset` unschädlich).

**Abschnitt D — nur per Screendump.**
`graf_mouse` alle Formen, `objc_draw`s „niceline", das Zeichnen von
`form_dial(1|2|3)`, `menu_bar`, jedes `dodraw`.

---

## II.6 Abweichungsliste — NewTOS gegen die gepinnte EmuTOS-Quelle

Sortiert nach Schwere. *gemessen* = auf der NewTOS-Seite reproduziert
(temporäres Prüfgerüst über `tos-api/tests/aes_reference/harness.rs`, nach dem
Audit wieder entfernt).

| # | Funktion / Unterfunktion | EmuTOS | NewTOS | Beleg | Status |
|---|---|---|---|---|---|
| A-1 | `wind_get(WF_NAME)` (2), `wind_get(WF_INFO)` (3) | `default: return FALSE`, Puffer unberührt | `1` + Text in den Puffer kopiert | `aes/gemwmlib.c:1591-1592` vs. `tos-api/src/aes/window.rs:2740-2754` | **Defekt**, gemessen |
| A-2 | `menu_settings(flag ∉ {0,1})` | No-op (kein `default`) | schreibt wie `flag == 1` | `aes/gemmnext.c:596-616` vs. `tos-api/src/aes/menu.rs::menu_settings` | **Defekt**, gemessen |
| A-9 | `appl_tplay` (14), `appl_trecord` (15) | spielt/zeichnet den Ereignisband ab bzw. auf | `pb.set_ret(1)` / `pb.set_ret(0)`, Argumente ignoriert | `aes/gemaplib.c:119-186,189-221` vs. `tos-api/src/aes/appl.rs::appl_tplay,appl_trecord` | **Stub** (Regel 2), gelesen |
| A-3 | `graf_mouse(n)` für n ∉ {0..7, 255..260} | `default:` klemmt auf `ARROW` und **setzt** den Cursor | Cursor bleibt unverändert | `aes/gemgrlib.c:426-429` vs. `tos-api/src/aes/graf.rs::graf_mouse` `_ => {}` | **Defekt**, nur per Screendump prüfbar |
| A-10 | `objc_draw` „nicelines" | DISABLED-`G_STRING` aus ≥ 4 Bindestrichen wird als durchgezogene Linie gezeichnet (`CONF_WITH_NICELINES = 1`) | gar nicht vorhanden — zeichnet die Bindestriche | `aes/gemoblib.c:833-841` vs. kein Treffer in `tos-api/src/` | **Defekt** (optisch), gelesen |
| A-8 | `objc_order` mit Alleinkind, `new_pos = 0` | `ob_tail = NIL`, `ob_next = NIL` (kaputter Baum, aber das ist das Soll) | `ob_tail = mov_obj`, `ob_next = parent` (repariert) | `aes/gemoblib.c:1209-1246` vs. `tos-api/src/aes/object.rs::ob_order` | **Abweichung**, gemessen |
| A-7 | `wind_create` `kind`-Bit `HOTCLOSE` (0x1000) | Control Manager schickt `WM_CLOSED` sofort beim Druck (`aes/gemctrl.c:90,210`), `CONF_WITH_PCGEM = 1` | Bit existiert nicht | kein Treffer für `HOTCLOSE` in `tos-api/src/` | **Lücke**, nur auf echter HW prüfbar |
| A-5 | `shel_write(SHW_RESCHNG)` (5) | `gl_changerez = 1 + isover`, `gl_nextrez = isgem` → AES startet in der neuen Auflösung neu | leert nur `s_cmd`/`s_tail`, kein Auflösungswechsel | `aes/gemshlib.c:187-191` vs. `tos-api/src/aes/shell.rs::shel_write` | **Lücke**, gelesen |
| A-4 | `rsrc_gaddr`/`rsrc_saddr` Randfälle | keine Indexprüfung; `TRUE`, sofern Adresse ≠ `-1` (also auch für einen **Null**-Slot) | Indexprüfung; `TRUE`, sofern Adresse ≠ `0` | `aes/gemrslib.c:112-176,717-742` vs. `tos-api/src/aes/resource.rs:2589-2719` | **Abweichung** (EmuTOS teils ohne Soll), gelesen |
| A-6 | `wind_get(WF_SCREEN)` (17) | Adresse + Länge des persistenten Menü-/Alarm-Sicherungspuffers (`gsx_mret`) | `ret=0`, nichts geschrieben (vor A54: vier Nullwörter **und** `1`) | `aes/gemwmlib.c:1563-1565` vs. `window.rs` (Fix + Begründung: `docs/COMPATIBILITY.md` "wind_get(WF_SCREEN)") | **kein Architekturentscheid, sondern gemessener Defekt** — die r4-Messung (Fälle 331/332/439) widerlegt die frühere Einstufung als "bewusste Abweichung"; behoben und als sanktionierte Abweichung dokumentiert, siehe `TASK/A54.md` |
| A-11 | freier Opcode ≠ 0 | `fm_show(ALNOFUNC, &opcode, 1)` — sichtbarer Alarm, dann `-1` | nur `-1`, kein Alarm | `aes/gemsuper.c:411-417` vs. `tos-api/src/aes/mod.rs::crysbind` `_`-Arm (dokumentiert als D-084) | **bewusste Abweichung**, gemessen (`-1` stimmt) |
| A-12 | `appl_read` (11) bei leerer Schlange | blockiert (`ap_rdwr` → `ev_block(MU_MESAG)`) | `0` | `aes/gemaplib.c:96-106` vs. `tos-api/src/aes/appl.rs::appl_read` | **Abweichung**, gelesen |
| A-13 | `appl_write` (12) bei voller Schlange | blockiert den Absender | `0` | bereits als AUD-P1-006 geführt | **Abweichung**, gelesen |
| A-14 | `graf_mouse(USER_DEF)` | `gsx_mfset` übergibt auch `mf_nplanes`, `mf_fg`, `mf_bg` | nur Hotspot + Masken/Daten | `aes/gemgrlib.c:454` vs. `tos-api/src/aes/graf.rs::set_user_form` | **kleine Lücke** (Cursorfarbe), gelesen |
| A-15 | `graf_mouse(M_SAVE/M_RESTORE)` | je Prozess (`rlr->p_mouse`) | global | `aes/gemgrlib.c:440-444` | **unbeobachtbar bei Einzelprozess** |
| A-16 | `scrp_clear` (82) | EmuTOS baut den Löschpfad als `<dir>` + `'S'` + Dateiname (`strcpy(ptmp + 1, …)`, `aes/gemsclib.c:96`) und löscht damit vermutlich nichts | baut `<dir><Dateiname>` und löscht wirklich | `aes/gemsclib.c:89-99` vs. `tos-api/src/aes/shell.rs::scrp_clear` | **unentschieden** — sieht nach einem EmuTOS-Fehler aus; nicht gefahren |

---

## II.7 Was **nicht** entschieden ist

* **A-16** (`scrp_clear`s `ptmp + 1`): gelesen, nicht gefahren. Ob EmuTOS 1.4
  dort wirklich nichts löscht, wäre ein Hatari-Lauf — aber `scrp_clear` löscht
  Dateien und gehört nicht in eine Messtabelle. Als Beobachtung geführt.
* **A-3, A-10, A-7**: die Wirkung ist sichtbar, nicht als Rückgabewert messbar.
  A-3 und A-10 brauchen einen Screendump, A-7 einen Klick auf echter Hardware.
* **`menu_register` / `menu_bar` jenseits von EmuTOS**: ob echtes TOS ≥ 3.3 eine
  Deregistrierung über einen negativen `me_pid` oder weitere `show`-Werte kennt,
  ist aus dem gepinnten EmuTOS **nicht** zu belegen. Unentschieden; Beleg müsste
  aus Compendium/Profibuch kommen.
* **`objc_draw`/`objc_find` `depth`**: als Zählwert geführt, nicht als
  Unterfunktion. Das Grenzverhalten (`depth = 0`, `depth < 0`) ist nicht
  gefahren.
* **`wind_create` `kind`-Kombinatorik**: die 13 Bits sind einzeln belegt, die
  8192 Kombinationen nicht gefahren. `aes-r1.tsv` misst einen Teil davon über
  `wind_calc`.
