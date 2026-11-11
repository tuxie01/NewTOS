# PORT06 — Doom portieren: konform ueber BIOS, AES und VDI

Status: PARKED (2026-11-11) | Priority: P1 | Reason: legacy, evidence unverified; receipt PORT06-STATUS-DONE.MD exists | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 15, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: no cited file of the ticket/receipt is touched by bed4ed417 (feat(sdk): Doom draws straight into the EdDI devic), named by body

Line 3 before the migration:
Status: **ERLEDIGT — Pi-Abnahme 2026-10-04** ([Statusdatei](PORT06-STATUS-DONE.MD); TT als Machbarkeitsbeweis) · Prioritaet: **P1** (vorgezogen vor den OFAM-Termin, Anwender 2026-10-02) · Typ: Portierung — Anwenderwunsch 2026-09-28

Abhaengigkeiten: — (**nicht** von [SDL01](SDL01.md); siehe §2)

## Die Auflage steht vor allem anderen

**Nur BIOS, AES und VDI. Kein `Physbase`, kein `Logbase`, kein `Setscreen`,
kein direkter Griff in den Framebuffer.**

Der Grund ist nicht Formalismus, sondern Reichweite: eine Grafikkarte ersetzt
den **VDI-Treiber**, nicht den Shifter dahinter. Was ueber die VDI geht, laeuft
auf NewTOS, auf einem TT, auf einem Falcon, auf einer Nova, unter Hatari. Was
`Physbase` anfasst, laeuft nur dort, wo der Shifter sitzt.

### Das staerkere Argument ist nicht Portabilitaet, sondern Geschwindigkeit

Anwender 2026-09-28: *„wenn man VDI-Routinen verwendet, dann wird auch die
Hardwarebeschleunigung verwendet."*

**Das dreht die uebliche Abwaegung um.** Der VDI-Treiber einer Grafikkarte kann
benutzen, was die Karte mitbringt; ein Programm, das selbst in den Framebuffer
schreibt, bekommt davon **nichts** — es hat sich die Beschleunigung durch
seinen eigenen Sonderweg weggenommen. Konformitaet ist hier also der
**schnellere** Weg, nicht der brave.

Fuer uns gilt dasselbe in die Zukunft: die VDI-Zeichenroutinen sollen spaeter
in optimiertes aarch64-Assembler wandern
([[vdi-draw-asm-optimization-later]]). Wer ueber die VDI geht, erbt das;
wer am Framebuffer vorbei arbeitet, nicht.

### ST-RAM ist auch auf dem TT kein Thema

Anwender: *„auch am TT brauchen wir kein ST-RAM, weil die Nova von der CPU
bedient wird."*

Das verallgemeinert, was [FS04](FS04.md) belegt hat: EmuTOS bevorzugt ST-RAM
fuer Puffer wegen **Floppy-/ACSI-DMA**
(`references/emutos/include/config.h:1723-1724`). Eine **CPU-bediente** Karte
hat keine DMA-Einschraenkung — der Grund faellt dort genauso weg wie bei uns.
**Kein Pi-Sonderfall, sondern die allgemeine Regel**, und damit traegt die
TT-RAM-Richtung aus §3 auf beiden Maschinen.

**Die Ball-Demo ist der Beleg in beide Richtungen** (`/Volumes/Daten/Development/ball-demo`,
ausserhalb des Repos). Gezaehlt ueber ihre Quellen: `vro_cpyfm` 41x,
`v_opnvwk` 16x, `v_get_pixel` 18x, `vs_color` 9x, `appl_init`/`graf_handle` je
9x — das ist die konforme Haelfte. Daneben `Vsync` 39x, `Setscreen` 21x,
`Logbase` 18x, `Physbase` 15x — die nicht konforme. Dass sie ein `plat-tt.c`
**und** ein `plat-pistorm.c` braucht, zeigt genau diese Trennlinie.

## Entscheidungen 2026-10-02 (Anwender)

**Ziel: ein Quelltext, zwei Ziele — NewTOS auf der Pi (aarch64) und ein Atari
TT mit Nova (m68k).** Konform ueber VDI/AES/BIOS/GEMDOS; was auf dem TT nicht
liefe, ist nicht konform genug.

* **Grundlage: doomgeneric** (GPLv2), `https://github.com/ozkl/doomgeneric`,
  Stand `dcb7a8dbc7a16ce3dda29382ac9aae9d77d21284`, lokal in
  `Applications/doomgeneric` (nicht eingecheckt, wie QED/TeraDesk).
  Plattformschicht: `DG_Init`, `DG_DrawFrame`, `DG_SleepMs`,
  `DG_GetTicksMs`, `DG_GetKey`. Die Portierung liegt in `sdk/apps/doom`.
  Damit ist die Lizenzfrage aus §4 entschieden: die GPL-Fassung.
* **WAD:** Shareware `doom1.wad` v1.9, `Applications/doom-wad/doom1.wad`,
  4 196 020 Byte, MD5 `f0cefca49926d00903cf57551d901abe` (die bekannte
  Shareware-1.9-Summe). Weitergabe erlaubt; nicht eingecheckt.
* **Pixelformat — beide Wege bauen:**
  1. **`vr_trnfm`** aus dem VDI-Standardformat: laeuft ueberall (TT ohne
     Karte, Nova, NewTOS), langsamer.
  2. **EdDI `vq_scrninfo`**, wenn der Cookie `EdDI` da ist: Doom rechnet direkt
     ins gemeldete Geraeteformat, dann `vro_cpyfm`. NVDI/Nova-Treiber liefern
     es; **NewTOS bekommt es als Erweiterung ueber EmuTOS hinaus** (eigene
     Spec + eigenes Ticket, EmuTOS kennt kein EdDI).
* **Aufloesung:** auf eine kleine beschraenken — Doom zeichnet 320x200;
  ausgegeben wird klein (ggf. ganzzahlig skaliert, mittig), nicht der volle
  Schirm.
  **Praezisiert 2026-10-02:** auf der Pi (1920x1080) wird **mit
  4:3-Korrektur** vergroessert, Stufen einstellbar per Startparameter:
  x2 = 640x480, x3 = 960x720, x4 = 1280x960 (waagerecht ganzzahlig,
  senkrecht x1,2 durch Zeilenwiederholung), mittig. Vergroessert wird **nach**
  `vr_trnfm` im Geraeteformat (Pixel verdoppeln; nur bei gepackten
  Truecolor-Formaten, auf dem TT wird nicht vergroessert). Bildrate ist
  zweitrangig (Anwender: „selbst 30 fps sind egal, die VDI wird spaeter
  optimiert"); [GUI07](GUI07.md) bleibt nach dem Termin. Die Standardstufe
  wird nach dem ersten Pi-Lauf festgelegt.
* **TT:** Compiler `m68k-atari-mint-gcc` 7.5.0 in `/opt/cross-mint`; eine
  m68k-gemlib liegt in `sdk/gemlib/gemlib-m68k.a`. FPU (68881/68882) ggf.
  als Bauvariante; Doom rechnet im Kern Festkomma. Erster TT-Lauf in Hatari
  (`~/Downloads/hatari-2.6.1_macos/`, TT 256 Farben; eine Nova emuliert Hatari
  nicht), der echte TT mit Nova durch den Anwender.
* **Reihenfolge:** zuerst NewTOS mit dem `vr_trnfm`-Weg (geht ohne
  Kerneleingriff), dann EdDI-Spec → NewTOS-`vq_scrninfo` → EdDI-Weg in Doom,
  dann der m68k-Bau.

## Entscheidung 2026-10-04 — TT: Machbarkeit belegt, keine weitere Optimierung

Anwender am echten TT (TT-Low): ca. 3–4 fps, „unspielbar“; spielbar waere
erst ab ca. 20 fps — das Fuenffache, auf einem 68030/32 MHz bei 320x200 auch
mit Assembler-Renderer nicht erreichbar (Profil:
`measurements/measurement-2026-10-04-doom-m68k-profile/`, nach Tabellen-c2p
`measurements/measurement-2026-10-04-doom-c2p/`). Der TT-Bau bleibt als
**Machbarkeitsbeweis** (ein Quelltext, konform: EdDI, `kbdvec`, 4:3,
Tabellen-c2p); keine weitere TT-Optimierung geplant. Schwerpunkt Pi.

## 1. Warum es machbar ist — der Weg ist nicht nur vorhanden, sondern gemessen

Doom zeichnet in einen **eigenen 8-Bit-Palettenpuffer** (320x200) und schiebt
ihn je Bild auf den Schirm. Konform heisst das: **`vro_cpyfm` mit einem
Speicher-MFDB**, Palette ueber `vs_color`.

Genau diese Gruppe ist im September gegen echte TT- und Falcon-Dumps geprueft
worden: **61 von 62 Faellen wortgleich** (R02–R05,
`measurements/measurement-2026-09-21-r05/`, Falltabelle `tosdump/cases/vdi-raster-screen.tsv`).
Der Bildweg ist also belegt, nicht erhofft.

**Der Doppelpuffer ohne `Physbase`** ist die eine echte Entwurfsfrage: zweites
MFDB im Speicher, und `vro_cpyfm` schreibt es in einem Rutsch. Das ist
dieselbe Operation, die die Ball-Demo mit ihren 41 Aufrufen ohnehin misst.

## 2. Doom braucht SDL nicht

Die Originalfassung trennt die Plattform sauber ab — Video, Ton, Eingabe und
Netz liegen in eigenen Dateien, und jede Portierung ersetzt genau die. Wir
haben Framebuffer ueber VDI, Tastatur, und den 200-Hz-Takt
(`tos-api/src/bios/tick.rs`). Ton und Netz koennen beim ersten Anlauf
stillgelegt werden.

**PORT06 ist damit kein Folgeprojekt von SDL01, sondern ein eigener, moeglicherweise
kuerzerer Weg.**

## 3. Speicher — die Entwurfsentscheidung, die vor der ersten Zeile faellt

**Unsere TPA ist 4 MiB je Prozess** (`tos-api/src/bdos/process/arena.rs:27`).
Doom will fuer seine Zonenverwaltung mehr.

**Richtung des Anwenders (2026-09-28): vorrendern und im TT-RAM ablegen.**
Dort liegen ~942 MB brach (gegen 10 MB ST-RAM), und der Grund, aus dem EmuTOS
ST-RAM bevorzugt, ist Floppy-/ACSI-DMA — den gibt es bei uns nicht
([FS04](FS04.md) hat das belegt). Also: Texturen und Sprites **einmal** in die
Bildschirmform bringen, im TT-RAM halten, und der Bildaufbau wird eine Kopie
statt einer Umrechnung je Bild.

Der Weg dorthin ist `Malloc`/`Mxalloc` mit TT-RAM-Praeferenz; dass ein solcher
Block in den EL0-Raum der Anwendung eingeblendet werden muss, ist bei uns
bekannt und gebaut ([[el0-app-memory-must-be-mapped]]).

**Berichtigung 2026-10-17 ([MEM02](MEM02.md)):** Die Voraussetzung dieses
Abschnitts galt, entgegen der Lesart von MEM02 („`MX_PREFTTRAM` liefert
ST-RAM“, „10-MB-Wand“). `Mxalloc` hat die TT-RAM-Praeferenz schon damals aus
dem TT-Pool bedient; der ST-Pool blieb unberuehrt. Was die Ball-Demo am
2026-09-28 als „ST“ las, war die ADR-021-Fenster-VA ab 4 MiB, numerisch unter
`phystop`. Seit ARCH03 Stufe 1 (`a9279fe9`, VA = PA) ist der zurueckgegebene
Wert der Poolblock selbst
(`docs/analysis/mem02-mxalloc-pools-2026-10-17.md` §1–2,
`measurements/measurement-2026-10-17-mem02/RUNINFO.TXT`). Hosttests aller
vier Modi und die Sonde `MXPROBE` halten das fest.

## 3a. Der Farbweg — entschieden 2026-09-28, mit Begruendung

**Die Engine bleibt 8-Bit-indiziert. Umgerechnet wird einmal am Ende, beim
Bildaufbau.**

### Warum nicht der Bildschirm auf 8 Bit

Waere kein Schalter, sondern zwei Baustellen: der Framebuffer wird per Mailbox
mit **Tiefe 32** angefordert (`kernel/src/drivers/bcm2711/framebuffer.rs:233`),
den Paletten-Tag benutzen wir nirgends; und die VDI meldet **`PLANES = 16`**
(`kernel/src/vdi_backend.rs:43`), das Raster-Backend ist auf 32 Bit gebaut
(V11/V12 legten das `OVERLAY_BIT` ins X-Byte des Pixels). Daraus einen
**Laufzeit**-Modus zu machen ist ein Umbau. Konform waere es ohnehin ein
Aufloesungswechsel ueber eine Modusliste — das ist [X02](X02.md), **nach** dem
Termin.

### Warum nicht die Engine auf 32 Bit umrechnen

Mehr Arbeit **und** langsamer, und es verliert, was im 8-Bit-Entwurf umsonst
ist:

* **Das Lichtmodell ist der Palettenindex.** Entfernungsabdunklung und
  Helligkeitsstufen laufen ueber Umsetztabellen Index → Index. In Truecolor
  muesste man je Pixel rechnen statt nachzuschlagen. Dasselbe fuer die
  Spielerfarben und den Flimmereffekt der Spektren, der Nachbarpixel durch
  genau so eine Tabelle liest.
* **Paletteneffekte kosten in 8 Bit nichts** (Trefferblitz, Strahlenschutz,
  Einsammeln): das Bild ist gezeichnet, es wechselt nur die Palette. In
  Truecolor muesste jedes Pixel angefasst oder das Bild neu gezeichnet werden.
* **Der Renderer wuerde langsamer.** Die inneren Schleifen schreiben **ein
  Byte** je Pixel; in 32 Bit waeren es vier. Diese Schleifen sind
  speichergebunden — man vervierfacht den Verkehr genau dort, wo die Zeit
  verbraucht wird, und zwar auch fuer Pixel, die im selben Bild spaeter wieder
  ueberschrieben werden.

### Der gewaehlte Weg

Ein **einziger Durchlauf** ueber das fertige 320x200-Bild, durch eine Tabelle
mit 256 Eintraegen, in einen **Truecolor-Speicher-MFDB**; der geht per
`vro_cpyfm` auf den Schirm. Jedes Pixel genau **einmal**, gleich wie oft der
Renderer es vorher angefasst hat — ueberschlagen unter einer Millisekunde auf
einem 1,5-GHz-A72; die Kopie selbst sind 256 KB.

**Dooms Palette wird dabei schlicht zu unserer Umsetztabelle** — die
Paletteneffekte bleiben also kostenlos.

Das ist **vollstaendig konform**: die Anwendung rechnet ihre eigenen Pixel aus,
was ihr freisteht, und nach aussen benutzt sie nur `vro_cpyfm`. Kein
`Physbase`, kein Framebuffer-Umbau, **kein Kerneleingriff**. Es ist ausserdem
die **kleinste Aenderung am fremden Quelltext**, was
[[port-target-app-unmodified]] verlangt: die Umrechnung sitzt genau dort, wo
Doom ohnehin sein fertiges Bild hinausschiebt.

**Nebeneffekt:** derselbe Port laeuft auf einem TT mit Nova — dort in echtem
8 Bit ueber die Karte, ohne eine Zeile anders. Und kaeme spaeter ueber X02 ein
echter 256-Farben-Modus, faellt die Umrechnung weg, ohne dass am Port etwas zu
aendern waere.

### Gemessen 2026-09-28 durch [PORT07](PORT07.md) — zwei Praezisierungen

Die konforme Ball-Demo hat den Weg gefahren, den dieser Abschnitt entwirft.
Damit ist er **belegt statt angenommen** — und zwei Zahlen sind jetzt bekannt:

1. **„Truecolor" heisst hier 15 Bit (5-5-5), nicht 24.** Das VDI-Gerätewort
   ist der Falcon-Pixel. Gemessen: von 159 Farben im nicht-konformen Bild sind
   158 nicht in 5 Bit darstellbar, von 116 im konformen **keine einzige**. Die
   Umsetztabelle fuer Dooms Palette rechnet also auf 5-5-5, nicht auf 8-8-8.
2. **Der Preis je Bild ist die Kopie, nicht die Umrechnung.** 320x200 =
   64 000 px x **80,49 ns** (QEMU, nicht Pi) = **5,15 ms allein im `vro_cpyfm`** (18 % eines
   35-Hz-Taktes, 31 % eines 60-Hz-Bildes). Der Umrechnungsdurchlauf, den dieser
   Abschnitt mit „unter einer Millisekunde" veranschlagt hat, ist der
   **billige** Teil und verschwindet im Staging.

**Daraus folgt nichts gegen den Entwurf** — 5,15 ms sind tragbar —, aber es
verschiebt, wo sich Optimierung lohnt: nicht an Dooms Seite, sondern an
unserer. [GUI07](GUI07.md) hat die zwei konkreten Stellen.

## 4. Vor dem Anfangen zu klaeren

* **Lizenz.** Die Quelltextfreigabe von 1997 stand unter einer eigenen,
  **nicht-kommerziellen** Lizenz; die spaeter unter GPL gestellte Fassung ist
  eine andere und die, auf der praktisch alle Portierungen aufbauen. Fuer ein
  Projekt, das sich an EmuTOS' Lizenz anlehnt, ist das keine Formalie —
  **welche Fassung wir nehmen, gehoert entschieden, bevor Quelltext in den Baum
  kommt.** Dazu: Doom braucht eine WAD-Datei; die Shareware-WAD darf
  weitergegeben werden, die Vollversionen nicht.
* **Paletten-Pfad.** Doom ist 8-Bit-indiziert, unsere VDI geht Richtung
  Truecolor ([[vdi-truecolor-workstation]]). Wie ein indizierter Puffer samt
  Palette auf den Schirm kommt, ist **die** Stelle, an der die Arbeit steckt —
  nicht in der Spiellogik.
* **Eingabe.** Tastatur reicht fuer den ersten Anlauf; Maus ueber die AES ist
  vorhanden.

## Abnahme

Doom laeuft, spielbar, mit Tastatur — und ein `/usr/bin/grep` ueber die
Portierung findet **kein** `Physbase`, `Logbase` oder `Setscreen`. Bildrate
gemessen und festgehalten: sie ist der ehrlichste Durchsatztest, den wir
vorfuehren koennen, und beantwortet die Frage des OFAM-Veranstalters
(*wie schnell laeuft das?*) besser als jede Messreihe.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.
