# ZVIEW06 — zView: Bus Error while loading certain images (GitHub #18)

Status: OPEN (2026-11-18) | Priority: P2

Title: zView: Bus Error while loading certain images (GitHub #18)

Type: root-cause research first (auditor or researcher), then developer.
Owner decision 2026-11-18, from GitHub issue #18 (tuxie01/NewTOS, "zView
Absturz beim beenden"), the reporter's comment of 2026-11-13 with five photos.
No fix before the cause is established with evidence.

## Facts (read from the reporter's photos, 2026-11-18)

Three crash screens on the reporter's Pi, release 0.7 or later (the screen
report carries the `crash-walk` block and the ladder line):

| | Photo 1 | Photo 2 | Photo 3 |
|---|---|---|---|
| Report | `Panic: Bus Error`, `esr=92000007`, "Memory violation: free memory, read (translation fault, level 3)" | same | same |
| `addr` | `0000000002044fa8` | `0000000002015d58` | `0000000002014fa8` |
| `pc` | `000000001f7fdb8` | same | same |
| `X0` | `0000000002044fc0` | `0000000002015d70` | `0000000002014fc0` |
| `X30` | `000000001f7f788` | same | same |
| `basepage` | `01c64000` | same | same |

- `X0` is `addr` + `0x18` in all three.
- `text=01c65000 data=01cc5fe0 bss=01ccb230`: `pc` is not in the program's
  own text. The `crash-walk` block for `pc` reads "pool yes block
  1f7e000-1fab000 mode P own 1c64000 owners 2", `model=valid table=valid
  at=ok`: the code runs in a separate block owned by the zView process.
- The `crash-walk` block for the fault address reads "arena#362 used ->
  INVALID at L3", "model: pool yes, no block", `model=invalid table=invalid
  at=fault table==model yes`, ladder "=> none": the address lies in memory
  that is not allocated, and the page table agrees with the model. This is
  not the pattern of TASK/PROC08.md (there: `model=valid table=valid
  at=fault`).
- Photos 2 and 3 show zView's "Please Wait..." box and the busy pointer; photo
  3 shows zView's menu bar with *File* selected. The crash happens while an
  image is being loaded, not at exit as the issue title says.
- On the reporter's Milan the same images give messages, no crash: zView
  "zView doesn't support this image format." and Papillon "Das GIF-Bild
  konnte nicht ganz korrekt ausgepackt werden!" (photos 4 and 5). The
  reporter also names Smurf and Vision as opening them, "sometimes with a
  message".

## Not established

- Which code lies at `1f7fdb8` (which module of the zView port is loaded into
  that block) and which function reads at `X0 - 0x18`.
- Whether the read is a defect of upstream zView that a machine without
  memory protection hides, or of the port (LP64, `sdk/overlays/`).
- Whether the files are damaged GIFs; the reporter's files are not in the
  repository. Asked for in #18 on 2026-11-18.
- Whether the crash at exit of the original report (0.6.1) still exists.

## Work

1. With the reporter's files: reproduce in QEMU raspi4b with memory
   protection on; record `pc`, the block and its load address.
2. Resolve `pc` to module and function (link map of the zView build), read
   the function in upstream and in the overlay patches, name the read.
3. Report the cause to the owner with the fix options (patch file in
   `sdk/overlays/`, upstream unchanged); then the developer.

## Acceptance

- The cause is named with evidence (address resolved, source line, a run
  directory under `measurements/` that reproduces it).
- After the fix: the reporter's files load or give zView's own error message,
  no Bus Error; the images zView showed before still show.

## Blocked on

The image files from the reporter. Without them step 1 cannot start; steps 2
and 3 can start from the addresses above if the card's zView build is still
at hand.
