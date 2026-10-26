# BIOS — vollständiges Funktions- und Unterfunktionsinventar

**Stand:** 2026-09-14 · **Quelle:** `references/emutos` @
`1a7484aba1bfba23e2c0899e243b6d8177fe41df` (mit `git -C references/emutos log -1`
bestätigt) · **Zielprofil:** `make 512`, Falcon030/TOS4 · **NewTOS-Seite:**
HEAD `19871c0`.

Formvorlage: `docs/api/aes/_full-function-inventory.md`.

---

## 0. Methode und Abgrenzung

**Die Liste kommt aus der Vektortabelle, nicht aus Kopfdateien.**
Maßgeblich sind zwei Stellen:

* `bios/bios.c:1252-1265` — `bios_vecs[]`, **12 Einträge**;
  `const UWORD bios_ent = ARRAY_SIZE(bios_vecs);` (`:1267`),
* `bios/vectors.S:1268-1359` — der Trap-13-Einsprung `_biostrap` (`:1268-1271`)
  und der gemeinsame Verteiler `biosxbios` (`:1282-1333`).

`bios_vecs` hat **keine** Lücken: alle 12 Nummern 0x00…0x0B sind belegt, keine
davon steht unter einem `#if`. Der Schalter `DBGBIOS` (`bios/bios.c:1246-1250`)
wählt nur zwischen der Protokoll-Hülle `bios_N` und der nackten Funktion — er
ändert nichts an der Belegung.

### Der Nicht-Weg des Verteilers — **kein `EINVFN`**

```
        move.w  (sp)+,d0        // Funktionsnummer vom Stapel
        cmp.w   d1,d0           // Higher than highest number?
        jhs     bx_ret_exc      // → direkt zum RTE
```

`bios/vectors.S:1324-1326`. `d0` enthält zu diesem Zeitpunkt noch die
**Funktionsnummer**; `bx_ret_exc` (`:1334`) stellt nur die Register wieder her
und macht `rte`. **Eine Nummer ≥ 12 gibt also die Nummer selbst zurück, nicht
einen Fehlercode.** Das ist ein eigener, gut messbarer Fall
(`Bios(12) == 12`, `Bios(99) == 99`).

NewTOS bildet das nach: `other => other as i64`
(`tos-api/src/bios/mod.rs:92`).

### Profilgatter — nachgeprüft für `make 512`

Keine BIOS-Funktion ist gatterbehaftet. Relevant sind nur die Gatter *innerhalb*
der Zeichengeräte-Funktionen:

| Schalter | Wert | BIOS-Folge |
|---|---|---|
| `CONF_WITH_SCC` | **1** | `BCONMAP_AVAILABLE = (CONF_WITH_SCC \|\| CONF_WITH_TT_MFP)` (`bios/serport.h:18`) ist **wahr** → die Handle-Umleitung ab `BCONMAP_START_HANDLE = 6` (`serport.h:121`) ist übersetzt |
| `CONF_WITH_MFP` | 1 | zusätzliche serielle Geräte in der `bconmap`-Tabelle |
| `CONF_SERIAL_CONSOLE` | **0** | `bconout2` bleibt der Bildschirm |

---

## 1. Opcode-Ebene: 12 Funktionen, keine Lücken

NewTOS-Verteiler: `tos-api/src/bios/mod.rs::dispatch` (`:72-93`, 12 Arme +
Auffangarm).

| Nr | Name | EmuTOS Tabelle | EmuTOS Rumpf | N | NewTOS |
|---|---|---|---|---|---|
| 0x00 | `Getmpb` | `bios/bios.c:1253` | `bios/biosmem.c` (`getmpb`) | ja | `bios/mod.rs:84` → `getmpb.rs` |
| 0x01 | `Bconstat` | `:1254` | `bios/bios.c:914-934` | ja | `:85` → `chardev.rs:293-295` |
| 0x02 | `Bconin` | `:1255` | `bios/bios.c:958-978` | ja | `:86` |
| 0x03 | `Bconout` | `:1256` | `bios/bios.c:991-1011` | ja | `:87` |
| 0x04 | `Rwabs` | `:1257` | `bios/bios.c:1045-1048` (`lrwabs`) | ja | `:89` → `tos-api/src/bios/mod.rs:148-228` |
| 0x05 | `Setexc` | `:1258` | `bios/bios.c:1070-1080` | ja (Schattenkopie) | `:90` → `vectors.rs` |
| 0x06 | `Tickcal` | `:1259` | `bios/bios.c:1096-1099` — `return 20L;` | ja (derselbe feste Wert) | `:91`, `tos-api/src/bios/mod.rs:76` |
| 0x07 | `Getbpb` | `:1260` | `bios/bios.c:1120-1123` | ja | `:92` → `bpb.rs` |
| 0x08 | `Bcostat` | `:1261` | `bios/bios.c:1143-1163` | ja | `:88` |
| 0x09 | `Mediach` | `:1262` | `bios/bios.c:1184-1187` | HW-Adapter (SD nicht wechselbar → 0) | `:93`, `tos-api/src/bios/mod.rs:237-243` |
| 0x0A | `Drvmap` | `:1263` | `bios/bios.c:1206-1209` | ja | `:94`, `tos-api/src/bios/mod.rs:247-249` |
| 0x0B | `Kbshift` | `:1264` | `bios/ikbd.c:145-153` | ja | `:95`, `tos-api/src/bios/mod.rs:116-127` |

### 1.1 Summenzeile

| | Anzahl |
|---|---|
| EmuTOS `bios_vecs[]`-Einträge (`bios_ent`) | **12** |
| davon belegt (keine `unimpl`-Plätze) | **12** |
| davon in NewTOS verdrahtet | **12** |
| davon fester Wert, der EmuTOS aber entspricht | **1** (`Tickcal` — EmuTOS liefert ebenfalls hart `20L`) |
| davon HW-Adapter | **1** (`Mediach`) |
| unter einem bei uns abgeschalteten Schalter | **0** |
| in NewTOS **nicht** verdrahtet | **0** |
| NewTOS-Erweiterungen über EmuTOS hinaus | **0** |

**Gegengezählt:** `tos-api/src/bios/mod.rs:74-85` hat 12 Arme, `bios_vecs[]`
12 Einträge. **Auf Opcode-Ebene ist NewTOS vollständig.**

---

## 2. Unterfunktionen

### 2.1 Die Gerätehandles von `Bconstat` (1), `Bconin` (2), `Bconout` (3), `Bcostat` (8)

Das ist die einzige große Unterfunktionsebene des BIOS. Die **Behandlungsliste**
sind die vier Vektortabellen `bcon*_vec[8]` (`NUM_CHAR_VECS = 8`,
`bios/chardev.c:28`), gefüllt aus `bconstat_init` (`:30-32`), `bconin_init`
(`:34-36`), `bcostat_init` (`:38-41`) und `bconout_init` (`:43-45`).

| Handle | Gerät | `Bconstat` | `Bconin` | `Bconout` | `Bcostat` |
|---|---|---|---|---|---|
| 0 | PRT (Centronics) | `char_dummy` → **0** | `bconin0` | `bconout0` | `bcostat0` |
| 1 | AUX (RS-232) | `bconstat1` | `bconin1` | `bconout1` | `bcostat1` |
| 2 | CON (Bildschirm / IKBD) | `bconstat2` | `bconin2` | `bconout2` | `bcostat2` |
| 3 | MIDI | `bconstat3` | `bconin3` | `bconout3` | **`bcostat4`** |
| 4 | IKBD (Tastatur-Sendekanal) | `char_dummy` → **0** | `char_dummy` → **0** | `bconout4` | **`bcostat3`** |
| 5 | roh-CON (`ascii_out`) | `char_dummy` → **0** | `char_dummy` → **0** | `bconout5` | `char_dummy` → **0** |
| 6 | — | `char_dummy` | `char_dummy` | `charout_dummy` | `char_dummy` |
| 7 | — | `char_dummy` | `char_dummy` | `charout_dummy` | `char_dummy` |

> **Die Vertauschung bei Handle 3/4 steht so im Quelltext**, mit Kommentar:
> „note that IKBD and MIDI bcostat() are swapped!" (`bios/chardev.c:40`).
> Ein Prüfprogramm, das `Bcostat(3)` und `Bcostat(4)` misst, misst also **nicht**
> MIDI und IKBD in dieser Reihenfolge.

Bereichs- und Vorprüfungen (alle vier Funktionen gleich, `bios/bios.c:914-1011`,
`:1143-1163`):

| Klasse | Wirkung | Beleg |
|---|---|---|
| `boot_status & CHARDEV_AVAILABLE` nicht gesetzt | **0**, noch vor jeder Handle-Prüfung | `bios/bios.c:920-921` |
| `handle - BCONMAP_START_HANDLE ≥ maptabsize` | **0** | `bios/bios.c:925-926` |
| `handle ≥ BCONMAP_START_HANDLE` (= 6) | **Umleitung in die `bconmap`-Tabelle**, nicht in `bcon*_vec` | `bios/bios.c:924,927-928` |
| `0 ≤ handle ≤ 7` | `bcon*_vec[handle]` | `bios/bios.c:931-932` |
| `handle < 0` | **0** | `bios/bios.c:933` |

**Wichtig für die Falltabelle:** die Handles 6 und 7 sind in `bcon*_vec`
zwar belegt, werden aber wegen `BCONMAP_AVAILABLE` **nie** darüber erreicht —
der `bconmap`-Zweig greift vorher. Das ist ein Messfall, kein toter Code.

NewTOS-Seite (`tos-api/src/bios/chardev.rs:31-42`): **Eingabe nur auf Handle 2**,
**Ausgabe auf 2 und 5**, alles andere 0. Kein `bconmap`-Zweig.

→ 8 Handles × 4 Funktionen = **32 Werte** + 3 Vorprüfklassen
(`CHARDEV_AVAILABLE`, `bconmap`-Bereich, negatives Handle).

### 2.2 `Bconout` (3) — das Zeichen

`bconout(handle, what)`: `what` ist ein Datenwert, **außer** auf Handle 2, wo
er durch die VT-52-Emulation läuft (`bconout2` → `ascii_out`/`vt52`). Die
VT-52-Sequenzen sind eine eigene Unterfunktionsebene, die aber nicht zum
BIOS-Verteiler gehört — sie ist in `docs/api/bios/03-bconout.md` zu führen.

### 2.3 `Rwabs` (4) — `r_w`-Bits

Der BIOS-Rumpf reicht nur durch (`lrwabs` → `hdv_rw`); die Bits werden im
Blockgerät ausgewertet (`bios/blkdev.c`). Die Behandlungsliste
(`bios/disk.h:52-61`):

| Bit | Name | Wirkung |
|---|---|---|
| 0x01 | `RW_WRITE` | schreiben statt lesen |
| 0x02 | `RW_NOMEDIACH` | Medienwechselprüfung überspringen |
| 0x04 | `RW_NORETRIES` | keine Wiederholungen |
| 0x08 | `RW_NOTRANSLATE` | **physische** Sektornummer, keine Partitionsumrechnung (`blkdev.c:438`) |

Dazu die Sonderklasse `first == -1` → `lfirst` (der 32-Bit-Sektor) wird
benutzt.

NewTOS wertet **0x01 und 0x08** aus (`tos-api/src/bios/mod.rs:46-50`); 0x02 und
0x04 sind im Kopfkommentar als bewusst unbehandelt benannt (kein wechselbares
Medium, keine Wiederholungen).

→ 4 Bits + 1 Sektorklasse = **5 Werte**.

### 2.4 `Setexc` (5) — Vektornummer

`setexc(num, vector)`, `bios/bios.c:1070-1080`:

| Klasse | Wirkung |
|---|---|
| `vector == -1` | **nur abfragen**, Vektor bleibt (`:1070-1072`) |
| `vector ≠ -1` | schreiben, **alten** Wert zurückgeben |

`num` ist auf m68k die Adresse `4 * num` — **ohne jede Bereichsprüfung**.
EmuTOS schreibt also für jedes `num` an `4*num`. Die für ein Prüfprogramm
interessanten Nummern sind die TOS-Haken:

| `num` | m68k-Adresse | Name |
|---|---|---|
| 0x100 | `$0400` | `etv_timer` |
| 0x101 | `$0404` | `etv_critic` |
| 0x102 | `$0408` | `etv_term` |
| 0x103…0x11F | `$040C`…`$047C` | GEM-Haken |
| 0x00…0xFF | `$0000`…`$03FC` | die 256 m68k-CPU-Vektoren |

NewTOS führt eine Schattenkopie von `0x120` Einträgen
(`tos-api/src/bios/vectors.rs:23`) und setzt `etv_timer`/`etv_term`/
`etv_critic` auf nicht-null Vorgabewerte (`:41-56`), damit das Abfragemuster
`old = Setexc(num,-1)` nie 0 liefert. **Die Schattenkopie wird nicht
ausgeführt** — NewTOS ruft die `etv_*`-Haken namentlich auf.

→ 2 Klassen (`-1` / schreiben) × 0x120 Vektornummern.

### 2.5 `Kbshift` (0x0B) — `flag`

| Klasse | Wirkung | Beleg |
|---|---|---|
| `flag < 0` | nur abfragen | `bios/ikbd.c:145-153` |
| `flag ≥ 0` | `flag` in die **`UBYTE`**-Variable `shifty` speichern (Abschneiden auf ein Byte, `bios/lowstram.c:53`), **alten** Wert zurückgeben | `ikbd.c:149-152` |

Die Bits (`include/biosdefs.h:90-100`): `MODE_RSHIFT` 0x01,
`MODE_LSHIFT` 0x02, `MODE_CTRL` 0x04, `MODE_ALT` 0x08, `MODE_CAPS` 0x10,
`MODE_HOME` 0x20, `MODE_INSERT` 0x40.

→ 2 Klassen + 7 Bits = **9 Werte**.

### 2.6 `Getbpb` (7) / `Mediach` (9) / `Drvmap` (0xA) — Laufwerksnummern

`Getbpb(drv)` und `Mediach(drv)` nehmen eine Laufwerksnummer 0…31 (A:…). Es ist
ein Datenwert, kein Modus — aber die **Klassen** gehören in eine Falltabelle:
vorhandenes Laufwerk · nicht vorhandenes Laufwerk · Nummer außerhalb 0…31 ·
negative Nummer. `Drvmap` hat keine Argumente.

`Mediach`-Rückgaben (`bios/bios.c:1175-1181`): 0 = sicher nicht gewechselt,
1 = vielleicht, 2 = sicher gewechselt. NewTOS liefert **immer 0** für ein
gültiges Laufwerk und `EUNDEV` sonst (`tos-api/src/bios/mod.rs:237-243`) — die Karte ist nicht
wechselbar.

### 2.7 Funktionen **ohne** Unterfunktion

`Getmpb` 0x00 · `Tickcal` 0x06 · `Drvmap` 0x0A.

### 2.8 Summe der Unterfunktionswerte

| Träger | Werte | Rückfallklassen |
|---|---|---|
| Gerätehandles 0…7 × {`Bconstat`, `Bconin`, `Bconout`, `Bcostat`} | 32 | 3 |
| `Rwabs` `r_w`-Bits + `first == -1` | 5 | — |
| `Setexc` (abfragen / schreiben) | 2 | — |
| `Kbshift` (abfragen / schreiben + 7 Bits) | 9 | — |
| `Getbpb`/`Mediach` Laufwerksklassen (4 je Funktion) | 8 | — |
| **Summe** | **56** | **3** |

**Also: 12 Funktionen, davon 9 mit Unterfunktionen und 3 ohne, zusammen
56 diskrete Unterfunktionswerte + 3 Rückfallklassen + 1 Fehlerklasse der
Opcode-Ebene (Nummer ≥ 12 → *gibt die Nummer zurück*).**

---

## 3. Was eine vollständige Falltabelle abdecken muss

1. **Alle 12 Nummern** im Gutfall.
2. **Mindestens zwei Nummern ≥ 12** — sie müssen die Nummer selbst liefern,
   **nicht** `EINVFN`. Das ist der am leichtesten falsch implementierte Fall im
   ganzen BIOS.
3. **Alle 8 Gerätehandles** für jede der vier Zeichenfunktionen — einschließlich
   der vertauschten Paarung bei `Bcostat(3)`/`Bcostat(4)`.
4. **Ein negatives Handle** je Zeichenfunktion (→ 0).
5. `Setexc` mit `-1` (abfragen) und mit einem Wert (schreiben + alten Wert
   zurückbekommen), für `etv_timer`/`etv_critic`/`etv_term`.
6. `Kbshift(-1)` und `Kbshift(n)` mit Rückgabe des alten Werts.
7. `Getbpb`/`Mediach` für ein vorhandenes, ein nicht vorhandenes und ein
   unmögliches Laufwerk.

### Abschnitte für die Messtabelle

**A — frei messbar.** `Getmpb` 0x00, `Bconstat` 0x01, `Bcostat` 0x08,
`Tickcal` 0x06, `Getbpb` 0x07, `Mediach` 0x09, `Drvmap` 0x0A,
`Kbshift` 0x0B, alle Nummern ≥ 12.

**B — blockiert.** `Bconin` 0x02 auf einem Handle mit echter Eingabe (CON).
Auf den `char_dummy`-Handles kehrt es sofort mit 0 zurück und ist messbar.

**C — verändert den Zustand.** `Rwabs` 0x04 mit `RW_WRITE`, `Setexc` 0x05 mit
einem Wert ≠ −1, `Kbshift` mit `flag ≥ 0`, `Bconout` 0x03 (schreibt auf den
Bildschirm).

---

## 4. Belege

* Vektortabelle: `bios/bios.c:1252-1265`; `bios_ent` `:1267`; `DBGBIOS`-Hülle
  `:1245-1249`
* Trap-13-Einsprung und Verteiler: `bios/vectors.S:1268-1271`, `:1282-1333`;
  Bereichsprüfung `:1324-1326`; Rückweg `:1334-1359`
* Zeichengeräte-Vektoren: `bios/chardev.c:28-45`, `char_dummy` `:50-53`,
  `charout_dummy` `:55-58`, Erstbelegung `:63-82`
* `BCONMAP_AVAILABLE` / `BCONMAP_START_HANDLE`: `bios/serport.h:18,121`
* `Rwabs`-Bits: `bios/disk.h:52-61`
* Profilgatter: `include/config.h` (mit `cc -E -dM -Iinclude -DTARGET_512`
  ausgewertet)
* NewTOS-Verteiler: `tos-api/src/bios/mod.rs:52-93`
* NewTOS-Zeichengeräte: `tos-api/src/bios/chardev.rs:19-53`
