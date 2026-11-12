# run from the worktree root
tools/mkcard.sh $IMG measurements/measurement-2026-11-12-a81/method/newdesk-user.inf \
  APPS/TERADESK/DESKTOP.NAP=sdk/apps/teradesk/build/dist/DESKTOP.NAP \
  APPS/TERADESK/TERADESK.INF=measurements/measurement-2026-11-12-a81/method/teradesk-user.inf \
  APPS/TERADESK/DESKTOP.RSC=sdk/apps/teradesk/build/dist/desktop.rsc \
  APPS/TERADESK/ICONS.RSC=sdk/apps/teradesk/build/dist/icons.rsc \
  APPS/TERADESK/CICONS.RSC=sdk/apps/teradesk/build/dist/cicons.rsc
