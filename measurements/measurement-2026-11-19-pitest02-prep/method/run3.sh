#!/usr/bin/env bash
cd "$(dirname "$0")"
for t in vdidraw:32 present:32 present:16; do
  n=${t%%:*}; p=${t##*:}; app="serial=on"; [ $p = 32 ] && app="serial=on vdiplanes=32"
  cp -c card-$n.img run.img
  qemu-system-aarch64 -M raspi4b -display none -no-reboot -append "$app" -kernel k.img -drive file=run.img,if=sd,format=raw -serial file:serial-$n-$p.log -qmp unix:q.sock,server,nowait >/dev/null 2>&1 &
  pid=$!; sleep 35
  python3 -I qmpshot.py "$PWD/q.sock" "$PWD/shot-$n-$p.png"; sleep 2; kill $pid 2>/dev/null; wait $pid 2>/dev/null
  rm -f run.img q.sock
  echo "== $n $p: EXC/PANIC count $(grep -a -c 'EXC\|PANIC\|panicked' serial-$n-$p.log)"; grep -a "exited\|starting" serial-$n-$p.log | head -3
done
