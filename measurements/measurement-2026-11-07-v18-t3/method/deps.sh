#!/bin/sh
# cargo xtask deps on the main tree at 5a8e01b5, outputs in deps/
M=measurements/measurement-2026-11-07-v18-t3
for f in vb_draw mov_cur set_mouse_form vdi_vsc_form vdimouse_init vdimouse_exit vdi_v_opnwk vdi_v_clswk timer_init mouse_int; do
  for d in down up; do cargo xtask deps $f --$d > $M/deps/$f-$d.txt 2>&1; done
done
