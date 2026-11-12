# run from the worktree root (IMG = output image)
# tera card:
tools/mkcard.sh $IMG_TERA /Volumes/Daten/Development/PiRusTOS/measurements/measurement-2026-11-12-a83/method/newdesk-user.inf \
  APPS/TERADESK/DESKTOP.NAP=sdk/apps/teradesk/build/dist/DESKTOP.NAP \
  APPS/TERADESK/TERADESK.INF=/Volumes/Daten/Development/PiRusTOS/measurements/measurement-2026-11-12-a83/method/teradesk-user.inf \
  APPS/TERADESK/DESKTOP.RSC=sdk/apps/teradesk/build/dist/desktop.rsc \
  APPS/TERADESK/ICONS.RSC=sdk/apps/teradesk/build/dist/icons.rsc \
  APPS/TERADESK/CICONS.RSC=sdk/apps/teradesk/build/dist/cicons.rsc
# plain card:
tools/mkcard.sh $IMG_PLAIN /Volumes/Daten/Development/PiRusTOS/measurements/measurement-2026-11-12-a83/method/newdesk-plain.inf
