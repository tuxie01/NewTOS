# TeraDesk (arm up:DESKTOP): start QED and zView from desktop icons, quit each with Ctrl-Q, then quit TeraDesk with Ctrl-Q.
at 600
park
move 522 40
nop
mark pre
wait 100
dclick
wait 300
nop
mark q1
wait 500
nop
mark q2
wait 1000
nop
mark qed
ctrl 0x10
wait 700
nop
mark back1
move 602 40
wait 100
dclick
wait 400
nop
mark z1
wait 1200
nop
mark zview
ctrl 0x10
wait 900
nop
mark back2
ctrl 0x10
wait 1200
nop
mark quit
move 320 313
wait 100
nop
mark hq
click
wait 1000
nop
mark end
wait 600
nop
mark end2
