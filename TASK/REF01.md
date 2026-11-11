# REF01 — Die Referenzkonfiguration festschreiben, statt sie je Extraktion neu zu raten

Status: DONE (2026-11-11) | Priority: P2 | Commit: a0eeebb3c | Receipt: REF01-STATUS-DONE.MD

Migrated (owner batch 20, 2026-11-11, to DONE), evidence: receipt TASK/REF01-STATUS-DONE.MD exists, commit a0eeebb3c (docs(api): pin the EmuTOS reference configuration as a generated table), VERIFIED: a0eeebb3c (docs(api): pin the EmuTOS reference configuration as a gener) added the receipt TASK/REF01-STATUS-DONE.MD itself, the report of this research or measurement ticket (6 files), legacy line: ERLEDIGT (2026-09-24)

Line 3 before the migration:
Status: **ERLEDIGT (2026-09-24)** · Prioritaet: **P2** · Typ: Werkzeugkette / Quellenanalyse

Abhaengigkeiten: —

## Warum jetzt

Jede EmuTOS-Extraktion prueft die `CONF_WITH_*`-Schalter neu, und zweimal in
einer Woche ging das schief: bei DESK05 und bei DESK15 stand im
`#ifdef TARGET_192`-Block eine `0`, die fuer unser Profil gar nicht gilt.
Beide Male hat es der Entwickler gemerkt, beide Male hat es Zeit gekostet, und
beim naechsten Mal merkt es vielleicht keiner.

Die Projektregel „pruefe die CONF-Gatter" bleibt richtig — aber sie soll gegen
eine **festgeschriebene Tabelle** gehen statt gegen `config.h`s Verschachtelung.

## Der Befund, der das moeglich macht

Die Referenzkonfiguration ist exakt reproduzierbar:

* `DEF` ist leer (`references/emutos/Makefile:132`),
* `make 512` setzt genau **ein** Flag: `override DEF += -DTARGET_512`
  (`:558-563`),
* eine `localconf.h` liegt nicht im Baum (`:167-172` pruefen).

Damit liefert der Praeprozessor die ganze Wahrheit:

```
m68k-atari-mint-gcc -E -dM -DTARGET_512 -Iinclude -I. <leere .c mit #include "config.h">
```

Probe in diesem Baum: **106** `CONF_*`-Makros, darunter
`CONF_WITH_DESKTOP_CONFIG=1`, `CONF_WITH_WINDOW_ICONS=1`,
`CONF_WITH_DESKTOP_SHORTCUTS=1`, `CONF_WITH_READ_INF=1`,
`CONF_WITH_VDI_16BIT=1`.

## Auftrag

1. **Skript** `tools/emutos-config.sh` — erzeugt die Tabelle aus dem gepinnten
   Baum, ohne ihn zu veraendern, und schreibt sie nach `docs/api/_emutos-config.md`.
   Scratch nur im Job-Verzeichnis. Das Skript nennt im Kopf die drei Zeilen
   oben als Begruendung, warum `-DTARGET_512` allein genuegt.
2. **Tabelle** `docs/api/_emutos-config.md` — alle `CONF_*` mit Wert,
   alphabetisch, dazu der Commit-Hash des gepinnten EmuTOS-Baums und das
   Erzeugungsdatum, damit man sieht, wogegen sie gilt.
3. **Die Fallen benennen**, die die Tabelle ausdruecklich aufloest: der
   `TARGET_192`-Block, und jeder weitere `#ifdef TARGET_*`-Block, den es gibt.
   Pruefen und schreiben, ob es fuer `TARGET_512` ueberhaupt einen eigenen
   Block gibt (ACC01 sagt nein — gegenpruefen, nicht uebernehmen).
4. **Die Maschinenfrage beantworten.** Das 512K-Abbild ist laut Makefile-
   Kommentar fuer TT *oder* Falcon. Welche Schalter haengen dann an einer
   **Laufzeit**-Erkennung statt am Uebersetzen, und was heisst das fuer eine
   Aussage wie „im Falcon-Profil ist X an"? Das ist die Frage, die die Tabelle
   allein nicht beantwortet, und sie gehoert in ihren Kopf.
5. **Eine Zeile in `docs/SOURCE-ANALYSIS-PROTOCOL.md`**, die kuenftige
   Extraktionen auf die Tabelle verweist statt auf `config.h`.

## Abnahme

Das Skript zweimal laufen lassen ergibt dieselbe Datei. Fuenf Stichproben aus
frueheren Tickets gegen die Tabelle halten (DESK05 `DESKTOP_CONFIG`, DESK15
`WINDOW_ICONS` und `DESKTOP_SHORTCUTS`, DESK14 `READ_INF`, ACC01 — die
Lebenszyklusfunktionen haengen an **keinem** Schalter) und bestaetigen, dass
die Tabelle dasselbe sagt wie die damalige Handpruefung.

Keine Produktcodeaenderung.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

---

## Abschluss

**2026-09-24.** Deliverables: `tools/emutos-config.sh` (neu, ausfuehrbar),
`docs/api/_emutos-config.md` (neu, generiert — nicht von Hand
nachbearbeitet), eine Zeile in `docs/SOURCE-ANALYSIS-PROTOCOL.md`
Schritt 3.7. Skript zweimal laufen lassen: byte-identische Datei
(`diff` leer). Keine Produktcodeaenderung, `references/emutos/`
unveraendert (nur `.DS_Store` untracked, wie vorher).

**Punkt 4, die Maschinenfrage (die wichtigste).** `make 512` baut **ein**
ROM fuer TT, Falcon und (unter Hatari) ST/STe — keinen eigenen Bau je
Maschine. Der Mechanismus, Ende-zu-Ende gelesen, nicht angenommen:
`include/has.h:26-45` macht aus jedem `HAS_X` entweder eine feste `0`
(wenn `CONF_WITH_X` aus ist) oder eine Referenz auf eine **Laufzeit**-
Variable `has_x` (wenn `CONF_WITH_X` an ist). Im TARGET_512-Profil sind
`CONF_WITH_STE_SHIFTER`/`_TT_SHIFTER`/`_VIDEL`/`_VME` alle `1` — die
Erkennungs-Routinen sind also mitkompiliert, fuer alle vier Chips
gleichzeitig. Welcher Zweig beim konkreten Boot **tatsaechlich**
greift, entscheidet `detect_video()` (`bios/machine.c:101-162`) per
Bus-Error-Sondierung echter Hardwareregister (STe-Register
`0xffff820d`, TT-Palette `0xffff8400`, Falcon-`HHT` `0xffff8282` ueber
`check_read_byte`), setzt `has_ste_shifter`/`has_tt_shifter`/
`has_videl`; `detect_vme()` ebenso fuer `has_vme`
(`bios/machine.c:200-221`). `fill_cookie_jar()`
(`bios/machine.c:665-694`) leitet daraus `cookie_mch`
(`MCH_ST`/`_STE`/`_MSTE`/`_TT`/`_FALCON`, `include/cookie.h:49-56`) und
`cookie_vdo` ab — beides unbedingt beim Boot, vor jedem AES/Desktop-Code
(`bios/bios.c:246-248`, `:279-280`). **Folge fuer die Tabelle:** eine
Zeile mit Wert `1` heisst "der Code ist mitkompiliert, laeuft auf allen
vieren" — sie heisst **nicht** "auf dem Falcon ist X an"; das ist eine
Laufzeitaussage ueber `cookie_mch`/`has_*`, die die Tabelle nicht treffen
kann. Dieser Absatz steht jetzt im Kopf von `docs/api/_emutos-config.md`,
mitsamt der aus `include/has.h` **mechanisch** extrahierten Liste aller
18 derart laufzeitgesteuerten `CONF_WITH_*`. Gegengeprueft: die fuenf
Stichproben-Flags (unten) haengen an **keinem** dieser 18, und nur eine
einzige Stelle in `desk/`/`aes/` fragt ueberhaupt `cookie_mch`/`HAS_*`
ab (`aes/gemgsxif.c:161`) — die Maschinenfrage aendert an den fuenf
Stichproben nichts.

**Punkt 3, ACC01 gegengeprueft.** Die Behauptung haelt: **kein**
`#ifdef TARGET_512`-Block existiert in `config.h`, auf zwei Wegen
bestaetigt (nicht nur uebernommen) — (a) von den 7 gefundenen
`#ifdef TARGET_*`-Bloecken traegt keiner den Namen `TARGET_512`, (b)
der Literalstring `TARGET_512` kommt im **gesamten** gepinnten Baum
genau **einmal** vor: `references/emutos/Makefile:561`, die Zeile, die
das Makro definiert — niemals als Verbraucher in `config.h`.
`TARGET_512` faellt fuer **jedes** `CONF_*`-Makro auf den generischen
Default zurueck, ausnahmslos. Alle 7 vorhandenen `#ifdef TARGET_*`-
Bloecke sind gezaehlt und mit Zeilenbereich in der Tabelle: `TARGET_192`
(`config.h:210-394`, 57 abweichende Werte gegen TARGET_512 —
ACC01 selbst zitierte `210-401`, das war um 7 Zeilen zu weit; korrigiert
in der neuen Tabelle), `TARGET_256` (`:402-448`, 14), `TARGET_CART`
(`:469-554`, 26), `TARGET_FLOPPY` (`:559-563`, 2), `TARGET_AMIGA_ROM`
(`:568-570`, 48), `TARGET_AMIGA_FLOPPY` (`:575-577`, 50),
`TARGET_LISA_FLOPPY` (`:597-599`, 54) — letztere drei fremde
Maschinenfamilien (Amiga/Lisa), nicht Atari, der Vollstaendigkeit halber
mitgezaehlt. Die Abweichungslisten sind nicht durch Text-Scan der
Bloecke entstanden, sondern durch echten Praeprozessorlauf je Block plus
Diff gegen TARGET_512 — ein Text-Scan haette Kaskaden durch einen
nicht-`CONF_`-praefigierten Schalter uebersehen (`TARGET_192` kippt
`DETECT_NATIVE_FEATURES`, was zweitausend Zeilen entfernt
`CONF_WITH_ARANYM`s generischen Default aendert — beim Bau der Tabelle
tatsaechlich aufgetreten, kein theoretisches Risiko).

**Punkt 3, Nebenbefund.** Zwei `CONF_*`-Makros (`CONF_WITH_SCSI_DRIVER`,
`CONF_WITH_XHDI`) sind als `!CONF_WITH_EXTERNAL_DISK_DRIVER` definiert
(`config.h:1685`, `:1692`) — reine Textsubstitution (`-E -dM`) laesst da
ein rohes `!0` stehen statt `1`. Das Skript wertet jeden nicht-
literalen Rest arithmetisch aus (gleiche Semantik wie ein C-`#if`);
beide stehen jetzt korrekt als `1` in der Tabelle.

**Punkt 1/2, Stichprobe der Methode.** Der in REF01 vorgegebene
Praeprozessor-Befund (`-DTARGET_512 -Iinclude -I.` auf eine leere
Uebersetzungseinheit) liefert exakt **106** `CONF_*`-Makros, wie im
Ticket vorgerechnet — nachvollzogen, nicht nur uebernommen. Eine
Ergaenzung, die REF01 nicht auf dem Schirm hatte: `make 512`s echter
Compile-Aufruf traegt zusaetzlich `-DWITH_AES=1 -DWITH_CLI=1`
(`Makefile:267`) — geprueft, ob das die Tabelle veraendert: nein,
`config.h` liest `WITH_AES`/`WITH_CLI` nirgends (einzige Verbraucher
`bios/endrom.c:15,30` und `vdi/vdi_mouse.c:26,67`, Datei-Auswahl beim
Link, kein `CONF_*`-Makro). Die `-DTARGET_512`-allein-Begruendung haelt
also vollstaendig, mit dieser einen zusaetzlich geprueften Randbedingung.

**Fuenf Stichproben gegen die alten Tickets — alle halten:**

| Ticket | Flag | alte Handpruefung | Tabelle |
|---|---|---|---|
| DESK05 | `CONF_WITH_DESKTOP_CONFIG` | `1` (`config.h:1774-1776`) | `1` |
| DESK15 | `CONF_WITH_WINDOW_ICONS` | `1` (`config.h:1866-1868`, nicht der `TARGET_192`-Zweig `:322-324`) | `1` |
| DESK15 | `CONF_WITH_DESKTOP_SHORTCUTS` | `1` (`config.h:1782-1783` analog) | `1` |
| DESK14 | `CONF_WITH_READ_INF` | an (`app_read_inf` hinter `#if CONF_WITH_READ_INF`, `deskapp.c:1486`) | `1` (`config.h:1819-1820`) |
| ACC01 | `.ACC`-Lebenszyklus | an **keinem** Schalter | keine passende Zeile — bestaetigt |

DESK15s eigene Zeilenzitate (`:322-324` fuer den `TARGET_192`-Nullwert,
`:1866-1868` fuer den generischen Default) wurden gegen den Baum
nachgezaehlt und stimmen exakt. Keine Diskrepanz an einem der fuenf
frueheren Tickets gefunden — die Tabelle sagt an allen fuenf Stellen
dasselbe wie die damalige Handpruefung.

**Fuer ein neues Ticket:** keins. Die einzige Randnotiz ist die
7-Zeilen-Ungenauigkeit in ACC01s eigenem `TARGET_192`-Zitat
(`:210-401` statt `:210-394`) — ACC01 hatte bereits selbst mehrere
Off-by-N-Zitate in seinem eigenen Abschluss eingeraeumt; diese eine
zusaetzliche gehoert in dieselbe Kategorie und rechtfertigt kein eigenes
Ticket, ist hier nur der Vollstaendigkeit halber notiert.

## Nachtrag — zwei Korrekturen des Koordinators waehrend der Bearbeitung

**1. Skript-Robustheit (Koordinator-Fund).** `sh tools/emutos-config.sh`
sah nach Erfolg aus, war es nicht: `/bin/sh` ist auf dieser Maschine
bash selbst, das beim Aufruf als `sh` in den POSIX-Kompatibilitaetsmodus
wechselt (`<(...)` Prozess-Substitution dort ungueltig) — und der
resultierende Syntaxfehler tief im Skript endete mit **Exitcode 0**,
sodass „zweimal laufen lassen, `diff` leer" fuer den falschen Grund
bestand (kein Lauf hatte etwas geschrieben). Ein blosses
`[ -z "$BASH_VERSION" ]` haette das **nicht** gefangen (die Variable
bleibt gesetzt, sie nennt das Binary, nicht den Modus) — verifiziert,
nicht angenommen: `sh -c 'echo "$SHELLOPTS"'` zeigt auf dieser Maschine
`...:posix`, `bash -c` dasselbe nicht. Die neue Schutzklausel prueft
`$SHELLOPTS` auf `posix` **und** `$BASH_VERSION` auf leer, ganz am
Skriptanfang, vor jeder Seitenwirkung — geprueft unter `sh`, `zsh` und
`bash`: `sh`/`zsh` brechen jetzt mit Exitcode 1 und einer erklaerenden
Meldung ab, ohne die Ausgabedatei anzuruehren; `bash` laeuft
unveraendert durch.

**2. Die Maschinenfrage war zu breit formuliert (User-Korrektur).**
Der urspruengliche Kopfabsatz liess den Vorbehalt „kann keine
Maschinen-Aussage treffen" pauschal ueber die ganze Tabelle wirken. Der
Vorbehalt betrifft aber nur **Hardware**: `include/has.h`s eigene erste
Zeile sagt es selbst („BIOS `HAS_*` macros to determine if hardware is
available", `include/has.h:2`), und alle 18 laufzeitgesteuerten
`CONF_WITH_*` sind ausnahmslos Chips/Busse (mechanisch gegen die
Arbeitsmenge-Klassifikation gegengeprueft, Schritt 6c des Skripts —
Ueberschneidung **null**, jeden Lauf neu verifiziert, nicht nur
behauptet). Genau diese Hardware-Ebene bildet NewTOS laut
`docs/COMPATIBILITY.md:35-47` bewusst **nicht** nach — die Vertragsflaeche
ist der Trap/Verhaltens-Rand, nicht der Chip dahinter, und der Unterbau
ist ueberall die RPi4-HAL. Fuer alles, was diese Tabelle als
Grafik/AES/VDI klassifiziert, gilt der Vorbehalt deshalb **gar nicht** —
keiner der 28 Arbeitsmenge-Schalter ist laufzeitgesteuert.

## Nachtrag 2 — Klassifikation aller 106 Schalter (User-Auftrag)

Auf Zuruf des Nutzers (ueber den Koordinator, waehrend der Bearbeitung
von Nachtrag 1) wurde die flache alphabetische Tabelle durch eine
**Klassifikation** ersetzt, die genau eine Frage beantwortet: *„Was
haben wir bisher schon, was nicht — fuer Grafik/AES/VDI, die eigentlich
nachzubauende Flaeche?"* Hardware/Fremdmaschine (65) und GEMDOS/BIOS-
Oberflaeche/Dateisystem/Build/Debug (13) werden nur **benannt**, keine
Einzelpruefung. Die 28 Grafik/AES/VDI-Schalter (die „Arbeitsmenge")
sind einzeln gegen `kernel/`/`tos-api/` gegengeprueft: **21 vorhanden**
(mit `datei:zeile`), **7 Luecke** (mit EmuTOS-Fundstelle + Ticketstatus).
Die Klassifikationsdaten liegen als Tabelle im Skript selbst
(`CLASSIFICATION_DATA`), nicht ableitbar durch den Praeprozessor, und
werden jeden Lauf gegen die lebende 106-Namen-Liste gegengeprueft
(Schritt 6b — bricht mit Namensliste ab, wenn `config.h` und die
Klassifikation auseinanderlaufen). Wartungsanleitung fuer einen
kuenftigen Neu-Pin steht als Kommentar direkt ueber der Tabelle im
Skript.

**Gegenpruefung der Koordinator-Vermutungsliste.** Neun der 19
Kandidaten, die der Koordinator als moegliche Luecken nannte, sind
tatsaechlich **vorhanden** (Korrektur, nicht Bestaetigung):
`CONF_WITH_SEARCH`, `CONF_WITH_GDOS` (Fix unter V16 bereits verifiziert),
`CONF_WITH_COLOUR_ICONS`, `CONF_WITH_WINDOW_COLOURS`,
`CONF_WITH_EXTENDED_OBJECTS`, `CONF_WITH_NICELINES`,
`CONF_WITH_MENU_EXTENSION`, `CONF_WITH_GRAF_MOUSE_EXTENSION`,
`CONF_WITH_PCGEM`. Die restlichen sieben echten Luecken stehen unten.

Die vollstaendige Tabelle mit allen Fundstellen liegt in
`docs/api/_emutos-config.md`, Abschnitt „Classification". Keine
Produktcodeaenderung.

## Nachtrag 3 — zwei Fehlklassifikationen korrigiert, eine bestaetigt, ein echter Skript-Bug gefunden

Der Koordinator fand zwei falsche Luecken, beide durch denselben
Mechanismus: eine Namenssuche, die das Merkmal verfehlt, weil NewTOS es
anders nennt. **`CONF_WITH_BOTTOMTOTOP`** ist vorhanden — `Item::Cycle`
im `BTOPITEM`-Slot (`kernel/src/desk/menu.rs:108`, Dockommentar „Cycle
windows bottom-to-top" `:187-188`), Handler `do_cycle()`
(`kernel/src/desk/main.rs:1474`). **`CONF_WITH_DESKTOP_SHORTCUTS`** ist
vorhanden — DESK30 ist nicht „ticketiert", sondern **ERLEDIGT** und
heute committet (`f85f9c8`): `ins_shortcut` (`kernel/src/desk/ins.rs:1016`),
`remove_locate_shortcut` (`kernel/src/desk/dir.rs:1701`). Beide von
MISS auf HAVE umklassifiziert; eine neue Kopfsatz-Warnung in der
Tabelle sagt jetzt ausdruecklich: ein Zeilentreffer wird nach dem
**Verhalten** des Schalters klassifiziert, nie nach dem Namensgrep,
weil NewTOS bewusst keine `CONF_*`-Namen im eigenen Baum traegt.

**`CONF_WITH_ALT_DESKTOP_GRAPHICS` selbst gegengeprueft** (Koordinator-
Hinweis: „unsere Dialoge zentrieren ihre Titel"): `align_title()`
(`deskmain.c:1727-1747`) linksbuendig + Unterstrich bei
`CONF_WITH_ALT_DESKTOP_GRAPHICS`, zentriert ohne Unterstrich sonst.
NewTOS zentriert tatsaechlich — `kernel/src/desk/dir.rs:567-572`
(`(dw - title_w) / 2`), `kernel/src/desk/inf.rs:3403-3404`
(`(dw - 21 * CELL_W) / 2`), beide im eigenen Kommentar `deskmain.c`s
Zentrier-Zweig zitierend. Bleibt Luecke, jetzt mit Codebeleg statt
„nichts gefunden".

**Ein echter Skript-Bug beim Einbau der Korrekturen gefunden und
behoben:** `bash` 3.2 (das tatsaechliche `/bin/bash` dieser Maschine)
hat einen reproduzierbaren Parserfehler — ein in `$(...)` eingebettetes
Heredoc mit einer **ungeraden** Anzahl Apostrophe im Textkoerper macht
die Substitution fuer den Parser „nie geschlossen", *obwohl* das
Heredoc mit `<<'X'` als woertlich markiert ist. Minimal reproduziert
(ein Apostroph bricht, zwei nicht; ohne `$(...)`-Einbettung bricht auch
einer nicht), nicht geraten. Die 106-Zeilen-Klassifikationstabelle
schreibt jetzt direkt in eine Datei (`cat <<'X' > datei`) statt in eine
Variable via `$(cat <<'X' ...)"` — strukturelle Loesung statt
Apostroph-Zaehlen bei jeder kuenftigen Zeile. Kommentar dazu direkt im
Skript ueber der Tabelle.

**Neue Zaehlung:** Arbeitsmenge weiterhin 28, jetzt **23 vorhanden, 5
Luecke** (vorher 21/7). Determinismus mit zwei `bash`-Laeufen erneut
bestaetigt, `diff` leer.
