#!/bin/bash
t=$(ls -d "$TMPDIR"/mkcard.* "$TMPDIR"/make-sd-image.* 2>/dev/null | wc -l | tr -d ' ')
echo "  tmpdirs: $t  attached-images: $(hdiutil info | grep -c image-path)  NEWTOS/mkcard mounts: $(mount | grep -c -E '/Volumes/NEWTOS |mkcard\.')"
