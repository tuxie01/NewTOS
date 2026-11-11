# PROC04 — Remaining GEMDOS handle deviations found during PROC02

Status: DONE (2026-11-11) | Priority: P3 | Commit: a37f0c7d4 | Receipt: PROC04-STATUS-DONE.MD

Migrated (owner batch 3, 2026-11-11), evidence: receipt TASK/PROC04-STATUS-DONE.MD exists; commit a37f0c7d4; TASK/PROC04-STATUS-DONE.MD, commit a37f0c7d4 (feat(gemdos): character functions follow p_uft redirection (PROC04 pt ), fix/feat commits naming the id: f5efac771, a37f0c7d4, 40ca78f70 (n=4), measurements/measurement-2026-10-06-proc02/

Line 3 before the migration:
Status: **ERLEDIGT** 2026-10-19 (QEMU) — [Quittung](PROC04-STATUS-DONE.MD) · Prioritaet: **P3** · Typ: Abweichungen von EmuTOS — aus [PROC02](PROC02-STATUS-DONE.MD)

Abhaengigkeiten: —

## Sachverhalt (vom PROC02-Worker gemeldet 2026-10-06, nicht einzeln nachgeprueft)

1. Zeichenfunktionen (`Cconout`, `Cconin`, … ) folgen der Umlenkung ueber
   `p_uft` nicht und gehen immer zur Konsole; EmuTOS leitet sie um
   (`references/emutos/bdos/bdosmain.c:498-568`).
2. `p_curdir`/`p_curdrv` sind in NewTOS global statt je Prozess; deshalb erbt
   `init_pd_files` sie nicht (`bdos/proc.c` `init_pd_files`, zweite Haelfte).
3. `Fread`/`Fwrite` lesen den Handle als u16; ein negativer BIOS-Handle ergibt
   `EIHNDL` statt Geraetezugriff. AUX/PRN (-2/-3) landen auf der Konsole.
4. Die Dateitabelle hat 16 Plaetze, EmuTOS 75 (`OPNFILES`).
5. `Fdelete` auf eine offene Datei macht alle Handles darauf ungueltig
   (alte Abweichung F6); EmuTOS haelt sie am Leben.

## Arbeit

Jeden Punkt gegen die EmuTOS-Quelle bestaetigen, Spec ergaenzen, 1:1 umsetzen,
Hosttests; Abnahme: QEMU-Reihe wie `measurements/measurement-2026-10-06-proc02/`.

## Punkt 2 erledigt (2026-10-18, QEMU, `59c8eb4c`)

Laufwerk und Verzeichnis je PD wie EmuTOS (Teil des PROC03-Pakets,
Anwenderentscheidung 2026-10-18). Belege
`measurements/measurement-2026-10-18-proc04-curdir/` (rot 19 Fehler → gruen)
und `measurements/measurement-2026-10-18-proc04-curdir-test/` (Tester PASS,
`4f4602aa`: Ordnernavigation in NEWDESK/TeraDesk, Start aus Ordner, QED-fsel).
Accessories: eigener PD bis B5 entschieden. Nebenbefunde: 16 statt 26
Laufwerke (`BLKDEVNUM`/`NUMCURDIR`, vgl. FS12), Pfade auf 63 statt 111 Byte.
Punkte 1, 3, 4, 5 offen.

## Re-Validierung Punkte 1, 3, 4, 5 (2026-10-18, Researcher)

Spec `docs/api/gemdos/_dispatcher.md` §5, `_state.md` §2,
`file/41-fdelete.md`, `3f-fread.md`, `40-fwrite.md`, `3e-fclose.md`.

1. **Bestaetigt, praeziser:** zwei Mechanismen — Datei-Umlenkung
   (`bdosmain.c:498-568`, `stdio_typ` `:150-197`, Crawio ausgenommen) und
   BIOS-Geraet ueber `HXFORM(run->p_uft[n])` (`console.c`). NewTOS ignoriert
   `p_uft` in allen 15 Zeichenfunktionen (`tos-api/src/gemdos/console.rs`).
   Betrifft `system()`/`popen()` der mintlib mit Kindern im TOS-Stil.
3. **Ueberholt** — seit FS11 (2026-10-13) EmuTOS-gleich (`file.rs:231-246,393-455`).
4. **Bestaetigt:** EmuTOS `OPNFILES 75`, `NUMHANDLES 81`, systemweit, ohne
   CONF-Gate (`bdos/fs.h:43-45`); NewTOS 16 (`file.rs:30`).
5. **Bestaetigt, praeziser:** EmuTOS `ixdel` (`fsopnclo.c:652-669`): eigene
   offene Datei → `ixclose`, Handle bleibt gueltig, `Fdelete` = `E_OK`,
   spaeteres `Fclose` = `EINTRN`; fremde → `EACCDN`. NewTOS (F6, `00c8137a`)
   macht alle Handles ungueltig. Keine benannte Abweichung — F6 war eine
   Fehllesung. EmuTOS-Gefahr: Schreiben nach dem Loeschen landet in freien
   Clustern → Anwenderentscheidung.

Nebenfund → [FS15](FS15.md) (`CON:`/`AUX:`/`PRN:` bei `Fopen`/`Fcreate`).
Reihenfolge: 4 → 1 → 5 (nach Entscheidung).

## Anwenderentscheidung Punkt 5 (2026-10-18)

Variante (b): `Fdelete` auf eine eigene offene Datei liefert `E_OK`, der
Handle bleibt gueltig (wie `ixdel`/`ixclose`), auf eine fremde `EACCDN`;
spaeteres Schreiben/`Fclose` ueber den Handle fasst die Karte nicht mehr an
(keine freigegebenen Cluster, kein Verzeichnisslot) — benannte Abweichung in
`docs/COMPATIBILITY.md` beim Bau.

## Punkt 4 erledigt (2026-10-18, QEMU, `5fe11974`)

75 Eintraege, Handles 6..80, `ENHNDL` bei voller Tabelle. Belege
`measurements/measurement-2026-10-18-proc04-files/` (rot → gruen),
`measurements/measurement-2026-10-18-proc04-files-test/` (Tester PASS).

## Anwenderentscheidungen zu Punkt 1 (2026-10-18)

- `Cconrs` aus einer umgelenkten Datei: EmuTOS liest `max` vorzeichenbehaftet
  (`bdosmain.c:535-553`), ab 128 liest es ohne Grenze bis CR/EOF und kann den
  Puffer ueberlaufen. NewTOS stoppt bei `max` (0..255, vorzeichenlos) —
  benannte Abweichung in `docs/COMPATIBILITY.md`; fuer `max` ≤ 127 identisch.
  Umsetzung zusammen mit CON01.
- Zeichenweises Zurueckschreiben bei Umlenkung auf eine Datei → [FS16](FS16.md).

## Punkt 1 erledigt (2026-10-18, QEMU, `1a306120`)

Belege `measurements/measurement-2026-10-18-proc04-redir/` (rot → gruen) und
`measurements/measurement-2026-10-18-proc04-redir-test/` (Tester PASS). Damit
ist auch [CON02](CON02.md) erledigt.
