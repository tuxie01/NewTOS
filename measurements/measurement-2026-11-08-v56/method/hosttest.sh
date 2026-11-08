#!/bin/sh
# V56: the new host test in an optimised build (the fold is an optimiser effect): base tree red, fix tree green.
S=${S:?}
for T in base fix; do
  (cd $S/t/$T && cargo test -p tos-api --release --test aes_v56_vector_tokens > $S/hosttest/release-$T.txt 2>&1; echo "rc=$?" >> $S/hosttest/release-$T.txt)
done
