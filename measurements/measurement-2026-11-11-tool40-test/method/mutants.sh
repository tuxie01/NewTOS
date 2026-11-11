# run in clone; each mutant: patch, run host test, record, restore via git checkout (throwaway clone only)
run() { name=$1; shift
  git checkout -q -- tools/xtask/src
  "$@"
  git diff --stat | tail -2 > ../mutant-$name.diff.txt; git diff > ../mutant-$name.patch
  cargo test -p xtask staged_reports > ../mutant-$name.log 2>&1
  echo "== $name: $(grep -E '^test result' ../mutant-$name.log | head -1) | build_err=$(grep -c '^error' ../mutant-$name.log)"
  grep -E "panicked|assert" ../mutant-$name.log | head -4
  git checkout -q -- tools/xtask/src
}
# a: read working tree instead of the view
perl_a() { perl -0pi -e 's/return Ok\(None\);\n            \}\n            return self\.workspace_file\(rel\)\.map\(Some\);/return Ok(None);\n            }\n            return Ok(fs::read(self.root.join(rel)).ok().map(Rc::from));/' tools/xtask/src/doc_check.rs; }
# b: compare by line number instead of row identity
perl_b() { perl -0pi -e 's/BTreeMap<\(String, String\), usize>/BTreeMap<(Option<usize>, String), usize>/; s/left\.entry\(\(p\.row, p\.what\)\)/left.entry((p.line, p.what))/; s/left\.get_mut\(&\(p\.row\.clone\(\), p\.what\.clone\(\)\)\)/left.get_mut(&(p.line, p.what.clone()))/' tools/xtask/src/deps.rs; }
# c: drop HEAD comparison, everything blocks
perl_c() { perl -0pi -e 's/Some\(n\) if \*n > 0 => \{/Some(n) if false \&\& *n > 0 => {/' tools/xtask/src/deps.rs; }
run a perl_a; run b perl_b; run c perl_c
