#!/usr/bin/env bash
# mkcards.sh -- A115: the QEMU cards of the run (64 MiB MBR+FAT16, never a real card) in $S/cards, and their sha256.
#   gh1   NEWDESK (reg/gh1.inf: C:\ACC\ window open) + C:\ACC\SYSACC.NAC                  (scenario nd)
#   td    TeraDesk #Z (reg/z-teradesk.inf) + TERADESK.INF + QED + C:\ACC\SYSACC.NAC         (scenario td)
#   qed   QED #Z (reg/z-qed.inf)                                                           (scenario qed)
#   std   tools/make-sd-image.sh (the default QEMU card: 2 accessories SYSACC + SYSACC2)    (plain boot)
#   api   C:\AUTO\APITEST.NCP (sdk/apps/apitest), NEWDESK.INF = gh1.inf                    (APITEST)
#   a117  A117 probe accessory C:\ACC\A117ACC.NAC + a117/a117.inf                          (A117 desktop method)
#   a117td TeraDesk #Z (a117/td.inf) + C:\ACC\A117ACC.NAC                                   (A117 TeraDesk method)
# The cards follow measurements/measurement-2026-10-21-gh1-reg (gh1/td/qed/std/api) and measurements/measurement-2026-10-25-a117 (a117*).
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; . "$HERE/reg/env.sh"
C="$S/cards"; B="$S/userbin"; mkdir -p "$C" "$B"
bash "$WS/sdk/build.sh" "$WS/sdk/examples/sysacc.c" "$B/SYSACC.NAC" >/dev/null
bash "$WS/sdk/build.sh" "$HERE/a117/a117acc.c" "$B/A117ACC.NAC" >/dev/null
TD="$WS/sdk/apps/teradesk/build/dist"; QB="$WS/sdk/apps/qed/build"
printf 'A115 regression test document.\r\n' > "$B/DOC.TXT"
TDF=(Apps/TERADESK/DESKTOP.NAP="$TD/DESKTOP.NAP" Apps/TERADESK/desktop.rsc="$TD/desktop.rsc" Apps/TERADESK/icons.rsc="$TD/icons.rsc"
     Apps/TERADESK/cicons.rsc="$TD/cicons.rsc" Apps/TERADESK/TERADESK.INF="$HERE/reg/TERADESK.INF")
QF=(Apps/QED/QED.NAP="$QB/QED.NAP" Apps/QED/qed.rsc="$QB/qed.rsc")
MK=(bash "$HERE/reg/mkcard.sh")
"${MK[@]}" "$C/gh1.img" "$HERE/reg/gh1.inf" ACC/SYSACC.NAC="$B/SYSACC.NAC" >/dev/null
"${MK[@]}" "$C/td.img" "$HERE/reg/z-teradesk.inf" ACC/SYSACC.NAC="$B/SYSACC.NAC" DOC.TXT="$B/DOC.TXT" "${TDF[@]}" "${QF[@]}" >/dev/null
"${MK[@]}" "$C/qed.img" "$HERE/reg/z-qed.inf" DOC.TXT="$B/DOC.TXT" "${QF[@]}" >/dev/null
"${MK[@]}" "$C/api.img" "$HERE/reg/gh1.inf" AUTO/APITEST.NCP="$WS/sdk/apps/apitest/APITEST.NCP" >/dev/null
bash "$HERE/a117/mkcard.sh" "$C/a117.img" "$HERE/a117/a117.inf" ACC/A117ACC.NAC="$B/A117ACC.NAC" >/dev/null
bash "$HERE/a117/mkcard.sh" "$C/a117td.img" "$HERE/a117/td.inf" ACC/A117ACC.NAC="$B/A117ACC.NAC" "${TDF[@]}" >/dev/null
bash "$WS/tools/make-sd-image.sh" "$C/std.img" > "$S/logs/make-sd-image.log" 2>&1
(cd "$C" && shasum -a 256 *.img) ; (cd "$B" && shasum -a 256 *)
# --- Tester additions (a115-test): two accessories with different titles ---
bash "$WS/sdk/build.sh" "$HERE/two/accalpha.c" "$B/ACCAL.NAC" >/dev/null
bash "$WS/sdk/build.sh" "$HERE/two/accbeta.c" "$B/ACCBE.NAC" >/dev/null
bash "$HERE/a117/mkcard.sh" "$C/two.img" "$HERE/a117/a117.inf" ACC/ACCAL.NAC="$B/ACCAL.NAC" ACC/ACCBE.NAC="$B/ACCBE.NAC" >/dev/null
(cd "$C" && shasum -a 256 two.img); (cd "$B" && shasum -a 256 ACCAL.NAC ACCBE.NAC)
