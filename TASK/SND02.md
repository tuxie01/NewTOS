# SND02 — HDMI-Ton: die Quellenlage ist ungeklaert, nicht guenstig

Status: **OFFEN** · Prioritaet: **P2** (gehoert zu 1.0, Anwender 2026-10-23; vorher P3) · Typ: Track-B-Recherche — aus [SND01](SND01.md) Schritt 2

Abhaengigkeiten: [SND01](SND01.md) (Klinke zuerst; HDMI ist der zweite Ausgang)

## Warum dieses Ticket existiert

Der Anwender will den Ton **ueber HDMI** (2026-09-28), Klinke als Rueckfall.
Der Lead hat daraufhin aus dem Geraetebaum geschlossen, HDMI-Ton sei „ein
normaler Treiber gegen dokumentierte Register statt eines Firmware-Handels".

**Das ist widerrufen.** Track B hat es geprueft: das **BCM2711-Datenblatt
(Rel. 4) hat gar kein HDMI-Kapitel** — dreizehn Kapitel, keines davon HDMI.

## Was der Geraetebaum zeigt und was er nicht zeigt

**Zeigt:** der HDMI-Knoten hat einen eigenen Audiotakt, einen eigenen
DMA-Kanal und benannte Registerbereiche (`hdmi`, `packet`, `metadata`, `hd`).

**Zeigt nicht:** ob diese Register irgendwo **dokumentiert** sind. Ein
Geraetebaum ordnet Adressen und Namen zu; er ist keine Registerbeschreibung.
Der Lead hat aus dem einen auf das andere geschlossen — das war der Fehler.

Zwei Beobachtungen, die eher in die andere Richtung deuten und **nicht**
weiterverfolgt wurden: der DMA-Name des Knotens ist `audio-**rx**` (nicht
`tx`), und die Bootkarte traegt `snd_bcm2835.enable_hdmi` in ihren Schaltern —
also einen **Firmware**-Schalter.

## Auftrag

1. **Feststellen, ob es ueberhaupt eine Primaerquelle gibt.** Raspberry-Pi-
   Dokumentation, VideoCore-Unterlagen, alles was sich pinnen laesst. Gibt es
   keine, ist das die Antwort — und dann ist HDMI-Ton bei uns eine
   Reverse-Engineering-Aufgabe und keine Treiberaufgabe.
2. **Den Firmware-Weg pruefen.** `snd_bcm2835` und der `enable_hdmi`-Schalter
   deuten darauf, dass der Ton normalerweise durch die Firmware geht. Ob es
   dafuer eine Mailbox-Schnittstelle gibt, die wir bedienen koennen, ist
   ungeklaert — und waere der weit billigere Weg.
3. Erst danach entscheiden, ob HDMI-Ton fuer NewTOS realistisch ist.

**Nicht anfangen zu bauen, bevor Punkt 1 beantwortet ist** (Regel 6: keine
erfundene Hardware).

## Abnahme

Eine Antwort auf „gibt es eine Quelle" — ja mit Pin, oder nein mit Begruendung.
Kein Code.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

---

## Nachtrag 2026-09-29 — die Frage „gibt es eine Quelle" hat eine Antwort

Dieses Ticket stellte die Frage, ob fuer HDMI-Ton ueberhaupt eine Quelle
existiert. **Ja — aber keine primaere.**

`references/circle` @ `6177984e` enthaelt `lib/sound/hdmisoundbasedevice.cpp`
(794 Zeilen) mit einer **eigenen Registerkarte fuer RPi 4**: der Quelltext
unterscheidet `RASPPI <= 3`, `== 4` und `== 5` an sechzehn Stellen und fuehrt
die HDMI-Audioregister relativ zu `ARM_HDMI_BASE` — Audio-Paket-Konfiguration,
CRP/CTS-Taktrueckgewinnung, Kanalstatus. `README.md:90` fuehrt „HDMI sound
output (without VCHIQ)" als unterstuetzt.

**Was das aendert und was nicht.**

- Die Aussage des Leads *„das BCM2711-Datenblatt hat kein HDMI-Kapitel"*
  bleibt **wahr**. Es gibt weiterhin **keine primaere** Quelle.
- Es gibt aber eine **getestete tertiaere**. Damit ist HDMI-Ton nicht mehr
  „Quellenlage ungeklaert", sondern „nur Rang 6 belegt" — eine andere,
  schwaechere, aber benennbare Lage.
- Regel 6 („No invented hardware") ist damit **nicht** erledigt. Rang 6 traegt
  eine Gegenprobe, keine Autoritaet. Ob dieses Projekt einen Treiber auf einer
  reinen Rang-6-Belegung baut, ist eine Entscheidung, keine Ableitung — und
  sie faellt, wenn dieses Ticket drankommt, nicht hier.

**Dritter Weg bleibt offen und ist weiter der billigere:** circles Klasse
umgeht VCHIQ ausdruecklich. Ob die Firmware eine Mailbox-Schnittstelle fuer
HDMI-Audio anbietet, ist damit **nicht** beantwortet — Punkt 2 der
Arbeitsliste oben steht unveraendert.

**Reihenfolge:** unveraendert nach [SND01](SND01.md). HDMI ist mit 794 gegen
257 Zeilen der groessere Brocken, nicht der kleinere. Und: HDMI0 only auf
RPi 4 (`include/circle/sound/hdmisoundbasedevice.h:41`).
