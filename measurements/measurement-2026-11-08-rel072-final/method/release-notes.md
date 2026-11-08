**NewTOS 0.7.2 — alpha for testers** · *Deutsch weiter unten*

Installation is unchanged: SD card with an **MBR** partition table and a first partition **FAT16, at most 2 GB**; copy the contents of the ZIP's `KARTE` folder onto it and boot. Commands for macOS, Windows and Linux are in `README.TXT`. Only the kernel changed since 0.7.1; the included programs are the same.

### New in 0.7.2 — operating system

- **VDI, speed:** the basic drawing routines now use the Pi's vector unit (NEON): filling areas, text and icons, copying screen areas (scrolling, moving windows), lines and the mouse pointer. Same pixels as before, checked picture by picture. Fbench on a Raspberry Pi 4 against the drawing code of 0.7.1: filling a whole window about 16×, opening and closing a window about 8×, text about 1.8–2.6×, scrolling and moving windows about 2×, showing/hiding the mouse pointer about 5× as fast.
- **VDI:** the mouse pointer is drawn once per screen frame, as in EmuTOS; the position a program asks for is always current.
- **VDI:** smaller corrections, each measured against EmuTOS (enlarged text over the window edge, `v_justified`, end of an XOR underline, short coordinate lists, mouse on `v_opnwk`/`v_clswk`, `vsc_form`, `Scrdmp`/`v_hardcopy` flag).
- **VDI:** a flood fill (`v_contourfill`) that never ends in EmuTOS now stops after a generous limit instead of freezing the machine.
- **VDI:** the desktop's mouse wheel no longer stops working after a program borrowed the wheel handler and gave it back.

SDK: no changes since 0.7.1.

**Please test:** the mouse pointer (desktop, menus, dragging, dialogs) and the speed. While a program is busy (e.g. Doom starting): does the pointer keep moving or stand still? Please tell us.

### Known limitations

No 68000 programs (.PRG/.TOS/.TTP/.ACC), one program at a time (plus accessories), no sound, no network, no battery clock, FAT16 only. Switch off only after a program has finished saving. The wheel of some wired mice does not scroll yet (#8). Menus needing several clicks (#19) we cannot reproduce — a short video helps. During one long system call (e.g. loading a program file) the pointer stands still for that moment.

### Reporting bugs

Please use **[Issues → New issue → Bug report](https://github.com/tuxie01/NewTOS/issues/new/choose)** (English or German). Please attach photos of crash reports.

Sources: this repository, tag v0.7.2.

---

**NewTOS 0.7.2 — Alpha für Tester**

Installation wie bisher: SD-Karte mit **MBR**-Partitionstabelle und erster Partition **FAT16, höchstens 2 GB**; den Inhalt des Ordners `KARTE` aus dem ZIP daraufkopieren und booten. Befehle für macOS, Windows und Linux stehen in `LIESMICH.TXT`. Seit 0.7.1 hat sich nur der Kern geändert; die mitgelieferten Programme sind dieselben.

### Neu in 0.7.2 — Betriebssystem

- **VDI, Tempo:** Die grundlegenden Zeichenroutinen nutzen jetzt die Vektoreinheit des Pi (NEON): Flächen füllen, Text und Icons, Bildschirmbereiche kopieren (Scrollen, Fenster verschieben), Linien und der Mauszeiger. Gleiche Pixel wie vorher, Bild für Bild geprüft. Fbench auf einem Raspberry Pi 4 gegen den Zeichencode von 0.7.1: ganzes Fenster füllen etwa 16×, Fenster öffnen und schließen etwa 8×, Text etwa 1,8–2,6×, Scrollen und Fenster verschieben etwa 2×, Mauszeiger zeigen/verbergen etwa 5× so schnell.
- **VDI:** Der Mauszeiger wird wie bei EmuTOS einmal je Bild gezeichnet; die Position, die ein Programm abfragt, ist immer aktuell.
- **VDI:** Kleinere Korrekturen, jede an EmuTOS gemessen (vergrößerter Text über den Fensterrand, `v_justified`, Ende einer XOR-Unterstreichung, kurze Koordinatenlisten, Maus bei `v_opnwk`/`v_clswk`, `vsc_form`, Flag von `Scrdmp`/`v_hardcopy`).
- **VDI:** Eine Flutfüllung (`v_contourfill`), die bei EmuTOS nie endet, hört jetzt nach einer großzügigen Grenze auf, statt den Rechner anzuhalten.
- **VDI:** Das Mausrad im Desktop fällt nicht mehr aus, nachdem ein Programm den Rad-Handler ausgeliehen und zurückgegeben hat.

SDK: keine Änderungen seit 0.7.1.

**Bitte testen:** den Mauszeiger (Desktop, Menüs, Ziehen, Dialoge) und das Tempo. Während ein Programm beschäftigt ist (z. B. Doom beim Start): Bewegt sich der Zeiger weiter oder steht er? Bitte melden.

### Bekannte Einschränkungen

Keine 68000-Programme (.PRG/.TOS/.TTP/.ACC), nur ein Programm gleichzeitig (plus Accessories), kein Ton, kein Netzwerk, keine Uhr mit Batterie, nur FAT16. Erst ausschalten, wenn ein Programm fertig gespeichert hat. Das Rad mancher Kabelmäuse scrollt noch nicht (#8). Menüs, die mehrere Klicks brauchen (#19), können wir nicht nachstellen — ein kurzes Video hilft. Während eines einzelnen langen Systemaufrufs (z. B. beim Laden einer Programmdatei) steht der Mauszeiger kurz still.

### Fehler melden

Bitte über **[Issues → New issue → Bug report](https://github.com/tuxie01/NewTOS/issues/new/choose)** (Deutsch oder Englisch). Fotos von Absturzberichten bitte anhängen.

Quellen: dieses Repository, Tag v0.7.2.
