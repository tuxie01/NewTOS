# PORT01 — Eine auf QED gezogene Datei erreicht die Anwendung nicht

Status: PARKED (2026-11-11) | Priority: P2 | Reason: legacy, evidence unverified; receipt PORT01-STATUS-DONE.MD exists | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 15, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: no cited file of the ticket/receipt is touched by a758173e1 (fix(sdk): crt0 builds argv from the command tail), named by body

Line 3 before the migration:
Status: **URSACHE GEFUNDEN UND BEHOBEN (2026-09-25) — Abnahme blockiert durch einen zweiten, eigenen Defekt (siehe Abschluss §4)** · Prioritaet: **P2** · Typ: Belegter Defekt — Pi-Runde 2026-09-24

Abhaengigkeiten: [DESK19](DESK19.md) (erledigt)

## Sachverhalt

Auf der echten Pi 4: eine `.TXT`-Datei auf `QED.NAP` gezogen **startet** QED,
aber die Datei wird nicht geoeffnet — die Anwendung sieht ihr Argument nicht.

## Was schon ausgeschlossen ist

**Der Kernel uebergibt den Schwanz.** Beide Startwege in
`kernel/src/desk/main.rs::launch_native` gehen durch
`crate::launch::run_program_with_tail`: der GEM-Zweig ueber `run_gem_app`
(`:1072-1081`) und der Konsolenzweig direkt (`:1013`). Es gibt keinen Pfad,
der den Schwanz fuer `.NAP` verwirft.

**Und er kommt nachweislich an — bei einem anderen Programm.** DESK19s
Messung (`measurements/measurement-2026-09-23-desk19-desk20/`): dieselbe Geste auf
`CMDLINE.NCP` druckt

```
CMDLINE: len 13 arg "C:\README.TXT"     (Parameter = Full path)
CMDLINE: len 10 arg "README.TXT"        (Parameter = File name)
```

Der Unterschied zwischen den beiden Faellen ist **nicht** der Kernel, sondern
was die Anwendung mit der Basepage macht: `sdk/examples/cmdline.c` liest
`p_cmdlin` selbst, QED geht durch die SDK-Startroutine und erwartet
`argc`/`argv`.

## Wo zu suchen ist (Eingaben, keine Diagnose)

1. **Die Startroutine des SDK** — baut sie `argc`/`argv` aus `p_cmdlin`?
   `sdk/libc/`s crt0. Es gibt einen Praezedenzfall: ein frueherer
   QED-Startabsturz lag genau dort (crt0 `argc`/`argv`), nicht an der Stelle,
   die zuerst verdaechtigt wurde.
2. **Die Basepage-Struktur unter LP64.** Bekanntes, bisher **latentes**
   Problem: mintlibs TOS-Strukturen (`BASEPAGE`, `DTA`) sind auf aarch64
   8-Byte-Felder, auf dem Draht 4 Byte. Wenn die Startroutine `p_cmdlin` ueber
   die mintlib-Struktur liest statt ueber den Draht-Offset, liest sie
   danebenвЂ" und das waere genau dieses Symptom.
3. ~~QED selbst~~ — **ausgeschlossen.** Der Benutzer arbeitet auf seinem
   echten TT genau so: Datei auf QED ziehen, und zwar auf eine
   **Desktop-Verknuepfung**, auf die er sie ablegt. QED wertet den Dateinamen
   also aus, und der Weg ist sein normaler Arbeitsablauf, kein Sonderfall.

## Beide Ziehwege sind auf unserer Seite verdrahtet

Auch der zweite, den der Benutzer auf dem TT benutzt: das Ablegen auf eine
**Desktop-Verknuepfung** baut den Schwanz genauso
(`kernel/src/desk/dir.rs:1885`, `AType::File`-Zweig, seit DESK30) wie das
Ablegen im Fenster (`:1793`, `fun_win2win`, seit DESK19). Beide enden in
`do_aopen_drop`. Es gibt also keinen dritten Pfad, an dem der Schwanz
verlorengehen koennte — und wenn der Defekt in der Werkzeugkette liegt,
betrifft er **beide** Wege gleichermassen.

## Arbeitsregel

Die portierte Anwendung bleibt **unveraendert**: Korrekturen gehoeren in die
Werkzeugkette (`sdk/`), nicht in QEDs Quelltext. Einzige Ausnahme waere
m68k-Assembler, den es hier nicht gibt.

Abnahme: dieselbe Geste auf der Pi, QED oeffnet die Datei. Ein
Zwischenbeleg, der ohne Hardware geht: ein `.NAP`, das sein `argv` druckt —
`sdk/examples/cmdline.c` ist die Konsolenfassung davon und kann als Vorlage
dienen.

[Gemeinsame Regeln](README.md) gelten.

---

## Abschluss (2026-09-25)

Messung: [`measurements/measurement-2026-09-25-port01/`](../measurements/measurement-2026-09-25-port01/).
Basis: Branch `desk-rebuild`, HEAD `45de38b`; die Aenderung liegt
**uncommitted** im gemeinsamen Baum. Nichts geflasht, nichts ausgeworfen,
keine Pi gefahren, nichts committet; kein
`git checkout/restore/clean/stash/reset` auf dem gemeinsamen Baum.

### 1. Es war Kandidat 1 — die Startroutine

`sdk/libc/crt0.S` hat `argc`/`argv` **fest verdrahtet**: `argc = 1`,
`argv = { "", NULL }`, ohne `p_cmdlin` je anzusehen. Der eigene Kommentar
dort sagte es sogar: mintlibs `crtinit.c` `parseargs()` — die Funktion, die
`argv` aus dem Schwanz baut — wird nicht uebersetzt (die *andere* Haelfte
derselben Datei ist m68k-Assembler, `sdk/libc/expected-failures.txt:25`), und
`crt0.S` hat ihre Stelle eingenommen, ohne ihre Arbeit zu tun. Dokumentiert
war der Zustand ebenfalls: `docs/sdk/07-mintlib-limits.md` §5 fuehrte
„`argc`/`argv` | always `1` / `{"", NULL}`" als bekannte Einschraenkung.

Damit sah **jedes** libc-gebundene Programm keine Argumente, egal was der
Starter uebergab — nicht nur QED.

### 2. Kandidat 2 ist ausgeschlossen, und das latente Problem ist **nicht** real geworden

`sdk/libc/mintlib/include/mint/basepage.h` ist bereits die LP64-korrigierte
Fassung: jedes Adress-/LONG-Feld ist ein festes `__UINT32_TYPE__`/
`__INT32_TYPE__`, und drei `_Static_assert` nageln `p_hitpa@4`, `p_env@0x2c`
und `sizeof(BASEPAGE) == 256` auf **beiden** ABIs fest. `ARGVTEST.NAP` liest
den Schwanz durch genau diese Struktur (`_base->p_cmdlin`) und bekommt ihn
in *beiden* Laeufen richtig:

```
ARGVTEST: cmdlin len 13 raw "C:\README.TXT"     (vorher UND nachher)
ARGVTEST: argc 1                                (vorher)
ARGVTEST: argc 2 / argv[1] "C:\README.TXT"      (nachher)
```

### 3. Der Fix

| Datei | Was |
|-------|-----|
| `sdk/libc/newtos-arch/cmdargs.c` | **neu**: `__libc_parseargs(basepage)` — die Argument-Haelfte von `crtinit.c`s `parseargs()`, fuer AArch64 neu geschrieben. Definiert ausserdem `__libc_argc`/`__libc_argv` (die zwei Symbole, die `posix/wordexp.c` bisher offen liess) |
| `sdk/libc/crt0.S` | ruft sie zwischen `__libc_init_heap` und `main` und reicht `argc`/`argv` durch; das feste `__crt0_argv` entfaellt |
| `sdk/libc/tests/cmdargs-host.{c,sh}` | **neu**: Host-Pruefstand, neun Faelle |
| `sdk/run-host-tests.sh` | haengt ihn ein |
| `sdk/examples/argvtest.c` | **neu**: `ARGVTEST.NAP`, druckt Rohschwanz **und** `argc`/`argv` in einem Lauf |
| `tools/make-sd-image.sh` | legt `C:\ARGVTEST.NAP` ins Abbild |
| `sdk/libc/build-libc.sh`, `sdk/lib-stale.sh` | Uebersetzungsschritt + Kommentare |

Die Regeln sind mintlibs eigene, weil portierte Anwendungen gegen sie
geschrieben sind (QEDs `src/main.c:147-190` setzt einen Namen mit Leerzeichen
aus mehreren `argv`-Eintraegen wieder zusammen — genau, weil diese libc so
trennt): `argv[0]` ist `""`, fuehrende Leerzeichen/Tabs fallen weg, getrennt
wird an Laeufen von Leerzeichen/Tabs, und beginnt der Schwanz mit `'`, gelten
die MagX-Anfuehrungsregeln.

Was **nicht** dabei ist und warum: die Umgebungs-Haelfte von `parseargs()`
(`environ` und die `ARGV=`-Erweiterung). Auf NewTOS ist `p_env` auf jedem
existierenden Startweg 0 (`EnvSpec::None`), also kann kein `ARGV=`-Block ein
Programm erreichen; die Stelle dafuer ist im Kopf von `cmdargs.c` benannt.

Ort der Ablage: mintlib legt `argv` in die Luecke ueber dem BSS und startet
den Heap darueber — das geht hier nicht, weil `__libc_init_heap` die Arena
genau dort (`_end`) beginnt. Zeichen und Zeigerfeld liegen darum im BSS
dieser Datei; beide Groessen sind aus `p_cmdlin`s 128 Byte hergeleitet, nicht
geraten (hoechstens 64 Argumente).

### 4. Ein zweiter, eigener Defekt blockiert die Abnahme (neues Ticket)

Mit dem Fix **handelt** QED nach dem Argument, kann die Datei aber nicht
lesen und meldet „Fehler beim Lesen der Datei: README.TX" — ein Zeichen zu
kurz. Gemessen, nicht gefolgert
(`measurements/measurement-2026-09-25-port01/serial/trace-serial.log:1033`):

```
[port01] Fopen len 12 path "C:\\README.TX"
```

An der Quelle gelesen:

* cflibs `make_normalpath` (`sdk/cflib/cflib-21/fsnorm.c:85-95`) haengt einen
  `\` an und **entfernt ihn wieder**, wenn `path_exists()` nicht „existierendes
  Verzeichnis" sagt;
* `path_exists` (`fsexists.c:35-45`) ist `Fattrib(p,0,0) >= 0 && (attr&FA_DIR)`
  — fuer einen Dateipfad falsch auf TOS 1-3, TOS 4 und EmuTOS gleichermassen
  (EmuTOS `xchmod` → `findit("C:\README.TXT\")` → `EPTHNF`,
  `references/emutos/bdos/fsdir.c:402-411`);
* QED entfernt den Trenner **bedingungslos** (`src/poslist.c:53-54`, gleiches
  Muster in `src/dd.c:99-101`), ist also gegen ein `make_normalpath`
  geschrieben, das ihn immer stehen laesst.

Unsere `fsnorm.c` ist **byte-identisch** mit dem unveraenderten cflib-21-Tarball
— kein Portierungsfehler von uns; QED und cflib-21 sind ein nicht zueinander
passendes Paar.

Belegt, nur im Scratch, **nicht uebernommen**
(`method/cflib_normalpath_experiment.py`, eine Zeile ueber den gewachten
`sdk/cflib/overlay/`-Kanal): laesst `make_normalpath` den angehaengten Trenner
stehen, fragt QED den richtigen Namen ab und **oeffnet die Datei**
(`serial/exp2-serial.log:1033` `Fopen len 13 path "C:\\README.TXT"`,
`screendumps/exp2-qed.png` — Fenstertitel `C:\README.TXT` mit Inhalt).

Nicht uebernommen, weil die Aenderung das Verhalten einer *fremden* Bibliothek
fuer **jeden** cflib-Aufrufer aendert (in cflib: `filesel.c:205,227,265`; in
QED: `clipbrd.c:281`, `dd.c:99`, `global.c:433`, `poslist.c:53`,
`projekt.c:938`) und damit fuer jede portierte cflib-Anwendung. Welche Seite
zu korrigieren ist — cflib ueber `sdk/cflib/overlay/`, oder eine andere
cflib-Fassung — ist eine Entscheidung des Leads, nicht dieses Tickets.

### 5. Abnahme

* Der **Auftrag dieses Tickets** — das Argument erreicht die Anwendung — ist
  erfuellt und belegt (§2).
* Die **Abnahme des Tickets** („QED oeffnet die Datei") ist es erst nach §4.
  Der Weg dorthin ist belegt und einzeilig, aber nicht meine Entscheidung.
* Nur auf der echten Pi zu bestaetigen: die Geste mit echter Maus, und der
  zweite Ziehweg ueber eine Desktop-Verknuepfung
  (`kernel/src/desk/dir.rs:1885`). Beide enden in `do_aopen_drop` und
  uebergeben denselben Schwanz; die Startroutine sieht ohnehin nur die
  BasePage.
