# SDK07: TeraDesk (arm up:DESKTOP): Options menu -> Video options... dialog, screendump of the open dialog (dlg),
# then try to tick Overscan (the box position of the ARCH06 run: x 201 y 281) and leave with Return (= OK, the default
# button). = measurements/measurement-2026-10-29-arch06/method/vid-novdo.spec plus a nop + mark dlg on the box before
# the click and 200 ticks (1 s at 200 Hz) for its screendump (0.8 s after the nop) before the click comes; after the
# click 300 ticks (1.5 s) instead of 150 before Return, so that the ovs screendump (0.8 s after the button release)
# shows the dialog after the click and before OK.
# 640x480, 16-px menu rows: Options title at x~260 y 9; "Video options..." is the 10th row of the drop-down.
at 600
park
move 260 9
click
wait 150
mark opts
move 300 171
wait 100
nop
mark hover
click
wait 400
wait 200
move 201 281
wait 100
nop
mark dlg
wait 200
click
mark ovs
wait 300
key 0x1C
wait 400
mark ok
wait 400
nop
mark after
