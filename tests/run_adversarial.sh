#!/usr/bin/env bash
# Aethel Adversarial Test Suite
set -e
PASS=0
FAIL=0
cd "."

echo "=== Aethel Adversarial Test Suite ==="
echo ""

run_test() {
    local name="$1" file="$2" should_fail="$3"
    local output
    set +e
    output=$(cargo run -p aethel-cli -- check "$file" 2>&1)
    local status=$?
    set -e
    if [ "$should_fail" = "true" ]; then
        if echo "$output" | grep -q "AE-EPISTEMIC-001"; then
            echo "  ✅ $name (correctly rejected)"
            PASS=$((PASS+1))
        elif echo "$output" | grep -q "AE-EPISTEMIC"; then
            echo "  ✅ $name (rejected with other epistemic)"
            PASS=$((PASS+1))
        else
            echo "  ❌ $name (SHOULD HAVE FAILED)"
            echo "$output" | tail -3
            FAIL=$((FAIL+1))
        fi
    else
        if echo "$output" | grep -q "type checks"; then
            echo "  ✅ $name (correctly accepted)"
            PASS=$((PASS+1))
        else
            echo "  ❌ $name (SHOULD HAVE PASSED)"
            echo "OUTPUT:"
            echo "$output" | tail -8
            FAIL=$((FAIL+1))
        fi
    fi
}

run_test "Different effect name" "tests/fixtures/test_prod.aet" true
run_test "Different operation name" "tests/fixtures/test_op.aet" true
run_test "Renamed function/param" "tests/fixtures/test_rename.aet" true
run_test "Valid verify (renamed)" "tests/fixtures/test_valid.aet" false
run_test "Refund invalid (regression)" "examples/refund/invalid_unverified.aet" true
run_test "Refund valid (regression)" "examples/refund/valid_verified.aet" false

echo ""
echo "=== Results: $PASS passed, $FAIL failed ==="
if [ "$FAIL" -eq 0 ]; then
    echo "🎉 All adversarial tests pass!"
else
    echo "💥 $FAIL test(s) failed!"
    exit 1
fi
