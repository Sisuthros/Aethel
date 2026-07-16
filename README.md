# Aethel

**A deterministic policy and type language for trustworthy AI-agent effects.**

Aethel enforces one core invariant:

> A `Claim<T>` cannot be used where an effect requires `Verified<T, Policy>`.

The compiler rejects the program before an effect is dispatched. The bundled interpreter is a fail-closed symbolic simulator for tests and traces, not a production effect runtime.

```bash
cargo build --release -p aethel-cli

./target/release/aethel-cli check examples/refund/invalid_unverified.aet
./target/release/aethel-cli run examples/full_pipeline.aet --trace
```

## Core model

AI models produce claims. Claims are untrusted values:

```aethel
Claim<RiskAssessment>
```

A declared policy can transform a compatible claim into a policy-bound verified value:

```aethel
let assessed = verify(raw_action, RiskAssessmentPolicy);
// assessed: Verified<UserAction, RiskAssessmentPolicy>
```

Effects declare the exact verified type and policy they accept:

```aethel
effect ExecutionService {
    fn execute(action: Verified<UserAction, RiskAssessmentPolicy>) -> ActionResult
}
```

The checker rejects:

- `Claim<T>` assigned to `Verified<T, Policy>`
- raw claims crossing effect boundaries
- policy mismatches
- unknown policies, effects, operations, types, or values
- wrong argument counts and incompatible argument types
- returning `Claim<T>` from a function that promises `Verified<T, Policy>`
- direct construction of `Verified` values outside `verify`

## Compiler pipeline

```text
source
  → lexer and parser
  → AST
  → HIR lowering
  → name and declaration resolution
  → semantic type and policy checking
  → IR
  → fail-closed symbolic simulation
```

All CLI commands use the same checker entrypoint. There is no separate permissive path for `emit-ir`.

## Commands

```bash
alias aethel=./target/release/aethel-cli

aethel check file.aet
aethel run file.aet
aethel run file.aet --trace
aethel emit-ir file.aet
aethel fmt file.aet       # parser validation only; formatter remains a stub
```

## Example

```aethel
struct UserAction {
    id: string,
    description: string,
}

struct ActionResult {
    status: string,
}

policy RiskAssessmentPolicy {
    ActionRisk: UserAction {
        evidence SignedAttestation "risk model assessment"
    }
}

effect AuditService {
    fn log_action(action: Verified<UserAction, RiskAssessmentPolicy>) -> ActionResult
}

effect ExecutionService {
    fn execute(action: Verified<UserAction, RiskAssessmentPolicy>) -> ActionResult
}

fn process_action(raw_action: Claim<UserAction>) -> ActionResult
uses AuditService, ExecutionService:
    {
        let assessed = verify(raw_action, RiskAssessmentPolicy);
        let _audit = audit_service.log_action(assessed);
        let result = execution_service.execute(assessed);
        return result;
    }
```

See `examples/full_pipeline.aet` for the complete runnable demonstration.

## Safety posture

### Enforced by the checker

- scoped function parameters and local bindings
- assignment and return-type compatibility
- function and effect argument arity
- structural `Claim<T>` and `Verified<T, Policy>` separation
- exact verification-policy matching
- policy existence and accepted claim types
- fail-closed unknown and ambiguous effect operations
- stable machine-readable diagnostics such as `AE-EPISTEMIC-001`

### Enforced by the symbolic interpreter

- generic calls never mint verified values
- `verify` accepts only runtime `Claim` values
- unverified effect attempts produce violations and error values
- failed effects never return verified values
- arithmetic no longer returns fabricated booleans

### Not provided by Aethel Core

- external effect execution
- durable resume after crashes
- exactly-once side effects
- real evidence acquisition or cryptographic proof validation
- model-provider integrations
- WASM isolation
- distributed execution
- package management or LSP support

Those boundaries are deliberate. In the intended stack:

```text
Lumen OS    chooses why work is done
Aethel      decides what may be done and what proof is required
FamilyClaw  executes approved work durably and exactly once
```

## Verification

The repository includes positive examples and adversarial breaker fixtures. CI requires:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-features
cargo build --release -p aethel-cli
cargo test -p aethel-cli --test integration_test --release
```

Breaker fixtures must fail with their expected diagnostic code. A parser crash or unrelated error does not count as a successful security test.

## Status

Aethel v0.3 is an **alpha policy compiler and symbolic simulator**. Its compiler boundary is intended to fail closed, but it is not a standalone production runtime. Review `docs/non-guarantees.md` before integrating it with real effects.

## License

MIT OR Apache-2.0
