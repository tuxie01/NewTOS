# TOOL15 — Orakel: mtools pinnen, `--fast-forward` als Profilparameter

Status: **OFFEN** · Prioritaet: **P2** · Typ: Developer (Werkzeug) — Anwenderentscheidung 2026-10-28

## Sachverhalt

- `tools/mktestdisk.sh` nimmt mtools aus `PATH`
  (`tools/mktestdisk.sh:65@mformat`); `tools/oracle/oracle.env` sagt „nothing below
  may come from $PATH“. Die sha256-Pruefung des Abbilds faengt eine andere
  Version nur nachtraeglich ab (TOOL06-Quittung).
- `tools/hatari-oracle.sh:215-218@ORACLE_HATARI` setzt `--fast-forward on` fest.

## Arbeit

1. mtools-Pfad und -Version in `oracle.env`; `mktestdisk.sh` und jeder
   andere Orakel-Nutzer von mtools nimmt den Pfad von dort und prueft die
   Version.
2. `--fast-forward` als ausdruecklicher Profilparameter (Standard `on`, wie
   heute); der Wert steht im RUNINFO bzw. Manifest jedes Laufs.
3. Vergleich nur innerhalb derselben Einstellung: tosdiff (bzw. der
   Aufrufer) verweigert den Vergleich zweier Laeufe mit verschiedenem
   Wert, sofern der Wert in beiden Laeufen steht.
4. `docs/analysis/ORACLE.md` nachziehen.

## Abnahme

- Kanarienvogel aller Profile (ORACLE §6) unveraendert OK mit Standardwert;
  ein Lauf mit `off` ist im Manifest erkennbar und wird gegen einen `on`-Lauf
  nicht verglichen (belegt).
- doc-check 0 Fehler; `cargo xtask test` gruen.
