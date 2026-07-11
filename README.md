# Aethel Core

> Proof-carrying effects for AI agents.

Aethel is a language runtime that prevents AI agents from turning unverified claims into unauthorized, over-budget, or duplicate real-world actions.

## Core Guarantee

```aethel
// A model output is a Claim — a proposal, not evidence
fn refund(claim: Claim<RefundDecision>) -> Receipt
uses PaymentGateway:
    // COMPILE ERROR: AE-EPISTEMIC-001
    // Expected: Verified<RefundDecision, RefundPolicy>
    // Received: Claim<RefundDecision>
    return payments.refund(claim)
```

The compiler enforces that `Claim<T>` cannot cross an effect boundary. It must first be verified under a policy:

```aethel
fn refund(claim: Claim<RefundDecision>) -> Receipt
uses PaymentGateway:
    let verified = verify(claim, RefundPolicy)  // Claim → Verified
    return payments.refund(verified)  // OK
```

## Architecture

| Crate | Responsibility |
|-------|----------------|
| `aethel-syntax` | Lexer, parser, AST, diagnostics |
| `aethel-hir` | Name resolution, HIR lowering |
| `aethel-check` | Type checking, epistemic rules |
| `aethel-ir` | Typed intermediate representation |
| `aethel-interpreter` | Durable execution (RACE) |
| `aethel-runtime` | Effect handlers, capabilities |
| `aethel-store-sqlite` | Event store, bitemporal queries |
| `aethel-model` | Model provider adapters |
| `aethel-effects` | Effect registry, operations |
| `aethel-wasm` | WASM component execution |
| `aethel-testkit` | Snapshot testing, compile-fail tests |
| `aethel-cli` | `aethel check`, `aethel fmt` |

## Quickstart

```bash
# Build
cargo build --release --workspace

# Check the canonical failing example
cargo run -p aethel-cli -- check examples/refund/invalid_unverified.aet
# → error[AE-EPISTEMIC-001]: unverified claim cannot authorize `PaymentGateway.refund`

# Check the valid version
cargo run -p aethel-cli -- check examples/refund/valid_verified.aet
# → OK

# Format
cargo run -p aethel-cli -- fmt examples/refund/main.aet
```

## Vertical Slice (v0.1)

The first milestone proves the epistemic type rule:

- ✅ Lexer (logos)
- ✅ Parser (recursive descent + Pratt)
- ✅ AST with `Claim<T>` / `Verified<T, Policy>`
- ✅ Name resolution
- ✅ Type checker with `AE-EPISTEMIC-001`
- ✅ Structured diagnostics with repair hints
- ✅ CLI (`check`, `fmt`)
- ✅ Snapshot tests

## Documentation

- [ADR 0001: Parser Architecture](docs/adr/0001-parser-architecture.md)
- [ADR 0002: First Semantic Vertical Slice](docs/adr/0002-first-semantic-vertical-slice.md)
- [Guarantees](docs/guarantees.md)
- [Non-Guarantees](docs/non-guarantees.md)

## License

MIT OR Apache-2.0