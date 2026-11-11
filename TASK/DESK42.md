# DESK42 — Der `Show info`-Kasten sieht EmuTOS' nicht aehnlich

Status: PARKED (2026-11-11) | Priority: P3 | Reason: legacy, evidence unverified; receipt DESK42-STATUS-DONE.MD exists | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 16, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: no cited file of the ticket/receipt is touched by f7c61f863 (feat(desk): rebuild the Show-info dialogs as EmuTO), named by body

Line 3 before the migration:
Status: **IMPLEMENTIERT — HW-Abnahme offen** (2026-09-25) · Prioritaet: **P3** · Typ: Konformitaet — Pi-Runde 2026-09-25

Abhaengigkeiten: [DESK38](DESK38.md) (erledigt)

## Sachverhalt

DESK38 hat `Show info` gebaut — es **funktioniert** (auf der Pi geprueft:
Laufwerk, Papierkorb, Drucker, Verknuepfung). Der Anwender: *„funktioniert,
aber das Fenster hat mit dem von EmuTOS rein gar nichts gemeinsam."*

Es geht also nicht um das Verhalten, sondern um den **Baum**: EmuTOS' Kaesten
sind Ressourcenbaeume mit fester Anordnung — `ADDISKIN` (Laufwerk),
`ADFILEIN` (Datei/Ordner) und die uebrigen in `references/emutos/desk/desk_rsc.c`.
DESK38 hat sie funktional nachgebaut, nicht in ihrer Form.

## Arbeit

Die betroffenen Baeume aus `desk_rsc.c` auslesen und Objekt fuer Objekt
nachbauen — dieselbe Reihenfolge, dieselben Objektnummern, dieselben
Beschriftungen, wie es DESK03 fuer `ADINSDSK` und DESK05 fuer `ADDESKCF`
gemacht haben. Die 3D-Rollen nach der Tafel in `inf.rs` (DESK01b); ein
3D-Objekt waechst vor dem Zeichnen um 2 px je Seite, also EmuTOS' eigene
Masse nehmen statt zu draengen.

Abnahme: Screendump je Kasten gegen den entsprechenden Baum. Wo eine
Abweichung bleibt (etwa weil eine Zeile bei uns keinen Inhalt hat), wird sie
benannt statt stillschweigend gelassen.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

## Abschluss (2026-09-25)

`ADDISKIN` (`desk_rsc.c:709-768`, 10 Objekte, 304x200) und `ADFFINFO`
(`:622-703`, 14 Objekte, 320x256) sind Objekt fuer Objekt nachgebaut —
gleiche Reihenfolge, Objektnummern, Beschriftungen, Masse. `inf_file_folder`
traegt wieder EmuTOS' Signatur (`ppath, pf -> 0/1/-1`), `InfFF` entfaellt.

**Umfang minimal ueber „nur der Baum" hinaus, und begruendet:** der Baum
*enthaelt* `FFSKIP` und das Attribut-Radiopaar. Sie inert zu lassen waeren
Stubs (Regel 2), also sind sie verdrahtet. Damit schliessen sich DESK38s
zwei benannte Abweichungen: die Attributrueckschreibung (`dos_chmod`,
`deskinf.c:585-605`) und die `STRENAME`-Retry-Schleife (`:551-635`, jetzt
**in** `inf_file_folder` wie in der Quelle).

Neun QEMU-Laeufe, 0 `[EXC]`/`[PANIC]`, `fsck_msdos -n` sauber auf allen zehn
Abbildern; beide Baeume zur Laufzeit Objekt fuer Objekt ausgegeben
(`serial/dlg-dumps.txt`) und gegen `desk_rsc.c` gehalten. Basislinien
unveraendert: 944/0/1, 64/94 beide Boards, 80/125 beide Feature-Saetze.

**Was nur die echte Pi zeigt:** Mausklick auf einen Radioknopf, echte
Ruecktaste im Namensfeld, `count_ffs` ueber die vier echten Partitionen
(Checklistenpunkt 69) — dort ist die **Dauer** des Laufs der eigentliche Punkt.

**Herausgefallen als eigenes Ticket:** [FS06](FS06.md) — `Frename` auf einen
leeren Namen liefert `E_OK` und legt einen leeren 8.3-Eintrag an.
