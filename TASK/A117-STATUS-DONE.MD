# A117 — STATUS: DONE (2026-10-25, QEMU) — MN_SELECTED geht an den Besitzer der Menueleiste

**Auftrag:** Wirkung belegen, Stelle gegen EmuTOS, Fix nach EmuTOS mit Hosttest.

## Wirkung (vor dem Fix, HEAD `c4e8d1be`)

Sonden-Accessory `measurements/measurement-2026-10-25-a117/method/a117acc.c`
wartet in `evnt_multi(MU_MESAG)` und meldet jede Nachricht seriell; es besitzt
keine Menueleiste. QEMU raspi4b, IKBD-Injektor:

| Fall | vorher | nachher |
|---|---|---|
| NEWDESK: Desk → A117 probe, dann Desk → Desktop info | `MN_SELECTED` 3/9 beim **Accessory**, kein Dialog, Titel bleibt invers (2/2 Laeufe); zweiter gleicher Klick erreicht den Desktop | Dialog bei beiden Klicks, Accessory bekommt nur `AC_OPEN` (2/2) |
| TeraDesk: Desk → A117 probe, dann File → Quit/Shutdown | `MN_SELECTED` 4/32 beim **Accessory**, kein Quit-Dialog, „File“ bleibt invers (3/3) | Dialog „Quit / Shutdown Options“ (3/3) |

## Stelle

* EmuTOS `aes/gemctrl.c:353` `owner = gl_mnppd;`, `:365` `ct_msgup(mesag,
  owner, …)` → `:83-84` `ap_sendmsg`; `aes/gemmnlib.c:707,712` `gl_mnppd = p`
  (Aufrufer von `menu_bar(tree, 1)`). Der Control Manager ist dort ein eigener
  Prozess, der Wartende entscheidet nie.
* NewTOS vorher: `tos-api/src/aes/event.rs` `menu_intercept(idx)` stellte an
  `idx` = Aufrufer des `evnt_multi`, in dem der Inline-Control-Manager lief —
  auch ein Accessory. `MNPPD` war schon gespeichert, aber nicht Ziel.
* Spec: `docs/api/aes/menu.md` und `_state.md` nannten `gl_mnppd` richtig,
  schwiegen aber zum Inline-Modell; ergaenzt (menu.md NewTOS-Bindung A117,
  event.md A91-Abschnitt).

## Fix

`menu::mn_selected_target()`: `MNPPD` → pid seines Deskriptors → lebendes
Mitglied, wie `appl_write` eine pid adressiert (`identity_of(reported_pid)`):
Kette wartet selbst → der Wartende (ein Deskriptor `D.g_int[0]`); Accessory
wartet → innerstes gestartetes Kettenmitglied; Accessory-Leiste → dieses
Accessory. `AC_OPEN` unveraendert.

Hosttest `tos-api/tests/aes_a117_mn_selected_owner.rs`: Faelle 1 und 2 rot auf
dem alten Code (Accessory bekam `MU_MESAG` mit `[10,1,0,4,12,…]`), Fall 3
(Pexec-Kind des Leistenbesitzers) auf beiden gruen; alle drei gruen mit Fix.

## Belege

`measurements/measurement-2026-10-25-a117/RUNINFO.TXT` (Bau/Lint/Hosttests: beide Boards dev+release rc 0, Warnungen 159/76/121 unveraendert, `cargo xtask test` 1636/0/1 → 1639/0/1;
Basis gegen neu dort). Commits: Fix + Test + Spec `eaf32b04`, Beleg + Doku im Folgecommit.

## Verhaeltnis zu GitHub #14

Gelesen (nur lesend, `gh issue view 14`): „Programm laesst sich manchmal
nicht beenden, nachdem ein Accessory geoeffnet und geschlossen wurde“, echte Pi,
SYSACC und X-Shot; ragnar76 bestaetigt ohne Details.

* **Abgedeckt — fuer ein Accessory, das in `evnt_multi(MU_MESAG)` wartet.**
  Der TeraDesk-Fall ist genau das Symptom: Accessory geoeffnet, dann
  *File → Quit/Shutdown* — keine Reaktion, „File“ bleibt invers; die Wahl liegt
  in der Queue des Accessorys (`serial/base-td3..5.log`). „Manchmal“ passt: es
  haengt davon ab, wessen Warteschleife den Control Manager gerade faehrt (im
  NEWDESK-Fall ging die erste Wahl ans Accessory, die zweite an den Desktop).
* **Nicht abgedeckt — SYSACC allein.** SYSACC wartet in `evnt_mesag`
  (`sdk/examples/sysacc.c:1293`); dort laeuft der Inline-Control-Manager nicht
  (`tos-api/src/aes/event.rs` `evnt_mesag` ruft kein `menu_intercept`, nur
  `evnt_multi` Zeile 643). SYSACC kann diese Wirkung also nicht ausloesen; ein
  #14-Fall nur mit SYSACC hat eine andere Ursache oder ist nicht erklaert.
* **X-Shot: offen.** Quelle liegt nicht im Repo; ob es in `evnt_multi(MU_MESAG)`
  wartet, ist hier nicht belegt (Fensteraccessory, daher wahrscheinlich, aber
  ungeprueft). Bestaetigung nur auf der echten Pi mit X-Shot.

#14 bleibt offen, bis die Pi-Gegenprobe (X-Shot, Programm beenden) vorliegt.

## Offen

* Echte Pi 4 (keine Hardwareaktionen): TeraDesk/QED + X-Shot bzw. Sonde,
  Accessory oeffnen/schliessen, dann Beenden.
* Nicht gelaufen: Accessory mit eigener Menueleiste (nur per Konstruktion
  abgedeckt).
* Ausserhalb (nicht angefasst): `hctl_rect` ruft fuer `AC_OPEN` zusaetzlich
  `perform_untop(gl_wtop)` (`aes/gemctrl.c:343-344`); NewTOS' AC_OPEN-Zweig in
  `menu_intercept` tut das nicht — eigener Ticketkandidat, nicht geprueft.
  `docs/api/aes/_state.md:227-230` „Single-app note“ (DA-Zustand „inert“) ist
  veraltet.
