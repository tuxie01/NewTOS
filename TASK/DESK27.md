# DESK27 — `docs/api/xbios/26-supexec.md` widerspricht sich

Status: DONE (2026-11-11) | Priority: P4 | Commit: f497ec8d7 | Receipt: DESK27-STATUS-DONE.MD

Migrated (owner batch 20, 2026-11-11, to DONE), evidence: receipt TASK/DESK27-STATUS-DONE.MD exists, commit f497ec8d7 (docs(xbios): the Supexec spec contradicted itself, the head was right), VERIFIED: f497ec8d7 (docs(xbios): the Supexec spec contradicted itself, the head ) added the receipt TASK/DESK27-STATUS-DONE.MD itself, the report of this research or measurement ticket (3 files), legacy line: ERLEDIGT (2026-09-23)

Line 3 before the migration:
Status: **ERLEDIGT (2026-09-23)** · Prioritaet: **P4** · Typ: Dokumentationsfehler — Nebenbefund DESK13

Abhaengigkeiten: —

## Sachverhalt

In `docs/api/xbios/26-supexec.md` sagt der Kopf richtig, der Dispatcher gebe
die Funktionsnummer zurueck; der Abschnitt *Pi / BCM2711 binding* behauptet
weiter `EINVFN`. Eine der beiden Aussagen ist falsch, und die Spezifikation
ist damit als Beleg unbrauchbar.

DESK13 hat die Stelle nur gelesen (um zu entscheiden, dass der `Default`-Knopf
den Keks ueber `Ssystem` liest statt ueber `Supexec`) und bewusst nicht
angefasst.

## Arbeit

Gegen `tos-api/src/xbios/mod.rs` pruefen, welches der beiden Verhalten der
Dispatcher wirklich zeigt, und die andere Stelle richtigstellen.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

## Abschluss

**ERLEDIGT (2026-09-23).** Der Kopf hatte recht, der Abschnitt *Pi / BCM2711
binding* war falsch.

Belegt am Quelltext: `xbios::dispatch` endet auf `other => other as i64`
(`tos-api/src/xbios/mod.rs`, Sammelzweig) — ein nicht belegter Steckplatz
liefert die **Funktionsnummer**, zero-extended, nicht `EINVFN`. `Supexec`
(0x26) hat keinen Steckplatz, ein Aufruf bekommt also `38`, und ein Aufrufer,
der auf `result < 0` prueft, sieht „kein Fehler". Das ist dieselbe Form wie
beim BIOS-Trampolin und das dokumentierte Atari-TOS-Verhalten
(`docs/api/xbios/_dispatcher.md` §3, D-008).

Die drei Stellen im Abschnitt, die `EINVFN` behaupteten, sind
richtiggestellt; der zurueckgenommene Wortlaut steht als Zitat dabei, damit
die Korrektur im Text steht und nicht nur in der Versionsgeschichte. Dazu ein
Satz, was das fuer einen Aufrufer **praktisch** heisst (der Rueckruf laeuft
nicht, und die Rueckgabe sagt es nicht) und wohin der eine Anwendungsfall
gegangen ist, den der Desktop wirklich brauchte: `Ssystem(S_GETCOOKIE)` fuer
den `_IDT`-Keks (DESK13).

Keine Codeaenderung, keine Messung — reine Dokumentationskorrektur.
