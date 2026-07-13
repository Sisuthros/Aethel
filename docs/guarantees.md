# Aethel Guarantees (Aspirational 10/10)

This document enumerates the guarantees that the **full Aethel language** (the 10/10 vision) aims to provide, with v0.1 as the first step toward them. These are the invariants we will hold sacred as we build the real language for trustworthy AI agency.

## G1: Epistemic Type Safety
**Claim<T> cannot be used where Verified<T, Policy> is required.**

The type checker rejects programs that pass a `Claim<T>` to an effect operation requiring `Verified<T, Policy>`. The diagnostic `AE-EPISTEMIC-001` is emitted with:
- Stable error code
- Source span of the mismatch
- Expected type (`Verified<T, Policy>`)
- Received type (`Claim<T>`)
- Concrete repair direction

## G2: Effect Boundary Enforcement
**Effect operations can only be called from functions declaring the effect in their `uses` clause.**

Functions must declare `uses EffectName` to call operations from that effect. Cross-effect calls without declaration are rejected.

## G3: Capability Linearity
**Linear capabilities cannot be duplicated or dropped.**

Capability tokens (like `Budget`, `Context`) are affine/linear: they must be used exactly once. The type checker tracks consumption.

## G4: Budget Reservation
**Model calls must reserve budget before dispatch.**

The `ask` expression requires a `Budget` capability and reserves tokens statically. Exhaustion is a compile-time error.

## G5: Verified Construction
**`Verified<T, Policy>` can only be constructed via `verify(claim, policy)`.**

There is no public constructor for `Verified`. The `verify` expression is the only way to produce it, and it type-checks that the policy exists and the claim matches.

## G6: Commit Once Semantics
**`commit_once` expressions are recorded in the event log before execution.**

The checker ensures `commit_once` appears only in functions with the corresponding effect declared, and that its arguments are well-typed.

## G7: Deterministic Diagnostics
**All diagnostics are deterministic and include machine-readable codes.**

Every diagnostic has a stable code (e.g., `AE-EPISTEMIC-001`), structured labels, and optional help text. Output is reproducible across runs.

## G8: Parse-Format-Parse Idempotence
**Formatting a parsed file produces the same AST.**

The formatter is a pretty-printer that round-trips through the AST without semantic changes.