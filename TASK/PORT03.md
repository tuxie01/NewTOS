# PORT03 — cflib und QED sind sich ueber den Pfadtrenner uneinig

Status: PARKED (2026-11-11) | Priority: P2 | Reason: legacy, evidence unverified; receipt PORT03-STATUS-DONE.MD exists | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 19, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: no cited file of the ticket/receipt is touched by 38f9bd649 (fix(sdk): pin cflib to ce57817, where upstream fix), named by body

Line 3 before the migration:
Status: **ERLEDIGT** (2026-09-25) · Prioritaet: **P2** · Typ: Versionswechsel — Nebenbefund PORT01

Abhaengigkeiten: [PORT01](PORT01.md) (erledigt)

## Sachverhalt

Seit PORT01 erreicht das Argument QED. Es kommt aber **ein Zeichen zu kurz**
an — QED meldet „Fehler beim Lesen der Datei: README.TX". Gemessen, nicht
gefolgert:

```
[port01] Fopen len 12 path "C:\README.TX"
```

Die Kette, an der Quelle gelesen:

* cflibs `make_normalpath` (`sdk/cflib/cflib-21/fsnorm.c:85-95`) haengt einen
  `\` an und **entfernt ihn wieder**, wenn `path_exists()` nicht
  „existierendes Verzeichnis" meldet.
* `path_exists` (`fsexists.c:35-45`) ist `Fattrib(p,0,0) >= 0 && (attr&FA_DIR)`
  — fuer einen **Dateipfad** liefert das auf TOS 1-3, TOS 4 **und** EmuTOS
  gleichermassen falsch (`xchmod` → `findit("C:\README.TXT\")` → `EPTHNF`,
  `references/emutos/bdos/fsdir.c:402-411`).
* QED entfernt den Trenner **bedingungslos** (`src/poslist.c:53-54`, dasselbe
  Muster in `src/dd.c:99-101`) — es ist also gegen ein `make_normalpath`
  geschrieben, das den Trenner **immer stehen laesst**.

**Kein Portierungsfehler von uns:** unsere `fsnorm.c` ist bytegleich mit dem
unveraenderten cflib-21-Tarball (geprueft). QED und cflib-21 sind ein nicht
zueinander passendes Paar.

## Belegt, aber bewusst nicht uebernommen

Im Scratch end-to-end gezeigt (`measurements/measurement-2026-09-25-port01/method/
cflib_normalpath_experiment.py`, eine Zeile ueber den ueberwachten
`sdk/cflib/overlay/`-Kanal): laesst `make_normalpath` den Trenner stehen,
fragt QED `Fopen len 13 "C:\README.TXT"` und **oeffnet die Datei**
(`screendumps/exp2-qed.png`, Fenstertitel mit Inhalt).

Nicht gelandet, weil es das Verhalten einer **fremden Bibliothek** fuer
**jeden** cflib-Aufrufer aendert — in cflib selbst (`filesel.c:205,227,265`)
und in QED (`clipbrd.c:281`, `dd.c:99`, `global.c:433`, `poslist.c:53`,
`projekt.c:938`) — und damit fuer jede kuenftig portierte cflib-Anwendung.

## Der Altersunterschied, der das erklaert

| | |
|---|---|
| **cflib 0.21.0** | **18.07.2004**, Frank Naumann (`sdk/cflib/cflib-21/ChangeLog`) — unser Stand, eingefroren |
| **QED 5.0.5** | **12.11.2004**, aber `doc/history.txt` laeuft bis mindestens **2010** weiter (Patches u. a. Gerhard Stoll 2010, Odd Skancke 2005, Lonny Pursell 2006) |

QED ist also **juenger** als unsere cflib und wurde danach jahrelang weiter
gepflegt — gegen das, was cflib in dieser Zeit geworden ist. Die Uneinigkeit
ueber den Trenner ist damit kein Raetsel, sondern ein Altersunterschied.

**cflib wird gepflegt:** `github.com/freemint/cflib`, aktueller Schnappschuss
laut der eigenen README unter `https://tho-otto.de/snapshots/cflib/cflib-latest.tar.bz2`.

Damit ist **Weg 2 der naheliegendste** und **zuerst** zu pruefen: sagt eine
aktuelle cflib in `make_normalpath` etwas anderes? Wenn ja, ist der ganze
Befund kein Eingriff, sondern eine Versionsfrage — und das waere das beste
Ergebnis, weil es ohne Abweichung von einer fremden Bibliothek auskommt.

**Vorsicht dabei:** eine 21 Jahre neuere cflib bringt mehr mit als diese eine
Zeile. Vor dem Tausch feststellen, was sich sonst geaendert hat und was davon
unsere Portierung beruehrt.

## Beantwortet: die gepflegte cflib hat es selbst behoben

Geklont nach `Applications/cflib-git` (`github.com/freemint/cflib`, Stand
`ce57817`, 2026-07-14). Die fragliche Stelle in `fsnorm.c`:

```c
/* unsere Fassung (cflib-21, 2004) */   /* github, heute */
ret = path_exists(p);                   ret = path_exists(p);
if (!ret)                               /* always return path
    p[i] = '\0';   <-- kappt              with trailing backslash */
strcpy(path, p);                        strcpy(path, p);
return ret;                             return ret;
```

Das Abschneiden ist **entfernt**, mit einem Kommentar, der die Absicht
festhaelt: *„always return path with trailing backslash"*. Genau das Verhalten,
gegen das QED geschrieben ist — und genau die eine Zeile, die das Experiment
im Scratch nachgestellt hatte.

**Damit ist es keine Entscheidung mehr, sondern eine Versionsfrage.** Kein
Eingriff in eine fremde Bibliothek noetig; wir haben schlicht eine 21 Jahre
alte Fassung eingefroren, deren Fehler die Bibliothek selbst behoben hat. Die
unten aufgefuehrten Wege 1 und 3 entfallen.

## Was jetzt zu tun ist

Auf die gepflegte cflib wechseln — **aber nicht blind**: zwischen 2004 und
heute liegt mehr als diese Zeile. Zu klaeren, bevor getauscht wird:

* Welche Dateien `sdk/cflib/` heute baut und welche davon sich geaendert
  haben (der Tarball liegt als `cflib-21.tar.xz` daneben, der Vergleich ist
  mechanisch machbar).
* Was `sdk/cflib/expected-failures.txt` heute ausschliesst und ob das noch
  gilt.
* Ob `sdk/cflib/overlay/` (heute `fontsel.c`/`inline.rsh`) nach dem Wechsel
  noch gebraucht wird oder ob die Anpassungen inzwischen oben drin sind.
* Danach QED neu bauen und die Abnahme fahren: Datei auf QED ziehen, sie
  oeffnet sich.

## Die urspruenglich angenommene Entscheidung (hinfaellig)

Welche Seite wird korrigiert:

1. **cflib-Overlay** — `make_normalpath` laesst den Trenner stehen. Wirkt
   sofort, trifft aber alle cflib-Anwendungen; es ist eine Abweichung von der
   Originalbibliothek, die begruendet und festgehalten werden muss.
2. **Eine andere cflib-Fassung** — pruefen, ob eine spaetere/andere cflib das
   Paar mit QED richtig bildet. Dann ist es kein Eingriff, sondern die
   passende Version.
3. **`path_exists` reparieren** statt `make_normalpath` — es prueft heute
   „ist ein Verzeichnis" und wird fuer Dateipfade benutzt. Moeglicherweise
   ist **das** der eigentliche Fehler, und die anderen beiden Wege kurieren
   nur sein Symptom.

Reihenfolge: **erst Weg 2** (Versionsfrage, siehe oben — kein Eingriff noetig,
wenn er traegt), **dann Weg 3** (`path_exists` ist der einzige der beiden
Eingriffe, der die Bibliothek *richtiger* macht statt nur anders), **Weg 1
zuletzt**.

Die Projektregel bleibt: **QEDs Quelltext wird nicht geaendert.**

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

## Abschluss (2026-09-25)

Der Wechsel ist gelandet. Vorher stand eine Annahme im Ticket, die die
Messung **nicht bestaetigt hat** und die deshalb zuerst richtiggestellt
gehoert:

### Der Altersunterschied war nicht 21 Jahre, sondern 26 Monate

`sdk/cflib/cflib-21.tar.xz` war **kein Tarball von 2004**. Es war
`download/mint/cflib-21.tar.xz` vom **2024-04-30** aus dem tho-otto-Spiegel —
byteidentisch, sha256
`7f0d69f43f450df1f0a4699bda8738f5d8f7cc31257e0db531b8e8ba4062e349` gegen
`/Volumes/Daten/tho-otto-backu/www.tho-otto.de/download/mint/cflib-21.tar.xz`.
Er enthaelt `.github/workflows/`, `.travis.yml`, `README.md`; das ist ein
Quellschnappschuss des Git-Stands, kein Release von 2004. Die Zahl **0.21.0
vom 18.07.2004** steht nur in `VERSION`/`ChangeLog` — cflib hat seine
Versionsnummer seit 2004 nie erhoeht. **Die Versionsnummer sagt hier nichts
ueber den Stand der Quellen.**

Der tatsaechliche Abstand zum geklonten `ce57817` (2026-07-14) sind **13
Commits**.

### 1. Was `sdk/cflib/` baut und was sich davon geaendert hat

`build-cflib.sh` uebersetzt die COBJS-Liste aus cflibs eigener `SRCFILES`:
**129 .c-Dateien**, unveraendert 129 vor und nach dem Wechsel (`SRCFILES`
selbst ist gleich geblieben). Der mechanische Vergleich Tarball ↔ `ce57817`
(`diff -rq`) nennt **12 geaenderte Dateien**, davon **vier uebersetzte**:

| Datei | Commit | Was |
|---|---|---|
| `fsnorm.c` | `2ac3e85` 2024-06-12 | `make_normalpath` kappt den angehaengten `\` nicht mehr — **der PORT03-Defekt** |
| `fsexists.c` | `af02175` 2024-06-12, `cdb1fdf` 2024-06-18 | `path_exists` schneidet einen Schluss-`\` ab und prueft mit `Fsfirst` + `Fgetdta()->dta_attribute` statt `Fattrib` |
| `fexists.c` | `cdb1fdf` 2024-06-18 | `file_exists` ebenso auf `Fsfirst` statt `Fattrib` |
| `fontsel.c` | `399f7a6` 2025-04-02 | Speicherleck auf dem FONTSELECT-Pfad: `vst_unload_fonts` + `v_clsvwk` vor `do_fprotokoll` |

Die uebrigen acht sind `Makefile`, `README.md` und sechs CI-Skripte
(`.github/workflows/`, `.scripts/`) plus der Wegfall von `.travis*` — nichts
davon benutzt unser Bauskript, es faehrt `SRCFILES` direkt.

**Bemerkenswert und geprueft, weil es uns betrifft:** `cdb1fdf`/`af02175`
sind genau **Weg 3 des Tickets** („`path_exists` reparieren"), stromaufwaerts
schon getan. Die neue Fassung liest die DTA (`Fgetdta()->dta_attribute`) —
der Pfad, den `docs/MEMORY.md` als LP64-Struktur-Falle fuehrt. Er traegt:
mintlibs `_DTA` ist seit ADR-020/Task #98 die gepackte 44-Byte-Fassung mit
`_Static_assert`-gesicherten Offsets gegen `makbuf`
(`sdk/libc/mintlib/include/mint/ostruct.h:85-101`), und `Fgetdta` kommt als
`(_DTA *)trap_1_w(0x2F)` ueber `__newtos_trap0`, das ein volles 64-Bit-`x0`
liefert. Im Lauf sichtbar: cflibs Dateiauswahl listet Verzeichnisse als
Verzeichnisse (`screendumps/qed-d-fsel.png`).

### 2. `expected-failures.txt`

Die Liste ist leer, und sie **gilt unveraendert**: `compiled OK=129
skipped=0`, `failures match cflib/expected-failures.txt: 0 today, 0
recorded, 0 new`. Keine der vier geaenderten Dateien erzeugt eine Warnung
(`build/obj/{fsnorm,fsexists,fexists,fontsel}.err` sind leer); die einzige
Warnung im ganzen Lauf ist die alte Atari-Zeichensatz-Warnung in
`menucre.c:51`, unveraendert.

### 3. `overlay/` — beide Dateien bleiben noetig

Keine der beiden Anpassungen kann stromaufwaerts sein: beide haengen an
`__NEWTOS__`, das cflib nicht kennt.

* `inline.rsh` — stromaufwaerts unveraendert, das Overlay passt unveraendert.
* `fontsel.c` — **musste umgesetzt (rebased) werden**. Unser Overlay ist eine
  Vollkopie der Datei; die alte Kopie haette `399f7a6` (das Leck) wieder
  zurueckgenommen. Sie liegt jetzt auf der neuen Fassung, mit genau unserem
  einen Block (`#if defined(__GNUC__) && !defined(__NEWTOS__)` plus
  Begruendung) als einzigem Unterschied — nachgewiesen per `diff -u` gegen
  `ce57817`s `fontsel.c`.

### 4. Gewaehlte Quelle: Commit-Pin, kein datierter Tarball

**Entscheidung: der Git-Commit `ce57817c3f68c7d346d1081768505e81acaee807`
(2026-07-14), als `git archive --prefix=cflib-21/` gepackt und als
`sdk/cflib/cflib-21-ce57817.tar.xz` eingelagert** (sha256
`8427502d6a309864cfd4621201d032d04b512dddeb8f59eff33eca545683ed8e`).

Warum nicht der datierte Tarball: der Spiegel hat **keinen neueren
Quelltarball**. `download/mint/cflib-21.tar.xz` ist genau der Stand, den wir
gerade verlassen; die neueren `snapshots/cflib/cflib-0.21.0-*.tar.bz2` sind
**m68k-Binaerpakete** (`usr/lib/**/libcflib.a`, `usr/include/cflib.h`,
`stguide/`), null `.c`-Dateien. Das Repository ist also die einzige Quelle,
die die Korrekturen traegt.

Warum HEAD und nicht `399f7a6` (der letzte Code-Commit): `git diff --stat
399f7a6 ce57817` beruehrt nur CI-Skripte, `Makefile` und `README.md` — der
Tip kostet nichts und ist der Stand, den das Ticket untersucht hat.

Warum ein **Commit** und kein „latest": dieselbe Regel wie bei
`references/emutos` und `references/freemint`. Der Pin steht im Dateinamen
und in `build-cflib.sh`; `Applications/cflib-git` ist nur die Herkunft, kein
Bauabhaengigkeit — der Tarball ist selbsttragend.

### 5. Ein Nebenbefund, der den Wechsel sonst still verschluckt haette

Einen neuen Tarball hinzulegen hatte **keine Wirkung**: `sdk/lib-stale.sh`
hat die Tarballs bewusst nicht beobachtet, und `build-cflib.sh` extrahierte
nur, wenn der Baum **fehlt**. Auf jedem bestehenden Arbeitsbaum waeren
weiter die alten Quellen uebersetzt worden, ohne eine einzige Zeile Ausgabe —
derselbe Fehlermodus wie der Stale-Archive-Vorfall vom 2026-09-02. Zwei
kleine Aenderungen schliessen das: `build-cflib.sh` extrahiert neu, wenn der
Tarball juenger als der Baum ist (und stempelt den Baum danach, weil `tar`
sonst das Commit-Datum aus dem Archiv zurueckschreibt), und `lib-stale.sh`
beobachtet cflibs Tarball (per Glob, damit der Pin nur an **einer** Stelle
steht). mintlib/gemlib haben die Paarung noch nicht — sie brauchen sie, wenn
ihr Pin bewegt wird; das gehoert in ein eigenes Ticket.

### 6. Abnahme (QEMU raspi4b, 640x480)

`measurements/measurement-2026-09-25-port03/`.

* **Die Abnahme selbst:** `C:\README.TXT` auf `C:\QED.NAP` gezogen → QED
  oeffnet die Datei, Fenstertitel **`C:\README.TXT`**, voller Name
  (`screendumps/qed-c-open.png`, 3x-Ausschnitt `qed-c-open-title-zoom.png`),
  Inhalt im Fenster. Vorher: „Fehler beim Lesen der Datei: README.TX".
* **Zweiter cflib-Pfad:** Ctrl-O in QED oeffnet cflibs Dateiauswahl
  („Dateien oeffnen", `screendumps/qed-d-fsel.png`) — der Weg ueber
  `select_multi` → `select_file` → `filesel.c`, also genau die Stelle, an der
  die neuen `path_exists`/`file_exists` je Eintrag laufen.
* **Zweiter cflib-Anwender:** TeraDesk, ueber den `#Z`-Autorun gestartet,
  kommt hoch (`screendumps/tera-b-up.png`).
* Drei Laeufe, **null `[EXC]`, null `[PANIC]`**; `fsck_msdos -n` auf allen
  drei Kartenabbildern sauber (`serial/fsck.txt`).

QED wurde **nicht angefasst** — der Fehler sass in der Bibliothek und ist in
der Bibliothek behoben, indem wir sie auf den Stand gebracht haben, auf dem
sie ihn selbst behoben hat.

### 7. Was nur die echte Pi 4 bestaetigen kann

Nichts an diesem Ticket ist Pi-spezifisch (reiner Hostcode ueber den
GEMDOS-Trap), aber die Geste selbst — ein echtes Ziehen mit echter Maus statt
des IKBD-Injektors — bleibt Hardwaresache, wie in PORT01.
