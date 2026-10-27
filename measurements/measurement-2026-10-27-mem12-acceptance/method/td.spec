# MEM12 TeraDesk (#Z), armed up:DESKTOP: drive C: double-click (geometry measurement-2026-10-21-gh1-reg/method/td.spec),
# Shift+Esc twice (force_mediach: Super(0L) .. hdv_* .. Super(stack)), then Ctrl+Q -> Quit dialog -> "Reboot"
# (geometry measurement-2026-10-17-rst01/scripts/tdr.spec: Super(0L), memvalid/... = 0, Supexec(reset stub)).
at 600
park
move 195 40
wait 50
dclick
wait 300
mark t-open
shift 0x01
wait 300
mark t-shesc1
shift 0x01
wait 300
mark t-shesc2
ctrl 0x10
mark t-quit-dialog
wait 400
move 320 281
wait 20
click
mark t-reboot-click
wait 400
