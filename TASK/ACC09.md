# ACC09 — `CONFIG.NAC`: die `config.txt` aus dem laufenden System einstellen

Status: PARKED (2026-11-11) | Priority: P3 | Reason: owner decision 2026-11-11 at the legacy migration: not on the work queue | Reopen: a card program or port is affected, or the owner schedules it

Migrated (owner batch 9, 2026-11-11), evidence: TASK/ACC09.md status line: OFFEN, no DONE receipt, no fix/feat commit names the id

Line 3 before the migration:
Status: **OFFEN** · Prioritaet: **P3** · Typ: Werkzeug (Accessory) — Anwenderwunsch 2026-09-25

Abhaengigkeiten: [ACC02](ACC02.md)/[ACC03](ACC03.md)/[ACC04](ACC04.md)
(erledigt — Accessories laufen), [X02](X02.md) (Aufloesungswechsel),
[HW02](HW02.md) (gemeldeter gegen gemessener Takt)

## Die Idee

Ein Accessory, das die `config.txt` der Bootpartition liest, ihre Schalter als
Dialog anbietet und sie zurueckschreibt. Damit stellt man Dinge ein, die heute
nur am Mac gehen — allen voran die **Aufloesung**, dazu Takt und Spannung.

Der Anwender hat es als Idee eingebracht; der eigentliche Wert ist, dass die
Maschine sich damit ohne zweiten Rechner konfigurieren laesst.

## Was wirklich drinsteht

Die Karte vom 2026-09-24 traegt 14 wirksame Zeilen
(`/Volumes/Daten/Development/newtos-card-archive-2026-09-24/C-BOOT/config.txt`):

```
arm_64bit=1            kernel=kernel8.img          <- Bootkette, unantastbar
hdmi_group=2           hdmi_mode=82                <- die Aufloesung
hdmi_force_hotplug=1   hdmi_pixel_encoding=2
gpu_mem=256
dtparam=audio=off      dtparam=i2c_arm=off         dtparam=spi=off
enable_uart=1          uart_2ndstage=1             init_uart_clock=48000000
force_turbo=0
```

`arm_freq` und `over_voltage` — die Takt- und Spannungsschalter, die der
Anwender genannt hat — stehen **nicht** drin; das Werkzeug muesste sie
anlegen.

## Drei Dinge, die vor dem Bauen geklaert sein muessen

1. **Die `config.txt` ist die Bootdatei.** Eine kaputte Datei heisst: die Pi
   startet nicht mehr, und die Reparatur braucht den Mac — genau das, was das
   Werkzeug ueberfluessig machen soll. Schreibweg deshalb: Sicherung anlegen,
   in eine Nebendatei schreiben, erst dann umbenennen; `arm_64bit` und
   `kernel=` gar nicht erst zur Bearbeitung anbieten.
2. **`cmdline.txt` lesen wir heute nicht.** Im ganzen Baum kommt sie nicht
   vor (geprueft 2026-09-25). Sie in das Werkzeug aufzunehmen waere also
   folgenlos, solange der Kernel sie ignoriert — entweder wird das ein
   eigenes Ticket, oder die Datei bleibt draussen. **Nicht** stillschweigend
   mitanbieten.
3. **Takt und Spannung koennen Hardware beschaedigen.** Das ist die eigene
   Maschine des Anwenders und auf der Pi gaengige Praxis, also keine Frage des
   Ob — aber das Werkzeug zeigt bekannte Bereiche, schreibt **nie** einen
   Wert, den der Benutzer nicht selbst gewaehlt hat, und sagt vor dem
   Speichern, dass ein Neustart noetig ist.

## Zusammenhang mit X02 und HW02

* **X02** waegt zwei Wege fuer den Aufloesungswechsel ab: `config.txt` plus
  Neustart gegen einen Laufzeitwechsel per Mailbox. Dieses Werkzeug **ist**
  die Bedienoberflaeche des ersten Weges. X02 sollte vorher entschieden sein,
  sonst baut man die Oberflaeche zum falschen Mechanismus.
* **HW02** fragt, ob wir den *eingestellten* oder den *gemessenen* Takt
  melden. `force_turbo=0` steht heute in der Datei, der Takt schwankt also —
  wer ihn hier einstellt, will hinterher den echten Wert sehen. Die beiden
  Tickets gehoeren zusammen gelesen.

## Arbeit

Erst X02 entscheiden. Dann: Datei lesen (GEMDOS ueber die Accessory-Seite),
Schalter in einen Dialog, sicher zurueckschreiben, Neustarthinweis.
Abnahme auf echter Hardware — eine Aufloesung umstellen, neu starten, das
Ergebnis sehen.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.
