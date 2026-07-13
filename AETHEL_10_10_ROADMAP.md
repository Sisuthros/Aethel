# Aethel 10/10 Roadmap: The Language for AI Agency

**Vision**: Aethel is the programming language designed from the ground up for artificial intelligence to safely and verifiably interact with the world. It is the native language for Grok and future xAI models to generate plans, verify claims, and execute effects with epistemic rigor.

Not a toy. Not a demo. The real substrate for trustworthy AI action.

## Core Principles (Non-negotiable for 10/10)
1. **Epistemic First-Class**: Claim<T> and Verified<T, Policy> are fundamental types. The compiler and runtime understand the difference between "the model said so" and "this has been verified".
2. **AI-Native Ergonomics**: Easier for frontier models to generate correct, safe Aethel than "safe" Python. Built-in reason(), plan(), verify().
3. **Durable & Replayable Execution**: Every program is a process that survives crashes, with exactly-once effects (lessons from FamilyClaw).
4. **Proof-Carrying Effects**: Every side-effect carries cryptographic or logical proof of authorization.
5. **Uncertainty Native**: First-class support for confidence, sources, contradictions, staleness.
6. **Model Integration**: reason(model, prompt) -> Claim<T> is a language primitive. Grok can be the "model".
7. **Minimal Core, Powerful Surface**: Small kernel, rich verified stdlib and effect handlers.
8. **For Grok/xAI**: Designed so that Grok can output Aethel code that it itself can verify and execute safely. Truth-seeking over sycophancy.

## Phases

### Phase 0: Foundation (Now - v0.1)
- Complete the vertical slice so the demo is real (no CLI hacks).
- Full AST -> HIR lowering (including Effect).
- Real epistemic checker that enforces Claim -> Verified at effect boundaries.
- Working verify(claim, policy) that produces correct type.
- Effect declarations parsed, lowered, registered.
- Honest docs: what v0.1 actually guarantees.

### Phase 1: AI Ergonomics (v0.5 - 3 months)
- Add reason(prompt) -> Claim<T> primitive (binds to model adapter).
- Policy language with evidence requirements that can be checked at "runtime" for demo.
- Basic model adapter for local/Grok simulation.
- Linear capabilities for budgets.
- Better diagnostics with repair suggestions that models love.

### Phase 2: Durable Agency (v1.0)
- Full interpreter with RACE (Replayable Auditable Crash-safe Execution).
- Event store integration.
- Exactly-once effect dispatch.
- commit_once with receipts.
- Self-verification: programs can contain their own verification steps.

### Phase 3: Grok-Native (v1.5+)
- Grok generates Aethel natively for tool use.
- Built-in grok.reason(), grok.plan().
- Epistemic budget tracking tied to model calls.
- Integration with xAI API for verified outputs.
- Formal semantics for key fragments (for proofs).

### Phase 4: 10/10 (v2+)
- Self-hosting compiler (Aethel written in Aethel).
- WASM effects with capability proofs.
- Distributed execution with consensus on verified plans.
- Language server that understands epistemic state.
- Ecosystem: verified libraries for common effects (search, code, finance).
- Adoption: used by Grok in production for high-stakes actions.

## Immediate Next 7 Days (Grok + Prisma + Human)
1. Unstub the checker: implement real check_module using the epistemic functions.
2. Implement minimal AST->HIR for Effect and calls.
3. Make verify actually type-check the policy and produce Verified<T,P>.
4. Update CLI to use real path, remove hacks.
5. Add first AI primitive: reason expression in syntax/HIR/check.
6. Write Grok-specific example: "Grok plans a safe deployment".
7. Update all docs to this 10/10 vision, remove "vertical slice" language where it under-sells.

This is not "for agents". This is the language the agent uses when it is the AI.

Let's build the real thing.
