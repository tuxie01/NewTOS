# DESK69 — EmuDESKs zwei Line-A-Leser 1:1 ueber VDI-Werte nachbauen

Status: **OFFEN** · Prioritaet: **P3** · Typ: Abweichung von EmuTOS — ADR-030 Q3 (Anwender 2026-10-26: „NEWDESK nimmt die VDI-Werte“)

## Sachverhalt

NewTOS hat keinen LINEA-Variablenblock (ADR-030, Accepted 2026-10-26,
`docs/MEMORY.md`; `docs/COMPATIBILITY.md` „No Line-A“). EmuDESK liest zwei
Felder daraus direkt (`#include "lineavars.h"`,
`references/emutos/desk/desksupp.c:56`, EmuTOS @ `1a7484ab`):

1. **`V_REZ_HZ` in `do_xyfix`** (`desk/desksupp.c:262-272`). Unter
   `CONF_WITH_3D_OBJECTS` (`:265-269`):
   `if (*px + gl_wbox + 2*ADJ3DSTD >= V_REZ_HZ) *px -= 16;` (`:267-268`) —
   der Fensterverschieber bleibt erreichbar. `ADJ3DSTD` = 2
   (`include/obdefs.h:136`), `gl_wbox` aus `graf_handle`
   (`include/gsxdefs.h:88`, `desk/deskmain.c:1956`).
   **NEWDESK** laesst den Schubs weg: `programs/desktop/src/main.rs:2814-2826`
   (Begruendung im Kommentar `:2817-2819`). Aufrufer:
   `programs/desktop/src/main.rs:2846`, `:2877`, `:2967`, `:3041`, `:3265`.
2. **`MOUSE_BT` in `get_key`** (`desk/desksupp.c:509-532`): rechte Taste =
   `'Q'`, linke = `' '` (`:515-518`), solange keine Taste anliegt. Erreicht
   ueber `user_input` (`:543`, `:555`) aus `show_buf`/`show_file`
   (`:724`, `:740`, `:819`, `CONF_WITH_SHOW_FILE`, ab `:667`) und aus
   `user_quit` (`:569-576`) im Druckpfad `print_buf`/`print_file`
   (`:595`, `CONF_WITH_PRINTER_ICON`, Block `:474-664`). Im Druckpfad ruft
   `user_quit` `get_key` nur bei anliegender Taste (`:571-572`); die
   `MOUSE_BT`-Schleife laeuft dort nicht an — wirksam ist der Leser nur in
   `show_file`. **NEWDESK** hat weder `show_file` noch `print_file` (kein
   Treffer in `programs/desktop/src/`); `show_file` wird bewusst nicht gebaut
   ([GUI01](GUI01.md), `docs/COMPATIBILITY.md` „no built-in file viewer“).

## VDI-Quelle der Werte (ADR-030, Entscheidung 3)

| EmuDESK liest | VDI-Weg | EmuTOS-Beleg | NEWDESK |
|---|---|---|---|
| `V_REZ_HZ` | `vq_extnd(h, 0)` `work_out[0]` + 1 (ebenso `v_opnvwk` `work_out[0]`) | `DEV_TAB[0] = V_REZ_HZ - 1` (`vdi/vdi_control.c:203`); `vdi_vq_extnd` kopiert `DEV_TAB` nach `INTOUT[0..44]` (`:626-657`) | Bindung `programs/newtos-gem/src/vdi.rs:266` (`work_out[i]` = `INTOUT[i]`, `i` < 45); Handle wie in `programs/desktop/src/gsx.rs:104-116` |
| `gl_wbox` | `graf_handle` (AES) | `desk/deskmain.c:1956` | `programs/newtos-gem/src/aes.rs:521-522` (`wbox`); `programs/desktop/src/lib.rs:317` liest ihn, `global::set_cell_metrics` (`:320`) speichert ihn nicht |
| `MOUSE_BT` | `vq_mouse` `INTOUT[0]` | `vdi/vdi_mouse.c:341-352` (`:347`) | Kernel `tos-api/src/vdi/mouse.rs:897-904`; **keine** `vq_mouse`-Bindung in `programs/newtos-gem/src/vdi.rs` |

## Auftrag

1. `do_xyfix` 1:1 nach `desk/desksupp.c:262-272` (ADR-022): `gl_wbox` wie
   EmuTOS als Global halten (aus dem vorhandenen `graf_handle`-Aufruf),
   `V_REZ_HZ` aus `vq_extnd(h, 0)[0] + 1` auf der Workstation des Desktops,
   `ADJ3DSTD` wie in `programs/desktop/src/inf.rs:31` importiert. Den Satz „is omitted“ im
   Kommentar `programs/desktop/src/main.rs:2817-2819` ersetzen.
2. `get_key`: nur falls `show_file` (oder `print_file`) je gebaut wird —
   dann `MOUSE_BT` per `vq_mouse` (Bindung in `programs/newtos-gem/src/vdi.rs`
   ergaenzen), keine Line-A-Nachbildung. Bis dahin nichts zu bauen; der
   Punkt haelt die Regel fest.
3. Beleg: ein Fenster so verschieben, dass `x + gl_wbox + 4 >= V_REZ_HZ`
   (nach der 16-Pixel-Ausrichtung); `x` vor/nach (`wind_get(WF_CXYWH)`) in
   QEMU raspi4b, Gegenprobe auf dem gepinnten EmuTOS-Orakel
   (`tools/hatari-oracle.sh`).

[Gemeinsame Regeln](README.md) gelten.
