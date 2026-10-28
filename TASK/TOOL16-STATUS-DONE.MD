# TOOL16 — Quittung: Dumper liest und schreibt durch zurueckgegebene Zeiger, Layout aus `docs/api/`

Datum 2026-10-29 · Beleg
[measurements/measurement-2026-10-29-tool16/](../measurements/measurement-2026-10-29-tool16/RUNINFO.TXT)
· Spec `docs/TOS-REFERENCE-DUMPER.md` Abschnitt 6, "Structures behind a pointer"
· Ticket [TOOL16](TOOL16.md)

- **Quelle des Layouts:** ein Block ```` ```tosdump-layout ```` in der
  Struktur-Spec (`docs/api/structures/iorec.md`: `iorec`, `extiorec`;
  `docs/api/structures/md.md`: `md`, `mpb`), eine Zeile
  `!layout <name> <m68k>/<LP64> <felder>` je Struktur, Feld
  `<name>:<typ>@<off m68k>/<off LP64>[:<klasse>]` (Typen `b w L l p`,
  `p><layout>` gefolgt, `[<layout>]` eingebettet). Die Falltabelle traegt
  eine Kopie; der Hosttest `layoutcheck` (`tosdump/host/layoutcheck.c`, in
  `make -C tosdump host-test`) weist jede Tabellenzeile ab, die nicht
  bytegleich eine Spec-Zeile ist. Der Lader prueft jede Zahl gegen C auf
  beiden ABIs (m68k WORD-Ausrichtung, LP64 natuerlich, keine Luecke, Groesse
  = Ende gerundet). Kein Layout im Dumper-Code.
- **Lesen:** `r:<layout>` (Rueckgabezeiger, nur mit `PTR_RAM`) und
  `<n>:<layout>` (Puffer `b:`); je Feld ein 64-Bit-Slot im Capture
  (Recordformat unveraendert). Einem Zeiger wird nur gefolgt, wenn er
  `PTR_RAM` besteht und die Struktur bis `0x80000000` endet
  (`td_layout_follow_refusal`); sonst kein Lesen, jeder Slot
  `0xB0F3B0F3_000000NN` mit Grund. tosdiff vergleicht feldweise nach der
  Klasse aus der Tabelle (Zeiger `PTR_RAM`, sonst `EXACT`, je Feld
  ueberschreibbar).
- **Schreiben (Nachtrag):** nur mit `!write return`;
  `+put <feld>=<feld>` direkt nach dem `r:<layout>`-Fall, ein Feld nicht-
  Zeiger auf ein anderes gleichen Typs, nur durch einen Zeiger, den die
  Folgeregel zulaesst; Ergebnis im LOG nach dem Lauf.
- **`Getmpb`** zugelassen: `b:`-Puffer, in einer `Super`-Klammer, als
  letzter Aufruf (wie die Sonde G des bios-core-Audits).
- **Neue Tabelle** `tosdump/cases/xbios-ptr-r1.tsv` (9 Faelle, 1 Vorstufe,
  CRC `87DF757E`): `Iorec(0/1/2)`, Flush `+put ibufhd=ibuftl` +
  `Bconstat(2)` + Rueckleser, `Getmpb` (`0:mpb`, MD ueber `mp_free`).
- **Orakel:** `tt` und `falcontc` je zweimal bytegleich (BIN und LOG),
  Selfcheck beider Referenzen besteht, tt gegen falcontc 0 von 9 Faellen
  verschieden. Referenzen `tosdump/reference/EMUTOS-1a7484ab-XBIOS-ptr`
  und `-XBIOS-ptr-falcontc`. Werte = Sonden I01-I08 / G00-G06.
- **NewTOS** (`cargo xtask tosdump`, QEMU raspi4b): 6 von 9 Faellen
  verschieden, wie erwartet bis XBIOS01/BIOS06 — `Iorec` liefert `0x0E`
  (Folgeregel: nicht gelesen, "below 0x400"; `+put` nicht geschrieben),
  `Getmpb`: `md_start` `0x00A00000_00400000` (PTR_RAM), `md_length` 0
  (NONZERO). Gleich: `Bconstat(2)`, beide `Super`.
- **Grenzen:** der Ring war am Orakel leer (Kopf = Schwanz = 0); der
  Flush-Fall misst "Schreiben angenommen, `Bconstat(2)` = 0", nicht das
  Leeren eines vollen Rings (braucht Tasteneingabe im Lauf). Kein
  NewTOS-Eingabe-Trace: `tools/biostrace-check.py` liest keine `+put`-
  (und keine `+set`-)Zeilen.
- `make -C tosdump host-test` gruen (Selfcheck 32 Referenzen, layoutcheck
  4 Layouts), `cargo xtask test` 1767/0/1 (kein Rust geaendert), alle
  Dumper bauen mit `-Werror`; bestehende BIOS-Tabellen am Orakel (tt) mit
  dem neuen Dumper 0 Faelle verschieden; doc-check der geaenderten Doku
  0 Fehler.
