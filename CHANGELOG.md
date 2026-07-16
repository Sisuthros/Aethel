# Changelog

## v0.3.0 — "Verified Pipeline" (2026-07-16)

### 🎯 Pipeline complete: source → check → interpret → trace

**New: AST→IR lowering** — `check_module()` now produces non-empty `IrModule` with:
- Function definitions with bodies, params, return types, effects
- All expression types (21) including Ask, Verify, Reason, CommitOnce
- All statement types (10) including Let, If, While, For, Match, Block
- Pattern matching (Wild, Ident, Literal, Tuple, Struct, Enum, Or, Ref)
- Type lowering (15 types including Claim, Verified, Fn, Tuple, Array)

**New: Interpreter effect dispatch**
- `IrExpr::Verify` → `Verified<T, Policy>`
- `IrExpr::Ask` → `Claim<T>`
- `IrExpr::CommitOnce` — checks first argument for Verified
- `IrExpr::Reason` — side-effect marker
- `IrExpr::Call` — namespace calls produce Verified
- `IrExpr::MethodCall` — checks first arg (or receiver) for verification
- Function params bound as Claim before body evaluation

**New: Integration tests** — 9 tests covering:
- Check valid/invalid/full_pipeline
- Run with/without --trace
- Emit IR JSON structure
- Simple function pipeline test

**New: Full pipeline examples**
- `examples/full_pipeline.aet` — 3-step agent workflow
- `examples/runtime_violation.aet` — type-checker rejection demo

**Infrastructure:**
- CI pipeline: fmt → clippy → tests → negative test handling → interpreter validation
- Version bump: 0.1.0 → 0.3.0

### Test Status

| Group | Count | Status |
|-------|-------|--------|
| aethel-syntax | 9 | ✅ |
| aethel-check (pipeline) | 2 | ✅ |
| aethel-interpreter | 21 | ✅ |
| Integration tests | 9 | ✅ |
| **Total** | **32** | **✅ All green** |

---

## v0.2.0 — "IR Interpreter" (2026-07-15)

- IR interpreter with Value model (Unit, Bool, Int, Float, String, Claim, Verified, Struct)
- Environment with scoping
- Effect tracing
- Policy violation detection
- `aethel run <file>` CLI command
- `--trace` flag for effect chain display
- 21/21 interpreter tests
- Semantic Truth Slice: Claim vs Verified enforcement proved at type level

---

## v0.1.0 — "Semantic Truth Slice" (2026-07-13)

- Lexer, parser, AST for core Aethel syntax
- Effects with operations parsed from source
- Policy declarations with evidence requirements
- `Claim<T>` and `Verified<T, Policy>` types
- `verify(claim, Policy)` expression
- Scoped type environment
- Epistemic enforcement: `AE-EPISTEMIC-001`
- 6/6 adversarial tests (different names, effects, operations)
- `aethel emit-ir` — deterministic Semantic IR JSON
- 9/9 syntax tests
- Workspace compiles with 0 errors (12 crates)
