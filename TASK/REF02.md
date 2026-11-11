# REF02 — Die SD-Spezifikationen fehlen in `references/`, und die Hardwarenotiz nennt keine Zeitgrenzen

Status: PARKED (2026-11-11) | Priority: P2 | Reason: legacy, evidence unverified; receipt REF02-STATUS-DONE.MD exists | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 19, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: no cited file of the ticket/receipt is touched by f3a468fb2 (fix(fs,emmc): a failed write now reaches the appli), named by body

Line 3 before the migration:
Status: **ERLEDIGT** (2026-09-28) · Prioritaet: **P2** · Typ: Quellenlage (Researcher) — aus dem EMMC-Audit

Abhaengigkeiten: `docs/api/hw/emmc-audit.md` (Befund S1)

## Warum

Das EMMC-Audit hat einen Treiberfehler gefunden, der eine Zeitgrenze
verletzt — und dann festgestellt, dass diese Zeitgrenze **nirgends bei uns
aufgeschrieben ist**:

* Die SD-Spezifikationen (Physical Layer Simplified, Host Controller
  Simplified) liegen **nicht** in `references/`, anders als EmuTOS und
  FreeMiNT.
* `docs/hardware/emmc-sd.md` nennt **keinerlei** Zeitgrenzen.

**Der Treiber konnte sie nicht einhalten, weil sie nicht da waren.** Das ist
genau der Fall, den `docs/SOURCE-ANALYSIS-PROTOCOL.md` Track B verhindern
soll, und er hat uns die FS04-Sekunden gekostet.

## Auftrag

1. Die beiden SD-Spezifikationen nach `references/` holen und **pinnen**, wie
   EmuTOS und FreeMiNT — mit Fassungsnummer, damit ein Zitat spaeter
   nachpruefbar bleibt. Lizenz- und Weitergabefrage dabei klaeren und
   festhalten (die „Simplified Specifications" sind dafuer der uebliche Weg).
2. `docs/hardware/emmc-sd.md` um die Zeitgrenzen ergaenzen, je mit
   Abschnittsangabe: Programmierzeit der Karte, Busy nach dem letzten Block,
   Lese-Zeitgrenze, die Empfehlung an den Host. Das Audit hat sie bereits
   zusammengetragen — von dort uebernehmen **und gegen die Quelle pruefen**.
3. **Zwei Korrekturen an `docs/hardware/emmc-sd.md`**, vom Audit gemeldet:
   * §3 gibt fuer ACMD41 `0x5100_0000` an — **ohne Spannungsfenster**. Der
     Treiber macht es richtig; die Notiz ist falsch.
   * Die Behauptung, `0xFE300000` sei „sdhost → Wi-Fi", ist ungeprueft. Fuer
     uns folgenlos, weil wir beide Basen probieren — aber eine ungepruefte
     Behauptung in einer Hardwarenotiz ist eine Falle fuer den naechsten.

## Nachtrag 2026-09-27 aus HW03

HW03 brauchte ein Zitat je R1-Fehlerbit und stand vor derselben Luecke. Der
Entwickler hat sich die **Physical Layer Simplified Specification Ver. 9.10**
von der oeffentlichen Downloadseite der SD Association geholt und unter
`references/sd-spec/` abgelegt — das Verzeichnis ist **gitignored**
(`/references/*`), die Fassung ist also **nicht** gepinnt und liegt nur auf
diesem Rechner.

**Zusaetzliche Aufgabe fuer dieses Ticket:** das Audit
(`docs/api/hw/emmc-audit.md`) zitiert **Ver. 6.00**, HW03s Bitpositionen
stammen aus **9.10**, und die beiden sind **nicht gegeneinander geprueft**.
Beim Pinnen ist also zu entscheiden, welche Fassung die Referenz ist, und die
Zitate der anderen sind nachzuziehen.

## Abnahme

`references/` traegt die gepinnten Spezifikationen; `docs/hardware/emmc-sd.md`
traegt die Zeitgrenzen mit Abschnittsangabe und ist um die beiden Fehler
bereinigt. Keine Codeaenderung.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

## Abschluss (2026-09-28)

**Referenzfassungen:** Physical Layer **Ver. 9.10**, Host Controller
**Ver. 3.00** — letzteres nicht aus Bequemlichkeit: EMMC2 meldet seine
HC-Fassung in `SLOTISR_VER[23:16]`, und der einzige gemessene Wert ist `0x02`
= HC 3.00 (QEMU; auf echter Pi weiterhin ungemessen). 4.20 bringt
UHS-II/ADMA3/64-Bit, die EMMC2 uns nicht anbietet.

**Die 6.00-gegen-9.10-Frage ist beantwortet, und HW03 kommt unbeschadet
durch:** beide PDFs extrahiert und Text gegen Text verglichen — §4.6.2 samt
Unterabschnitten und §4.10.1 sind **wortgleich**, Tabelle 4-42 hat
**identische Bitbelegung und Typspalte**. HW03s `R1_ERROR_MASK = 0xFDF9_8008`
ist aus der Typ-E-Menge nachgerechnet und gegen 6.00 genauso richtig. **Kein
Zitat umzuschreiben.**

**Dafuer beim Host Controller ein Fund:** 3.00 gegen 4.20 sind **alle**
Registerabschnitte um eins verschoben (Software Reset §2.2.16 → §2.2.17,
Capabilities §2.2.25 → §2.2.26 und so weiter). Umrechnungstabelle in
`emmc-sd.md` §8, mit der Regel „immer die Ausgabe vor die Abschnittsnummer".
Eine Simplified 4.00/4.10 existiert nicht.

**Lizenz im Wortlaut gelesen:** §2 erlaubt Nicht-Mitgliedern Herunterladen,
Ansehen und Drucken fuer den internen Gebrauch, verbietet aber ausdruecklich
*„copy portions … (whether by screen scraping, screen shots, electronically or
otherwise)"* und *„distribute the Simplified Specs or any portion thereof"*.
§3 (Auszuege in eigene Doku) gilt **nur Mitgliedern**.

**Pinnen kann deshalb nicht Ablegen heissen.** Der Pin ist eine Tabelle
(Ausgabe, Datum, kanonische Quelle, **SHA-256**, Bytegroesse) in der
committeten `references/README.md` — dieselbe Stelle, an der EmuTOS und
FreeMiNT ueber ihre SHA gepinnt sind. Dazu der Bezugsweg (die Site ist
Cloudflare-geschuetzt, nackter `curl` liefert 403) und eine Ein-Zeilen-Pruefung.

**Zeitgrenzen** stehen jetzt in `emmc-sd.md` §7, **jede gegen die Quelle
geprueft** statt aus dem Audit uebernommen — samt einer Praezisierung: die
500 ms sind ausdruecklich **SDXC/SDUC** zugestanden, fuer SDHC bleiben 250 ms.
Folgenlos fuers Ergebnis (der Application Note verlangt > 500 ms unabhaengig
von der Klasse), aber in einer Hardwarenotiz muss die Einschraenkung mit.

**Ehrlich markierte Luecke (§7.5):** die Bus-Zeitwerte `N_CR`/`N_AC` sind in
der *Simplified*-Fassung **nicht enthalten** (§4.12 ist als „Removed in the
Simplified Specification" gefuehrt). Die Aussage „N_CR ≤ 64 Takte" in einem
Treiberkommentar hat in unseren gepinnten Dokumenten **keinen Beleg**.

**Die zwei gemeldeten Fehler, beide bestaetigt, einer schaerfer als gedacht:**

* **ACMD41 `0x5100_0000`** ist nicht bloss unvollstaendig, sondern ein
  **anderes Kommando**: Spannungsfenster = 0 ist nach §4.2.3.1 (1) das
  **inquiry ACMD41** — startet die Initialisierung nicht, liefert nur das OCR
  und **ignoriert ausdruecklich** Bits 31:24, also auch `HCS`. Ein Host, der
  darauf das Busy-Bit abwartet, wartet ewig. Der Treiber macht es richtig.
* **`0xFE300000` = „sdhost → Wi-Fi":** der **Name** ist falsch (SDHOST liegt
  auf `0xFE20_2000` und ist auf der Pi 4 `disabled`), die **Sache** stimmt —
  belegt aus dem Geraetebaum der **Pi-Firmware selbst**
  (`bcm2711-rpi-4-b.dtb` unserer Bootkarte): `mmcnr@7e300000`, `non-removable`,
  Kind `wifi@1` `brcm,bcm4329-fmac`; der Kartenschacht ist `mmc@7e340000`.

**Nebenertrag:** die Quellentabelle fuehrte „BCM2711 §1.2 (address map)" als
Quelle der EMMC2-Basis. Das Datenblatt enthaelt diese Basis **nicht** — es hat
gar kein EMMC-Kapitel, nur die Umrechnungsregel §1.2.4. Die Basis kommt aus
dem Firmware-DT und circle.

## Herausgefallen

* **[REF03](REF03.md)** — drei falsche Zitate, zwei davon im Produktcode.
* **[REF04](REF04.md)** — die woertliche Tabelle 4-42 in `emmc-sd.md` §5 gegen
  die Lizenz; Anwenderentscheidung.
* Offen und ausserhalb des Auftrags: **BCM2835 ARM Peripherals** liegt
  weiterhin nicht in `references/` (Quelle 2 des Audits, einzige Quelle fuer
  die 32-Bit-Registergruppierung); `docs/SOURCE-ANALYSIS-PROTOCOL.md` Track B
  fuehrt die SD-Spezifikationen in seiner Quellenhierarchie **nicht** auf.
