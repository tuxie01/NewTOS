# DESK13 — Reste aus DESK02: Kopier-/Ueberschreibruecksprache, `Supexec`, Datumstrenner, `_IDT`

Status: IMPLEMENTED (2026-11-11) | Priority: P3 | Commit: 43e257776 | Missing: hardware acceptance

Migrated (owner batch 15, 2026-11-11, to IMPLEMENTED), evidence: receipt TASK/DESK13-STATUS-DONE.MD exists (legacy DONE receipt, IMPLEMENTED accepted by TOOL55), commit 43e257776 (feat(desk): copy and overwrite confirmations, and one date rule), VERIFIED: 43e257776 (feat(desk): copy and overwrite confirmations, and one date r) names the id by body, its message names the ticket-title identifier desk02, touches code (20 files), legacy line: ERLEDIGT — HW-Abnahme offen

Line 3 before the migration:
Status: **ERLEDIGT — HW-Abnahme offen** · Prioritaet: **P3** · Typ: Funktionsluecken — Nebenbefunde DESK02

Abhaengigkeiten: [DESK02](DESK02.md) (erledigt)

## Die Stellen

1. **Kopier- und Ueberschreibruecksprache fehlen.** EmuTOS fragt in `d_doop`
   (`references/emutos/desk/deskdir.c:939`) und beim Ueberschreiben
   (`:325`); `kernel/src/desk/dir.rs` hat beides nicht und ueberschreibt
   immer (`Fcreate` kuerzt). Deshalb fehlen zwei Felder in
   `Set preferences` (DESK02 liess sie bewusst weg statt tote Knoepfe zu
   bauen).
2. **`Supexec` (XBIOS 0x26) ist nicht implementiert**
   (`tos-api/src/xbios/mod.rs:129`) — damit fehlen die `Default`-Knoepfe
   beider Formatgruppen, die EmuTOS ueber den `_IDT`-Keks fuellt
   (`desksupp.c:219`).
3. **Datumstrenner**: `win.rs` `format_fnode` druckt `MM-DD-YY`, EmuTOS nimmt
   an beiden Aufrufstellen `DATEFORM_SEP` = `/` (`desk/deskbind.h:64`).
4. **Der `_IDT`-Keks traegt `0x112F`** (24H, DMY, `/`) und widerspricht dem,
   was die Textansicht druckt (MDY, `-`) — Quelle der Wahrheit festlegen.
5. `check_alt_letter_key` ist auf der aktuellen Tastaturschicht wirkungslos
   (seit Task #8 vermerkt); eine der DESK11-Aufrufstellen liegt dadurch brach.

## Arbeit

1 und 2 je als eigener Schritt mit Test; 3/4 zusammen entscheiden (ein
Trenner, eine Quelle); 5 pruefen, ob die Tastaturschicht das inzwischen
kann.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

## Abschluss

**2026-09-23.** Alle fuenf Punkte bearbeitet; vier sind Code, einer ist eine
widerlegte Behauptung.

1. **Kopier- und Ueberschreibruecksprache — gebaut.** `ADCPYDEL` in `dir_op`
   (`deskdir.c:911-965,1051-1056,1075-1080`), `ADCPALER` in `output_fname`,
   aufgerufen aus `d_dofcopy` (`deskdir.c:290-397,417-424`). EmuTOS'
   `more`-Tristatus ist `More::{Ok,Skip,Stop}` (plus `More::Failed` fuer den
   E/A-Fehler, den NewTOS sammelt statt zu erfragen). Weil der Kasten dort
   steht, wo EmuTOS ihn hat, **entfallen** die beiden NewTOS-Ersatzalerts in
   `fun_del` und `fun_file2desk`: beide zitierten `deskdir.c:906`, also die
   jetzt wirklich implementierte Stelle. Die drei fehlenden `ADSETPRE`-Felder
   sind zurueck in `inf::inf_pref`, in EmuTOS' Reihenfolge und unter EmuTOS'
   Objektnummern.
2. **`Supexec` — Befund, kein Code.** XBIOS 0x26 bleibt ungebaut. EmuTOS
   braucht es nur, weil die Keksdose auf m68k hinter `$5A0` im
   Supervisor-Speicher liegt (`desksupp.c:219`); `docs/api/xbios/26-supexec.md`
   haelt die Entscheidung offen (ADR-Supexec) und
   `docs/api/bios/_cookie-jar.md` §Status 2026-09-14 benennt den Ersatz selbst
   ("There is no `Supexec` in the kernel"). Der `Default`-Knopf braucht also
   **keinen** anderen Weg als den, den der Desktop ohnehin nimmt: GEMDOS
   `Ssystem(S_GETCOOKIE, '_IDT', 0)` ueber `svc #1`, gebunden als
   `dosbind::dos_getcookie`. Gemessen: `[desk13] _IDT = 0x112f via
   Ssystem(S_GETCOOKIE)`.
3. **+ 4. Datumstrenner und `_IDT` — eine Entscheidung.** Der `sep`-Parameter
   von `fmt_date` ist geloescht; der Trenner ist `DATEFORM_SEP`, ausser im
   `DATEFORM_IDT`-Zweig (`deskinf.c:129,141-143`). `DateFmt::rendered(idt)`
   liefert jetzt Reihenfolge **und** Trenner und deckt alle vier
   `_IDT`-Ordnungen ab (`biosdefs.h:168-171`). Beide Leser stimmen ueberein:
   Textansicht `23/09/26`, Show-Info `23/09/2026`.
5. **`check_alt_letter_key` — Behauptung widerlegt.** `keymap::to_ascii` nullt
   ein alphabetisches Ergebnis, sobald Alt gesetzt ist, und beide Quellen
   (IKBD `0x38`, USB-HID LAlt **und** RAlt) setzen das Bit; der AES reicht das
   Wort unveraendert als `EV_KRET` durch. `Alt`+`C` oeffnet `C:\` — in QEMU
   raspi4b gefahren. Der falsche Vermerk stand an drei Stellen; alle drei sind
   richtiggestellt.

**Ein Befund beim Hinsehen:** die drei Knoepfe des `ADCPALER` ueberlappten sich
um 23 px (die 90-px-Breite des Zwei-Knopf-Kastens war uebernommen und der
mittlere zentriert). Die Kastenbreite kommt jetzt aus der Knopfreihe.

**Pruefung:** Hostsuite **938 / 0 / 1** (934 + 4 neue), beide Boards frisch
gebaut `tos-api 64 / kernel 94` (Delta 0), Clippy `80 / 125` auf beiden Boards
(Delta 0), null Treffer in `kernel/src/desk/`. Sieben QEMU-raspi4b-Laeufe
(640x480, `-display none`, QMP, Injektor), 24 Screendumps, `fsck_msdos -n` auf
allen sieben Kartenabbildern sauber, 0 `[EXC]`/`[PANIC]`.
Belege: `measurements/measurement-2026-09-23-desk13/`.

**Offen:** Pi-Sichtabnahme (HW-Punkt 42 in
`docs/api/desktop/_hw-test-2026-09-17.md`).

**Als eigene Tickets gemeldet:** die Ordner-Namenskonfliktfrage
(`output_path`/`get_new_name`, `deskdir.c:700-779` — NewTOS mischt weiter) und
die Zusatzfrage beim Loeschen einer ganzen Platte (`STDELDIS`,
`deskdir.c:971-975` — braucht `icontype` in `dir_op`).
