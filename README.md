# Aethel

**A deterministic policy- and type-language for AI agent workflows.**

Aethel is a proof-of-concept compiler that enforces one core guarantee:
a `Claim<T>` (model output) cannot cross an effect boundary where a
`Verified<T, Policy>` is required — unless it has been verified first.

## Quick Start

```bash
# Build
cargo build --release -p aethel-cli

# An invalid program: passing raw Claim to an effect that requires Verified
cargo run -p aethel-cli -- check examples/refund/invalid_unverified.aet
# → AE-EPISTEMIC-001: unverified claim cannot authorize `payments.refund`

# A valid program: verifying before the effect call
cargo run -p aethel-cli -- check examples/refund/valid_verified.aet
# → ✓ type checks

# Emit deterministic Semantic IR JSON
cargo run -p aethel-cli -- emit-ir examples/refund/valid_verified.aet
```

## What Aethel Proves

Aethel's type system understands the difference between `Claim<T>` and
`Verified<T, Policy>`. This is enforced at compile time, not as a
runtime guardrail. Key rules:

| Rule | Effect |
|------|--------|
| `verify(claim, Policy)` | Produces `Verified<T, Policy>` |
| `Claim<T>` → effect boundary | **AE-EPISTEMIC-001** — rejected |
| `Verified<T, Policy>` → effect boundary | Accepted |

The enforcement is based on **types, not names**. It works identically
regardless of:
- File names
- Function names
- Parameter/variable names
- Specific effect or operation names

## Architecture

| Crate | Status |
|-------|--------|
| `aethel-syntax` | Lexer, parser, AST, diagnostics — **stable** |
| `aethel-hir` | HIR types, name resolution — **stable** |
| `aethel-check` | Type checker, epistemic rules — **new** |
| `aethel-ir` | Typed IR representation — **new** |
| `aethel-effects` | Effect registry — **stable** |
| `aethel-cli` | CLI (check, emit-ir, fmt) — **new** |
| `aethel-interpreter` | Scaffold only |
| `aethel-runtime` | Scaffold only |
| `aethel-store-sqlite` | Scaffold only |
| `aethel-model` | Scaffold only |
| `aethel-wasm` | Scaffold only |
| `aethel-testkit` | Scaffold only |

## Current Status (v0.1 Semantic Truth Slice)

**What works:**
- ✅ Lexer, parser, AST for core Aethel syntax
- ✅ Effects with operations parsed from source
- ✅ Policy declarations with evidence requirements
- ✅ `Claim<T>` and `Verified<T, Policy>` types
- ✅ `verify(claim, Policy)` expression
- ✅ `uses X:` effect declarations on functions
- ✅ Scoped type environment (params, let-bindings, blocks)
- ✅ Epistemic enforcement: `Claim<T>` rejected, `Verified<T, Policy>` accepted
- ✅ 6/6 adversarial tests (different names, effects, operations)
- ✅ `aethel emit-ir` — deterministic Semantic IR JSON

**What does NOT yet work (truth in advertising):**
- ❌ Durable execution, runtime, interpreter
- ❌ WASM sandboxing
- ❌ Package management, LSP, self-hosting
- ❌ Model provider integrations (Grok etc.)
- ❌ SQLite event store
- ❌ CI pipeline (will be set up next)

## The Bigger Picture

Aethel is designed as one layer of a three-layer stack:

```
Aethel:   What an agent may do and with what proof
FamilyClaw: How work is executed durably and exactly-once
Lumen OS:  Why work is done and what the mission priority is
```

This repository contains only the Aethel layer. See
[RELEASE_PLAN.md](RELEASE_PLAN.md) for the full roadmap.

## Documentation

- [Architecture Decisions](docs/adr/)
- [Guarantees](docs/guarantees.md) (what this phase provides)
- [Non-Guarantees](docs/non-guarantees.md) (what this phase explicitly does NOT provide)
- [Release Plan](RELEASE_PLAN.md)

## License

MIT OR Apache-2.0
