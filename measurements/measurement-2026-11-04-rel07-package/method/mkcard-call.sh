#!/usr/bin/env bash
# mkcard-call.sh <KARTE-dir> <scratch> -- the smoke-test cards, each built by
# tools/mkcard.sh from the release folder's KARTE/ (every file as a dest=file
# pair, the empty CLIPBRD folder via MKCARD_DIRS, label NEWTOS as the README
# tells the tester):
#   card-karte.img  exactly KARTE/ (NEWDESK.INF = KARTE/NEWDESK.INF)
#   card-zview.img  KARTE/ with one line added to NEWDESK.INF after #R:
#                   "#Z 01 C:\APPS\ZVIEW\ZVIEW.NAP@" (autorun zView)
#   card-qed.img    the same with "#Z 01 C:\APPS\QED\QED.NAP@"
set -euo pipefail
K=$(cd "$1" && pwd); S=$2; ROOT=$(cd "$(dirname "$0")/../../.." && pwd)
mkdir -p "$S"
pairs=()
while IFS= read -r f; do
  [ "$f" = NEWDESK.INF ] && continue
  pairs+=("$f=$K/$f")
done < <(cd "$K" && find . -type f | sed 's#^\./##' | sort)
autorun() { # $1 = TOS path of the program
  Z="#Z 01 $1@" awk '{ print } NR == 1 { printf "%s\r\n", ENVIRON["Z"] }' "$K/NEWDESK.INF" > "$S/newdesk-$2.inf"
}
autorun 'C:\APPS\ZVIEW\ZVIEW.NAP' zview
autorun 'C:\APPS\QED\QED.NAP' qed
export MKCARD_LABEL=NEWTOS MKCARD_DIRS=CLIPBRD
"$ROOT/tools/mkcard.sh" "$S/card-karte.img" "$K/NEWDESK.INF" "${pairs[@]}"
"$ROOT/tools/mkcard.sh" "$S/card-zview.img" "$S/newdesk-zview.inf" "${pairs[@]}"
"$ROOT/tools/mkcard.sh" "$S/card-qed.img" "$S/newdesk-qed.inf" "${pairs[@]}"
shasum -a 256 "$S"/card-*.img "$S"/newdesk-*.inf
