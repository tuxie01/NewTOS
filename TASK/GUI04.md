# GUI04 — `CONF_WITH_EXTENDED_MOUSE`: Mausrad und Tasten 3 bis 5

Status: PARKED (2026-11-11) | Priority: P2 | Reason: legacy, evidence unverified; receipt GUI04-STATUS-DONE.MD exists | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 19, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: no cited file of the ticket/receipt is touched by 88a9a2b78 (feat(vdi,aes): the mouse wheel scrolls - USB byte ), named by body

Line 3 before the migration:
Status: **ERLEDIGT — auf echter Pi 4 abgenommen** (2026-09-28) · Prioritaet: **P2** · Typ: Funktionsluecke USB/AES + **benannte Erweiterung ueber EmuTOS hinaus** — aus der Schaltertabelle (REF01)

Abhaengigkeiten: —

## Sachverhalt

`CONF_WITH_EXTENDED_MOUSE` ist im Referenzbau **1**
(`references/emutos/include/config.h:1491-1498`): EmuTOS wertet die
IKBD-Scancode-Erweiterung fuer Maustasten 3/4/5 und das **Mausrad** aus
(`references/emutos/bios/aciavecs.S`, `bios/ikbd.c:748`).

`kernel/src/ikbd.rs` entschluesselt nur das Standardpaket (relative Bewegung,
zwei Tasten); `0xF6` gilt dort als Statusbericht-Kopf, eine Rad- oder
Zusatztastenauswertung gibt es nicht.

**Der Punkt, der es interessant macht:** die VDI-Seite ist schon da —
`vex_wheelv` (`tos-api/src/vdi/mouse.rs:710`) kann einen Radvektor
installieren. Es fuettert ihn nur niemand. Ein Programm, das sich dort
eintraegt, wartet bei uns vergeblich.

## Hochgestuft — Lead 2026-09-24

Von P3 auf **P2**: der Benutzer will das Mausrad. Es ist die einzige der fuenf
Luecken aus der Schaltertabelle, die man an einer Vorfuehrung sofort
bemerkt — und die halbe Strecke liegt schon da (`vex_wheelv`).

## Neuzuschnitt 2026-09-25 — die Quelle ist USB, und EmuTOS ist hier nicht fertig

**Der Weg ist USB, nicht IKBD.** Der Anwender: *„natuerlich eine USB-Maus,
eine Atari-Maus haette kein Mausrad."* Die IKBD-Erweiterung bleibt trotzdem
die Form, in der das Rad durch unser System laeuft — `hid.rs:106` uebersetzt
heute schon jeden USB-Mausbericht in ein IKBD-Paket
(`0xF8|Tasten, dx, dy`). Es liest dabei nur die Bytes 0–2; **Byte 3, das
Rad, bleibt liegen.**

**EmuTOS' eigene Radbehandlung ist auskommentiert.** Die Kette dort:
IKBD-Scancode (`bios/ikbd.c:748-760`: `0x59`/`0x5a` hoch/runter, `0x5c`/`0x5d`
links/rechts) → `mousexvec` → AES → `wheel_change` (`aes/geminput.c:395`).
Und dort steht:

```c
/* We have a problem here.
 * This function is called by forker(), where rlr is set to -1.
 * As a result, we can't call ap_sendmsg() which internally calls iasync().
 * Uncomment the following code when some solution has been found.
 */
/*  ap_sendmsg(appl_msg, WM_ARROWED, D.w_win[wh].w_owner, wh, type, 0, 0, 0);  */
```

EmuTOS nimmt das Rad entgegen, rechnet `WA_UPLINE`/`WA_DNLINE` aus — und
wirft es weg. Ein 1:1-Nachbau liefert ein Mausrad, das nichts tut.

**Lead-Entscheidung des Anwenders 2026-09-25: das Rad soll scrollen.** Also
ueber EmuTOS hinaus, und das ist hier ausdruecklich **keine** Drift im Sinne
von [Don't rationalize EmuTOS deviations] — EmuTOS selbst schreibt in den
Quelltext, dass hier eine Loesung fehlt. Wir bauen sie.

## Was bei uns fehlt (gelesen, nicht vermutet)

| Stueck | Stand |
|---|---|
| USB-Bericht Byte 3 (Rad) | `kernel/src/drivers/usb/hid.rs:106` liest `r[0..3]`, Byte 3 ungenutzt |
| IKBD-Erweiterung (Scancodes 0x37/0x59–0x5f) | `kernel/src/ikbd.rs` kennt sie nicht; `0xF6` gilt dort als Statusbericht-Kopf |
| `mousexvec` | fehlt |
| `wheel_change` | fehlt |
| AES traegt sich per `vex_wheelv` ein | fehlt (`aes/gemgsxif.c:291` ist das Vorbild) |
| `vex_wheelv` (Opcode 134) | **vorhanden**, `tos-api/src/vdi/mouse.rs:711` — niemand ruft es, niemand traegt sich ein |

## Auftrag

1. **Die Kette in EmuTOS' Form bauen**, Stueck fuer Stueck mit Zitat:
   USB-Byte 3 → IKBD-Scancode `0x59`/`0x5a` (und `0x5c`/`0x5d`, falls die Maus
   waagerecht liefert) → `mousexvec` → `wheel_change` mit `WA_UPLINE`/
   `WA_DNLINE`. So laeuft spaeter eine echte Rad-Quelle ueber denselben Weg,
   und `vex_wheelv` wird endlich bedient.
2. **Den letzten Schritt loesen**, den EmuTOS offengelassen hat: `WM_ARROWED`
   muss beim Fenster unter dem Mauszeiger ankommen. EmuTOS' Grund ist
   benannt (`forker()`, `rlr == -1`, `ap_sendmsg` ruft `iasync`); bei uns
   sieht der Weg vom Eingabepfad zur Nachrichtenzustellung anders aus —
   **erst pruefen, ob dieselbe Einschraenkung bei uns ueberhaupt gilt**, und
   das Ergebnis aufschreiben. Falls ja, ist eine andere Zustellform zu
   entwerfen und zu begruenden.
3. Die Abweichung **benennen**: ein Kommentar an `wheel_change`, der sagt,
   was EmuTOS an dieser Stelle tut, warum wir weitergehen und wie.

## Abnahme

Auf der **echten Pi** mit der USB-Maus: im Desktopfenster mit dem Rad
scrollen, Inhalt bewegt sich. Dazu ein Testprogramm, das sich per
`vex_wheelv` eintraegt und die Radereignisse sieht — das ist der Beleg, dass
die Kette und nicht nur der Desktop bedient wird. Menue aktiv ⇒ Rad wird
ignoriert (`geminput.c:401-402`).

## Arbeit

Die Scancode-Erweiterung in `ikbd.rs` nachziehen und an `vex_wheelv` binden.
Abnahme braucht echte Hardware mit Rad-Maus; in QEMU ist der Pfad nur bis zur
Dekodierung pruefbar.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

## Abschluss (2026-09-26)

Kette gebaut, EmuTOS-foermig, je Station mit Zitat: USB-Byte 3 →
IKBD-Scancode `0x59`/`0x5a` → `mousexvec` → `vdi_mousex_handler` →
`call_user_wheel` → entweder AES `wheel_change` → `WM_ARROWED` oder
EL0-Upcall an einen per `vex_wheelv` eingetragenen Vektor.

**Auftrag 2 beantwortet, und die Antwort ist Nein:** EmuTOS' Einschraenkung
gilt bei uns **nicht**. Sein Grund ist `forker()`, das fuer die Dauer des
Fork-Rings `rlr = (AESPD *) -1` setzt (`aes/gemdisp.c:101-106`), und
`ap_sendmsg` dereferenziert genau diesen Zeiger (`gemwmlib.c:855`). Bei uns
gibt es **keinen Fork-Ring** (kein `forker`/`fpdx` im Baum), `pump_input`
laeuft ausschliesslich synchron aus EL1-Code mit eindeutigem Aufrufer,
`pd::post_msg` bekommt `msg[1]` als Argument statt `rlr->p_pid` zu lesen, und
es dispatcht nicht (es reiht ein und weckt). Es war also **keine andere
Zustellform zu entwerfen** — die gewoehnliche
`ap_sendmsg` → `aqueue` → `doq`-Strecke traegt es, samt `doq`s
`WM_ARROWED`-Koaleszenz. Die Abweichung steht als Block-Doc an `wheel_change`.

**Nebenbefund, mitrepariert, weil er genau auf dieser Strecke liegt:**
`vex_wheelv(handle, new, &old)` schrieb `old` an CONTRL-Wort **9** (m68k), eine
LP64-Anwendung liest es bei **11** (`N_PTRINTS = 4`, `gem_vdiP.h:177,248`).
Jede Anwendung haette Muell als „alten Vektor" bekommen — auf genau dem Weg,
den dieses Ticket erst benutzbar macht. Behoben ueber `Vdipb::set_contrl_ptr`,
betrifft `vex_butv/motv/curv/timv` gleichermassen.

**Gemessen:** Lauf A — `WHEELT.NAP` per `#Z`-Autorun, acht synthetisierte
Berichte: `up=3 down=4 total=7`, der eine Bericht mit `len==3` erzeugt
**nichts**, `old` kommt im richtigen LP64-Slot zurueck. Lauf B — Desktop,
Textfenster, 6x Rad runter: genau sechs Zeilen weiter, Schieber gewandert.
Lauf C — dasselbe mit offenem Klappmenue: **0** unterschiedliche Pixel, der
`gl_mowner == ctl_pd`-Guard haelt (`geminput.c:401-402`). **Lauf D ist nicht
synthetisiert:** QEMUs echte `usb-mouse` hinter echtem xHCI auf `-M virt`,
Rad per QMP — `report len=4 byte3=0x01 → 0x59` (3x), `byte3=0xff → 0x5a` (4x).
Damit ist auch der xHCI-Sprung gemessen.

Basislinien: 955/0/1 (950 + 5), 64/94 beide Boards, 80/125 beide Feature-Saetze.

**Bewusst nicht geraten:** das **waagerechte** Rad. `0x5c`/`0x5d` sind im
`mousex_handler` implementiert, werden aber nicht aus USB erzeugt — ein
fuenftes Berichtbyte waere ueblicherweise AC Pan, das steht nicht im
Boot-Protokoll, und FreeMiNTs Boot-Maustreiber liest es nicht. Es zu wissen
braeuchte den HID-Report-Descriptor, den das Boot-Protokoll gerade vermeidet.
Festgehalten in `docs/api/usb/hid-mouse.md`.

**Was nur die echte Pi entscheidet:** ob eine physische Radmaus hinter dem
VL805 Byte 3 mit Laenge 4 liefert (fuer QEMUs Maus beantwortet Lauf D das),
und die Haptik — Rastungen je Zeile.

## Hardware-Abnahme (2026-09-28)

Kern `92ec0ab7`, echte USB-Maus. Anwender: *„Mausrad funktioniert."* Damit ist
die Frage beantwortet, die nur die Pi entscheiden konnte: eine **physische**
Radmaus hinter dem VL805 liefert Byte 3 mit Laenge 4, und die Kette
USB → IKBD-Scancode → `mousexvec` → `wheel_change` → `WM_ARROWED` traegt bis
ins Fenster.

Weiterhin **nicht** gebaut und benannt: das waagerechte Rad (kein fuenftes
Berichtbyte im Boot-Protokoll).
