# DESK36 — Beim Ziehen wird das Ziel unter dem Zeiger nicht hervorgehoben

Status: PARKED (2026-11-11) | Priority: P2 | Reason: legacy, evidence unverified; receipt DESK36-STATUS-DONE.MD exists | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 19, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: no cited file of the ticket/receipt is touched by 081ef4be5 (feat(desk): desktop background behind an app, drag), named by body

Line 3 before the migration:
Status: **ERLEDIGT** · Prioritaet: **P2** · Typ: Belegter Defekt — Pi-Runde 2026-09-24

Abhaengigkeiten: —

## Sachverhalt

Zieht man eine Datei ueber ein anderes Symbol (Programm, Ordner, Papierkorb,
Laufwerk), wird das Ziel **nicht** invertiert. Die Logik dahinter stimmt — das
Ablegen landet am richtigen Ziel (auf der Pi mit `CMDLINE.NCP` geprueft) —,
aber der Benutzer sieht beim Ziehen nicht, worauf er trifft.

EmuTOS macht das in der Ziehschleife selbst, `gr_drgplns`
(`references/emutos/desk/deskact.c:251-360`). Sie fuehrt ueber die ganze
Bewegung einen Zustand `curr_wh`/`curr_root`/`curr_sel` mit:

```c
*pdobj = gr_obfind(tree, root, l_mx, l_my);      /* :309  was liegt unter dem Zeiger */

if ((*pdobj == root) || (*pdobj == NIL)) {        /* :313  nichts mehr drunter */
    if (curr_sel) { act_chg(curr_wh, curr_root, curr_sel, pc, FALSE, TRUE); ... }
}
if (*pdobj != curr_sel) {                         /* :325  Ziel gewechselt */
    if (curr_sel) { act_chg(..., FALSE, TRUE); ... }
}
obj = tree + *pdobj;
if (!(obj->ob_state & SELECTED)) {                /* :335 */
    pa = i_find(dst_wh, *pdobj, NULL, &isapp);
    if (pa) {
        if ((pa->a_type == AT_ISFILE) && !isapp)  /* :340  "don't highlight plain files" */
            continue;
        curr_sel = *pdobj;
        act_chg(curr_wh, curr_root, curr_sel, pc, TRUE, TRUE);
    }
}
...
if (curr_sel)                                     /* :353  beim Loslassen zuruecknehmen */
    act_chg(curr_wh, curr_root, curr_sel, pc, FALSE, TRUE);
```

Drei Regeln stecken darin, die ein naiver Nachbau verfehlt:

1. **Eine gewoehnliche Datei wird nicht hervorgehoben** — nur Ordner,
   Laufwerke, Papierkorb, Drucker und *installierte Anwendungen* (`isapp`).
2. Beim Wechsel des Ziels wird das alte **erst zurueckgenommen**, dann das
   neue gesetzt; ein bereits `SELECTED`-Objekt wird gar nicht angefasst.
3. Beim Loslassen wird die Hervorhebung **immer** zurueckgenommen, auch wenn
   das Ablegen scheitert.

`kernel/src/desk/act.rs` hat `act_chg`, aber kein Gegenstueck zu
`gr_drgplns`' Zustandsfuehrung: der Zeiger zieht ueber die Symbole, ohne dass
eines seinen Zustand aendert.

## Arbeit

`gr_drgplns`' Schleife nachziehen. Vorher am Quelltext feststellen, wo unsere
Ziehschleife liegt und ob sie ueberhaupt pro Bewegung laeuft oder nur beim
Loslassen auswertet — davon haengt ab, ob das eine Ergaenzung oder ein Umbau
ist. `i_find` gibt es seit DESK19 als `dir::i_find_win`.

Abnahme: eine Datei ueber Programm, Ordner, Papierkorb und eine gewoehnliche
Datei ziehen. Die ersten drei muessen invertieren, die vierte nicht; beim
Verlassen geht die Hervorhebung zurueck, beim Loslassen ebenfalls.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

## Abschluss

Behoben, und es war ein **Umbau**, kein Zusatz.

**Wo unsere Ziehschleife lag.** Nicht im Desktop: `act_bdown` rief die **AES**
`graf_dragbox` (71), und die fuehrt die Schleife selbst
([tos-api/src/aes/graf.rs](../tos-api/src/aes/graf.rs) `:221-260`), zeichnet
einen einzelnen Kasten und gibt nur die Endposition zurueck. Damit kann kein
`act_chg` je gerufen werden: die Schleife lief in der AES, und die AES kennt
keine `ANODE`s. (Auf EmuTOS' eigener Rechnung war `graf_dragbox` hier ohnehin
falsch — [gembind.md:85](../docs/api/desktop/gembind.md) haelt fest, dass die
Desktop-Bindung sie gar nicht enthaelt, `aesbind.h:103-105`.)

**Jetzt nachgezogen**, in [kernel/src/desk/main.rs](../kernel/src/desk/main.rs)
neben `gr_obfind`/`act_bdown`: `gr_drgplns`, `gr_bwait`, `gr_isdown`,
`gr_plns`, `gr_extent`, `gr_obalign`, `rc_constrain`, `act_chg_draw`. Die
reinen Haelften in [act.rs](../kernel/src/desk/act.rs): `gr_accobs` und
`DeskState::obj_extent`. Die Umrisslisten `G.g_xyicon`/`G.g_xytext` baut
`app_start` ([app.rs](../kernel/src/desk/app.rs), `deskapp.c:988-1011`).
`i_find`s Desktop-Arm kam als `dir::i_find_atype` dazu (`desksupp.c:1746-1767`),
`graf_dragbox` ist aus der aesbind verschwunden, weil sie keinen Rufer mehr hat.

**Die drei Regeln, einzeln gemessen** — `method/d36.actions`, eine Geste:

| Bild | Ziel | Hervorhebung |
|---|---|---|
| `d36-after-C-prog.png` | `QED.NAP`, installierte Anwendung | **ja** |
| `d36-after-D-folder.png` | Ordner `SUB` | **ja** |
| `d36-after-E-file.png` | `README.TXT`, gewoehnliche Datei | **nein** |
| `d36-after-F-trash.png` | Papierkorb | **ja** |
| `d36-after-G-back.png` | das gezogene Symbol selbst (schon `SELECTED`) | unberuehrt |
| `d36-after-H-released.png` | nach dem Loslassen | ueberall zurueck |

Die `d36-before-*`-Gegenstuecke zeigen den Kasten und keine Hervorhebung.
Das XOR-Loeschen ist exakt: `d36-after-H-released.png` unterscheidet sich von
`d36-after-B-win1.png` (vor dem Ziehen) nur in der Infozeile und in der Zelle
des Quellsymbols — Rechteck `(58,70)-(383,211)`; auf dem Hintergrundraster und
auf jedem anderen Symbol steht Pixel fuer Pixel dasselbe.

## Benannte Abweichung

**Der Desktop hat eine eigene VDI-Arbeitsstation bekommen** —
[kernel/src/desk/vdibind.rs](../kernel/src/desk/vdibind.rs).
`gr_plns` zeichnet den Umriss mit `gsx_pline` → `gsx_xline`, also
`vsl_udsty` + `v_pline` je Segment in `MD_XOR`
(`references/emutos/aes/gemgraf.c:140-182`), und zwar auf der Arbeitsstation,
die EmuTOS' Desktop mit der AES **teilt** — er darf das, weil der ROM-Desktop
in die AES gelinkt ist. Unser Desktop erreicht das System nur ueber Traps, und
unsere AES hat keine VDI-Arbeitsstation zum Teilen: sie malt direkt ueber das
Raster-Backend, `v_opnwk` wird nie gerufen, Handle 1 ist nicht offen. Also
oeffnet der Desktop seine **eigene** virtuelle, wie jede NewTOS-GEM-Anwendung:
`graf_handle` (AES 77) → `v_opnvwk`, dazu `vsl_type(7)` + `vsl_udsty(0xffff)`,
was in EmuTOS zur Einrichtung der AES-Arbeitsstation gehoert
(`gsx_start`, `aes/gemgraf.c:420-422`). `gr_bwait` setzt Clip und
Schreibmodus in **jedem** Durchlauf neu — genau das verlangt EmuTOS' eigener
Kommentar an `references/emutos/desk/deskact.c:211-214`.

Zwei weitere, kleinere: `gsx_attr`s `gl_mode`/`gl_lcolor`-Cache
(`aes/gemgraf.c:205-240`) ist nicht nachgebaut (spart VDI-Aufrufe, laesst die
Arbeitsstation im selben Zustand), und `ins_shortcut`s doppelte Icon-Suche —
siehe [DESK30](DESK30.md) — betrifft diese Schleife nicht.

**Nicht nachgebaut, weil unser `gr_obfind` es nicht erzeugen kann:** EmuTOS
liest bei `*pdobj == NIL` und `curr_sel == 0` `tree[-1]`
(`deskact.c:334-335`); unser `gr_obfind` liefert an dieser Stelle `root`, der
Fall ist von der ersten Abfrage mit abgedeckt.

**Gegenprobe auf Hardware:** Punkt 62 in
[docs/api/desktop/_hw-test-2026-09-17.md](../docs/api/desktop/_hw-test-2026-09-17.md)
— vor allem Punkt 4 (keine Spur beim langsamen Ziehen) und Punkt 5 (mehrere
Dateien gleichzeitig), beides mit einer echten Maus aussagekraeftiger als mit
dem Injektor. Spec:
[docs/api/desktop/deskact.md](../docs/api/desktop/deskact.md), Abschnitt
„`gr_drgplns` — the drag loop, and the target highlight".
