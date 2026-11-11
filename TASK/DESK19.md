# DESK19 — Ablegen auf ein Programm startet es nicht (`fun_win2win`)

Status: DONE (2026-11-11) | Priority: P3 | Commit: cd6afabeb | Receipt: DESK19-STATUS-DONE.MD

Migrated (owner batch 19, 2026-11-11, to DONE), evidence: receipt TASK/DESK19-STATUS-DONE.MD exists, commit cd6afabeb (feat(desk): a program started from the desktop gets a working director), VERIFIED: cd6afabeb (feat(desk): a program started from the desktop gets a workin) names the id by body, its message names the ticket-title identifier fun_win2win, touches code (16 files), legacy line: ERLEDIGT (2026-09-23)

Line 3 before the migration:
Status: **ERLEDIGT** (2026-09-23) · Prioritaet: **P3** · Typ: Funktionsluecke — Nebenbefund DESK05

Abhaengigkeiten: [DESK05](DESK05.md) (erledigt)

## Sachverhalt

In EmuTOS ist das Ablegen einer Auswahl auf ein **Programmsymbol** ein
Programmstart mit Argumenten, kein Kopieren: `fun_win2win`
(`references/emutos/desk/deskfun.c:958-970`) erkennt das Ziel und baut die
Befehlszeile mit `build_selected_path` (`:903-931`), dessen Form die
Desktop-Konfiguration bestimmt (voller Pfad oder nur der Dateiname).

NewTOS kennt diesen Zweig nicht — ein Ablegen auf ein Programm ist eine
Kopie. Deshalb fehlt auch die Radiogruppe *Parameter* (`DCPMFULL`/
`DCPMFILE`) in `Desktop configuration` (DESK05); `INF_E5_ISFULL` wird
gelesen und unveraendert zurueckgeschrieben.

## Arbeit

Den Zweig in `fun_win2win` nachbauen, `build_selected_path` mit, dann die
Radiogruppe in `inf_conf` nachziehen.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

---

## Abschluss (2026-09-23)

Die Praemisse haelt Zeile fuer Zeile. `fun_win2win` loest das Ziel mit
`i_find` auf und prueft `if (pda->a_aicon >= 0)` (`deskfun.c:962`) — „dropping
file on to an application" —, baut die Befehlszeile mit `build_selected_path`
(`:903-928`) und ruft `do_aopen(pda, TRUE, dst_ob, pdw->w_pnode.p_spec,
pdf->f_name, destpath)` (`:967`), dann **return**: es wird nichts kopiert.
`CONF_WITH_DESKTOP_CONFIG` (die `G.g_fullpath`-Abfrage) und
`CONF_WITH_DESKTOP_SHORTCUTS` (der `if (tail)`-Zweig in `do_aopen`,
`desksupp.c:968-977`) stehen beide auf 1 (`include/config.h:1774-1784`), also
im Falcon030/TOS4-Profil an.

**1. Der Zweig.** `kernel/src/desk/dir.rs`: `i_find_win` ist `i_find`s
Fensterarm (`desksupp.c:1753-1770`) — Name + `ANODE` ueber
`inf::app_afind_by_name`, genau wie `win_icalc` das Symbol aufloest;
`build_selected_path` ist `:903-928` (erste gewaehlte Datei, voller Pfad oder
blosser Name); `fun_win2win` nimmt den Zweig und gibt `exit_desktop` zurueck,
das `fun_drag` jetzt durchreicht. `kernel/src/desk/main.rs::do_aopen_drop` ist
`do_aopen`s `isapp`-Arm mit nicht-`NULL`-`tail`: Verzeichniswahl ueber
`set_launch_dir` (DESK18), `app_path` aus Zielfenster + Programmname
(`:958-960`), Tail woertlich (`:969-975`). `inf::Afind` traegt dafuer `pappl`
und `appdir` mit, damit `is_installed(pa)` und `AF_APPDIR` echt sind.

**2. *Parameter* ist zurueck in `inf_conf`.** `ADDESKCF`s Objekte 6..9
woertlich (Beschriftung, **eigener** `G_IBOX`, *Full path*/*File name*,
`desk_rsc.c:1641-1663`), Kasten wieder 312x352 und keine Zeile mehr
verschoben; Vorwahl aus `G.g_fullpath`, Commit ueber
`inf_which(tree, DCPMFULL, 2)` (`deskinf.c:1113-1116`, `:1227`). Persistenz:
`global::CSave::cs_fullpath` + `cnx_put`/`cnx_get` (`deskmain.c:1392`,
`:1454`) + `INF_E5_ISFULL` im `#E`-Byte 5 (`deskapp.c:943`, `:1165`).

**Die Befehlszeile kommt an, gemessen.** `sdk/examples/cmdline.c` (neu, von
`tools/make-sd-image.sh` als `C:\CMDLINE.NCP` installiert) druckt sein eigenes
`p_cmdlin`. Dieselbe Karte, dieselbe Geste, ein Radioknopf Unterschied:

    Full path   CMDLINE: len 13 arg "C:\README.TXT"
    File name   CMDLINE: len 10 arg "README.TXT"

Vorher (HEAD) kommt auf dieselbe Geste die Kopierbestaetigung statt eines
Starts.

**Nebenbefund, der DESK18 aufloest.** *Default dir* ist jetzt **beobachtbar**:
zwei Fenster, Quelle oben, Programm im unteren — *Application* startet in
`C:\` (dem Zielfenster), *Window* in `C:\DESKT19` (dem obersten). DESK18
konnte das nicht zeigen, weil genau dieser Zweig fehlte.

**Benannte Abweichung.** EmuTOS setzt `exit_desktop` und verlaesst den
Desktop-Prozess; NewTOS startet **geschachtelt** (AES-Luecke G1,
`main::launch_native`), also gibt `do_aopen_drop` `false` zurueck und der
Desktop steht schon wieder. Dieselbe Abweichung wie beim Doppelklick, keine
neue. Zwei Antworten hat `do_aopen` nicht, weil NewTOS' Lader nur native
AArch64-ELF laedt (ADR-018): ein `.PRG`-Ziel bekommt die vorhandene
m68k-Meldung, ein Ziel, dessen Name keine native Endung traegt, „This file is
not a native NewTOS program".

Messung: [`measurements/measurement-2026-09-23-desk19-desk20/`](../measurements/measurement-2026-09-23-desk19-desk20/).
Empfang: [`DESK19-STATUS-DONE.MD`](DESK19-STATUS-DONE.MD).
