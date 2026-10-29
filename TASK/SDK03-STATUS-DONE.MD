# SDK03 — Quittung: mintlib `sysinfo(SI_PLATFORM)` mit NULL-Pruefung und Komma

Datum 2026-10-29 · Ticket [SDK03](SDK03.md) · Herkunft [ARCH05](ARCH05.md),
`docs/analysis/machine-identity-adr-proposal-2026-10-28.md` Abschnitt 2.1 ·
ADR-012 Nachtrag 2026-10-28 (`docs/MEMORY.md`)

## Patch (Overlay, Upstream unveraendert)

Neue Overlay-Datei `sdk/libc/overlay/unix/sysinfo.c` = unveraenderte
mintlib-Datei sysinfo.c (Unterordner unix) aus `sdk/libc/mintlib-0.60.1.tar.xz` (vorher `cmp`-gleich mit dem
extrahierten Baum) plus zwei benannte Patch-Stellen, je mit Kommentar
`NewTOS (TASK/SDK03), patch site N of 2`:

1. `platforms[]`: Komma hinter `"cfevb"`. Vorher verschmolzen `"cfevb"
   "atariclone"` zu einem Eintrag, das Feld hatte neun Eintraege, der
   `default`-Arm (`_MCH`-Familie > 5) las `platforms[9]` hinter dem Ende, und
   ColdFire-EVB hiess `"cfevbatariclone"`. Neu angehaengt: `"unknown"`
   (`_PLATFORM_NOHARD 10`).
2. `si_platform`, `case -1`: `else`-Zweig ohne ColdFire setzt
   `platform = "unknown"`, `hw_provider = "unknown"` (vorhandener
   `_HW_PROVIDER_UNKNOWN`). Vorher blieb `platform` NULL, danach
   `strlen(platform)`. Name nach EmuTOS: `MCH_NOHARD` = `0xFFFFFFFF`
   (`references/emutos/include/cookie.h:50`, gesetzt ohne
   `CONF_ATARI_HARDWARE` in `references/emutos/bios/machine.c:400-401`);
   `guess_machine_name()` (`references/emutos/bios/machine.c:807-821`) hat
   fuer `MCH_NOHARD` keinen `case` und liefert im `default` `"unknown"`.

`sdk/libc/build-libc.sh` kopiert das Overlay ueber den Baum; `libc.a` neu
gebaut (17 s), `unix__sysinfo.o` enthaelt `cfevb`, `atariclone`, `unknown`
als getrennte Zeichenketten.

## Test (SDK-Hostlauf)

`sdk/libc/tests/sysinfo-host.sh` + `sysinfo-host.c` +
`sysinfo-host-mintbind.h`, eingehaengt in `sdk/run-host-tests.sh` als
`sysinfo-host`. Uebersetzt das Overlay (unter AddressSanitizer) und das echte
`sdk/libc/newtos-arch/getcookie.c`; `Ssystem(S_GETCOOKIE)` spielt der Harness
wie der Kernel (`tos-api/src/gemdos/ssystem.rs:345@s_getcookie`: Zeigerform
schreibt vier Byte und liefert 0, Fehlschlag -1). Jeder Fall laeuft in einem
eigenen Kindprozess (`si_platform` merkt sich das Ergebnis statisch).
15 Faelle: kein `_MCH`, ST, STe, MSTe, TT, Falcon, FireBee (`_CF_`), Milan,
ARAnyM, Hades-Cookie, ColdFire-EVB (`0xFFFFFFFF` + `_CF_`), **`0xFFFFFFFF`
(NewTOS)**, **Familie 6, 0x7FFF, 0x8000**. Geprueft: Plattform, Anbieter und
beide Rueckgabewerte (`strlen + 1`).

- **Mit Patch:** `sysinfo-host: 0 of 15 cases failed`.
- **Ohne Patch** (`SYSINFO_SRC=` die Tarball-Datei, im Session-Scratch
  entpackt): `sysinfo-host: 5 of 15 cases failed`, Exit 5:
  - `_MCH 0xFFFFFFFF + _CF_ (ColdFire EVB)`: `got "cfevbatariclone" (return 16), want "cfevb" (return 6)`
  - `_MCH 0xFFFFFFFF (MCH_NOHARD, NewTOS)`: `AddressSanitizer: SEGV ... in _platform_strlen`
  - Familie 6, 0x7FFF, 0x8000: ebenso SEGV in `strlen`; clang warnt beim
    Uebersetzen `array index 9 is past the end of the array (that has type
    'const char *const[9]') [-Warray-bounds]` (undefiniertes Verhalten, im
    -O1-Bau endet es im selben `strlen`-Absturz).
  - Die zehn bekannten Werte bestehen auch ohne Patch.

## Zahlen

- `sdk/run-host-tests.sh`: `host tests: 28 passed, 0 failed, 0 skipped`
  (Basislinie 27 + `sysinfo-host`; das Ticket nennt 26, der Lauf vor SDK03
  hatte 27).
- `cargo xtask test`: 1824 passed, 0 failed, 1 ignored (keine Rust-Datei
  beruehrt).

## Upstream-Bericht (Vorschlag, nicht gesendet)

> **sysinfo(SI_PLATFORM): NULL dereference for _MCH = 0xFFFFFFFF without
> ColdFire, and missing comma in platforms[]**
>
> unix/sysinfo.c, si_platform():
>
> 1. `platforms[]` lacks a comma after `"cfevb"`, so `"cfevb" "atariclone"`
>    concatenate. The table has 9 entries instead of 10: `_PLATFORM_CFEVB`
>    yields "cfevbatariclone", and the `default:` arm
>    (`platforms[_PLATFORM_CLONE]`, index 9) reads past the end of the array
>    for any `_MCH` family other than -1..5 (gcc/clang warn with
>    -Warray-bounds).
> 2. `case -1:` sets `platform` only when a `_CF_` cookie exists. On a
>    machine reporting `_MCH = 0xFFFFFFFF` ("no ST-compatible hardware",
>    as EmuTOS does when built without Atari hardware support) and no `_CF_`,
>    `platform` and `hw_provider` stay NULL and the following
>    `strlen (platform)` dereferences NULL; `uname()` takes this path.
>
> Suggested fix: add the comma, and give `case -1` an else branch with a
> name, e.g. "unknown" (EmuTOS's machine_name() for that value) and
> `hw_providers[_HW_PROVIDER_UNKNOWN]`. Patch attached
> (diff of unix/sysinfo.c, two hunks).

## Dateien

- neu: `sdk/libc/overlay/unix/sysinfo.c`, `sdk/libc/tests/sysinfo-host.sh`
  (ausfuehrbar), `sdk/libc/tests/sysinfo-host.c`,
  `sdk/libc/tests/sysinfo-host-mintbind.h`, diese Quittung
- geaendert: `sdk/run-host-tests.sh`
- nicht geaendert: `docs/STATUS.md` (Lead), Ticket-Statuszeile in `TASK/SDK03.md`
