# PORT02 — Welche Anwendungen koennen wir bis zum Termin portieren?

Status: PARKED (2026-11-11) | Priority: P2 | Reason: legacy, evidence unverified; receipt PORT02-STATUS-DONE.MD exists | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 20, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: 526ad8ac0 (docs(task): close six open tickets per the open-list review) added the receipt TASK/PORT02-STATUS-DONE.MD itself, the report of this research or measurement ticket (16 files), bookkeeping or bulk commit, not accepted as evidence

Line 3 before the migration:
Status: **ERLEDIGT** 2026-10-17 ([Quittung](PORT02-STATUS-DONE.MD)) · Prioritaet: **P2** · Typ: Bewertung / Portierbarkeit — Anwenderwunsch 2026-09-25

Abhaengigkeiten: [PORT01](PORT01.md) (Kommandozeile erreicht die Anwendung)

## Warum

Ohne Anwendungen laesst sich am 9./10. Oktober wenig zeigen. Heute laeuft
**QED**; **TeraDesk** ist portiert. Was sonst schon im Baum liegt, ist
ungeportet und unbewertet.

## Was da ist (Bestandsaufnahme, 2026-09-25)

| Ort | Was |
|---|---|
| `Applications/z-tools/zview` | Bildbetrachter |
| `Applications/z-tools/ztask` | Task-/Programmleiste |
| `Applications/z-tools/zweather` | Wetteranzeige |
| `Applications/z-tools/zcodeclib` | Codec-Bibliothek (Unterbau von zView) |
| `Applications/thing-neo-codeSVN-r62` | Thing — alternativer Desktop |
| `sdk/apps/highwire` | HighWire — Browser |

## Lead-Entscheidung 2026-09-25: **zView zuerst**

Der Anwender hat gewaehlt. Ein Bildbetrachter wirkt auf einer Vorfuehrung
sofort und braucht kein Netzwerk. Die uebrigen Kandidaten bleiben in der
Liste, werden aber **nach** zView bewertet.

**Was `Applications/z-tools/prerequisites.txt` selbst verlangt** (nicht
geschaetzt, dort nachgelesen):

```
ldg (latest SVN, mandatory)     windom 1.21.3       freetype 2.4.6
gemlib (CVS)                    mintlib (CVS)       libxml 2.9.0
libshare 0.1.1                  libmem              pml 2.03
xpdf 3.03                       zlib 1.2.8          transprt.h (STiK/STinG)
```

**Die Frage, die zuerst beantwortet sein muss:** `ldg` ist als *mandatory*
gefuehrt — zView laedt seine Bilddecoder als LDG-Module nach, und
nachladbare Module sind bei uns [MOD01](MOD01.md), ungebaut. Lassen sich die
Decoder **statisch** dazubinden? Davon haengt ab, ob zView ein Kandidat fuer
Oktober ist oder ein Projekt danach.

Zwei Spuren aus der README des Projekts, die dabei zaehlen: ein
**`newldg`-Zweig**, der genau die LDG-Probleme angeht, und eine gepflegtere
Fassung unter `th-otto/zview`. Der Autor nennt ausserdem **ImgView** (Lonny
Pursell) als Alternative, die zView „in manchen Bereichen abloest" — pruefen,
ob die weniger verlangt.

**LDG-Quellen gibt es** (Anwender 2026-09-25) — sie liegen nur noch nicht im
Baum (`references/` hat heute `emutos`, `freemint`, `circle`). Damit ist LDG
kein fremder Klotz, sondern portierbar wie alles andere, und die Frage wird
feiner:

* **Jetzt machbar:** LDG' *Schnittstelle* portieren und die Decoder
  **statisch** dazubinden — der Lader loest die Module dann aus einer
  eingebauten Tabelle auf, statt Dateien nachzuladen. Damit laeuft zView,
  ohne dass MOD01 fertig sein muss.
* **Spaeter:** echtes Nachladen, sobald [MOD01](MOD01.md) steht — dann
  greift dieselbe Schnittstelle, und aus der Tabelle wird ein Dateizugriff.

Zu klaeren ist, ob zViews Decoder diese Trennung mitmachen oder ob sie
LDG-Eigenheiten benutzen, die nur beim echten Nachladen existieren. **Das
entscheidet, ob zView bis Oktober laeuft** — nicht die Frage, ob wir LDG
haben.

**Sie liegen bereits da: `Applications/ldg-2` (LDG 2.35)** — und beim
Hineinsehen zeigt sich, dass es die **SDK-Fassung** ist, nicht der Quelltext
des Laders:

* **7 C-Dateien, null Assembler** — und alle sieben sind Demos bzw.
  Beispielbibliotheken (`demo/client/client.c`, `demo/libs/popup.c`,
  `contrib/fileutil/fileutil.c` …).
* Die Umsetzung kommt fertig: `lib/gcc/libldg.a` (m68k, fuer uns wertlos)
  und `68K/auto/ldg.prg` als Binaerdatei.
* **Was brauchbar ist, ist die Schnittstelle:** `include/ldg.h` mit
  `ldg_open`/`ldg_close`/`ldg_find`/`ldg_init`, den `LDG`-Strukturen,
  Fehlercodes und Flags — dazu eine Handbuchseite je Funktion unter
  `man/cat3/`.

**Das macht den statischen Weg kleiner als gedacht:** LDGs Lader muss gar
nicht portiert werden. Zu bauen waeren rund sechs dokumentierte Funktionen,
die statt aus Dateien aus einer eingebauten Tabelle aufloesen.

**Projektseite: https://ldg.sourceforge.net** (vom Anwender). Was sie sagt:

* **2.35 ist der aktuelle Stand** (Juni 2015) — unser Paket ist die letzte
  Fassung, nicht eine alte.
* **Lizenz LGPL** seit 2.32. Dagegen linken ist erlaubt; Aenderungen an LDG
  selbst muessen offen sein, was bei uns ohnehin zutrifft.
* Sie unterscheidet ein **Hauptpaket** („system components, documentation,
  development libraries") von einem **DEV-Paket** („headers and compiled
  libraries only"). Unseres hat System, Doku *und* Bibliotheken, ist also das
  Hauptpaket — und traegt trotzdem keinen Laderquelltext. Der liegt entweder
  als eigenes Paket im Dateibereich des Projekts, oder er ist trotz LGPL nicht
  veroeffentlicht. **Im Dateibereich nachsehen, bevor das eine oder das andere
  behauptet wird.**

Fuer den statischen Weg ist das ohnehin zweitrangig — dort braucht es die
Schnittstelle, nicht den Lader. Es entscheidet aber darueber, ob spaeter
echtes Nachladen (MOD01) auf LDGs eigenem Code aufsetzen kann oder auf einem
Nachbau.

Zwei Dinge, die dabei auffallen und zu pruefen sind:

* `LDG_STDCALL` ist dokumentiert als *„a2/d2 are not scratch register"* — die
  Modulschnittstelle traegt **m68k-Registerkonventionen**. Fuer statisch
  gebundenen AArch64-Code gegenstandslos, aber es zeigt, dass die
  Schnittstelle Annahmen mitbringt; welche noch, ist Teil der Bewertung.
* Die Dateinamen im Paket enthalten **literale Backslashes**
  (`ldg-2.35\include\ldg.h` ist *ein* Dateiname) — das Archiv wurde ohne
  Pfadumsetzung entpackt. Jedes Bauskript stolpert darueber; entweder
  vorher umsetzen oder ausdruecklich behandeln.

**windom — Quellen und ein Versionsproblem.** Der Anwender nennt
`https://github.com/ggnkua/Atari_ST_Sources/tree/master/C/Arnaud Bercegeay/WINDOM`.
Dort liegt **WinDom 1.1** (September 2001) bzw. `WNDM0901` (0.9.01), als
Archive plus Beispiele; Lizenz laut README *„FreeWare … Please use it as a GPL
licence"*, Autoren Dominique Bereziat und Arnaud Bercegeay, offizielle Seite
`http://windom.free.fr`.

zViews Voraussetzungsliste verlangt **1.21.3** — und die liegt inzwischen im
Baum: **`Applications/windom-1.21.3`**, die genau geforderte Fassung. Die
GitHub-Fundstelle oben (1.1 von 2001) wird damit nicht gebraucht; sie bleibt
hier nur als Hinweis stehen, falls jemand spaeter eine aeltere Fassung sucht.

**Was der Bestand zeigt** (nachgesehen, nicht geschaetzt):

* **199 C-Dateien in `src/`, null Assembler.** Reiner C-Quelltext.
* **LGPL 2.1** (`COPYRIGHT`).
* Eingebunden werden `gem.h` (gemlib), `osbind.h`/`tos.h` (mintlib),
  `mint/cookie.h`, `mint/sysvars.h`, `osheader.h`, `atari.h` und
  Standard-C — **also genau die zwei Bibliotheken, die fuer QED bereits
  portiert sind**, plus Systemvariablen und OS-Kopf, die wir haben.

Damit faellt windom aus der Rubrik „Unbekannte" heraus: es ist Umfang
(199 Dateien uebersetzen-und-beheben), nicht Risiko. Der Aufwand ist
trotzdem getrennt auszuweisen.

Zum Aufwand von windom: ein GEM-Rahmenwerk ueber gemlib, das wir
bisher nicht gebaut haben. Aufwand dafuer getrennt ausweisen.

## Das Materialarchiv: `/Volumes/Daten/tho-otto-backu`

Ein 42-GB-Spiegel von `tho-otto.de` liegt lokal (Anwender 2026-09-25), mit
`www.tho-otto.de/download/` als Paketablage. Nachgesehen, was von zViews
Liste darin steht:

| gesucht | gefunden |
|---|---|
| `pml 2.03` | **`pml-2.03.tar.xz`** — Quelltext, genau die geforderte Fassung |
| `windom 1.21.3` | **`windom-2.0.1.tar.xz`** — *neuer* als gefordert, siehe unten |
| `cflib` | datierte Schnappschuesse bis **`cflib-21-20240430`** |
| `libshare`, `libmem` | **nicht** darin |

Jedes Paket bringt ein `*-build.sh` mit, das zeigt, wie es fuer Atari
konfiguriert und gebaut wird — fuer unsere Portierung wertvoller als der
Quelltext allein.

**Die Frage, die daraus faellt:** zViews Liste nennt `windom 1.21.3` *„plus
fix for gcc4 and memory protection"*; im Archiv liegt **2.0.1**. Entweder
enthaelt 2.0.1 diese Korrekturen laengst — dann ist es die bessere Wahl — oder
es hat sich die Schnittstelle geaendert und zView baut nicht mehr dagegen. Am
Quelltext klaeren, nicht annehmen. `Applications/windom-1.21.3` (die exakt
geforderte Fassung) liegt als Rueckfallebene daneben.

`libshare` und `libmem` fehlen noch; `libmem` koennte laut zViews eigener
Liste („same source as ldg") in `Applications/ldg-2` mitstecken — dort zuerst
nachsehen, bevor gesucht wird.

## Auftrag

Je Kandidat **eine belegte Einschaetzung**, keine Vermutung:

1. **Was braucht er?** Welche Bibliotheken (gemlib, cflib, LDG, eigene), und
   welche davon hat `sdk/` schon. Fehlende benennen, nicht ueberspringen.
2. **Wie viel m68k-Assembler?** Zaehlen, nicht schaetzen. Das ist die
   einzige Stelle, an der die Projektregel „die portierte Anwendung bleibt
   unveraendert" eine Ausnahme kennt.
3. **Welche Systemfunktionen braucht er, die uns fehlen?** Netzwerk faellt
   auf, aber nicht nur — gegen `docs/api/` und die offenen Tickets pruefen.
4. **Lizenz** — was erlaubt sie, was verlangt sie.
5. **Vorfuehrwert** und geschaetzter Aufwand, beides in einem Satz, und
   **getrennt** voneinander: das Beeindruckendste ist nicht zwingend das
   Naechstliegende.

Am Ende eine **Reihenfolge** mit Begruendung — was zuerst, was danach, was
realistisch nicht bis Oktober.

## Wie bewertet wird

Der Massstab ist unsere eigene Erfahrung mit QED, nicht ein Gefuehl: dort hat
sich gezeigt, dass eine Portierung im Kern ein **Neuausrichten der duennen
Trap-/Arch-Grenze plus Uebersetzen-und-Beheben** ist, keine Neuschreibung.
Ein Kandidat, der das nicht zulaesst — weil er zu tief auf Hardware sitzt
oder zu viel Assembler traegt — ist ein schlechter Kandidat, egal wie schoen
er aussieht.

Keine Codeaenderung in diesem Ticket. Ergebnis ist ein Dokument.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.


## Vorschlaege des Veranstalters, bewertet am vorhandenen Unterbau (2026-09-28)

Der Veranstalter der OFAM — Freund des Anwenders und Programmierer — wurde
gefragt, was sich zum Vorfuehren eignet, und nannte: Doom, einen Videoplayer,
HighWire/NetSurf, einen SDL-Port, „irgendein Spiel, wo man sieht wie schnell
das laeuft", den FireBee-m68k-Kern, CastAway, Atari800.

**Gepruefter Unterbau, nicht vermutet:** 200-Hz-Takt ✓
(`tos-api/src/bios/tick.rs`), AES-Ereignisse ✓ (31 `evnt_*`), Tastatur ✓,
Maus inkl. Rad ✓ (GUI04), Framebuffer/VDI ✓ (alle ~70 Opcodes).
**Ton ✗** (`xbios/system.rs:145-161`, ehrliche No-ops „no PSG").
**Netz ✗** (NET01–NET06 alle offen).

| Vorschlag | Bewertung |
|---|---|
| **Videoplayer** | **Faellt aus.** Nicht wegen des Tons: die **Leseseite** der Karte ist gemessen **~730 KB/s** (`QED.NAP`, 502 KB in 687 ms, `measurements/measurement-2026-09-25-fs04/raw/pi05-…`). Das reicht fuer Video nicht. Ursache ist [HW04](HW04.md)/[HW05](HW05.md) — 1-Bit-Bus, Einzelblock, kein DMA |
| **HighWire / NetSurf** | **Blockiert, aber nicht woran der Veranstalter dachte.** Er vermutet NVDI; unsere VDI ist vollstaendig. Der Blocker ist der **fehlende Netzwerkstapel** (NET01–06). HighWire liegt bereits unter `sdk/apps/highwire` und wartet genau darauf |
| **SDL-Port** | **Der beste Vorschlag der Liste** — aber als *Unterbau*, nicht als Anwendung. Eigenes Ticket [SDL01](SDL01.md) |
| **Doom, Spiele** | **Korrektur 2026-09-28:** Doom braucht SDL **nicht** — die Originalfassung trennt die Plattform in eigene Dateien, und wir haben Framebuffer, Tastatur und Takt. Eigener, moeglicherweise kuerzerer Weg: [PORT06](PORT06.md). Andere Spiele weiterhin ueber SDL01 |
| **CastAway, Atari800** | **Die passendste Kategorie fuer das, was wir heute haben:** Framebuffer, Eingabe, Timer — alles da; ohne Ton laufen sie still. Und sie beantworten die Frage des Veranstalters (*wie schnell laeuft das?*) ehrlicher als jede Messreihe |
| **FireBee-m68k-Kern** | **Liegt auf unserer eigenen Linie, kein Seitenprojekt.** ADR-018 reserviert `.PRG`/`.TOS`/`.TTP` ausdruecklich fuer einen spaeteren m68k-Emulator — deshalb heissen unsere Endungen `.NAP`/`.NCP` |

**Einordnung des Leads zur OFAM (9./10. Oktober):** keiner dieser Kandidaten
ist in der verbleibenden Zeit serioes zu machen, **zView eingeschlossen** — der
Anwender hat ihn am 2026-09-28 selbst als „wichtiges Werkzeug, fuer eine
Vorfuehrung aber nicht unbedingt noetig" eingeordnet.

**Empfohlene Reihenfolge nach dem Termin:**
[SND01](SND01.md) (Ton) → [SDL01](SDL01.md) (Backend) → Vorfuehrmaterial kommt
dann von selbst; parallel [HW04](HW04.md)/[HW05](HW05.md), weil ohne Durchsatz
jedes davon zaeh aussieht.
