# ADR 0002: First Semantic Vertical Slice

## Status
Accepted

## Context
The Aethel implementation spec defines a 12-month roadmap with multiple phases. We need to define the **first vertical slice** that proves the core guarantee: *the compiler refuses to cross an effect boundary with an unverified claim*.

## Decision
The first vertical slice implements exactly this pipeline:

```
Aethel source
→ lexer (logos)
→ parser (recursive descent + Pratt)
→ AST
→ minimal HIR (name resolution)
→ narrow type/effect validation
→ structured diagnostic (AE-EPISTEMIC-001)
```

### In Scope
- Lexer for all keywords, operators, literals
- Parser for: modules, functions, structs, enums, policies, effects, expressions
- Name resolution for types, values, effects, policies
- Type checking for:
  - Basic types (int, bool, string, unit)
  - Function types with effect sets
  - **Epistemic types: `Claim<T>` and `Verified<T, Policy>`**
  - **Rule: `Claim<T>` is not assignable to `Verified<T, Policy>` at effect boundary**
- Diagnostic emission with:
  - Stable error code `AE-EPISTEMIC-001`
  - Source span
  - Expected vs received types
  - Repair hint: "Verify the claim under a policy before crossing this effect boundary"
- CLI command `aethel check <file>` with proper exit codes
- Formatter `aethel fmt <file> --check`
- Snapshot tests for the diagnostic

### Out of Scope (Deferred)
- Durable execution / RACE interpreter
- Exactly-once logical effects (`commit once`)
- Model budget enforcement
- WASM sandboxing
- Prompt injection prevention
- Runtime authorization
- SQLite event store
- PostgreSQL backend
- LSP server
- Package manager
- Native code generation (LLVM/MLIR)
- Distributed execution

## Rationale
This slice is the **smallest possible proof** of Aethel's differentiating feature. It:
- Can be implemented in ~2 weeks by a small team
- Produces a compelling demo: compiler error screenshot
- Establishes the testing infrastructure (snapshots, CI)
- Validates the parser/lexer/HIR/type checker integration
- Forces early decisions on error message format

## Success Criteria
The slice is complete when:
1. `cargo run -p aethel-cli -- check examples/refund/invalid_unverified.aet` exits with code 1 and prints `AE-EPISTEMIC-001`
2. `cargo run -p aethel-cli -- check examples/refund/valid_verified.aet` exits with code 0
3. `cargo run -p aethel-cli -- fmt examples/refund/main.aet --check` passes
4. All snapshot tests pass in CI
5. No warnings from `cargo clippy --workspace --all-targets --all-features -- -D warnings`

## Implementation Plan
1. Workspace + 12 crates scaffold
2. Lexer (logos) + token snapshots
3. Parser (recursive descent + Pratt) + AST snapshots
4. HIR + name resolution
5. Type checker with epistemic rule
6. Diagnostic formatting + AE-EPISTEMIC-001
7. CLI wiring
8. Formatter (dprint-based)
9. Snapshot tests + CI
10. Documentation (ADRs, guarantees, non-guarantees)