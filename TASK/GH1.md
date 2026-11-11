# GH1 — Popup im System-Info-Accessory: Fenster dahinter zeichnet sich ueber den Dialog

Status: PARKED (2026-11-11) | Priority: P2 | Reason: legacy, evidence unverified; no receipt | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 17, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: no cited file of the ticket/receipt is touched by 7b295ab3c (fix(aes): make wind_update a real mutex, route cli), named by body

Line 3 before the migration:
Status: **ERLEDIGT (QEMU) 2026-10-21** (`7b295ab3`; Pi abgenommen 2026-10-22, `docs/HW-TEST-LOG.md`) · Prioritaet: **P2** · Typ: Fehler — GitHub-Issue #1 (ragnar76, 2026-10-19), echte Pi 4, NewTOS 0.6 (`f7cb0fd9`)

## Meldung

1. Accessory „NewTOS System Information“ (SYSACC) ueber das Desk-Menue oeffnen.
2. Im Dialog ein Popup-Menue oeffnen (Auswahlboxen „Next start“).
3. Das Verzeichnisfenster dahinter (`C:\ACC\`) zeichnet sich ueber den Dialog.
   Erwartet: Fenster bleibt im Hintergrund. Immer reproduzierbar, System laeuft weiter.

Foto: `measurements/measurement-2026-10-19-gh1/issue1-photo.png`
(https://github.com/tuxie01/NewTOS/issues/1).

## Auftrag (Ursache zuerst, kein Fix)

In QEMU raspi4b mit Injektor nachstellen; Ursache mit Beleg (Datei:Zeile)
gegen EmuTOS: wer schickt welche Nachricht (WM_REDRAW?) beim Oeffnen/Schliessen
des Popups, warum zeichnet der Desktop ueber den Dialog (Rechteckliste,
`wind_update`-Sperre, `form_dial`), was tut SYSACC (`sdk/examples/sysacc.c`),
und ob A112 (Abgabe nach jedem AES-Aufruf, `1762e837` alt) die Ursache ist —
Gegenprobe mit einem Kernel vor A112.

[Gemeinsame Regeln](README.md) gelten.


## Ursache (2026-10-19, Auditor, QEMU nachgestellt)

Belege `measurements/measurement-2026-10-19-gh1/RUNINFO.TXT`. Nicht SYSACC,
nicht A112 (vor A112 dasselbe Bild).

1. AES `post_button` erfuellt die wartenden Knopf-Events **aller** Prozesse
   statt nur des Maus-Besitzers (`tos-api/src/aes/input.rs:499-552`; EmuTOS
   `aes/geminput.c:463-483`). Jeder Klick im Accessory-Dialog erreicht auch
   NEWDESK. Spec `docs/api/aes/event.md:524-526` nennt das als Naeherung.
2. AES `wind_update` blockiert nie, und die Menue-Steuerung haelt keinen Lock
   bis zum Loslassen (`window.rs:4167-4195`, `event.rs:158-238`; EmuTOS
   `gemflag.c:68-129`, `gemwmlib.c:1778-1797`, `gemctrl.c:463,484`) — deshalb
   laeuft `fm_own` des Accessorys bei gedrueckter Taste, der Maus-Besitz geht
   nicht ans Accessory. Spec `window-graphics.md:1132-1135`,
   `accessories.md:873` („nothing observes it“) widerlegt.
3. NEWDESK: `wind_update(BEG/END_UPDATE)` um den Event-Handler fehlt
   (`programs/desktop/src/main.rs:84-107`; EmuTOS `desk/deskmain.c:2094,2121`).
4. AES `wind_set(WF_INFO/WF_NAME)` zeichnet den ganzen Rahmen statt nur des
   Elements (`window.rs:3976-3983,4122`; EmuTOS `gemwmlib.c:1668-1674,1754-1757`).

Fix-Vorschlag: F1 echter `wind_update`-Mutex (Wartende schlafen, Menue haelt
den Lock bis Loslassen), F2 `post_button` nur an den Maus-Besitzer, F3 Klammer
in NEWDESK, F4 Element-Neuzeichnen — F1–F3 nur zusammen (F2 allein verschiebt
das Symptom, im Lauf belegt).


## Fix (2026-10-21, `7b295ab3`, Anwender-Freigabe F1–F3)

F1 echter `wind_update`-Mutex (Wartende parken, FIFO-Uebergabe; Control
Manager haelt die Sperre als `ctl_pd` bis zum Loslassen, auch beim Druck,
der ein Drop-down ueber dem Top-Fenster abbricht — `aes/gemctrl.c:81-102`),
F2 `post_button` nur an den Maus-Besitzer, F3 BEG/END_UPDATE in NEWDESK.
F4 nicht umgesetzt, jetzt [A119](A119.md). Specs vorher korrigiert (`window-graphics.md`,
`event.md`, `accessories.md`, `menu.md`).

Belege: `measurements/measurement-2026-10-21-gh1/` (Umsetzung),
`measurements/measurement-2026-10-21-gh1-reg/` (Tester: FAIL — QED hing nach
Menue + Klick ins Fenster), `measurements/measurement-2026-10-21-gh1-fix2/`
(Ursache + Fix der Regression; Desktop-Serie 40/47 gleich HEAD, Rest = GH1
und Cursor-Blinkphase). Hosttests 1504/0/1.

Gewollt (EmuTOS): ein Klick, der ein Drop-down abbricht, kommt nicht mehr
als MU_BUTTON bei der Anwendung an.

Offen:
- ~~Echte Pi 4~~ abgenommen 2026-10-22 (Anwender-Kurztest, `docs/HW-TEST-LOG.md`).
- Benannte Restabweichungen (in der Spec): `wm_open`/`wm_close`/`wm_set`/
  `w_redraw_desktop`, `graf_rubberbox`/`graf_dragbox`, `ap_exit` und `fsel`
  nehmen die Sperre nicht; `fm_error` aus der Shell-Schleife kann nicht
  parken; Quick-Check in `ev_multi` fragt `own_mouse` statt „Aufrufer“.
- Nebenbefund: `menu_intercept` stellt MN_SELECTED an die aktuelle Identitaet
  statt an `gl_mnppd` — laeuft der Control Manager im `evnt_multi` eines
  Accessorys, landet eine Desktop-Menuewahl dort.
