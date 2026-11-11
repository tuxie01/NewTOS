# DESK41 — `TERADESK.INF` gehoert ins Kartenabbild, nicht auf die Karte

Status: PARKED (2026-11-11) | Priority: P2 | Reason: legacy, evidence unverified; receipt DESK41-STATUS-DONE.MD exists | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 19, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: no cited file of the ticket/receipt is touched by 337f3553c (feat(tools): make-sd-image.sh generates TERADESK.I), named by body

Line 3 before the migration:
Status: **ERLEDIGT** (HW-Abnahme aus TeraDesk 2026-10-01, [Statusdatei](DESK41-STATUS-DONE.MD)) — implementiert 2026-09-29 · Prioritaet: **P2** · Typ: Werkzeugkette + Ergaenzung — Pi-Runde 2026-09-25

Abhaengigkeiten: —

## Was passiert ist

TeraDesk kannte nach dem Kartenaufraeumen unsere Dateiendungen nicht mehr.
Ursache: **der Lead hat `TERADESK.INF` geloescht** — sie sah wie eine alte
Desktop-Variante aus, war aber TeraDesks Konfiguration.

Die Datei registriert die Endungen in `ptype`-Bloecken:

```
	ptype=
	{
		name=*.NAP
		flag=0x2001
	}
```

Belegt: `*.NAP` und `*.NCP` stehen darin
(`/Volumes/Daten/Development/newtos-card-archive-2026-09-24/C-BOOT/TERADESK.INF`,
322 Zeilen). Eine Kopie liegt in `measurements/hw-round-2026-09-24/TERADESK.INF`.

## Zwei Befunde, nicht einer

1. **Sie war handgepflegt und lag nur auf der Karte.** Damit ist sie bei
   jedem Neubespielen weg — der Fehler war unvermeidlich, nur der Zeitpunkt
   war Zufall. `tools/make-sd-image.sh` muss sie **erzeugen**, wie es
   `NEWDESK.INF`s Inhalt und die Programme erzeugt. Solange sie nicht im
   Abbildbau steht, ist sie nicht reproduzierbar.
2. **Sie kennt nur `.NAP` und `.NCP`.** `.NTP` (nimmt eine Kommandozeile) und
   `.NAC` (Accessory) fehlen — die gab es zum Zeitpunkt der letzten
   Handpflege noch nicht. ADR-018 fuehrt inzwischen alle vier.

## Arbeit

1. Die Erzeugung nach `tools/make-sd-image.sh` ziehen, mit allen **vier**
   Endungen der ADR-018-Familie. Vorher an TeraDesks Quelltext
   (`sdk/apps/teradesk`) klaeren, was die `flag`-Werte bedeuten und welcher
   fuer `.NTP` und `.NAC` richtig ist — `0x2001` fuer `.NAP` nicht blind
   uebernehmen.
2. Klaeren, ob TeraDesk die Datei im Wurzelverzeichnis sucht oder neben
   seinem Programm; die Karte hatte sie in der Wurzel.

Abnahme: frisches Abbild bauen, TeraDesk starten, ein `.NAP`, ein `.NCP` und
ein `.NTP` zeigen ihr Programmsymbol und starten per Doppelklick.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

---

## Abschluss (2026-09-29)

`tools/make-sd-image.sh` **erzeugt** `TERADESK.INF` — eine geaenderte Datei,
+412/−2. Die Datei landet in der Wurzel **und** neben `DESKTOP.NAP` in
`C:\Apps\TERADESK\`, aus demselben Heredoc, sie koennen also nicht
auseinanderlaufen.

### Die offene Frage des Tickets, aus TeraDesks Quelltext beantwortet

| Endung | `appt` | `flag` | Beleg |
|---|---|---|---|
| `*.NAP` | — | `0x2001` | Kartenstand **und** TeraDesks `.PRG`/`.APP`-Vorgabe (`prgtype.c:346-347`) |
| `*.NCP` | 3 (`PTOS`) | `0x2000` | `.TOS`-Vorgabe, ebenda |
| `*.NTP` | 4 (`PTTP`) | `0x2000` | `.TTP`-Vorgabe, ebenda |
| `*.NAC` | 2 (`PACC`) | `0x2000` | `.ACC`-Vorgabe, ebenda |

`ApplType` aus `desk.h:130-137` (`PGEM=0, PGTP=1, PACC=2, PTOS=3, PTTP=4`),
Flag-Bits aus `prgtype.h:42,55` (`PT_ARGV=0x0001`, `PD_PDIR=0x2000`, also
`0x2001 = PD_PDIR|PT_ARGV`). Der Lead hat alle fünf Fundstellen nachgesehen.

**`0x2001` wurde nicht blind uebernommen, und das ist der Punkt:** `.NTP` und
`.NAC` bekommen **kein** `PT_ARGV`, weil TeraDesks eigene `.TTP`- und
`.ACC`-Vorgaben es nicht haben. Mit gesetztem Bit wuerde der Schwanz als
ARGV signalisiert (`cl.length = 127`, `startprg.c:654-655`) statt gewoehnlich
uebergeben.

Die Wahl traegt nebenbei zweimal das gewuenschte Verhalten: `PTTP` loest
TeraDesks Kommandozeilen-Dialog aus (`applik.c:1390`), `PACC` laesst einen
Doppelklick auf ein Accessory **verweigern statt starten**
(`startprg.c:644`, `alert_iprint(MCANTACC)`) — was ADR-018/ACC02 verlangen.

### Zweite Frage: Wurzel oder neben dem Programm

**Beides, Programmverzeichnis zuerst.** TeraDesk loest `"teradesk.inf"`
ueber `shel_find` auf (`main.c:1111-1132`); `shel_find` sucht
Anwendungsverzeichnis, dann aktuelles Verzeichnis, dann Wurzel (EmuTOS
`findfile`, `aes/gemshlib.c:406-470`; bei uns
`tos-api/src/aes/shell.rs::sh_find`).

### Umfang: ein Punkt mehr, begruendet

`CMDLINE.NTP` liegt jetzt auf der Karte — dieselbe gebaute `cmdline.c` ein
zweites Mal unter `.NTP`. Grund: `.NTP` war das einzige Familienmitglied
**ohne jede Datei auf der Karte**, die Abnahme oben verlangt aber einen
Doppelklick darauf. Der Lead haelt das fuer im Umfang, nicht darueber
hinaus: ohne die Datei ist die Abnahme nicht durchfuehrbar.

### Belege

**Vom Lead unabhaengig nachgepruefte Punkte:** frisches Abbild gebaut,
`TERADESK.INF` per `mtools` ausgelesen — 3058 Bytes, 339 Zeilen, LF (kein
CR), erste Zeile `TeraDesk-inf`, Abschluss `end`, alle vier `*.N*`-Bloecke
mit den obigen Werten; `fsck_msdos -n` Phasen 1–3 sauber; die vier Dateien
`ARGVTEST.NAP`, `CMDLINE.NCP`, `CMDLINE.NTP`, `SYSACC2.NAC` liegen in der
Wurzel. Basislinien **unveraendert**: 962/0/1, 64/94 beide Boards, 80/125
beide Feature-Saetze (keine Rust-Datei angefasst).

**Gegenprobe des Developers in QEMU raspi4b** (headless, `#Z`-Autorun,
Bildabzug): mit der erzeugten Datei zeigt TeraDesk vier HARD-DISC-Symbole
(C–F) und TRASH auf einfarbigem Gruen — also den `deskicons`-Abschnitt der
Datei; **dieselbe Karte ohne** `TERADESK.INF` zeigt FLOPPY A/B, ein HARD
DISC, PRINTER auf gemustertem Gruen, also `dsk_default()`
(`icon.c:2931-2962`). Damit ist belegt, dass die Datei **gefunden und
gelesen** wird. Keiner der drei Beschwerde-Alerts (`AUNKRTYP`, `AVALIDCF`,
`BADCFG`) erschien — die Datei wurde also vollstaendig fehlerfrei geparst,
einschliesslich `appt=2` und `appt=4`.

### Was nur die echte Pi zeigt

Die eigentliche Abnahme: `.NAP`, `.NCP`, `.NTP` im geoeffneten `C:\`-Fenster
mit **Programmsymbol**, Start per Doppelklick. Dafuer muss ein Fenster
geoeffnet werden, also Klick. Dazu der Vier-Partitionen-Fall — die
`deskicons` nennen C–F, im Ein-Partitions-Abbild sind drei davon tote
Laufwerke, auf der Karte sind sie richtig.

### Herausgefallen als eigene Tickets

- [DESK49](DESK49.md) — der `filetypes`-Abschnitt (Maskenliste des
  Fenster-Popups) kennt die vier Endungen weiterhin nicht.
- [PORT09](PORT09.md) — `PT_ARGV` bei `*.NAP` verspricht die
  ARGV-Konvention, unser `cmdargs.c` kennt sie nicht.

---

## Nachtrag 2026-09-30 — die Abnahme ist noch **nicht** gefahren

Die Pi-Runde vom 2026-09-30 (Kern `2597951a`, Karte frisch aus
`make-sd-image.sh`) hat die vier Endungen **aus unserem eigenen Desktop**
geprueft, nicht aus TeraDesk. Der Anwender hat das auf Rueckfrage bestaetigt.

**Das ist nicht die Abnahme dieses Tickets.** `TERADESK.INF` konfiguriert
**TeraDesk**; unser Desktop liest sie nicht und kann sie nicht lesen. Was
beobachtet wurde, sagt ueber die erzeugte Datei nichts aus.

Was die Runde stattdessen belegt hat: `.NTP` startet aus unserem Desktop ohne
Parameterdialog — das ist [DESK47](DESK47.md), dort bestaetigt.

**Offen bleibt woertlich die Abnahme oben:** TeraDesk starten, `C:` oeffnen,
und dort muessen `.NAP`, `.NCP` und `.NTP` ihr **Programmsymbol** tragen und
per Doppelklick starten — bei `.NTP` mit TeraDesks eigenem
Kommandozeilen-Dialog davor (`applik.c:1390`), bei `.NAC` mit der Weigerung
(`startprg.c:644`). Die QEMU-Gegenprobe (Datei wird gefunden, gelesen und
fehlerfrei geparst) traegt das Ticket bis dahin, ersetzt es aber nicht.
