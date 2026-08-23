#!/usr/bin/env bash
# Full release gate
set -u
cd /c/Users/Ismael/aethel || exit 1
echo "=== fmt ==="
cargo fmt --all -- --check > /dev/null 2>&1 && echo "FMT ok" || echo "FMT FAIL"
echo "=== clippy ==="
cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1 | grep -cE "^error"
echo "=== tests ==="
cargo test --workspace --all-features 2>&1 | grep -E "test result:" | awk '{s+=$4; f+=$6} END {print s" passed, "f" failed"}'
echo "=== build release ==="
cargo build --release -p aethel-cli 2>&1 | tail -1
echo "=== breakers ==="
CLI=target/release/aethel-cli
pass=0; fail=0
while IFS=$'\t' read -r file code; do
  file=$(echo "$file" | tr -d '\r'); code=$(echo "$code" | tr -d '\r')
  [ -z "$file" ] && continue; [ "$file" = "fixture" ] && continue
  out=$(timeout 20 "$CLI" check "examples/breakers/$file" 2>&1); rc=$?
  if [ $rc -ne 0 ] && echo "$out" | grep -q "$code"; then pass=$((pass+1)); else fail=$((fail+1)); echo "FAIL $file"; fi
done < examples/breakers/required.tsv
echo "REQUIRED: $pass/$((pass+fail))"
echo "=== gaps line count (expect 1 = header only) ==="
wc -l < examples/breakers/known-gaps.tsv
echo "=== determinism ==="
"$CLI" emit-ir examples/refund/valid_verified.aet > "$LOCALAPPDATA/Temp/i1.json"
"$CLI" emit-ir examples/refund/valid_verified.aet > "$LOCALAPPDATA/Temp/i2.json"
diff -q "$LOCALAPPDATA/Temp/i1.json" "$LOCALAPPDATA/Temp/i2.json" && echo "emit-ir deterministic OK"
echo "=== examples ==="
"$CLI" check examples/refund/valid_verified.aet > /dev/null 2>&1 && echo "refund ok"
"$CLI" check examples/full_pipeline.aet > /dev/null 2>&1 && echo "full_pipeline ok"
"$CLI" check examples/budget/valid_ask.aet > /dev/null 2>&1 && echo "budget ok"
