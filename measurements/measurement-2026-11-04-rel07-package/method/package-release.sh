#!/usr/bin/env bash
#
# package-release.sh -- build the NewTOS tester release ZIP from the checked-out
# commit (TASK/REL07 step 2). Same layout and scope as v0.6.1: a folder
# NewTOS-<version>/ with the card folder KARTE/ (copied by the tester onto the
# first FAT16 partition) and the release texts plus LICENCE.broadcom beside it.
#
# Usage:
#   package-release.sh <version> <work-dir> <previous-release-folder>
#
#     version        product version as shown, e.g. 0.7 -- folder NewTOS-<version>,
#                    ZIP NewTOS-<version>.zip, texts docs/release/v<version>/;
#                    the kernel must show "NewTOS <version>" (refused otherwise)
#     work-dir       scratch directory (NOT in the repository); gets
#                    NewTOS-<version>/, NewTOS-<version>.zip, FILES.TSV
#                    (zip path, sha256, bytes, source of every file),
#                    CHECKS.TXT, build/ (intermediate products), logs/
#     previous-release-folder
#                    the unpacked previous release (e.g. .../NewTOS-0.6.1); the
#                    source of the files that have no source in this
#                    repository (firmware, LICENCE.broadcom, DOOM1.WAD, the
#                    third-party .HYP texts, the card's own NEWDESK.INF,
#                    ZVIEW.INF, QED\CONFIG.TXT) and the name list the new ZIP
#                    is checked against
#
#   Environment:
#     KERNEL_FEATURES  cargo features of the release kernel (default crash-walk,
#                      user decision 2026-11-04, TASK/REL07)
#     BALL_DIR         the bouncing-ball demo, which lives outside this
#                      repository (default /Volumes/Daten/Development/ball-demo);
#                      its sources are COPIED into <work-dir> and built there,
#                      so nothing is written into BALL_DIR
#     ALLOW_TODO=1     package even though a release text still carries a
#                      `TODO-REL` marker line (a draft package; the default
#                      refuses, so a real release cannot ship the markers)
#     ALLOW_DIRTY=1    package from a dirty tree (default: refused, because the
#                      kernel's build stamp carries no dirty flag)
#
# Every build product is rebuilt by this script from the checked-out sources
# (each build script rebuilds stale libraries itself, sdk/lib-stale.sh); a
# product older than the script's start is refused. Nothing in the repository
# is modified: the script checks `git status` before and after.
#
set -euo pipefail

[ "$#" -eq 3 ] || { sed -n '/^# Usage:/,/^# Every build/p' "$0" | sed '$d; s/^# \{0,1\}//' >&2; exit 2; }
VER="$1"; WORK="$2"; PREV="$3"
ROOT="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
KERNEL_FEATURES="${KERNEL_FEATURES:-crash-walk}"
BALL_DIR="${BALL_DIR:-/Volumes/Daten/Development/ball-demo}"
NAME="NewTOS-$VER"
PREVNAME="$(basename "$PREV")"
TEXTS="$ROOT/docs/release/v$VER"
export COPYFILE_DISABLE=1

die() { echo "package-release: $*" >&2; exit 1; }

case "$VER" in ''|*[!0-9.]*) die "version '$VER' is not of the form x.y[.z]" ;; esac
[ -d "$PREV/KARTE" ] || die "$PREV is not an unpacked release folder (no KARTE/)"
[ -d "$TEXTS" ] || die "release texts $TEXTS missing"
case "$(cd "$(dirname "$WORK")" 2>/dev/null && pwd)/" in "$ROOT"/*) die "work-dir must be outside the repository" ;; esac
mkdir -p "$WORK"; WORK="$(cd "$WORK" && pwd)"
[ ! -e "$WORK/$NAME" ] && [ ! -e "$WORK/$NAME.zip" ] || die "$WORK/$NAME(.zip) exists -- use a fresh work-dir"

# The workspace version (one source, TASK/REL07 step 1) must match <version>
# by EmuTOS' display rule: x.y.0 shows as x.y.
CARGO_VER="$(sed -n '/^\[workspace.package\]/,/^\[/s/^version *= *"\(.*\)"/\1/p' "$ROOT/Cargo.toml")"
SHOWN="$CARGO_VER"; case "$CARGO_VER" in *.*.0) SHOWN="${CARGO_VER%.0}" ;; esac
[ "$SHOWN" = "$VER" ] || die "workspace version $CARGO_VER shows as $SHOWN, not $VER"

# Uncommitted changes are tolerated only where no build reads: docs/ (the
# release texts are copied from there and recorded as such), TASK/,
# measurements/. Anything else would build products the stamp does not name.
dirty="$(git -C "$ROOT" status --porcelain)"
dirty_build="$(printf '%s\n' "$dirty" | grep -v '^$' | grep -Ev '^.. "?(docs|TASK|measurements)/' || true)"
if [ -n "$dirty_build" ] && [ "${ALLOW_DIRTY:-0}" != 1 ]; then
  echo "$dirty_build" >&2; die "tree is dirty outside docs/ TASK/ measurements/ -- commit first (ALLOW_DIRTY=1 overrides)"
fi
HEAD="$(git -C "$ROOT" rev-parse HEAD)"; SHORT="$(git -C "$ROOT" rev-parse --short=8 HEAD)"

B="$WORK/build"; L="$WORK/logs"; mkdir -p "$B" "$L"
STAMP="$WORK/.start"; : > "$STAMP"; sleep 1
step() { # $1 = log name, rest = command (run from the repository root)
  local log="$L/$1.log"; shift
  echo "== $*" | tee "$log" >&2
  if ! (cd "$ROOT" && "$@") >>"$log" 2>&1; then tail -30 "$log" >&2; die "step failed, log: $log"; fi
}
fresh() { for f in "$@"; do [ -f "$f" ] && [ "$f" -nt "$STAMP" ] || die "$f was not rebuilt by this run"; done; }

# ---- 1. build every product from the checked-out sources --------------------
step kernel cargo xtask build --board rpi4 --release --features "$KERNEL_FEATURES"
KIMG="$ROOT/target/aarch64-unknown-none-softfloat/release/kernel8.img"
fresh "$KIMG"; cp -X "$KIMG" "$B/kernel8.img"

step sysacc sdk/build.sh sdk/examples/sysacc.c "$B/SYSACC.NAC"; fresh "$B/SYSACC.NAC"
step zview sdk/zview/build-zview.sh
ZV="$ROOT/sdk/zview/build/dist/ZVIEW"; fresh "$ZV/ZVIEW.NAP" "$ZV/ZVIEW.RSC" "$ZV"/CODECS/*.NLD "$ZV"/ICONS/*.GOD
step qed sdk/apps/build-qed.sh
QD="$ROOT/sdk/apps/qed/build"; fresh "$QD/QED.NAP" "$QD/qed.rsc"
step teradesk sdk/apps/build-teradesk.sh
TD="$ROOT/sdk/apps/teradesk/build/dist"; fresh "$TD/DESKTOP.NAP" "$TD/desktop.rsc" "$TD/icons.rsc" "$TD/cicons.rsc"
step coremark sdk/apps/build-coremark.sh
CM="$ROOT/sdk/apps/coremark/build"; fresh "$CM/CORE.NCP" "$CM/COREVAL.NCP"
step doom env OUT="$B/DOOM.NAP" sdk/apps/build-doom.sh; fresh "$B/DOOM.NAP"
step hypview sdk/apps/build-hypview.sh
HV="$ROOT/sdk/apps/hypview/build"; fresh "$HV/HYP_VIEW.NAP" "$HV/hyp_view.rsc"

# The ball demo lives outside the repository: copy its sources, build the copy.
BALL_SRC="build-ball.sh ball.c ball-tex.c plat-newtos.c ball.h ball-tex.h BALL.NTX BALL.CFG"
mkdir -p "$WORK/ball-src"
for f in $BALL_SRC; do [ -f "$BALL_DIR/$f" ] || die "$BALL_DIR/$f missing"; cp -X "$BALL_DIR/$f" "$WORK/ball-src/$f"; done
(cd "$WORK/ball-src" && shasum -a 256 $BALL_SRC) > "$WORK/ball-src.sha256"
step ball env NEWTOS_SDK="$ROOT/sdk" "$WORK/ball-src/build-ball.sh" "$B/BALL.NAP"; fresh "$B/BALL.NAP"

# TERADESK.INF is generated by tools/make-sd-image.sh (DESK41): take its body.
sed -n "/^cat > \"\$V\/TERADESK.INF\" <<'TERAINF'\$/,/^TERAINF\$/p" "$ROOT/tools/make-sd-image.sh" | sed '1d;$d' > "$B/TERADESK.INF"
[ -s "$B/TERADESK.INF" ] && head -1 "$B/TERADESK.INF" | grep -qx 'TeraDesk-inf' && tail -2 "$B/TERADESK.INF" | grep -qx 'end' \
  || die "could not extract TERADESK.INF from tools/make-sd-image.sh"

# GEM.RSC = the file-selector resource the kernel embeds; cross-check that
# the converter still produces it from the EmuTOS source.
python3 "$ROOT/tools/mkrsc.py" convert "$ROOT/references/emutos/aes/gem.rsc" "$B/GEM.RSC.check" > "$L/gemrsc.log"
cmp -s "$B/GEM.RSC.check" "$ROOT/tos-api/src/aes/gem_fsel.nrsc" || die "mkrsc output differs from tos-api/src/aes/gem_fsel.nrsc"
# KEYBOARD.KBD = keyboards/german.kbd; cross-check against its generator.
python3 "$ROOT/tools/mkkbd.py" de > "$B/KEYBOARD.KBD.check"
cmp -s "$B/KEYBOARD.KBD.check" "$ROOT/keyboards/german.kbd" || die "tools/mkkbd.py de differs from keyboards/german.kbd"

after="$(git -C "$ROOT" status --porcelain)"
[ "$after" = "$dirty" ] || { echo "$after" >&2; die "the builds changed the repository"; }
[ "$(git -C "$ROOT" rev-parse HEAD)" = "$HEAD" ] || die "HEAD moved during the build"

# ---- 2. assemble ------------------------------------------------------------
# One line per file: <path below NewTOS-x.y/> | <source file> | <source label>.
# The label is what FILES.TSV records as the file's origin.
R="repo $SHORT"
P="previous release $PREVNAME (no source in this repository)"
MANIFEST="
KARTE/start4.elf|$PREV/KARTE/start4.elf|$P: Raspberry Pi firmware
KARTE/fixup4.dat|$PREV/KARTE/fixup4.dat|$P: Raspberry Pi firmware
KARTE/bcm2711-rpi-4-b.dtb|$PREV/KARTE/bcm2711-rpi-4-b.dtb|$P: Raspberry Pi firmware
LICENCE.broadcom|$PREV/LICENCE.broadcom|$P: Raspberry Pi firmware licence
KARTE/config.txt|$ROOT/boot/rpi4/config.txt|$R boot/rpi4/config.txt
KARTE/kernel8.img|$B/kernel8.img|$R cargo xtask build --board rpi4 --release --features $KERNEL_FEATURES
KARTE/NEWDESK.INF|$PREV/KARTE/NEWDESK.INF|$P: tester desktop configuration
KARTE/TERADESK.INF|$B/TERADESK.INF|$R tools/make-sd-image.sh (TERADESK.INF heredoc)
KARTE/KEYBOARD.KBD|$ROOT/keyboards/german.kbd|$R keyboards/german.kbd (= tools/mkkbd.py de)
KARTE/BALL.CFG|$WORK/ball-src/BALL.CFG|ball-demo $BALL_DIR/BALL.CFG (outside the repository)
KARTE/BALL.NTX|$WORK/ball-src/BALL.NTX|ball-demo $BALL_DIR/BALL.NTX (outside the repository)
KARTE/GEMSYS/RSC/GEM.RSC|$ROOT/tos-api/src/aes/gem_fsel.nrsc|$R tos-api/src/aes/gem_fsel.nrsc (= tools/mkrsc.py convert references/emutos/aes/gem.rsc)
KARTE/GEMSYS/RSC/qed.rsc|$QD/qed.rsc|$R sdk/apps/build-qed.sh
KARTE/ACC/SYSACC.NAC|$B/SYSACC.NAC|$R sdk/build.sh sdk/examples/sysacc.c
KARTE/APPS/DOOM/DOOM.NAP|$B/DOOM.NAP|$R sdk/apps/build-doom.sh
KARTE/APPS/DOOM/DOOM.NTP|$B/DOOM.NAP|$R sdk/apps/build-doom.sh (same program as DOOM.NAP, .NTP = asks for parameters)
KARTE/APPS/DOOM/DOOM1.WAD|$PREV/KARTE/APPS/DOOM/DOOM1.WAD|$P: id Software shareware WAD
KARTE/APPS/BALL/BALL.NAP|$B/BALL.NAP|ball-demo build-ball.sh, built from a copy against $R sdk
KARTE/APPS/ZVIEW/ZVIEW.NAP|$ZV/ZVIEW.NAP|$R sdk/zview/build-zview.sh
KARTE/APPS/ZVIEW/ZVIEW.RSC|$ZV/ZVIEW.RSC|$R sdk/zview/build-zview.sh
KARTE/APPS/ZVIEW/ZVIEW.INF|$PREV/KARTE/APPS/ZVIEW/ZVIEW.INF|$P: zView settings
KARTE/APPS/ZVIEW/ZVIEW.HYP|$ROOT/sdk/zview/z-tools-04805b7/zview/_dist/doc/zView.hyp|$R sdk/zview/z-tools-04805b7.tar.xz zview/_dist/doc/zView.hyp
KARTE/APPS/QED/QED.NAP|$QD/QED.NAP|$R sdk/apps/build-qed.sh
KARTE/APPS/QED/qed.rsc|$QD/qed.rsc|$R sdk/apps/build-qed.sh
KARTE/APPS/QED/CONFIG.TXT|$PREV/KARTE/APPS/QED/CONFIG.TXT|$P: empty file
KARTE/APPS/QED/QED.HYP|$PREV/KARTE/APPS/QED/QED.HYP|$P: QED hypertext (FreeMiNT 1.19 distribution)
KARTE/APPS/COREMARK/CORE.NCP|$CM/CORE.NCP|$R sdk/apps/build-coremark.sh
KARTE/APPS/COREMARK/COREVAL.NCP|$CM/COREVAL.NCP|$R sdk/apps/build-coremark.sh
KARTE/APPS/TERADESK/DESKTOP.NAP|$TD/DESKTOP.NAP|$R sdk/apps/build-teradesk.sh
KARTE/APPS/TERADESK/desktop.rsc|$TD/desktop.rsc|$R sdk/apps/build-teradesk.sh
KARTE/APPS/TERADESK/icons.rsc|$TD/icons.rsc|$R sdk/apps/build-teradesk.sh
KARTE/APPS/TERADESK/cicons.rsc|$TD/cicons.rsc|$R sdk/apps/build-teradesk.sh
KARTE/APPS/TERADESK/TERADESK.INF|$B/TERADESK.INF|$R tools/make-sd-image.sh (TERADESK.INF heredoc)
KARTE/APPS/TERADESK/TERADESK.HYP|$PREV/KARTE/APPS/TERADESK/TERADESK.HYP|$P: TeraDesk hypertext (FreeMiNT 1.19 distribution)
KARTE/APPS/HYPVIEW/HYP_VIEW.NAP|$HV/HYP_VIEW.NAP|$R sdk/apps/build-hypview.sh
KARTE/APPS/HYPVIEW/hyp_view.rsc|$HV/hyp_view.rsc|$R sdk/apps/build-hypview.sh
"
for g in "$ZV"/CODECS/*.NLD "$ZV"/ICONS/*.GOD; do
  MANIFEST="$MANIFEST
KARTE/APPS/ZVIEW/$(basename "$(dirname "$g")")/$(basename "$g")|$g|$R sdk/zview/build-zview.sh"
done
for t in README.TXT LIESMICH.TXT TESTING.TXT TESTANLEITUNG.TXT FEEDBACK.TXT FEEDBACK-EN.TXT; do
  state=""; [ -z "$(git -C "$ROOT" status --porcelain -- "docs/release/v$VER/$t")" ] || state=" (working tree, uncommitted)"
  MANIFEST="$MANIFEST
$t|$TEXTS/$t|$R docs/release/v$VER/$t$state"
done
EMPTY_DIRS="KARTE/CLIPBRD"

if [ "${ALLOW_TODO:-0}" != 1 ] && grep -l '^TODO-REL' "$TEXTS"/*.TXT >/dev/null 2>&1; then
  grep -n '^TODO-REL' "$TEXTS"/*.TXT >&2; die "release texts still carry TODO markers (ALLOW_TODO=1 for a draft)"
fi

D="$WORK/$NAME"; mkdir -p "$D"
printf 'path\tsha256\tbytes\tsource\n' > "$WORK/FILES.TSV"
while IFS='|' read -r dst src label; do
  [ -n "$dst" ] || continue
  [ -f "$src" ] || die "source of $dst missing: $src"
  mkdir -p "$D/$(dirname "$dst")"; cp -X "$src" "$D/$dst"
done <<< "$MANIFEST"
for e in $EMPTY_DIRS; do mkdir -p "$D/$e"; done
# Normalise metadata: one time stamp (the commit's), plain permissions.
TS="$(TZ=UTC git -C "$ROOT" show -s --format=%cd --date=format-local:%Y%m%d%H%M.%S HEAD)"
find "$D" -type d -exec chmod 755 {} + ; find "$D" -type f -exec chmod 644 {} +
find "$D" -exec env TZ=UTC touch -t "$TS" {} +

while IFS='|' read -r dst src label; do
  [ -n "$dst" ] || continue
  printf '%s/%s\t%s\t%s\t%s\n' "$NAME" "$dst" "$(shasum -a 256 "$D/$dst" | awk '{print $1}')" \
    "$(stat -f %z "$D/$dst")" "$label" >> "$WORK/FILES.TSV"
done <<< "$MANIFEST"

(cd "$WORK" && TZ=UTC zip -q -X -r "$NAME.zip" "$NAME")

# ---- 3. checks --------------------------------------------------------------
C="$WORK/CHECKS.TXT"; : > "$C"; fail=0
check() { if [ "$2" = ok ]; then echo "PASS $1" >> "$C"; else echo "FAIL $1: $2" >> "$C"; fail=1; fi; }
unzip -Z1 "$WORK/$NAME.zip" | sed "s#^$NAME/#X/#" | sed 's#/$##' | sort > "$WORK/names-new.txt"
(cd "$(dirname "$PREV")" && find "$PREVNAME" ! -name .DS_Store ! -name '._*') | sed "s#^$PREVNAME#X#" | sort > "$WORK/names-prev.txt"
if diff "$WORK/names-prev.txt" "$WORK/names-new.txt" > "$WORK/names.diff"; then
  check "name list equals $PREVNAME (apart from the version, case-sensitive)" ok
else check "name list equals $PREVNAME" "see names.diff"; fi
bad="$(unzip -Z1 "$WORK/$NAME.zip" | grep -E '(^|/)(\._|\.DS_Store|__MACOSX)' || true)"
check "no AppleDouble/.DS_Store/__MACOSX entries" "${bad:-ok}"
n_ver="$(LC_ALL=C grep -c -a "NewTOS $VER" "$D/KARTE/kernel8.img" || true)"
check "kernel8.img carries \"NewTOS $VER\"" "$([ "$n_ver" -gt 0 ] && echo ok || echo missing)"
check "kernel8.img carries build rev ($SHORT)" "$(LC_ALL=C grep -q -a "($SHORT)" "$D/KARTE/kernel8.img" && echo ok || echo missing)"
crashwalk_state() { # the crash-walk report's serial tag is in the image iff the feature is on
  local want=no have=no
  case " $KERNEL_FEATURES " in *crash-walk*) want=yes ;; esac
  LC_ALL=C grep -q -a '\[crashwalk\]' "$D/KARTE/kernel8.img" && have=yes
  [ "$want" = "$have" ] && echo ok || echo "feature wanted=$want, in image=$have"
}
check "kernel8.img crash-walk report matches KERNEL_FEATURES" "$(crashwalk_state)"
# 8.3: every path component on the card, except the firmware's own long
# names (read by the VideoCore firmware, never by TOS).
RE83='^[A-Za-z0-9_$~!#%&(){}^@-]{1,8}(\.[A-Za-z0-9_$~!#%&(){}^@-]{1,3})?$'
non83="$(cd "$D/KARTE" && find . -mindepth 1 | sed 's#^\./##' | tr '/' '\n' | sort -u | grep -Ev "$RE83" | grep -vx 'bcm2711-rpi-4-b.dtb' || true)"
check "every TOS-visible name on KARTE is 8.3" "${non83:-ok}"
if grep -q '^TODO-REL' "$D"/*.TXT; then
  echo "DRAFT release texts carry $(grep -c '^TODO-REL' "$D"/*.TXT | awk -F: '{n+=$2} END {print n}') TODO-REL marker line(s) (ALLOW_TODO=1) -- not a release" >> "$C"
else check "release texts free of TODO markers" ok; fi
( cd "$D" && find . -type f | sed 's#^\./##' | sort ) > "$WORK/files-on-disk.txt"
sed 1d "$WORK/FILES.TSV" | cut -f1 | sed "s#^$NAME/##" | sort > "$WORK/files-listed.txt"
check "FILES.TSV lists every file of the ZIP" "$(cmp -s "$WORK/files-on-disk.txt" "$WORK/files-listed.txt" && echo ok || echo 'differs')"

{
  echo "version $VER  commit $HEAD  tree_dirty_build=$([ -n "$dirty_build" ] && echo yes || echo no)  tree_dirty_docs=$([ -n "$dirty" ] && echo yes || echo no)"
  echo "kernel_features $KERNEL_FEATURES"
  echo "host_clang $(clang --version | head -1)"
  echo "zip $(shasum -a 256 "$WORK/$NAME.zip" | awk '{print $1}') $(stat -f %z "$WORK/$NAME.zip") $WORK/$NAME.zip"
  echo "entries $(unzip -Z1 "$WORK/$NAME.zip" | wc -l | tr -d ' ')"
} >> "$C"
cat "$C"
[ "$fail" = 0 ] || die "checks failed (see $C)"
echo "done: $WORK/$NAME.zip"
