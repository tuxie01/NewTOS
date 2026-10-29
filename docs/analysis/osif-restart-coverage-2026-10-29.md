# osif-Neustart nach Medienwechsel: Abdeckung (2026-10-29)

Frage: FS20 Option A/B (Stand 2026-10-29, `TASK/FS20.md`). Pin: EmuTOS 1a7484ab
(`docs/analysis/ORACLE.md`). READ = Quelle gelesen, DERIVED = geschlossen, MEASURED = keine
neue Messung in dieser Notiz. Nur Fakten, keine Empfehlung.

## 1. osif: setjmp, longjmp-Quellen, Medienwechsel-Block, Neustart

- READ: Funktionsanfang `restrt: fn = pw[0];` und direkt danach `if (setjmp(errbuf))`:
  `references/emutos/bdos/bdosmain.c:442-449`. `errbuf` ist global (`bdos/fsglob.c:48`),
  deklariert in `bdos/fs.h:56`.
- READ: osif wird nur von `bdos/rwa.S:227` (`jsr _osif`) aufgerufen, also nur aus dem
  trap-#1-Eintritt.
- READ, longjmp-Quellen (alle landen in `errbuf`):
  - Makro `longjmp_rwabs` (`bdos/fs.h:60-77`): `Rwabs` != 0 -> `errdrv=dev; errcode=rwerr; longjmp(errbuf,1)`.
    Benutzt von `bdos/fsbuf.c:95,100` (flush), `:165` (getbcb, Lesen), `bdos/fsio.c:73` (Daten-I/O).
  - `bdos/fsbuf.c:179-190` (getbcb, Puffer-Treffer): `Mediach(drv)==2` -> `rwerr=E_CHNG; longjmp`;
    Ergebnis 1 -> nur `goto doio` (neu lesen, kein Neustart).
  - `bdos/fsdrive.c:107-114` (`ckdrv(d, checkrem)`): bei `checkrem` und `mask & drvrem` und
    `Mediach(d)` -> `rwerr=E_CHNG; longjmp`. `ckdrv(..., TRUE)` rufen `bdos/fsdir.c:1715` (Pfadaufloesung)
    und `bdos/fsfat.c:345`; `fsdir.c:1237` ruft mit FALSE (kein Mediach).
- READ, Block `if (rc == E_CHNG)` (`bdosmain.c:455-486`):
  1. `dmd = drvtbl[errdrv]; dn = dmd->m_dtl; offree(dmd); xmfreblk(dmd); drvtbl[errdrv] = NULL;`
  2. `offree` (`:384-402`): jedes SFT-Slot mit `f_ofd->o_dmd == d` -> OFD freigeben,
     `f_ofd=NULL; f_own=NULL; f_use=0`. Alle offenen Handles des Laufwerks sind danach tot
     (naechster Zugriff Fehler).
  3. `if (dn) freetree(dn);` (`:340-372`): DND-Baum rekursiv, OFDs der DNDs freigeben,
     Eintraege in `dirtbl` (die aktuellen Verzeichnisse aller Prozesse) mit `p->dnd == d` auf
     `dnd=NULL; use=0`. `run->p_curdir[]` selbst bleibt unveraendert; `ckdrv` (`fsdrive.c:132-142`) legt bei
     `!dirtbl[curdir].dnd` ein Wurzel-DND neu an. Aktuelle Verzeichnisse fallen also auf Wurzel zurueck.
  4. `mark_bcbs_invalid(errdrv)` (`:410-423`): `b_bufdrv = -1` fuer alle BCBs des Laufwerks (ungeschriebene
     Dirty-Daten gehen verloren).
  5. `b = Getbpb(errdrv)`; `!b` -> `drvsel &= ~(1L<<errdrv); return rc` (E_CHNG an den Aufrufer, kein Neustart).
  6. `log_media(b, errdrv)` Fehler -> `drvsel &= ~..; return ENSMEM`.
  7. `rwerr = 0; errdrv = 0; goto restrt;`
- READ, Neustart: `goto restrt` springt VOR `setjmp`, liest `fn = pw[0]` neu. `pw` zeigt auf die
  unveraenderten Aufrufparameter auf dem Stack (`rwa.S:222-227`); Funktionsnummer und alle
  Argumente werden also identisch wiederholt, der gesamte BDOS-Aufruf laeuft von vorn. Kein weiterer
  Zustand wird zurueckgesetzt ausser `rwerr`/`errdrv`. Eine Schleife ohne Begrenzung: jeder erneute E_CHNG
  startet wieder neu.
- READ, anderer Fehlercode (kein E_CHNG): `mark_bcbs_invalid(errdrv); return rc;` (`:489-493`).
- Das Neu-Einloggen geschieht also innerhalb von osif; ausserhalb von trap #1 (Aufruf von `osif`) gibt es keinen
  setjmp-Fang (siehe 3. fuer den einzigen verschachtelten).

## 2. Gehen EmuTOS-AES und Desktop ueber trap #1?

- READ: Die Bindungen sind Inline-Wrapper `include/gemdos.h:55-100` (`dos_gdta`, `dos_sfirst`,
  `dos_open`, `dos_close`, `dos_read`, `dos_write`, `dos_lseek`, `dos_chdir`, ...) auf Makros
  `include/bdosbind.h:38-50` (`Fopen -> trap1(0x3d,...)`, `Pexec -> trap1_pexec(...)`). `trap1` und
  `trap1_pexec` (`util/miscasm.S:21-27, 50-66`) fuehren `trap #1` aus. Dort gibt es keinen direkten
  C-Aufruf nach `bdos/`; jeder dieser Aufrufe betritt `osif` neu (`rwa.S:227`) und steht unter dessen setjmp.
- READ, je Aufrufer (alle ueber `dos_*`/`Fxxx`):
  - rsrc_load: `aes/gemrslib.c:839` `dos_open`, `:758-782` `dos_lseek`/`dos_read`, `:847` `dos_close`.
  - shel_find: `aes/gemshlib.c:414-462` `dos_sdta`/`dos_sfirst`. Programmstart: `:579,645` `dos_exec`
    (PE_GOTHENFREE bzw. PE_LOADGO).
  - fsel: `aes/gemfslib.c:285-310` `dos_gdta/sdta/sfirst/snext`.
  - Desktop-INF lesen: `desk/deskapp.c:688` `dos_load_file`, `:1520` ebenso; `util/gemdos.c:153`
    `dos_open`/`dos_read`/`dos_close`. INF schreiben: `desk/deskapp.c:1054-1059` `dos_create`/`dos_write`/`dos_close`.
  - Desktop Kopieren/Anzeigen: `desk/deskdir.c:402-494`, `desk/desksupp.c:623-656,777-811`.
  - AES-Start `aes/geminit.c:247` `dos_open`.
- READ, kritischer Fehler: `criterr_handler` (`aes/gemdosif.S:153-212`) ist der `etv_critic`-Handler
  (installiert `:101-131`). Er ruft `eralert` (Form-Alert) und gibt Retry/Abbruch zurueck; er wird
  aus dem BIOS (Rwabs-Pfad) aufgerufen, nicht aus osif, und ist fuer Fehler, nicht fuer E_CHNG-Neuanmeldung zustaendig.
  Ob/wann das BIOS E_CHNG vor dem Critic-Aufruf meldet, ist hier nicht untersucht (Out-of-scope, BIOS-Seite).
  Vor `enable_ceh` (Desktop-Hauptschleife) wird an den alten Handler weitergereicht (`:154-158`).
- DERIVED: Fuer EmuTOS gilt damit: Es gibt in AES/Desktop keinen Datei-Zugriff, der den osif-Neustart umgeht.
  Die Frage "Option B (nur gemdos::dispatch)" ist in EmuTOS gar nicht gestellt: alles laeuft durch den Dispatcher.

## 3. Pexec

- READ: `{ F(xexec), 0, 7 }` (`bdos/bdosmain.c:271`): Pexec (0x4B) laeuft als normaler Funktionsaufruf in
  osif, also unter osif-setjmp. Siehe auch `trap1_pexec` (2.).
- READ, `xexec` (`bdos/proc.c:179-359`), Pfad PE_LOAD/PE_LOADGO:
  - `ixsfirst` (`:256`), `xopen` (`:262`), `kpgmhdrld` (`:272`): longjmp hier landet direkt in osif; Kommentar
    `:271-274` "we haven't allocated anything yet". Das offene Handle `fh` ist zu diesem Zeitpunkt aber offen.
  - Danach alloc env/TPA (`:277-301`), `memcpy(bakbuf, errbuf, ...)` und **eigenes** `setjmp(errbuf)`
    (`:319-320`, `static jmp_buf bakbuf` `:59`). Handler `:322-332`: `xmfree(cur_p->p_env); xmfree(cur_p);
    xclose(fh); longjmp(bakbuf,1);` also Aufraeumen und Weiterwurf in osifs gesicherten Puffer.
  - `kpgmld` (`:338`) laeuft unter diesem verschachtelten setjmp. Nach dem Handler landet der Wurf in osif
    (`bakbuf` = Kopie des osif-`errbuf`, genauer: ein jmp_buf mit gleichem Inhalt, der Sprung geht in den
    Stackrahmen von osif). Ein `rc != 0` aus kpgmld gibt env/PD frei und liefert den Fehler (`:339-343`).
  - Erfolg: `init_pd_files` (`:350`), danach `proc_go` bei PE_LOADGO (`:358`). Nach Erfolg bleibt `errbuf` auf den
    xexec-setjmp gesetzt; ein spaeterer osif-Aufruf macht wieder `setjmp(errbuf)` (`bdosmain.c:449`).
- READ/DERIVED, Wirkung der Neustart-Semantik fuer Pexec:
  - Wurf in der Header-/Erstphase (vor Zeile 319): landet direkt in osif; E_CHNG -> `offree` schliesst das
    Handle (SFT-Slot freigegeben), Neustart `xexec` von vorn. Kein Speicher allokiert. Fuer andere Fehler als E_CHNG
    ohne `offree` bleibt `fh` offen (DERIVED aus `bdosmain.c:489-493` und `bdos/proc.c:262-274`: kein xclose in diesem Pfad).
  - Wurf in der Ladephase (nach 319): xexec gibt env und Basepage frei, schliesst `fh` (noch vor `offree`; das
    Handle ist gueltig, weil `offree` erst im osif-Block laeuft), wirft weiter nach osif; Neustart beginnt ohne
    Reste. Kein Nebeneffekt bleibt (READ `:322-332`).
  - Nebeneffekte eines zweiten Durchlaufs: `alloc_env`/`alloc_tpa` werden im Neustart neu ausgefuehrt, `p_curdir`/
    DTA des Aufrufers bleiben, aber `ixsfirst` (`:256`) schreibt in die DTA des Aufrufers (Pexec-interne DTA-Wirkung ist
    nicht geprueft; "needs deeper analysis").
  - PE_LOADGO: `proc_go` kehrt nicht zurueck; I/O-Fehler danach laufen im Kontext des neuen Prozesses (eigene osif-Aufrufe).
- Verschachtelte setjmp insgesamt: nur xexec (`bdos/proc.c:320`) in `bdos/`; sonst keine (`grep setjmp bdos/*.c`:
  `bdosmain.c:449`, `bdos/proc.c:320`).

## Was das fuer Option A vs. B bedeutet (Fakten)

- In EmuTOS gibt es keinen Dateizugriff von AES/Desktop/Pexec-Lader, der ausserhalb des osif-setjmp laeuft; es
  gibt genau eine Neuanmelde-Stelle (`bdosmain.c:455-486`) und sie deckt jeden Aufruf durch trap #1 ab.
- Option A (kernelinterne Leser rufen ebenfalls neu an) entspricht dem EmuTOS-Verhalten fuer alle genannten Leser,
  weil dort jeder Leser einen vollen BDOS-Aufruf macht, der neu startet. Option B (nur `gemdos::dispatch`) deckt
  in NewTOS nur die Aufrufe ab, die durch dieselbe Stelle gehen; Leser, die daran vorbeigehen, haben das EmuTOS-Verhalten
  nicht (DERIVED; NewTOS-Seite hier nicht gelesen).
- Der Neustart wiederholt den kompletten Aufruf mit unveraenderten Argumenten; Aufrufe mit Teilzustand (Pexec) haben
  einen eigenen Aufraeumpfad (`bdos/proc.c:322-332`), nur die Header-Phase hinterlaesst bei Nicht-E_CHNG-Fehlern ein offenes Handle.
- Offen: Verhalten von `ixsfirst`-DTA und E_CHNG-Meldung aus dem BIOS vor dem Critic-Handler nicht untersucht.
