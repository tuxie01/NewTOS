# DESK16 — `IG_REMOV` (5) hat kein Raster

Status: DONE (2026-11-11) | Priority: P4 | Commit: 561b99326 | Receipt: DESK16-STATUS-DONE.MD

Migrated (owner batch 19, 2026-11-11, to DONE), evidence: receipt TASK/DESK16-STATUS-DONE.MD exists, commit 561b99326 (feat(desk): the removable-media icon, the ntrash rule, Set background ), VERIFIED: 561b99326 (feat(desk): the removable-media icon, the ntrash rule, Set b) names the id by body, its message names the ticket-title identifier ig_remov, touches code (19 files), legacy line: ERLEDIGT (2026-09-23)

Line 3 before the migration:
Status: **ERLEDIGT** (2026-09-23) · Prioritaet: **P4** · Typ: Ressourcenluecke — gemeldet in DESK03

Abhaengigkeiten: [DESK03](DESK03.md) (erledigt)

## Sachverhalt

Die `IG_*`-Nummern sind **Dateiformat**: `app_parse`, `app_save` und
`desk_inf_data2` schreiben sie woertlich in die `#M`/`#T`/`#O`-Zeilen
(`references/emutos/desk/deskapp.c`). NewTOS hat Raster fuer 0 Festplatte,
1 Diskette, 2 Ordner, 3 Papierkorb, 4 Drucker, 6 Programm, 7 Dokument —
**`IG_REMOV` (5, Wechselmedium) fehlt** in den aus `desk/icon.rsc`
uebernommenen Bitmaps.

Folgen, beide in DESK03 gemeldet und nicht stillschweigend ersetzt: das
Schrittwerk des `Install icon`-Dialogs laeuft ueber sieben Eintraege statt
`0..g_numiblks-1`, und eine `a_aicon`, die nicht aufgeloest werden kann,
faellt auf das typgerechte Bild zurueck (`app::default_icon_kind`), statt
leer zu zeichnen.

## Arbeit

Das Raster aus `references/emutos/desk/icon.rsc` mit demselben Weg
uebernehmen, den die uebrigen sieben genommen haben, `BUILTIN_ICONS` um den
Eintrag erweitern und den Rueckfall in `default_icon_kind` fuer diesen Fall
entfernen. Danach laeuft das Schrittwerk ueber acht Eintraege.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

## Abschluss

**Erledigt am 2026-09-23**, Branch `desk-rebuild`, Basis HEAD `a8d4b42`,
Aenderungen **uncommitted** im Baum. Quittung:
[DESK16-STATUS-DONE.MD](DESK16-STATUS-DONE.MD).

Das Raster kam denselben Weg wie die uebrigen sieben: aus
`references/emutos/desk/icons.c`, dem C, das der EmuTOS-Bau mit `ird` aus
`desk/icon.rsc` erzeugt. `icon_rs_iconblk[5]` ist `rs_icondata5` ueber
**`rs_iconmask0`** — das Wechselmedium hat **keine eigene Maske**, es teilt
sich die der Festplatte; `ib_xchar`/`ib_ychar` sind ebenfalls die der
Festplatte, `(13,16)`. Alle sieben vorhandenen Raster wurden bei der
Gelegenheit byteweise gegen dieselbe Datei nachgeprueft.

`BUILTIN_ICONS` hat jetzt acht Eintraege (`icons::NUM_GEM_IBLKS`). Das
Schrittwerk laeuft ueber `0 .. G.g_numiblks-1` wie `deskins.c:902-906` —
abzueglich der Diskette, solange kein Laufwerk `A:`/`B:` gemeldet wird; das
ist die bestehende, davon unabhaengige NewTOS-Regel.

Der Rueckfall ist weg. Damit `app_blddesk` wie EmuTOS ohne Rueckfall
indizieren kann, traegt `desk::inf::parse_anode` jetzt `app_parse`s eigene
Klemmung (`deskapp.c:359-365`): `a_aicon >= NUM_GEM_IBLKS` → `IG_APPL`,
`a_dicon >= NUM_GEM_IBLKS` → `IG_DOCU`. `app::default_icon_kind` ist
zusammen mit dem Loch, das sie bediente, entfallen.

**Eine benannte Abweichung**, auf einem Wert, den EmuTOS ausserhalb der
Grenzen liest: `a_aicon == -1` auf einer Schreibtischzeile (`FF` im
Bildfeld; nur eine handgeschriebene Datei kann das tragen, `build_inf`
schreibt immer die Nummer, `deskapp.c:723`). EmuTOS greift auf
`G.g_iblist[-1]`; NewTOS nimmt `app_parse`s eigene Antwort fuer ein
unbrauchbares `a_aicon`, `IG_APPL`.

Beleg: [`measurements/measurement-2026-09-23-desk16-desk17-desk21/`](../measurements/measurement-2026-09-23-desk16-desk17-desk21/).
Die serielle Zeile des Kastens geht von `curr_icon=0 of 6` auf `of 7`, vier
Klicks auf den Pfeil nach unten landen auf dem neuen Bild, `Save desktop`
schreibt `#M 00 00 05 FF C DISK C@ @`, ein **Kaltstart** liest es zurueck.
`method/pixcheck.py` vergleicht jedes maskengesetzte Pixel der Vorschau und
des Schreibtischsymbols mit `icons.c`: 654 bzw. 618 Pixel, **0** Abweichungen.

Spezifikation nachgezogen: `docs/api/desktop/deskapp.md` (neuer Abschnitt),
`docs/api/desktop/deskins.md` (Symboltabelle + Schrittwerk),
`docs/api/desktop/deskwin.md` (Rueckfallformulierung).
Hardware-Punkt **45** in `docs/api/desktop/_hw-test-2026-09-17.md`.
