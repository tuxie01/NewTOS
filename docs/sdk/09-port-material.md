# 09 — Woher die Quellen kommen: Bestand, Fassungen, Lizenzen

Stand: Abschnitte 1, 2 und 5 am 2026-10-20 gegen den Baum neu geschrieben
(TASK/SDK01 B); die Abschnitte 3 und 4 sind Stand 2026-09-25 (PORT03) und
unveraendert. §1 um compiler-rt ergaenzt am 2026-10-21 (TASK/SDK01 E). Diese Seite haelt fest, **welches Material fuer Portierungen
vorliegt, in welcher Fassung und unter welcher Lizenz** — damit die naechste
Portierung nicht wieder mit der Suche anfaengt.

Sie ist eine Bestandsaufnahme, keine Anleitung. Wie portiert wird, steht in
[04 — Porting anatomy](04-porting-anatomy.md) und
[08 — Porting checklist](08-porting-checklist.md).

---

## 1. Was bereits portiert ist und laeuft

(Lizenzen: LGPL 2.1 fuer LDG, WinDom und die zView-Codecs, GPL fuer zView,
GPLv2 fuer doomgeneric, jeweils wie in den Kapiteln 10 bis 13 und im Kopf von
`build-doom.sh` belegt; fuer die uebrigen Eintraege hier nicht neu erhoben.)

Zur Herkunft: Bibliotheken liegen als unveraenderte Upstream-Archive im
Repository (Tarball/Zip), die App-Baeume unter `sdk/apps/` sind nicht
versioniert (siehe [04](04-porting-anatomy.md)).

| Was | Wo | Bemerkung |
|---|---|---|
| **mintlib** 0.60.1 | `sdk/libc/` | libc-Unterbau. Bekannte Luecken in [07](07-mintlib-limits.md); `crt0.S` baut `argv` seit PORT01 aus `p_cmdlin`, `environ` und `ARGV=` seit PORT09 |
| **gemlib** 0.44.0 | `sdk/gemlib/` | AES/VDI-Bindungen |
| **cflib** | `sdk/cflib/` | auf Commit `ce57817` (2026-07-14) gepinnt, seit PORT03 — siehe §3 |
| **libm** = OpenLibm 0.8.8 | `sdk/libm/` | seit T07; die `long double`-Funktionen (`sdk/libm/openlibm-0.8.8/ld128/`) seit TASK/SDK01 J ([07](07-mintlib-limits.md) §4, §6) |
| **compiler-rt 21.1.8** (LLVM, nur `lib/builtins`) | `sdk/compiler-rt/` | seit TASK/SDK01 E: Laufzeit des Compilers (`libclang_rt.builtins.a`: binary128-`long double`, komplexes `*`/`/`), von `newtos-cc` immer gelinkt. Quelle: unveraenderter Release-Tarball `compiler-rt-21.1.8.src.tar.xz` von `github.com/llvm/llvm-project/releases/tag/llvmorg-21.1.8`, SHA-256 und gepruefte Signatur in `compiler-rt-sources.sha256`. **Lizenz: Apache-2.0 WITH LLVM-exception** (`LICENSE.TXT` im Tarball, SPDX-Kopf jeder Quelle); die LLVM-Ausnahme befreit Programme, in die die Bausteine eingebunden werden, von den Hinweispflichten der Apache-Lizenz (Abschnitte 4(a), 4(b), 4(d)); ihr zweiter Absatz regelt die Verbindung mit GPLv2-Software (Doom) (`LICENSE.TXT` Z. 208-222) |
| **LDG** (SVN r133, `sdk/ldg/ldg-r133/src/devel/`-Bibliothek) | `sdk/ldg/` | `libldg.a`, [10](10-ldg-modules.md); Quelle `Applications/ldg-src` wurde nur zum Einlagern des Tarballs benutzt |
| **libshare** 0.1.1 | `sdk/libshare/` | Allokator jeder `.NLD`-Bibliothek, [10](10-ldg-modules.md) §6 |
| **zlib, libpng, libjpeg, giflib, libexif** | `sdk/{zlib,libpng,libjpeg,giflib,libexif}/` | Codec-Bibliotheken, [11](11-codec-libraries.md) |
| **WinDom 1.21.3** (+ zViews Patch) | `sdk/windom/` | `libwindom1.a`, [12](12-windom.md) |
| **zView** (z-tools @ `04805b7`, LDG-Fassung, ohne PDF) | `sdk/zview/` | `ZVIEW.NAP` + zehn `.NLD`-Codecs, [13](13-zview.md) |
| **QED** (`freemint/qed` @ `f12055f`) | `sdk/apps/qed` (nicht versioniert; Patches in `sdk/overlays/qed/`) | laeuft |
| **TeraDesk** (`freemint/teradesk` @ `b5532b8`) | `sdk/apps/teradesk` (nicht versioniert; Patches in `sdk/overlays/teradesk/`) | laeuft |
| **CoreMark** (`czietz/coremark` @ `b546802`) | `sdk/apps/coremark` (nicht versioniert; Portierungsschicht `sdk/overlays/coremark/newtos/`) | Benchmark |
| **Doom** (`ozkl/doomgeneric` @ `dcb7a8d`) | `Applications/doomgeneric` (nicht versioniert), eigene Plattformschicht `sdk/apps/doom/` | `build-doom.sh` |
| `PRESENT.NAP` | `sdk/apps/present/` | eigenes Programm, [14](14-present.md) |

Die Herkunftsangaben QED/TeraDesk/CoreMark/Doom stammen aus `git remote -v`
und `git rev-parse --short HEAD` der eingebetteten Checkouts (2026-10-20);
nirgends im Repository ist festgehalten, wie man sie wieder beschafft. Ein
Klon-Rezept steht noch aus (geplant, TASK/SDK01 Paket C).

## 2. Quellen im Baum, noch nicht portiert

Alles, was nicht in §1 steht. Die Tabelle ist die Bestandsaufnahme von
`Applications/` und `sdk/apps/`; Lizenzangaben dort, wo sie frueher erhoben
wurden.

| Was | Wo | Umfang / Lizenz |
|---|---|---|
| **zTask, zWeather, zcodeclib** (der Rest von z-tools) | `Applications/z-tools/` | nicht gebaut; zView selbst ist portiert (§1) |
| **LDG 2.35** (SDK-Fassung) | `Applications/ldg-2` | **nicht** der Laderquelltext: die 7 C-Dateien sind Demos, die Umsetzung kommt als m68k-`libldg.a` und `ldg.prg`. Fuer die Portierung wird der SVN-Stand r133 benutzt (§1). LGPL seit 2.32 |
| **cflib (Herkunft des Pins)** | `Applications/cflib-git` | `github.com/freemint/cflib`; aus `ce57817` (2026-07-14) ist `sdk/cflib/cflib-21-ce57817.tar.xz` gepackt. Der Klon ist **keine** Bauabhaengigkeit — siehe §3 |
| **Thing** | `Applications/thing-neo-codeSVN-r62` | alternativer Desktop, unbewertet |
| **HighWire** | `sdk/apps/highwire` | Browser — braucht Netzwerk (NET01-NET06, ungebaut) |

## 3. Die Altersfalle: cflib — behoben, aber anders als gedacht (PORT03)

**Stand seit 2026-09-25: `sdk/cflib/` ist auf den Git-Commit `ce57817`
(2026-07-14) gepinnt** (`sdk/cflib/cflib-21-ce57817.tar.xz`, aus
`git archive --prefix=cflib-21/` gepackt; Herkunft `Applications/cflib-git`).

Der Fehler, der dahin gefuehrt hat (PORT01/PORT03): `make_normalpath` kappte
den Pfadtrenner, QED erwartet ihn — QED las `README.TX` statt `README.TXT`.
Die gepflegte cflib hat das Kappen laengst entfernt, mit Kommentar:

```c
/* unsere alte Fassung */            /* seit 2ac3e85, 2024-06-12 */
ret = path_exists(p);                ret = path_exists(p);
if (!ret)                            /* always return path
    p[i] = '\0';                        with trailing backslash */
strcpy(path, p);                     strcpy(path, p);
```

### Die Lehre ist nicht „2004 gegen heute", sondern „die Versionsnummer luegt"

Die naheliegende Erzaehlung war: eine Bibliothek von **2004** trifft auf ein
QED von 2010. Gemessen stimmt sie nicht. Der eingelagerte Tarball war der
**Quellschnappschuss vom 2024-04-30** aus dem tho-otto-Spiegel (byteidentisch,
sha256 `7f0d69f4…4062e349`; er enthaelt `.github/workflows/`, `.travis.yml`).
Er war **26 Monate**, nicht 21 Jahre alt. Nur `VERSION`/`ChangeLog` sagen
„0.21.0, 18.07.2004" — **cflib hat seine Versionsnummer seit 2004 nie
erhoeht.**

Merksatz dieser Seite, praeziser gefasst: *eine eingefrorene Fremdbibliothek
ist eine Altlast mit Verfallsdatum — und ihre Versionsnummer sagt nicht, wie
alt der Stand ist.* Deshalb wird hier nach **Commit** gepinnt, wie bei
`references/emutos` und `references/freemint`, und nicht nach Version oder
Datum.

### Was der Abstand von 13 Commits enthielt

Vier uebersetzte Dateien: `fsnorm.c` (der Defekt), `fsexists.c` + `fexists.c`
(`path_exists`/`file_exists` pruefen mit `Fsfirst` + DTA statt `Fattrib` und
schneiden einen Schluss-`\` ab) und `fontsel.c` (Leck auf dem
FONTSELECT-Pfad). Alles Uebrige waren CI-Skripte, `Makefile` und `README.md`.
`SRCFILES`/COBJS blieben bei 129 Dateien, `expected-failures.txt` bleibt leer.

**Wichtig fuer kuenftige Wechsel:** `sdk/cflib/overlay/fontsel.c` ist eine
Vollkopie der Originaldatei. Ein Pin-Wechsel muss sie **umsetzen** (rebasen),
sonst nimmt sie stromaufwaerts behobene Fehler wieder zurueck. Das ist die
generelle Gefahr jedes Voll-Datei-Overlays.

### Der Wechsel muss auch ankommen

Einen neuen Tarball hinzulegen genuegte nicht: `build-cflib.sh` extrahierte
nur bei **fehlendem** Baum, und `sdk/lib-stale.sh` beobachtete die Tarballs
bewusst nicht. Seit PORT03 extrahiert cflibs Skript neu, wenn der Tarball
juenger als der Baum ist, und `lib-stale.sh` beobachtet ihn. **mintlib und
gemlib haben diese Paarung noch nicht** — wer ihren Pin bewegt, braucht sie
dort ebenso (`MINTLIB_REFRESH=1` / `GEMLIB_REFRESH=1` bis dahin).

**Der naechste Pruefstand:** gemlib (0.44.0) und mintlib (0.60.1) stammen aus
derselben Ecke und derselben Zeit und sind bisher nicht gegen ihren gepflegten
Stand gehalten worden.

## 4. Das Materialarchiv

`/Volumes/Daten/tho-otto-backu` — 42 GB Spiegel von `tho-otto.de`,
Paketablage unter www.tho-otto.de/download/. Enthaelt unter anderem:

* `pml-2.03.tar.xz` — Quelltext, die von zView geforderte Fassung
* `windom-2.0.1.tar.xz` — **neuer** als die geforderte 1.21.3
* `cflib-21-2024*.tar.xz` — datierte Schnappschuesse. **Achtung (PORT03):**
  nur download/mint/cflib-21.tar.xz (2024-04-30) ist **Quelltext**; die
  `*-dev.tar.xz` und die neueren `snapshots/cflib/*.tar.bz2` sind
  m68k-**Binaer**pakete (`usr/lib/**/libcflib.a`, usr/include/, stguide/),
  fuer eine Portierung also wertlos. Der Spiegel hatte keinen Quelltarball,
  der den PORT03-Fix enthielt — deshalb der Commit-Pin
* dazu Dutzende weiterer portierter Bibliotheken

**Der eigentliche Wert liegt in den `*-build.sh`-Skripten**, die jedem Paket
beiliegen: sie zeigen, wie die Bibliothek fuer Atari konfiguriert und gebaut
wird. Genau dieses Wissen hat bei QED gefehlt und musste erarbeitet werden.

Nicht darin gefunden: `libshare`, `libmem` (letzteres steckt laut zViews
eigener Liste moeglicherweise in `ldg-2`).

## 5. Die Regel, die ueber allem steht

(Korrigiert 2026-10-20, TASK/SDK01 B. Hier stand: „Die portierte Anwendung
bleibt unveraendert. Korrekturen gehoeren in die Werkzeugkette, die einzige
Ausnahme ist m68k-Assembler.“ Das stimmte nicht mit dem Baum ueberein: von
den neun Patch-Dateien fuer QED und TeraDesk ist eine m68k-Assembler, acht
sind LP64-Korrekturen, darunter eine Neufassung des QED-Speichermanagers.)

**Upstream-Quellen bleiben unveraendert** — als Archiv oder als unversionierter
Checkout; im Baum der Anwendung wird nichts editiert. Was **jedes** Programm
trifft (eine Bibliothek, ein Header, `crt0`, ein Compiler-Schalter, die
LP64-Breite eines TOS-Feldes in libc/gemlib), wird **einmal im SDK** behoben.
Was **nur ein Programm** braucht (m68k-Assembler, Annahmen ueber `long`,
Strukturgroesse oder Byte-Reihenfolge im Anwendungscode), steht als
**Patch-Datei** in `sdk/overlays/<programm>/` (zView: `sdk/zview/overlay/`),
mit einem Kopfkommentar, der sagt, warum und was sich gegenueber Upstream
aendert. Die Liste der vorhandenen Patches mit ihren Gruenden steht in
[04](04-porting-anatomy.md#the-rule-upstream-stays-unchanged-fixes-go-where-they-belong).

PORT03 bleibt das Musterbeispiel fuer die erste Haelfte: der Fehler sass in
einer Bibliothek, die Versuchung war, ihn in QED zu umgehen — richtig war,
die Bibliothek auf den Stand zu bringen, auf dem sie ihn selbst behoben hat.
