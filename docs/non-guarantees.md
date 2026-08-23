# Aethel Core v0.1 Non-Guarantees

This document explicitly lists what Aethel Core v0.1 does **not** yet provide. These are tracked as future work.

## NG1: Durable Execution
**No resume-on-crash, no persistent coroutine state.**

The interpreter does not serialize stack frames to disk. A crash loses in-flight workflow state.

## NG2: Exactly-Once Logical Effects
**`commit_once` is checked syntactically but not enforced at runtime.**

The checker ensures `commit_once` is well-formed, but the runtime does not yet implement the reconciliation protocol that guarantees exactly-once execution against external providers.

## NG3: Model Budget Enforcement
**Budget tracking is static (call-count), not metered.**

Since 2026-08-23 the type checker enforces that every `ask` consumes a live,
linear `Budget` token (G4): the number of model dispatches in a function is
bounded at compile time by its token parameters. What is still *not*
enforced: actual token/currency metering against real provider APIs, and
runtime budget accounting for dynamic dispatch counts.

## NG4: Wasm Sandboxing
**No WebAssembly execution or component model isolation.**

The `aethel-wasm` crate exists but does not execute Wasm components. Tool calls run with full host authority.

## NG5: Prompt Injection Prevention
**No context separation or compiler-enforced prompt boundaries.**

The `TrustedRegion` / `UntrustedRegion` syntax exists in the grammar but is not type-checked or enforced at runtime.

## NG6: Runtime Authorization
**No authorization decisions at effect boundaries.**

Effect operations do not check policies at runtime. The `verify` expression is a type-level construct only.

## NG7: Production Security Guarantees
**No threat model validation, no supply-chain security, no audit trail.**

This is a research prototype. Do not use for production workloads without additional safeguards.

## NG8: Distributed Execution
**No remote workers, no consensus, no leader election.**

All execution is local single-process.

## NG9: Package Manager / Module System
**No dependency resolution, no versioning, no registry.**

Imports are resolved by filesystem path only.

## NG10: LSP / IDE Support
**No language server, no hover, no go-to-definition.**

Editor integration is limited to syntax highlighting via tree-sitter (future).