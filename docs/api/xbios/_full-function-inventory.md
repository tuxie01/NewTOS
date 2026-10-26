# XBIOS — vollständiges Funktions- und Unterfunktionsinventar

**Stand:** 2026-09-14 · **Quelle:** `references/emutos` @
`1a7484aba1bfba23e2c0899e243b6d8177fe41df` (mit `git -C references/emutos log -1`
bestätigt) · **Zielprofil:** `make 512`, Falcon030/TOS4 · **NewTOS-Seite:**
HEAD `19871c0`.

Formvorlage: `docs/api/aes/_full-function-inventory.md`.

> **Vollständigkeitsgrad.** Die **Opcode-Ebene ist vollständig** (alle 142
> Tabellenplätze, beide Seiten). Die **Unterfunktionsebene ist es nicht**: für
> die drei großen Gerätegruppen — TT-Shifter (0x50–0x57), Videl (0x58–0x5F),
> DSP (0x60–0x7F) und DMA-Sound (0x80–0x8D) — sind die Unterfunktionen
> **nicht erhoben**. Sie sind in §2.9 als offen gekennzeichnet, nicht
> weggelassen.

---

## 0. Methode und Abgrenzung

**Die Liste kommt aus der Vektortabelle, nicht aus Kopfdateien.**

* `bios/xbios.c:1285-1515` — `xbios_vecs[]`;
  `const UWORD xbios_ent = ARRAY_SIZE(xbios_vecs);` (`:1517`),
* `bios/xbios.c:1273-1283` — `LAST_ENTRY`, das die Tabellenlänge steuert,
* `bios/vectors.S:1275-1277` — der Trap-14-Einsprung `_xbiostrap`, und
  `biosxbios` (`:1282-1333`), derselbe Verteiler wie beim BIOS.

### Die Tabellenlänge hängt am Profil

```c
#if CONF_WITH_DMASOUND
# define LAST_ENTRY 0x8d
#elif CONF_WITH_DSP
# define LAST_ENTRY 0x7f
#elif CONF_WITH_VIDEL
# define LAST_ENTRY 0x5f
#elif CONF_WITH_TT_SHIFTER
# define LAST_ENTRY 0x57
#else
# define LAST_ENTRY 0x40
#endif
```

`bios/xbios.c:1273-1283`. Mit `cc -E -dM -Iinclude -DTARGET_512` gegen
`include/config.h` gemessen ist `CONF_WITH_DMASOUND = 1`, also
**`LAST_ENTRY = 0x8d`** und die Tabelle hat **142 Plätze** (0x00…0x8D).

### Die zwei Nicht-Wege

| Fall | EmuTOS | Beleg |
|---|---|---|
| Nummer **innerhalb** der Tabelle, Platz = `xbios_unimpl` | `_xbios_unimpl` → `xbios_do_unimpl(d0)` → **`return number;`** | `bios/vectors.S:1366-1370`, `bios/xbios.c:1251-1257` |
| Nummer **≥ `xbios_ent`** | `cmp.w d1,d0` / `jhs bx_ret_exc` — `d0` enthält noch die Nummer → **die Nummer wird zurückgegeben** | `bios/vectors.S:1324-1326,1334` |

**Beide Nicht-Wege geben dieselbe Zahl zurück: die Funktionsnummer.**
Es gibt im XBIOS **keinen** `EINVFN`. Ein Prüfprogramm kann eine
unimplementierte von einer implementierten Funktion also nur dann unterscheiden,
wenn deren Gutfall-Rückgabe ≠ der Nummer ist.

NewTOS bildet das nach: `other => other as i64`
(`tos-api/src/xbios/mod.rs:136`).

### Profilgatter — nachgeprüft für `make 512`

| Schalter | Wert | XBIOS-Folge |
|---|---|---|
| `CONF_WITH_DMASOUND` | **1** | 0x80–0x8D existieren; `LAST_ENTRY = 0x8d` |
| `CONF_WITH_DSP` | **1** | 0x60–0x7F existieren |
| `CONF_WITH_VIDEL` | **1** | 0x58–0x5B, 0x5D–0x5F existieren |
| `CONF_WITH_TT_SHIFTER` | **1** | 0x50–0x57 existieren |
| `CONF_WITH_MFP` | **1** | 0x0D `Mfpint`, 0x1A `Jdisint`, 0x1B `Jenabint`, 0x1F `Xbtimer` existieren |
| `CONF_WITH_PRINTER_PORT` | **1** | 0x21 `Setprt` existiert |
| `CONF_WITH_NVRAM` | **1** | 0x2E `NVMaccess` existiert |
| `CONF_WITH_ACSI` / `CONF_WITH_FDC` | **1** | 0x08–0x0A, 0x13, 0x29–0x2B tun echte Arbeit |

**Im Zielprofil ist kein einziger XBIOS-Platz durch einen `CONF_WITH_*`-Schalter
abgeschaltet.** Alle 39 leeren Plätze sind fest `xbios_unimpl`.

### Was „verdrahtet" heißt

**ja** = echte Implementierung · **HW-Adapter** = fester, im Vertrag definierter
Wert für abwesende Atari-Peripherie · **Nummer** = fällt in den Auffangarm und
gibt die Funktionsnummer zurück (also „ehrlich nicht vorhanden", denselben Weg
wie EmuTOS' `xbios_unimpl`).

---

## 1. Opcode-Ebene: 142 Plätze, 103 belegt

NewTOS-Verteiler: `tos-api/src/xbios/mod.rs::dispatch` (`:64-138`).

### 1.1 Die 103 belegten Plätze

| Nr | Name | EmuTOS Tabelle | Gatter | N | NewTOS |
|---|---|---|---|---|---|
| 0x00 | `Initmous` | `bios/xbios.c:1286` | — | **Nummer** | `tos-api/src/xbios/mod.rs:138` |
| 0x02 | `Physbase` | `:1288` | — | ja | `tos-api/src/xbios/mod.rs:67` |
| 0x03 | `Logbase` | `:1289` | — | ja | `:68` |
| 0x04 | `Getrez` | `:1290` | — | ja | `:69` |
| 0x05 | `Setscreen` | `:1291` | — | ja | `:70` |
| 0x06 | `Setpalette` | `:1292` | — | ja | `:71` |
| 0x07 | `Setcolor` | `:1293` | — | ja | `:72` |
| 0x08 | `Floprd` | `:1294` | — | HW-Adapter `EUNDEV` | `:80` |
| 0x09 | `Flopwr` | `:1295` | — | HW-Adapter `EUNDEV` | `:80` |
| 0x0A | `Flopfmt` | `:1296` | — | HW-Adapter `EUNDEV` | `:80` |
| 0x0C | `Midiws` | `:1298` | — | HW-Adapter (No-op) | `:89` |
| 0x0D | `Mfpint` | `:1300` | `CONF_WITH_MFP` | **Nummer** | `:138` |
| 0x0E | `Iorec` | `:1304` | — | **Nummer** | `:138` |
| 0x0F | `Rsconf` | `:1305` | — | ja (PL011-Adapter) | `:93` |
| 0x10 | `Keytbl` | `:1306` | — | ja | `:99` |
| 0x11 | `Random` | `:1307` | — | ja | `:100` |
| 0x12 | `Protobt` | `:1308` | — | ja | `:81` |
| 0x13 | `Flopver` | `:1309` | — | HW-Adapter `EUNDEV` | `:80` |
| 0x14 | `Scrdmp` | `:1310` | — | HW-Adapter (No-op) | `:88` |
| 0x15 | `Cursconf` | `:1311` | — | ja | `:73` |
| 0x16 | `Settime` | `:1312` | — | ja | `:104` |
| 0x17 | `Gettime` | `:1313` | — | ja | `:108` |
| 0x18 | `Bioskeys` | `:1314` | — | **Nummer** | `:138` |
| 0x19 | `Ikbdws` | `:1315` | — | HW-Adapter (No-op) | `:111` |
| 0x1A | `Jdisint` | `:1317` | `CONF_WITH_MFP` | **Nummer** | `:138` |
| 0x1B | `Jenabint` | `:1318` | `CONF_WITH_MFP` | **Nummer** | `:138` |
| 0x1C | `Giaccess` | `:1323` | — | HW-Adapter `0` | `:114` |
| 0x1D | `Offgibit` | `:1324` | — | HW-Adapter (No-op) | `:115` |
| 0x1E | `Ongibit` | `:1325` | — | HW-Adapter (No-op) | `:116` |
| 0x1F | `Xbtimer` | `:1327` | `CONF_WITH_MFP` | **Nummer** | `:138` |
| 0x20 | `Dosound` | `:1331` | — | HW-Adapter `0` | `:117` |
| 0x21 | `Setprt` | `:1333` | `CONF_WITH_PRINTER_PORT` | ja (reine Software) | `:121` |
| 0x22 | `Kbdvbase` | `:1337` | — | ja (Tabelle mit Nullvektoren) | `:109` |
| 0x23 | `Kbrate` | `:1338` | — | ja (Softwareschatten) | `:124` |
| 0x25 | `Vsync` | `:1340` | — | ja | `:125` |
| 0x26 | `Supexec` | `:1341` | — | **Nummer** | `:138` |
| 0x29 | `Floprate` | `:1344` | — | HW-Adapter `EUNDEV` | `:80` |
| 0x2A | `DMAread` | `:1345` | — | HW-Adapter `EUNDEV` | `:80` |
| 0x2B | `DMAwrite` | `:1346` | — | HW-Adapter `EUNDEV` | `:80` |
| 0x2C | `Bconmap` | `:1347` | — | HW-Adapter (`0x2c` = „nicht vorhanden") | `:90` |
| 0x2E | `NVMaccess` | `:1350` | `CONF_WITH_NVRAM` | **Nummer** | `:138` |
| 0x40 | `Blitmode` | `:1371` | — | HW-Adapter `0x0000` | `:128` |
| 0x50 | `EsetShift` | `:1392` | `CONF_WITH_TT_SHIFTER` | **Nummer** | `:138` |
| 0x51 | `EgetShift` | `:1393` | dito | **Nummer** | `:138` |
| 0x52 | `EsetBank` | `:1394` | dito | **Nummer** | `:138` |
| 0x53 | `EsetColor` | `:1395` | dito | **Nummer** | `:138` |
| 0x54 | `EsetPalette` | `:1396` | dito | **Nummer** | `:138` |
| 0x55 | `EgetPalette` | `:1397` | dito | **Nummer** | `:138` |
| 0x56 | `EsetGray` | `:1398` | dito | **Nummer** | `:138` |
| 0x57 | `EsetSmear` | `:1399` | dito | **Nummer** | `:138` |
| 0x58 | `VsetMode` | `:1412` | `CONF_WITH_VIDEL` | **Nummer** | `:138` |
| 0x59 | `VgetMonitor` (`vmontype`) | `:1413` | dito | **Nummer** | `:138` |
| 0x5A | `VsetSync` | `:1414` | dito | **Nummer** | `:138` |
| 0x5B | `VgetSize` | `:1415` | dito | **Nummer** | `:138` |
| 0x5D | `VsetRGB` | `:1417` | dito | **Nummer** | `:138` |
| 0x5E | `VgetRGB` | `:1418` | dito | **Nummer** | `:138` |
| 0x5F | `Vfixmode` | `:1419` | dito | **Nummer** | `:138` |
| 0x60–0x7F | 32 × `Dsp_*` | `:1432-1463` | `CONF_WITH_DSP` | **Nummer** (alle 32) | `:138` |
| 0x80–0x8D | 14 × DMA-Sound (`Locksnd`…`Buffptr`) | `:1500-1513` | `CONF_WITH_DMASOUND` | **Nummer** (alle 14) | `:138` |

Die DSP-Namen im Einzelnen (`bios/xbios.c:1432-1463`): `Dsp_DoBlock` 0x60,
`Dsp_BlkHandShake` 0x61, `Dsp_BlkUnpacked` 0x62, `Dsp_InStream` 0x63,
`Dsp_OutStream` 0x64, `Dsp_IOStream` 0x65, `Dsp_RemoveInterrupts` 0x66,
`Dsp_GetWordSize` 0x67, `Dsp_Lock` 0x68, `Dsp_Unlock` 0x69,
`Dsp_Available` 0x6A, `Dsp_Reserve` 0x6B, `Dsp_LoadProg` 0x6C,
`Dsp_ExecProg` 0x6D, `Dsp_ExecBoot` 0x6E, `Dsp_LodToBinary` 0x6F,
`Dsp_TriggerHC` 0x70, `Dsp_RequestUniqueAbility` 0x71,
`Dsp_GetProgAbility` 0x72, `Dsp_FlushSubroutines` 0x73,
`Dsp_LoadSubroutine` 0x74, `Dsp_InqSubrAbility` 0x75,
`Dsp_RunSubroutine` 0x76, `Dsp_Hf0` 0x77, `Dsp_Hf1` 0x78, `Dsp_Hf2` 0x79,
`Dsp_Hf3` 0x7A, `Dsp_BlkWords` 0x7B, `Dsp_BlkBytes` 0x7C, `Dsp_HStat` 0x7D,
`Dsp_SetVectors` 0x7E, `Dsp_MultBlocks` 0x7F.

Die DMA-Sound-Namen (`bios/xbios.c:1500-1513`): `Locksnd` 0x80, `Unlocksnd` 0x81,
`Soundcmd` 0x82, `Setbuffer` 0x83, `Setsndmode` 0x84, `Settracks` 0x85,
`Setmontracks` 0x86, `Setinterrupt` 0x87, `Buffoper` 0x88, `Dsptristate` 0x89,
`Gpio` 0x8A, `Devconnect` 0x8B, `Sndstatus` 0x8C, `Buffptr` 0x8D.

### 1.2 Die 39 `xbios_unimpl`-Plätze

| Nr | Kommentar in der Tabelle | Zeile |
|---|---|---|
| 0x01 | `ssbrk` | `bios/xbios.c:1287` |
| 0x0B | `used_by_bios` | `:1297` |
| 0x24 | `prtblk` | `:1339` |
| 0x27 | `puntaes` | `:1342` |
| 0x28 | — | `:1343` |
| 0x2D | — | `:1348` |
| 0x2F–0x3F | 17 Füllplätze | `:1354-1370` |
| 0x41–0x4F | 15 Füllplätze | `:1374-1388` |
| 0x5C | — | `:1416` |

**Alle 39 geben die Funktionsnummer zurück** (`xbios_do_unimpl`,
`bios/xbios.c:1251-1257`). NewTOS ebenso, über denselben Auffangarm (`tos-api/src/xbios/mod.rs:138`).

> **Anmerkung.** `Ssbrk` (0x01) und `Puntaes` (0x27) sind in echtem TOS belegt.
> Im gepinnten EmuTOS sind sie **nicht** implementiert — die Tabelle sagt
> `xbios_unimpl`. Was echtes TOS dort tut, ist aus dieser Quelle **nicht**
> belegbar und gehört, wenn es gebraucht wird, aus Compendium/Profibuch.

### 1.3 Summenzeile

| | Plätze |
|---|---|
| `xbios_vecs[]` (0x00…0x8D, `LAST_ENTRY = 0x8d`) | **142** |
| davon belegt | **103** |
| davon `xbios_unimpl` | **39** |

| | Anzahl |
|---|---|
| EmuTOS belegt (Zielprofil) | **103** |
| davon in NewTOS beantwortet | **33** |
| — mit echter Implementierung | **17** (0x02–0x07, 0x0F, 0x10, 0x11, 0x12, 0x15, 0x16, 0x17, 0x21, 0x22, 0x23, 0x25) |
| — als HW-Adapter mit Vertragswert | **16** (0x08, 0x09, 0x0A, 0x0C, 0x13, 0x14, 0x19, 0x1C, 0x1D, 0x1E, 0x20, 0x29, 0x2A, 0x2B, 0x2C, 0x40 — davon 7 auf einem gemeinsamen `EUNDEV`-Arm) |
| in NewTOS **nicht** beantwortet (Auffangarm, gibt die Nummer zurück) | **70** |
| unter einem bei uns abgeschalteten Schalter | **0** |
| NewTOS-Erweiterungen über EmuTOS hinaus | **0** |

Die 70 nicht beantworteten: `Initmous` 0x00 · `Mfpint` 0x0D · `Iorec` 0x0E ·
`Bioskeys` 0x18 · `Jdisint` 0x1A · `Jenabint` 0x1B · `Xbtimer` 0x1F ·
**`Supexec` 0x26** · `NVMaccess` 0x2E · TT-Shifter 0x50–0x57 (8) ·
Videl 0x58–0x5B, 0x5D–0x5F (7) · DSP 0x60–0x7F (32) ·
DMA-Sound 0x80–0x8D (14).

**Gegengezählt:** `tos-api/src/xbios/mod.rs:65-137` hat 27 `match`-Arme; der
Sammelarm `:79` deckt 7 Nummern ab, alle anderen je eine → **33 Nummern**.
142 − 33 = 109 Nummern fallen auf `:136` (39 davon sind auch in EmuTOS leer).

> **`Supexec` (0x26) ist der einzige der 70, der keine Atari-Peripherie
> braucht.** Er ist reine Ausführungssemantik („rufe diese Funktion im
> Supervisor-Modus"). Das ist hier als **Faktum** vermerkt, nicht bewertet.

---

## 2. Unterfunktionen

### 2.1 `Setscreen` (0x05) — `rez` und die Zeiger-Sonderwerte

`setscreen`, `bios/screen.c:1102-1166`.

| Parameter | Klasse | Wirkung | Beleg |
|---|---|---|---|
| `logLoc` | `> 0` | `v_bas_ad = logLoc` | `:1106-1109` |
| `logLoc` | `≤ 0` (auch `-1`) | unverändert | dito |
| `physLoc` | `> 0` | `setphys` | `:1110-1112` |
| `physLoc` | `≤ 0` | unverändert | dito |
| `rez` | **`-1`** | **sofort `0`**, keine Auflösungsänderung | `:1115-1117` |
| `rez` | `< MIN_REZ (0)` oder `> MAX_REZ (7)` | **`-1`** = Fehler | `:1120-1122` |
| `rez` | 0…7 | Auflösung setzen, Line-A + VT-52 neu | `:1148-1163` |
| `videlmode` | `-1` bei `rez == FALCON_REZ` (3) | Modus **nicht** ändern | `:1131-1143` |
| `videlmode` | ≠ `-1` bei `rez == 3` | `vfixmode` + ggf. `Srealloc` | dito |
| Sonderfall | `logLoc == 0 && physLoc == 0 && rez == 3` | Bildschirmspeicher wird **neu belegt** | `:1134-1141` |

Die Auflösungswerte (`include/biosdefs.h:124-133`): `ST_LOW` 0, `ST_MEDIUM` 1,
`ST_HIGH` 2, `FALCON_REZ` 3, `TT_MEDIUM` 4, **5 nicht vergeben**, `TT_HIGH` 6,
`TT_LOW` 7.

→ 8 `rez`-Werte + 2 Fehlerklassen + 2 Zeigerklassen × 2 + 2 `videlmode`-Klassen
= **16 Werte**.

### 2.2 `Cursconf` (0x15) — `function`

`cursconf`, `bios/vt52.c:893-915`. Ein `switch` **ohne `default`**; ein
unbekannter Wert ist ein No-op mit Rückgabe 0.

| Wert | Wirkung | Beleg |
|---|---|---|
| 0 | Cursor unsichtbar | `:896-898` |
| 1 | Cursor sichtbar | `:899-901` |
| 2 | Blinken **aus** (`v_stat_0 &= ~M_CFLASH`) | `:902-904` |
| 3 | Blinken **ein** | `:905-907` |
| 4 | Blinkdauer setzen = `LOBYTE(operand)` | `:908-910` |
| 5 | Blinkdauer **zurückgeben** (`v_period`) | `:911-912` |
| sonst | No-op, Rückgabe 0 | `:913-914` |

→ 6 Werte + 1 Rückfallklasse.

### 2.3 `Setcolor` (0x07) — `color`

`setcolor`, `bios/screen.c:1192-1202` → `atari_setcolor`. Die
Abfragekonvention: `color == -1` liest nur; jeder andere Wert schreibt und gibt
den **alten** zurück. `colorNum` ist ein Index in die ST-kompatiblen
Palettenregister.

→ 2 Klassen (`-1` / schreiben).

### 2.4 `Keytbl` (0x10) — die drei `-1`-Zeiger

`keytbl`, `bios/ikbd.c:122-134`: jeder der drei Zeiger wird **einzeln** gegen
`(UBYTE*)-1` geprüft (`:124,127,130`); nur ein Zeiger ≠ `-1` überschreibt.
Rückgabe ist **immer** `&current_keytbl`.

→ 2 Klassen × 3 Zeiger = **6 Werte**, also 8 Kombinationen.

### 2.5 `Iorec` (0x0E) — `devno`

`iorec`, `bios/xbios.c:336-348`:

| Wert | Gerät |
|---|---|
| 0 | RS-232 (`rs232iorecptr`) |
| 1 | IKBD (`&ikbdiorec`) |
| 2 | MIDI (`&midiiorec`) |
| sonst | **`-1`** (`:345-346`) |

→ 3 Werte + 1 Fehlerklasse. **In NewTOS gar nicht beantwortet** — `Iorec(0)`
liefert dort `0x0E`.

### 2.6 `Bconmap` (0x2C) — `dev`

`bconmap`, `bios/serport.c:1082-...`:

| Wert | Wirkung | Beleg |
|---|---|---|
| **−1** | aktuell zugeordnete Gerätenummer zurückgeben | `:1089-1090` |
| **−2** | Zeiger auf `bconmap_root` zurückgeben | `:1092-1093` |
| `dev − 6 < 0` oder `≥ maptabsize` | **`0`** = ungültig | `:1095-1097` |
| 6 … 6+`maptabsize`−1 | Umschalten, alte Nummer zurückgeben | `:1099-...` |

→ 2 Sonderwerte + n Gerätenummern + 1 Fehlerklasse.
NewTOS liefert **immer `0x2c`** (`system.rs:94-...`), also den
„nicht vorhanden"-Wert — auch für −1 und −2.

### 2.7 `Kbrate` (0x23) / `Setprt` (0x21) / `Blitmode` (0x40) / `Dosound` (0x20)

Alle vier folgen der TOS-Abfragekonvention „negativer Wert = nur lesen":

| Opcode | Abfragewert | Rückgabe |
|---|---|---|
| 0x23 `Kbrate(initial, repeat)` | je `< 0` | gepackter **alter** Wert `(initial<<8)\|repeat` |
| 0x21 `Setprt(config)` | `-1` | **alter** Konfigurationswert |
| 0x40 `Blitmode(mode)` | `-1` | Statuswort |
| 0x20 `Dosound(ptr)` | `-1` | Zeiger auf den laufenden Klangzeiger |

NewTOS: `Kbrate` und `Setprt` echt (`system.rs:206-...`, `:242-...`),
`Blitmode` fest `0x0000` (`:52-55`), `Dosound` fest `0` (`:162-165`).

→ 2 Klassen × 4 Opcodes = **8 Werte**.

### 2.8 `Giaccess` (0x1C) — Register + Schreibbit

`Giaccess(data, register)`: Bit 7 von `register` = schreiben, sonst lesen;
Bits 0–3 wählen eines der 16 PSG-Register. **In NewTOS fest `0`**
(`system.rs:146-148`) — es gibt keinen YM2149.

→ 16 Register × 2 Richtungen = **32 Werte**.

### 2.9 **Nicht erhoben** — die Gerätegruppen

Für die folgenden Gruppen ist die Opcode-Ebene vollständig (§1.1), die
Unterfunktionsebene **nicht untersucht**:

| Gruppe | Opcodes | offene Frage |
|---|---|---|
| TT-Shifter | 0x50–0x57 | `EsetShift`/`EsetBank`/`EsetGray`/`EsetSmear` haben je eine `-1`-Abfrage; Wertebereiche nicht erhoben |
| Videl | 0x58–0x5B, 0x5D–0x5F | `VsetMode` hat eine Bitmaske (Auflösung, Farbtiefe, Interlace …); `VsetSync` eine Modusmaske; nicht erhoben |
| DSP | 0x60–0x7F | 32 Opcodes, jeder mit eigenen Parametern; nicht erhoben |
| DMA-Sound | 0x80–0x8D | `Soundcmd`, `Setsndmode`, `Devconnect`, `Buffoper`, `Sndstatus` haben je einen Moduswert; nicht erhoben |
| Floppy/DMA | 0x08–0x0A, 0x13, 0x29–0x2B | Parameter sind Geometriewerte; `Flopfmt`'s `virgin`/`interlv` sind Modi; nicht erhoben |
| MFP | 0x0D, 0x1A, 0x1B, 0x1F | `Mfpint`/`Jdisint`/`Jenabint` nehmen eine Interruptnummer 0…15; `Xbtimer` eine Timernummer 0…3. **Erhoben ist nur der Bereich, nicht die Behandlung.** |
| `NVMaccess` 0x2E | — | `type` 0 = lesen, 1 = schreiben, 2 = löschen (nicht gegen die Quelle geprüft) |
| `Protobt` 0x12 | — | `serialno`/`disktype`/`execflag` haben je einen `-1`-Sonderwert (nicht gegen die Quelle geprüft; NewTOS implementiert die Funktion, `floppy.rs`) |
| `Rsconf` 0x0F | — | sechs Parameter mit je `-1`-Abfrage (nicht gegen die Quelle geprüft; NewTOS implementiert sie, `system.rs:279-...`) |
| `Initmous` 0x00 | — | `type` 0…4 (nicht gegen die Quelle geprüft) |

**Das ist die ehrliche Grenze dieses Inventars.** Wer die Falltabelle baut,
muss diese Gruppen nacherheben — die Opcode-Liste oben ist dafür vollständig.

### 2.10 Summe der **erhobenen** Unterfunktionswerte

| Träger | Werte | Rückfallklassen |
|---|---|---|
| `Setscreen` `rez` + Zeiger + `videlmode` | 16 | 2 |
| `Cursconf` `function` | 6 | 1 |
| `Giaccess` (16 Register × lesen/schreiben) | 32 | — |
| `Keytbl` (3 Zeiger × `-1`/setzen) | 6 | — |
| `Bconmap` `dev` (−1, −2) | 2 | 1 |
| `Iorec` `devno` | 3 | 1 |
| `Kbrate`/`Setprt`/`Blitmode`/`Dosound` Abfragekonvention | 8 | — |
| `Setcolor` `color` (−1 / schreiben) | 2 | — |
| **Summe (erhoben)** | **75** | **5** |
| **nicht erhoben** | **≥ 60 Opcodes der Gerätegruppen** (§2.9) | — |

**Also: 142 Plätze, davon 103 belegt; für 8 davon sind die Unterfunktionen
erhoben (75 Werte + 5 Rückfallklassen), für die restlichen Gerätegruppen
nicht. Dazu 1 Fehlerklasse der Opcode-Ebene — und die ist im XBIOS
ungewöhnlich: sie gibt die Funktionsnummer zurück, nicht `EINVFN`.**

---

## 3. Was eine vollständige Falltabelle abdecken muss

1. **Alle 103 belegten Nummern** im Gutfall — mit dem Vorbehalt, dass eine
   Funktion, deren Gutfall-Rückgabe zufällig ihrer Nummer gleicht, nicht von
   „nicht implementiert" unterscheidbar ist.
2. **Alle 39 `xbios_unimpl`-Plätze** — sie müssen die Nummer zurückgeben.
3. **Mindestens zwei Nummern > 0x8D** — ebenfalls die Nummer.
4. Die erhobenen Unterfunktionen aus §2.1–2.8.
5. Die nicht erhobenen Gruppen aus §2.9 — **erst nach einer Nacherhebung**.

### Abschnitte für die Messtabelle

**A — frei messbar.** `Physbase` 0x02, `Logbase` 0x03, `Getrez` 0x04,
`Setcolor(n,-1)` 0x07, `Random` 0x11, `Cursconf(5,0)` 0x15, `Gettime` 0x17,
`Giaccess` 0x1C, `Kbdvbase` 0x22, `Kbrate(-1,-1)` 0x23, `Bconmap(-1)` 0x2C,
`Blitmode(-1)` 0x40, `Setprt(-1)` 0x21, `Iorec(0..3)` 0x0E, alle
`xbios_unimpl`-Plätze, alle Nummern > 0x8D.

**B — blockiert.** `Vsync` 0x25 (wartet bis zu einem Rahmen — endlich, aber
zeitabhängig).

**C — verändert den Zustand grundlegend.** `Setscreen` 0x05, `Setpalette` 0x06,
`Setcolor` mit Wert, `Settime` 0x16, `Bioskeys` 0x18, `Keytbl` mit Zeigern,
`Bconmap` mit Gerätenummer, `Supexec` 0x26 (führt fremden Code aus),
`Floprd`/`Flopwr`/`Flopfmt` 0x08–0x0A, `Protobt` 0x12, `NVMaccess` 0x2E,
alle Videl- und TT-Shifter-Opcodes (Auflösungswechsel).

**D — nur per Screendump bzw. auf echter Hardware.** `Cursconf` 0–4,
`Scrdmp` 0x14, alle Sound- und DSP-Opcodes.

---

## 4. Belege

* Vektortabelle: `bios/xbios.c:1285-1515`; `xbios_ent` `:1517`;
  `LAST_ENTRY` `:1273-1283`; `DBG_XBIOS`-Hülle `:1267-1271`
* `xbios_do_unimpl`: `bios/xbios.c:1251-1257`; Aufruf `bios/vectors.S:1366-1370`
* Trap-14-Einsprung: `bios/vectors.S:1275-1277`; gemeinsamer Verteiler
  `:1282-1333`; Bereichsprüfung `:1324-1326`; Rückweg `:1334-1359`
* `Setscreen`: `bios/screen.c:1102-1166`; Auflösungskonstanten
  `include/biosdefs.h:124-133`
* `Cursconf`: `bios/vt52.c:893-915`
* `Setcolor`: `bios/screen.c:1192-1202`
* `Keytbl`: `bios/ikbd.c:122-134`
* `Iorec`: `bios/xbios.c:336-348`
* `Bconmap`: `bios/serport.c:1082-...`; `BCONMAP_START_HANDLE`
  `bios/serport.h:121`
* Profilgatter: `include/config.h` (mit `cc -E -dM -Iinclude -DTARGET_512`
  ausgewertet)
* NewTOS-Verteiler: `tos-api/src/xbios/mod.rs:28-138`
