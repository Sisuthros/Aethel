#!/usr/bin/env bash
# Breaker-ajuri: ajaa kaikki required.tsv -fixturet.
# PASS = exit != 0 (hylätty) JA odotettu diagnostiikkakoodi esiintyy lähdössä.
# Fixtures may emit multiple diagnostics; the expected code must be among them.
cd /c/Users/Ismael/aethel || exit 1

cargo build -p aethel-cli 2>/dev/null
CLI=target/debug/aethel-cli.exe

pass=0; fail=0
while IFS=$'\t' read -r fixture expected; do
  fixture=$(echo "$fixture" | tr -d '\r')
  expected=$(echo "$expected" | tr -d '\r')
  [ "$fixture" = "fixture" ] && continue
  [ -z "$fixture" ] && continue
  out=$("$CLI" check "examples/breakers/$fixture" 2>&1)
  rc=$?
  if [ $rc -ne 0 ] && echo "$out" | grep -q "$expected"; then
    pass=$((pass+1))
  else
    fail=$((fail+1))
    echo "FAIL: $fixture expected=$expected rc=$rc"
    echo "$out" | head -4
  fi
done < examples/breakers/required.tsv

echo "BREAKERS: $pass pass, $fail fail"
[ $fail -eq 0 ] && echo "GATE4_BREAKERS_OK"
