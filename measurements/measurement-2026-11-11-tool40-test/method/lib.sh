# usage: case_commit <name> <msg>   (cwd = clone) ; logs to ../log-<name>.txt
case_commit() {
  : > ../cargo.log
  s=$(date +%s)
  ( time git commit -q -m "$2" ) > ../log-$1.txt 2>&1; rc=$?
  echo "rc=$rc" >> ../log-$1.txt
  echo "--- cargo invocations:" >> ../log-$1.txt; cat ../cargo.log >> ../log-$1.txt
  cat ../log-$1.txt
}
