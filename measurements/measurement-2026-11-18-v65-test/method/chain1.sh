#!/bin/bash
M=$(cd $(dirname $0) && pwd)
bash $M/boots.sh > /Volumes/Daten/Development/PiRusTOS/target-agents/agent-a7a3bbc49cba62b53/checks/boots.out 2>&1
bash $M/apitest-runs.sh > /Volumes/Daten/Development/PiRusTOS/target-agents/agent-a7a3bbc49cba62b53/checks/apitest.out 2>&1
echo done > /Volumes/Daten/Development/PiRusTOS/target-agents/agent-a7a3bbc49cba62b53/chain1.done
