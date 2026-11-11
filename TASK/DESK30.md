# DESK30 — Desktop-Verknuepfungen (`CONF_WITH_DESKTOP_SHORTCUTS`)

Status: PARKED (2026-11-11) | Priority: P2 | Reason: legacy, evidence unverified; receipt DESK30-STATUS-DONE.MD exists | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 15, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: no cited file of the ticket/receipt is touched by 0364966cd (feat(desk): window icons - a file's icon can come ), named by body

Line 3 before the migration:
Status: **ERLEDIGT** · Prioritaet: **P2** · Typ: Funktionsluecke — gemeldet in DESK15

Abhaengigkeiten: [DESK15](DESK15.md) (erledigt)

## Sachverhalt

DESK15 hat die Fenstersymbole gebaut (`CONF_WITH_WINDOW_ICONS`), also den
Leser `win_icalc` und den `ADINSWIN`-Kasten. Die **zweite** Haelfte derselben
Meldung ist offen: `CONF_WITH_DESKTOP_SHORTCUTS` — eine Datei oder ein Ordner
als Symbol **auf dem Desktop**.

Sie ist nicht dieselbe Ebene. Ein Fenstersymbol ist ein `#I`/`#N`/`#D`-Knoten
der Zuordnungsliste (`desk::inf`s Session-Modell), eine Verknuepfung ist ein
`#X`/`#V`-Knoten der **Desktop**-Liste `DeskState::g_ahead`, und die hat bis
heute kein `AT_ISFILE`/`AT_ISFOLD` (`global::Anode` traegt weder `a_pdata`
noch `a_pargs`, siehe ihr Doc-Kommentar).

`CONF_WITH_DESKTOP_SHORTCUTS` ist im Profil Falcon030/TOS4 (`make 512`) **1**
(`references/emutos/include/config.h:1782-1784`; die `0` bei `:325-327` steht
in `#ifdef TARGET_192`), also eine echte Luecke.

## Auf der Pi bestaetigt (2026-09-24)

Der Benutzer hat beim Hardwarelauf versucht, **QED als Verknuepfung auf den
Desktop zu legen** — Ziehen funktioniert, aber das Ablegen auf der
Schreibtischflaeche endet mit einer Fehlermeldung. Das ist genau diese Luecke
und kein zusaetzlicher Defekt: unsere Desktop-Liste kennt nur
Laufwerk/Papierkorb/Drucker, ein `AT_ISFILE`-Knoten existiert nicht, also ist
die Ablehnung folgerichtig.

**Deshalb von P3 auf P2 hochgestuft:** ein Programm auf den Desktop zu legen
ist das Erste, was jemand bei einer Vorfuehrung versucht.

## Arbeit

Quellen: `references/emutos/desk/deskins.c` — `ins_shortcut` (`:1341-…`),
`is_installed` (`:71-116`), die `AT_ISFILE`/`AT_ISFOLD`-Zweige von
`install_desktop_icon` (`:781-978`), `ins_app`s Verknuepfungszweig
(`:619-720`); `references/emutos/desk/deskapp.c` — `app_parse`/`app_save` fuer
`#X`/`#V` (`:333-344`, `:1223-1244`) und die Vertauschung von
`a_pappl`/`a_pdata` unterhalb von `inf_rev_level 2` (`:404-415`);
`references/emutos/desk/deskfun.c` — `fun_win2win`s Ablegen-auf-Programm.

Reihenfolge wie bei DESK15: zuerst die Datenebene (`Anode` bekommt
`a_type = File/Folder`, `a_pdata`, `a_flags`-Ersatz), dann `app_blddesk` und
`app_afind_by_id`s Konsumenten (Doppelklick oeffnet die Datei, Ablegen darauf
startet sie), dann `ins_shortcut` und die Dialogzweige.

Nebenbefund aus DESK18, der hier haengt: EmuTOS unterscheidet die beiden
Stellungen von *Default dir* nur ueber Verknuepfungen und `fun_win2win`s
Ablegen-auf-Programm — erst diese Haelfte macht den Schalter sichtbar.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

## Abschluss

Gebaut, in der im Ticket vorgegebenen Reihenfolge.

**Datenebene.** `global::Anode` hat `a_dicon`, `a_flags` und `a_pdata`
bekommen ([global.rs](../kernel/src/desk/global.rs)). `AF_ISDESK` bleibt
implizit — jeder Knoten von `G.g_ahead` *ist* ein Desktop-Knoten, das ist
genau die `AF_ISDESK`-Haelfte von EmuTOS' einer Liste —, von den uebrigen
`AF_*` traegt der Desktop-Knoten nur `AF_ISEXEC`: `do_aopen` ersetzt die
`ANODE` einer Verknuepfung vor jedem Flag-Lesen durch die der
Zuordnungsliste (`desksupp.c:857-871`), die uebrigen Flags haette also
niemand gelesen. Fuer eine Verknuepfung ist `a_pappl` der **volle Pfad** und
`a_pdata` der blosse Name — die Stellung ab `inf_rev_level 2`.

**`app_blddesk` und die `.INF`-Ebene.** Die Beschriftung kommt bei
`AT_ISFILE`/`AT_ISFOLD` aus `a_pdata` (`deskapp.c:1392-1396`), das Bild aus
`(a_dicon < 0) || (a_type == AT_ISFILE && (a_flags & AF_ISEXEC))`
(`:1374-1378`). `#X`/`#V` lesen und schreiben ([inf.rs](../kernel/src/desk/inf.rs)),
samt der Vertauschung unterhalb `inf_rev_level 2` (`:404-415`) — dafuer wird
`#R` jetzt wirklich gelesen statt verworfen — und samt
`AF_ISEXEC`-aus-Endung (`:422-426`) ueber `ins::is_executable`, dessen Menge
EmuTOS' fuenf plus die native `.NAP`/`.NCP`/`.NTP`-Familie ist (ADR-018).

**`ins_shortcut`** ([ins.rs](../kernel/src/desk/ins.rs), `deskins.c:1341-1406`),
gerufen aus `dir::fun_drag`s Ablegen-auf-die-Flaeche (`deskfun.c:1312-1319`),
wo bisher der `#else`-Zweig `STNODRA1` stand. `fun_drag` nimmt dafuer den
Ablegepunkt entgegen, den `act_bdown` zurueckgibt (`*in_mx = dulx - offx`,
`deskact.c:683-684`).

**Verbraucher.** Doppelklick: `main::open_item`s Desktop-Arm (`do_open`,
`desksupp.c:1219-1259`) — eine Datei wird nach ihrer Endung geleitet wie ihr
Zwilling im Fenster, ein Ordner bekommt **immer** ein neues Fenster
(`do_fopen`s `WN_DESKTOP`-Arm, `:1112-1126`). Ziehen **von** einer
Verknuepfung: `dir::desktop_selected_sources`, `fun_file2any`s
`an_src`-Schalter (`deskfun.c:1149-1160`). Ablegen **auf** eine Verknuepfung:
die zwei neuen Arme von `fun_file2desk` (`:1021-1040`). Loeschen ueber das
Menue: `fun_del`s Schalter (`:1393-1405`). `move_drvicon` und `rmv_icon`
waren schon typunabhaengig.

**Waisen.** `remove_locate_shortcut` (`desksupp.c:376-412`) ist in
[dir.rs](../kernel/src/desk/dir.rs) gebaut: der `STRMVLOC`-Kasten
(Remove | Locate | Cancel), *Locate* mit `fsel_exinput` ab der Wurzel des
Startlaufwerks und der Neuausrichtung von `a_pappl`. Gerufen wird er, wo
EmuTOS ihn ruft: vor dem Start (`file_exists`, `:861-868`), vor dem
Ordnerfenster (`set_default_path == EPTHNF`, `:1119-1123`) und beim Ziehen
einer Verknuepfung ohne Datei dahinter (`:1183-1188`).

**Eine zusammengefasste Stelle, benannt.** `ins_shortcut` nimmt die
Iconnummern von der `ANODE`, die `i_find` liefert, und **ueberschreibt** sie
mit einer zweiten Suche, `app_afind_by_name(…, AF_ISDESK, …)` (`:1394-1400`).
Die beiden unterscheiden sich nur in der Ignoriermaske — `i_find` ueberspringt
zusaetzlich `AF_WINDOW` (`desksupp.c:1755`) —, und unser
`app_afind_by_name` ueberspringt nur Desktop-Knoten, ist also schon die
Variante des zweiten Aufrufs. Die erste Suche liefert damit den Knoten, den
die Ueberschreibung gewaehlt haette; es wird einmal gesucht.

**Gemessen** (`measurements/measurement-2026-09-24-desk37-desk36-desk30/`, QEMU raspi4b
640x480):

* `d30-after-D-shortcut.png` — `QED.NAP` aus dem `C:\`-Fenster auf die
  Flaeche gezogen ergibt ein Symbol mit der Beschriftung `QED.NAP` und dem
  Programmbild. `d30-before-D-shortcut.png` ist der Befund vom Pi-Lauf:
  „Can't drop items here."
* `d30-after-E-saved.png` und die geschriebene Karte: *Options → Save
  desktop* legt `#X 02 09 06 FF   C:\QED.NAP@ QED.NAP@` in `NEWDESK.INF`.
* `d30b-A-coldstart.png` — dieselbe Karte kalt gebootet: die Verknuepfung ist
  zurueck, in derselben Rasterzelle, mit derselben Beschriftung und demselben
  Bild.
* `d30c-D-qed.png` — Doppelklick auf die wiederhergestellte Verknuepfung
  startet QED.
* `d30d-D-opened.png` — der Ordnerfall: `SUB` auf die Flaeche
  (`d30d-C-shortcut.png`), Doppelklick, ein **neues** Fenster `C:\SUB\` mit
  seinen drei Eintraegen.

## Offen geblieben, mit Grund

* **Der Waisenfall ist nur gebaut, nicht gefahren.** Er braucht ein
  Verschieben der Zieldatei zwischen zwei Sitzungen; im Injektor waere das
  eine lange Klickfolge ohne Mehrwert gegenueber der Hardware. Er steht als
  Punkt 63.5 in
  [_hw-test-2026-09-17.md](../docs/api/desktop/_hw-test-2026-09-17.md).
* ***Show info* auf einem Desktop-Symbol** gibt es in NewTOS ueberhaupt
  nicht: `main::do_info_rename` kehrt fuer `G.g_cwin == DESKWH` sofort um.
  `do_info`s Verknuepfungsarm haette also nichts, woran er haengen kann. Die
  Luecke ist aelter als DESK30 und betrifft Laufwerke, Papierkorb und Drucker
  genauso — eigener Task.

Spec: [docs/api/desktop/deskins.md](../docs/api/desktop/deskins.md), Abschnitt
„`ins_shortcut` — desktop shortcuts", und
[deskapp.md](../docs/api/desktop/deskapp.md). Hardware-Gegenprobe: Punkt 63.
