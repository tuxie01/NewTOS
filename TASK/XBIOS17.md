# XBIOS17 — qemu-virt: `Bconout(1)` verwirft, obwohl der Empfang jetzt arbeitet

Status: **OFFEN** · Prioritaet: **P3** · Typ: Developer — Anwenderentscheidung 2026-10-29, Fund aus [XBIOS15](XBIOS15.md); Entscheidung offen, erst Bericht

## Sachverhalt

- Seit XBIOS15 liest AUX auf qemu-virt aus dem Ring von Geraet 0
  (`kernel/src/console.rs@AuxSerial`); `Bconout(1)`/`Bcostat(1)` laufen
  weiter ueber `AuxNoCable` (`tos-api/src/bios/chardev.rs:120@AuxNoCable`:
  `Bconout` verwirft und meldet 1, `Bcostat` -1).
- **Bericht des Leads (2026-10-29), keine Entscheidung:**
  - Die UART ist geteilt: `SerialConsole` ist der einzige Sendeweg fuer
    `kprint!`/`kprintln!`, Programm-Konsolenausgabe, Panik-Handler und
    Selbsttests (`kernel/src/console.rs:106-111@SerialConsole`). `Bconout(1)`
    auf die UART wuerde sich mit dem Kernel-Log mischen.
  - Was es beruehren koennte: Laeufe, die das serielle Log nach Markern
    durchsuchen — `cargo xtask tosdump` (`scan_serial`,
    `tools/xtask/src/tosdump.rs:527@scan_serial`, „desktop up“, Panik-Zeilen),
    die Sondenskripte unter `tosdump/probes/` und Messlaeufe mit
    `-serial file:`. Diese laufen auf raspi4b; dort ist AUX „kein Kabel“ und
    bliebe es. Auf qemu-virt lesen `virtrun.py`-artige Laeufe das Log per
    Regex: ein Programm, das auf AUX schreibt, koennte Marker vortaeuschen
    oder Zeilen zerreissen.
  - EmuTOS gibt AUX-Ausgabe ueber `bconout1` an die serielle Schnittstelle
    (`references/emutos/bios/serport.c:279-317`); das Kernel-Log hat EmuTOS
    nicht auf dieser Leitung.

## Arbeit (nach Entscheidung)

0. Lokalisierung (Lead) nach CLAUDE.md „Testing“.
1. Entscheidung beim Anwender, z. B. `Bconout(1)` auf die UART (mit
   Trennung vom Kernel-Log, etwa Kernel-Log nur bei `serial=on`), oder
   weiter verwerfen als dokumentierte Abweichung.

## Abnahme

- Nach Entscheidung; Messfall gegen das Orakel (Hatari `--rs232-out`).
