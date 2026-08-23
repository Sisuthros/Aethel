# Aethel Guarantees

This document enumerates the guarantees that the **full Aethel language** (the
10/10 vision) aims to provide, with v0.1 as the first step toward them.

Every guarantee below carries a status, and the status is not a matter of
opinion: each one names the fixture in `examples/breakers/` that holds it, or
the fixture in `examples/breakers/known-gaps.tsv` that proves it is still open.
CI runs both manifests on every push.

| Status | Meaning |
|---|---|
| **Enforced** | The checker rejects the violation today, with the stated diagnostic code, and a fixture in `required.tsv` holds it. |
| **Partial** | Enforced for some shapes and demonstrably not for others. The gap is named below and has a fixture. |
| **Target** | Designed, not yet enforced. No fixture asserts it passes. |

If you find a case where an **Enforced** guarantee does not hold, that is a bug
and the fixture that missed it is the more serious bug. Please report both.

---

## G1: Epistemic Type Safety — **Enforced**
**Claim<T> cannot be used where Verified<T, Policy> is required.**

The type checker rejects programs that pass a `Claim<T>` to an effect operation
requiring `Verified<T, Policy>`. The diagnostic `AE-EPISTEMIC-001` is emitted
with:
- Stable error code
- Source span of the mismatch
- Expected type (`Verified<T, Policy>`)
- Received type (`Claim<T>`)
- Concrete repair direction

Held by: `breaker-001-tyyppivaarennos.aet`, `breaker-008-claim-returnina.aet`,
`breaker-010-claim-type-swap.aet`, `breaker-019-claim-wrong-arg.aet`,
`breaker-020-scope-escape.aet`.

This is *placement* safety: a claim cannot be pushed into a verified position.
Placement is defended. Origin is a separate question — see G5.

## G2: Effect Boundary Enforcement — **Enforced**
**Effect operations can only be called from functions declaring the effect in their `uses` clause.**

Functions must declare `uses EffectName` to call operations from that effect.
Cross-effect calls without declaration are rejected.

Held by: `breaker-011-undeclared-effect.aet` (`AE-TYPE-018`).

## G3: Capability Linearity — **Partial**
**Linear capabilities cannot be duplicated or dropped.**

The intent is that capability tokens (like `Budget`, `Context`) are
affine/linear: used exactly once, with the type checker tracking consumption.

**Drop side enforced (2026-08-23):** a `Claim`-typed function parameter that is
never consumed by `verify` before the body ends is rejected with
`AE-TYPE-013`. Held by: `breaker-016-unused-claim.aet` (promoted from
known-gaps to `required.tsv`).

What is still open is the duplication side: one claim verified twice under two
different policies and dispatched to two different effects can still pass both
gates (`AE-TYPE-012`, use-after-move, is not yet emitted). In a payments
language that is a double charge.

## G4: Budget Reservation — **Target**
**Model calls must reserve budget before dispatch.**

The intent is that the `ask` expression requires a `Budget` capability and
reserves tokens statically, making exhaustion a compile-time error.

Budget tracking today is compile-time bookkeeping with no runtime enforcement.
See `docs/non-guarantees.md` NG3, which is the accurate description.

## G5: Verified Construction — **Enforced**
**`Verified<T, Policy>` can only be constructed via `verify(claim, policy)`.**

There is no public constructor for `Verified`. The `verify` expression is the
only way to produce it, and it type-checks that the policy exists and the
claim matches.

**Origin enforced (2026-08-23):** a type annotation without an initialiser is
no longer a second constructor. A bare `let v: Verified<D, DPolicy>;`
declaration is rejected with `AE-EPISTEMIC-002` — only a binding produced by
`verify` may carry the `Verified` type.

Held by: `breaker-021-origin-uninitialised.aet` (promoted from known-gaps to
`required.tsv`, expected code `AE-EPISTEMIC-002`). The symbolic interpreter
still blocks the same program at runtime as defence in depth.

## G6: Commit Once Semantics — **Enforced**
**`commit_once` expressions are recorded in the event log before execution.**

The checker ensures `commit_once` appears only in functions with the
corresponding effect declared, and that its arguments are well-typed.

## G7: Deterministic Diagnostics — **Enforced**
**All diagnostics are deterministic and include machine-readable codes.**

Every diagnostic has a stable code (e.g., `AE-EPISTEMIC-001`), structured
labels, and optional help text. Output is reproducible across runs. The CI
breaker gate depends on this: a fixture counts as caught only if the checker
exits non-zero *and* prints the exact code its manifest names.

## G8: Parse-Format-Parse Idempotence — **Enforced**
**Formatting a parsed file produces the same AST.**

The formatter is a pretty-printer that round-trips through the AST without
semantic changes.

---

## Related documents

- `docs/non-guarantees.md` — what Aethel deliberately does not promise.
- `examples/breakers/required.tsv` — every fixture the checker must reject, with
  the exact diagnostic code each must produce.
- `examples/breakers/known-gaps.tsv` — every fixture that is *not* caught today,
  each with a written reason. These run for visibility and cannot fail the build.
