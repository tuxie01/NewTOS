# ARCH06 — Quittung: `_MCH` = `0xFFFFFFFF` umgesetzt, `_VDO` gemessen

Datum 2026-10-29 · Ticket [ARCH06](ARCH06.md) · Entscheidung: Nachtrag zu
ADR-012 vom 2026-10-28 (`docs/MEMORY.md`) · Beleg der Kopien:
`docs/analysis/machine-identity-adr-proposal-2026-10-28.md` Abschnitt 6 ·
Lauf `measurements/measurement-2026-10-29-arch06/RUNINFO.TXT`

`_VDO` ist **nicht** geaendert: die Messung (Punkt 3) liefert die Fakten,
die Entscheidung liegt beim Anwender.

## 1. Wert und Kopien

| Stelle | Aenderung |
|---|---|
| `tos-api/src/cookie.rs` | neue Konstante `MCH_NOHARD = 0xFFFF_FFFF` (`tos-api/src/cookie.rs:250@MCH_NOHARD`, mit Beleg `references/emutos/include/cookie.h:50`, `references/emutos/bios/machine.c:400-401`, ARAnyM-Hinweis, Mehrdeutigkeit der Wertform); `seed_defaults` setzt `_MCH` darauf (`tos-api/src/cookie.rs:225@MCH_NOHARD`); Doc-Kommentar von `seed_defaults` nennt `_CPU` und den offenen `_VDO`. Die Konstante steht **hinter** `seed_defaults`, damit die verankerten Zitate der datierten Analysen (`abi-assumptions-check`, `lp64-tos-struct-survey`, `machine-identity-adr-proposal`) gueltig bleiben |
| `kernel/src/selftest_cookie.rs` | `_MCH`-Pruefung: Wertform `== -1`, dazu Zeigerform `E_OK` und `v == 0xFFFFFFFF`, eine Messzeile mit beiden Formen |
| `kernel/src/initinfo.rs`, `programs/desktop/src/inf.rs` | nur Kommentare (`_MCH` = `MCH_NOHARD`, der veraltete Zeilenverweis auf `cookie.rs` ersetzt). Die About-Box zeigt den Wert **nicht**; die Maschinenzeile bleibt die Baukonstante `MACHINE_NAME` (X03) |
| `tos-api/tests/gemdos_ssystem_mch_cookie.rs` | neu, 4 Tests: Konstante, Zeigerform liest `0xFFFFFFFF` mit `E_OK`, Wertform liefert `-1` wie ein Fehltreffer, Zeigerform trennt Fehltreffer (`EERROR`, Puffer unberuehrt) |
| `sdk/cflib/tests/cookie-host.c` | Jar-Kopie des Kernsatzes: `_MCH` `0xFFFFFFFFL` (null-erweitert, wie `Getcookie` liefert) |
| `docs/api/bios/_cookie-jar.md` | Zeilen 53, 128-145, 148: Wert, Beleg, Mehrdeutigkeit der Wertform, `_CPU`/`_FPU` nach dem Nachtrag, Zitate verankert |
| `docs/HARDWARE-PROFILE.md` | Zeilen 410, 415-416 |
| `docs/api/desktop/deskmain.md` | Zeilen 1339-1341 |
| `TASK/COOK01.md`, `TASK/X03.md` | je ein datierter Nachtrag, kein Umschreiben |
| `docs/analysis/deps/mapping.tsv` | Gruppe E, siehe 4 |

Nicht geaendert, weil schon richtig: `docs/COMPATIBILITY.md` (ACCEPTED-Eintrag
`_MCH`), `docs/sdk/08-porting-checklist.md`, `docs/MEMORY.md` (Nachtrag).
mintlib `sysinfo` (SDK03-Overlay) und sein Hosttest decken `0xFFFFFFFF`
bereits ab.

## 2. Gemessenes `_MCH` (QEMU raspi4b)

Kernel-Selbsttest (Verdrahtung nur in einer Scratch-Kopie,
`measurements/measurement-2026-10-29-arch06/method/selftest-wire.py`):
`[cookie] === Cookie jar PASS (18/18) ===`, darin
`_MCH: value form x0 = 0xffffffffffffffff, pointer form rc = 0, v = 0xffffffff`.

EL0-Programm aus `C:\AUTO` (`measurements/measurement-2026-10-29-arch06/method/mchprobe.c`):

- Wertform `Ssystem(S_GETCOOKIE, '_MCH', 0)`: `x0 = 0xFFFFFFFFFFFFFFFF` (-1) —
  **dieselbe Zahl** wie fuer ein fehlendes Tag (`ZZZZ`: ebenfalls -1). Die
  Wertform kann `_MCH` und „nicht gefunden“ nicht unterscheiden (gilt in
  FreeMiNT fuer jeden Cookie mit Wert -1).
- Zeigerform `Ssystem(S_GETCOOKIE, '_MCH', &v)`: `rc = 0`, der Kern schreibt
  4 Byte; ein 8-Byte-`long` mit Vorbelegung `0x5A5A…` liest danach
  `0x5A5A5A5AFFFFFFFF`. Fehlendes Tag: `rc = -1`, Puffer unberuehrt.
- mintlib `Getcookie(_MCH, &v)`: `C_FOUND`, `v = 0x00000000FFFFFFFF`
  (null-erweitert, wie im Nachtrag zu ADR-012 beschrieben).
- `$5A0`-Spiegel: Slot 1 `_MCH` `0xFFFFFFFF`.
- mintlib nach SDK03: `sysinfo(SI_PLATFORM)` = „unknown“,
  `SI_HW_PROVIDER` = „unknown“, `uname().machine` = „unknown“ — kein
  Null-Zeiger.
- zView (`sdk/zview/overlay/zview/plugins/jpg/jpgldg.c:801-811@mach`): der Zweig
  `mach == 0x50000L` laeuft nur mit `_JPD`; das Jar haelt nur
  `NTOS _MCH _IDT _AKP EdDI` — toter Pfad bestaetigt (Lesen + Jar-Abzug,
  zView nicht gestartet). Mit `0xFFFFFFFF` waere er auch mit `_JPD` nicht
  genommen.

## 3. `_VDO`-Messung (Fakten fuer die Anwenderentscheidung)

TeraDesk = `DESKTOP.NAP` mit behobenem `find_cookie` (SDK04 C6, 8da6370e),
`sdk/apps/teradesk/video.c` unveraendert. Aufrufe per `bios-arg-trace`,
Ablauf per `svc-seq-trace`, Bedienung per Injektor (Options → Video options…).

**Ohne `_VDO` (ST-Zweig):**

- Start: `Getrez` → 4, drei Jar-Suchen (`_VDO`, `OVER`, `Lace`: alle fehlen),
  `Blitmode(-1)` → 0. Kein `VsetMode`/`VgetMonitor`. `Blitmode` gibt es nur
  im ST/STe-Zweig: der Zweig ist durch den Aufruf belegt.
- Desktop unauffaellig (Screendump bytegleich mit dem `_VDO`-Lauf).
- Video-Dialog: ST-Low/ST-Medium waehlbar, TT-Medium als Auswahl gesetzt
  (aus `Getrez` 4), Blitter gesperrt, **Overscan freigeschaltet**.
- **Overscan ankreuzen + OK → TeraDesk stirbt mit Bus Error** bei
  `0xFFFC00` (Schreibzugriff, `X11 = 0xD6`), Absturzstelle in
  `get_set_video`: `sdk/apps/teradesk/video.c:280-294` (nicht versionierter Upstream-Baum) ruft `Super(0L)` und schreibt das
  ACIA-Steuerregister (Lacescan-Schalter). Grund fuer die Freischaltung:
  `long over` ist global null-initialisiert (Zeile 52), `find_cookie`
  laesst es ohne `OVER`/`Lace` unberuehrt, also gilt `over != -1L`
  (Zeile 487). Eine gewoehnliche Bedienung bringt TeraDesk ohne `_VDO`
  also zum Absturz.

**Mit `_VDO = 0x00030000` (Falcon-Zweig, per `Ssystem(S_SETCOOKIE)` aus einem
Scratch-AUTO-Programm):**

- Start: `Getrez` → 4, Jar-Suche `_VDO` (gefunden), dann
  **`VsetMode(-1)` → `0x58` und `VgetMonitor()` → `0x59`** — NewTOS
  antwortet mit der Funktionsnummer, wie vermutet. TeraDesk liest das als
  Modus `0x58` (80 Spalten, VGA, Overscan, 2 Farben) und Monitor `0x59`
  (kein gueltiger Typ). Kein `Blitmode`, keine `OVER`/`Lace`-Suche.
- Video-Dialog: TT-Knoepfe heissen „VGA-40-column“/„VGA-80-column“,
  VGA-80 gesetzt, **Overscan angekreuzt** (aus Bit `0x40` der
  Funktionsnummer), „Doubleline“ statt Blitter.
- **OK ohne jede Aenderung → Alert „A change of video mode will terminate
  all running applications“** (der aus dem Dialog gebaute Modus weicht von
  `0x58` ab); OK → `shel_write(SHW_RESCHNG)` → AES-Neustart → TeraDesk
  startet neu, gleicher Zustand, kein Absturz. Die `shel_write`-Argumente
  zeichnet der Tracer nicht auf (nur gelesen, Zeilen 356-379 derselben Datei).

Zusammengefasst: ohne `_VDO` bietet TeraDesk eine ST-Overscan-Option an,
deren OK auf NewTOS zum Bus Error fuehrt; mit Falcon-`_VDO` baut TeraDesk
seinen Dialog aus den Funktionsnummern `0x58`/`0x59` und verlangt beim
blossen OK einen Aufloesungswechsel (harmloser AES-Neustart).

## 4. `mapping.tsv` Gruppe E

| Funktion | Klasse | NewTOS / Beleg |
|---|---|---|
| `setvalue_mch` | mapped | `tos-api/src/cookie.rs` `seed_defaults` (`MCH_NOHARD` = Zweig ohne `CONF_ATARI_HARDWARE`) |
| `setvalue_vdo` | `design:ADR-012@docs/MEMORY.md:401` | `_VDO` fehlt; der Nachtrag (Zeile 401) gibt den Wert nach dieser Messung dem Anwender — Zeile mit der Entscheidung neu einordnen |
| `machine_detect` | `design:source:docs/HARDWARE-PROFILE.md` | sondiert Atari-Hardware; NewTOS hat keine, fester Satz ohne Sondierung (3.7) |
| `machine_init` | `design:hardware:mfp68901` | maskiert MFP/TT-MFP, setzt SCC zurueck |
| `guess_machine_name` | mapped | `kernel/src/initinfo.rs` `machine_name` (Analogon: Name aus dem Board, X03) |
| `add_cookie_frb` | `design:hardware:acsi` | `_FRB` = ST-RAM-Puffer fuer ACSI/SCSI-DMA (`bios/acsi.c:145-160`) |
| `total_alt_ram` | mapped (2 Zeilen) | `tos-api/src/bdos/mem/stats.rs` `pool_stats` + `total_bytes` (TT-RAM-Zeile des Startschirms) |
| `altram_init` | mapped | `tos-api/src/bdos/mem/init.rs` `init` (TT-RAM aus der Speicherkarte der Firmware) |

Zeile 401 ist die Nachtragszeile im Abschnitt von ADR-012 (geprueft durch
`cargo xtask deps --check-mapping`: 1040 Zeilen, alle aufgeloest).
`cargo xtask deps setvalue_mch --up --layers bios,bdos`: unvollstaendig nur
wegen `bios_init`/`biosmain`, wie im Ticket erwartet.

## 5. Tests, Bau, doc-check

- `cargo xtask test`: **1828 passed, 0 failed, 1 ignored** (Basis 1824/0/1 + 4 neu).
- SDK-Hosttests: **30/0/0**; `make -C tosdump host-test`: gruen.
- Bau beider Boards (`cargo xtask build`): Exit 0, je 159 Warnungen, gleich
  HEAD 8b11d2f4; Clippy kernel (tos-api 74 / kernel 121), desktop, newdesk,
  tos-api `--tests`: keine neue Warnung (`checks/lint-*.txt` im Lauf).
- Smoke-Boot unveraendert: raspi4b „desktop up (1 drives)“, qemu-virt
  „desktop up (0 drives)“. Volle Regression = Tester.
- doc-check: `docs/api/bios/_cookie-jar.md` 0, `docs/api/desktop/deskmain.md` 0,
  `docs/MEMORY.md` 0, `TASK/COOK01.md` 0, diese Quittung 0.
  `docs/HARDWARE-PROFILE.md` 27 und `TASK/X03.md` 2 — **alle vorbestehend**,
  keiner auf einer geaenderten Zeile (alte Treiber- und
  Desktop-Pfade aus der Zeit vor den Umzuegen). Keine Datei unter `docs/` zitiert die
  geaenderten Quellen jetzt falsch (geprueft fuer alle Dokumente, die
  `cookie.rs`, `selftest_cookie.rs`, `initinfo.rs`, `inf.rs` zitieren).

## 6. Offen / Hinweise

- `_VDO`: Entscheidung beim Anwender (Abschnitt 3). Danach `setvalue_vdo` in
  `mapping.tsv` neu einordnen und ggf. `docs/api/bios/_cookie-jar.md` anpassen.
- `TASK/ARCH06.md` Status und `docs/STATUS.md` nicht angefasst (Lead).
- Der tosdiff-Fall folgt mit TOOL18 (Abnahme des Tickets).
