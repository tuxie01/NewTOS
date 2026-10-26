# EMMC/SD-Treiber — Quellenabgleich (FS04, Audit 2026-09-25)

> Track-B-Audit nach `docs/SOURCE-ANALYSIS-PROTOCOL.md`. Gegenstand:
> `kernel/src/drivers/bcm2711/emmc.rs` und der Pfad darueber
> (`kernel/src/host_env.rs`, `tos-api/src/bdos/fs/cache.rs`,
> `tos-api/src/bios/mod.rs`). Grundlage: BCM-Datenblatt, SD Physical Layer
> Simplified Spec, SD Host Controller Simplified Spec, und die Pi-Messung
> `measurements/measurement-2026-09-25-fs04/raw/pi04-FS04.TXT` (14 Bloecke).
>
> **Die Frage des Tickets — „macht die Karte uns warten, oder warten wir
> falsch?" — ist mit *beides* beantwortet, und zwar quantitativ.** Die Karte
> braucht gemessen bis **170 ms** fuer *einen* kalten Sektor (erlaubt sind
> 250 ms); unser Wartebudget betraegt gemessen **104–200 ms** und ist kein
> Zeitmass, sondern ein Schleifenzaehler. In Block 2 war das Budget kuerzer
> als die Kartenlatenz, **12 von 23 Sektorschreibungen liefen leer aus**, und
> der Blockcache hat sie stillschweigend wiederholt.

---

## 0. Quellenlage — eine Luecke gleich vorweg

| # | Quelle | Stand | Verwendung |
|---|--------|-------|------------|
| 1 | **BCM2711 ARM Peripherals** (`references/datasheets/RP-008248-DS-1-bcm2711-peripherals.pdf`, Release 4, 2022-01-18) | vorhanden | **enthaelt kein EMMC-Kapitel.** Einziger Treffer im ganzen Dokument: IRQ 62 „EMMC & EMMC2" (Tabelle 102, Kap. 6). Fuer Register und Bits ist es **keine** Quelle. |
| 2 | **BCM2835 ARM Peripherals**, Kap. 5 „External Mass Media Controller", S. 65–81 | nachgeladen | Registerlage + Broadcom-eigene Warnungen. Gilt fuer den Arasan-EMMC; EMMC2 ist ein anderer Block, siehe unten. |
| 3 | **SD Physical Layer Simplified Specification Ver. 6.00** (Titelblatt: 2018-08-29) | seit REF02 `references/sd-spec/archive/`; **Referenzausgabe ist jetzt 9.10**, siehe Nachtrag | Kommandofolge, Takt- und **Timeout-Grenzen**. |
| 4 | **SD Host Controller Simplified Specification Ver. 3.00** (2011-02-25) | seit REF02 `references/sd-spec/archive/`, **Referenzausgabe** | Registersemantik (EMMC2 = Standard-SDHCI), Kommando- und **Fehlerbehandlungsablauf**. |

**Befund S1 (Quellenlage, an den Researcher):** Quellen 2–4 liegen **nicht**
in `references/`. `references/README.md` fuehrt „BCM2711 ARM Peripherals" als
TODO und nennt die SD-Spezifikationen gar nicht in der Checkliste, obwohl
`docs/hardware/emmc-sd.md` §Sources sie als Quelle 3 und 4 auffuehrt. Der
Treiber ist also gegen Quellen geschrieben worden, die im Projekt nicht
abgelegt sind — und die Zeitgrenzen aus Quelle 3, an denen dieses Ticket
haengt, stehen in **keinem** unserer Dokumente. Das ist die Wurzel von
Befund 1. Beide Simplified Specs sind frei beziehbar und gehoeren nach
`references/sd-spec/`.

> **Nachtrag REF02, 2026-09-28 — Befund S1 ist erledigt.** Beide
> Simplified Specs liegen jetzt unter `references/sd-spec/` und sind in
> `references/README.md` nach Ausgabe, Datum und SHA-256 **gepinnt**
> (das Verzeichnis ist gitignored, die Tabelle im committeten README ist
> deshalb der Pin). Die Zeitgrenzen dieses Audits stehen jetzt in
> `docs/hardware/emmc-sd.md` §7, je mit Abschnittsangabe.
>
> **Zur Fassung:** Referenzausgabe ist fuer die Physical Layer Spec
> **Ver. 9.10**, nicht die hier zitierte 6.00 — die tragenden Stellen
> wurden Text gegen Text verglichen und sind gleich: §4.6.2 samt
> §4.6.2.1/§4.6.2.2 unterscheidet sich nur dadurch, dass 9.10 in drei
> Saetzen „SDUC" neben SDXC nennt; §4.10.1 ist wortgleich; Tabelle 4-42
> hat dieselbe Bitbelegung und Typspalte. **Jedes `6.00 §4.6.2.x`-Zitat
> in diesem Dokument ist damit auch als 9.10-Zitat richtig** und bleibt
> unveraendert stehen. Fuer die Host Controller Spec bleibt **Ver. 3.00**
> die Referenz (das ist die Version, die der Controller meldet); die
> Registerabschnitte sind in 4.20 **um eins verschoben**, die
> Umrechnungstabelle steht in `docs/hardware/emmc-sd.md` §8.
>
> **Eine Praezisierung zu §4.6.2.2, die dieses Audit nicht macht:** die
> 500 ms aus Fall a)–d) sind ausdruecklich den **SDXC/SDUC**-Karten
> zugestanden („in the case of SDXC and SDUC card"); fuer SDHC bleibt die
> Obergrenze 250 ms. Am Schluss des Audits aendert das nichts — der
> Application Note verlangt vom Host ohnehin „more than 500 ms"
> unabhaengig von der Kartenklasse.

**Zur Zustaendigkeit von Quelle 2:** BCM2835 Kap. 5 beschreibt den
Arasan-EMMC an `0x7E30_0000`. Der microSD-Schacht der Pi 4 haengt an
**EMMC2** (`0xFE34_0000`), einem anderen Block, den weder BCM2711 noch
BCM2835 dokumentiert. Fuer EMMC2 ist die **SD Host Controller Spec** die
Registerquelle (Standard-SDHCI). Wo unten BCM2835 zitiert wird, ist das
kenntlich; wo die beiden sich widersprechen koennten (CONTROL0-Bits 15:8),
steht es dabei.

---

## 1. Methode — was getrieben wurde

* **Register real gelesen.** Temporaerer Dump in `kernel/src/selftest_emmc.rs`
  (Feature `selftest-emmc`), QEMU `raspi4b`, Release-Build. Ergebnis unten in
  §2.2/§2.4. Die Verdrahtung ist zurueckgebaut (§6).
* **Fehlerpfad getrieben.** Schreiben hinter das Kartenende ueber denselben
  temporaeren Selftest.
* **Teilerarithmetik getrieben.** Host-Replik von `set_clock`
  (`emmc.rs:216-236`) ueber alle plausiblen `CAPABILITIES`-Werte.
* **Die Pi-Messung ausgewertet, nicht nur gelesen.** Alle 14 Bloecke
  maschinell ausgezaehlt; daraus die Kalibrierung „Schleifendurchlauf ↔
  Wandzeit" und die Schliessung der im Ticket offen gelassenen Luecke (§3.2).
* **Nicht getrieben:** die echte Pi (verboten), und damit jede Aussage ueber
  `CAPABILITIES`/`SLOTISR_VER` **auf EMMC2**. Alles, was davon abhaengt, ist
  unten als *vermutet* markiert und in §5 mit einer entscheidenden Messung
  hinterlegt.

---

## 2. Unsere Folge gegen die spezifizierte, Phase fuer Phase

### 2.1 Controller-Reset und Busspannung

| Schritt | Spezifiziert | Unser Code | Urteil |
|---|---|---|---|
| Software-Reset | SDHCI Software Reset All, warten bis 0 | `CONTROL0=0`, `CONTROL1=SRST_HC`, Warteschleife `emmc.rs:199-205` | Form richtig, **Fehlerfall fehlt**: laeuft die Schleife leer, faehrt der Code wortlos weiter (`break` ohne `else`) |
| Busspannung | SD-HC §3.3 Fig. 3-6: erst *SD Bus Voltage Select*, **dann** *SD Bus Power*; vorher *Voltage Support* in CAPABILITIES pruefen | ein Schreibzugriff `wr(CONTROL0, 0x0F00)` (`emmc.rs:208`) | Bitlage korrekt (Bit 8 = SD Bus Power, 11:9 = 111b = 3,3 V, SD-HC Ver. 3.00 **§2.2.11** `Power Control Register` (Offset 029h) — im Audit stand versehentlich §2.2.1, das ist das SDMA-Adressregister; korrigiert REF02); Reihenfolge und Voltage-Support-Pruefung fehlen. **Ohne beobachtbare Wirkung** — im QEMU-Lauf liest CONTROL0 an einem Sockel *ohne* Karte als `0x0000_0e00` zurueck, also Power-Bit geloescht, genau wie die Spec es fordert („If the Host Controller detects the No Card state, this bit shall be cleared") |
| 74 Takte vor CMD0 | SD Phys §6.4.1: „start to supply **at least 74 SD clocks** to the SD card with keeping CMD line to high"; Fig. 6-4: Initialisierungsverzoegerung = max(1 ms, 74 Takte, Ramp-up) | `set_clock` setzt `CLK_EN` (`emmc.rs:234`), danach zwei MMIO-Schreibzugriffe (`:210-211`) und sofort `GO_IDLE` (`:242`) | **Verletzt.** 74 Takte bei 400 kHz = 185 µs; zwischen `CLK_EN` und CMD0 liegen einige hundert Nanosekunden. Siehe Befund 6 |

### 2.2 Takt

`set_clock` (`emmc.rs:216-236`) liest `CAPABILITIES[15:8]` als Basistakt in
MHz und faellt bei 0 auf **fest verdrahtete 100 MHz** zurueck (`:219`).

* SD-HC §3.2.1 (1): *„If Base Clock Frequency For SD Clock is 00 0000b, the
  Host System shall **provide this information to the Host Driver by another
  method**."* — Ein geratener Wert ist nicht „another method".
* `docs/hardware/emmc-sd.md` §2 (Schlusskasten) schreibt genau diese Methode vor:
  Mailbox-Tag `0x00030002`, Clock-ID EMMC. **Die Funktion existiert bereits**
  — `kernel/src/drivers/bcm2711/mailbox.rs:419` `get_clock_rate()` — und wird
  vom Treiber nicht benutzt.

Die Teilerwahl selbst („kleinster Teiler, dessen Ergebnis ≤ Ziel", `:222-224`)
ist **spec-konform** (SD-HC §2.2.14: *„choose the smallest possible divisor
which results in a clock frequency that is less than or equal to the target
frequency"*). Getrieben ueber alle plausiblen Basiswerte:

| CAPS[15:8] | Basistakt | Ziel 400 kHz → SDCLK | Ziel 25 MHz → SDCLK | 8-Bit-Modus (HC < 3.00) |
|---|---|---|---|---|
| 0 (Fallback 100) | 100 MHz | 400 kHz (div 125) | 25 MHz (div 2) | **781 kHz** — fast 2× ueber fOD |
| 52 (QEMU, gemessen) | 52 MHz | 397 kHz (div 63) | **13 MHz** (div 2) | 781 kHz |
| 63 | 63 MHz | 399 kHz | **15,75 MHz** | 492 kHz |
| 200 | 200 MHz | 400 kHz (div 250) | 25 MHz (div 4) | **781 kHz** |

Zwei Dinge daraus:

1. **Im 10-Bit-Teilermodus (SDHCI ≥ 3.00) ist die Arithmetik in jedem Fall
   korrekt.** Im 8-Bit-Modus (SDHCI 1.00/2.00) sind nur Zweierpotenzen
   erlaubt — *„When setting multiple bits, the most significant bit is used as
   the divisor but it should not be set"* (SD-HC §2.2.14) — und dann laeuft die
   Identifikation mit **781 kHz statt 400 kHz**. Der Treiber liest
   `SLOTISR_VER` nur als Anwesenheitsprobe und wirft den Wert weg
   (`emmc.rs:378-381`); die Modusfrage wird nie gestellt. **QEMU meldet
   `SLOTISR_VER=0x2402_0000` → Spec-Version 0x02 = SDHCI 3.00**, dort also
   unkritisch. Fuer EMMC2 auf der Pi ist es ungeprueft.
2. **Der Transfertakt kann deutlich unter 25 MHz landen**, ohne dass das
   irgendwo auffaellt: bei 52 MHz Basistakt sind es 13 MHz — in QEMU
   nachgemessen (`CONTROL1=0x000e0207`, div8=2, divms2=0 → 52/(2·2) MHz).

### 2.3 Kartenidentifikation

| Kommando | SD Phys | Unser Code | Urteil |
|---|---|---|---|
| CMD0 | Reset in Idle | `:242` | ok (bis auf die 74 Takte) |
| CMD8 | Spannungspruefung, Echo `0xAA`; Erfolg ⇒ v2 | `:245`, wertet nur `is_ok()` | **Das Echo wird nicht geprueft.** Die Spec verlangt den Vergleich des zurueckgelieferten Musters; ein Karte, die mit falschem Echo antwortet, gilt uns als v2 |
| ACMD41 | §4.2.3: *„Card initialization shall be completed within 1 second from the first ACMD41. The host repeatedly issues ACMD41 for at least 1 second"*, §4.2.3.1: *„The host shall set ACMD41 timeout **more than 1 second**"* | `for _ in 0..SPIN_LIMIT` ueber **Kommandos**, `:250-256` | 1 000 000 CMD55+ACMD41-Paare à ~10 µs ≈ **10 s** im Fehlerfall. Zu lang, nicht zu kurz — nur Boot-Zeit, keine Datenwirkung |
| CMD2/CMD3/CMD9 | Identifikation, RCA, CSD | `:262-270` | ok; CSD-Auswertung `parse_capacity` deckt v1 und v2 |
| CMD7 (R1b) | SD-HC §3.7.1.2 (4)(5): bei „command with busy" **auf Transfer Complete warten** | `command()` wartet nur auf `CMD_DONE` (`:185`) | **Nicht eingehalten.** Der Busy nach CMD7 wird nie abgewartet |
| CMD16 | nur SDSC | `:274-276` | korrekt uebersprungen bei SDHC |
| Takt auf 25 MHz | §4.3: bis Ende der Identifikation fOD, danach fPP | `set_clock(25_000_000)` `:279` | siehe naechster Absatz |
| **ACMD6** (Busbreite) | §4.3.1: *„The default bus width after power up or GO_IDLE (CMD0) is 1 bit bus width"*; 4 Bit nur ueber ACMD6 | **wird nie gesendet** | 1-Bit-Betrieb. `CONTROL0.HCTL_DWIDTH` bleibt 0 — in QEMU nachgemessen (`CONTROL0=0x0000_0f00`) |
| **CMD6** (High Speed) | Default Speed ≤ 25 MHz; High Speed 50 MHz nur nach SWITCH_FUNC | **wird nie gesendet**, `CONTROL0.HCTL_HS_EN` bleibt 0 | Default Speed |
| ACMD51 (SCR) | Voraussetzung, um 4-Bit-Faehigkeit zu erfahren | fehlt | Folge des fehlenden ACMD6 |

**Taktwechsel im Busy (SD-HC §3.2.2):** *„The Host Driver shall **not** stop
the SD Clock when a SD transaction is occurring on the SD Bus — namely, when
either Command Inhibit (DAT) or Command Inhibit (CMD) in the Present State
register is set to 1."* Unser `set_clock(25_000_000)` (`:279`) folgt
unmittelbar auf CMD7 (R1b, also mit Busy) und schreibt `CONTROL1` **ohne**
`CLK_EN` (`:227`) — das ist ein Taktstopp. Ob die Karte zu dem Zeitpunkt noch
busy ist, wird nie geprueft. Siehe Befund 7.

### 2.4 Einzelblocklesen und -schreiben

Reihenfolge nach SD-HC §3.7.1.1 / §3.7.2 (Fig. 3-13, Non-DMA):

| Schritt | Spezifiziert | `read_one` / `write_one` | Urteil |
|---|---|---|---|
| (1) | Command Inhibit (CMD) = 0 abwarten | `wait_clear(STATUS, SR_CMD_INHIBIT)` in `command()` `:179` | vorhanden |
| (4) | bei Busy-Kommandos Command Inhibit (DAT) = 0 abwarten | `wait_clear(STATUS, SR_DAT_INHIBIT)` `:289` / `:315` | vorhanden — **das ist `wait_dat`** |
| (5)(6) | ARG1, dann CMDTM | `:183-184` | ok |
| (7)/§3.7.1.2 (1)(2) | Command Complete abwarten und loeschen | `wait_interrupt(INT_CMD_DONE)` `:185` | ok |
| §3.7.1.2 (3)(7) | Response lesen und **auf Fehler pruefen** | `read_one` liest RESP0 und wirft ihn weg (`let _ = resp0;` `:299-300`); `write_one` wirft ihn direkt weg (`:324`) | **Fehlt vollstaendig.** Siehe Befund 3 |
| Puffer | Buffer Read/Write Ready abwarten, dann 128 Worte | `:301-305` / `:325-328` | ok; 512 ≤ 1 KiB FIFO, ein Ready pro Block genuegt |
| Abschluss | Transfer Complete abwarten | `wait_interrupt(INT_DATA_DONE)` `:306` / `:329` | **das ist der teure Punkt**, siehe §3 |

**Was Transfer Complete beim Schreiben bedeutet** — SD-HC §2.2.17, Bit 01:
*„(2) In the case of a Write Transaction: This bit is set at the falling edge
of the DAT Line Active Status … when the last data is written to the SD card
as specified by data length **and the busy signal released**."* Und Present
State Bit 02 (DAT Line Active), Schreibfall: *„This bit shall be cleared …
When the SD card releases **write busy** of the last data block."* Bit 01
(Command Inhibit (DAT)) ist gesetzt, solange DAT Line Active gesetzt ist.

Daraus folgt der Kern der ganzen Sache: **`dev.w.data_done` und
`dev.w.wait_dat` messen dieselbe physikalische Groesse — die
Programmierzeit der Karte.** Einmal wird sie beim eigenen Schreiben
abgewartet (Transfer Complete), einmal beim *naechsten* Kommando nachgeholt
(Command Inhibit (DAT)). Welcher der beiden Zaehler sie einsammelt, haengt
nur daran, ob wir beim eigenen Schreiben durchgehalten haben.

### 2.5 Fehlerbehandlung

SD-HC §3.10.1 (Error Interrupt Recovery) verlangt: Fehlerstatus lesen, bei
CMD-Line-Fehler `Software Reset For CMD Line`, bei DAT-Line-Fehler
`Software Reset For DAT Line`, Status sichern und loeschen, **Abort-Kommando
absetzen** (§3.8: CMD12 + beide Software-Resets), Command Inhibit pruefen,
und bei anhaltender Blockade „Non-recoverable" melden.

Unser `wait_interrupt` (`emmc.rs:161-175`):

```rust
if i & INT_ERROR_MASK != 0 {
    wr(INTERRUPT, i);          // clear (W1C)
    return Err(BlockError::Io);
}
```

und im Zeitablauf:

```rust
Err(BlockError::Timeout)       // nach SPIN_LIMIT, ohne irgendetwas zu tun
```

**Kein `SRST_CMD`, kein `SRST_DATA`, kein CMD12, kein CMD13, keine
Eskalation.** Bei `Timeout` wird nicht einmal das `INTERRUPT`-Register
angefasst; der Controller bleibt mitten im Transfer, die DAT-Leitung bleibt
ihm gehoerend. Siehe Befund 2.

Zusaetzlich eine Prioritaetsverletzung: SD-HC §2.2.17 stellt fest
*„Transfer Complete has higher priority than Data Timeout Error. **If both
bits are set to 1, execution of a command can be considered to be
completed.**"* Unser `wait_interrupt` prueft die Fehlermaske **vor** dem
Flag (`:164` vor `:168`) und meldet in genau diesem Fall `Err(Io)`, obwohl
der Transfer fertig ist. Siehe Befund 8.

---

## 3. Die fuenf Fragen

### 3.1 Frage 1 — worauf wartet jede Warteschleife, und ist es das richtige Bit?

Fuenf Warteschleifen pro Sektoroperation, alle mit demselben Budget
`SPIN_LIMIT = 1_000_000` (`emmc.rs:91`):

| Sonden-Zaehler | Code | Register/Bit | Spec-Sicht |
|---|---|---|---|
| `dev.*.wait_dat` | `wait_clear(STATUS, SR_DAT_INHIBIT)` `:289`/`:315` | STATUS Bit 1 = Command Inhibit (DAT) | **richtiges Bit** fuer „darf ich ein DAT-Kommando absetzen" (SD-HC §3.7.1.1 (4)). Inhaltlich: die Programmierzeit der **vorigen** Schreibung |
| `dev.*.cmd17/24` | `command()` `:178-187` | STATUS Bit 0 + INTERRUPT Bit 0 | richtig |
| `dev.r.read_rdy` | `wait_interrupt(INT_READ_RDY)` `:301` | INTERRUPT Bit 5 = Buffer Read Ready | richtig |
| `dev.w.write_rdy` | `wait_interrupt(INT_WRITE_RDY)` `:325` | INTERRUPT Bit 4 = Buffer Write Ready | richtig |
| `dev.*.data_done` | `wait_interrupt(INT_DATA_DONE)` `:306`/`:329` | INTERRUPT Bit 1 = Transfer Complete | richtig; beim Schreiben = **Ende des Karten-Busy** |

**Die Bits stimmen.** Eine Einschraenkung, und sie ist von Broadcom selbst:
BCM2835 Kap. 5, STATUS-Register, Synopsis (S. 72): *„This register contains
information intended for debugging. … it changes only after some latency and
it is easy sample the values too early. **Therefore it is not recommended to
use this register for polling.** Instead use the INTERRUPT register which
implements a handshake mechanism which makes it impossible to miss a change
when polling."* Unsere zwei `wait_clear`-Schleifen tun genau das
(`STATUS`-Polling). Fuer „warten bis frei" ist die Fehlerrichtung
„zu frueh weiterlaufen".

**Nicht die Bits sind falsch, sondern das Budget.** `SPIN_LIMIT` ist ein
Zaehler, kein Zeitmass. Aus der Pi-Messung laesst es sich exakt kalibrieren —
`emmc.spin max/call` ist die Durchlaufzahl der laengsten Schleife, und der
groesste Einzelwert der Zeitzaehler gehoert zu derselben Schleife:

| Block | `emmc.spin max/call` | zugehoeriger Zeitmaximalwert | ns je Durchlauf |
|---|---|---|---|
| 3 | 11 443 | `w.data_done max` 2 288 µs | 199,9 |
| 4 | 850 010 | `w.data_done max` 170 002 µs | **200,0** |
| 5 | 10 851 | 2 170 µs | 200,0 |
| 6 | 3 678 | `r.read_rdy max` 736 µs | 200,1 |
| 8–14 | 10 621 … 147 927 | 2 124 … 29 586 µs | 200,0 (je) |
| **1** | 6 952 | `r.read_rdy max` 737 µs | **106,0** |

Dreizehn Bloecke liefern **200,0 ns**, der allererste Block **106 ns** — ein
Faktor 1,9, mutmasslich der CPU-Takt der Pi 4 (600 MHz ↔ 1,5 GHz).

> **`SPIN_LIMIT = 1 000 000` ist also ein Timeout von 104–200 ms, und es
> haengt am CPU-Takt: je schneller die CPU, desto kuerzer der Timeout.**

Dem gegenueber die Spezifikation, SD Phys §4.6.2.2:

* *„In case of High Capacity SD Memory Card, maximum length of busy is defined
  as **250 ms** for all write operation."*
* *„a) The last busy in any write operation up to **500 ms** including single
  and multiple block write."*
* Application Note: *„The host should use a **fixed timeout** for write
  operations … It is **strongly recommended for hosts to implement more than
  500 ms timeout value** even if the card indicates the 250 ms maximum busy
  length."*

und §4.6.2.1 fuer Lesen: *„The host should use **100 ms timeout (minimum)**
for single and multiple read operations"*.

**Unser Schreib-Timeout liegt unter der von der Spec erlaubten
Kartenlatenz, und unser Lese-Timeout liegt genau auf der Untergrenze.**
Gemessen hat die Karte in Block 4 fuer **einen** Sektor 170 002 µs gebraucht
— legal, und nur deshalb nicht abgebrochen, weil dort 200 ns/Durchlauf
galten. In Block 2 galten ~104 ns, das Budget war ~104 ms, und dieselbe
Kartenlatenz passte nicht mehr hinein.

### 3.2 Frage 2 — was passiert, wenn `SPIN_LIMIT` leer ausläuft?

**Spur, Glied fuer Glied:**

1. `wait_interrupt` → `Err(BlockError::Timeout)` (`emmc.rs:174`), **ohne
   Reset, ohne Abort, ohne `INTERRUPT` zu loeschen**.
2. `write_one` gibt den Fehler mit `?` weiter (`:329`), `write_blocks` bricht
   die Schleife ab (`:353-355`).
3. `kernel/src/host_env.rs:401-404`: `… .write_blocks(lba, buf).is_ok()` —
   aus dem `BlockError` wird ein **bool**; welcher Fehler es war, ist ab hier
   weg.
4. `tos-api/src/bdos/fs/cache.rs`:
   * in `flush()` (`:128`): `Some(b) if b.write_blocks(…) => s.dirty = false`
     — schlaegt es fehl, bleibt `dirty = true`, und **der naechste `flush()`
     schreibt denselben Sektor erneut**. Das ist die stille Wiederholung.
   * in `evict()` (`:67-69`): `b.write_blocks(slots[v].lba, &slots[v].data);`
     — **Rueckgabewert verworfen**, und `slots[v].dirty = false` wird
     **bedingungslos** gesetzt. Ein hier fehlgeschlagener Rueckschreibvorgang
     ist **endgueltig verloren**.
   * `cache::write()` gibt **immer** `true` zurueck (`:118`).
5. Alle **neun** Aufrufstellen von `cache::flush()` verwerfen den
   Rueckgabewert (cite stale 2026-10-26: tos-api/src/bdos/fs/file.rs:273,446,556,679,782,865,914,
   tos-api/src/bdos/fs/dirops.rs:102,152) — maschinell geprueft.
6. `EWRITF`/`EREADF` kommen in ganz `tos-api/src` **nur** in
   `bios/mod.rs:200,202` vor, also ausschliesslich im `Rwabs`-Pfad. Der
   GEMDOS-Dateipfad hat keinen Weg, einen Geraetefehler zu melden.

> **Antwort: geschluckt — und dazu wiederholt.** `Fwrite` meldet Erfolg,
> auch wenn kein einziger Sektor angekommen ist.

**Und es ist in Block 2 passiert, nachweisbar.** Die Sonde umschliesst
jede Warteschleife einzeln (`measurements/measurement-2026-09-25-fs04/method/fs04-gesture-r2.diff`), `add()` zaehlt
bedingungslos hoch, der Gesamtzaehler `dev.write_one` wird aber erst **nach**
der letzten Warteschleife gesetzt. Auszaehlung aller 14 Bloecke:

| Block | `blk.write_blocks` | `dev.w.pio` n | `dev.w.data_done` n | **Fehlschlaege** | `emmc.spin` calls | 5·(Lesungen+Schreibungen) | `w.wait_dat` gesamt |
|---|---|---|---|---|---|---|---|
| 1 | 0 | 0 | 0 | 0 | 10 | 10 | 0 |
| **2** | **23** | **23** | **11** | **12** | **180** | **180** | **537 387 µs** |
| 3 | 7 | 7 | 7 | 0 | 70 | 70 | 0 |
| 4 | 6 | 6 | 6 | 0 | 60 | 60 | 0 |
| 5 | 6 | 6 | 6 | 0 | 45 | 45 | 0 |
| 6–14 | 0…17 | = | = | **0** | = | = | **0** |

Drei Dinge stehen damit fest:

1. **`emmc.spin calls` ist in *allen* 14 Bloecken exakt `5 × (read_blocks +
   write_blocks)`** — jede Operation hat alle fuenf Warteschleifen
   durchlaufen. Der Abbruch kann also nur in der **letzten** gewesen sein,
   dem `DATA_DONE`-Warten.
2. **`dev.w.pio` n = 23, `dev.w.data_done` n = 11** ⇒ **12 Schreiboperationen
   sind aus dem `DATA_DONE`-Warten mit Fehler zurueckgekommen.** In keinem
   anderen Block gibt es eine solche Differenz.
3. **`dev.w.wait_dat` ist in genau dem einen Block ungleich null, in dem es
   die 12 Abbrueche gab** (537 ms; sonst 0 µs, in den spaeten Bloecken
   hoechstens 1 µs). Das ist der Beleg fuer §2.4: wer das Transfer-Complete
   wegwirft, holt dieselbe Wartezeit beim naechsten Kommando nach.

**Und die im Ticket offen gelassene Luecke schliesst sich.** `emmc.spin
amount` = 19 749 327 Durchlaeufe in Block 2, die *gemessenen* Warteschleifen
summieren 807 404 µs. Mit der Annahme „die 12 Abbrueche waren volle
`SPIN_LIMIT`-Timeouts" bleiben 7 749 327 Durchlaeufe fuer die gemessene Zeit
→ **104,2 ns je Durchlauf**, unabhaengig bestaetigt durch die 106 ns aus dem
unmittelbar davorliegenden Block 1. Daraus:

```
  12 × 1 000 000 × 104,2 ns  = 1 250 ms   (leer ausgelaufene DATA_DONE-Warten)
  gemessene Warteschleifen   =   807 ms
                              ---------
  Summe                       = 2 058 ms
  gemessen: gemdos.dispatch   = 2 080 ms   (Abweichung 1,1 %)
```

Die Gegenhypothese „die 12 waren schnelle `Err(Io)`" scheitert: dann muessten
alle 19,75 Mio. Durchlaeufe auf die 807 ms entfallen, also 40,9 ns je
Durchlauf — im Widerspruch zu den in 13 Bloecken gemessenen 106/200 ns. Auch
11 oder 13 Timeouts passen nicht (die Bilanz weicht dann um 260 ms nach unten
bzw. 290 ms nach oben ab). **12 ist die einzige konsistente Loesung — und
genau die Zahl, die auch der Zaehlerabgleich liefert.**

> **Von den 2,18 s Einfrieren in Block 2 sind ~1,25 s unser eigenes, leer
> auslaufendes Wartebudget und ~0,76 s das Nachholen desselben Karten-Busy
> beim Folgekommando.** Die Karte war dabei wirklich beschaeftigt — aber
> **12 der 23 Sektorprogrammierungen waren Wiederholungen**, die es ohne den
> zu kurzen Timeout nicht gegeben haette. Ohne ihn waeren es ~11 Schreibungen
> statt 23 gewesen.

Warum Block 4 (ebenfalls kalte Partition) *nicht* gescheitert ist, faellt
damit auch heraus: dort galten 200 ns/Durchlauf, Budget ~200 ms, und die
laengste Kartenlatenz war 170 ms — sie passte knapp hinein
(`emmc.spin max/call = 850 010` von 1 000 000). **Der Unterschied zwischen
Block 2 und Block 4 ist nicht die Karte, sondern der CPU-Takt.**

### 3.3 Frage 3 — Initialisierung

Siehe §2.1–2.3. Kurzfassung:

* **ACMD6 wird nie gesendet.** Der Bus laeuft 1-bittig (`CONTROL0.HCTL_DWIDTH`
  = 0, in QEMU nachgemessen). Vierfache Datenphase.
* **Takt:** 400 kHz zur Identifikation (spec-konform, *sofern* SDHCI ≥ 3.00),
  danach „≤ 25 MHz" nach korrekter Regel, aber auf Basis eines **geratenen**
  Basistakts, wenn CAPABILITIES 0 meldet. In QEMU sind es real 13 MHz.
* **High Speed wird nie ausgehandelt** (kein CMD6, `HCTL_HS_EN` = 0).
* **74 Takte vor CMD0 fehlen.**
* **Taktwechsel unmittelbar nach dem R1b-Kommando CMD7**, ohne den Busy
  abzuwarten — §3.2.2 verbietet genau das.

**Wichtig fuer die Einordnung:** Busbreite und Takt erklaeren die Sekunden
**nicht**. Die Datenphase ist in der Messung winzig: `dev.w.pio` 1–4 µs je
Sektor, `dev.r.read_rdy` 497–1 092 µs. Selbst eine Vervierfachung des
Durchsatzes wuerde in Block 2 unter 10 ms bewegen. Das sind Durchsatzmaengel,
keine Ursache von FS04 — und sie werden hier **nicht** dazu gemacht.

### 3.4 Frage 4 — Einzelblock statt Mehrblock

Der Controller kann es: `CMDTM` hat `TM_MULTI_BLOCK` (Bit 5) und
`TM_BLKCNT_EN` (Bit 1), `BLKSIZECNT[31:16]` traegt die Blockzahl, und
`ARG2` existiert fuer ACMD23 — alles im Register-Map von
`docs/hardware/emmc-sd.md` §1 und in BCM2835 Kap. 5 beschrieben. Unser
Treiber benutzt **nur** CMD17/CMD24 und schleift sektorweise
(`emmc.rs:343-345`, `:353-355`).

Broadcom dazu, BCM2835 Kap. 5 S. 66: *„In order to get maximum performance
for data transfers it is necessary to use **multiple block data transfers**.
In this case the EMMC module uses two FIFOs in ping-pong mode … If the EMMC
module is configured for **single block transfers only one FIFO is used**, so
no DMA access is possible while data is transferred to/from the card and vice
versa **resulting in long dead times**."*

**Fuer FS04 ist das trotzdem nicht die Ursache, und es wird hier auch nicht
dazu gestreckt:** die Messung zeigt `blk.read_blocks max/call = 1` und
`blk.write_blocks max/call = 1` in **allen** 14 Bloecken. Der Blockcache
(`cache.rs`) arbeitet sektorweise, es wird also nie mehr als ein Sektor
angefordert. **Der Begrenzer liegt ueber dem Treiber, nicht in ihm.**
Mehrblock wuerde erst wirken, wenn der Cache (oder `Rwabs`, das
mehrsektorige Anforderungen durchaus durchreicht, `bios/mod.rs:190-196`)
zusammenhaengende Laeufe buendelt. Eigenstaendige Frage, eigene Arbeit.

### 3.5 Frage 5 — was ist am ersten Zugriff auf eine Partition anders?

**In unserem Code: fast nichts.** Gelesen, nicht vermutet:

* `Volume::mount` (`tos-api/src/bdos/fs/fat.rs:96-110`) haelt eine
  Tabelle `MOUNTED[16]`; nur der erste Zugriff auf ein Laufwerk laeuft in
  `log_in` (`:112-130`) → `bios::bpb::geometry` (`bios/bpb.rs:232-243`) →
  **eine** Bootsektorlesung. Gemessen ~0,7 ms (`dev.read_one` avg).
* `bios::mount()` (das alle Laufwerke ausloggt, `bios/mod.rs:31`) wird
  **nur beim Booten** aufgerufen (`kernel/src/lib.rs:308`) — geprueft, es
  gibt keine zweite Aufrufstelle ausser dem Selftest. **Eine
  Neuinitialisierung je Zugriff findet nicht statt.** Ebenso ist
  `emmc::init()` idempotent (`emmc.rs:372-374`).
* `alloc_cluster` (`fat.rs:248-276`) sucht ab dem ersten FAT-Sektor; auf
  einer leeren Partition ist Cluster 2 sofort frei — eine Lesung.
* Der Sektorcache hat 16 Plaetze fuer alle Laufwerke gemeinsam
  (`cache.rs:18`). Beim Partitionswechsel sind FAT-, Wurzel- und
  Bootsektor der neuen Partition kalt: in der Messung **13 Lesungen** im
  kalten Block gegen **7** im warmen.

Zusammen sind das eine Handvoll zusaetzlicher Sektorzugriffe, also
einstellige Millisekunden. **Sekunden entstehen dort nicht.**

**Anders ist die Karte.** Die Messung zeigt es direkt, an der
Programmierzeit **eines** Sektors:

| Block | Partition | `dev.w.data_done` je Sektor |
|---|---|---|
| 2 | erste Beruehrung von D: | 1 966 … 14 771 µs (die 12 laengeren wurden abgebrochen, ihre Zeit steht in `w.wait_dat`, max 62 890 µs) |
| **4** | erste Beruehrung einer **dritten** Partition | avg 84 693 µs, **max 170 002 µs** |
| 3 | dieselbe Partition wie Block 2, zweites Mal | avg 2 082 µs, max 2 288 µs |
| 5, 8–14 | bereits beruehrte Partitionen | avg 2 017 … 2 253 µs |

**Faktor 40–80 zwischen der ersten und jeder weiteren Schreibung in
dieselbe Gegend der Karte.** Das ist die Signatur der Flash-Translation-Layer
der Karte: der erste Schreibzugriff in einen seit dem Einschalten
unberuehrten Bereich oeffnet einen Loeschblock und aktualisiert die
Zuordnungstabelle; Folgeschreibungen in denselben offenen Block sind billig.
Partitionen liegen hunderte MB auseinander, also ist „einmal je Partition"
genau „einmal je Kartenbereich". Die Spec deckt das ausdruecklich: bis
250 ms Busy je Schreibvorgang sind erlaubt (§4.6.2.2), und der Application
Note empfiehlt dem Host **ueber 500 ms** Timeout.

> **Antwort auf die schaerfste Frage des Tickets: unser Code tut beim ersten
> Zugriff auf eine Partition nichts Teures — er zahlt eine Bootsektorlesung
> und ein paar kalte Cachezeilen. Teuer ist die erste Schreibung in einen
> kalten Kartenbereich, und das ist spezifikationskonformes
> Kartenverhalten.** Unser Anteil daran ist, dass wir diese legale Latenz
> nicht aushalten und sie durch Wiederholung ungefaehr verdoppeln.

---

## 4. Befunde, nach Schwere

### Befund 1 — `SPIN_LIMIT` ist kein Timeout und liegt unter der zugelassenen Kartenlatenz · **BELEGT** · Fehler

* **Quelle:** SD Phys Simplified Spec 6.00 §4.6.2.2 (250 ms Busy bei SDHC,
  500 ms fuer den letzten Busy; Application Note: „strongly recommended …
  more than 500 ms"); §4.6.2.1 (Lesen: 100 ms Minimum).
* **Unser Code:** `kernel/src/drivers/bcm2711/emmc.rs:91`
  (`const SPIN_LIMIT: u32 = 1_000_000;`), benutzt in
  `wait_clear` `:151` und `wait_interrupt` `:162`.
* **Spec-Stand:** `docs/hardware/emmc-sd.md` nennt **keinerlei** Zeitgrenzen —
  weder die 250/500 ms noch die 100 ms. Der Treiber konnte sie gar nicht
  einhalten. **An den Researcher.**
* **Fehlerfall:** Kleine Datei auf eine seit dem Einschalten unberuehrte
  Partition kopieren. Die Karte braucht fuer einen Sektor legal bis 250 ms
  (gemessen 170 ms). Das Budget betraegt je nach CPU-Takt 104–200 ms. Ist es
  kuerzer als die Latenz, bricht das Warten ab; das Kommando gilt als
  gescheitert, obwohl die Karte gerade korrekt arbeitet.
* **Belegt durch:** 13 Bloecke Kalibrierung (200,0 ns/Durchlauf), Block 1
  (106 ns), Block 4 (`max/call = 850 010` ↔ 170 002 µs), Block 2
  (`max/call = 1 000 000` = ausgeschoepft, 12 Fehlschlaege).
* **Nebenwirkung, die es besonders unangenehm macht:** Ein Zaehlerbudget
  skaliert mit CPU- und Bustakt. Dieselbe Karte scheitert auf der schnellen
  CPU und geht auf der langsamen durch — in *einem* Messlauf um Faktor 1,9
  auseinander.

### Befund 2 — Nach einem Fehler bleibt der Controller ohne Wiederherstellung stehen · **BELEGT** (Korrelation ueber 14 Bloecke) · Fehler

* **Quelle:** SD-HC Simplified Spec 3.00 §3.10.1 (Error Interrupt Recovery:
  `Software Reset For CMD Line` / `For DAT Line`, Status sichern+loeschen,
  Abort-Kommando, Command Inhibit pruefen, sonst „Non-recoverable"); §3.8
  (Abort Transaction: CMD12 + beide Software-Resets).
* **Unser Code:** `emmc.rs:164-167` (Fehler: nur `INTERRUPT` loeschen,
  `Err(Io)`), `:174` (Timeout: gar nichts, `Err(Timeout)`).
* **Fehlerfall:** Das `DATA_DONE`-Warten laeuft leer aus. Die Karte
  programmiert weiter, der Controller haelt die DAT-Leitung. Der naechste
  Sektorzugriff bleibt in `wait_clear(STATUS, SR_DAT_INHIBIT)` haengen und
  zahlt den Rest desselben Busy.
* **Belegt durch:** `dev.w.wait_dat` ist in **genau dem einen** von 14
  Bloecken ungleich null (537 ms), in dem es die 12 Abbrueche gab; in den
  uebrigen 13 Bloecken 0–1 µs. `dev.r.wait_dat` ebenso (224 ms in Block 2,
  sonst 0–1 µs). Eine Lesung kam dabei mit 92 971 µs auf 89 % des eigenen
  Budgets.

### Befund 3 — Die Antwort der Karte wird nie geprueft · **BELEGT (getrieben)** · Fehler

* **Quelle:** BCM2835 Kap. 5 S. 65: *„The software is responsible for checking
  the status bits of the card's response in order to verify successful
  processing by the card."* SD-HC §3.7.1.2 Schritt (7): *„Check for errors in
  Response Data … Return Status (Response Contents Error)."*
* **Unser Code:** `emmc.rs:299-300`
  (`let resp0 = self.command(READ_SINGLE, addr)?; let _ = resp0;`) und
  `:324` (`self.command(WRITE_SINGLE, addr)?;`). Die R1-Statusbits
  (`OUT_OF_RANGE`, `ADDRESS_ERROR`, `WP_VIOLATION`, `CARD_ECC_FAILED`, `ERROR`
  …) werden nie gelesen.
* **Getriebener Nachweis:** QEMU `raspi4b`, 64-MiB-Karte (131 072 Bloecke),
  Schreiben auf LBA 135 168 (hinter dem Kartenende) →
  `write_blocks(...) -> Ok(())`, `INTERRUPT=0x00000000`. Ein Zugriff weit
  hinter dem Kartenende wird als **Erfolg** gemeldet.
* **Fehlerfall:** Jede karten**seitige** Ablehnung ist fuer uns unsichtbar.
  Schreibgeschuetzte Karte, adressfehlerhafter Zugriff, ECC-Fehler beim Lesen
  — `Fwrite`/`Fread` melden Erfolg.
* **Abgrenzung:** `Rwabs` faengt Bereichsfehler vorher ab
  (`bios/mod.rs:169-181`), der GEMDOS-Dateipfad nicht.

### Befund 4 — Ein fehlgeschlagener Rueckschreibvorgang bei der Verdraengung verliert den Sektor · **BELEGT (Quelltext)** · Fehler, Datenverlust

* **Unser Code:** `tos-api/src/bdos/fs/cache.rs:63-72`:

  ```rust
  if slots[v].valid && slots[v].dirty {
      if let Some(b) = block() {
          b.write_blocks(slots[v].lba, &slots[v].data);   // Ergebnis verworfen
      }
      slots[v].dirty = false;                             // bedingungslos
  }
  ```

  `bool` ist nicht `#[must_use]`, deshalb schweigt der Compiler.
  `flush()` (`:128`) macht es richtig (dirty bleibt stehen) — `evict()` nicht.
* **Fehlerfall:** 16 Cache-Plaetze, alle belegt und schmutzig; ein weiterer
  Sektor kommt herein; der Rueckschreibvorgang der Verdraengung scheitert
  (auf der Pi in Block 2 mit 12/23 Wahrscheinlichkeit real gegeben, es gab dort
  `cache.evict.wb n=1`); der Platz wird als sauber markiert und
  ueberschrieben. **Der Sektor ist weg, ohne jede Meldung.** Bei einem
  FAT-Sektor ist das eine Dateisystembeschaedigung.
* **Einordnung:** Dass der Verlust in diesem Messlauf eingetreten ist, ist
  **nicht** belegt (die eine Verdraengung kann die erfolgreiche gewesen sein)
  — der lose Pfad selbst ist belegt und braucht keine weitere Messung.

### Befund 5 — Geraetefehler erreichen GEMDOS nie · **BELEGT (Quelltext, maschinell)** · Fehler

* **Unser Code:** `kernel/src/host_env.rs:401-404` (`BlockError` → `bool`);
  `cache.rs:104-119` (`write()` gibt **immer** `true` zurueck);
  alle **neun** Aufrufstellen von `cache::flush()` verwerfen das Ergebnis
  (cite stale 2026-10-26: tos-api/src/bdos/fs/file.rs:273,446,556,679,782,865,914, tos-api/src/bdos/fs/dirops.rs:102,152).
  `EWRITF`/`EREADF` kommen in `tos-api/src` ausschliesslich in
  `bios/mod.rs:200,202` vor.
* **Fehlerfall:** `Fwrite` liefert die volle Byte-Zahl zurueck, `Fclose`
  liefert `E_OK`, obwohl kein Sektor auf der Karte gelandet ist. Der Desktop
  meldet die Kopie als gelungen.
* **Gegenprobe EmuTOS:** dort meldet der BDOS-Schreibpfad `EWRITF` durch
  (`bdos/fsio.c`/`bdos/fsbuf.c`), der Fehler kommt bis in `Fwrite`.

### Befund 6 — 74-Takte-Regel vor CMD0 verletzt · **VERMUTET (Quelltext gegen Spec, Wirkung ungemessen)** · Fehler

* **Quelle:** SD Phys §6.4.1.4 (5): *„After 1ms VDD stable time, host provides
  at least 74 clocks before issuing the first command"*; §6.4.1.**1**
  (Text unter Figur 6-4; im Audit stand §6.4.1.2, korrigiert REF02): *„start to
  supply at least 74 SD clocks to the SD card with keeping CMD line to high"*.
* **Unser Code:** `emmc.rs:234` (`CLK_EN`), `:210-211`, `:242` (`GO_IDLE`).
  Bei 400 kHz waeren 185 µs zu warten; dazwischen liegen einige hundert
  Nanosekunden.
* **Fehlerfall:** Karten, die die Vorbereitungstakte brauchen, antworten auf
  CMD0/CMD8 nicht → `init()` schlaegt fehl → kein Laufwerk. Sporadisch,
  kartenabhaengig, kaltstartabhaengig.

### Befund 7 — Taktstopp waehrend eines laufenden Busy · **VERMUTET (Quelltext gegen Spec)** · Fehler

* **Quelle:** SD-HC §3.2.2: *„The Host Driver shall **not** stop the SD Clock
  when a SD transaction is occurring on the SD Bus — namely, when either
  Command Inhibit (DAT) or Command Inhibit (CMD) in the Present State register
  is set to 1."* §3.7.1.2 (4)(5): bei R1b ist auf Transfer Complete zu warten.
* **Unser Code:** `emmc.rs:272` (CMD7, `RSP_48BUSY`), unmittelbar gefolgt von
  `:279` `set_clock(25_000_000)`, das `CONTROL1` ohne `CLK_EN` schreibt
  (`:227`) — also den Takt anhaelt, ohne Command Inhibit (DAT) zu pruefen.
* **Fehlerfall:** Karte bleibt nach CMD7 im Busy, der Takt wird
  weggenommen, die Karte kann den Busy nicht beenden. Zeigt sich als
  haengender erster Zugriff nach dem Booten.

### Befund 8 — `Transfer Complete` zusammen mit `Data Timeout Error` wird als Fehler gemeldet · **VERMUTET (Quelltext gegen Spec)** · Fehler

* **Quelle:** SD-HC §2.2.17, Normal Interrupt Status Bit 01: *„Transfer
  Complete has **higher priority** than Data Timeout Error. If both bits are
  set to 1, execution of a command can be considered to be completed."*
* **Unser Code:** `emmc.rs:161-175` prueft `INT_ERROR_MASK` in `:164`, also
  **vor** dem Flagtest in `:168`, und meldet `Err(Io)`.
* **Fehlerfall:** Ein Transfer, der fertig ist und nebenher den
  Data-Timeout gesetzt hat, gilt uns als gescheitert → Befund 2 und die
  Wiederholung aus Befund 5 schliessen sich an.

### Befund 9 — `set_clock` kann keinen Fehler melden; der Basistakt ist geraten · **BELEGT (Quelltext)** · Fehler, zugleich Verstoss gegen die Keine-Stubs-Regel

* **Quelle:** SD-HC §3.2.1 (1): *„If Base Clock Frequency For SD Clock is
  00 0000b, the Host System shall provide this information to the Host Driver
  by another method."* `docs/hardware/emmc-sd.md` §2 (Schlusskasten) schreibt die Methode
  vor (Mailbox `0x00030002`).
* **Unser Code:** `emmc.rs:219` (`if base_mhz == 0 { 100_000_000 }`),
  `:228-233` (CLK_STABLE-Schleife ohne Fehlerpfad), `:235` (`Ok(())`
  bedingungslos). Die bereits vorhandene
  `kernel/src/drivers/bcm2711/mailbox.rs:419 get_clock_rate()` wird nicht
  benutzt. Genauso ignoriert `reset_and_clock` `:200-205` einen nie
  abgeschlossenen Reset.
* **Fehlerfall:** Meldet EMMC2 CAPABILITIES[15:8] = 0 und liegt der wahre
  Takt nicht bei 100 MHz, laufen Identifikation und Transfer mit dem falschen
  Vielfachen — bei 200 MHz waeren es 800 kHz statt 400 kHz (ueber fOD) und
  50 MHz statt 25 MHz (ueber fPP, bei ausgeschaltetem High Speed). **Das ist
  eine ernstzunehmende Zweitursache fuer die 12 Fehlschlaege**, weil ein
  ueberzogener Bustakt CRC-/Timeout-Fehler erzeugt. Die Sonde kann die beiden
  nicht unterscheiden, weil `wait_interrupt` die Fehlerbits loescht, ohne sie
  zu melden. Entscheidende Messung in §5.

### Befund 10 — Teilermodus wird nicht abgefragt · **VERMUTET** · bedingter Fehler

* **Quelle:** SD-HC §2.2.14: 8-Bit-Teilermodus (HC 1.00/2.00) erlaubt **nur
  Zweierpotenzen**; 10-Bit-Modus (HC 3.00) erlaubt alle Werte. §2.2.26:
  Bedeutung von CAPABILITIES[15:8] haengt ebenfalls an der HC-Version.
* **Unser Code:** `emmc.rs:378-381` liest `0xFC` nur als Anwesenheitsprobe und
  verwirft den Wert; `set_clock` `:225-226` schreibt beliebige Teiler.
* **Stand:** QEMU meldet `SLOTISR_VER = 0x2402_0000` → Spec-Version 0x02 =
  **SDHCI 3.00**, dort also folgenlos. Fuer EMMC2 auf der Pi ungeprueft.
  Waere es dort 1.00/2.00, liefe die Identifikation mit 781 kHz.

### Befund 11 — ACMD41-Schleife ohne Zeitgrenze · **BELEGT (Quelltext)** · kleiner Fehler

* **Quelle:** SD Phys §4.2.3: *„Card initialization shall be completed within
  1 second from the first ACMD41"*; *„The host shall set ACMD41 timeout more
  than 1 second"*.
* **Unser Code:** `emmc.rs:250-256`, bis zu 1 000 000 **Kommandopaare** à
  ~10 µs ≈ 10 s. Nur Bootzeit, kein Datenrisiko — aber es ist wieder ein
  Zaehler statt einer Zeit.

### Befund 12 — Fehlerursache wird geloescht statt gemeldet · **BELEGT (Quelltext)** · Beobachtbarkeitsdefekt, blockiert die naechste Messung

* **Unser Code:** `emmc.rs:165` (`wr(INTERRUPT, i)` loescht **alle** Bits,
  bevor irgendjemand sie sehen konnte), `:182`
  (`wr(INTERRUPT, rd(INTERRUPT))` loescht am Kommandoanfang pauschal alles,
  auch ein spaet eintreffendes Transfer Complete des abgebrochenen
  Vorgaengers). `BlockError::Io` unterscheidet CRC, Timeout, End-Bit,
  Index nicht.
* **Wirkung:** Genau deshalb kann die vorliegende Pi-Messung Befund 1 und
  Befund 9 nicht trennen. Siehe §5.

### Nicht-Befunde — ausdruecklich entlastet

* **1-Bit-Bus / kein High Speed / kein Mehrblock:** reale
  Durchsatzabweichungen (Befunde fuer ein eigenes Ticket), **aber nicht die
  Ursache von FS04.** Die Datenphase betraegt in der Messung 1–4 µs
  (schreiben) bzw. 0,5–1,1 ms (lesen) je Sektor; die Sekunden liegen in den
  Busy-Warten.
* **Kein DMA, keine IRQ:** kostet CPU-Zeit waehrend des Wartens, aendert an
  der Kartenlatenz nichts. Fuer FS04 ohne Wirkung.
* **Neuinitialisierung je Zugriff:** findet **nicht** statt (`emmc::init`
  idempotent `:372-374`, `bios::mount()` nur im Bootpfad `kernel/src/lib.rs:397`)
.
  Damit ist ein Verdacht aus dem Ticket ausgeraeumt.
* **`CONTROL0 = 0x0F00`:** fuer Standard-SDHCI (EMMC2) korrekt (Power +
  3,3 V). BCM2835 fuehrt CONTROL0[15:6] als reserviert — das betrifft den
  aelteren Arasan-Block, nicht EMMC2. Keine beobachtete Wirkung.
* **Der Fussboden von ~230 ms je Kopie** gehoert zu GUI06, nicht hierher.

---

## 5. Was auf der Pi zu messen ist — und welche Zahl was entscheidet

Alles in **einer** Sonde, ohne Kartenbeschreibung, wenige Zeilen:

**M1 — Controllerkennung (entscheidet Befund 9 und 10).**
Nach `init()` einmal ausgeben, fuer **beide** Basen (`0xFE34_0000`,
`0xFE30_0000`): `SLOTISR_VER` (0xFC), `CAPABILITIES` (0x40), `CONTROL0`
(0x28), `CONTROL1` (0x2C), und dazu `mailbox::get_clock_rate()` fuer die
EMMC-Clock-IDs.

* `SLOTISR_VER` Bits 23:16: **≥ 0x02** ⇒ SDHCI 3.00, Befund 10 erledigt;
  **0x00/0x01** ⇒ Befund 10 ist real, die Identifikation lief mit 781 kHz.
* `CAPABILITIES[15:8]`: **0** ⇒ unser 100-MHz-Rateschritt greift, und die
  Mailboxzahl sagt, ob er stimmt. Weicht die Mailboxzahl um Faktor 2 ab,
  laufen wir mit doppeltem Bustakt — dann ist **Befund 9 die Hauptursache
  der 12 Fehlschlaege**, nicht Befund 1.
* Aus `CONTROL1` Bits 15:8 und 7:6 den tatsaechlichen SDCLK ausrechnen und
  mitschreiben. **Erwartet 25 MHz; alles darunter erklaert nebenbei die
  680 µs `read_rdy`, alles darueber ist ein Spec-Verstoss.**

**M2 — Fehlerart statt nur Fehlschlag (entscheidet Befund 1 gegen Befund 9).**
In `wait_interrupt` beim Verlassen den Wert von `INTERRUPT` und die
Durchlaufzahl mitzaehlen, getrennt nach Ausgang: `Ok`, `Err(Io)` mit den
gesetzten Fehlerbits (16=CTO, 17=CCRC, 20=DTO, 21=DCRC, 22=DEND), `Err(Timeout)`.

* **Sind die 12 Fehlschlaege `Err(Timeout)` mit Durchlaufzahl 1 000 000** →
  Befund 1 bestaetigt (so sagt es die Bilanz in §3.2 voraus).
* **Sind es `Err(Io)` mit DCRC/DEND** → Signalintegritaet/Takt, also
  Befund 9/10, und die Arbeit verschiebt sich auf die Initialisierung.
* **Sind es `Err(Io)` mit DTO (Bit 20)** → der Controller selbst hat
  abgebrochen; dann gehoert `DATA_TOUNIT` geprueft (wir setzen `0xE`, den
  Maximalwert; in QEMU ergibt das mit TMCLK 52 MHz **2,58 s**).

**M3 — Gegenprobe zum Budget (entscheidet Befund 1 direkt, eine Zeile).**
`SPIN_LIMIT` versuchsweise auf 10 000 000 setzen (also ~1–2 s statt
104–200 ms) und **dieselbe Geste** wiederholen.

* **Erwartung, wenn Befund 1 stimmt:** `dev.w.data_done n` = `dev.w.pio n`
  (keine Fehlschlaege mehr), `blk.write_blocks calls` faellt von 23 auf
  ~11, `dev.w.wait_dat` faellt auf 0, `emmc.spin max/call` bleibt unter dem
  Limit, und die Wandzeit von Block 2 **halbiert sich auf ~1,0–1,2 s**.
* **Bleibt die Wandzeit bei ~2,2 s und verschwinden die Fehlschlaege
  trotzdem**, dann war die Kartenlatenz wirklich die ganzen 2 s und die
  Wiederholungen waren kostenlos — dann liegt die Arbeit bei Mehrblock,
  4 Bit und einem Cache, der weniger Einzelsektoren schreibt.
* **Das ist die billigste entscheidende Messung im ganzen Ticket.** Sie
  ersetzt keine Korrektur (ein rohes Hochsetzen von `SPIN_LIMIT` bleibt ein
  Zaehler statt einer Zeit), aber sie trennt „Karte" von „wir" endgueltig.

**M4 — nur falls M1 einen falschen Bustakt zeigt:** dieselbe Geste mit
`set_clock` auf einen sicher konservativen Wert (z. B. 12,5 MHz). Verschwinden
die Fehlschlaege, ist es der Takt.

---

## 6. Rueckbau

Die temporaere Verdrahtung lag ausschliesslich in
`kernel/src/selftest_emmc.rs` (Registerdump + Fehlerpfadtest, beide hinter
dem bestehenden Feature `selftest-emmc`) und ist **von Hand
zurueckgeschrieben**; `git status` zeigt die Datei wieder als unveraendert.
Kein `git checkout/restore/clean/stash/reset`. Produktcode wurde nicht
geaendert.

Beruehrt bleiben nur ignorierte Artefakte: `target/` und **`kernel8.img`** im
Wurzelverzeichnis (Build-Artefakt, in `.gitignore`), das durch die Laeufe
ueberschrieben wurde — zuletzt als planer `--board rpi4 --release`-Build
**ohne** Selftest-Feature. Wer es braucht, baut es frisch je Board neu.
