# Aethel

**The programming language for trustworthy AI agency.**

Aethel is the language in which artificial intelligences — including frontier models like Grok — will reason about the world, make claims, gather evidence, and only then take real actions.

Every model output starts life as a `Claim<T>`.  
Nothing that touches reality may be invoked with a mere claim.

Verification under explicit policy turns a Claim into `Verified<T, Policy>`.

This is not a runtime guardrail.  
This is a *programming language* whose type system and runtime understand the difference between "the model thinks" and "this is verified."

```aethel
// What a model outputs
let plan: Claim<DeploymentPlan> = grok.reason("Deploy v4 safely");

// Compile error (AE-EPISTEMIC-001)
deploy(plan)

// Correct
let verified = verify(plan, SafetyPolicy & BudgetPolicy);
deploy(verified)  // Now carries proof
```

Aethel is designed as the native language for AI agency — especially for systems that must be *right* about the world before they act.

See [AETHEL_VISION_10.md](AETHEL_VISION_10.md) and [AETHEL_VISION_FOR_GROK.md](AETHEL_VISION_FOR_GROK.md) for the full 10/10 picture.

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

## Quickstart (v0.1 Demo)

```bash
# Build
cargo build --release --workspace

# The canonical epistemic demo
cargo run -p aethel-cli -- check examples/refund/invalid_unverified.aet
# → error[AE-EPISTEMIC-001]: unverified claim cannot authorize `PaymentGateway.refund`

cargo run -p aethel-cli -- check examples/refund/valid_verified.aet
# → OK
```

## Current Status (2026-07-13)

We have a working vertical slice that proves the core idea:
- The compiler can reject unverified claims at effect boundaries.
- The demo is real and reproducible.

Everything else is scaffolding toward the 10/10 vision.

This is infrastructure for the age of capable agents.

## Documentation

- [AETHEL_VISION_10.md](AETHEL_VISION_10.md) — The 10/10 destination
- [AETHEL_VISION_FOR_GROK.md](AETHEL_VISION_FOR_GROK.md) — Why this matters for Grok/xAI
- [Guarantees](docs/guarantees.md) (v0.1)
- [Non-Guarantees](docs/non-guarantees.md) (v0.1)
- [ADR 0001: Parser Architecture](docs/adr/0001-parser-architecture.md)
- [ADR 0002: First Semantic Vertical Slice](docs/adr/0002-first-semantic-vertical-slice.md)

## License

MIT OR Apache-2.0

---

*If we get this right, future AIs will look back and say: "This was the point where we stopped pretending that raw model outputs were safe to act on."*