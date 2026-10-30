# XBIOS06 — `Rsconf`: Geraeteprofil der Registerbytes und Iorec(0)-Schreibzugriff — 2026-10-30

Stand: Researcher-Teil von `TASK/XBIOS06.md` (Arbeitspunkt 1). Fakten mit
`file:line`, Optionen mit Wirkung/Kosten/Wegwerfanteil/Fahrplan, Empfehlung
ausdruecklich als solche markiert. Kein Produktcode. Die Entscheidung trifft
der Anwender.

Legende: **[Q]** = aus Quelltext gelesen (zitiert); **[M]** = gemessen (Lauf
zitiert); **[I]** = abgeleitet, nicht gelesen/gemessen.

## 1. Auftrag A — Spec gegen das Trap-ABI korrigiert

`docs/api/xbios/0f-rsconf.md` ist korrigiert (Abschnitte "Native (AArch64)
ABI", "Installed handler", "Outputs", "Side effects", "Error conditions",
"Edge cases" 4-6, "Configuration", "NewTOS implementation notes"):

- **Vertrag:** das `ULONG` aus `d0` kommt vorzeichenerweitert in `x0`
  (`tos-api/src/trap.rs:6@TrapArgs`; `docs/COMPATIBILITY.md` Abschnitt "LONG
  results in `x0`: sign-extended"; `docs/TOS-REFERENCE-DUMPER.md` Abschnitt 3
  `ret_hi`, Tabellenzeile 296). Weil das UCR-Byte in Bit 31..24 steht, ist
  jedes Ergebnis mit gesetztem UCR-Bit 7 (Einschaltwert `0x88`) negativ. [Q]
- **Heute (Abweichung):** nullerweitert,
  `tos-api/src/xbios/system.rs:286@old` (`old as i64`, `old: u32`); die
  Kommentare `:253@rsconf` und `:285` beschreiben diesen Code und aendern sich
  mit dem Developer-Fix. [Q]
- **Gemessen:** TT `0xFFFFFFFF_88018100`, Falcon `0xFFFFFFFF_88000000`,
  NewTOS `0x00000000_88010100` (`tosdump/cases/bios-r1.tsv:105@Rsconf`,
  `tosdump/cases/xbios-chardev-r1.tsv:31@Rsconf`, Probe S02 in
  `measurements/measurement-2026-10-28-xbios-chardev-audit/runs/PROBE-EMUTOS-tt/XCPROBE.LOG`,
  `measurements/measurement-2026-10-28-xbios-chardev-audit/runs/PROBE-EMUTOS-falcontc/XCPROBE.LOG`, `measurements/measurement-2026-10-28-xbios-chardev-audit/runs/PROBE-NEWTOS-raspi4b/XCPROBE.LOG`). [M]

### Weitere Spec-Korrekturen aus der Neuvalidierung (gegen die Quelle)

1. Die Spec kannte nur `rsconf_mfp` als "den" Handler. Tatsaechlich haengt der
   Handler an der **Maschine**: `init_bconmap` setzt `mapped_device` auf 7 bei
   Falcon/ARAnyM, sonst 6 (`references/emutos/bios/serport.c:966`) und kopiert
   `maptable[mapped_device-6]` in `rsconfptr` (`:997-1003`). Falcon/MSTe:
   `maptable[1]` = SCC B (`:974-979`), also `rsconfB` -> `rsconf_scc`
   (`:872-876`); TT/ST: `maptable[0]` = `maptable_mfp` -> `rsconf1` ->
   `rsconf_mfp(MFP_BASE,&iorec1,..)` (`:386-389`, `:972`). [Q]
2. **Das SCC-Paket war falsch/leer beschrieben.** `rsconf_scc`
   (`serport.c:775-863`): `old = iorec->ucr << 24`, plus `0x0800` wenn das
   Schatten-`wr5` Bit 4 (Break) traegt (`:788-790`); RSR und SCR sind 0, im TSR
   gibt es nur Bit 3 (Kommentar `:783-786`). Der UCR-Wert ist der *gemerkte*
   zuletzt gesetzte (`iorec->ucr`, Einschaltwert `0x88`, `:89`), keine
   Hardware. [Q] Das passt zur Falcon-Messung `0x88000000`. [M]
3. **`rsconf_mfp` liest echte Hardwareregister** (`mfp->ucr/rsr/tsr`,
   `:229`); TT-Messung `0x88 01 81 00`. [Q/M]
4. **Eingabeverarbeitung:** `baud` ausserhalb 0..15 und `ctrl` ausserhalb 0..3
   werden uebersprungen (`:231`, `:237`; SCC `:792,797`); der alte Satz
   "Out-of-range values are passed to hardware without validation" galt nur
   fuer `ucr/rsr/tsr/scr`. [Q]
5. **Kein `iorec->ucr` im MFP-Pfad:** `rsconf_mfp` schreibt nur `baudrate`
   (`:232`) und `flowctrl` (`:238`) in den Record; der SCC-Pfad zusaetzlich
   `ucr` (`:811`), `datamask` (`:830`), `wr5` (`:832`, `:857`). [Q]

### Korrekturen am Ticket (Zitate)

- Das Ticket zitiert `tos-api/src/xbios/system.rs:239@RS_TSR` und
  `:247-279@rsconf`; heute `:239@RS_TSR`, `rsconf` ab `:254`, `old as i64` in
  `:286`. (Zeilen verschoben, Sachverhalt unveraendert.)
- **Das Ticket sagt, `Bcostat` liefere `-1`, wenn `tsr & 0x80`
  (`serport.c:277`).** Das gilt nur im Zweig `RS232_DEBUG_PRINT`
  (`serport.c:276-277`; `RS232_DEBUG_PRINT` ist nur bei `CONF_SERIAL_CONSOLE
  && !CONF_WITH_COLDFIRE_RS232` 1, `include/config.h:1963-1968`; der
  Referenzbau make 512 hat `CONF_SERIAL_CONSOLE` 0, `docs/api/_emutos-config.md`
  Zeile 624). Im Normalbau antwortet `bcostat1` aus dem Ausgabering
  (`serport.c:279-282`), nicht aus dem TSR; `bcostatTT`/`bcostatA`/`bcostatB`
  ebenso (`:410-415`, `:509-514`). TSR Bit 7 steht im Normalbau nur in
  `bconout1` (`:314`) und `bconoutTT` (`:433`) als "darf direkt senden". [Q]
  Folge: das Argument "Bit 7 ist Bcostat" traegt nicht; es bleibt das Argument
  "ein Programm, das den Rsconf-Rueckgabewert auf Bit 7 abfragt" [I].

## 2. Auftrag B — Geraeteprofil der Registerbytes

### 2.1 Fakten: was beansprucht NewTOS heute?

| Flaeche | Anspruch | Beleg |
|---|---|---|
| `Bconmap` | Tabelle einer Maschine, die weder Falcon noch TT ist: `maptabsize` 1, `mapped_device` 6, Eintrag 0 = AUX = EmuTOS' `maptable_mfp` (MFP-Port) | `tos-api/src/xbios/bconmap.rs:38@MAPTABSIZE`, `:92@MAPTABSIZE`, `:116@MAPPED`; `docs/COMPATIBILITY.md` Bconmap-Tabelle (TT EQUAL, Falcon ACCEPTED); `references/emutos/bios/serport.c:962-966,972` |
| `Rsconf` | `rsconf1` -> `rsconf_mfp` ("the contract NewTOS rsconf implements over software shadows"); `rsconfB` (Falcon SCC B) bewusst `unmapped` | `docs/analysis/deps/mapping.tsv:946` (`rsconf1`), `:948` (`rsconfB`), `:951` (`rsconf_mfp`) |
| `Rsconf`-Startwerte | `ucr 0x88`, `rsr 1`, `tsr 1` = die Argumente von EmuTOS' `Rsconf(DEFAULT_BAUDRATE,0,0x88,1,1,0)` (`serport.c:1066`), Baud 1, Flow 0 | `tos-api/src/xbios/system.rs:235@RS_BAUD`..`:239@RS_TSR` |
| Zeichen-E/A (`Bconin/stat/out/costat`) | gleiche `AuxSerial`-Koerper fuer alle; im Mapping als "Falcon target profile AUX (SCC B ...; SERRX-004)" beschriftet (`bconinB`, `bconstatB`, `bconoutB`, `bcostatB`) **und** als `bconin1`/`bcostat1` | `docs/analysis/deps/mapping.tsv:906-931` |
| `Bcostat(1)` / AUX-Ausgabe | immer `-1`, Byte verworfen | `docs/analysis/deps/mapping.tsv:927-928`, `kernel/src/console.rs:218@bcostat` |
| Referenzbau | EmuTOS make 512 = Falcon030/TOS4-Profil, alle `CONF_WITH_*` mitkompiliert; "Ausnahme nur die HW-Ebene" | `docs/MEMORY.md` Zeile 65, `docs/api/_emutos-config.md` Zeilen 64-68 |

**Antwort auf die Schluesselfrage [I aus den Belegen]:** Die *Tabellenseite*
(Bconmap, Rsconf-Zuordnung, Startwerte) beansprucht heute den **MFP-Port auf
Geraet 6** (Maschine ohne SCC-Eintrag). Nur die *Beschriftung* der
Zeichen-E/A-Zeilen nennt Falcon/SCC B; das Verhalten dort ist ein einziger
Port ohne Kabel und beweist keinen SCC-Anspruch. Konsistent zu *dieser*
Tabellenseite ist die MFP-Antwort (Option a); die SCC-Antwort passt zur
Falcon-Beschriftung und zum Referenzbau, aber nicht zur gewaehlten
Bconmap-Tabelle (Geraet 7 existiert in NewTOS nicht, `0` fuer Geraet 7,
`docs/COMPATIBILITY.md`: `Bcostat(7)` ACCEPTED "no port 7").

**Mischung ist schon da:** Das Repo fuehrt zwei Profile nebeneinander (Zeichen-
E/A als SCC B beschriftet, Bconmap/Rsconf MFP). Das ist als Zuordnungs-Etikett
harmlos, solange kein Programm es beobachtet; `Rsconf` ist die erste Stelle,
an der die Bytes es beobachtbar machen.

### 2.2 Fakten: Bytes je Profil

Messung (Probe S01-S18, `.../runs/PROBE-*/XCPROBE.LOG`) [M]:

| Aufruf | TT (MFP) | Falcon (SCC B) | NewTOS heute (nullerw.) |
|---|---|---|---|
| S02 `Rsconf(-1 x6)` | `ffffffff 88018100` | `ffffffff 88000000` | `00000000 88010100` |
| S11 nach `ucr=0x98` | `98018100` | `98000000` | `98010100` |
| S13 nach `rsr=0` | `98008100` | `98000000` | `98000100` |
| S15 (Lesen nach S14: `ucr=0x188`, Hochbyte) | `88008100` | `88000000` | `88000100` |
| `-2`-Abfragen (S01,03,05,07,09,18) | gleich auf allen | gleich | gleich |

(Die UCR-Maskierung auf ein Byte ist auf allen drei
Seiten gleich — Ticket "Sonde S01–S18 gleich" fuer alles ausser TSR-Byte und
Hochhaelfte.) Die RSR-Byte-Abweichung NewTOS-vs-Falcon (`01` gegen `00`) und
die TSR-Byte-Abweichung (`01` gegen `00`) sind im Ticket-Sachverhalt nur
teilweise genannt: **gegen Falcon weichen zwei Bytes ab, gegen TT eines.** [M]

### 2.3 Optionen

Alle Optionen enthalten die Vorzeichenerweiterung (Vertrag, unstrittig, A).
Tabelle: Wirkung auf die zwei Tabellenfaelle `bios-r1.tsv:105@Rsconf` und
`xbios-chardev-r1.tsv:31@Rsconf` (die Fragen `-2` sind schon EQUAL) und auf Probe
S02/S04/S06/S08/S10-S17.

| | (a) TT/MFP-Bytes, TSR Bit 7 gesetzt | (b) Falcon/SCC-Bytes | (c) eigene Bytes + COMPATIBILITY |
|---|---|---|---|
| Rueckgabe `Rsconf(-1 x6)` in Ruhe | `0xFFFFFFFF_88018100` | `0xFFFFFFFF_88000000` | `0xFFFFFFFF_88010100` (heute plus Erweiterung) |
| `bios-r1` 105 / `xbios-chardev-r1` 31 vs **tt** | EQUAL | DIFF (RSR `01`, TSR `81`), ACCEPTED | DIFF (TSR `81`), ACCEPTED |
| dieselben vs **falcontc** | DIFF (RSR, TSR), ACCEPTED | EQUAL | DIFF (RSR, TSR), ACCEPTED |
| Probe S: Lesezeilen vs tt | alle EQUAL, *wenn* TSR-Schreibsemantik trifft (siehe unten) | DIFF in jeder Lesezeile | DIFF in jeder Lesezeile |
| Probe S: Lesezeilen vs falcontc | DIFF in jeder Lesezeile (RSR+TSR-Bytes) | EQUAL | DIFF |
| Konsistenz mit Bconmap (6 = MFP, Eintrag 0) | ja (`rsconf1`) | nein: dort liefe Geraet 7 = SCC B, das es in NewTOS nicht gibt | neutral |
| Konsistenz mit `Bcostat(1)` | TSR Bit 7 = "Sender leer" passt zu `Bcostat` immer `-1` (AUX nimmt immer an) | TSR Bit 7 nie gesetzt; SCC-Programme fragen kein Bit 7 ab (EmuTOS liefert es dort auch nicht, `serport.c:783-790`) | TSR `0x01`: "Sender leer" nie gemeldet, obwohl Bcostat `-1` meldet |
| Programme, die Bit 7 des Rueckgabe-TSR abfragen (ST/TT-Stil) | funktionieren | warten ewig (wie auf einem echten Falcon, Bit 7 dort 0) | warten ewig |
| Programme, die Bit 3 (Break) abfragen | Bit 3 = 0 (`0x81`): kein Break | Bit 3 = 0 | Bit 3 = 0 |
| Programme, die den Rueckgabewert nur als Restore-Wert zurueckgeben (`Rsconf(.., ret>>24, ..)`) | ok | ok | ok |
| Aufwand | klein: `RS_TSR` Anzeige mit Bit 7 verodern; Schreibsemantik klaeren (s.u.); Tests | klein bis mittel: RSR/TSR-Bytes entfallen, TSR-Bit 3 / Break-Schatten fuehren; `rsr`/`scr`-Schreibzugriffe ohne Wirkung | klein: nur Vorzeichen; dafuer COMPATIBILITY-Zeilen |
| Wegwerfanteil | gering (MFP-Bytes bleiben richtig, wenn je Geraet 7+ kommt, dann je Eintrag) | gering/mittel: bei spaeterem MFP-Geraet 6 muss Bytes je Eintrag getrennt werden | hoch: bleibt ein erklaerter Fehler |
| Fahrplan | passt zu XBIOS05 (Tabelle wie Maschine ohne SCC) und zum Bconmap-Muster (TT EQUAL, Falcon ACCEPTED) | passt zum Referenzbau-Profil, kollidiert mit XBIOS05 | widerspricht "1:1 EmuTOS, abweichen nur belegt" |

**TSR-Schreibsemantik bei (a) — offen [I]:** Die Messdaten (TSR geschrieben
`1` und `0x81`, gelesen `0x81`) tragen zwei Lesarten: "Lesewert = gespeichert |
0x80" oder "Lesewert = gespeichert & 0x7F | 0x80". Was ein MFP-TSR bei
Schreibwert `0x00`, `0x08`, `0xFF` zurueckgibt, steht **nicht** im EmuTOS-
Quelltext (`rsconf_mfp` schreibt und liest das Hardware-Register, `:243-244`,
`:229`) und ist hier nicht gemessen. Die Oracle-Seite ist Hatari, nicht ein
echter MFP. Vorschlag: vor der Umsetzung drei Zeilen S19-S21 (`tsr=0x00`,
`0x08`, `0xFF`, jeweils mit Rueckfrage und Wiederherstellung) im Probe auf dem
TT-Profil messen.

**Empfehlung (Researcher, Empfehlung — keine Entscheidung):** **(a)**, mit
COMPATIBILITY-Zeile "Falcon: ACCEPTED (RSR/TSR: SCC liefert 0)" im Muster der
Bconmap-Tabelle. Gruende: (1) einzige Option, die zu der vom Anwender am
2026-10-28 gewaehlten Bconmap-Tabelle (Geraet 6, MFP) passt; (2) TT bleibt wie
bei Bconmap die EQUAL-Seite; (3) beseitigt den Fehlerfall "wartet ewig" fuer
ST/TT-Programme ohne einen Falcon-Fall zu verschlechtern (der Falcon liefert
dort ohnehin 0); (4) kleiner Aufwand. (b) waere die richtige Wahl, falls der
Anwender die gesamte serielle Flaeche auf "Falcon-Port SCC B" umstellt (dann
Bconmap, Bcostat(7) usw. mitziehen — eigenes Ticket). (c) nur, wenn der
Anwender die Bytes ausdruecklich als NewTOS-eigen erklaert; dann bleibt der
Wartefall.

## 3. Auftrag C — Teilpunkt 2026-10-29: `Iorec(0)`-Felder

### 3.1 Fakten

- `Iorec(0)` liefert `rs232iorecptr` (`references/emutos/bios/xbios.c:339-340`),
  den Record des gerade auf Geraet 1 gemappten Ports (`serport.c:1003`). [Q]
- **EmuTOS schreibt beim `Rsconf`** (Feldlage `docs/api/structures/iorec.md`,
  `extiorec`: m68k/LP64-Offsets in Klammern):
  - MFP-Handler: `baudrate` (28/48) <- `baud`, nur bei `0 <= baud <= 15`
    (`serport.c:231-232`); `flowctrl` (29/49) <- `ctrl`, nur bei
    `0 <= ctrl <= 3` (`:237-238`). `ucr` (30/50), `datamask` (31/51), `wr5`
    (32/52) bleiben unberuehrt. [Q]
  - SCC-Handler: dazu `ucr` <- `ucr` (`:811`), `datamask` aus den
    Bits/Zeichen-Bits von `ucr` (`:813-830`: 8 Bit `0xff`, 7 `0x7f`, 6 `0x3f`,
    5 `0x1f`), `wr5` (`:831-832`, Break `:848-857`). [Q]
  - `baud == -2` liefert `iorec->baudrate` zurueck und aendert nichts
    (`:222-223`, `:779-780`): der **Record ist die Quelle der Wahrheit** fuer
    die Abfrage, nicht ein Schatten. [Q]
- **NewTOS:** `Rsconf` fuehrt private Schatten (`tos-api/src/xbios/system.rs:235@RS_BAUD`,
  `:236@RS_FLOW`, `:237@RS_UCR`); der Record `Iorec(0)` behaelt die
  Einschaltzeile `tos-api/src/xbios/iorec.rs:154@AUX_LINE` = `[1, 0, 0x88, 0xFF, 0xEA]`
  (geschrieben `tos-api/src/xbios/iorec.rs:283@AUX_RECORD`). Die Feldoffsets sind schon definiert
  (`tos-api/src/xbios/iorec.rs:104@OFF_BAUDRATE`ff.; `EXT_IOREC_SIZE` 56). [Q]
  `Rsconf(-2)` antwortet aus `RS_BAUD`, nicht aus dem Record
  (`tos-api/src/xbios/system.rs:262@RS_BAUD`). [Q]
- **Folge [I]:** (i) `Iorec(0)->baudrate`/`flowctrl` bleiben nach `Rsconf` auf
  `1`/`0`; (ii) schreibt ein Programm `Iorec(0)->baudrate` und fragt dann
  `Rsconf(-2)`, antwortet EmuTOS den neuen Wert, NewTOS den Schatten; (iii) bei
  einem SCC-Orakel (Falcon) kommen `ucr` und `datamask` hinzu — dasselbe
  Profilthema wie in 2.3. Die Nahtstelle "Kernel antwortet aus eigenem
  Zustand, liest die Seite nie" (`tos-api/src/xbios/bconmap.rs:34-44@MAPPED`) ist ein
  Konflikt zu (ii) und eine Entscheidung fuer den Developer/Anwender.

### 3.2 Entwurf der MEASURED-Sonde (nur Entwurf)

**Warum nicht als tosdump-Tabelle:** die Guard-Whitelist lehnt jedes `Rsconf`
mit gesetztem Baud ab ("Rsconf: a baud rate reprograms the serial port",
`tosdump/core/cases.c:4050-4055@CR_RSCONF`); `+put` kopiert nur ein Feld auf ein anderes
(`docs/TOS-REFERENCE-DUMPER.md` "Writing through a returned pointer"), nicht
ein Argument eines Aufrufs. Der Record-Capture (`r:extiorec`) existiert
(`tosdump/cases/xbios-ptr-r1.tsv`, Zeile `Iorec(0)`), nur ohne vorhergehenden
Set-Aufruf. Nach Sondenregel (CLAUDE.md "Testing") ist daher ein
**dauerhaftes Probe-Programm** unter `tosdump/probes/` der Weg; ein Folgeticket
verschiebt es spaeter nach tosdiff (Guard-Erweiterung fuer Set-Rsconf in einem
TOOL-Ticket).

**Programm** (rsprobe.c (neu, unter tosdump/probes), ein Quelltext, zweimal gebaut; Muster
`xcprobe.c`, `measurements/measurement-2026-10-28-xbios-chardev-audit/method/xcprobe.c`
und `tosdump/probes/mdprobe.c`; Aufrufe ueber `osbind.h` wie ein portiertes
Programm; Feldoffsets m68k 28..32 / LP64 48..52 aus dem `tosdump-layout`-Block
in `docs/api/structures/iorec.md`, **nicht** aus mintlibs `_IOREC`, die kennt
die Erweiterung nicht; ein Selbsttest vergleicht die Offsets mit
`sizeof(IOREC)*2`):

| Zeile | Aufruf | Dump danach (Bytes 0..4 hinter den beiden IOREC + `wr5`) |
|---|---|---|
| P00 | `Iorec(0)` Grundzustand | baudrate, flowctrl, ucr, datamask, wr5 |
| P01-P04 | `Rsconf(0/1/4/15, -1,-1,-1,-1,-1)`, je mit `Rsconf(-2,..)` | dito |
| P05-P06 | `Rsconf(16,..)`, `Rsconf(-3,..)` (ausserhalb, `-1` behaelt) | dito (bleibt) |
| P07-P10 | `Rsconf(-1, 0/1/2/3, ..)` Flow | dito |
| P11 | `Rsconf(-1, 4, ..)`, `Rsconf(-1,-2,..)` | dito (bleibt) |
| P12 | `Rsconf(4, 2, 0x98..)` beides in einem Aufruf | dito |
| P13-P15 | `ucr` = `0x68` (7 Bit), `0x48` (6 Bit), `0x28` (5 Bit) | dito (TT: ucr/datamask bleiben; Falcon: aendern sich) |
| P16 | Programm schreibt `Iorec(0)->baudrate = 7`, danach `Rsconf(-2,..)` | Antwort (Quelle der Wahrheit?) |
| P17 | Wiederherstellung `Rsconf(1,0,0x88,1,1,0)` | dito |
| S19-S21 | `tsr` = `0x00`, `0x08`, `0xFF` mit Rueckfrage (offene Frage 2.3) | Rueckgabewert |

Jede Zeile gibt `ret`, `hi` und die fuenf Bytes aus (Format wie `XCPROBE.LOG`:
`P05 ... ret=%08x hi=%08x ib=%02x if=%02x iu=%02x idm=%02x iw=%02x`); `wr5`
wird ausgegeben, aber wie im Layout als `IGNORE` (Referenz, kein Vertrag,
`docs/api/structures/iorec.md`). Vergleich beidseitig per Skript im Muster von
`tosdump/probes/pxprobe-compare.py` / `gcprobe-compare.py`; Soll "gleich dem
Orakel" gilt fuer `baudrate` und `flowctrl` auf beiden Orakelprofilen (`tt`,
`falcontc`); `ucr`/`datamask` je Profil getrennt (TT bleibt `0x88`/`0xff`,
Falcon folgt `ucr`) und werden gegen die Profilentscheidung aus 2.3 gewertet.

**Vorhandene Infrastruktur, in die er passt:**

- Quelle/Bau: `tosdump/Makefile` (Muster `mdprobe-m68k`/`mdprobe-newtos`,
  Zeilen 204-209) und `tosdump/build-mdprobe-newtos.sh` (ein
  `build-rsprobe-newtos.sh` nach demselben Link-Weg); m68k via
  `$(M68K_CC)`.
- Lauf beidseitig: `tosdump/probes/run-mdprobe.sh emutos tt|falcontc <out>`
  und `... newtos <out> <kernel8.img>` (Hatari ueber `tools/hatari-oracle.sh`,
  NewTOS ueber `tools/mkcard.sh` + QEMU raspi4b, `-append serial=on`, `#Z`-Autorun)
  — der Runner ist fast wiederverwendbar; Anpassung: Probe-Name, Log-Name,
  keine HOLD-Stelle noetig, da die Probe terminiert.
- Eingabetrace auf NewTOS: `--features bios-arg-trace` (CLAUDE.md "Testing",
  Sondenregel).
- Struktur-Capture/TOOL16: liefert die Feldlage (`!layout extiorec`, Abschnitt
  "Structures behind a pointer" in `docs/TOS-REFERENCE-DUMPER.md`); der
  spaetere tosdiff-Fall brauchte eine Guard-Erweiterung fuer `Rsconf` mit
  gesetztem Baud (`tosdump/core/cases.c:4050-4055@CR_RSCONF`) und dann `r:extiorec` nach dem
  Set-Aufruf — Folgeticket, nicht jetzt.
- Rohmuster der Messordnung: ein Lauf-Verzeichnis measurements/measurement-<Datum>-xbios06
  mit `RUNINFO.TXT`, Manifest der Karte, `PROBE-EMUTOS-tt`,
  `PROBE-EMUTOS-falcontc`, `PROBE-NEWTOS-raspi4b` (Muster des Audit-Laufs
  `measurements/measurement-2026-10-28-xbios-chardev-audit/`).

Die Sonde schreibt auf dem Orakel in den seriellen Port (Hatari) bzw. die
NewTOS-AUX-Schatten; sie sendet kein Zeichen. Sie reprogrammiert die Baudrate
auf Hatari (kein Hardwareeffekt, Wiederherstellung P17).

## 4. Offene Fragen an den Anwender / STOP-Bloecke

1. Option a, b oder c (2.3)? Empfehlung (a).
2. Bei (a): TSR-Schreib-/Lesesemantik jenseits `0x01`/`0x81` — nicht im
   Quelltext, nicht gemessen; S19-S21 vorab messen. *Needs deeper analysis /
   measurement.*
3. Soll `Rsconf(-2)` kuenftig aus dem `Iorec(0)`-Record antworten (EmuTOS) oder
   weiter aus dem Schatten (Konflikt mit der Bconmap-Regel "Kernel liest die
   Seite nie", `tos-api/src/xbios/bconmap.rs:34-44@MAPPED`)? Developer/Anwender; messbar mit P16.
4. Schreibt ein Fix beim MFP-Profil nur `baudrate`+`flowctrl` oder (SCC-Profil)
   auch `ucr`/`datamask`? Folgt aus 1.
5. Der Ticket-Satz zu `Bcostat` und `serport.c:277` ist nur im
   `RS232_DEBUG_PRINT`-Bau wahr (1, Korrekturen); Ticketkopf anpassen (nicht
   hier editiert).

## 5. Ausserhalb des Scopes

- Mapping-Etikett: `bconinB`/`bcostatB` (Falcon SCC B) und `bconin1` zeigen
  auf dieselbe NewTOS-Funktion, `rsconfB` ist `unmapped`
  (`docs/analysis/deps/mapping.tsv:906-931,948`) — Profilmischung, siehe 2.1.
- Kein portiertes Programm ruft `Rsconf` (`/usr/bin/grep` ueber `sdk/apps`,
  `sdk/overlays`, `sdk/examples`, `programs`, `tosdump/probes`: nur Treffer fuer
  `Cursconf`; highwire mintlib.lib ist ein Binaerarchiv der Bibliothek,
  kein Aufrufer). Die Luecke "kein lokaler Quelltext" wurde nicht per
  Manifest/`make-sd-image.sh` geschlossen, weil kein Treffer zu deuten war.
- `Rsconf` programmiert den PL011 nicht (nur Schatten); die Baudrate hat
  keinen Hardwareeffekt (`tos-api/src/xbios/system.rs:254@rsconf`) — eigene
  Frage, nicht dieses Tickets.
