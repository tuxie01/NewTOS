# DESK18 — Kein Arbeitsverzeichnis beim Programmstart (`set_default_path`)

Status: DONE (2026-11-11) | Priority: P2 | Commit: cd6afabeb | Receipt: DESK18-STATUS-DONE.MD

Migrated (owner batch 19, 2026-11-11, to DONE), evidence: receipt TASK/DESK18-STATUS-DONE.MD exists, commit cd6afabeb (feat(desk): a program started from the desktop gets a working director), VERIFIED: cd6afabeb (feat(desk): a program started from the desktop gets a workin) names the id by body, its message names the ticket-title identifier set_default_path, touches code (16 files), legacy line: ERLEDIGT (2026-09-23)

Line 3 before the migration:
Status: **ERLEDIGT** (2026-09-23) · Prioritaet: **P2** · Typ: Kompatibilitaetsdefekt — Nebenbefund DESK05

Abhaengigkeiten: [DESK05](DESK05.md) (erledigt)

## Sachverhalt

EmuTOS setzt vor **jedem** Programmstart das Arbeitsverzeichnis. `do_aopen`
(`references/emutos/desk/desksupp.c:880-915`) waehlt den Pfad nach den
Desktop-Konfigurationsschaltern — installierte Anwendung: ihr eigener Pfad;
sonst `G.g_appdir` (Anwendungsverzeichnis) oder das oberste Fenster — und
ruft `set_default_path(app_path)`; scheitert das, bricht der Start mit
`STDEFDIR` ab.

NewTOS setzt **gar keins**: `desk::main::launch_native` geht ueber
`crate::launch::run_program`, ohne vorher `Dsetpath`/`Dsetdrv` zu stellen.
Ein portiertes Programm, das relativ oeffnet (`Fopen("FOO.TXT")`), sieht
damit irgendein Verzeichnis.

Zweite Folge: die Radiogruppe *Default dir* (`DCDEFAPP`/`DCDEFWIN`) fehlt in
`Desktop configuration` (DESK05), weil sie genau diesen Schalter bedient —
`INF_E5_APPDIR` wird gelesen und unveraendert zurueckgeschrieben, aber von
niemandem ausgewertet.

## Arbeit

`do_aopen`s Pfadwahl nachbauen, mit `dosbind`s Bindungen (`Dsetpath`,
`Dsetdrv`), `STDEFDIR`-Alert im Fehlerfall; danach die Radiogruppe in
`inf_conf` nachziehen.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

---

## Abschluss (2026-09-23)

Beides gebaut, in dieser Reihenfolge.

**1. Der Startpfad setzt ein Arbeitsverzeichnis.**
`kernel/src/desk/pro.rs::set_default_path` ist `desksupp.c:1785-1801`
(`desk_busy_on` → `dos_sdrv(path[0]-'A')` → `dos_chdir(path)` →
`desk_busy_off`); die dafuer fehlende Bindung `dosbind::dos_chdir` =
`Dsetpath` (0x3B, `gemdos.h:95-98`) ist neu. Darueber liegt
`kernel/src/desk/main.rs::set_launch_dir`, `do_aopen`s Pfadwahl
(`desksupp.c:893-916`): installierte Anwendung → ihr `AF_APPDIR`, sonst
`G.g_appdir`; ist der Schalter aus und ein Fenster oben, gilt **dessen**
Pfad. Beide Aufrufer sind die zwei Haelften von `do_aopen` —
`open_item`s `OpenTarget::App` und `open_document`. Scheitert `Dsetpath`,
kommt `STDEFDIR` (`desk_rsc.c:2558-2559`) und es wird **nicht** gestartet.

**2. *Default dir* ist zurueck in `inf_conf`.**
`ADDESKCF`s Objekte 2..5 woertlich (Beschriftung, eigener `G_IBOX`,
*Application*/*Window*), Kasten 312x320 statt 312x288, Vorwahl aus
`G.g_appdir` und Commit ueber `inf_which(tree, DCDEFAPP, 2)`
(`deskinf.c:1105-1112`, `:1226`). Persistenz: `global::CSave::cs_appdir`
+ `cnx_put`/`cnx_get` (`deskmain.c:1391`, `:1453`) + `INF_E5_APPDIR` im
`#E`-Byte 5. *Parameter* bleibt draussen — sein Verbraucher
(`fun_win2win`s Ablegen-auf-Programm) fehlt weiter.

**Benannte Abweichung / ehrlicher Befund.** Auf dem heutigen Desktop
waehlen *beide* Stellungen von *Default dir* dasselbe Verzeichnis: ein
Klick auf ein nicht oberstes Fenster wird von der AES als `WM_TOPPED`
verbraucht (`aes/gemctrl.c:172-179`), also ist das Fenster des
doppelgeklickten Programms immer das oberste. Das ist EmuTOS' eigenes
Verhalten; EmuTOS unterscheidet die Stellungen nur ueber
Desktop-Verknuepfungen (`CONF_WITH_DESKTOP_SHORTCUTS`) und
`fun_win2win`s Ablegen-auf-Programm, die NewTOS beide nicht hat. Der
**gleiche Zweig** ist ueber die Zuordnungsseite (`INF_AT_APPDIR`)
beidseitig gemessen und unterscheidet sauber.

Messung: [`measurements/measurement-2026-09-23-desk18/`](../measurements/measurement-2026-09-23-desk18/).
Empfang: [`DESK18-STATUS-DONE.MD`](DESK18-STATUS-DONE.MD).
