# KEYS02 — Tastaturvektor (`Kbdvbase` / `kbdvec` / `ikbdsys`) fuer Programme (Luecke K4)

Status: PARKED (2026-11-11) | Priority: P1 | Reason: legacy, evidence unverified; receipt KEYS02-STATUS-DONE.MD exists | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 19, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: no cited file of the ticket/receipt is touched by f8b9116d0 (feat(sdk): Doom reads exact key press and release ), named by body

Line 3 before the migration:
Status: **ERLEDIGT — Pi-Abnahme 2026-10-04** ([Statusdatei](KEYS02-STATUS-DONE.MD); TT in Hatari bestanden) · Prioritaet: **P1** (Anwender 2026-10-04: „B richtig") · Typ: Fehlende TOS-Funktion — aus [KEYS01](KEYS01.md) / [PORT06](PORT06.md)

Abhaengigkeiten: [KEYS01](KEYS01.md) (Tastenwiederholung, erledigt)

## Warum

TOS meldet Programmen ueber GEMDOS/BIOS/AES nie, wann eine Taste
**losgelassen** wird. Doom muss das aus Wiederholungen raten. Folgen auf der
echten Pi (2026-10-04):

1. Laufen + Links/Rechts: „vorwaerts" endet, sobald eine zweite Taste
   gedrueckt wird, und kommt nicht wieder — TOS-konform, gegen EmuTOS
   gemessen ([KEYS01-SEQUENCE-CHECK](KEYS01-SEQUENCE-CHECK.MD)).
2. Antippen dreht zu weit: Doom haelt die Taste bis zum ersten
   Wiederholungsfenster (360 ms) fuer gedrueckt, dazu Dooms Dreh-Beschleunigung.

Atari-Spiele loesen das ueber den Tastaturvektor: `Kbdvbase()` liefert die
Vektortabelle, ueber `kbdvec` (ab TOS 2.0) bzw. `ikbdsys` sieht ein Programm
jedes Make **und** Break als Scancode. NewTOS hat das nicht: die Slots sind
0, liegen in EL1-Speicher, und der Dekoder ruft keinen Vektor
(`PORT06-KEYS-DIAGNOSIS.MD` Frage 4).

## Schritt 1 — Spec (Regel 5)

Aus EmuTOS (`bios/aciavecs.S`, `bios/ikbd.c`, `bios/xbios.c` `Kbdvbase`)
plus Querverweis Compendium/tos.hyp und FreeMiNT: Struktur, Aufrufkonvention,
wann welcher Vektor mit welchen Registern/Daten gerufen wird, was ein Programm
beim Einhaengen/Aushaengen tun muss, Verhalten beim Programmende. Dazu die
NewTOS-Abbildung: Rueckruf in EL0 (vorhandener Weg `exceptions.rs:538-605`,
G_USERDEF), LP64/ILP32, Aufrufzeitpunkt (gepollter Eingabe-Pump, KEYS01),
Wechselwirkung mit der Wiederholung (Stopp bei `ikbdsys`-Wechsel,
`bios/ikbd.c:451-460`). Freigabe durch den Anwender, dann Bau.

## Abnahme

Doom (Pi und TT) haengt sich an den Vektor und sieht Druecken/Loslassen exakt:
Laufen + Drehen gleichzeitig, Antippen dreht nur kurz. Normale Programme
unveraendert.
