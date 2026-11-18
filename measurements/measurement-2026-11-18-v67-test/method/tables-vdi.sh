#!/bin/bash
# tables-all.sh -- the four table configs, one after the other
M=$(cd $(dirname $0) && pwd)
bash $M/run-tables-vdi.sh main base -
bash $M/run-tables-vdi.sh default wt -
bash $M/run-tables-vdi.sh v16 wt "serial=on vdiplanes=16"
bash $M/run-tables-vdi.sh v32 wt "serial=on vdiplanes=32"
bash $M/run-aes.sh
