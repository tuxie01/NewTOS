#!/usr/bin/env bash
#
# package-release.sh -- build the NewTOS tester release ZIP from the checked-out
# commit (TASK/REL07; first used for 0.7). Layout as since v0.6.1: a folder
# NewTOS-<version>/ with the card folder KARTE/ (copied by the tester onto the
# first FAT16 partition) and the release texts plus LICENCE.broadcom beside it.
#
# Usage:
#   tools/release/package-release.sh <version> <work-dir> <previous-release>
#
#     version        product version as shown, e.g. 0.7 -- folder NewTOS-<version>,
#                    ZIP NewTOS-<version>.zip, texts docs/release/v<version>/;
#                    the workspace version (Cargo.toml [workspace.package]) and
#                    the kernel must show "NewTOS <version>" (refused otherwise)
#     work-dir       scratch directory outside the repository; gets
#                    NewTOS-<version>/ (the unpacked release), NewTOS-<version>.zip,
#                    FILES.TSV (zip path, sha256, bytes, source of every file),
#                    CHECKS.TXT, names-*.txt, build/ (intermediate products),
#                    logs/ (one log per build step), prev/ (previous release)
#     previous-release
#                    the previous release, as its ZIP (e.g. NewTOS-0.6.1.zip,
#                    unpacked into <work-dir>/prev/) or as an unpacked folder
#                    (e.g. .../NewTOS-0.6.1); the source of the files that have no
#                    source in this repository (firmware, LICENCE.broadcom,
#                    DOOM1.WAD, the third-party .HYP texts, the tester
#                    NEWDESK.INF, ZVIEW.INF, QED\CONFIG.TXT) and the name list the
#                    new ZIP is checked against
#
#   Environment:
#     KERNEL_FEATURES  cargo features of the release kernel (default crash-walk,
#                      user decision 2026-11-04, TASK/REL07)
#     NAMES_REMOVED    paths below the release folder that the previous release
#                      has and this one intentionally drops, space-separated
#                      (a folder covers everything below it), e.g.
#                      "KARTE/APPS/BALL KARTE/BALL.CFG KARTE/BALL.NTX" for 0.7
#     NAMES_ADDED      paths this release intentionally adds (their parent
#                      folders are implied), space-separated
#     ALLOW_DIRTY=1    package from a dirty tree (default: refused, because the
#                      kernel's build stamp carries no dirty flag); not for a
#                      release
#
# What it guarantees (a run that ends with "done:" and exit 0):
#   - the tree had no uncommitted change outside docs/ TASK/ measurements/ --
#     except docs/release/, which is packaged and must be committed -- and
#     outside this script itself (not read by any build; its sha256 and git
#     state go into CHECKS.TXT);
#   - the release texts (README/LIESMICH etc. and apps/<DIR>.TXT, packaged as
#     KARTE/APPS/<DIR>/CHANGES.TXT) carry no `TODO-REL` marker (checked before any build
#     and again in the packaged copies; there is no override);
#   - every build product was rebuilt by this run from the checked-out sources
#     (each build script rebuilds stale libraries itself, sdk/lib-stale.sh); a
#     product older than the script's start is refused;
#   - the builds left the repository unchanged (git status before = after)
#     and HEAD did not move;
#   - the release is assembled only from the table below (destination, source,
#     origin label); every file has the commit's time stamp and mode 644
#     (folders 755); zip -X (no extra attributes), COPYFILE_DISABLE
#     (no AppleDouble);
#   - checks (CHECKS.TXT, any FAIL ends the run non-zero): name list = the
#     previous release's minus NAMES_REMOVED plus NAMES_ADDED (case-sensitive,
#     version aside); no ._/.DS_Store/__MACOSX entry; kernel8.img carries
#     "NewTOS <version>", the build revision and the crash-walk report iff
#     KERNEL_FEATURES asks for it; every TOS-visible name on KARTE is 8.3;
#     FILES.TSV lists every file; the ZIP unpacks byte-identical to the folder.
#
# Adding or removing a file: edit the MANIFEST table in section 2 (one line
# `<path below NewTOS-x.y/>|<source file>|<origin label>`; a new build product
# also needs its build `step` and a `fresh` check in section 1; an empty folder
# goes into EMPTY_DIRS). Then declare the change against the previous release
# in the call: NAMES_REMOVED / NAMES_ADDED. Without that the name-list check
# fails, so a file cannot appear or vanish unnoticed.
#
set -euo pipefail

[ "$#" -eq 3 ] || { sed -n '/^# Usage:/,/^# What it guarantees/p' "$0" | sed '$d; s/^# \{0,1\}//' >&2; exit 2; }
VER="$1"; WORK="$2"; PREV="$3"
ROOT="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
SELF="$(cd "$(dirname "$0")" && pwd)/$(basename "$0")"; SELF_REL="${SELF#"$ROOT"/}"
KERNEL_FEATURES="${KERNEL_FEATURES:-crash-walk}"
NAMES_REMOVED="${NAMES_REMOVED:-}"; NAMES_ADDED="${NAMES_ADDED:-}"
NAME="NewTOS-$VER"
TEXTS="$ROOT/docs/release/v$VER"
TEXT_FILES="README.TXT LIESMICH.TXT TESTING.TXT TESTANLEITUNG.TXT FEEDBACK.TXT FEEDBACK-EN.TXT"
export COPYFILE_DISABLE=1

die() { echo "package-release: $*" >&2; exit 1; }

case "$VER" in ''|*[!0-9.]*) die "version '$VER' is not of the form x.y[.z]" ;; esac
[ -d "$TEXTS" ] || die "release texts $TEXTS missing"
for t in $TEXT_FILES; do [ -f "$TEXTS/$t" ] || die "release text $TEXTS/$t missing"; done
if grep -rn 'TODO-REL' "$TEXTS" >&2; then die "release texts still carry TODO-REL markers"; fi
case "$(cd "$(dirname "$WORK")" 2>/dev/null && pwd)/" in "$ROOT"/*) die "work-dir must be outside the repository" ;; esac
mkdir -p "$WORK"; WORK="$(cd "$WORK" && pwd)"
[ ! -e "$WORK/$NAME" ] && [ ! -e "$WORK/$NAME.zip" ] && [ ! -e "$WORK/prev" ] || die "$WORK already holds a release -- use a fresh work-dir"

# The previous release: a ZIP is unpacked into <work-dir>/prev/.
PREV_ID=""
if [ -f "$PREV" ]; then
  case "$PREV" in *.zip) ;; *) die "$PREV is neither a .zip nor a folder" ;; esac
  PREV_ID="zip $(shasum -a 256 "$PREV" | awk '{print $1}') $PREV"
  top="$(unzip -Z1 "$PREV" | sed -n '1s#/.*##p')"
  [ -n "$top" ] && [ -z "$(unzip -Z1 "$PREV" | grep -v "^$top/" || true)" ] || die "$PREV has no single top-level folder"
  mkdir -p "$WORK/prev"; (cd "$WORK/prev" && unzip -q "$PREV")
  PREV="$WORK/prev/$top"
else
  PREV="$(cd "$PREV" 2>/dev/null && pwd)" || die "previous release $3 not found"
  PREV_ID="folder $PREV"
fi
[ -d "$PREV/KARTE" ] || die "$PREV is not an unpacked release folder (no KARTE/)"
PREVNAME="$(basename "$PREV")"

# The workspace version (one source, TASK/REL07 step 1) must match <version>
# by EmuTOS' display rule: x.y.0 shows as x.y.
CARGO_VER="$(sed -n '/^\[workspace.package\]/,/^\[/s/^version *= *"\(.*\)"/\1/p' "$ROOT/Cargo.toml")"
SHOWN="$CARGO_VER"; case "$CARGO_VER" in *.*.0) SHOWN="${CARGO_VER%.0}" ;; esac
[ "$SHOWN" = "$VER" ] || die "workspace version $CARGO_VER shows as $SHOWN, not $VER"

# Uncommitted changes are tolerated only where no build reads: docs/ (except
# docs/release/, packaged from here), TASK/, measurements/, and this script.
# Anything else would build products the stamp does not name.
dirty="$(git -C "$ROOT" status --porcelain --untracked-files=all)"
dirty_build="$(printf '%s\n' "$dirty" | awk -v self="$SELF_REL" 'NF {
  p = substr($0, 4); gsub(/^"|"$/, "", p)
  if (p == self) next
  if (p ~ /^docs\/release\// || p !~ /^(docs|TASK|measurements)\//) print }')"
if [ -n "$dirty_build" ] && [ "${ALLOW_DIRTY:-0}" != 1 ]; then
  echo "$dirty_build" >&2; die "tree is dirty outside docs/ TASK/ measurements/ (or in docs/release/) -- commit first (ALLOW_DIRTY=1 overrides)"
fi
SELF_STATE=committed; [ -z "$(git -C "$ROOT" status --porcelain --untracked-files=all -- "$SELF_REL")" ] || SELF_STATE=uncommitted
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

after="$(git -C "$ROOT" status --porcelain --untracked-files=all)"
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
KARTE/GEMSYS/RSC/GEM.RSC|$ROOT/tos-api/src/aes/gem_fsel.nrsc|$R tos-api/src/aes/gem_fsel.nrsc (= tools/mkrsc.py convert references/emutos/aes/gem.rsc)
KARTE/GEMSYS/RSC/qed.rsc|$QD/qed.rsc|$R sdk/apps/build-qed.sh
KARTE/ACC/SYSACC.NAC|$B/SYSACC.NAC|$R sdk/build.sh sdk/examples/sysacc.c
KARTE/APPS/DOOM/DOOM.NAP|$B/DOOM.NAP|$R sdk/apps/build-doom.sh
KARTE/APPS/DOOM/DOOM.NTP|$B/DOOM.NAP|$R sdk/apps/build-doom.sh (same program as DOOM.NAP, .NTP = asks for parameters)
KARTE/APPS/DOOM/DOOM1.WAD|$PREV/KARTE/APPS/DOOM/DOOM1.WAD|$P: id Software shareware WAD
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
for t in $TEXT_FILES; do
  state=""; [ -z "$(git -C "$ROOT" status --porcelain -- "docs/release/v$VER/$t")" ] || state=" (working tree, uncommitted)"
  MANIFEST="$MANIFEST
$t|$TEXTS/$t|$R docs/release/v$VER/$t$state"
done
# Changes to an included program (not to NewTOS) go beside the program, not
# into README/LIESMICH (user 2026-11-07): docs/release/v<version>/apps/<DIR>.TXT
# becomes KARTE/APPS/<DIR>/CHANGES.TXT. A new one is a new name: NAMES_ADDED.
for a in "$TEXTS"/apps/*.TXT; do
  [ -f "$a" ] || continue
  n="$(basename "$a" .TXT)"
  state=""; [ -z "$(git -C "$ROOT" status --porcelain -- "docs/release/v$VER/apps/$n.TXT")" ] || state=" (working tree, uncommitted)"
  MANIFEST="$MANIFEST
KARTE/APPS/$n/CHANGES.TXT|$a|$R docs/release/v$VER/apps/$n.TXT$state"
done
EMPTY_DIRS="KARTE/CLIPBRD"

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
# Expected = previous minus NAMES_REMOVED (each must exist there) plus
# NAMES_ADDED (each must be new) with their parent folders.
cp "$WORK/names-prev.txt" "$WORK/names-expected.txt"; decl_bad=""
for r in $NAMES_REMOVED; do
  grep -Fqx "X/$r" "$WORK/names-prev.txt" || decl_bad="$decl_bad removed:$r(not in $PREVNAME)"
  awk -v r="X/$r" '$0 != r && index($0, r "/") != 1' "$WORK/names-expected.txt" > "$WORK/names-expected.tmp"
  mv "$WORK/names-expected.tmp" "$WORK/names-expected.txt"
done
for a in $NAMES_ADDED; do
  ! grep -Fqx "X/$a" "$WORK/names-prev.txt" || decl_bad="$decl_bad added:$a(already in $PREVNAME)"
  p="$a"; while [ "$p" != . ]; do echo "X/$p"; p="$(dirname "$p")"; done >> "$WORK/names-expected.txt"
done
sort -u -o "$WORK/names-expected.txt" "$WORK/names-expected.txt"
check "NAMES_REMOVED/NAMES_ADDED declared against $PREVNAME" "${decl_bad:-ok}"
if diff "$WORK/names-expected.txt" "$WORK/names-new.txt" > "$WORK/names.diff"; then
  check "name list = $PREVNAME minus [${NAMES_REMOVED}] plus [${NAMES_ADDED}] (apart from the version, case-sensitive)" ok
else check "name list = $PREVNAME minus [${NAMES_REMOVED}] plus [${NAMES_ADDED}]" "see names.diff"; fi
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
todo="$(grep -rl 'TODO-REL' "$D" --include='*.TXT' || true)"
check "packaged release texts free of TODO-REL markers" "${todo:-ok}"
( cd "$D" && find . -type f | sed 's#^\./##' | sort ) > "$WORK/files-on-disk.txt"
sed 1d "$WORK/FILES.TSV" | cut -f1 | sed "s#^$NAME/##" | sort > "$WORK/files-listed.txt"
check "FILES.TSV lists every file of the ZIP" "$(cmp -s "$WORK/files-on-disk.txt" "$WORK/files-listed.txt" && echo ok || echo 'differs')"
rm -rf "$WORK/unzip-check"; mkdir "$WORK/unzip-check"
(cd "$WORK/unzip-check" && unzip -q "$WORK/$NAME.zip")
check "ZIP unpacks byte-identical to $NAME/" "$(diff -r "$WORK/unzip-check/$NAME" "$D" >/dev/null && echo ok || echo differs)"
rm -rf "$WORK/unzip-check"

{
  echo "version $VER  commit $HEAD  tree_dirty_build=$([ -n "$dirty_build" ] && echo yes || echo no)  tree_dirty_docs=$([ -n "$dirty" ] && echo yes || echo no)"
  echo "packager $SELF_REL $(shasum -a 256 "$SELF" | awk '{print $1}') ($SELF_STATE)"
  echo "previous $PREV_ID"
  echo "names_removed ${NAMES_REMOVED:-none}"
  echo "names_added ${NAMES_ADDED:-none}"
  echo "kernel_features $KERNEL_FEATURES"
  echo "host_clang $(clang --version | head -1)"
  echo "zip $(shasum -a 256 "$WORK/$NAME.zip" | awk '{print $1}') $(stat -f %z "$WORK/$NAME.zip") $WORK/$NAME.zip"
  echo "entries $(unzip -Z1 "$WORK/$NAME.zip" | wc -l | tr -d ' ')"
} >> "$C"
cat "$C"
[ "$fail" = 0 ] || die "checks failed (see $C)"
echo "done: $WORK/$NAME.zip"
