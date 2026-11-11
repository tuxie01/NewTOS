# DESK50 — `do_wopen`/`do_diropen` fehlen; der Fensterschwanz ist bei uns verteilt

Status: PARKED (2026-11-11) | Priority: P2 | Reason: legacy, evidence unverified; receipt DESK50-STATUS-DONE.MD exists | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 19, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: hand-check rejected the automatic match: 4b3d73828 (feat(desk): opn_appl asks a .NTP for its parameters, and the) names the id by body, its message names the ticket-title identifier do_wopen,

Line 3 before the migration:
Status: **ERLEDIGT — Pi-Abnahme 2026-10-04** ([Statusdatei](DESK50-STATUS-DONE.MD)) · Prioritaet: **P2** · Typ: Fehlende EmuTOS-Funktion — abgegrenzt aus [DESK47](DESK47.md)

Abhaengigkeiten: [DESK47](DESK47.md) (dort als eigenes Paket erkannt), [DESK43](DESK43.md) U3 (machte den Zweig erreichbar)

## Warum das ein eigenes Ticket ist

[DESK47](DESK47.md) nannte `pro_run`s `(wh, curr)`-Zweig und `do_wopen` als
„in derselben Ecke offen" und liess pruefen, wie viel davon dorthin gehoert.
Die Antwort aus dem Quelltext: **nichts davon.**

`do_wopen` (`references/emutos/desk/desksupp.c:294`) hat in EmuTOS **zwei**
Aufrufer — `pro_run` (`deskpro.c:61`) und `do_diropen`
(`desksupp.c:454`) — und `do_diropen` selbst **vier** (`deskfun.c:413`,
`deskmain.c:1496`, `desksupp.c:1067`, `:1156`). Es ist der **gemeinsame
Schwanz jedes Fensteroeffnens**, kein Helfer von `pro_run`.

`do_wopen` nur fuer `pro_run` zu bauen haette eine Funktion ergeben, die den
Namen traegt, ohne es zu sein. Genau die Art Naeherung, die ADR-022 ausschliesst.

## Der Befund

- **`do_diropen` gibt es bei uns nicht.** Sein Rumpf liegt verteilt in
  `open_dir_window_at`, `navigate_window`, `rebuild_window`,
  `open_root_window` und dem `cnx_get`-Pfad (`kernel/src/desk/`).
- **Keiner dieser Wege macht `graf_growbox`**, und keiner die Abwahl
  `act_chg(…, FALSE, FALSE)`. `do_fopen`s Doc-Kommentar haelt dieselbe
  Luecke bereits von der Fensterseite her fest — die Begruendung steht also
  schon zweimal im Baum, der Code fehlt trotzdem.
- **Es fehlt eine `graf_growbox`-Bindung** in `kernel/src/desk/mod.rs`.
  Opcode 73 ist in `tos-api/src/aes/graf.rs:356` **vorhanden**; es fehlt nur
  der Weg des Desktops dorthin.
- Vorhanden und benutzbar: `do_xyfix`, `obj_extent` (= `get_xywh` +
  Wurzelversatz), `act_chg`, `g_wlastsel`.

**Der Zweig war bis [DESK43](DESK43.md) U3 unerreichbar und ist es seitdem
nicht mehr.** Das ist der Grund, warum die Luecke jetzt zaehlt und vorher
nicht.

## Sichtbare Folge, heute schon belegt

Nach *Cancel* im Parameterdialog bleibt das Programmsymbol ausgewaehlt
(`measurements/measurement-2026-09-30-desk47/screendumps/c2-after-cancel.png`).

**Das ist EmuTOS-konform und kein Fehler** — die Abwahl sitzt in `do_wopen`,
das nur ein **erfolgreiches** `pro_run` erreicht. Es ist hier nur vermerkt,
damit die naechste Einheit es nicht als Defekt tickert
([„Tickets aus Screendumps gehen oft daneben"](../docs/MEMORY.md)).

## Arbeit

1. `do_wopen` nach `desksupp.c:294` bauen, mit `graf_growbox` ueber eine neue
   Bindung in `mod.rs` (der AES-Opcode ist da).
2. `do_diropen` nach `desksupp.c:454` bauen und die **vier** EmuTOS-Aufrufer
   auf unsere Entsprechungen abbilden — das ist der Teil, der den verteilten
   Rumpf wieder zusammenzieht.
3. `pro_run`s `(wh, curr)`-Zweig anschliessen.

**Reihenfolge beachten:** Punkt 2 nimmt bestehenden Wegen Code weg. Die
Driftprobe (`sh tools/desk-drift-probe.sh`, derzeit **32**) darf dabei nicht
steigen.

## Abnahme

* Ein Fenster oeffnet mit `graf_growbox` aus dem Symbol heraus — sichtbar im
  Screendump, nicht nur im Protokoll.
* Nach einem **erfolgreichen** Programmstart ist das Symbol abgewaehlt, nach
  *Cancel* bleibt es ausgewaehlt (beides EmuTOS).
* Die vier `do_diropen`-Aufrufwege gehen durch **eine** Funktion.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.
