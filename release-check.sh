#!/usr/bin/env bash
# release-check.sh — Aethel v0.3.0 release checklist
set -euo pipefail

echo "╔══════════════════════════════════════╗"
echo "║   Aethel v0.3.0 — Release Check    ║"
echo "╚══════════════════════════════════════╝"

# 1. Build
echo -n "[1/5] Build release... "
cargo build --release -p aethel-cli 2>/dev/null
echo "✅"

# 2. Unit tests
echo -n "[2/5] Unit tests... "
cargo test -p aethel-check -p aethel-interpreter --release 2>/dev/null
echo "✅"

# 3. Integration tests
echo -n "[3/5] Integration tests... "
cargo test -p aethel-cli --test integration_test --release 2>/dev/null
echo "✅"

# 4. Run valid example
echo -n "[4/5] Valid example run... "
./target/release/aethel-cli run examples/refund/valid_verified.aet 2>/dev/null | grep -q "No policy violations"
echo "✅"

# 5. Invalid example
echo -n "[5/5] Invalid example (exit=1)... "
set +e
./target/release/aethel-cli check examples/refund/invalid_unverified.aet 2>/dev/null
if [ $? -ne 0 ]; then echo "✅"; else echo "❌"; exit 1; fi

echo ""
echo "🎯 Release check: ALL PASS"
