# Aethel

**A deterministic policy- and type-language for AI agent workflows.**

*Aethel enforces one core guarantee: a `Claim<T>` (model output) cannot cross an effect boundary where a `Verified<T, Policy>` is required — verified first, or rejected.*

```bash
# Quick start
cargo build --release -p aethel-cli

# Type-check and run a complete agent workflow
./target/release/aethel-cli run examples/full_pipeline.aet --trace
```

---

## What Aethel Does

AI agents produce claims. An agent says *"this transaction is safe"* — that's a `Claim<RiskAssessment>`. Before that claim can authorize a bank transfer, it must be **verified** against a policy.

Aethel enforces this at **compile time** and **at runtime**:

| What you write | What it means | Enforced? |
|---|---|---|
| `verify(claim, Policy)` | Produces `Verified<T, Policy>` | ✅ Type system |
| `Claim<T>` → effect boundary | Rejected | ✅ **AE-EPISTEMIC-001** |
| `Verified<T, Policy>` → effect boundary | Accepted | ✅ Runtime trace |

The enforcement is **type-based** — it works regardless of names, files, or conventions.

## Demo

```bash
# An invalid program: passing raw Claim to an effect
./target/release/aethel-cli check examples/refund/invalid_unverified.aet
# → AE-EPISTEMIC-001: unverified claim cannot authorize `payments.refund`

# A valid program: verify first
./target/release/aethel-cli run examples/refund/valid_verified.aet
# → ✓ No policy violations

# Full pipeline: reason → verify → log → execute
./target/release/aethel-cli run examples/full_pipeline.aet --trace
# → 4 claims, 3 verified, 0 violations
# → Effect Trace:
#      1. ✓ effect `log_action`
#      2. ✓ effect `execute`

# Inspect the program as Semantic IR JSON
./target/release/aethel-cli emit-ir examples/refund/valid_verified.aet
```

## Status — v0.3 "Verified Pipeline"

### ✅ Pipeline: source → verified execution

| Stage | Status | Tests |
|-------|--------|-------|
| Lexer | ✅ 9/9 syntax | `cargo test -p aethel-syntax` |
| Parser → AST | ✅ | |
| HIR lowering + name resolution | ✅ | |
| Type checker (Claim/Verified) | ✅ 6/6 adversarial | `cargo test -p aethel-check` |
| **AST→IR lowering** | ✅ **NEW** | collector produces non-empty IR |
| **Interpreter** (effect dispatch) | ✅ **NEW** | 21/21 tests |
| **Integration tests** | ✅ **NEW** | 9/9 (check, run, trace, emit-ir) |
| **CI pipeline** | ✅ GitHub Actions | fmt + clippy + test + negative tests |

### ✅ What Aethel understands

| Feature | Example | Status |
|---------|---------|--------|
| Types | `int`, `bool`, `string`, structs | ✅ |
| Epistemic types | `Claim<T>`, `Verified<T, Policy>` | ✅ |
| Effects | `effect Name { fn op(params) -> Ret }` | ✅ |
| Policies | `policy Name { ClaimName: Type { evidence ... } }` | ✅ |
| `verify(claim, Policy)` | → `Verified<T, Policy>` | ✅ |
| `ask(model, goal, input)` | → `Claim<T>` | ✅ |
| `reason("prompt")` | Reasoning step marker | ✅ |
| `commit_once effect(args)` | Effect dispatch with verification | ✅ |
| Method calls | `effect.op(verified)` with arg check | ✅ |
| `aethel run --trace` | Effect chain trace | ✅ |

## Architecture

```
┌──────────────────────────────────────────────────┐
│                   aethel-cli                      │
│  check │ run │ emit-ir │ fmt                     │
├──────────────────────────────────────────────────┤
│  aethel-check   │  aethel-interpreter             │
│  ┌──────────────┤  ┌───────────────────────────┐  │
│  │ Type checker  │  │ Evaluator                │  │
│  │ Epistemic     │  │ Value model              │  │
│  │ rules         │  │ Effect trace             │  │
│  │ AST→IR lower  │  │ Policy violation detect  │  │
│  └──────────────┤  └───────────────────────────┘  │
├──────────────────┴───────────────────────────────┤
│  aethel-ir   │  aethel-hir   │  aethel-syntax    │
│  IR types    │  HIR types    │  Lexer → Parser   │
│  lowering    │  resolution   │  → AST → spans    │
├──────────────────────────────────────────────────┤
│  aethel-effects   │  aethel-runtime  │  storage   │
│  (effect reg.)    │  (stubs)         │  (stubs)    │
└──────────────────────────────────────────────────┘
```

**12 crates** | **~8,842 LOC** | **32 tests** (23 unit + 9 integration)

## Quick Reference

```bash
cargo build --release -p aethel-cli
alias aethel=./target/release/aethel-cli

aethel check file.aet                # Type-check only
aethel run file.aet                  # Type-check + interpret
aethel run file.aet --trace          # + show effect trace
aethel emit-ir file.aet              # Semantic IR JSON
aethel fmt file.aet                  # Format source (stub)
```

## Language Example

```aethel
fn process_action(raw_action: Claim<UserAction>) -> ActionResult
uses AuditService, ExecutionService:
    {
        // Reason about the action
        let _ = reason("Analyzing action risk ...");

        // Verify — Claim → Verified
        let assessed = verify(raw_action, RiskAssessment);

        // Use verified value across multiple effects
        let _log = audit_service.log_action(assessed);
        let result = execution_service.execute(assessed);

        return result;
    }
```

See `examples/full_pipeline.aet` for the complete version.

## The Bigger Picture

Aethel is one layer of a three-layer stack:

```
Aethel:     What an agent may do and with what proof
FamilyClaw: How work is executed durably and exactly-once
Lumen OS:   Why work is done and what the mission priority is
```

## License

MIT OR Apache-2.0
