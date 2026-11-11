# DESK34 — Ein Unterverzeichnis oeffnet ein neues Fenster statt im selben zu navigieren

Status: PARKED (2026-11-11) | Priority: P1 | Reason: legacy, evidence unverified; receipt DESK34-STATUS-DONE.MD exists | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 16, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: no cited file of the ticket/receipt is touched by c0906fe4f (fix(desk): a folder navigates its window, a GEM ap), named by body

Line 3 before the migration:
Status: **IMPLEMENTIERT — HW OFFEN** · Prioritaet: **P1** · Typ: Belegter Defekt — Pi-Runde 2026-09-24

Abhaengigkeiten: —

## Sachverhalt

Auf der echten Pi 4 (Karte C:–F:, Stand `25396fa`) oeffnet ein Doppelklick auf
einen **Ordner in einem Fenster** jedes Mal ein **neues** Fenster. Erwartet ist
Navigation im selben Fenster.

EmuTOS entscheidet das in `do_aopen` (`references/emutos/desk/desksupp.c`):

```c
if (allow_new_win)
{
    graf_mkstate(&junk, &junk, &junk, &keystate);
    if ((keystate & MODE_SCA) == MODE_ALT)
        new_win = TRUE;             /* :1130-1135 */
}

if (new_win)
{
    pw = win_alloc(curr);           /* :1141-1149 */
    ...
}
else
{
    pn_close(&pw->w_pnode);         /* :1151-1154  <- derselbe Knoten wird neu belegt */
}
do_diropen(pw, new_win, curr, app_path, &t, TRUE);
```

Ein **neues** Fenster gibt es also nur in drei Faellen: beim Oeffnen von der
Schreibtischflaeche (Laufwerkssymbol — eigener Aufrufer, `:1067`, uebergibt
`TRUE`), bei einer Desktop-Verknuepfung (`WN_DESKTOP`, `:1112-1127`,
`CONF_WITH_DESKTOP_SHORTCUTS`) und wenn beim Oeffnen **allein Alt** gehalten
wird. Sonst `pn_close` + Wiederverwendung.

## Arbeit

`do_aopen`s Fallunterscheidung in `kernel/src/desk/main.rs` nachziehen —
`allow_new_win`, die `graf_mkstate`-Abfrage der Umschalttasten und den
`pn_close`-Zweig. Vorher am Quelltext feststellen, welcher unserer Aufrufer
heute welchen Wert setzt; der Laufwerkspfad muss weiterhin ein neues Fenster
bekommen.

Abnahme: Ordner in einem Fenster oeffnen → derselbe Fensterrahmen, neuer
Pfad im Titel; Laufwerkssymbol oeffnen → neues Fenster; Alt+Doppelklick auf
einen Ordner → neues Fenster. Die Alt-Probe braucht eine echte Tastatur.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

## Abschluss

**Implementiert, QEMU-belegt, Pi-Abnahme offen.** Quittung:
[DESK34-STATUS-DONE.MD](DESK34-STATUS-DONE.MD).

`do_fopen`s Fallunterscheidung steht jetzt in `kernel/src/desk/main.rs` als
eigene Funktion `do_fopen(handle, spec, title, allow_new_win)`
(`desksupp.c:1089-1160`): `graf_mkstate` liest den Umschalttastenzustand im
Moment des Oeffnens, `(keystate & MODE_SCA) == MODE_ALT` ist der einzige
`new_win`-Erzeuger, den dieser Aufrufer hat, und der `else`-Zweig ist
`pn_close` + `do_diropen(pw, FALSE, …)` = `navigate_window` — dieselbe
Funktion, die *Close folder* schon benutzt, weil EmuTOS dort denselben Koerper
aufruft (`w_setpath`, `deskfun.c:817-824`).

Welcher Aufrufer heute welchen Wert setzt, am Quelltext festgestellt: beide
NewTOS-Aufrufer von `open_item` (Doppelklick und *File → Open*) sind EmuTOS'
`do_open`, und `do_open` uebergibt `TRUE` (`desksupp.c:1259`; der Doppelklick
laeuft ueber `do_filemenu(OPENITEM)`, `deskmain.c:988`). Der Laufwerkspfad
geht gar nicht durch diese Funktion — `do_dopen` (`:1041-1069`) ist ein
eigener Aufrufer und ruft `win_alloc` unbedingt; `open_root_window` blieb
unveraendert. Der dritte EmuTOS-Fall (`WN_DESKTOP`,
`CONF_WITH_DESKTOP_SHORTCUTS`, `:1112-1127`) ist bei uns nicht erreichbar,
weil es keinen `AT_ISFOLD`-Desktop-`ANODE` gibt (DESK30).

**Abnahme (QEMU raspi4b, `measurements/measurement-2026-09-24-desk34-desk35/`, §2):**
Ordner `SUB` im `C:\`-Fenster geoeffnet -> Fensterslot 0 / AES-Handle 1
bleiben, `p_spec` wechselt von `C:\*.*` auf `C:\SUB\*.*`, Titel `C:\SUB\`
(`d34-after-C-folder.png`); dieselbe Geste auf dem HEAD-Stand oeffnete ein
zweites Fenster (`d34-before-C-folder.png`). Laufwerkssymbol danach ->
neues Fenster, Slot 1 / Handle 2 (`d34-after-D-drive.png`). **Alt+Doppelklick
braucht eine echte Tastatur** und steht als Pruefpunkt 59 in
`docs/api/desktop/_hw-test-2026-09-17.md`.

Benannte Abweichungen: `docs/api/desktop/deskmain.md`, Abschnitt
„opening a folder navigates the window (DESK34)".
